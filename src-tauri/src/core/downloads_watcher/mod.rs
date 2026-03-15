use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::AppHandle;
use walkdir::WalkDir;

use crate::{
    app_state::AppState,
    commands::{emit_downloads_status, emit_workspace_change},
    core::{
        bundle_detector, content_versions, duplicate_detector,
        install_profile_engine::{
            self, DownloadItemAssessment, SpecialDecisionContext, SpecialDecisionDetailLevel,
        },
        rule_engine,
        scanner::{self, DiscoveredFile},
    },
    database,
    error::{AppError, AppResult},
    models::{
        CatalogSourceInfo, DownloadInboxDetail, DownloadInboxFile, DownloadIntakeMode,
        DownloadQueueLane, DownloadRiskLevel, DownloadsInboxItem, DownloadsInboxOverview,
        DownloadsInboxQuery, DownloadsInboxResponse, DownloadsSelectionResponse,
        DownloadsTimelineEntry, DownloadsWatcherState, DownloadsWatcherStatus, GuidedInstallPlan,
        LibrarySettings, OrganizationPreview, SpecialReviewPlan, WorkspaceChange, WorkspaceDomain,
    },
};

const WATCHER_DEBOUNCE_MS: u64 = 900;
// Bump when download-side evidence rules change so existing inbox items get reassessed once.
const DOWNLOADS_ASSESSMENT_VERSION_PREFIX: &str = "downloads-assessment-v3";
const AUTO_RECHECK_NOTE_PREFIX: &str = "Rechecked with newer SimSuite rules";
const SLOW_DOWNLOADS_LOG_THRESHOLD_MS: u128 = 40;
const HELD_ARCHIVE_SAFETY_NOTE: &str =
    "SimSuite paused this archive type for safety. Use a ZIP version for now, or review the archive manually.";
const IGNORED_NON_SIMS_DOWNLOAD_NOTE: &str =
    "SimSuite ignored this download because it does not contain Sims mod or Tray files.";

#[cfg(debug_assertions)]
fn append_perf_trace(line: &str) {
    let Some(path) = std::env::var("SIMSUITE_PERF_TRACE_PATH")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    let trace_path = PathBuf::from(path);
    if let Some(parent) = trace_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&trace_path)
    {
        let _ = writeln!(file, "{line}");
    }
}

fn log_slow_downloads_operation(operation: &str, started_at: Instant, count: usize) {
    #[cfg(debug_assertions)]
    {
        let elapsed_ms = started_at.elapsed().as_millis();
        if elapsed_ms >= SLOW_DOWNLOADS_LOG_THRESHOLD_MS {
            let line = format!("[perf] {operation} took {elapsed_ms}ms for {count} item(s)");
            eprintln!("{line}");
            append_perf_trace(&line);
        }
    }
}

fn log_slow_downloads_step(operation: &str, started_at: Instant, detail: impl FnOnce() -> String) {
    #[cfg(debug_assertions)]
    {
        let elapsed_ms = started_at.elapsed().as_millis();
        if elapsed_ms >= SLOW_DOWNLOADS_LOG_THRESHOLD_MS {
            let line = format!("[perf] {operation} took {elapsed_ms}ms {}", detail());
            eprintln!("{line}");
            append_perf_trace(&line);
        }
    }
}

fn checking_downloads_status(
    watched_root: &Path,
    current_item: Option<String>,
) -> DownloadsWatcherStatus {
    DownloadsWatcherStatus {
        state: DownloadsWatcherState::Processing,
        watched_path: Some(watched_root.to_string_lossy().to_string()),
        configured: true,
        current_item,
        last_run_at: None,
        last_change_at: None,
        last_error: None,
        ready_items: 0,
        needs_review_items: 0,
        active_items: 0,
    }
}

#[derive(Debug, Clone)]
struct ObservedSource {
    path: PathBuf,
    display_name: String,
    source_kind: String,
    archive_format: Option<String>,
    source_size: i64,
    source_modified_at: Option<String>,
}

#[derive(Debug, Clone)]
struct ExistingDownloadItem {
    id: i64,
    source_path: String,
    source_size: i64,
    source_modified_at: Option<String>,
    status: String,
    source_kind: String,
    active_file_count: i64,
}

#[derive(Debug, Clone)]
pub struct DownloadItemSourceRecord {
    pub id: i64,
    pub display_name: String,
    pub source_path: String,
    pub source_kind: String,
    pub archive_format: Option<String>,
    pub source_size: i64,
    pub source_modified_at: Option<String>,
    pub staging_path: Option<String>,
}

pub fn restart_watcher(app: &AppHandle, state: &AppState) -> AppResult<()> {
    stop_watcher(state)?;

    let connection = state.connection()?;
    let settings = database::get_library_settings(&connection)?;
    let Some(downloads_path) = settings
        .downloads_path
        .filter(|value| !value.trim().is_empty())
    else {
        let status = DownloadsWatcherStatus::default();
        store_status(state, app, status)?;
        return Ok(());
    };

    let watched_root = PathBuf::from(downloads_path.trim());
    if !watched_root.exists() {
        store_status(
            state,
            app,
            DownloadsWatcherStatus {
                state: DownloadsWatcherState::Error,
                watched_path: Some(watched_root.to_string_lossy().to_string()),
                configured: true,
                current_item: None,
                last_run_at: None,
                last_change_at: None,
                last_error: Some("Downloads folder does not exist.".to_owned()),
                ready_items: 0,
                needs_review_items: 0,
                active_items: 0,
            },
        )?;
        return Ok(());
    }

    store_status(
        state,
        app,
        checking_downloads_status(&watched_root, Some("Initial inbox refresh".to_owned())),
    )?;

    let (stop_sender, stop_receiver) = mpsc::channel::<()>();
    {
        let control = state.downloads_watcher_control();
        let mut guard = control
            .lock()
            .map_err(|_| AppError::Message("Downloads watcher lock poisoned".to_owned()))?;
        guard.stop_sender = Some(stop_sender);
    }

    let thread_app = app.clone();
    let thread_state = state.clone();
    thread::spawn(move || watch_loop(thread_app, thread_state, watched_root, stop_receiver));

    Ok(())
}

pub fn refresh_inbox(app: &AppHandle, state: &AppState) -> AppResult<DownloadsWatcherStatus> {
    let current_status = state
        .downloads_status()
        .lock()
        .map_err(|_| AppError::Message("Downloads status lock poisoned".to_owned()))?
        .clone();

    if current_status.state == DownloadsWatcherState::Processing {
        return Ok(current_status);
    }

    let connection = state.connection()?;
    let settings = database::get_library_settings(&connection)?;
    let watched_path = settings
        .downloads_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_owned());

    if watched_path.is_none() {
        return process_downloads_once(app, state, Some("Manual inbox refresh".to_owned()), true);
    }

    let starting_status = DownloadsWatcherStatus {
        state: DownloadsWatcherState::Processing,
        watched_path,
        configured: true,
        current_item: Some("Manual inbox refresh".to_owned()),
        last_run_at: current_status.last_run_at.clone(),
        last_change_at: current_status.last_change_at.clone(),
        last_error: None,
        ready_items: current_status.ready_items,
        needs_review_items: current_status.needs_review_items,
        active_items: current_status.active_items,
    };

    store_status(state, app, starting_status.clone())?;

    let thread_app = app.clone();
    let thread_state = state.clone();
    std::thread::spawn(move || {
        if let Err(error) = process_downloads_once(
            &thread_app,
            &thread_state,
            Some("Manual inbox refresh".to_owned()),
            true,
        ) {
            let fallback = thread_state
                .downloads_status()
                .lock()
                .map(|status| DownloadsWatcherStatus {
                    state: DownloadsWatcherState::Error,
                    watched_path: status.watched_path.clone(),
                    configured: status.configured,
                    current_item: Some("Manual inbox refresh".to_owned()),
                    last_run_at: status.last_run_at.clone(),
                    last_change_at: status.last_change_at.clone(),
                    last_error: Some(error.to_string()),
                    ready_items: status.ready_items,
                    needs_review_items: status.needs_review_items,
                    active_items: status.active_items,
                })
                .unwrap_or_default();
            let _ = store_status(&thread_state, &thread_app, fallback);
        }
    });

    Ok(starting_status)
}

pub fn list_download_items(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    query: DownloadsInboxQuery,
) -> AppResult<DownloadsInboxResponse> {
    list_download_items_internal(connection, settings, seed_pack, query, true)
}

pub fn list_download_queue(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    query: DownloadsInboxQuery,
) -> AppResult<DownloadsInboxResponse> {
    list_download_items_internal(connection, settings, seed_pack, query, false)
}

fn list_download_items_internal(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    query: DownloadsInboxQuery,
    include_timelines: bool,
) -> AppResult<DownloadsInboxResponse> {
    let started_at = Instant::now();
    let overview_started_at = Instant::now();
    let mut overview = load_overview(connection, settings)?;
    log_slow_downloads_step("downloads_queue::overview", overview_started_at, || {
        format!(
            "with {} total item(s) and {} active file(s)",
            overview.total_items, overview.active_files
        )
    });
    let mut sql = String::from(
        "SELECT
            di.id,
            di.display_name,
            di.source_path,
            di.source_kind,
            di.archive_format,
            di.status,
            di.source_size,
            di.detected_file_count,
            di.intake_mode,
            di.risk_level,
            di.matched_profile_key,
            di.matched_profile_name,
            di.special_family,
            di.assessment_reasons,
            di.dependency_summary,
            di.missing_dependencies,
            di.inbox_dependencies,
            di.incompatibility_warnings,
            di.post_install_notes,
            di.evidence_summary,
            di.catalog_source_url,
            di.catalog_download_url,
            di.latest_check_url,
            di.latest_check_strategy,
            di.catalog_reference_source,
            di.catalog_reviewed_at,
            di.existing_install_detected,
            di.guided_install_available,
            di.first_seen_at,
            di.last_seen_at,
            di.updated_at,
            di.error_message,
            di.notes,
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.download_item_id = di.id
                  AND f.source_location = 'downloads'
            ) AS active_file_count,
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.download_item_id = di.id
                  AND f.source_location <> 'downloads'
            ) AS applied_file_count,
            (
                SELECT COUNT(DISTINCT rq.file_id)
                FROM review_queue rq
                JOIN files f ON f.id = rq.file_id
                WHERE f.download_item_id = di.id
                  AND f.source_location = 'downloads'
            ) AS review_file_count
         FROM download_items di
         WHERE 1 = 1",
    );
    let mut params = Vec::new();

    if let Some(search) = query
        .search
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        sql.push_str(" AND (di.display_name LIKE ?1 OR di.source_path LIKE ?1)");
        params.push(format!("%{search}%"));
    }

    let requested_status = query
        .status
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());

    if let Some(status) = requested_status {
        let index = params.len() + 1;
        sql.push_str(&format!(" AND di.status = ?{index}"));
        params.push(status.to_owned());
    } else {
        sql.push_str(" AND di.status <> 'ignored'");
    }

    let limit = query.limit.unwrap_or(120);
    let index = params.len() + 1;
    sql.push_str(&format!(
        " ORDER BY di.updated_at DESC, di.display_name COLLATE NOCASE LIMIT ?{index}"
    ));
    let mut values = params
        .into_iter()
        .map(rusqlite::types::Value::Text)
        .collect::<Vec<_>>();
    values.push(rusqlite::types::Value::Integer(limit));

    let query_started_at = Instant::now();
    let mut statement = connection.prepare(&sql)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values.iter()), |row| {
            Ok(DownloadsInboxItem {
                id: row.get(0)?,
                display_name: row.get(1)?,
                source_path: row.get(2)?,
                source_kind: row.get(3)?,
                archive_format: row.get(4)?,
                status: row.get(5)?,
                source_size: row.get(6)?,
                detected_file_count: row.get(7)?,
                intake_mode: parse_intake_mode(row.get::<_, String>(8)?),
                risk_level: parse_risk_level(row.get::<_, String>(9)?),
                matched_profile_key: row.get(10)?,
                matched_profile_name: row.get(11)?,
                special_family: row.get(12)?,
                assessment_reasons: parse_string_array(row.get::<_, String>(13)?),
                dependency_summary: parse_string_array(row.get::<_, String>(14)?),
                missing_dependencies: parse_string_array(row.get::<_, String>(15)?),
                inbox_dependencies: parse_string_array(row.get::<_, String>(16)?),
                incompatibility_warnings: parse_string_array(row.get::<_, String>(17)?),
                post_install_notes: parse_string_array(row.get::<_, String>(18)?),
                evidence_summary: parse_string_array(row.get::<_, String>(19)?),
                catalog_source: parse_catalog_source(
                    row.get(20)?,
                    row.get(21)?,
                    row.get(22)?,
                    row.get(23)?,
                    row.get::<_, String>(24)?,
                    row.get(25)?,
                ),
                existing_install_detected: row.get::<_, i64>(26)? != 0,
                guided_install_available: row.get::<_, i64>(27)? != 0,
                first_seen_at: row.get(28)?,
                last_seen_at: row.get(29)?,
                updated_at: row.get(30)?,
                error_message: row.get(31)?,
                notes: parse_string_array(row.get::<_, String>(32)?),
                active_file_count: row.get(33)?,
                applied_file_count: row.get(34)?,
                review_file_count: row.get(35)?,
                sample_files: Vec::new(),
                queue_lane: DownloadQueueLane::ReadyNow,
                queue_summary: String::new(),
                family_key: None,
                related_item_ids: Vec::new(),
                timeline: Vec::new(),
                special_decision: None,
                version_resolution: None,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    log_slow_downloads_step("downloads_queue::query", query_started_at, || {
        format!(
            "for {} row(s) include_timelines={}",
            rows.len(),
            include_timelines
        )
    });

    let mut items = rows;
    let sample_started_at = Instant::now();
    let sample_names = load_item_sample_names_batch(
        connection,
        &items.iter().map(|item| item.id).collect::<Vec<_>>(),
    )?;
    for item in items.iter_mut() {
        item.sample_files = sample_names.get(&item.id).cloned().unwrap_or_default();
    }
    log_slow_downloads_step("downloads_queue::samples", sample_started_at, || {
        format!("for {} visible item(s)", items.len())
    });

    let enrich_started_at = Instant::now();
    enrich_download_items(
        connection,
        settings,
        seed_pack,
        &mut items,
        include_timelines,
    )?;
    log_slow_downloads_step("downloads_queue::enrich", enrich_started_at, || {
        format!("for {} visible item(s)", items.len())
    });
    overview.total_items = items.len() as i64;
    overview.ready_now_items = items
        .iter()
        .filter(|item| item.queue_lane == DownloadQueueLane::ReadyNow)
        .count() as i64;
    overview.special_setup_items = items
        .iter()
        .filter(|item| item.queue_lane == DownloadQueueLane::SpecialSetup)
        .count() as i64;
    overview.waiting_on_you_items = items
        .iter()
        .filter(|item| item.queue_lane == DownloadQueueLane::WaitingOnYou)
        .count() as i64;
    overview.blocked_items = items
        .iter()
        .filter(|item| item.queue_lane == DownloadQueueLane::Blocked)
        .count() as i64;
    overview.done_items = items
        .iter()
        .filter(|item| item.queue_lane == DownloadQueueLane::Done)
        .count() as i64;
    log_slow_downloads_operation("downloads_queue", started_at, items.len());

    Ok(DownloadsInboxResponse { overview, items })
}

fn enrich_download_items(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    items: &mut [DownloadsInboxItem],
    include_timelines: bool,
) -> AppResult<()> {
    let mut context = SpecialDecisionContext::default();
    for item in items.iter_mut() {
        hydrate_download_item(
            connection,
            settings,
            seed_pack,
            item,
            &mut context,
            false,
            false,
        )?;
    }

    let mut family_members: HashMap<String, Vec<i64>> = HashMap::new();
    for item in items.iter() {
        if let Some(family_key) = item.family_key.as_ref() {
            family_members
                .entry(family_key.clone())
                .or_default()
                .push(item.id);
        }
    }

    for item in items.iter_mut() {
        item.related_item_ids = item
            .special_decision
            .as_ref()
            .map(|decision| decision.sibling_item_ids.clone())
            .or_else(|| {
                item.family_key
                    .as_ref()
                    .and_then(|family_key| family_members.get(family_key))
                    .map(|ids| {
                        ids.iter()
                            .copied()
                            .filter(|related_id| *related_id != item.id)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();
        item.timeline = if include_timelines {
            build_download_timeline(connection, item)
        } else {
            Vec::new()
        };
    }

    Ok(())
}

fn should_load_special_decision(item: &DownloadsInboxItem) -> bool {
    item.intake_mode != DownloadIntakeMode::Standard
        || item.guided_install_available
        || item
            .matched_profile_key
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
        || item
            .special_family
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty())
}

fn hydrate_download_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item: &mut DownloadsInboxItem,
    context: &mut SpecialDecisionContext,
    allow_network_latest: bool,
    include_full_special_details: bool,
) -> AppResult<()> {
    item.version_resolution = None;
    item.special_decision = if should_load_special_decision(item) {
        install_profile_engine::build_special_mod_decision_cached(
            connection,
            settings,
            seed_pack,
            item.id,
            context,
            allow_network_latest,
            if include_full_special_details {
                SpecialDecisionDetailLevel::Full
            } else {
                SpecialDecisionDetailLevel::Queue
            },
        )?
    } else {
        None
    };
    if let Some(decision) = item.special_decision.as_ref() {
        if decision.apply_ready {
            item.intake_mode = DownloadIntakeMode::Guided;
            item.guided_install_available = true;
        }
        item.queue_lane = decision.queue_lane.clone();
        item.queue_summary = decision.queue_summary.clone();
        item.family_key = Some(decision.family_key.clone());
        return Ok(());
    }

    item.version_resolution = content_versions::resolve_download_item_version(
        connection,
        settings,
        seed_pack,
        item.id,
        if include_full_special_details {
            content_versions::CompareDetailLevel::Full
        } else {
            content_versions::CompareDetailLevel::Queue
        },
    )?;

    item.queue_lane = derive_queue_lane(item);
    item.queue_summary = build_queue_summary(item);
    item.family_key = build_family_key(item);

    Ok(())
}

fn derive_queue_lane(item: &DownloadsInboxItem) -> DownloadQueueLane {
    if matches!(item.status.as_str(), "applied" | "ignored") {
        return DownloadQueueLane::Done;
    }

    if item.status == "error" || item.intake_mode == DownloadIntakeMode::Blocked {
        return DownloadQueueLane::Blocked;
    }

    if item.intake_mode == DownloadIntakeMode::Guided {
        return DownloadQueueLane::SpecialSetup;
    }

    if item.intake_mode == DownloadIntakeMode::NeedsReview || item.status == "needs_review" {
        return DownloadQueueLane::WaitingOnYou;
    }

    DownloadQueueLane::ReadyNow
}

fn build_queue_summary(item: &DownloadsInboxItem) -> String {
    if let Some(resolution) = item.version_resolution.as_ref() {
        match resolution.status {
            crate::models::VersionCompareStatus::SameVersion => {
                return "SimSuite found a matching installed copy for this download.".to_owned();
            }
            crate::models::VersionCompareStatus::IncomingOlder => {
                return "The installed copy looks newer than this download.".to_owned();
            }
            crate::models::VersionCompareStatus::IncomingNewer => {
                return "This download looks newer than the installed copy.".to_owned();
            }
            crate::models::VersionCompareStatus::Unknown => {
                if !resolution
                    .matched_subject_label
                    .as_deref()
                    .unwrap_or("")
                    .is_empty()
                {
                    return "SimSuite found a possible installed match, but the version is still unclear."
                        .to_owned();
                }
            }
            crate::models::VersionCompareStatus::NotInstalled => {}
        }
    }

    match derive_queue_lane(item) {
        DownloadQueueLane::ReadyNow => {
            if item.review_file_count > 0 {
                "Safe files are ready to move, while the unsure ones stay visible for review."
                    .to_owned()
            } else {
                "This batch is ready for a safe hand-off into your library.".to_owned()
            }
        }
        DownloadQueueLane::SpecialSetup => {
            if !item.missing_dependencies.is_empty() {
                format!(
                    "Special setup found. {} needs to be handled first.",
                    item.missing_dependencies[0]
                )
            } else if item.guided_install_available {
                if item.existing_install_detected {
                    "SimSuite found an older setup and is ready to update it safely.".to_owned()
                } else {
                    "SimSuite recognized a supported special mod and has a safe install plan ready."
                        .to_owned()
                }
            } else if item.existing_install_detected {
                "SimSuite found an older setup and is still checking the safest update path."
                    .to_owned()
            } else {
                "Supported special setup found. Open the side panel for the safest next step."
                    .to_owned()
            }
        }
        DownloadQueueLane::WaitingOnYou => {
            if !item.inbox_dependencies.is_empty() {
                format!(
                    "Another Inbox item needs your attention first: {}.",
                    item.inbox_dependencies[0]
                )
            } else if !item.missing_dependencies.is_empty() {
                format!(
                    "This setup is waiting on a required helper: {}.",
                    item.missing_dependencies[0]
                )
            } else if item
                .catalog_source
                .as_ref()
                .and_then(|source| source.official_download_url.as_ref())
                .is_some()
            {
                "Important files are missing, but SimSuite can fetch the trusted official pack into the Inbox first."
                    .to_owned()
            } else {
                "This batch needs one more choice from you before anything moves.".to_owned()
            }
        }
        DownloadQueueLane::Blocked => item
            .error_message
            .clone()
            .or_else(|| item.incompatibility_warnings.first().cloned())
            .unwrap_or_else(|| "SimSuite stopped this batch to avoid a risky move.".to_owned()),
        DownloadQueueLane::Done => {
            if item.applied_file_count > 0 {
                "This batch already handed off its safe files.".to_owned()
            } else {
                "This batch is hidden from the active Inbox.".to_owned()
            }
        }
    }
}

fn build_family_key(item: &DownloadsInboxItem) -> Option<String> {
    if let Some(value) = item
        .special_family
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(format!("family:{}", normalize_family_token(value)));
    }

    if let Some(value) = item
        .matched_profile_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(format!("profile:{}", normalize_family_token(value)));
    }

    if item.intake_mode != DownloadIntakeMode::Standard {
        if let Some(value) = item
            .matched_profile_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(format!("profile-name:{}", normalize_family_token(value)));
        }
    }

    None
}

fn normalize_family_token(value: &str) -> String {
    let mut token = String::with_capacity(value.len());
    let mut last_was_dash = false;

    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            last_was_dash = false;
            Some(ch.to_ascii_lowercase())
        } else if last_was_dash {
            None
        } else {
            last_was_dash = true;
            Some('-')
        };

        if let Some(character) = mapped {
            token.push(character);
        }
    }

    token.trim_matches('-').to_owned()
}

fn load_related_item_ids(
    connection: &Connection,
    item: &DownloadsInboxItem,
) -> AppResult<Vec<i64>> {
    let query = if let Some(value) = item
        .special_family
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(("special_family", value.to_owned()))
    } else if let Some(value) = item
        .matched_profile_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(("matched_profile_key", value.to_owned()))
    } else {
        item.matched_profile_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| ("matched_profile_name", value.to_owned()))
    };

    let Some((column, value)) = query else {
        return Ok(Vec::new());
    };

    let sql = format!(
        "SELECT id
         FROM download_items
         WHERE id <> ?1
           AND {column} = ?2
           AND status <> 'ignored'
         ORDER BY updated_at DESC, id DESC"
    );

    let mut statement = connection.prepare(&sql)?;
    let ids = statement
        .query_map(params![item.id, value], |row| row.get(0))?
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(ids)
}

fn build_download_timeline(
    connection: &Connection,
    item: &DownloadsInboxItem,
) -> Vec<DownloadsTimelineEntry> {
    let mut timeline = vec![DownloadsTimelineEntry {
        label: "Added to Inbox".to_owned(),
        detail: Some(if item.source_kind == "archive" {
            format!(
                "Archive staged and {} file(s) were detected inside.",
                item.detected_file_count
            )
        } else {
            "Direct download staged for a safe check.".to_owned()
        }),
        at: Some(item.first_seen_at.clone()),
    }];

    if let Some(note) = item
        .notes
        .iter()
        .find(|note| note.starts_with(AUTO_RECHECK_NOTE_PREFIX))
        .cloned()
    {
        timeline.push(DownloadsTimelineEntry {
            label: "Rechecked".to_owned(),
            detail: Some(note),
            at: Some(item.updated_at.clone()),
        });
    }

    if item.existing_install_detected {
        timeline.push(DownloadsTimelineEntry {
            label: "Existing setup found".to_owned(),
            detail: Some(
                "SimSuite found matching files in your Mods folder and compared them before suggesting the next step."
                    .to_owned(),
            ),
            at: Some(item.updated_at.clone()),
        });
    }

    timeline.push(DownloadsTimelineEntry {
        label: match item.queue_lane {
            DownloadQueueLane::ReadyNow => "Ready now",
            DownloadQueueLane::SpecialSetup => "Special setup",
            DownloadQueueLane::WaitingOnYou => "Waiting on you",
            DownloadQueueLane::Blocked => "Blocked for safety",
            DownloadQueueLane::Done => {
                if item.applied_file_count > 0 {
                    "Installed safely"
                } else {
                    "Done"
                }
            }
        }
        .to_owned(),
        detail: Some(item.queue_summary.clone()),
        at: Some(item.updated_at.clone()),
    });

    if let Ok(mut events) = database::load_download_item_events(connection, item.id, 12) {
        timeline.append(&mut events);
    }

    timeline.sort_by(|left, right| right.at.cmp(&left.at));
    timeline
}

pub fn get_download_item_detail(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
) -> AppResult<Option<DownloadInboxDetail>> {
    let mut context = SpecialDecisionContext::default();
    get_download_item_detail_cached(
        connection,
        settings,
        seed_pack,
        item_id,
        &mut context,
        false,
        true,
    )
}

fn get_download_item_detail_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
    allow_network_latest: bool,
    include_full_special_details: bool,
) -> AppResult<Option<DownloadInboxDetail>> {
    let Some(item) = load_item_by_id_cached(
        connection,
        settings,
        seed_pack,
        item_id,
        context,
        allow_network_latest,
        include_full_special_details,
    )?
    else {
        return Ok(None);
    };

    let mut statement = connection.prepare(
        "SELECT
            f.id,
            f.filename,
            f.path,
            COALESCE(f.source_origin_path, di.source_path),
            f.archive_member_path,
            f.kind,
            f.subtype,
            c.canonical_name,
            f.confidence,
            f.size,
            f.source_location,
            f.safety_notes
         FROM files f
         LEFT JOIN creators c ON c.id = f.creator_id
         JOIN download_items di ON di.id = f.download_item_id
         WHERE f.download_item_id = ?1
         ORDER BY CASE WHEN f.source_location = 'downloads' THEN 0 ELSE 1 END,
                  f.filename COLLATE NOCASE",
    )?;
    let files = statement
        .query_map(params![item_id], |row| {
            Ok(DownloadInboxFile {
                file_id: row.get(0)?,
                filename: row.get(1)?,
                current_path: row.get(2)?,
                origin_path: row.get(3)?,
                archive_member_path: row.get(4)?,
                kind: row.get(5)?,
                subtype: row.get(6)?,
                creator: row.get(7)?,
                confidence: row.get(8)?,
                size: row.get(9)?,
                source_location: row.get(10)?,
                safety_notes: parse_string_array(row.get::<_, String>(11)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(DownloadInboxDetail { item, files }))
}

pub fn get_download_item_selection(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
    preset_name: Option<String>,
) -> AppResult<DownloadsSelectionResponse> {
    let started_at = Instant::now();
    let mut context = SpecialDecisionContext::default();
    let detail_started_at = Instant::now();
    let detail = get_download_item_detail_cached(
        connection,
        settings,
        seed_pack,
        item_id,
        &mut context,
        false,
        true,
    )?;
    log_slow_downloads_step("downloads_selection::detail", detail_started_at, || {
        format!("for item {}", item_id)
    });
    let mut preview = None;
    let mut guided_plan = None;
    let mut review_plan = None;

    if let Some(detail_item) = detail.as_ref().map(|value| &value.item) {
        if detail_item.intake_mode == DownloadIntakeMode::Standard
            && matches!(
                detail_item.status.as_str(),
                "ready" | "partial" | "needs_review"
            )
        {
            let preview_started_at = Instant::now();
            preview = Some(preview_download_item(
                connection,
                settings,
                seed_pack,
                item_id,
                preset_name.clone(),
            )?);
            log_slow_downloads_step("downloads_selection::preview", preview_started_at, || {
                format!("for item {}", item_id)
            });
        }

        if detail_item.intake_mode == DownloadIntakeMode::Guided {
            let guided_started_at = Instant::now();
            guided_plan = install_profile_engine::build_guided_plan_cached(
                connection,
                settings,
                seed_pack,
                item_id,
                &mut context,
            )?;
            log_slow_downloads_step(
                "downloads_selection::guided_plan",
                guided_started_at,
                || format!("for item {}", item_id),
            );
        }

        let should_load_review_plan = matches!(
            detail_item.intake_mode,
            DownloadIntakeMode::NeedsReview | DownloadIntakeMode::Blocked
        ) || (detail_item.intake_mode == DownloadIntakeMode::Guided
            && !detail_item
                .special_decision
                .as_ref()
                .is_some_and(|decision| decision.apply_ready));

        if should_load_review_plan {
            let review_started_at = Instant::now();
            review_plan = install_profile_engine::build_review_plan_cached(
                connection,
                settings,
                seed_pack,
                item_id,
                &mut context,
            )?;
            log_slow_downloads_step(
                "downloads_selection::review_plan",
                review_started_at,
                || format!("for item {}", item_id),
            );
        }
    }

    let file_count = detail.as_ref().map(|value| value.files.len()).unwrap_or(0);
    log_slow_downloads_operation("downloads_selection", started_at, file_count);

    Ok(DownloadsSelectionResponse {
        item_id,
        detail,
        preview,
        guided_plan,
        review_plan,
    })
}

pub fn preview_download_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
    preset_name: Option<String>,
) -> AppResult<OrganizationPreview> {
    let Some(item) = load_item_by_id(connection, settings, seed_pack, item_id)? else {
        return Err(AppError::Message("Inbox item was not found.".to_owned()));
    };
    if item.intake_mode != DownloadIntakeMode::Standard {
        return Err(AppError::Message(
            "This inbox item needs a guided special setup flow instead of the normal hand-off preview."
                .to_owned(),
        ));
    }

    let file_ids = load_active_file_ids(connection, item_id)?;
    if file_ids.is_empty() {
        return Err(AppError::Message(
            "This inbox item has no active files left to preview.".to_owned(),
        ));
    }

    rule_engine::build_preview_for_files(connection, settings, preset_name, &file_ids)
}

pub fn get_download_item_guided_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
) -> AppResult<Option<GuidedInstallPlan>> {
    install_profile_engine::build_guided_plan(connection, settings, seed_pack, item_id)
}

pub fn get_download_item_review_plan(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
) -> AppResult<Option<SpecialReviewPlan>> {
    install_profile_engine::build_review_plan(connection, settings, seed_pack, item_id)
}

pub fn ignore_download_item(connection: &mut Connection, item_id: i64) -> AppResult<()> {
    connection.execute(
        "DELETE FROM files
         WHERE download_item_id = ?1
           AND source_location = 'downloads'",
        params![item_id],
    )?;
    connection.execute(
        "UPDATE download_items
         SET status = 'ignored',
             error_message = NULL,
             updated_at = ?2
         WHERE id = ?1",
        params![item_id, Utc::now().to_rfc3339()],
    )?;
    bundle_detector::rebuild_bundles(connection)?;
    duplicate_detector::rebuild_duplicates(connection)?;
    Ok(())
}

pub fn refresh_download_item_status(connection: &Connection, item_id: i64) -> AppResult<()> {
    let Some((current_status, active_file_count, intake_mode)) = connection
        .query_row(
            "SELECT
                status,
                (
                    SELECT COUNT(*)
                    FROM files
                    WHERE download_item_id = ?1
                      AND source_location = 'downloads'
                ) AS active_file_count,
                COALESCE(intake_mode, 'standard')
             FROM download_items
             WHERE id = ?1",
            params![item_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(());
    };

    if current_status == "ignored" {
        return Ok(());
    }
    if current_status == "error" && active_file_count == 0 && intake_mode != "blocked" {
        return Ok(());
    }

    let status = derive_item_status(connection, item_id)?;
    connection.execute(
        "UPDATE download_items
         SET status = ?2,
             updated_at = ?3
         WHERE id = ?1",
        params![item_id, status, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn get_download_item_source(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Option<DownloadItemSourceRecord>> {
    connection
        .query_row(
            "SELECT id, display_name, source_path, source_kind, archive_format, source_size, source_modified_at, staging_path
             FROM download_items
             WHERE id = ?1",
            params![item_id],
            |row| {
                Ok(DownloadItemSourceRecord {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    source_path: row.get(2)?,
                    source_kind: row.get(3)?,
                    archive_format: row.get(4)?,
                    source_size: row.get(5)?,
                    source_modified_at: row.get(6)?,
                    staging_path: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

pub fn import_download_source(
    connection: &mut Connection,
    state: &AppState,
    source_path: &Path,
    display_name: Option<String>,
    existing_item_id: Option<i64>,
) -> AppResult<i64> {
    let category_overrides = database::list_category_overrides(connection)?
        .into_iter()
        .map(|item| (normalize_path_key(&item.match_path), item))
        .collect::<HashMap<_, _>>();
    let source = build_observed_source_from_path(source_path, display_name)?;
    let watched_root = source_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let existing_item = match existing_item_id {
        Some(item_id) => load_existing_download_item(connection, item_id)?,
        None => None,
    };
    let mut special_context = SpecialDecisionContext::default();

    process_source(
        connection,
        state,
        &watched_root,
        &state.seed_pack,
        &category_overrides,
        &source,
        existing_item.as_ref(),
        &mut special_context,
    )
}

pub fn import_staged_batch(
    connection: &mut Connection,
    state: &AppState,
    source: &DownloadItemSourceRecord,
    staging_root: &Path,
    display_name: String,
    existing_item_id: Option<i64>,
    notes: Vec<String>,
) -> AppResult<i64> {
    let category_overrides = database::list_category_overrides(connection)?
        .into_iter()
        .map(|item| (normalize_path_key(&item.match_path), item))
        .collect::<HashMap<_, _>>();
    let discovered = WalkDir::new(staging_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| build_discovered_file(staging_root, entry.path()))
        .collect::<AppResult<Vec<_>>>()?;
    let observed = ObservedSource {
        path: PathBuf::from(&source.source_path),
        display_name,
        source_kind: source.source_kind.clone(),
        archive_format: source.archive_format.clone(),
        source_size: source.source_size,
        source_modified_at: source.source_modified_at.clone(),
    };
    let mut special_context = SpecialDecisionContext::default();

    ingest_processed_source(
        connection,
        &state.seed_pack,
        &category_overrides,
        &observed,
        existing_item_id,
        discovered,
        Some(staging_root),
        &notes,
        &mut special_context,
    )
}

pub fn load_active_file_ids(connection: &Connection, item_id: i64) -> AppResult<Vec<i64>> {
    let mut statement = connection.prepare(
        "SELECT id
         FROM files
         WHERE download_item_id = ?1
           AND source_location = 'downloads'
         ORDER BY filename COLLATE NOCASE",
    )?;
    let file_ids = statement
        .query_map(params![item_id], |row| row.get(0))?
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(file_ids)
}

fn watch_loop(app: AppHandle, state: AppState, watched_root: PathBuf, stop: mpsc::Receiver<()>) {
    let (event_tx, event_rx) = mpsc::channel();

    let mut watcher = match recommended_watcher(move |result| {
        let _ = event_tx.send(result);
    }) {
        Ok(watcher) => watcher,
        Err(error) => {
            let _ = store_status(
                &state,
                &app,
                DownloadsWatcherStatus {
                    state: DownloadsWatcherState::Error,
                    watched_path: Some(watched_root.to_string_lossy().to_string()),
                    configured: true,
                    current_item: None,
                    last_run_at: None,
                    last_change_at: None,
                    last_error: Some(error.to_string()),
                    ready_items: 0,
                    needs_review_items: 0,
                    active_items: 0,
                },
            );
            return;
        }
    };

    if let Err(error) = watcher.watch(&watched_root, RecursiveMode::Recursive) {
        let _ = store_status(
            &state,
            &app,
            DownloadsWatcherStatus {
                state: DownloadsWatcherState::Error,
                watched_path: Some(watched_root.to_string_lossy().to_string()),
                configured: true,
                current_item: None,
                last_run_at: None,
                last_change_at: None,
                last_error: Some(error.to_string()),
                ready_items: 0,
                needs_review_items: 0,
                active_items: 0,
            },
        );
        return;
    }

    if let Err(error) = process_downloads_once(
        &app,
        &state,
        Some("Initial inbox refresh".to_owned()),
        false,
    ) {
        let fallback = state
            .downloads_status()
            .lock()
            .map(|status| DownloadsWatcherStatus {
                state: DownloadsWatcherState::Error,
                watched_path: status.watched_path.clone(),
                configured: status.configured,
                current_item: Some("Initial inbox refresh".to_owned()),
                last_run_at: status.last_run_at.clone(),
                last_change_at: status.last_change_at.clone(),
                last_error: Some(error.to_string()),
                ready_items: status.ready_items,
                needs_review_items: status.needs_review_items,
                active_items: status.active_items,
            })
            .unwrap_or(DownloadsWatcherStatus {
                state: DownloadsWatcherState::Error,
                watched_path: Some(watched_root.to_string_lossy().to_string()),
                configured: true,
                current_item: Some("Initial inbox refresh".to_owned()),
                last_run_at: None,
                last_change_at: None,
                last_error: Some(error.to_string()),
                ready_items: 0,
                needs_review_items: 0,
                active_items: 0,
            });
        let _ = store_status(&state, &app, fallback);
    }

    loop {
        if stop.try_recv().is_ok() {
            break;
        }

        match event_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
                let mut changed_paths = event.paths.clone();
                let current_item = event
                    .paths
                    .first()
                    .map(|path| {
                        path.file_name()
                            .map(|value| value.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.to_string_lossy().to_string())
                    })
                    .unwrap_or_else(|| "Downloads update".to_owned());

                thread::sleep(Duration::from_millis(WATCHER_DEBOUNCE_MS));
                while let Ok(next_event) = event_rx.try_recv() {
                    if let Ok(next_event) = next_event {
                        changed_paths.extend(next_event.paths);
                    }
                }

                let changed_paths = dedupe_changed_paths(changed_paths);
                let _ = process_downloads_once_for_paths(
                    &app,
                    &state,
                    Some(current_item),
                    false,
                    Some(&changed_paths),
                );
            }
            Ok(Err(error)) => {
                let current = state.downloads_status();
                let snapshot = current
                    .lock()
                    .map(|status| DownloadsWatcherStatus {
                        state: DownloadsWatcherState::Error,
                        watched_path: status.watched_path.clone(),
                        configured: status.configured,
                        current_item: None,
                        last_run_at: status.last_run_at.clone(),
                        last_change_at: status.last_change_at.clone(),
                        last_error: Some(error.to_string()),
                        ready_items: status.ready_items,
                        needs_review_items: status.needs_review_items,
                        active_items: status.active_items,
                    })
                    .unwrap_or_default();
                let _ = store_status(&state, &app, snapshot);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn process_downloads_once(
    app: &AppHandle,
    state: &AppState,
    current_item: Option<String>,
    manual: bool,
) -> AppResult<DownloadsWatcherStatus> {
    process_downloads_once_for_paths(app, state, current_item, manual, None)
}

fn process_downloads_once_for_paths(
    app: &AppHandle,
    state: &AppState,
    current_item: Option<String>,
    manual: bool,
    changed_paths: Option<&[PathBuf]>,
) -> AppResult<DownloadsWatcherStatus> {
    let started_at = Instant::now();
    let processing_lock = state.downloads_processing_lock();
    let _processing_guard = processing_lock
        .lock()
        .map_err(|_| AppError::Message("Downloads processing lock poisoned".to_owned()))?;

    let connection_started_at = Instant::now();
    let mut connection = state.connection()?;
    let settings = database::get_library_settings(&connection)?;
    log_slow_downloads_step("downloads_sync::connection", connection_started_at, || {
        "opened database connection and loaded settings".to_owned()
    });
    let Some(downloads_path) = settings
        .downloads_path
        .clone()
        .filter(|value| !value.trim().is_empty())
    else {
        let status = DownloadsWatcherStatus::default();
        store_status(state, app, status.clone())?;
        return Ok(status);
    };

    let watched_root = PathBuf::from(downloads_path.trim());
    if !watched_root.exists() {
        let status = DownloadsWatcherStatus {
            state: DownloadsWatcherState::Error,
            watched_path: Some(watched_root.to_string_lossy().to_string()),
            configured: true,
            current_item,
            last_run_at: Some(Utc::now().to_rfc3339()),
            last_change_at: None,
            last_error: Some("Downloads folder does not exist.".to_owned()),
            ready_items: 0,
            needs_review_items: 0,
            active_items: 0,
        };
        store_status(state, app, status.clone())?;
        return Ok(status);
    }

    store_status(
        state,
        app,
        checking_downloads_status(&watched_root, current_item.clone()),
    )?;

    let assessment_started_at = Instant::now();
    let base_seed = state.seed_pack();
    let assessment_version = current_downloads_assessment_version(base_seed.as_ref());
    let assessment_version_changed =
        database::get_app_setting(&connection, "downloads_assessment_version")?.as_deref()
            != Some(assessment_version.as_str());
    let runtime_seed_pack = database::load_runtime_seed_pack(&connection, base_seed.as_ref())?;
    let category_overrides = database::list_category_overrides(&connection)?
        .into_iter()
        .map(|item| (normalize_path_key(&item.match_path), item))
        .collect::<HashMap<_, _>>();
    log_slow_downloads_step("downloads_sync::setup", assessment_started_at, || {
        format!(
            "assessment_version_changed={} category_overrides={}",
            assessment_version_changed,
            category_overrides.len()
        )
    });

    let observe_started_at = Instant::now();
    let use_full_scan = should_use_full_downloads_scan(
        &watched_root,
        manual,
        assessment_version_changed,
        changed_paths,
    );
    let observed = if use_full_scan {
        collect_observed_sources(&watched_root)?
    } else {
        collect_observed_sources_for_paths(&watched_root, changed_paths.unwrap_or_default())?
    };
    log_slow_downloads_step("downloads_sync::observe", observe_started_at, || {
        format!(
            "use_full_scan={} observed_sources={}",
            use_full_scan,
            observed.len()
        )
    });

    let existing_started_at = Instant::now();
    let existing = load_existing_items(&connection)?;
    log_slow_downloads_step("downloads_sync::existing", existing_started_at, || {
        format!("existing_items={}", existing.len())
    });
    let mut changed = false;
    let mut reassessed_existing = false;
    let should_reassess_unchanged = manual || assessment_version_changed;
    let mut unchanged_items = 0_usize;
    let mut processed_items = 0_usize;
    let mut special_context = SpecialDecisionContext::default();

    let source_loop_started_at = Instant::now();
    for source in &observed {
        let key = normalize_path_key(&source.path.to_string_lossy());
        let existing_item = existing.get(&key);
        let unchanged = existing_item.is_some_and(|item| can_skip_observed_source(item, source));

        if unchanged {
            let existing_item = existing_item.expect("existing item");
            update_last_seen(&connection, existing_item.id)?;
            unchanged_items += 1;
            // Version bumps need a true rebuild so stored file clues refresh too.
            if assessment_version_changed {
                process_source(
                    &mut connection,
                    state,
                    &watched_root,
                    &runtime_seed_pack,
                    &category_overrides,
                    source,
                    Some(existing_item),
                    &mut special_context,
                )?;
                mark_item_rechecked_with_new_rules(&connection, existing_item.id)?;
                changed = true;
                processed_items += 1;
                continue;
            }
            if should_reassess_unchanged {
                reassess_existing_item(
                    &connection,
                    &settings,
                    &runtime_seed_pack,
                    existing_item.id,
                    &mut special_context,
                )?;
                reassessed_existing = true;
            }
            continue;
        }

        process_source(
            &mut connection,
            state,
            &watched_root,
            &runtime_seed_pack,
            &category_overrides,
            source,
            existing_item,
            &mut special_context,
        )?;
        changed = true;
        processed_items += 1;
    }
    log_slow_downloads_step("downloads_sync::sources", source_loop_started_at, || {
        format!(
            "processed={} unchanged={} reassessed_existing={}",
            processed_items, unchanged_items, reassessed_existing
        )
    });

    let missing_started_at = Instant::now();
    let missing_changed = if use_full_scan {
        mark_missing_direct_sources(&connection, &existing, &observed)?
    } else if let Some(paths) = changed_paths {
        mark_missing_direct_sources_for_paths(&connection, &existing, paths)?
    } else {
        false
    };
    changed |= missing_changed;
    log_slow_downloads_step("downloads_sync::missing", missing_started_at, || {
        format!("missing_changed={missing_changed}")
    });

    if changed {
        let bundles_started_at = Instant::now();
        bundle_detector::rebuild_bundles(&mut connection)?;
        log_slow_downloads_step("downloads_sync::bundles", bundles_started_at, || {
            "rebuilt tray bundles".to_owned()
        });

        let duplicates_started_at = Instant::now();
        duplicate_detector::rebuild_duplicates(&mut connection)?;
        log_slow_downloads_step("downloads_sync::duplicates", duplicates_started_at, || {
            "rebuilt duplicate indexes".to_owned()
        });
    }

    let statuses_started_at = Instant::now();
    recompute_item_statuses(&connection)?;
    log_slow_downloads_step("downloads_sync::statuses", statuses_started_at, || {
        "recomputed item statuses".to_owned()
    });
    if changed || reassessed_existing || assessment_version_changed {
        let version_save_started_at = Instant::now();
        database::save_app_setting(
            &mut connection,
            "downloads_assessment_version",
            Some(&assessment_version),
            "seed",
        )?;
        log_slow_downloads_step(
            "downloads_sync::assessment_version",
            version_save_started_at,
            || "saved downloads assessment version".to_owned(),
        );
    }

    let summarize_started_at = Instant::now();
    let status = summarize_status(
        &connection,
        Some(watched_root.to_string_lossy().to_string()),
    )?;
    log_slow_downloads_step("downloads_sync::summarize", summarize_started_at, || {
        format!(
            "ready={} needs_review={} active={}",
            status.ready_items, status.needs_review_items, status.active_items
        )
    });
    drop(connection);

    log_slow_downloads_operation("downloads_sync", started_at, observed.len());
    store_status(state, app, status.clone())?;
    let _ = emit_workspace_change(
        app,
        &WorkspaceChange {
            domains: vec![WorkspaceDomain::Downloads],
            reason: "downloads-sync-finished".to_owned(),
            item_ids: Vec::new(),
            family_keys: Vec::new(),
        },
    );
    if changed || reassessed_existing || assessment_version_changed {
        let _ = emit_workspace_change(
            app,
            &WorkspaceChange {
                domains: vec![
                    WorkspaceDomain::Home,
                    WorkspaceDomain::Review,
                    WorkspaceDomain::Duplicates,
                ],
                reason: "downloads-processed".to_owned(),
                item_ids: Vec::new(),
                family_keys: Vec::new(),
            },
        );
    }
    Ok(status)
}

fn current_downloads_assessment_version(seed_pack: &crate::seed::SeedPack) -> String {
    format!(
        "{DOWNLOADS_ASSESSMENT_VERSION_PREFIX}:{}:{}",
        seed_pack.seed_version, seed_pack.install_catalog.seed_version
    )
}

fn process_source(
    connection: &mut Connection,
    state: &AppState,
    watched_root: &Path,
    seed_pack: &crate::seed::SeedPack,
    category_overrides: &HashMap<String, database::UserCategoryOverride>,
    source: &ObservedSource,
    existing: Option<&ExistingDownloadItem>,
    special_context: &mut SpecialDecisionContext,
) -> AppResult<i64> {
    if should_hold_archive_for_safety(source) {
        return ingest_held_archive_source(connection, source, existing.map(|item| item.id));
    }

    let mut notes = Vec::new();
    let mut staged_root = None;
    let discovered = if source.source_kind == "file" {
        vec![build_discovered_file(watched_root, &source.path)?]
    } else if !should_extract_archive_source(source, &mut notes)? {
        Vec::new()
    } else {
        let next_root = build_archive_staging_root(
            &state.app_data_dir,
            existing.map(|item| item.id),
            &source.display_name,
            Utc::now(),
        );
        let extracted = extract_archive(source, &next_root, &mut notes)?;
        staged_root = Some(next_root);
        extracted
    };

    ingest_processed_source(
        connection,
        seed_pack,
        category_overrides,
        source,
        existing.map(|item| item.id),
        discovered,
        staged_root.as_deref(),
        &notes,
        special_context,
    )
}

fn staging_segment_for_source(display_name: &str) -> String {
    let mut sanitized = display_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized = sanitized.trim_matches('-').to_owned();
    if sanitized.is_empty() {
        "download".to_owned()
    } else {
        sanitized.chars().take(48).collect()
    }
}

fn build_archive_staging_root(
    app_data_dir: &Path,
    existing_item_id: Option<i64>,
    display_name: &str,
    now: DateTime<Utc>,
) -> PathBuf {
    app_data_dir
        .join("downloads_inbox")
        .join(
            existing_item_id
                .map(|item_id| item_id.to_string())
                .unwrap_or_else(|| "new".to_owned()),
        )
        .join(format!(
            "{}-{}",
            now.format("%Y%m%d%H%M%S%f"),
            staging_segment_for_source(display_name)
        ))
}

fn should_hold_archive_for_safety(source: &ObservedSource) -> bool {
    matches!(source.archive_format.as_deref(), Some("7z") | Some("rar"))
}

fn held_archive_reason(source: &ObservedSource) -> String {
    let archive_label = match source.archive_format.as_deref() {
        Some("7z") => ".7z",
        Some("rar") => ".rar",
        _ => "this archive",
    };
    format!("SimSuite paused {archive_label} extraction until a safer handler is in place.")
}

fn held_archive_assessment(reason: &str) -> DownloadItemAssessment {
    DownloadItemAssessment {
        intake_mode: DownloadIntakeMode::Blocked,
        risk_level: DownloadRiskLevel::High,
        matched_profile_key: None,
        matched_profile_name: None,
        special_family: None,
        assessment_reasons: vec![reason.to_owned()],
        dependency_summary: Vec::new(),
        missing_dependencies: Vec::new(),
        inbox_dependencies: Vec::new(),
        incompatibility_warnings: Vec::new(),
        post_install_notes: Vec::new(),
        evidence_summary: vec![HELD_ARCHIVE_SAFETY_NOTE.to_owned()],
        catalog_source: None,
        existing_install_detected: false,
        guided_install_available: false,
    }
}

fn ignored_non_sims_download_assessment() -> DownloadItemAssessment {
    DownloadItemAssessment {
        intake_mode: DownloadIntakeMode::Standard,
        risk_level: DownloadRiskLevel::Low,
        matched_profile_key: None,
        matched_profile_name: None,
        special_family: None,
        assessment_reasons: Vec::new(),
        dependency_summary: Vec::new(),
        missing_dependencies: Vec::new(),
        inbox_dependencies: Vec::new(),
        incompatibility_warnings: Vec::new(),
        post_install_notes: Vec::new(),
        evidence_summary: vec![IGNORED_NON_SIMS_DOWNLOAD_NOTE.to_owned()],
        catalog_source: None,
        existing_install_detected: false,
        guided_install_available: false,
    }
}

fn ingest_held_archive_source(
    connection: &mut Connection,
    source: &ObservedSource,
    existing_item_id: Option<i64>,
) -> AppResult<i64> {
    let item_id = upsert_download_item(connection, source, existing_item_id)?;
    let now = Utc::now().to_rfc3339();
    let reason = held_archive_reason(source);
    let notes = vec![
        HELD_ARCHIVE_SAFETY_NOTE.to_owned(),
        "This Inbox item was held before extraction, so SimSuite did not unpack any archive contents."
            .to_owned(),
    ];

    connection.execute(
        "UPDATE files
         SET download_item_id = NULL
         WHERE download_item_id = ?1
           AND source_location <> 'downloads'",
        params![item_id],
    )?;
    connection.execute(
        "DELETE FROM files
         WHERE download_item_id = ?1
           AND source_location = 'downloads'",
        params![item_id],
    )?;

    let assessment = held_archive_assessment(&reason);
    install_profile_engine::store_download_item_assessment(connection, item_id, &assessment)?;
    connection.execute(
        "UPDATE download_items
         SET status = 'needs_review',
             error_message = ?2,
             notes = ?3,
             detected_file_count = 0,
             staging_path = NULL,
             updated_at = ?4,
             last_seen_at = ?4
         WHERE id = ?1",
        params![item_id, reason, serde_json::to_string(&notes)?, now],
    )?;
    Ok(item_id)
}

fn ingest_ignored_non_sims_source(
    connection: &mut Connection,
    source: &ObservedSource,
    existing_item_id: Option<i64>,
    notes: &[String],
) -> AppResult<i64> {
    let item_id = upsert_download_item(connection, source, existing_item_id)?;
    let now = Utc::now().to_rfc3339();
    let mut item_notes = vec![IGNORED_NON_SIMS_DOWNLOAD_NOTE.to_owned()];
    for note in notes {
        if item_notes.iter().any(|existing| existing == note) {
            continue;
        }
        item_notes.push(note.clone());
    }

    connection.execute(
        "UPDATE files
         SET download_item_id = NULL
         WHERE download_item_id = ?1
           AND source_location <> 'downloads'",
        params![item_id],
    )?;
    connection.execute(
        "DELETE FROM files
         WHERE download_item_id = ?1
           AND source_location = 'downloads'",
        params![item_id],
    )?;

    let assessment = ignored_non_sims_download_assessment();
    install_profile_engine::store_download_item_assessment(connection, item_id, &assessment)?;
    connection.execute(
        "UPDATE download_items
         SET status = 'ignored',
             error_message = NULL,
             notes = ?2,
             detected_file_count = 0,
             staging_path = NULL,
             updated_at = ?3,
             last_seen_at = ?3
         WHERE id = ?1",
        params![item_id, serde_json::to_string(&item_notes)?, now],
    )?;
    Ok(item_id)
}

fn ingest_processed_source(
    connection: &mut Connection,
    seed_pack: &crate::seed::SeedPack,
    category_overrides: &HashMap<String, database::UserCategoryOverride>,
    source: &ObservedSource,
    existing_item_id: Option<i64>,
    discovered: Vec<DiscoveredFile>,
    staged_root: Option<&Path>,
    notes: &[String],
    special_context: &mut SpecialDecisionContext,
) -> AppResult<i64> {
    let item_id = upsert_download_item(connection, source, existing_item_id)?;

    connection.execute(
        "UPDATE files
         SET download_item_id = NULL
         WHERE download_item_id = ?1
           AND source_location <> 'downloads'",
        params![item_id],
    )?;
    connection.execute(
        "DELETE FROM files
         WHERE download_item_id = ?1
           AND source_location = 'downloads'",
        params![item_id],
    )?;

    if discovered.is_empty() {
        if source.source_kind == "archive" {
            return ingest_ignored_non_sims_source(connection, source, Some(item_id), notes);
        }
        connection.execute(
            "UPDATE download_items
             SET status = 'error',
                 error_message = ?2,
                 notes = ?3,
                 detected_file_count = 0,
                 staging_path = ?4,
                 updated_at = ?5,
                 last_seen_at = ?5
             WHERE id = ?1",
            params![
                item_id,
                "No supported Sims files were found in this download.",
                serde_json::to_string(notes)?,
                staged_root.map(|value| value.to_string_lossy().to_string()),
                Utc::now().to_rfc3339()
            ],
        )?;
        return Ok(item_id);
    }

    let transaction = connection.transaction()?;
    {
        let mut creator_cache = HashMap::new();
        let mut file_insert = transaction.prepare(
            "INSERT INTO files (
                path, filename, extension, hash, size, created_at, modified_at,
                creator_id, kind, subtype, confidence, source_location,
                scan_session_id, relative_depth, safety_notes, parser_warnings, insights
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        )?;
        let mut review_insert = transaction.prepare(
            "INSERT OR IGNORE INTO review_queue (file_id, reason, confidence)
             VALUES (?1, ?2, ?3)",
        )?;
        let origin_path = source.path.to_string_lossy().to_string();

        for file in &discovered {
            let archive_member_path = if source.source_kind == "archive" {
                file.path
                    .strip_prefix(file.root_path.as_path())
                    .ok()
                    .map(|value| value.to_string_lossy().replace('\\', "/"))
            } else {
                None
            };
            let hash = Some(scanner::hash_file(&file.path)?);
            scanner::insert_parsed_file(
                &transaction,
                &mut creator_cache,
                seed_pack,
                category_overrides,
                &mut file_insert,
                &mut review_insert,
                None,
                file,
                hash,
            )?;
            transaction.execute(
                "UPDATE files
                 SET download_item_id = ?1,
                     source_origin_path = ?2,
                     archive_member_path = ?3
                 WHERE path = ?4",
                params![
                    item_id,
                    &origin_path,
                    archive_member_path,
                    file.path.to_string_lossy().to_string()
                ],
            )?;
        }
    }
    transaction.commit()?;

    connection.execute(
        "UPDATE download_items
         SET staging_path = ?2,
             detected_file_count = ?3,
             notes = ?4,
             error_message = NULL,
             status = 'pending',
             updated_at = ?5,
             last_seen_at = ?5
         WHERE id = ?1",
        params![
            item_id,
            staged_root.map(|value| value.to_string_lossy().to_string()),
            discovered.len() as i64,
            serde_json::to_string(notes)?,
            Utc::now().to_rfc3339()
        ],
    )?;

    let assessment = install_profile_engine::assess_download_item_cached(
        connection,
        &database::get_library_settings(connection)?,
        seed_pack,
        item_id,
        special_context,
    )?;
    install_profile_engine::store_download_item_assessment(connection, item_id, &assessment)?;

    Ok(item_id)
}

fn reassess_existing_item(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
    special_context: &mut SpecialDecisionContext,
) -> AppResult<()> {
    let assessment = install_profile_engine::assess_download_item_cached(
        connection,
        settings,
        seed_pack,
        item_id,
        special_context,
    )?;
    install_profile_engine::store_download_item_assessment(connection, item_id, &assessment)?;
    Ok(())
}

fn mark_item_rechecked_with_new_rules(connection: &Connection, item_id: i64) -> AppResult<()> {
    let existing_notes = connection
        .query_row(
            "SELECT notes FROM download_items WHERE id = ?1",
            params![item_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    let Some(existing_notes) = existing_notes else {
        return Ok(());
    };

    let mut notes = parse_string_array(existing_notes);
    notes.retain(|note| !note.starts_with(AUTO_RECHECK_NOTE_PREFIX));
    notes.push(format!(
        "{AUTO_RECHECK_NOTE_PREFIX} on {}.",
        Utc::now().format("%b %-d, %Y")
    ));

    connection.execute(
        "UPDATE download_items
         SET notes = ?2,
             updated_at = ?3
         WHERE id = ?1",
        params![
            item_id,
            serde_json::to_string(&notes)?,
            Utc::now().to_rfc3339()
        ],
    )?;

    Ok(())
}

fn upsert_download_item(
    connection: &Connection,
    source: &ObservedSource,
    existing_id: Option<i64>,
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();
    let source_path = source.path.to_string_lossy().to_string();

    if let Some(existing_id) = existing_id {
        connection.execute(
            "UPDATE download_items
             SET display_name = ?2,
                 source_kind = ?3,
                 archive_format = ?4,
                 source_size = ?5,
                 source_modified_at = ?6,
                 updated_at = ?7,
                 last_seen_at = ?7
             WHERE id = ?1",
            params![
                existing_id,
                source.display_name,
                source.source_kind,
                source.archive_format,
                source.source_size,
                source.source_modified_at,
                now
            ],
        )?;
        return Ok(existing_id);
    }

    connection.execute(
        "INSERT INTO download_items (
            source_path,
            display_name,
            source_kind,
            archive_format,
            source_size,
            source_modified_at,
            first_seen_at,
            last_seen_at,
            updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?7)",
        params![
            source_path,
            source.display_name,
            source.source_kind,
            source.archive_format,
            source.source_size,
            source.source_modified_at,
            now
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

fn collect_observed_sources(root: &Path) -> AppResult<Vec<ObservedSource>> {
    let mut observed = Vec::new();
    for entry in WalkDir::new(root).into_iter() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                return Err(AppError::Message(error.to_string()));
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        let extension = normalize_extension(entry.path());
        if !is_observable_download_extension(&extension) {
            continue;
        }

        let metadata = entry
            .metadata()
            .map_err(|error| AppError::Message(error.to_string()))?;
        observed.push(ObservedSource {
            path: entry.path().to_path_buf(),
            display_name: entry
                .path()
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| entry.path().to_string_lossy().to_string()),
            source_kind: if is_archive_extension(&extension) {
                "archive".to_owned()
            } else {
                "file".to_owned()
            },
            archive_format: archive_format_for_extension(&extension),
            source_size: metadata.len() as i64,
            source_modified_at: metadata.modified().ok().map(system_time_to_rfc3339),
        });
    }

    observed.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(observed)
}

fn should_use_full_downloads_scan(
    watched_root: &Path,
    manual: bool,
    assessment_version_changed: bool,
    changed_paths: Option<&[PathBuf]>,
) -> bool {
    if manual || assessment_version_changed {
        return true;
    }
    let Some(changed_paths) = changed_paths else {
        return true;
    };
    if changed_paths.is_empty() || changed_paths.len() > 24 {
        return true;
    }
    changed_paths.iter().any(|path| {
        path.as_os_str().is_empty()
            || path == watched_root
            || !path.starts_with(watched_root)
            || path.is_dir()
    })
}

fn collect_observed_sources_for_paths(
    watched_root: &Path,
    changed_paths: &[PathBuf],
) -> AppResult<Vec<ObservedSource>> {
    let mut observed = Vec::new();
    let mut seen = HashSet::new();

    for path in changed_paths {
        if path.as_os_str().is_empty() || !path.starts_with(watched_root) || !path.exists() {
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let extension = normalize_extension(path);
        if !is_observable_download_extension(&extension) {
            continue;
        }
        let key = normalize_path_key(&path.to_string_lossy());
        if !seen.insert(key) {
            continue;
        }
        observed.push(build_observed_source_from_path(path, None)?);
    }

    observed.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(observed)
}

fn build_observed_source_from_path(
    source_path: &Path,
    display_name: Option<String>,
) -> AppResult<ObservedSource> {
    let metadata = source_path.metadata()?;
    let extension = normalize_extension(source_path);
    Ok(ObservedSource {
        path: source_path.to_path_buf(),
        display_name: display_name.unwrap_or_else(|| {
            source_path
                .file_name()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| source_path.to_string_lossy().to_string())
        }),
        source_kind: if is_archive_extension(&extension) {
            "archive".to_owned()
        } else {
            "file".to_owned()
        },
        archive_format: archive_format_for_extension(&extension),
        source_size: metadata.len() as i64,
        source_modified_at: metadata.modified().ok().map(system_time_to_rfc3339),
    })
}

fn load_existing_items(
    connection: &Connection,
) -> AppResult<HashMap<String, ExistingDownloadItem>> {
    let mut statement = connection.prepare(
        "SELECT
            di.id,
            di.source_path,
            di.source_size,
            di.source_modified_at,
            di.status,
            di.source_kind,
            (
                SELECT COUNT(*)
                FROM files f
                WHERE f.download_item_id = di.id
                  AND f.source_location = 'downloads'
            ) AS active_file_count
         FROM download_items di",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExistingDownloadItem {
                id: row.get(0)?,
                source_path: row.get(1)?,
                source_size: row.get(2)?,
                source_modified_at: row.get(3)?,
                status: row.get(4)?,
                source_kind: row.get(5)?,
                active_file_count: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(|item| (normalize_path_key(&item.source_path), item))
        .collect())
}

fn load_existing_download_item(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Option<ExistingDownloadItem>> {
    connection
        .query_row(
            "SELECT
                di.id,
                di.source_path,
                di.source_size,
                di.source_modified_at,
                di.status,
                di.source_kind,
                (
                    SELECT COUNT(*)
                    FROM files f
                    WHERE f.download_item_id = di.id
                      AND f.source_location = 'downloads'
                ) AS active_file_count
             FROM download_items di
             WHERE di.id = ?1",
            params![item_id],
            |row| {
                Ok(ExistingDownloadItem {
                    id: row.get(0)?,
                    source_path: row.get(1)?,
                    source_size: row.get(2)?,
                    source_modified_at: row.get(3)?,
                    status: row.get(4)?,
                    source_kind: row.get(5)?,
                    active_file_count: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn can_skip_observed_source(existing: &ExistingDownloadItem, source: &ObservedSource) -> bool {
    if existing.source_size != source.source_size
        || existing.source_modified_at != source.source_modified_at
    {
        return false;
    }

    if existing.status == "ignored" {
        return true;
    }

    existing.active_file_count > 0
}

fn build_discovered_file(root_path: &Path, path: &Path) -> AppResult<DiscoveredFile> {
    let metadata = path.metadata()?;
    let relative_depth = path
        .strip_prefix(root_path)
        .ok()
        .and_then(|relative| relative.parent().map(|parent| parent.components().count()))
        .unwrap_or(0);

    Ok(DiscoveredFile {
        root_path: root_path.to_path_buf(),
        source_location: "downloads".to_owned(),
        path: path.to_path_buf(),
        filename: path
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        extension: normalize_extension(path),
        size: metadata.len() as i64,
        created_at: metadata.created().ok().map(system_time_to_rfc3339),
        modified_at: metadata.modified().ok().map(system_time_to_rfc3339),
        relative_depth: relative_depth as i64,
    })
}

fn extract_archive(
    source: &ObservedSource,
    destination_root: &Path,
    notes: &mut Vec<String>,
) -> AppResult<Vec<DiscoveredFile>> {
    fs::create_dir_all(destination_root)?;

    match source.archive_format.as_deref() {
        Some("zip") => extract_zip_archive(&source.path, destination_root, notes)?,
        Some("7z") => {
            sevenz_rust::decompress_file(&source.path, destination_root)
                .map_err(|error| AppError::Message(error.to_string()))?;
        }
        Some("rar") => {
            rar::Archive::extract_all(
                &source.path.to_string_lossy(),
                &destination_root.to_string_lossy(),
                "",
            )
            .map_err(|error| AppError::Message(error.to_string()))?;
        }
        _ => {
            return Err(AppError::Message("Unsupported archive format.".to_owned()));
        }
    }

    let mut discovered = Vec::new();
    let mut ignored_entries = 0_i64;
    for entry in WalkDir::new(destination_root).into_iter() {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => return Err(AppError::Message(error.to_string())),
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let extension = normalize_extension(entry.path());
        if !is_supported_content_extension(&extension) {
            ignored_entries += 1;
            continue;
        }
        discovered.push(build_discovered_file(destination_root, entry.path())?);
    }

    if ignored_entries > 0 {
        notes.push(format!(
            "Ignored {ignored_entries} unsupported archive entries."
        ));
    }

    Ok(discovered)
}

fn should_extract_archive_source(
    source: &ObservedSource,
    notes: &mut Vec<String>,
) -> AppResult<bool> {
    if !matches!(source.archive_format.as_deref(), Some("zip")) {
        return Ok(true);
    }

    if zip_archive_contains_supported_content(&source.path)? {
        return Ok(true);
    }

    notes.push(
        "Skipped ZIP extraction because no supported Sims files were found inside.".to_owned(),
    );
    Ok(false)
}

fn zip_archive_contains_supported_content(source_path: &Path) -> AppResult<bool> {
    let file = fs::File::open(source_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| AppError::Message(error.to_string()))?;

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| AppError::Message(error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let extension = normalize_extension(&enclosed);
        if is_supported_content_extension(&extension) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn extract_zip_archive(
    source_path: &Path,
    destination_root: &Path,
    notes: &mut Vec<String>,
) -> AppResult<()> {
    let file = fs::File::open(source_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| AppError::Message(error.to_string()))?;
    let mut ignored_entries = 0_i64;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AppError::Message(error.to_string()))?;
        let enclosed = match entry.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => {
                ignored_entries += 1;
                continue;
            }
        };

        if entry.is_dir() {
            fs::create_dir_all(destination_root.join(&enclosed))?;
            continue;
        }

        let extension = normalize_extension(&enclosed);
        if !should_extract_archive_entry(&extension) {
            ignored_entries += 1;
            continue;
        }

        let output_path = destination_root.join(&enclosed);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(&output_path)?;
        std::io::copy(&mut entry, &mut output)?;
    }

    if ignored_entries > 0 {
        notes.push(format!(
            "Ignored {ignored_entries} unsupported zip entries."
        ));
    }

    Ok(())
}

fn mark_missing_direct_sources(
    connection: &Connection,
    existing: &HashMap<String, ExistingDownloadItem>,
    observed: &[ObservedSource],
) -> AppResult<bool> {
    let observed_paths = observed
        .iter()
        .map(|item| normalize_path_key(&item.path.to_string_lossy()))
        .collect::<HashSet<_>>();
    let now = Utc::now().to_rfc3339();
    let mut changed = false;

    for item in existing.values() {
        if observed_paths.contains(&normalize_path_key(&item.source_path)) {
            continue;
        }
        if item.source_kind != "file" || item.status == "applied" || item.status == "ignored" {
            continue;
        }

        changed |= connection.execute(
            "UPDATE download_items
             SET status = 'error',
                 error_message = ?2,
                 updated_at = ?3
             WHERE id = ?1",
            params![item.id, "Source file is missing from Downloads.", now],
        )? > 0;
    }

    Ok(changed)
}

fn mark_missing_direct_sources_for_paths(
    connection: &Connection,
    existing: &HashMap<String, ExistingDownloadItem>,
    changed_paths: &[PathBuf],
) -> AppResult<bool> {
    let now = Utc::now().to_rfc3339();
    let mut changed = false;

    for path in changed_paths {
        let key = normalize_path_key(&path.to_string_lossy());
        let Some(item) = existing.get(&key) else {
            continue;
        };
        if path.exists()
            || item.source_kind != "file"
            || item.status == "applied"
            || item.status == "ignored"
        {
            continue;
        }

        changed |= connection.execute(
            "UPDATE download_items
             SET status = 'error',
                 error_message = ?2,
                 updated_at = ?3
             WHERE id = ?1",
            params![item.id, "Source file is missing from Downloads.", now],
        )? > 0;
    }

    Ok(changed)
}

fn recompute_item_statuses(connection: &Connection) -> AppResult<()> {
    let mut statement = connection.prepare("SELECT id, status FROM download_items")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (item_id, current_status) in rows {
        if current_status == "ignored" || current_status == "error" {
            continue;
        }
        let next_status = derive_item_status(connection, item_id)?;
        connection.execute(
            "UPDATE download_items
             SET status = ?2,
                 updated_at = ?3
             WHERE id = ?1",
            params![item_id, next_status, Utc::now().to_rfc3339()],
        )?;
    }

    Ok(())
}

fn derive_item_status(connection: &Connection, item_id: i64) -> AppResult<String> {
    let (
        active_file_count,
        applied_file_count,
        review_file_count,
        intake_mode,
        guided_install_available,
    ): (i64, i64, i64, String, i64) = connection.query_row(
        "SELECT
                (
                    SELECT COUNT(*)
                    FROM files
                    WHERE download_item_id = ?1
                      AND source_location = 'downloads'
                ),
                (
                    SELECT COUNT(*)
                    FROM files
                    WHERE download_item_id = ?1
                      AND source_location <> 'downloads'
                ),
                (
                    SELECT COUNT(DISTINCT rq.file_id)
                    FROM review_queue rq
                    JOIN files f ON f.id = rq.file_id
                    WHERE f.download_item_id = ?1
                      AND f.source_location = 'downloads'
                ),
                COALESCE(di.intake_mode, 'standard'),
                COALESCE(di.guided_install_available, 0)
             FROM download_items di
             WHERE di.id = ?1",
        params![item_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;

    if active_file_count == 0 && applied_file_count > 0 {
        return Ok("applied".to_owned());
    }

    if intake_mode == "blocked" {
        return Ok("needs_review".to_owned());
    }

    if intake_mode == "needs_review" && active_file_count > 0 {
        return Ok("needs_review".to_owned());
    }

    if intake_mode == "guided" && guided_install_available != 0 && active_file_count > 0 {
        return Ok("ready".to_owned());
    }

    if review_file_count > 0 && review_file_count < active_file_count {
        return Ok("partial".to_owned());
    }

    if review_file_count > 0 {
        return Ok("needs_review".to_owned());
    }

    if active_file_count > 0 {
        return Ok("ready".to_owned());
    }

    Ok("pending".to_owned())
}

fn summarize_status(
    connection: &Connection,
    watched_path: Option<String>,
) -> AppResult<DownloadsWatcherStatus> {
    let (ready_items, needs_review_items, active_items): (i64, i64, i64) = connection.query_row(
        "SELECT
            SUM(CASE WHEN status IN ('ready', 'partial') THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'needs_review' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status IN ('ready', 'partial', 'needs_review') THEN 1 ELSE 0 END)
         FROM download_items",
        [],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
            ))
        },
    )?;

    Ok(DownloadsWatcherStatus {
        state: DownloadsWatcherState::Watching,
        watched_path,
        configured: true,
        current_item: None,
        last_run_at: Some(Utc::now().to_rfc3339()),
        last_change_at: Some(Utc::now().to_rfc3339()),
        last_error: None,
        ready_items,
        needs_review_items,
        active_items,
    })
}

fn store_status(
    state: &AppState,
    app: &AppHandle,
    status: DownloadsWatcherStatus,
) -> AppResult<()> {
    {
        let status_handle = state.downloads_status();
        let mut guard = status_handle
            .lock()
            .map_err(|_| AppError::Message("Downloads status lock poisoned".to_owned()))?;
        *guard = status.clone();
    }

    emit_downloads_status(app, &status).map_err(AppError::Message)?;
    Ok(())
}

fn stop_watcher(state: &AppState) -> AppResult<()> {
    let control = state.downloads_watcher_control();
    let mut guard = control
        .lock()
        .map_err(|_| AppError::Message("Downloads watcher lock poisoned".to_owned()))?;
    if let Some(sender) = guard.stop_sender.take() {
        let _ = sender.send(());
    }
    Ok(())
}

fn update_last_seen(connection: &Connection, item_id: i64) -> AppResult<()> {
    connection.execute(
        "UPDATE download_items
         SET last_seen_at = ?2
         WHERE id = ?1",
        params![item_id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

fn load_overview(
    connection: &Connection,
    settings: &LibrarySettings,
) -> AppResult<DownloadsInboxOverview> {
    let (
        total_items,
        ready_items,
        needs_review_items,
        applied_items,
        error_items,
        active_files,
        ready_now_items,
        special_setup_items,
        waiting_on_you_items,
        blocked_items,
        done_items,
    ): (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64) = connection.query_row(
        "SELECT
            COUNT(*),
            SUM(CASE WHEN status IN ('ready', 'partial') THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'needs_review' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'applied' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END),
            (
                SELECT COUNT(*)
                FROM files
                WHERE source_location = 'downloads'
            ),
            SUM(
                CASE
                    WHEN status NOT IN ('applied', 'ignored')
                     AND intake_mode = 'standard'
                     AND status IN ('ready', 'partial')
                    THEN 1
                    ELSE 0
                END
            ),
            SUM(
                CASE
                    WHEN status NOT IN ('applied', 'ignored')
                     AND intake_mode = 'guided'
                    THEN 1
                    ELSE 0
                END
            ),
            SUM(
                CASE
                    WHEN status NOT IN ('applied', 'ignored')
                     AND (intake_mode = 'needs_review' OR status = 'needs_review')
                    THEN 1
                    ELSE 0
                END
            ),
             SUM(
                CASE
                    WHEN status = 'error'
                      OR (
                        status NOT IN ('applied', 'ignored')
                        AND intake_mode = 'blocked'
                      )
                    THEN 1
                    ELSE 0
                END
             ),
             SUM(CASE WHEN status IN ('applied', 'ignored') THEN 1 ELSE 0 END)
         FROM download_items
         WHERE status <> 'ignored'",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(4)?.unwrap_or_default(),
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(7)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(8)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(9)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(10)?.unwrap_or_default(),
            ))
        },
    )?;

    Ok(DownloadsInboxOverview {
        total_items,
        ready_items,
        needs_review_items,
        applied_items,
        error_items,
        active_files,
        watched_path: settings.downloads_path.clone(),
        ready_now_items,
        special_setup_items,
        waiting_on_you_items,
        blocked_items,
        done_items,
    })
}

fn load_item_by_id(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
) -> AppResult<Option<DownloadsInboxItem>> {
    let mut context = SpecialDecisionContext::default();
    load_item_by_id_cached(
        connection,
        settings,
        seed_pack,
        item_id,
        &mut context,
        false,
        true,
    )
}

fn load_item_by_id_cached(
    connection: &Connection,
    settings: &LibrarySettings,
    seed_pack: &crate::seed::SeedPack,
    item_id: i64,
    context: &mut SpecialDecisionContext,
    allow_network_latest: bool,
    include_full_special_details: bool,
) -> AppResult<Option<DownloadsInboxItem>> {
    connection
        .query_row(
            "SELECT
                di.id,
                di.display_name,
                di.source_path,
                di.source_kind,
                di.archive_format,
                di.status,
                di.source_size,
                di.detected_file_count,
                di.intake_mode,
                di.risk_level,
                di.matched_profile_key,
                di.matched_profile_name,
                di.special_family,
                di.assessment_reasons,
                di.dependency_summary,
                di.missing_dependencies,
                di.inbox_dependencies,
                di.incompatibility_warnings,
                di.post_install_notes,
                di.evidence_summary,
                di.catalog_source_url,
                di.catalog_download_url,
                di.latest_check_url,
                di.latest_check_strategy,
                di.catalog_reference_source,
                di.catalog_reviewed_at,
                di.existing_install_detected,
                di.guided_install_available,
                di.first_seen_at,
                di.last_seen_at,
                di.updated_at,
                di.error_message,
                di.notes,
                (
                    SELECT COUNT(*)
                    FROM files f
                    WHERE f.download_item_id = di.id
                      AND f.source_location = 'downloads'
                ) AS active_file_count,
                (
                    SELECT COUNT(*)
                    FROM files f
                    WHERE f.download_item_id = di.id
                      AND f.source_location <> 'downloads'
                ) AS applied_file_count,
                (
                    SELECT COUNT(DISTINCT rq.file_id)
                    FROM review_queue rq
                    JOIN files f ON f.id = rq.file_id
                    WHERE f.download_item_id = di.id
                      AND f.source_location = 'downloads'
                ) AS review_file_count
             FROM download_items di
             WHERE di.id = ?1",
            params![item_id],
            |row| {
                Ok(DownloadsInboxItem {
                    id: row.get(0)?,
                    display_name: row.get(1)?,
                    source_path: row.get(2)?,
                    source_kind: row.get(3)?,
                    archive_format: row.get(4)?,
                    status: row.get(5)?,
                    source_size: row.get(6)?,
                    detected_file_count: row.get(7)?,
                    intake_mode: parse_intake_mode(row.get::<_, String>(8)?),
                    risk_level: parse_risk_level(row.get::<_, String>(9)?),
                    matched_profile_key: row.get(10)?,
                    matched_profile_name: row.get(11)?,
                    special_family: row.get(12)?,
                    assessment_reasons: parse_string_array(row.get::<_, String>(13)?),
                    dependency_summary: parse_string_array(row.get::<_, String>(14)?),
                    missing_dependencies: parse_string_array(row.get::<_, String>(15)?),
                    inbox_dependencies: parse_string_array(row.get::<_, String>(16)?),
                    incompatibility_warnings: parse_string_array(row.get::<_, String>(17)?),
                    post_install_notes: parse_string_array(row.get::<_, String>(18)?),
                    evidence_summary: parse_string_array(row.get::<_, String>(19)?),
                    catalog_source: parse_catalog_source(
                        row.get(20)?,
                        row.get(21)?,
                        row.get(22)?,
                        row.get(23)?,
                        row.get::<_, String>(24)?,
                        row.get(25)?,
                    ),
                    existing_install_detected: row.get::<_, i64>(26)? != 0,
                    guided_install_available: row.get::<_, i64>(27)? != 0,
                    first_seen_at: row.get(28)?,
                    last_seen_at: row.get(29)?,
                    updated_at: row.get(30)?,
                    error_message: row.get(31)?,
                    notes: parse_string_array(row.get::<_, String>(32)?),
                    active_file_count: row.get(33)?,
                    applied_file_count: row.get(34)?,
                    review_file_count: row.get(35)?,
                    sample_files: Vec::new(),
                    queue_lane: DownloadQueueLane::ReadyNow,
                    queue_summary: String::new(),
                    family_key: None,
                    related_item_ids: Vec::new(),
                    timeline: Vec::new(),
                    special_decision: None,
                    version_resolution: None,
                })
            },
        )
        .optional()
        .map_err(Into::into)
        .and_then(|item| {
            if let Some(mut item) = item {
                item.sample_files = load_item_sample_names(connection, item.id)?;
                hydrate_download_item(
                    connection,
                    settings,
                    seed_pack,
                    &mut item,
                    context,
                    allow_network_latest,
                    include_full_special_details,
                )?;
                item.related_item_ids = load_related_item_ids(connection, &item)?;
                item.timeline = build_download_timeline(connection, &item);
                Ok(Some(item))
            } else {
                Ok(None)
            }
        })
}

fn load_item_sample_names(connection: &Connection, item_id: i64) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT filename
         FROM files
         WHERE download_item_id = ?1
         ORDER BY CASE WHEN source_location = 'downloads' THEN 0 ELSE 1 END,
                  filename COLLATE NOCASE
         LIMIT 4",
    )?;
    let names = statement
        .query_map(params![item_id], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(names)
}

fn load_item_sample_names_batch(
    connection: &Connection,
    item_ids: &[i64],
) -> AppResult<HashMap<i64, Vec<String>>> {
    if item_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = std::iter::repeat("?")
        .take(item_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT download_item_id, filename
         FROM files
         WHERE download_item_id IN ({placeholders})
         ORDER BY download_item_id,
                  CASE WHEN source_location = 'downloads' THEN 0 ELSE 1 END,
                  filename COLLATE NOCASE"
    );

    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(rusqlite::params_from_iter(item_ids.iter()))?;
    let mut samples = HashMap::new();

    while let Some(row) = rows.next()? {
        let item_id = row.get::<_, i64>(0)?;
        let filename = row.get::<_, String>(1)?;
        let entry = samples.entry(item_id).or_insert_with(Vec::new);
        if entry.len() < 4 {
            entry.push(filename);
        }
    }

    Ok(samples)
}

fn normalize_extension(path: &Path) -> String {
    path.extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

fn dedupe_changed_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for path in paths {
        let key = normalize_path_key(&path.to_string_lossy());
        if seen.insert(key) {
            deduped.push(path);
        }
    }
    deduped
}

fn archive_format_for_extension(extension: &str) -> Option<String> {
    match extension {
        ".zip" => Some("zip".to_owned()),
        ".7z" => Some("7z".to_owned()),
        ".rar" => Some("rar".to_owned()),
        _ => None,
    }
}

fn is_archive_extension(extension: &str) -> bool {
    matches!(extension, ".zip" | ".7z" | ".rar")
}

fn is_supported_content_extension(extension: &str) -> bool {
    matches!(
        extension,
        ".package"
            | ".ts4script"
            | ".trayitem"
            | ".blueprint"
            | ".bpi"
            | ".householdbinary"
            | ".hhi"
            | ".sgi"
            | ".room"
            | ".rmi"
    )
}

fn is_observable_download_extension(extension: &str) -> bool {
    is_supported_content_extension(extension) || is_archive_extension(extension)
}

fn should_extract_archive_entry(extension: &str) -> bool {
    is_supported_content_extension(extension) || matches!(extension, ".txt" | ".md" | ".rtf")
}

fn parse_string_array(value: String) -> Vec<String> {
    serde_json::from_str(&value).unwrap_or_default()
}

fn parse_catalog_source(
    official_source_url: Option<String>,
    official_download_url: Option<String>,
    latest_check_url: Option<String>,
    latest_check_strategy: Option<String>,
    reference_source: String,
    reviewed_at: Option<String>,
) -> Option<CatalogSourceInfo> {
    let reference_source = parse_string_array(reference_source);
    if official_source_url.is_none()
        && official_download_url.is_none()
        && latest_check_url.is_none()
        && latest_check_strategy.is_none()
        && reference_source.is_empty()
        && reviewed_at.is_none()
    {
        None
    } else {
        Some(CatalogSourceInfo {
            official_source_url,
            official_download_url,
            latest_check_url,
            latest_check_strategy,
            reference_source,
            reviewed_at,
        })
    }
}

fn parse_intake_mode(value: String) -> DownloadIntakeMode {
    match value.as_str() {
        "guided" => DownloadIntakeMode::Guided,
        "needs_review" => DownloadIntakeMode::NeedsReview,
        "blocked" => DownloadIntakeMode::Blocked,
        _ => DownloadIntakeMode::Standard,
    }
}

fn parse_risk_level(value: String) -> DownloadRiskLevel {
    match value.as_str() {
        "medium" => DownloadRiskLevel::Medium,
        "high" => DownloadRiskLevel::High,
        _ => DownloadRiskLevel::Low,
    }
}

#[cfg(test)]
fn has_auto_recheck_note(notes: &[String]) -> bool {
    notes
        .iter()
        .any(|note| note.starts_with(AUTO_RECHECK_NOTE_PREFIX))
}

fn normalize_path_key(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

fn system_time_to_rfc3339(time: std::time::SystemTime) -> String {
    let date_time: DateTime<Utc> = time.into();
    date_time.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::{
        build_archive_staging_root, can_skip_observed_source, checking_downloads_status,
        derive_item_status, get_download_item_guided_plan, get_download_item_selection,
        has_auto_recheck_note, ingest_held_archive_source, ingest_ignored_non_sims_source,
        list_download_queue, load_existing_items, mark_item_rechecked_with_new_rules,
        mark_missing_direct_sources_for_paths, parse_string_array, preview_download_item,
        reassess_existing_item, refresh_download_item_status, should_extract_archive_source,
        should_use_full_downloads_scan, staging_segment_for_source, summarize_status,
        ExistingDownloadItem, ObservedSource,
    };
    use crate::core::install_profile_engine::SpecialDecisionContext;
    use crate::database::initialize;
    use crate::models::{
        DownloadQueueLane, DownloadsInboxQuery, DownloadsWatcherState, LibrarySettings,
    };
    use crate::seed;
    use chrono::{TimeZone, Utc};
    use rusqlite::{params, Connection};
    use std::{fs::File, io::Write};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn setup_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
    }

    #[test]
    fn checking_status_marks_downloads_as_configured_and_processing() {
        let status = checking_downloads_status(
            std::path::Path::new("C:/Users/Test/Downloads"),
            Some("Initial inbox refresh".to_owned()),
        );

        assert_eq!(status.state, DownloadsWatcherState::Processing);
        assert!(status.configured);
        assert_eq!(
            status.watched_path.as_deref(),
            Some("C:/Users/Test/Downloads")
        );
        assert_eq!(
            status.current_item.as_deref(),
            Some("Initial inbox refresh")
        );
    }

    #[test]
    fn staging_segment_normalizes_download_names_for_safe_paths() {
        assert_eq!(
            staging_segment_for_source("MCCC Update Test.zip"),
            "mccc-update-test-zip"
        );
        assert_eq!(staging_segment_for_source("   "), "download");
    }

    #[test]
    fn archive_staging_root_stays_unique_for_different_names_in_the_same_second() {
        let app_data_dir = std::path::Path::new("C:/Temp/SimSuite");
        let now = Utc
            .with_ymd_and_hms(2026, 3, 12, 14, 5, 7)
            .single()
            .expect("timestamp");
        let first_root = build_archive_staging_root(app_data_dir, None, "MCCC_Partial.zip", now);
        let second_root = build_archive_staging_root(app_data_dir, None, "MCCC_Update.zip", now);

        assert_ne!(first_root, second_root);
        assert!(first_root.to_string_lossy().contains("mccc-partial-zip"));
        assert!(second_root.to_string_lossy().contains("mccc-update-zip"));
    }

    #[test]
    fn applied_items_with_no_live_download_files_are_not_skipped_when_the_source_zip_is_still_there(
    ) {
        let existing = ExistingDownloadItem {
            id: 41,
            source_path: "C:/Users/Test/Downloads/XmlInjector_Script_v4.2.zip".to_owned(),
            source_size: 14262,
            source_modified_at: Some("2026-03-13T02:33:38Z".to_owned()),
            status: "applied".to_owned(),
            source_kind: "archive".to_owned(),
            active_file_count: 0,
        };
        let source = ObservedSource {
            path: std::path::PathBuf::from("C:/Users/Test/Downloads/XmlInjector_Script_v4.2.zip"),
            display_name: "XmlInjector_Script_v4.2.zip".to_owned(),
            source_kind: "archive".to_owned(),
            archive_format: Some("zip".to_owned()),
            source_size: 14262,
            source_modified_at: Some("2026-03-13T02:33:38Z".to_owned()),
        };

        assert!(!can_skip_observed_source(&existing, &source));
    }

    fn insert_download_item(connection: &Connection, item_id: i64, status: &str) {
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, status
                 ) VALUES (?1, ?2, ?3, 'file', ?4)",
                params![
                    item_id,
                    format!("C:/Downloads/item-{item_id}.package"),
                    format!("item-{item_id}.package"),
                    status
                ],
            )
            .expect("insert download item");
    }

    fn insert_download_item_with_mode(
        connection: &Connection,
        item_id: i64,
        status: &str,
        intake_mode: &str,
        guided_install_available: i64,
    ) {
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, status, intake_mode, guided_install_available
                 ) VALUES (?1, ?2, ?3, 'file', ?4, ?5, ?6)",
                params![
                    item_id,
                    format!("C:/Downloads/item-{item_id}.package"),
                    format!("item-{item_id}.package"),
                    status,
                    intake_mode,
                    guided_install_available
                ],
            )
            .expect("insert download item with mode");
    }

    fn create_test_zip(path: &std::path::Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).expect("create zip");
        let mut archive = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (entry_path, contents) in entries {
            archive
                .start_file(*entry_path, options)
                .expect("start zip entry");
            archive.write_all(contents).expect("write zip entry");
        }

        archive.finish().expect("finish zip");
    }

    fn insert_file(
        connection: &Connection,
        item_id: i64,
        source_location: &str,
        filename: &str,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    download_item_id,
                    parser_warnings
                 ) VALUES (?1, ?2, '.package', 'CAS', 0.92, ?3, ?4, '[]')",
                params![
                    format!("C:/Library/{source_location}/{item_id}-{filename}"),
                    filename,
                    source_location,
                    item_id
                ],
            )
            .expect("insert file");
        connection.last_insert_rowid()
    }

    fn insert_file_with_shape(
        connection: &Connection,
        item_id: i64,
        source_location: &str,
        filename: &str,
        extension: &str,
        kind: &str,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO files (
                    path,
                    filename,
                    extension,
                    kind,
                    confidence,
                    source_location,
                    download_item_id,
                    parser_warnings
                 ) VALUES (?1, ?2, ?3, ?4, 0.92, ?5, ?6, '[]')",
                params![
                    format!("C:/Library/{source_location}/{item_id}-{filename}"),
                    filename,
                    extension,
                    kind,
                    source_location,
                    item_id
                ],
            )
            .expect("insert shaped file");
        connection.last_insert_rowid()
    }

    #[test]
    fn derive_item_status_returns_ready_for_active_safe_files() {
        let connection = setup_connection();
        insert_download_item(&connection, 1, "pending");
        insert_file(&connection, 1, "downloads", "ready-one.package");
        insert_file(&connection, 1, "downloads", "ready-two.package");

        let status = derive_item_status(&connection, 1).expect("derive status");
        assert_eq!(status, "ready");
    }

    #[test]
    fn guided_ready_items_ignore_generic_review_flags() {
        let connection = setup_connection();
        insert_download_item_with_mode(&connection, 24, "partial", "guided", 1);
        let file_id = insert_file_with_shape(
            &connection,
            24,
            "downloads",
            "mc_cmd_center.package",
            ".package",
            "Unknown",
        );
        insert_file_with_shape(
            &connection,
            24,
            "downloads",
            "mc_cmd_center.ts4script",
            ".ts4script",
            "ScriptMods",
        );
        connection
            .execute(
                "INSERT INTO review_queue (file_id, reason, confidence)
                 VALUES (?1, ?2, ?3)",
                params![file_id, "low_confidence_parse", 0.41_f64],
            )
            .expect("insert review item");

        let status = derive_item_status(&connection, 24).expect("derive status");
        assert_eq!(status, "ready");
    }

    #[test]
    fn derive_item_status_returns_partial_when_only_some_files_need_review() {
        let connection = setup_connection();
        insert_download_item(&connection, 2, "pending");
        let first_file_id = insert_file(&connection, 2, "downloads", "review-one.package");
        insert_file(&connection, 2, "downloads", "review-two.package");
        connection
            .execute(
                "INSERT INTO review_queue (file_id, reason, confidence)
                 VALUES (?1, ?2, ?3)",
                params![first_file_id, "low_confidence_name", 0.44_f64],
            )
            .expect("insert review item");

        let status = derive_item_status(&connection, 2).expect("derive status");
        assert_eq!(status, "partial");
    }

    #[test]
    fn derive_item_status_returns_needs_review_when_all_active_files_are_flagged() {
        let connection = setup_connection();
        insert_download_item(&connection, 3, "pending");
        let first_file_id = insert_file(&connection, 3, "downloads", "flagged-one.package");
        let second_file_id = insert_file(&connection, 3, "downloads", "flagged-two.package");
        connection
            .execute(
                "INSERT INTO review_queue (file_id, reason, confidence)
                 VALUES (?1, ?2, ?3), (?4, ?5, ?6)",
                params![
                    first_file_id,
                    "low_confidence_name",
                    0.4_f64,
                    second_file_id,
                    "unsafe_script_depth",
                    0.5_f64
                ],
            )
            .expect("insert review items");

        let status = derive_item_status(&connection, 3).expect("derive status");
        assert_eq!(status, "needs_review");
    }

    #[test]
    fn derive_item_status_returns_applied_when_active_files_are_gone_but_moves_exist() {
        let connection = setup_connection();
        insert_download_item(&connection, 4, "pending");
        insert_file(&connection, 4, "mods", "moved-file.package");

        let status = derive_item_status(&connection, 4).expect("derive status");
        assert_eq!(status, "applied");
    }

    #[test]
    fn summarize_status_counts_ready_partial_and_review_items() {
        let connection = setup_connection();
        insert_download_item(&connection, 10, "ready");
        insert_download_item(&connection, 11, "partial");
        insert_download_item(&connection, 12, "needs_review");
        insert_download_item(&connection, 13, "applied");
        insert_download_item(&connection, 14, "ignored");
        insert_download_item(&connection, 15, "error");

        let summary = summarize_status(&connection, Some("C:/Users/Test/Downloads".to_owned()))
            .expect("summary");

        assert_eq!(summary.ready_items, 2);
        assert_eq!(summary.needs_review_items, 1);
        assert_eq!(summary.active_items, 3);
        assert!(summary.configured);
        assert_eq!(
            summary.watched_path.as_deref(),
            Some("C:/Users/Test/Downloads")
        );
    }

    #[test]
    fn preview_download_item_rejects_guided_inbox_items() {
        let connection = setup_connection();
        let seed_pack = crate::seed::load_seed_pack().expect("seed pack");
        insert_download_item_with_mode(&connection, 20, "ready", "guided", 1);
        insert_file(&connection, 20, "downloads", "mc_cmd_center.ts4script");

        let error = preview_download_item(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: None,
                downloads_path: Some("C:/Downloads".to_owned()),
            },
            &seed_pack,
            20,
            Some("Category First".to_owned()),
        )
        .expect_err("guided items should not use normal preview");

        assert!(error.to_string().contains("guided special setup flow"));
    }

    #[test]
    fn blocked_items_do_not_return_guided_plans() {
        let connection = setup_connection();
        insert_download_item_with_mode(&connection, 21, "needs_review", "blocked", 0);
        let seed_pack = seed::load_seed_pack().expect("seed");

        let plan = get_download_item_guided_plan(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: None,
                downloads_path: Some("C:/Downloads".to_owned()),
            },
            &seed_pack,
            21,
        )
        .expect("guided plan query");

        assert!(plan.is_none());
    }

    #[test]
    fn manual_reassessment_updates_unchanged_special_items() {
        let connection = setup_connection();
        insert_download_item_with_mode(&connection, 22, "partial", "standard", 0);
        insert_file_with_shape(
            &connection,
            22,
            "downloads",
            "mc_cmd_center.ts4script",
            ".ts4script",
            "ScriptMods",
        );
        insert_file_with_shape(
            &connection,
            22,
            "downloads",
            "mc_cmd_center.package",
            ".package",
            "Unknown",
        );
        insert_file_with_shape(
            &connection,
            22,
            "downloads",
            "mc_woohoo.ts4script",
            ".ts4script",
            "ScriptMods",
        );

        let seed_pack = seed::load_seed_pack().expect("seed");
        let settings = LibrarySettings {
            mods_path: Some("C:/Mods".to_owned()),
            tray_path: None,
            downloads_path: Some("C:/Downloads".to_owned()),
        };
        let mut special_context = SpecialDecisionContext::default();

        reassess_existing_item(&connection, &settings, &seed_pack, 22, &mut special_context)
            .expect("reassess special item");

        let (intake_mode, matched_profile_name, guided_install_available): (
            String,
            Option<String>,
            i64,
        ) = connection
            .query_row(
                "SELECT intake_mode, matched_profile_name, guided_install_available
                 FROM download_items
                 WHERE id = ?1",
                params![22],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load refreshed item");

        assert_eq!(intake_mode, "guided");
        assert_eq!(matched_profile_name.as_deref(), Some("MC Command Center"));
        assert_eq!(guided_install_available, 1);
    }

    #[test]
    fn hydrated_selection_uses_ready_special_guided_state() {
        let temp = tempdir().expect("temp dir");
        let downloads = temp.path().join("Downloads");
        let mods = temp.path().join("Mods");
        let staging = downloads.join("xml-same-version");
        let installed_root = mods.join("XML Injector");
        std::fs::create_dir_all(&staging).expect("staging");
        std::fs::create_dir_all(&installed_root).expect("installed root");

        let connection = setup_connection();
        let seed_pack = seed::load_seed_pack().expect("seed");
        let settings = LibrarySettings {
            mods_path: Some(mods.to_string_lossy().to_string()),
            tray_path: None,
            downloads_path: Some(downloads.to_string_lossy().to_string()),
        };

        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, status, intake_mode, guided_install_available
                 ) VALUES (?1, ?2, ?3, 'archive', 'ready', 'standard', 0)",
                params![
                    61_i64,
                    staging.join("XML_Injector_Same.zip")
                        .to_string_lossy()
                        .to_string(),
                    "XML_Injector_Same.zip"
                ],
            )
            .expect("insert download item");

        let script_path = staging.join("XmlInjector_Script_v4_0.ts4script");
        let installed_script = installed_root.join("XmlInjector_Script_v4_0.ts4script");
        create_test_zip(
            &script_path,
            &[("version.txt", b"XML Injector version 4.0")],
        );
        create_test_zip(
            &installed_script,
            &[("version.txt", b"XML Injector version 4.0")],
        );

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, confidence, source_location, download_item_id, parser_warnings
                ) VALUES (?1, ?2, ?3, ?4, 0.92, 'downloads', ?5, '[]')",
                params![
                    script_path.to_string_lossy().to_string(),
                    "XmlInjector_Script_v4_0.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    61_i64
                ],
            )
            .expect("insert script");

        let mut special_context = SpecialDecisionContext::default();
        reassess_existing_item(&connection, &settings, &seed_pack, 61, &mut special_context)
            .expect("reassess special item");

        connection
            .execute(
                "UPDATE download_items
                 SET intake_mode = 'standard',
                     guided_install_available = 0
                 WHERE id = ?1",
                params![61_i64],
            )
            .expect("downgrade row");

        let selection = get_download_item_selection(&connection, &settings, &seed_pack, 61, None)
            .expect("selection");
        let item = selection.detail.expect("detail").item;

        assert_eq!(item.intake_mode, crate::models::DownloadIntakeMode::Guided);
        assert!(item.guided_install_available);
    }

    #[test]
    fn queue_hydration_keeps_same_version_special_items_in_done_with_reinstall_path() {
        let temp = tempdir().expect("temp dir");
        let downloads = temp.path().join("Downloads");
        let mods = temp.path().join("Mods");
        let staging = downloads.join("xml-same-version-queue");
        let installed_root = mods.join("XML Injector");
        std::fs::create_dir_all(&staging).expect("staging");
        std::fs::create_dir_all(&installed_root).expect("installed root");

        let connection = setup_connection();
        let seed_pack = seed::load_seed_pack().expect("seed");
        let settings = LibrarySettings {
            mods_path: Some(mods.to_string_lossy().to_string()),
            tray_path: None,
            downloads_path: Some(downloads.to_string_lossy().to_string()),
        };

        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, status, intake_mode, guided_install_available
                 ) VALUES (?1, ?2, ?3, 'archive', 'ready', 'standard', 0)",
                params![
                    62_i64,
                    staging.join("XML_Injector_Same_Queue.zip")
                        .to_string_lossy()
                        .to_string(),
                    "XML_Injector_Same_Queue.zip"
                ],
            )
            .expect("insert download item");

        let script_path = staging.join("XmlInjector_Script_v4_0.ts4script");
        let installed_script = installed_root.join("XmlInjector_Script_v4_0.ts4script");
        create_test_zip(
            &script_path,
            &[("version.txt", b"XML Injector version 4.0")],
        );
        create_test_zip(
            &installed_script,
            &[("version.txt", b"XML Injector version 4.0")],
        );

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, confidence, source_location, download_item_id, parser_warnings
                ) VALUES (?1, ?2, ?3, ?4, 0.92, 'downloads', ?5, '[]')",
                params![
                    script_path.to_string_lossy().to_string(),
                    "XmlInjector_Script_v4_0.ts4script",
                    ".ts4script",
                    "ScriptMods",
                    62_i64
                ],
            )
            .expect("insert script");

        let mut special_context = SpecialDecisionContext::default();
        reassess_existing_item(&connection, &settings, &seed_pack, 62, &mut special_context)
            .expect("reassess special item");

        connection
            .execute(
                "UPDATE download_items
                 SET intake_mode = 'standard',
                     guided_install_available = 0
                 WHERE id = ?1",
                params![62_i64],
            )
            .expect("downgrade row");

        let queue = list_download_queue(
            &connection,
            &settings,
            &seed_pack,
            DownloadsInboxQuery::default(),
        )
        .expect("queue");
        let item = queue
            .items
            .iter()
            .find(|item| item.id == 62)
            .expect("queue item");

        assert_eq!(item.queue_lane, DownloadQueueLane::Done);
        assert!(item
            .queue_summary
            .contains("matches the version that is already installed"));
        assert_eq!(queue.overview.done_items, 1);
    }

    #[test]
    fn held_archives_are_kept_as_blocked_inbox_items() {
        let temp = tempdir().expect("temp dir");
        let archive_path = temp.path().join("test-download.rar");
        std::fs::write(&archive_path, b"not a real archive").expect("write archive");
        let metadata = archive_path.metadata().expect("archive metadata");
        let mut connection = setup_connection();
        let source = ObservedSource {
            path: archive_path.clone(),
            display_name: "test-download.rar".to_owned(),
            source_kind: "archive".to_owned(),
            archive_format: Some("rar".to_owned()),
            source_size: metadata.len() as i64,
            source_modified_at: metadata.modified().ok().map(super::system_time_to_rfc3339),
        };

        let item_id =
            ingest_held_archive_source(&mut connection, &source, None).expect("held archive item");

        let (status, intake_mode, error_message, detected_file_count): (
            String,
            String,
            Option<String>,
            i64,
        ) = connection
            .query_row(
                "SELECT status, intake_mode, error_message, detected_file_count
                 FROM download_items
                 WHERE id = ?1",
                params![item_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("load held archive item");

        assert_eq!(status, "needs_review");
        assert_eq!(intake_mode, "blocked");
        assert_eq!(detected_file_count, 0);
        assert!(error_message
            .as_deref()
            .is_some_and(|value| value.contains("paused .rar extraction")));
    }

    #[test]
    fn zip_quick_check_skips_archives_without_supported_sims_files() {
        let temp = tempdir().expect("temp dir");
        let archive_path = temp.path().join("notes-only.zip");
        create_test_zip(
            &archive_path,
            &[
                ("docs/readme.txt", b"read me"),
                ("docs/changelog.md", b"changes"),
            ],
        );
        let metadata = archive_path.metadata().expect("zip metadata");
        let source = ObservedSource {
            path: archive_path,
            display_name: "notes-only.zip".to_owned(),
            source_kind: "archive".to_owned(),
            archive_format: Some("zip".to_owned()),
            source_size: metadata.len() as i64,
            source_modified_at: metadata.modified().ok().map(super::system_time_to_rfc3339),
        };
        let mut notes = Vec::new();

        let should_extract =
            should_extract_archive_source(&source, &mut notes).expect("quick zip check");

        assert!(!should_extract);
        assert!(notes.iter().any(|note| {
            note.contains("Skipped ZIP extraction because no supported Sims files were found")
        }));
    }

    #[test]
    fn zip_quick_check_keeps_archives_with_supported_sims_files() {
        let temp = tempdir().expect("temp dir");
        let archive_path = temp.path().join("mod-files.zip");
        create_test_zip(
            &archive_path,
            &[
                ("Mods/test.package", b"package bytes"),
                ("docs/readme.txt", b"read me"),
            ],
        );
        let metadata = archive_path.metadata().expect("zip metadata");
        let source = ObservedSource {
            path: archive_path,
            display_name: "mod-files.zip".to_owned(),
            source_kind: "archive".to_owned(),
            archive_format: Some("zip".to_owned()),
            source_size: metadata.len() as i64,
            source_modified_at: metadata.modified().ok().map(super::system_time_to_rfc3339),
        };
        let mut notes = Vec::new();

        let should_extract =
            should_extract_archive_source(&source, &mut notes).expect("quick zip check");

        assert!(should_extract);
        assert!(notes.is_empty());
    }

    #[test]
    fn irrelevant_archives_are_auto_ignored() {
        let temp = tempdir().expect("temp dir");
        let archive_path = temp.path().join("notes-only.zip");
        create_test_zip(&archive_path, &[("docs/readme.txt", b"read me")]);
        let metadata = archive_path.metadata().expect("zip metadata");
        let mut connection = setup_connection();
        let source = ObservedSource {
            path: archive_path,
            display_name: "notes-only.zip".to_owned(),
            source_kind: "archive".to_owned(),
            archive_format: Some("zip".to_owned()),
            source_size: metadata.len() as i64,
            source_modified_at: metadata.modified().ok().map(super::system_time_to_rfc3339),
        };

        let item_id = ingest_ignored_non_sims_source(
            &mut connection,
            &source,
            None,
            &[
                "Skipped ZIP extraction because no supported Sims files were found inside."
                    .to_owned(),
            ],
        )
        .expect("ignored source");

        let (status, error_message, notes_json): (String, Option<String>, String) = connection
            .query_row(
                "SELECT status, error_message, notes
                 FROM download_items
                 WHERE id = ?1",
                params![item_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("load ignored item");
        let notes = parse_string_array(notes_json);

        assert_eq!(status, "ignored");
        assert!(error_message.is_none());
        assert!(notes.iter().any(|note| {
            note.contains(
                "ignored this download because it does not contain Sims mod or Tray files",
            )
        }));
    }

    #[test]
    fn default_queue_hides_ignored_items_but_ignored_filter_can_show_them() {
        let connection = setup_connection();
        let settings = LibrarySettings {
            mods_path: None,
            tray_path: None,
            downloads_path: Some("C:/Downloads".to_owned()),
        };
        let seed_pack = seed::load_seed_pack().expect("seed pack");
        insert_download_item(&connection, 51, "ignored");
        insert_download_item(&connection, 52, "ready");

        let default_queue = list_download_queue(
            &connection,
            &settings,
            &seed_pack,
            DownloadsInboxQuery::default(),
        )
        .expect("default queue");
        let ignored_queue = list_download_queue(
            &connection,
            &settings,
            &seed_pack,
            DownloadsInboxQuery {
                search: None,
                status: Some("ignored".to_owned()),
                limit: None,
            },
        )
        .expect("ignored queue");

        assert_eq!(default_queue.overview.total_items, 1);
        assert_eq!(default_queue.items.len(), 1);
        assert_eq!(default_queue.items[0].id, 52);
        assert_eq!(ignored_queue.items.len(), 1);
        assert_eq!(ignored_queue.items[0].id, 51);
    }

    #[test]
    fn refresh_download_item_status_can_recheck_blocked_error_items() {
        let connection = setup_connection();
        insert_download_item_with_mode(&connection, 24, "error", "blocked", 0);

        refresh_download_item_status(&connection, 24).expect("refresh blocked error item");

        let status: String = connection
            .query_row(
                "SELECT status FROM download_items WHERE id = ?1",
                params![24],
                |row| row.get(0),
            )
            .expect("load refreshed status");

        assert_eq!(status, "needs_review");
    }

    #[test]
    fn narrow_watcher_scan_stays_on_simple_file_events() {
        let temp = tempdir().expect("temp dir");
        let watched_root = temp.path().to_path_buf();
        let changed_path = watched_root.join("new-mod.package");
        std::fs::write(&changed_path, b"package").expect("write file");

        assert!(!should_use_full_downloads_scan(
            &watched_root,
            false,
            false,
            Some(&[changed_path]),
        ));
    }

    #[test]
    fn narrow_watcher_scan_marks_removed_direct_files_as_changed() {
        let temp = tempdir().expect("temp dir");
        let watched_root = temp.path().to_path_buf();
        let removed_path = watched_root.join("removed-mod.package");
        std::fs::write(&removed_path, b"package").expect("write file");
        let connection = setup_connection();
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, status
                 ) VALUES (?1, ?2, ?3, 'file', 'ready')",
                params![
                    41,
                    removed_path.to_string_lossy().to_string(),
                    "removed-mod.package"
                ],
            )
            .expect("insert download item");
        let existing = load_existing_items(&connection).expect("existing items");
        std::fs::remove_file(&removed_path).expect("remove file");

        let changed =
            mark_missing_direct_sources_for_paths(&connection, &existing, &[removed_path.clone()])
                .expect("mark removed file");

        let (status, error_message): (String, Option<String>) = connection
            .query_row(
                "SELECT status, error_message FROM download_items WHERE id = 41",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load removed item");

        assert!(changed);
        assert_eq!(status, "error");
        assert!(error_message
            .as_deref()
            .is_some_and(|value| value.contains("missing")));
    }

    #[test]
    fn auto_recheck_note_is_deduplicated_and_visible() {
        let connection = setup_connection();
        insert_download_item(&connection, 23, "ready");

        mark_item_rechecked_with_new_rules(&connection, 23).expect("first note");
        mark_item_rechecked_with_new_rules(&connection, 23).expect("replace note");

        let notes_json: String = connection
            .query_row(
                "SELECT notes FROM download_items WHERE id = ?1",
                params![23],
                |row| row.get(0),
            )
            .expect("load notes");
        let notes = parse_string_array(notes_json);

        assert_eq!(notes.len(), 1);
        assert!(has_auto_recheck_note(&notes));
    }
}
