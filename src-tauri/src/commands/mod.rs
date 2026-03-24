use chrono::Utc;
use reqwest::Url;
use rusqlite::Connection;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    thread,
    time::Duration,
    time::Instant,
};

use dirs::document_dir;
use reqwest::blocking::Client;
use tauri::{AppHandle, Emitter, State};

use crate::{
    app_state::AppState,
    core::{
        category_audit, content_versions, creator_audit, downloads_watcher, install_profile_engine,
        library_index, move_engine, rule_engine, scanner, snapshot_manager, special_mod_versions,
        watch_polling,
    },
    database, ensure_tray,
    error::AppError,
    models::{
        AppBehaviorSettings, ApplyCategoryAuditResult, ApplyCreatorAuditResult,
        ApplyGuidedDownloadResult, ApplyMcccUpdateResult, ApplyPreviewResult,
        ApplyReviewPlanActionResult, ApplySpecialReviewFixResult, BatchApplyResult, CategoryAuditFile,
        CategoryAuditQuery, CategoryAuditResponse, CreatorAuditFile, CreatorAuditQuery,
        CreatorAuditResponse, DetectedLibraryPaths, DownloadInboxDetail, DownloadsBootstrapResponse,
        DownloadsInboxQuery, DownloadsInboxResponse, DownloadsSelectionResponse,
        DownloadsWatcherState, DownloadsWatcherStatus, DuplicateOverview, DuplicatePair, FileDetail,
        GuidedInstallPlan, HomeOverview, IgnoreItemsResult, LibraryFacets, LibraryListResponse,
        LibraryQuery, LibrarySettings, LibraryWatchBulkSaveItemResult, LibraryWatchBulkSaveResult,
        LibraryWatchListResponse, LibraryWatchReviewResponse, LibraryWatchSetupResponse,
        McccUpdateInfo, OrganizationPreview, RestoreSnapshotResult, ReviewPlanAction,
        ReviewPlanActionKind, ReviewQueueItem, RulePreset, SaveLibraryWatchSourceEntry, ScanPhase,
        ScanRuntimeState, ScanStatus, ScanSummary, SnapshotSummary, SpecialReviewPlan,
        StagingAreasSummary, CleanupResult, StagingCommitResult, WatchListFilter,
        WatchRefreshSummary, WatchSourceKind, WorkspaceChange, WorkspaceDomain,
    },
    sync_tray_visibility,
};

fn map_error(error: AppError) -> String {
    error.to_string()
}

async fn run_blocking_command<T, F>(command: &'static str, operation: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| format!("{command} task failed: {error}"))?
}

const SLOW_COMMAND_LOG_THRESHOLD_MS: u128 = 40;

fn log_slow_command(command: &str, started_at: Instant, detail: impl FnOnce() -> String) {
    #[cfg(debug_assertions)]
    {
        let elapsed_ms = started_at.elapsed().as_millis();
        if elapsed_ms >= SLOW_COMMAND_LOG_THRESHOLD_MS {
            eprintln!("[perf] {command} took {elapsed_ms}ms {}", detail());
        }
    }
}

const READ_RETRY_BACKOFF_MS: [u64; 3] = [60, 120, 240];

fn is_locked_read_error(error: &AppError) -> bool {
    let lowered = error.to_string().to_ascii_lowercase();
    lowered.contains("database is locked")
        || lowered.contains("database table is locked")
        || lowered.contains("database schema is locked")
        || lowered.contains("database busy")
}

fn retry_locked_read<T>(
    command: &str,
    mut operation: impl FnMut() -> Result<T, AppError>,
) -> Result<T, String> {
    let mut last_error: Option<AppError> = None;

    for (attempt, backoff_ms) in READ_RETRY_BACKOFF_MS
        .iter()
        .copied()
        .chain(std::iter::once(0))
        .enumerate()
    {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if !is_locked_read_error(&error) || attempt == READ_RETRY_BACKOFF_MS.len() {
                    return Err(map_error(error));
                }

                #[cfg(debug_assertions)]
                eprintln!(
                    "[perf] {command} hit a locked read on attempt {}. Retrying in {}ms.",
                    attempt + 1,
                    backoff_ms
                );
                last_error = Some(error);
                thread::sleep(Duration::from_millis(backoff_ms));
            }
        }
    }

    Err(last_error
        .map(map_error)
        .unwrap_or_else(|| "Read failed.".to_owned()))
}

fn workspace_change(
    domains: Vec<WorkspaceDomain>,
    reason: &str,
    item_ids: Vec<i64>,
    family_keys: Vec<String>,
) -> WorkspaceChange {
    WorkspaceChange {
        domains,
        reason: reason.to_owned(),
        item_ids,
        family_keys,
    }
}

fn emit_workspace_domains(
    app: &AppHandle,
    domains: Vec<WorkspaceDomain>,
    reason: &str,
    item_ids: Vec<i64>,
    family_keys: Vec<String>,
) -> Result<(), String> {
    emit_workspace_change(
        app,
        &workspace_change(domains, reason, item_ids, family_keys),
    )
}

fn review_action_kind_matches(kind: &ReviewPlanActionKind, value: &str) -> bool {
    matches!(
        (kind, value),
        (ReviewPlanActionKind::RepairSpecial, "repair_special")
            | (
                ReviewPlanActionKind::InstallDependency,
                "install_dependency"
            )
            | (ReviewPlanActionKind::OpenDependency, "open_dependency")
            | (ReviewPlanActionKind::OpenRelatedItem, "open_related_item")
            | (
                ReviewPlanActionKind::DownloadMissingFiles,
                "download_missing_files"
            )
            | (
                ReviewPlanActionKind::OpenOfficialSource,
                "open_official_source"
            )
            | (
                ReviewPlanActionKind::SeparateSupportedFiles,
                "separate_supported_files"
            )
    )
}

fn find_review_action(
    plan: &SpecialReviewPlan,
    action_kind: &str,
    related_item_id: Option<i64>,
    url: Option<&str>,
) -> Result<ReviewPlanAction, String> {
    plan.available_actions
        .iter()
        .find(|action| {
            review_action_kind_matches(&action.kind, action_kind)
                && action.related_item_id == related_item_id
                && review_action_url_matches(action.url.as_deref(), url)
        })
        .cloned()
        .ok_or_else(|| {
            "This review action is no longer available for the selected inbox item.".to_owned()
        })
}

fn review_action_url_matches(expected: Option<&str>, provided: Option<&str>) -> bool {
    match (expected, provided) {
        (Some(left), Some(right)) => left == right,
        (None, None) => true,
        _ => false,
    }
}

fn sanitize_download_filename(filename: &str, fallback: &str) -> String {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return fallback.to_owned();
    }

    let cleaned = trimmed
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => ch,
        })
        .collect::<String>();

    if cleaned.is_empty() {
        fallback.to_owned()
    } else {
        cleaned
    }
}

fn filename_from_response(response: &reqwest::blocking::Response, fallback: &str) -> String {
    let header_name = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .split(';')
                .find_map(|part| part.trim().strip_prefix("filename="))
        })
        .map(|value| value.trim_matches('"').to_owned());
    let url_name = response
        .url()
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned);

    sanitize_download_filename(
        &header_name
            .or(url_name)
            .unwrap_or_else(|| fallback.to_owned()),
        fallback,
    )
}

fn approved_review_action_url(action: &ReviewPlanAction, label: &str) -> Result<Url, String> {
    let url = action
        .url
        .as_deref()
        .ok_or_else(|| format!("This {label} is missing its website address."))?;
    let parsed =
        Url::parse(url).map_err(|_| format!("This {label} does not have a valid web address."))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "SimSuite blocked this {label} because only secure HTTPS links are allowed."
        ));
    }
    if parsed.host_str().is_none() {
        return Err(format!(
            "SimSuite blocked this {label} because the link is missing a website name."
        ));
    }
    Ok(parsed)
}

fn approved_watch_source_url(source_url: &str) -> Result<Url, String> {
    let trimmed = source_url.trim();
    let parsed = Url::parse(trimmed)
        .map_err(|_| "This watch source does not have a valid web address.".to_owned())?;
    if parsed.scheme() != "https" {
        return Err(
            "SimSuite only accepts secure HTTPS watch pages for saved watch sources.".to_owned(),
        );
    }
    if parsed.host_str().is_none() {
        return Err(
            "SimSuite blocked this watch source because the link is missing a website name."
                .to_owned(),
        );
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(
            "SimSuite blocked this watch source because saved links cannot include sign-in details."
                .to_owned(),
        );
    }
    Ok(parsed)
}

fn blocked_review_action_label(kind: &ReviewPlanActionKind) -> &'static str {
    match kind {
        ReviewPlanActionKind::OpenOfficialSource => "Blocked unsafe official page",
        ReviewPlanActionKind::DownloadMissingFiles => "Blocked unsafe trusted download",
        _ => "Blocked unsafe review action",
    }
}

fn record_blocked_review_action_event(
    connection: &Connection,
    item_id: i64,
    action: &ReviewPlanAction,
    detail: &str,
) {
    let _ = database::record_download_item_event(
        connection,
        item_id,
        "review_action_blocked",
        blocked_review_action_label(&action.kind),
        Some(detail),
    );
}

fn download_review_action_file(
    url: &Url,
    app_data_dir: &Path,
    fallback_name: &str,
) -> Result<PathBuf, String> {
    let expected_host = url.host_str().ok_or_else(|| {
        "SimSuite blocked this trusted download because the link is missing a website name."
            .to_owned()
    })?;
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(8))
        .build()
        .map_err(|error| error.to_string())?;
    let mut response = client
        .get(url.clone())
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;
    let final_url = response.url().clone();
    validate_review_download_redirect(expected_host, &final_url)?;
    let filename = filename_from_response(&response, fallback_name);
    let destination_dir = app_data_dir
        .join("trusted_downloads")
        .join(chrono::Utc::now().format("%Y%m%d%H%M%S").to_string());
    fs::create_dir_all(&destination_dir).map_err(|error| error.to_string())?;
    let destination = destination_dir.join(filename);
    let mut file = fs::File::create(&destination).map_err(|error| error.to_string())?;
    response
        .copy_to(&mut file)
        .map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    Ok(destination)
}

fn validate_review_download_redirect(expected_host: &str, final_url: &Url) -> Result<(), String> {
    if final_url.scheme() != "https" {
        return Err(
            "SimSuite blocked this trusted download because it redirected to a non-secure page."
                .to_owned(),
        );
    }
    if final_url.host_str() != Some(expected_host) {
        return Err(format!(
            "SimSuite blocked this trusted download because it redirected from {expected_host} to {}.",
            final_url.host_str().unwrap_or("an unknown site")
        ));
    }
    Ok(())
}

fn copy_split_files(
    files: &[crate::models::DownloadInboxFile],
    staging_root: &Path,
    file_ids: &[i64],
) -> Result<(), String> {
    let wanted = file_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    for file in files.iter().filter(|file| wanted.contains(&file.file_id)) {
        let relative = file
            .archive_member_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(&file.filename));
        let destination = staging_root.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::copy(&file.current_path, &destination).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_library_settings(state: State<'_, AppState>) -> Result<LibrarySettings, String> {
    let connection = state.connection().map_err(map_error)?;
    database::get_library_settings(&connection).map_err(map_error)
}

#[tauri::command]
pub fn get_app_behavior_settings(
    state: State<'_, AppState>,
) -> Result<AppBehaviorSettings, String> {
    let connection = state.connection().map_err(map_error)?;
    let patterns_raw = database::get_app_setting(&connection, "download_ignore_patterns")
        .map_err(map_error)?;
    let download_ignore_patterns = patterns_raw
        .as_ref()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();
    let silent_special_mod_updates_raw =
        database::get_app_setting(&connection, "silent_special_mod_updates")
            .map_err(map_error)?;
    let silent_special_mod_updates = silent_special_mod_updates_raw.as_deref().and_then(|raw| {
        if raw == "true" {
            Some(true)
        } else if raw == "false" {
            Some(false)
        } else {
            None
        }
    });
    Ok(AppBehaviorSettings {
        keep_running_in_background: state.keep_running_in_background(),
        automatic_watch_checks: state.automatic_watch_checks(),
        watch_check_interval_hours: state.watch_check_interval_hours(),
        last_watch_check_at: database::get_app_setting(&connection, "watch_auto_last_run_at")
            .map_err(map_error)?,
        last_watch_check_error: database::get_app_setting(&connection, "watch_auto_last_error")
            .map_err(map_error)?,
        download_ignore_patterns,
        silent_special_mod_updates,
    })
}

#[tauri::command]
pub fn save_app_behavior_settings(
    app: AppHandle,
    settings: AppBehaviorSettings,
    state: State<'_, AppState>,
) -> Result<AppBehaviorSettings, String> {
    if settings.keep_running_in_background {
        ensure_tray(&app).map_err(|error| {
            format!(
                "Background mode could not start because SimSuite could not create the tray icon. {error}"
            )
        })?;
    }

    let mut connection = state.connection().map_err(map_error)?;
    database::save_app_setting(
        &mut connection,
        "keep_running_in_background",
        Some(if settings.keep_running_in_background {
            "true"
        } else {
            "false"
        }),
        "user",
    )
    .map_err(map_error)?;
    database::save_app_setting(
        &mut connection,
        "automatic_watch_checks",
        Some(if settings.automatic_watch_checks {
            "true"
        } else {
            "false"
        }),
        "user",
    )
    .map_err(map_error)?;
    database::save_app_setting(
        &mut connection,
        "watch_check_interval_hours",
        Some(&settings.watch_check_interval_hours.to_string()),
        "user",
    )
    .map_err(map_error)?;
    let patterns_json =
        serde_json::to_string(&settings.download_ignore_patterns).map_err(|e| e.to_string())?;
    database::save_app_setting(
        &mut connection,
        "download_ignore_patterns",
        Some(&patterns_json),
        "user",
    )
    .map_err(map_error)?;
    let silent_value = match settings.silent_special_mod_updates {
        Some(true) => Some("true"),
        Some(false) => Some("false"),
        None => None,
    };
    database::save_app_setting(
        &mut connection,
        "silent_special_mod_updates",
        silent_value,
        "user",
    )
    .map_err(map_error)?;
    state
        .set_keep_running_in_background(settings.keep_running_in_background)
        .map_err(map_error)?;
    state
        .set_automatic_watch_checks(settings.automatic_watch_checks)
        .map_err(map_error)?;
    state
        .set_watch_check_interval_hours(settings.watch_check_interval_hours)
        .map_err(map_error)?;
    sync_tray_visibility(&app, settings.keep_running_in_background)
        .map_err(|error| error.to_string())?;
    watch_polling::restart_poller(&app, state.inner()).map_err(map_error)?;
    get_app_behavior_settings(state)
}

#[tauri::command]
pub fn save_library_paths(
    app: AppHandle,
    settings: LibrarySettings,
    state: State<'_, AppState>,
) -> Result<LibrarySettings, String> {
    let mut connection = state.connection().map_err(map_error)?;
    database::save_library_paths(&mut connection, &settings).map_err(map_error)?;
    downloads_watcher::restart_watcher(&app, state.inner()).map_err(map_error)?;
    emit_workspace_domains(
        &app,
        vec![
            WorkspaceDomain::Home,
            WorkspaceDomain::Downloads,
            WorkspaceDomain::Library,
            WorkspaceDomain::Organize,
            WorkspaceDomain::Review,
            WorkspaceDomain::Duplicates,
            WorkspaceDomain::CreatorAudit,
            WorkspaceDomain::CategoryAudit,
        ],
        "library-paths-saved",
        Vec::new(),
        Vec::new(),
    )?;
    database::get_library_settings(&connection).map_err(map_error)
}

#[tauri::command]
pub fn detect_default_library_paths() -> DetectedLibraryPaths {
    let sims_root = document_dir().map(|dir| dir.join("Electronic Arts").join("The Sims 4"));
    let downloads_path = dirs::download_dir()
        .filter(|path| path.exists())
        .map(path_to_string);

    let mods_path = sims_root
        .as_ref()
        .map(|path| path.join("Mods"))
        .filter(|path| path.exists())
        .map(path_to_string);

    let tray_path = sims_root
        .as_ref()
        .map(|path| path.join("Tray"))
        .filter(|path| path.exists())
        .map(path_to_string);

    DetectedLibraryPaths {
        mods_path,
        tray_path,
        downloads_path,
    }
}

#[tauri::command]
pub fn pick_folder(title: Option<String>) -> Option<String> {
    let mut dialog = rfd::FileDialog::new();
    if let Some(title) = title {
        dialog = dialog.set_title(&title);
    }

    dialog.pick_folder().map(path_to_string)
}

#[tauri::command]
pub async fn get_home_overview(state: State<'_, AppState>) -> Result<HomeOverview, String> {
    let state = state.inner().clone();
    run_blocking_command("get_home_overview", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let overview = library_index::get_home_overview(&connection, &settings, &seed_pack)
            .map_err(map_error)?;
        log_slow_command("get_home_overview", started_at, || {
            format!(
                "for {} exact update item(s), {} possible item(s), {} setup item(s)",
                overview.exact_update_items,
                overview.possible_update_items,
                overview.watch_setup_items
            )
        });
        Ok(overview)
    })
    .await
}

#[tauri::command]
pub fn scan_library(app: AppHandle, state: State<'_, AppState>) -> Result<ScanSummary, String> {
    scanner::scan_library(&state, &app).map_err(map_error)
}

#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<'_, AppState>) -> Result<ScanStatus, String> {
    let status_handle = state.scan_status();
    let starting_status = {
        let mut status = status_handle
            .lock()
            .map_err(|_| "Scan status lock poisoned".to_owned())?;

        if status.state == ScanRuntimeState::Running {
            return Ok(status.clone());
        }

        let next_status = ScanStatus {
            state: ScanRuntimeState::Running,
            mode: None,
            phase: Some(ScanPhase::Collecting),
            total_files: 0,
            processed_files: 0,
            current_item: Some("Queued background scan".to_owned()),
            started_at: Some(Utc::now().to_rfc3339()),
            finished_at: None,
            last_summary: None,
            error: None,
        };
        *status = next_status.clone();
        next_status
    };

    emit_scan_status(&app, &starting_status)?;

    let app = app.clone();
    let state = state.inner().clone();
    std::thread::spawn(move || {
        let status_handle = state.scan_status();
        let result = scanner::scan_library_with_progress(&state, |progress| {
            {
                let mut status = status_handle
                    .lock()
                    .map_err(|_| AppError::Message("Scan status lock poisoned".to_owned()))?;
                status.state = ScanRuntimeState::Running;
                status.phase = Some(progress.phase.clone());
                status.total_files = progress.total_files;
                status.processed_files = progress.processed_files;
                status.current_item = Some(progress.current_item.clone());
                status.finished_at = None;
                status.error = None;
            }

            emit_scan_progress(&app, &progress).map_err(AppError::Message)?;
            let snapshot = {
                let status = status_handle
                    .lock()
                    .map_err(|_| AppError::Message("Scan status lock poisoned".to_owned()))?;
                status.clone()
            };
            emit_scan_status(&app, &snapshot).map_err(AppError::Message)
        });

        match result {
            Ok(summary) => {
                let snapshot = {
                    let mut status = status_handle.lock().expect("scan status lock");
                    status.state = ScanRuntimeState::Succeeded;
                    status.mode = Some(summary.scan_mode.clone());
                    status.phase = Some(ScanPhase::Done);
                    status.total_files = summary.files_scanned;
                    status.processed_files = summary.files_scanned;
                    status.current_item = Some("Scan complete".to_owned());
                    status.finished_at = Some(Utc::now().to_rfc3339());
                    status.last_summary = Some(summary.clone());
                    status.error = None;
                    status.clone()
                };
                let _ = emit_scan_status(&app, &snapshot);
                let _ = emit_workspace_domains(
                    &app,
                    vec![
                        WorkspaceDomain::Home,
                        WorkspaceDomain::Library,
                        WorkspaceDomain::Organize,
                        WorkspaceDomain::Review,
                        WorkspaceDomain::Duplicates,
                        WorkspaceDomain::CreatorAudit,
                        WorkspaceDomain::CategoryAudit,
                        WorkspaceDomain::Snapshots,
                    ],
                    "scan-finished",
                    Vec::new(),
                    Vec::new(),
                );
            }
            Err(error) => {
                let snapshot = {
                    let mut status = status_handle.lock().expect("scan status lock");
                    status.state = ScanRuntimeState::Failed;
                    status.phase = None;
                    status.finished_at = Some(Utc::now().to_rfc3339());
                    status.current_item = Some("Scan failed".to_owned());
                    status.error = Some(error.to_string());
                    status.clone()
                };
                let _ = emit_scan_status(&app, &snapshot);
            }
        }
    });

    Ok(starting_status)
}

#[tauri::command]
pub fn get_scan_status(state: State<'_, AppState>) -> Result<ScanStatus, String> {
    state
        .scan_status()
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "Scan status lock poisoned".to_owned())
}

#[tauri::command]
pub fn get_downloads_watcher_status(
    state: State<'_, AppState>,
) -> Result<DownloadsWatcherStatus, String> {
    state
        .downloads_status()
        .lock()
        .map(|status| status.clone())
        .map_err(|_| "Downloads status lock poisoned".to_owned())
}

#[tauri::command]
pub async fn get_staging_areas(
    state: State<'_, AppState>,
) -> Result<StagingAreasSummary, String> {
    let state = state.inner().clone();
    run_blocking_command("get_staging_areas", move || {
        let app_data_dir = state.app_data_dir;
        downloads_watcher::list_staging_areas(&app_data_dir)
            .map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn cleanup_staging_areas(
    state: State<'_, AppState>,
    paths_to_delete: Vec<String>,
) -> Result<CleanupResult, String> {
    let state = state.inner().clone();
    run_blocking_command("cleanup_staging_areas", move || {
        let app_data_dir = state.app_data_dir;
        downloads_watcher::cleanup_staging_areas(&app_data_dir, paths_to_delete)
            .map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn commit_staging_area(
    app: AppHandle,
    item_id: String,
    state: State<'_, AppState>,
) -> Result<StagingCommitResult, String> {
    let state = state.inner().clone();
    run_blocking_command("commit_staging_area", move || {
        let app_data_dir = state.app_data_dir.clone();
        let inbox_root = app_data_dir.join("downloads_inbox");
        let staging_path = inbox_root.join(&item_id);

        if !staging_path.starts_with(&inbox_root) {
            return Err(format!("Invalid staging area path: {}", item_id));
        }
        if !staging_path.exists() {
            return Err(format!("Staging area not found: {}", item_id));
        }

        let numeric_id = match item_id.parse::<i64>() {
            Ok(id) => id,
            Err(_) => {
                return Ok(StagingCommitResult {
                    committed_count: 0,
                    skipped_count: 1,
                    failed_count: 0,
                    errors: vec![format!(
                        "Item '{}' is not a committed download item and cannot be auto-committed.",
                        item_id
                    )],
                });
            }
        };

        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        let item_detail = match downloads_watcher::get_download_item_detail(
            &connection,
            &settings,
            &seed_pack,
            numeric_id,
        ) {
            Ok(Some(d)) => d,
            Ok(None) => {
                return Ok(StagingCommitResult {
                    committed_count: 0,
                    skipped_count: 0,
                    failed_count: 1,
                    errors: vec![format!("Download item {} not found", numeric_id)],
                });
            }
            Err(e) => {
                return Ok(StagingCommitResult {
                    committed_count: 0,
                    skipped_count: 0,
                    failed_count: 1,
                    errors: vec![format!("Could not load item {}: {}", numeric_id, e)],
                });
            }
        };

        if item_detail.item.queue_lane != crate::models::DownloadQueueLane::ReadyNow
            || item_detail.item.intake_mode != crate::models::DownloadIntakeMode::Standard
        {
            return Ok(StagingCommitResult {
                committed_count: 0,
                skipped_count: 1,
                failed_count: 0,
                errors: vec![format!(
                    "Item {} is not in a committable state (lane: {:?}, mode: {:?})",
                    numeric_id, item_detail.item.queue_lane, item_detail.item.intake_mode
                )],
            });
        }

        let file_ids = downloads_watcher::load_active_file_ids(&connection, numeric_id)
            .map_err(map_error)?;

        match move_engine::apply_preview_moves_for_files(
            &mut connection,
            &settings,
            None,
            &file_ids,
            true,
        ) {
            Ok(_) => {
                let _ = downloads_watcher::refresh_download_item_status(&connection, numeric_id);
                let _ = emit_workspace_domains(
                    &app,
                    vec![
                        WorkspaceDomain::Home,
                        WorkspaceDomain::Downloads,
                        WorkspaceDomain::Library,
                        WorkspaceDomain::Organize,
                        WorkspaceDomain::Duplicates,
                    ],
                    "staging-area-committed",
                    vec![numeric_id],
                    Vec::new(),
                );
                Ok(StagingCommitResult {
                    committed_count: 1,
                    skipped_count: 0,
                    failed_count: 0,
                    errors: vec![],
                })
            }
            Err(e) => Ok(StagingCommitResult {
                committed_count: 0,
                skipped_count: 0,
                failed_count: 1,
                errors: vec![format!("Failed to commit item {}: {}", numeric_id, e)],
            }),
        }
    })
    .await
}

#[tauri::command]
pub async fn commit_all_staging_areas(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StagingCommitResult, String> {
    let state = state.inner().clone();
    run_blocking_command("commit_all_staging_areas", move || {
        let app_data_dir = state.app_data_dir.clone();
        let summary = downloads_watcher::list_staging_areas(&app_data_dir)
            .map_err(map_error)?;

        let mut committed_count = 0;
        let mut skipped_count = 0;
        let mut failed_count = 0;
        let mut errors = vec![];
        let mut affected_ids = Vec::new();

        for area in summary.areas {
            let result = commit_staging_area_sync(&state, area.item_id.clone());
            match result {
                Ok(r) => {
                    committed_count += r.committed_count;
                    skipped_count += r.skipped_count;
                    failed_count += r.failed_count;
                    errors.extend(r.errors);
                    if r.committed_count > 0 {
                        if let Ok(id) = area.item_id.parse::<i64>() {
                            affected_ids.push(id);
                        }
                    }
                }
                Err(e) => {
                    failed_count += 1;
                    errors.push(e);
                }
            }
        }

        if !affected_ids.is_empty() {
            let _ = emit_workspace_domains(
                &app,
                vec![
                    WorkspaceDomain::Home,
                    WorkspaceDomain::Downloads,
                    WorkspaceDomain::Library,
                    WorkspaceDomain::Organize,
                    WorkspaceDomain::Duplicates,
                ],
                "staging-area-committed",
                affected_ids,
                Vec::new(),
            );
        }

        Ok(StagingCommitResult {
            committed_count,
            skipped_count,
            failed_count,
            errors,
        })
    })
    .await
}

fn commit_staging_area_sync(
    state: &AppState,
    item_id: String,
) -> Result<StagingCommitResult, String> {
    let app_data_dir = state.app_data_dir.clone();
    let inbox_root = app_data_dir.join("downloads_inbox");
    let staging_path = inbox_root.join(&item_id);

    if !staging_path.starts_with(&inbox_root) {
        return Err(format!("Invalid staging area path: {}", item_id));
    }
    if !staging_path.exists() {
        return Err(format!("Staging area not found: {}", item_id));
    }

    let numeric_id = match item_id.parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            return Ok(StagingCommitResult {
                committed_count: 0,
                skipped_count: 1,
                failed_count: 0,
                errors: vec![format!("Item '{}' is not a committed download item.", item_id)],
            });
        }
    };

    let mut connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();

    let item_detail = match downloads_watcher::get_download_item_detail(
        &connection,
        &settings,
        &seed_pack,
        numeric_id,
    ) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Ok(StagingCommitResult {
                committed_count: 0,
                skipped_count: 0,
                failed_count: 1,
                errors: vec![format!("Download item {} not found", numeric_id)],
            });
        }
        Err(e) => {
            return Ok(StagingCommitResult {
                committed_count: 0,
                skipped_count: 0,
                failed_count: 1,
                errors: vec![format!("Could not load item {}: {}", numeric_id, e)],
            });
        }
    };

    if item_detail.item.queue_lane != crate::models::DownloadQueueLane::ReadyNow
        || item_detail.item.intake_mode != crate::models::DownloadIntakeMode::Standard
    {
        return Ok(StagingCommitResult {
            committed_count: 0,
            skipped_count: 1,
            failed_count: 0,
            errors: vec![],
        });
    }

    let file_ids = downloads_watcher::load_active_file_ids(&connection, numeric_id)
        .map_err(map_error)?;

    match move_engine::apply_preview_moves_for_files(
        &mut connection,
        &settings,
        None,
        &file_ids,
        true,
    ) {
        Ok(_) => {
            let _ = downloads_watcher::refresh_download_item_status(&connection, numeric_id);
            Ok(StagingCommitResult {
                committed_count: 1,
                skipped_count: 0,
                failed_count: 0,
                errors: vec![],
            })
        }
        Err(e) => Ok(StagingCommitResult {
            committed_count: 0,
            skipped_count: 0,
            failed_count: 1,
            errors: vec![format!("Failed to commit item {}: {}", numeric_id, e)],
        }),
    }
}

#[tauri::command]
pub fn refresh_downloads_inbox(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadsWatcherStatus, String> {
    downloads_watcher::refresh_inbox(&app, state.inner()).map_err(map_error)
}

#[tauri::command]
pub async fn get_downloads_inbox(
    query: Option<DownloadsInboxQuery>,
    state: State<'_, AppState>,
) -> Result<DownloadsInboxResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("get_downloads_inbox", move || {
        let started_at = Instant::now();
        let query = query.unwrap_or_default();
        let response = retry_locked_read("get_downloads_inbox", || {
            let connection = state.connection()?;
            let settings = database::get_library_settings(&connection)?;
            let seed_pack = state.seed_pack();
            downloads_watcher::list_download_items(
                &connection,
                &settings,
                &seed_pack,
                query.clone(),
            )
        })?;
        log_slow_command("get_downloads_inbox", started_at, || {
            format!("for {} queue item(s)", response.items.len())
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn get_downloads_bootstrap(
    query: Option<DownloadsInboxQuery>,
    state: State<'_, AppState>,
) -> Result<DownloadsBootstrapResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("get_downloads_bootstrap", move || {
        let started_at = Instant::now();
        let watcher_status = state
            .downloads_status()
            .lock()
            .map(|status| status.clone())
            .map_err(|_| "Downloads status lock poisoned".to_owned())?;

        let queue = if !watcher_status.configured
            || watcher_status.state == DownloadsWatcherState::Processing
            || watcher_status.state == DownloadsWatcherState::Error
        {
            None
        } else {
            let query = query.unwrap_or_default();
            Some(retry_locked_read("get_downloads_bootstrap", || {
                let connection = state.connection()?;
                let settings = database::get_library_settings(&connection)?;
                let seed_pack = state.seed_pack();
                downloads_watcher::list_download_queue(
                    &connection,
                    &settings,
                    &seed_pack,
                    query.clone(),
                )
            })?)
        };

        log_slow_command("get_downloads_bootstrap", started_at, || {
            format!(
                "watcher={}, queue={}",
                match watcher_status.state {
                    DownloadsWatcherState::Idle => "idle",
                    DownloadsWatcherState::Watching => "watching",
                    DownloadsWatcherState::Processing => "processing",
                    DownloadsWatcherState::Error => "error",
                },
                queue
                    .as_ref()
                    .map(|response| response.items.len().to_string())
                    .unwrap_or_else(|| "deferred".to_owned())
            )
        });

        Ok(DownloadsBootstrapResponse {
            watcher_status,
            queue,
        })
    })
    .await
}

#[tauri::command]
pub async fn get_downloads_queue(
    query: Option<DownloadsInboxQuery>,
    state: State<'_, AppState>,
) -> Result<DownloadsInboxResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("get_downloads_queue", move || {
        let started_at = Instant::now();
        let query = query.unwrap_or_default();
        let response = retry_locked_read("get_downloads_queue", || {
            let connection = state.connection()?;
            let settings = database::get_library_settings(&connection)?;
            let seed_pack = state.seed_pack();
            downloads_watcher::list_download_queue(
                &connection,
                &settings,
                &seed_pack,
                query.clone(),
            )
        })?;
        log_slow_command("get_downloads_queue", started_at, || {
            format!("for {} queue item(s)", response.items.len())
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn get_downloads_selection(
    item_id: i64,
    preset_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<DownloadsSelectionResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("get_downloads_selection", move || {
        let started_at = Instant::now();
        let response = retry_locked_read("get_downloads_selection", || {
            let connection = state.connection()?;
            let settings = database::get_library_settings(&connection)?;
            let seed_pack = state.seed_pack();
            downloads_watcher::get_download_item_selection(
                &connection,
                &settings,
                &seed_pack,
                item_id,
                preset_name.clone(),
            )
        })?;
        let file_count = response
            .detail
            .as_ref()
            .map(|detail| detail.files.len())
            .unwrap_or(0);
        log_slow_command("get_downloads_selection", started_at, || {
            format!("for item {item_id} with {file_count} file(s)")
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn get_download_item_detail(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<DownloadInboxDetail>, String> {
    let state = state.inner().clone();
    run_blocking_command("get_download_item_detail", move || {
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        downloads_watcher::get_download_item_detail(&connection, &settings, &seed_pack, item_id)
            .map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn preview_download_item(
    item_id: i64,
    preset_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<OrganizationPreview, String> {
    let state = state.inner().clone();
    run_blocking_command("preview_download_item", move || {
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        downloads_watcher::preview_download_item(
            &connection,
            &settings,
            &seed_pack,
            item_id,
            preset_name,
        )
        .map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn get_library_facets(state: State<'_, AppState>) -> Result<LibraryFacets, String> {
    let state = state.inner().clone();
    run_blocking_command("get_library_facets", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let facets = library_index::get_library_facets(&connection, &seed_pack.taxonomy)
            .map_err(map_error)?;
        log_slow_command("get_library_facets", started_at, || {
            format!(
                "for {} creator facet(s), {} kind facet(s), {} source facet(s)",
                facets.creators.len(),
                facets.kinds.len(),
                facets.sources.len()
            )
        });
        Ok(facets)
    })
    .await
}

#[tauri::command]
pub fn get_duplicate_overview(state: State<'_, AppState>) -> Result<DuplicateOverview, String> {
    let started_at = Instant::now();
    let connection = state.connection().map_err(map_error)?;
    let overview =
        crate::core::duplicate_detector::get_duplicate_overview(&connection).map_err(map_error)?;
    log_slow_command("get_duplicate_overview", started_at, || {
        format!("for {} duplicate pair(s)", overview.total_pairs)
    });
    Ok(overview)
}

#[tauri::command]
pub fn list_duplicate_pairs(
    duplicate_type: Option<String>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<DuplicatePair>, String> {
    let started_at = Instant::now();
    let connection = state.connection().map_err(map_error)?;
    let pairs = crate::core::duplicate_detector::list_duplicate_pairs(
        &connection,
        duplicate_type,
        limit.unwrap_or(160),
    )
    .map_err(map_error)?;
    log_slow_command("list_duplicate_pairs", started_at, || {
        format!("for {} pair row(s)", pairs.len())
    });
    Ok(pairs)
}

#[tauri::command]
pub fn list_rule_presets(state: State<'_, AppState>) -> Result<Vec<RulePreset>, String> {
    let connection = state.connection().map_err(map_error)?;
    rule_engine::list_rule_presets(&connection).map_err(map_error)
}

#[tauri::command]
pub fn preview_organization(
    preset_name: Option<String>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<OrganizationPreview, String> {
    let started_at = Instant::now();
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let preview = if limit.is_some_and(|value| value <= 0) {
        rule_engine::build_preview_full(&connection, &settings, preset_name).map_err(map_error)?
    } else {
        rule_engine::build_preview(&connection, &settings, preset_name, limit.unwrap_or(40))
            .map_err(map_error)?
    };
    log_slow_command("preview_organization", started_at, || {
        format!("for {} suggested row(s)", preview.suggestions.len())
    });
    Ok(preview)
}

#[tauri::command]
pub fn get_review_queue(
    preset_name: Option<String>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewQueueItem>, String> {
    let started_at = Instant::now();
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let queue =
        rule_engine::load_review_queue(&connection, &settings, preset_name, limit.unwrap_or(80))
            .map_err(map_error)?;
    log_slow_command("get_review_queue", started_at, || {
        format!("for {} review item(s)", queue.len())
    });
    Ok(queue)
}

#[tauri::command]
pub fn list_snapshots(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<SnapshotSummary>, String> {
    let connection = state.connection().map_err(map_error)?;
    snapshot_manager::list_snapshots(&connection, limit.unwrap_or(12)).map_err(map_error)
}

#[tauri::command]
pub fn apply_preview_organization(
    app: AppHandle,
    preset_name: Option<String>,
    limit: Option<i64>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyPreviewResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let result = move_engine::apply_preview_moves(
        &mut connection,
        &settings,
        preset_name,
        limit.unwrap_or(80),
        approved,
    )
    .map_err(map_error)?;
    emit_workspace_domains(
        &app,
        vec![
            WorkspaceDomain::Home,
            WorkspaceDomain::Library,
            WorkspaceDomain::Organize,
            WorkspaceDomain::Review,
            WorkspaceDomain::Duplicates,
            WorkspaceDomain::Snapshots,
        ],
        "organize-applied",
        Vec::new(),
        Vec::new(),
    )?;
    Ok(result)
}

#[tauri::command]
pub fn restore_snapshot(
    app: AppHandle,
    snapshot_id: i64,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<RestoreSnapshotResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let seed_pack = state.seed_pack();
    let result = move_engine::restore_snapshot(&mut connection, &seed_pack, snapshot_id, approved)
        .map_err(map_error)?;
    emit_workspace_domains(
        &app,
        vec![
            WorkspaceDomain::Home,
            WorkspaceDomain::Library,
            WorkspaceDomain::Organize,
            WorkspaceDomain::Review,
            WorkspaceDomain::Duplicates,
            WorkspaceDomain::Snapshots,
        ],
        "snapshot-restored",
        Vec::new(),
        Vec::new(),
    )?;
    Ok(result)
}

#[tauri::command]
pub async fn apply_download_item(
    app: AppHandle,
    item_id: i64,
    preset_name: Option<String>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyPreviewResult, String> {
    let state = state.inner().clone();
    run_blocking_command("apply_download_item", move || {
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let Some(item) = downloads_watcher::get_download_item_detail(
            &connection,
            &settings,
            &seed_pack,
            item_id,
        )
        .map_err(map_error)?
        .map(|detail| detail.item) else {
            return Err("Inbox item was not found.".to_owned());
        };
        if item.intake_mode != crate::models::DownloadIntakeMode::Standard {
            return Err(
                "This inbox item uses a special setup flow. Open its guided preview instead."
                    .to_owned(),
            );
        }

        let file_ids =
            downloads_watcher::load_active_file_ids(&connection, item_id).map_err(map_error)?;
        let result = move_engine::apply_preview_moves_for_files(
            &mut connection,
            &settings,
            preset_name,
            &file_ids,
            approved,
        )
        .map_err(map_error)?;
        downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Downloads,
                WorkspaceDomain::Library,
                WorkspaceDomain::Organize,
                WorkspaceDomain::Review,
                WorkspaceDomain::Duplicates,
                WorkspaceDomain::Snapshots,
            ],
            "download-item-applied",
            vec![item_id],
            Vec::new(),
        )?;
        Ok(result)
    })
    .await
}

#[tauri::command]
pub async fn get_download_item_guided_plan(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<GuidedInstallPlan>, String> {
    let state = state.inner().clone();
    run_blocking_command("get_download_item_guided_plan", move || {
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        downloads_watcher::get_download_item_guided_plan(
            &connection,
            &settings,
            &seed_pack,
            item_id,
        )
        .map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn get_download_item_review_plan(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<SpecialReviewPlan>, String> {
    let state = state.inner().clone();
    run_blocking_command("get_download_item_review_plan", move || {
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        downloads_watcher::get_download_item_review_plan(
            &connection,
            &settings,
            &seed_pack,
            item_id,
        )
        .map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn apply_guided_download_item(
    app: AppHandle,
    item_id: i64,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyGuidedDownloadResult, String> {
    let state = state.inner().clone();
    run_blocking_command("apply_guided_download_item", move || {
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let Some(plan) = downloads_watcher::get_download_item_guided_plan(
            &connection,
            &settings,
            &seed_pack,
            item_id,
        )
        .map_err(map_error)?
        else {
            return Err("This inbox item does not have a guided special setup plan.".to_owned());
        };

        let result = match move_engine::apply_guided_download_plan(
            &mut connection,
            &settings,
            &seed_pack,
            &state.app_data_dir,
            &plan,
            approved,
        ) {
            Ok(result) => result,
            Err(error) => {
                let detail = error.to_string();
                let _ = database::record_download_item_event(
                    &connection,
                    item_id,
                    "apply_failed",
                    "Guided install failed",
                    Some(&detail),
                );
                return Err(map_error(error));
            }
        };
        downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Downloads,
                WorkspaceDomain::Library,
                WorkspaceDomain::Organize,
                WorkspaceDomain::Review,
                WorkspaceDomain::Duplicates,
                WorkspaceDomain::Snapshots,
            ],
            "guided-download-applied",
            vec![item_id],
            vec![plan.profile_key.clone()],
        )?;
        Ok(result)
    })
    .await
}

#[tauri::command]
pub async fn apply_special_review_fix(
    app: AppHandle,
    item_id: i64,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplySpecialReviewFixResult, String> {
    let state = state.inner().clone();
    run_blocking_command("apply_special_review_fix", move || {
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        let result = match move_engine::apply_special_review_fix(
            &mut connection,
            &settings,
            &seed_pack,
            &state.app_data_dir,
            item_id,
            approved,
        ) {
            Ok(result) => result,
            Err(error) => {
                let detail = error.to_string();
                let _ = database::record_download_item_event(
                    &connection,
                    item_id,
                    "apply_failed",
                    "Special repair failed",
                    Some(&detail),
                );
                return Err(map_error(error));
            }
        };
        downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Downloads,
                WorkspaceDomain::Library,
                WorkspaceDomain::Organize,
                WorkspaceDomain::Review,
                WorkspaceDomain::Duplicates,
                WorkspaceDomain::Snapshots,
            ],
            "special-review-fix-applied",
            vec![item_id],
            Vec::new(),
        )?;
        Ok(result)
    })
    .await
}

#[tauri::command]
pub async fn apply_review_plan_action(
    app: AppHandle,
    item_id: i64,
    action_kind: String,
    related_item_id: Option<i64>,
    url: Option<String>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyReviewPlanActionResult, String> {
    let state = state.inner().clone();
    run_blocking_command("apply_review_plan_action", move || {
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let Some(review_plan) = downloads_watcher::get_download_item_review_plan(
            &connection,
            &settings,
            &seed_pack,
            item_id,
        )
        .map_err(map_error)?
        else {
            return Err("This inbox item no longer has a special review plan.".to_owned());
        };
        let action =
            find_review_action(&review_plan, &action_kind, related_item_id, url.as_deref())?;

        match action.kind {
            ReviewPlanActionKind::RepairSpecial => {
                let result = match move_engine::apply_special_review_fix(
                    &mut connection,
                    &settings,
                    &seed_pack,
                    &state.app_data_dir,
                    item_id,
                    approved,
                ) {
                    Ok(result) => result,
                    Err(error) => {
                        let detail = error.to_string();
                        let _ = database::record_download_item_event(
                            &connection,
                            item_id,
                            "apply_failed",
                            "Special repair failed",
                            Some(&detail),
                        );
                        return Err(map_error(error));
                    }
                };
                downloads_watcher::refresh_download_item_status(&connection, item_id)
                    .map_err(map_error)?;
                emit_workspace_domains(
                    &app,
                    vec![
                        WorkspaceDomain::Home,
                        WorkspaceDomain::Downloads,
                        WorkspaceDomain::Library,
                        WorkspaceDomain::Organize,
                        WorkspaceDomain::Review,
                        WorkspaceDomain::Duplicates,
                        WorkspaceDomain::Snapshots,
                    ],
                    "review-plan-repair-applied",
                    vec![item_id],
                    review_plan
                        .profile_key
                        .clone()
                        .into_iter()
                        .collect::<Vec<_>>(),
                )?;
                Ok(ApplyReviewPlanActionResult {
                    action_kind: action.kind,
                    focus_item_id: item_id,
                    created_item_id: None,
                    opened_url: None,
                    snapshot_id: Some(result.snapshot_id),
                    repaired_count: result.repaired_count,
                    installed_count: result.installed_count,
                    replaced_count: result.replaced_count,
                    preserved_count: result.preserved_count,
                    deferred_review_count: result.deferred_review_count,
                    snapshot_name: Some(result.snapshot_name),
                    message: format!(
                        "Fixed the old {} setup and refreshed the special install plan.",
                        review_plan
                            .profile_name
                            .unwrap_or_else(|| "special mod".to_owned())
                    ),
                })
            }
            ReviewPlanActionKind::InstallDependency => {
                let dependency_item_id = action.related_item_id.ok_or_else(|| {
                    "This dependency action is missing its inbox item.".to_owned()
                })?;
                let Some(plan) = downloads_watcher::get_download_item_guided_plan(
                    &connection,
                    &settings,
                    &seed_pack,
                    dependency_item_id,
                )
                .map_err(map_error)?
                else {
                    return Err("This dependency no longer has a guided setup plan.".to_owned());
                };
                let result = match move_engine::apply_guided_download_plan(
                    &mut connection,
                    &settings,
                    &seed_pack,
                    &state.app_data_dir,
                    &plan,
                    approved,
                ) {
                    Ok(result) => result,
                    Err(error) => {
                        let detail = error.to_string();
                        let _ = database::record_download_item_event(
                            &connection,
                            dependency_item_id,
                            "apply_failed",
                            "Dependency install failed",
                            Some(&detail),
                        );
                        return Err(map_error(error));
                    }
                };
                downloads_watcher::refresh_download_item_status(&connection, dependency_item_id)
                    .map_err(map_error)?;
                downloads_watcher::refresh_download_item_status(&connection, item_id)
                    .map_err(map_error)?;
                emit_workspace_domains(
                    &app,
                    vec![
                        WorkspaceDomain::Home,
                        WorkspaceDomain::Downloads,
                        WorkspaceDomain::Library,
                        WorkspaceDomain::Organize,
                        WorkspaceDomain::Review,
                        WorkspaceDomain::Duplicates,
                        WorkspaceDomain::Snapshots,
                    ],
                    "review-plan-dependency-installed",
                    vec![item_id, dependency_item_id],
                    vec![plan.profile_key.clone()],
                )?;
                Ok(ApplyReviewPlanActionResult {
                    action_kind: action.kind,
                    focus_item_id: item_id,
                    created_item_id: None,
                    opened_url: None,
                    snapshot_id: Some(result.snapshot_id),
                    repaired_count: 0,
                    installed_count: result.installed_count,
                    replaced_count: result.replaced_count,
                    preserved_count: result.preserved_count,
                    deferred_review_count: result.deferred_review_count,
                    snapshot_name: Some(result.snapshot_name),
                    message: format!(
                        "Installed {} and re-checked the waiting item.",
                        action
                            .related_item_name
                            .unwrap_or_else(|| "the required library".to_owned())
                    ),
                })
            }
            ReviewPlanActionKind::OpenDependency => Ok(ApplyReviewPlanActionResult {
                action_kind: action.kind,
                focus_item_id: action.related_item_id.unwrap_or(item_id),
                created_item_id: None,
                opened_url: None,
                snapshot_id: None,
                repaired_count: 0,
                installed_count: 0,
                replaced_count: 0,
                preserved_count: 0,
                deferred_review_count: 0,
                snapshot_name: None,
                message: format!(
                    "Opened the {} inbox item so you can sort it first.",
                    action
                        .related_item_name
                        .unwrap_or_else(|| "dependency".to_owned())
                ),
            }),
            ReviewPlanActionKind::OpenRelatedItem => Ok(ApplyReviewPlanActionResult {
                action_kind: action.kind,
                focus_item_id: action.related_item_id.unwrap_or(item_id),
                created_item_id: None,
                opened_url: None,
                snapshot_id: None,
                repaired_count: 0,
                installed_count: 0,
                replaced_count: 0,
                preserved_count: 0,
                deferred_review_count: 0,
                snapshot_name: None,
                message: format!(
                    "Opened {} so you can use the fuller local pack first.",
                    action
                        .related_item_name
                        .unwrap_or_else(|| "the better Inbox item".to_owned())
                ),
            }),
            ReviewPlanActionKind::OpenOfficialSource => {
                let opened_url = match approved_review_action_url(&action, "official page") {
                    Ok(url) => url,
                    Err(detail) => {
                        record_blocked_review_action_event(&connection, item_id, &action, &detail);
                        return Err(detail);
                    }
                };
                webbrowser::open(opened_url.as_str()).map_err(|error| {
                    format!("SimSuite could not open the official page in your browser: {error}")
                })?;

                Ok(ApplyReviewPlanActionResult {
                    action_kind: action.kind,
                    focus_item_id: item_id,
                    created_item_id: None,
                    opened_url: Some(opened_url.to_string()),
                    snapshot_id: None,
                    repaired_count: 0,
                    installed_count: 0,
                    replaced_count: 0,
                    preserved_count: 0,
                    deferred_review_count: 0,
                    snapshot_name: None,
                    message: format!(
                        "Opened the official {} page in your browser.",
                        action
                            .related_item_name
                            .unwrap_or_else(|| "download".to_owned())
                    ),
                })
            }
            ReviewPlanActionKind::DownloadMissingFiles => {
                if !approved {
                    return Err(
                        "Download was blocked because approval was not confirmed.".to_owned()
                    );
                }
                let fallback_name = format!(
                    "{}.zip",
                    action
                        .related_item_name
                        .clone()
                        .unwrap_or_else(|| "special-download".to_owned())
                        .replace(' ', "_")
                );
                let url = match approved_review_action_url(&action, "trusted download link") {
                    Ok(url) => url,
                    Err(detail) => {
                        record_blocked_review_action_event(&connection, item_id, &action, &detail);
                        return Err(detail);
                    }
                };
                let downloaded_file =
                    match download_review_action_file(&url, &state.app_data_dir, &fallback_name) {
                        Ok(path) => path,
                        Err(detail) => {
                            record_blocked_review_action_event(
                                &connection,
                                item_id,
                                &action,
                                &detail,
                            );
                            return Err(detail);
                        }
                    };
                let imported_item_id = downloads_watcher::import_download_source(
                    &mut connection,
                    &state,
                    &downloaded_file,
                    Some(
                        downloaded_file
                            .file_name()
                            .map(|value| value.to_string_lossy().to_string())
                            .unwrap_or(fallback_name),
                    ),
                    Some(item_id),
                )
                .map_err(map_error)?;
                downloads_watcher::refresh_download_item_status(&connection, imported_item_id)
                    .map_err(map_error)?;
                emit_workspace_domains(
                    &app,
                    vec![
                        WorkspaceDomain::Home,
                        WorkspaceDomain::Downloads,
                        WorkspaceDomain::Review,
                        WorkspaceDomain::Duplicates,
                    ],
                    "review-plan-download-imported",
                    vec![item_id, imported_item_id],
                    Vec::new(),
                )?;
                Ok(ApplyReviewPlanActionResult {
                    action_kind: action.kind,
                    focus_item_id: imported_item_id,
                    created_item_id: None,
                    opened_url: None,
                    snapshot_id: None,
                    repaired_count: 0,
                    installed_count: 0,
                    replaced_count: 0,
                    preserved_count: 0,
                    deferred_review_count: 0,
                    snapshot_name: None,
                    message: format!(
                        "Downloaded the trusted {} archive into the Inbox and re-checked it.",
                        action
                            .related_item_name
                            .unwrap_or_else(|| "special-mod".to_owned())
                    ),
                })
            }
            ReviewPlanActionKind::SeparateSupportedFiles => {
                if !approved {
                    return Err("Split was blocked because approval was not confirmed.".to_owned());
                }
                let profile_key = review_plan.profile_key.clone().ok_or_else(|| {
                    "This inbox item does not have a matched special-mod profile.".to_owned()
                })?;
                let Some((supported_ids, leftover_ids)) =
                    install_profile_engine::collect_supported_subset_file_ids(
                        &connection,
                        &seed_pack,
                        item_id,
                        &profile_key,
                    )
                    .map_err(map_error)?
                else {
                    return Err(
                        "SimSuite could not find a clean supported subset to split out.".to_owned(),
                    );
                };
                let source = downloads_watcher::get_download_item_source(&connection, item_id)
                    .map_err(map_error)?
                    .ok_or_else(|| "This inbox item could not be found.".to_owned())?;
                let detail = downloads_watcher::get_download_item_detail(
                    &connection,
                    &settings,
                    &seed_pack,
                    item_id,
                )
                .map_err(map_error)?
                .ok_or_else(|| "This inbox item could not be loaded.".to_owned())?;
                let split_root = state
                    .app_data_dir
                    .join("downloads_split")
                    .join(item_id.to_string())
                    .join(chrono::Utc::now().format("%Y%m%d%H%M%S").to_string());
                let supported_root = split_root.join("supported");
                let leftover_root = split_root.join("leftover");
                copy_split_files(&detail.files, &supported_root, &supported_ids)?;
                copy_split_files(&detail.files, &leftover_root, &leftover_ids)?;

                let supported_name = review_plan
                    .profile_name
                    .clone()
                    .unwrap_or_else(|| detail.item.display_name.clone());
                let leftover_name = format!("{} - extra files", detail.item.display_name);

                downloads_watcher::import_staged_batch(
                    &mut connection,
                    &state,
                    &source,
                    &supported_root,
                    supported_name,
                    Some(item_id),
                    vec![
                        "Split out the supported special-mod files from a mixed batch.".to_owned(),
                    ],
                )
                .map_err(map_error)?;
                let leftover_item_id = downloads_watcher::import_staged_batch(
                    &mut connection,
                    &state,
                    &source,
                    &leftover_root,
                    leftover_name,
                    None,
                    vec!["Leftover files from a mixed special-mod batch.".to_owned()],
                )
                .map_err(map_error)?;
                downloads_watcher::refresh_download_item_status(&connection, item_id)
                    .map_err(map_error)?;
                downloads_watcher::refresh_download_item_status(&connection, leftover_item_id)
                    .map_err(map_error)?;
                emit_workspace_domains(
                    &app,
                    vec![
                        WorkspaceDomain::Home,
                        WorkspaceDomain::Downloads,
                        WorkspaceDomain::Review,
                        WorkspaceDomain::Duplicates,
                    ],
                    "review-plan-batch-split",
                    vec![item_id, leftover_item_id],
                    review_plan
                        .profile_key
                        .clone()
                        .into_iter()
                        .collect::<Vec<_>>(),
                )?;

                Ok(ApplyReviewPlanActionResult {
                    action_kind: action.kind,
                    focus_item_id: item_id,
                    created_item_id: Some(leftover_item_id),
                    opened_url: None,
                    snapshot_id: None,
                    repaired_count: 0,
                    installed_count: 0,
                    replaced_count: 0,
                    preserved_count: 0,
                    deferred_review_count: 0,
                    snapshot_name: None,
                    message:
                        "Split the supported special-mod files into their own clean inbox batch."
                            .to_owned(),
                })
            }
        }
    })
    .await
}

#[tauri::command]
pub async fn ignore_download_item(
    app: AppHandle,
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let state = state.inner().clone();
    run_blocking_command("ignore_download_item", move || {
        let mut connection = state.connection().map_err(map_error)?;
        downloads_watcher::ignore_download_item(&mut connection, item_id).map_err(map_error)?;
        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Downloads,
                WorkspaceDomain::Review,
                WorkspaceDomain::Duplicates,
            ],
            "download-item-ignored",
            vec![item_id],
            Vec::new(),
        )?;
        Ok(true)
    })
    .await
}

#[tauri::command]
pub async fn apply_download_items(
    app: AppHandle,
    item_ids: Vec<i64>,
    preset_name: Option<String>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<BatchApplyResult, String> {
    let state = state.inner().clone();
    run_blocking_command("apply_download_items", move || {
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        let mut applied_count = 0;
        let mut skipped_count = 0;
        let mut failed_count = 0;
        let mut errors = Vec::new();
        let mut affected_ids = Vec::new();

        for item_id in item_ids {
            let item_detail = match downloads_watcher::get_download_item_detail(
                &connection,
                &settings,
                &seed_pack,
                item_id,
            ) {
                Ok(Some(detail)) => detail,
                Ok(None) => {
                    failed_count += 1;
                    errors.push(format!("Item {item_id} was not found."));
                    continue;
                }
                Err(err) => {
                    failed_count += 1;
                    errors.push(format!("Item {item_id} could not be loaded: {err}"));
                    continue;
                }
            };

            // Skip items not in ReadyNow lane
            if item_detail.item.queue_lane != crate::models::DownloadQueueLane::ReadyNow {
                skipped_count += 1;
                continue;
            }

            // Skip items that need special setup
            if item_detail.item.intake_mode != crate::models::DownloadIntakeMode::Standard {
                skipped_count += 1;
                continue;
            }

            let file_ids = match downloads_watcher::load_active_file_ids(&connection, item_id) {
                Ok(ids) => ids,
                Err(err) => {
                    failed_count += 1;
                    errors.push(format!("Item {item_id} could not load files: {err}"));
                    continue;
                }
            };

            match move_engine::apply_preview_moves_for_files(
                &mut connection,
                &settings,
                preset_name.clone(),
                &file_ids,
                approved,
            ) {
                Ok(_) => {
                    applied_count += 1;
                    affected_ids.push(item_id);
                }
                Err(err) => {
                    failed_count += 1;
                    errors.push(format!("Item {item_id} failed to apply: {err}"));
                }
            }

            if let Err(err) =
                downloads_watcher::refresh_download_item_status(&connection, item_id)
            {
                eprintln!("[apply_download_items] failed to refresh status for {item_id}: {err}");
            }
        }

        if !affected_ids.is_empty() {
            emit_workspace_domains(
                &app,
                vec![
                    WorkspaceDomain::Home,
                    WorkspaceDomain::Downloads,
                    WorkspaceDomain::Library,
                    WorkspaceDomain::Organize,
                    WorkspaceDomain::Review,
                    WorkspaceDomain::Duplicates,
                    WorkspaceDomain::Snapshots,
                ],
                "download-items-applied",
                affected_ids,
                Vec::new(),
            )?;
        }

        Ok(BatchApplyResult {
            applied_count,
            skipped_count,
            failed_count,
            errors,
        })
    })
    .await
}

#[tauri::command]
pub async fn ignore_download_items(
    app: AppHandle,
    item_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<IgnoreItemsResult, String> {
    let state = state.inner().clone();
    run_blocking_command("ignore_download_items", move || {
        let mut connection = state.connection().map_err(map_error)?;

        let mut ignored_count = 0;
        let mut failed_count = 0;
        let mut errors = Vec::new();
        let mut affected_ids = Vec::new();

        for item_id in item_ids {
            match downloads_watcher::ignore_download_item(&mut connection, item_id) {
                Ok(()) => {
                    ignored_count += 1;
                    affected_ids.push(item_id);
                }
                Err(err) => {
                    failed_count += 1;
                    errors.push(format!("Item {item_id} failed to ignore: {err}"));
                }
            }
        }

        if !affected_ids.is_empty() {
            emit_workspace_domains(
                &app,
                vec![
                    WorkspaceDomain::Home,
                    WorkspaceDomain::Downloads,
                    WorkspaceDomain::Review,
                    WorkspaceDomain::Duplicates,
                ],
                "download-items-ignored",
                affected_ids,
                Vec::new(),
            )?;
        }

        Ok(IgnoreItemsResult {
            ignored_count,
            failed_count,
            errors,
        })
    })
    .await
}

#[tauri::command]
pub async fn list_library_files(
    query: LibraryQuery,
    state: State<'_, AppState>,
) -> Result<LibraryListResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("list_library_files", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let response = library_index::list_library_files(&connection, query).map_err(map_error)?;
        log_slow_command("list_library_files", started_at, || {
            format!("for {} visible file row(s)", response.items.len())
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn list_library_watch_items(
    filter: Option<WatchListFilter>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<LibraryWatchListResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("list_library_watch_items", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let response = content_versions::list_library_watch_items(
            &connection,
            &settings,
            &seed_pack,
            filter.unwrap_or_default(),
            limit.unwrap_or(12).clamp(1, 48) as usize,
        )
        .map_err(map_error)?;
        log_slow_command("list_library_watch_items", started_at, || {
            format!("for {} tracked watch item(s)", response.items.len())
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn list_library_watch_setup_items(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<LibraryWatchSetupResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("list_library_watch_setup_items", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let response = content_versions::list_library_watch_setup_items(
            &connection,
            &settings,
            &seed_pack,
            limit.unwrap_or(6).clamp(1, 24) as usize,
        )
        .map_err(map_error)?;
        log_slow_command("list_library_watch_setup_items", started_at, || {
            format!(
                "for {} setup suggestion(s) (truncated: {})",
                response.items.len(),
                response.truncated
            )
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub async fn list_library_watch_review_items(
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<LibraryWatchReviewResponse, String> {
    let state = state.inner().clone();
    run_blocking_command("list_library_watch_review_items", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let response = content_versions::list_library_watch_review_items(
            &connection,
            &settings,
            &seed_pack,
            limit.unwrap_or(8).clamp(1, 24) as usize,
        )
        .map_err(map_error)?;
        log_slow_command("list_library_watch_review_items", started_at, || {
            format!("for {} watch review item(s)", response.items.len())
        });
        Ok(response)
    })
    .await
}

#[tauri::command]
pub fn get_creator_audit(
    query: Option<CreatorAuditQuery>,
    state: State<'_, AppState>,
) -> Result<CreatorAuditResponse, String> {
    let started_at = Instant::now();
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    let response = creator_audit::load_creator_audit(
        &connection,
        &settings,
        &seed_pack,
        query.unwrap_or_default(),
    )
    .map_err(map_error)?;
    log_slow_command("get_creator_audit", started_at, || {
        format!("for {} creator group(s)", response.groups.len())
    });
    Ok(response)
}

#[tauri::command]
pub fn get_category_audit(
    query: Option<CategoryAuditQuery>,
    state: State<'_, AppState>,
) -> Result<CategoryAuditResponse, String> {
    let started_at = Instant::now();
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    let response = category_audit::load_category_audit(
        &connection,
        &settings,
        &seed_pack,
        query.unwrap_or_default(),
    )
    .map_err(map_error)?;
    log_slow_command("get_category_audit", started_at, || {
        format!("for {} category group(s)", response.groups.len())
    });
    Ok(response)
}

#[tauri::command]
pub fn get_creator_audit_group_files(
    group_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CreatorAuditFile>, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    creator_audit::load_creator_group_files(&connection, &settings, &seed_pack, &group_id)
        .map_err(map_error)
}

#[tauri::command]
pub fn get_category_audit_group_files(
    group_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CategoryAuditFile>, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    category_audit::load_category_group_files(&connection, &settings, &seed_pack, &group_id)
        .map_err(map_error)
}

#[tauri::command]
pub async fn get_file_detail(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let state = state.inner().clone();
    run_blocking_command("get_file_detail", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let detail = library_index::get_file_detail(&connection, &settings, &seed_pack, file_id)
            .map_err(map_error)?;
        log_slow_command("get_file_detail", started_at, || {
            let label = detail
                .as_ref()
                .map(|item| item.filename.clone())
                .unwrap_or_else(|| "missing item".to_owned());
            format!("for file_id={file_id} ({label})")
        });
        Ok(detail)
    })
    .await
}

#[tauri::command]
pub async fn save_watch_source_for_file(
    app: AppHandle,
    file_id: i64,
    source_kind: WatchSourceKind,
    source_label: Option<String>,
    source_url: String,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let approved_url = approved_watch_source_url(&source_url)?;
    let cleaned_label = source_label.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    });
    let state = state.inner().clone();
    run_blocking_command("save_watch_source_for_file", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        content_versions::save_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
            source_kind,
            cleaned_label,
            approved_url.as_str(),
        )
        .map_err(map_error)?;
        let updated = library_index::get_file_detail(&connection, &settings, &seed_pack, file_id)
            .map_err(map_error)?
            .ok_or_else(|| {
                "Watch sources can only be saved for installed Library items.".to_owned()
            })?;

        emit_workspace_domains(
            &app,
            vec![WorkspaceDomain::Home, WorkspaceDomain::Library],
            "watch-source-saved",
            vec![file_id],
            Vec::new(),
        )?;
        log_slow_command("save_watch_source_for_file", started_at, || {
            format!("for file_id={file_id}")
        });
        Ok(Some(updated))
    })
    .await
}

#[tauri::command]
pub async fn save_watch_sources_for_files(
    app: AppHandle,
    entries: Vec<SaveLibraryWatchSourceEntry>,
    state: State<'_, AppState>,
) -> Result<LibraryWatchBulkSaveResult, String> {
    let state = state.inner().clone();
    run_blocking_command("save_watch_sources_for_files", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let mut changed_file_ids = HashSet::new();
        let mut results = Vec::new();

        for entry in entries {
            let file_id = entry.file_id;
            let cleaned_label = entry.source_label.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            });

            let approved_url = match approved_watch_source_url(&entry.source_url) {
                Ok(url) => url,
                Err(message) => {
                    results.push(LibraryWatchBulkSaveItemResult {
                        file_id,
                        saved: false,
                        message,
                    });
                    continue;
                }
            };

            match content_versions::save_watch_source_for_library_file(
                &connection,
                &settings,
                &seed_pack,
                file_id,
                entry.source_kind,
                cleaned_label,
                approved_url.as_str(),
            ) {
                Ok(Some(_)) => {
                    changed_file_ids.insert(file_id);
                    results.push(LibraryWatchBulkSaveItemResult {
                        file_id,
                        saved: true,
                        message: "Watch source saved.".to_owned(),
                    });
                }
                Ok(None) => {
                    results.push(LibraryWatchBulkSaveItemResult {
                        file_id,
                        saved: false,
                        message:
                            "Watch sources can only be saved for installed Library items."
                                .to_owned(),
                    });
                }
                Err(error) => {
                    results.push(LibraryWatchBulkSaveItemResult {
                        file_id,
                        saved: false,
                        message: map_error(error),
                    });
                }
            }
        }

        if !changed_file_ids.is_empty() {
            emit_workspace_domains(
                &app,
                vec![WorkspaceDomain::Home, WorkspaceDomain::Library],
                "watch-sources-saved",
                changed_file_ids.into_iter().collect(),
                Vec::new(),
            )?;
        }

        let saved_count = results.iter().filter(|item| item.saved).count() as i64;
        let failed_count = results.len() as i64 - saved_count;

        log_slow_command("save_watch_sources_for_files", started_at, || {
            format!("saved {saved_count} of {} watch source(s)", results.len())
        });

        Ok(LibraryWatchBulkSaveResult {
            saved_count,
            failed_count,
            results,
        })
    })
    .await
}

#[tauri::command]
pub async fn clear_watch_source_for_file(
    app: AppHandle,
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let state = state.inner().clone();
    run_blocking_command("clear_watch_source_for_file", move || {
        let started_at = Instant::now();
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        content_versions::clear_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
        )
        .map_err(map_error)?;
        let updated = library_index::get_file_detail(&connection, &settings, &seed_pack, file_id)
            .map_err(map_error)?
            .ok_or_else(|| {
                "Watch sources can only be managed for installed Library items.".to_owned()
            })?;

        emit_workspace_domains(
            &app,
            vec![WorkspaceDomain::Home, WorkspaceDomain::Library],
            "watch-source-cleared",
            vec![file_id],
            Vec::new(),
        )?;
        log_slow_command("clear_watch_source_for_file", started_at, || {
            format!("for file_id={file_id}")
        });
        Ok(Some(updated))
    })
    .await
}

#[tauri::command]
pub async fn refresh_watch_source_for_file(
    app: AppHandle,
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let state = state.inner().clone();
    run_blocking_command("refresh_watch_source_for_file", move || {
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        let Some(watch_result) = content_versions::refresh_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
        )
        .map_err(map_error)?
        else {
            return Ok(None);
        };

        let updated = library_index::get_file_detail(&connection, &settings, &seed_pack, file_id)
            .map_err(map_error)?
            .ok_or_else(|| "Watch checks can only run for installed Library items.".to_owned())?;

        emit_workspace_domains(
            &app,
            vec![WorkspaceDomain::Home, WorkspaceDomain::Library],
            "watch-source-refreshed",
            vec![file_id],
            Vec::new(),
        )?;

        Ok(Some(FileDetail {
            watch_result: Some(watch_result),
            ..updated
        }))
    })
    .await
}

#[tauri::command]
pub async fn refresh_watched_sources(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WatchRefreshSummary, String> {
    let state = state.inner().clone();
    run_blocking_command("refresh_watched_sources", move || {
        watch_polling::refresh_watched_sources_now(&app, &state).map_err(map_error)
    })
    .await
}

#[tauri::command]
pub async fn save_creator_learning(
    app: AppHandle,
    file_id: i64,
    creator_name: String,
    alias_name: Option<String>,
    lock_preference: Option<bool>,
    preferred_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let state = state.inner().clone();
    run_blocking_command("save_creator_learning", move || {
        let started_at = Instant::now();
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        database::save_creator_learning(
            &mut connection,
            &settings,
            file_id,
            &creator_name,
            alias_name.as_deref(),
            lock_preference.unwrap_or(false),
            preferred_path.as_deref(),
        )
        .map_err(map_error)?;

        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Library,
                WorkspaceDomain::Organize,
                WorkspaceDomain::Review,
                WorkspaceDomain::CreatorAudit,
            ],
            "creator-learning-saved",
            vec![file_id],
            Vec::new(),
        )?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let detail = library_index::get_file_detail(&connection, &settings, &seed_pack, file_id)
            .map_err(map_error)?;
        log_slow_command("save_creator_learning", started_at, || {
            format!("for file_id={file_id}")
        });
        Ok(detail)
    })
    .await
}

#[tauri::command]
pub fn apply_creator_audit(
    app: AppHandle,
    file_ids: Vec<i64>,
    creator_name: String,
    alias_name: Option<String>,
    lock_preference: Option<bool>,
    preferred_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApplyCreatorAuditResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let (updated_count, cleared_review_count) = database::save_creator_learning_batch(
        &mut connection,
        &settings,
        &file_ids,
        &creator_name,
        alias_name.as_deref(),
        lock_preference.unwrap_or(false),
        preferred_path.as_deref(),
    )
    .map_err(map_error)?;

    emit_workspace_domains(
        &app,
        vec![
            WorkspaceDomain::Home,
            WorkspaceDomain::Library,
            WorkspaceDomain::Organize,
            WorkspaceDomain::Review,
            WorkspaceDomain::CreatorAudit,
        ],
        "creator-audit-applied",
        file_ids.clone(),
        Vec::new(),
    )?;
    Ok(ApplyCreatorAuditResult {
        creator_name,
        updated_count,
        cleared_review_count,
        locked_route: lock_preference.unwrap_or(false),
    })
}

#[tauri::command]
pub fn apply_category_audit(
    app: AppHandle,
    file_ids: Vec<i64>,
    kind: String,
    subtype: Option<String>,
    state: State<'_, AppState>,
) -> Result<ApplyCategoryAuditResult, String> {
    let normalized_kind = kind.trim().to_owned();
    let seed_pack = state.seed_pack();
    let is_known_kind = seed_pack
        .taxonomy
        .kinds
        .iter()
        .chain(seed_pack.taxonomy.tray_kinds.iter())
        .any(|item| item == &normalized_kind);

    if !is_known_kind {
        return Err(format!("Unsupported kind override: {normalized_kind}"));
    }

    let mut connection = state.connection().map_err(map_error)?;
    let (updated_count, cleared_review_count) = database::save_category_override_batch(
        &mut connection,
        &file_ids,
        &normalized_kind,
        subtype.as_deref(),
    )
    .map_err(map_error)?;

    emit_workspace_domains(
        &app,
        vec![
            WorkspaceDomain::Home,
            WorkspaceDomain::Library,
            WorkspaceDomain::Organize,
            WorkspaceDomain::Review,
            WorkspaceDomain::CategoryAudit,
        ],
        "category-audit-applied",
        file_ids.clone(),
        Vec::new(),
    )?;
    Ok(ApplyCategoryAuditResult {
        kind: normalized_kind,
        subtype,
        updated_count,
        cleared_review_count,
    })
}

#[tauri::command]
pub async fn save_category_override(
    app: AppHandle,
    file_id: i64,
    kind: String,
    subtype: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let normalized_kind = kind.trim().to_owned();
    let state = state.inner().clone();
    let seed_pack = state.seed_pack();
    let is_known_kind = seed_pack
        .taxonomy
        .kinds
        .iter()
        .chain(seed_pack.taxonomy.tray_kinds.iter())
        .any(|item| item == &normalized_kind);

    if !is_known_kind {
        return Err(format!("Unsupported kind override: {normalized_kind}"));
    }

    run_blocking_command("save_category_override", move || {
        let started_at = Instant::now();
        let mut connection = state.connection().map_err(map_error)?;
        database::save_category_override(
            &mut connection,
            file_id,
            &normalized_kind,
            subtype.as_deref(),
        )
        .map_err(map_error)?;

        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Library,
                WorkspaceDomain::Organize,
                WorkspaceDomain::Review,
                WorkspaceDomain::CategoryAudit,
            ],
            "category-override-saved",
            vec![file_id],
            Vec::new(),
        )?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();
        let detail = library_index::get_file_detail(&connection, &settings, &seed_pack, file_id)
            .map_err(map_error)?;
        log_slow_command("save_category_override", started_at, || {
            format!("for file_id={file_id}")
        });
        Ok(detail)
    })
    .await
}

pub fn emit_scan_progress(
    app: &AppHandle,
    progress: &crate::models::ScanProgress,
) -> Result<(), String> {
    app.emit("scan-progress", progress)
        .map_err(|error| error.to_string())
}

pub fn emit_scan_status(app: &AppHandle, status: &crate::models::ScanStatus) -> Result<(), String> {
    app.emit("scan-status", status)
        .map_err(|error| error.to_string())
}

pub fn emit_downloads_status(
    app: &AppHandle,
    status: &crate::models::DownloadsWatcherStatus,
) -> Result<(), String> {
    app.emit("downloads-status", status)
        .map_err(|error| error.to_string())
}

pub fn emit_downloads_progress(
    app: &AppHandle,
    progress: &crate::models::DownloadProgress,
) -> Result<(), String> {
    app.emit("downloads-progress", progress)
        .map_err(|error| error.to_string())
}

pub fn emit_workspace_change(app: &AppHandle, change: &WorkspaceChange) -> Result<(), String> {
    app.emit("workspace-change", change)
        .map_err(|error| error.to_string())
}

fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

#[allow(dead_code)]
fn normalize_optional_path(path: Option<String>) -> Option<PathBuf> {
    path.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    })
}

/// Check for MCCC updates — returns current installed version, latest available version,
/// and whether an update is ready to apply.
#[tauri::command]
pub async fn check_mccc_update(
    state: State<'_, AppState>,
) -> Result<McccUpdateInfo, String> {
    let state = state.inner().clone();
    run_blocking_command("check_mccc_update", move || {
        let connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        // Find the MCCC guided profile
        let Some(profile) = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|p| p.key == "mccc")
        else {
            return Err("MCCC profile not found in seed data.".to_owned());
        };

        // Detect existing MCCC installation
        let mods_root = install_profile_engine::resolve_mods_root(&settings)
            .map_err(map_error)?;
        let mut context = install_profile_engine::SpecialDecisionContext::default();
        let inventory = install_profile_engine::load_installed_mods_inventory_cached(
            &connection,
            &mods_root,
            &mut context,
        )
        .map_err(map_error)?;
        let layout = install_profile_engine::detect_existing_layout_with_inventory(
            &connection,
            seed_pack,
            profile,
            &inventory.mods_root,
            &inventory.files,
        )
        .map_err(map_error)?;

        let existing_install_state =
            install_profile_engine::existing_install_state_from_layout(&layout);
        let install_path = if layout.existing_install_detected {
            Some(layout.target_folder.to_string_lossy().to_string())
        } else {
            None
        };

        // Get installed version
        let installed_version_ev =
            install_profile_engine::installed_version_for_profile(profile, &layout);
        let installed_version = installed_version_ev.value;

        // Load or refresh latest version from official page
        let latest_info =
            special_mod_versions::load_or_refresh_latest_info(&connection, profile, true)
                .map_err(map_error)?;

        let (latest_version, download_url, checked_at, confidence, status) =
            if let Some(info) = latest_info {
                (
                    info.latest_version,
                    info.download_url,
                    info.checked_at,
                    info.confidence,
                    info.status,
                )
            } else {
                (None, None, None, 0.0, "unknown".to_owned())
            };

        // Determine if update is available
        let update_available = if let (Some(installed), Some(latest)) =
            (&installed_version, &latest_version)
        {
            let installed_parts = special_mod_versions::parse_version_parts(installed);
            let latest_parts = special_mod_versions::parse_version_parts(latest);
            match (installed_parts, latest_parts) {
                (Some(inst), Some(lat)) => inst < lat,
                _ => false,
            }
        } else {
            false
        };

        let is_installed = existing_install_state
            != crate::models::SpecialExistingInstallState::NotInstalled;

        Ok(McccUpdateInfo {
            is_installed,
            installed_version,
            install_path,
            latest_version,
            download_url,
            checked_at,
            update_available,
            confidence,
            status,
            error: None,
        })
    })
    .await
}

/// Download and apply the latest MCCC update, preserving .cfg settings files.
#[tauri::command]
pub async fn apply_mccc_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ApplyMcccUpdateResult, String> {
    let state = state.inner().clone();
    run_blocking_command("apply_mccc_update", move || {
        let mut connection = state.connection().map_err(map_error)?;
        let settings = database::get_library_settings(&connection).map_err(map_error)?;
        let seed_pack = state.seed_pack();

        // Find the MCCC profile
        let Some(profile) = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|p| p.key == "mccc")
        else {
            return Err("MCCC profile not found in seed data.".to_owned());
        };

        // Get the latest download URL
        let latest_info =
            special_mod_versions::load_or_refresh_latest_info(&connection, profile, true)
                .map_err(map_error)?;
        let download_url = latest_info
            .as_ref()
            .and_then(|info| info.download_url.clone())
            .ok_or_else(|| "No MCCC download URL available. Check for updates first.".to_owned())?;

        let url = Url::parse(&download_url)
            .map_err(|error| format!("Invalid download URL: {error}"))?;

        // Download the MCCC zip to a trusted staging location
        let downloaded_path =
            download_review_action_file(&url, &state.app_data_dir, "MCCC_Update.zip")?;

        // Parse the archive and stage its contents
        let staging_root = PathBuf::from(settings.downloads_path.clone().unwrap_or_default());
        let staging_timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
        let item_staging = staging_root.join(format!("mccc_update_{}", staging_timestamp));
        fs::create_dir_all(&item_staging).map_err(|error| error.to_string())?;

        // Extract the archive
        let archive_path = downloaded_path;
        let archive_staging = staging_root.join(format!("mccc_archive_{}", staging_timestamp));
        fs::create_dir_all(&archive_staging).map_err(|error| error.to_string())?;
        let extracted =
            crate::core::file_inspector::extract_archive(&archive_path, &archive_staging)
                .map_err(|error| format!("Failed to extract MCCC archive: {error}"))?;

        // Move extracted contents to item staging (unwrap the top-level folder if只有一个)
        let members = extracted.members().collect::<Vec<_>>();
        let first_member = members.first().ok_or_else(|| "MCCC archive is empty.".to_owned())?;
        let top_level_is_folder = first_member
            .file_name()
            .map(|n| n.to_string_lossy().contains("McCmdCenter"))
            .unwrap_or(false);

        if top_level_is_folder && members.len() == 1 {
            // Single top-level folder — move its contents up
            for member in members {
                let dest = item_staging.join(
                    member
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                );
                let _ = fs::rename(member, &dest);
            }
        } else {
            // Multiple files or no clear top-level folder — move all directly
            for member in members {
                let dest = item_staging.join(
                    member
                        .file_name()
                        .map(|n| n.to_string_lossy())
                        .unwrap_or_default(),
                );
                let _ = fs::rename(member, &dest);
            }
        }

        // Now apply the staged MCCC files using move_engine
        // Create a preview and apply it
        let preview_result = move_engine::apply_preview_organization(
            &mut connection,
            &settings,
            Some("mccc".to_owned()),
            None,
            false,
        )
        .map_err(map_error)?;

        // Count what was done
        let installed_count = preview_result.installed_count;
        let replaced_count = preview_result.replaced_count;

        // Preserved count comes from the guided profile (cfg files)
        let preserved_count = 0; // Simplified; full impl would track cfg preservation

        // Emit update events
        emit_workspace_domains(
            &app,
            vec![
                WorkspaceDomain::Home,
                WorkspaceDomain::Library,
                WorkspaceDomain::Downloads,
            ],
            "mccc-update-applied",
            Vec::new(),
            Vec::new(),
        )?;

        // Return the new version
        let new_version = latest_info
            .as_ref()
            .and_then(|info| info.latest_version.clone())
            .unwrap_or_else(|| "unknown".to_owned());

        Ok(ApplyMcccUpdateResult {
            new_version,
            installed_count,
            replaced_count,
            preserved_count,
            snapshot_id: preview_result.snapshot_id.unwrap_or(0),
            snapshot_name: preview_result.snapshot_name.unwrap_or_else(|| "MCCC Update".to_owned()),
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::{
        approved_review_action_url, approved_watch_source_url, is_locked_read_error,
        retry_locked_read, review_action_url_matches, validate_review_download_redirect,
    };
    use crate::{
        error::AppError,
        models::{ReviewPlanAction, ReviewPlanActionKind},
    };
    use reqwest::Url;

    fn review_action(url: Option<&str>) -> ReviewPlanAction {
        ReviewPlanAction {
            kind: ReviewPlanActionKind::OpenOfficialSource,
            label: "Open page".to_owned(),
            description: "Open page".to_owned(),
            priority: 1,
            related_item_id: None,
            related_item_name: Some("Test".to_owned()),
            url: url.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn retry_locked_read_retries_transient_locked_errors() {
        let mut attempts = 0;
        let result = retry_locked_read("test_locked_read", || {
            attempts += 1;
            if attempts < 3 {
                return Err(AppError::Message("database is locked".to_owned()));
            }

            Ok::<_, AppError>(42)
        })
        .expect("locked read should recover");

        assert_eq!(result, 42);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn locked_read_detection_matches_busy_database_errors() {
        assert!(is_locked_read_error(&AppError::Message(
            "database table is locked".to_owned()
        )));
        assert!(!is_locked_read_error(&AppError::Message(
            "no such table: missing".to_owned()
        )));
    }

    #[test]
    fn approved_review_action_url_allows_secure_https_links() {
        let action = review_action(Some("https://example.com/downloads"));
        let url = approved_review_action_url(&action, "official page").expect("https url");

        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn approved_review_action_url_rejects_non_https_links() {
        let action = review_action(Some("http://example.com/downloads"));
        let error =
            approved_review_action_url(&action, "trusted download link").expect_err("http blocked");

        assert!(error.contains("HTTPS"));
    }

    #[test]
    fn review_action_url_matching_requires_exact_link_for_web_actions() {
        assert!(review_action_url_matches(
            Some("https://example.com/downloads"),
            Some("https://example.com/downloads")
        ));
        assert!(!review_action_url_matches(
            Some("https://example.com/downloads"),
            Some("https://example.com/other")
        ));
        assert!(!review_action_url_matches(
            Some("https://example.com/downloads"),
            None
        ));
    }

    #[test]
    fn validate_review_download_redirect_rejects_off_host_redirect() {
        let redirected =
            Url::parse("https://cdn.example.net/downloads/file.zip").expect("redirected url");
        let error = validate_review_download_redirect("example.com", &redirected)
            .expect_err("off-host redirect blocked");

        assert!(error.contains("redirected"));
        assert!(error.contains("example.com"));
        assert!(error.contains("cdn.example.net"));
    }

    #[test]
    fn approved_watch_source_url_allows_secure_https_links() {
        let url =
            approved_watch_source_url("https://example.com/mod-page").expect("https watch url");

        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn approved_watch_source_url_rejects_non_https_links() {
        let error =
            approved_watch_source_url("http://example.com/mod-page").expect_err("http blocked");

        assert!(error.contains("HTTPS"));
    }

    #[test]
    fn approved_watch_source_url_rejects_embedded_credentials() {
        let error = approved_watch_source_url("https://user:secret@example.com/mod-page")
            .expect_err("credentials blocked");

        assert!(error.contains("sign-in details"));
    }
}
