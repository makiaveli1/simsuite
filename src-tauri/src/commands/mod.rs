use chrono::Utc;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use dirs::document_dir;
use reqwest::blocking::Client;
use tauri::{AppHandle, Emitter, State};

use crate::{
    app_state::AppState,
    core::{
        category_audit, creator_audit, downloads_watcher, install_profile_engine, library_index,
        move_engine, rule_engine, scanner, snapshot_manager,
    },
    database,
    error::AppError,
    models::{
        ApplyReviewPlanActionResult,
        ApplyCategoryAuditResult, ApplyCreatorAuditResult, ApplyGuidedDownloadResult,
        ApplyPreviewResult, ApplySpecialReviewFixResult, CategoryAuditFile, CategoryAuditQuery,
        CategoryAuditResponse, CreatorAuditFile, CreatorAuditQuery, CreatorAuditResponse,
        DetectedLibraryPaths, DownloadInboxDetail, DownloadsInboxQuery, DownloadsInboxResponse,
        DownloadsWatcherStatus, DuplicateOverview, DuplicatePair, FileDetail, GuidedInstallPlan,
        HomeOverview, LibraryFacets, LibraryListResponse, LibraryQuery, LibrarySettings,
        OrganizationPreview, RestoreSnapshotResult, ReviewPlanAction, ReviewPlanActionKind,
        ReviewQueueItem, RulePreset, ScanPhase, ScanRuntimeState, ScanStatus, ScanSummary,
        SnapshotSummary, SpecialReviewPlan,
    },
};

fn map_error(error: AppError) -> String {
    error.to_string()
}

fn review_action_kind_matches(kind: &ReviewPlanActionKind, value: &str) -> bool {
    matches!(
        (kind, value),
        (ReviewPlanActionKind::RepairSpecial, "repair_special")
            | (ReviewPlanActionKind::InstallDependency, "install_dependency")
            | (ReviewPlanActionKind::OpenDependency, "open_dependency")
            | (ReviewPlanActionKind::DownloadMissingFiles, "download_missing_files")
            | (ReviewPlanActionKind::OpenOfficialSource, "open_official_source")
            | (ReviewPlanActionKind::SeparateSupportedFiles, "separate_supported_files")
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
                && match (action.url.as_deref(), url) {
                    (Some(left), Some(right)) => left == right,
                    (None, None) => true,
                    (_, None) => true,
                    (None, Some(_)) => false,
                }
        })
        .cloned()
        .ok_or_else(|| "This review action is no longer available for the selected inbox item.".to_owned())
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
        .and_then(|segments| segments.last())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned);

    sanitize_download_filename(
        &header_name.or(url_name).unwrap_or_else(|| fallback.to_owned()),
        fallback,
    )
}

fn download_review_action_file(
    url: &str,
    app_data_dir: &Path,
    fallback_name: &str,
) -> Result<PathBuf, String> {
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::limited(8))
        .build()
        .map_err(|error| error.to_string())?;
    let mut response = client
        .get(url)
        .send()
        .and_then(|response| response.error_for_status())
        .map_err(|error| error.to_string())?;
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

fn copy_split_files(
    files: &[crate::models::DownloadInboxFile],
    staging_root: &Path,
    file_ids: &[i64],
) -> Result<(), String> {
    let wanted = file_ids.iter().copied().collect::<std::collections::HashSet<_>>();
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
pub fn save_library_paths(
    app: AppHandle,
    settings: LibrarySettings,
    state: State<'_, AppState>,
) -> Result<LibrarySettings, String> {
    let mut connection = state.connection().map_err(map_error)?;
    database::save_library_paths(&mut connection, &settings).map_err(map_error)?;
    downloads_watcher::restart_watcher(&app, state.inner()).map_err(map_error)?;
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
pub fn get_home_overview(state: State<'_, AppState>) -> Result<HomeOverview, String> {
    let connection = state.connection().map_err(map_error)?;
    library_index::get_home_overview(&connection).map_err(map_error)
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
pub fn refresh_downloads_inbox(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadsWatcherStatus, String> {
    downloads_watcher::refresh_inbox(&app, state.inner()).map_err(map_error)
}

#[tauri::command]
pub fn get_downloads_inbox(
    query: Option<DownloadsInboxQuery>,
    state: State<'_, AppState>,
) -> Result<DownloadsInboxResponse, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    downloads_watcher::list_download_items(&connection, &settings, query.unwrap_or_default())
        .map_err(map_error)
}

#[tauri::command]
pub fn get_download_item_detail(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<DownloadInboxDetail>, String> {
    let connection = state.connection().map_err(map_error)?;
    downloads_watcher::get_download_item_detail(&connection, item_id).map_err(map_error)
}

#[tauri::command]
pub fn preview_download_item(
    item_id: i64,
    preset_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<OrganizationPreview, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    downloads_watcher::preview_download_item(&connection, &settings, item_id, preset_name)
        .map_err(map_error)
}

#[tauri::command]
pub fn get_library_facets(state: State<'_, AppState>) -> Result<LibraryFacets, String> {
    let connection = state.connection().map_err(map_error)?;
    let seed_pack = state.seed_pack();
    library_index::get_library_facets(&connection, &seed_pack.taxonomy).map_err(map_error)
}

#[tauri::command]
pub fn get_duplicate_overview(state: State<'_, AppState>) -> Result<DuplicateOverview, String> {
    let connection = state.connection().map_err(map_error)?;
    crate::core::duplicate_detector::get_duplicate_overview(&connection).map_err(map_error)
}

#[tauri::command]
pub fn list_duplicate_pairs(
    duplicate_type: Option<String>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<DuplicatePair>, String> {
    let connection = state.connection().map_err(map_error)?;
    crate::core::duplicate_detector::list_duplicate_pairs(
        &connection,
        duplicate_type,
        limit.unwrap_or(160),
    )
    .map_err(map_error)
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
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    if limit.is_some_and(|value| value <= 0) {
        rule_engine::build_preview_full(&connection, &settings, preset_name).map_err(map_error)
    } else {
        rule_engine::build_preview(&connection, &settings, preset_name, limit.unwrap_or(40))
            .map_err(map_error)
    }
}

#[tauri::command]
pub fn get_review_queue(
    preset_name: Option<String>,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewQueueItem>, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    rule_engine::load_review_queue(&connection, &settings, preset_name, limit.unwrap_or(80))
        .map_err(map_error)
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
    preset_name: Option<String>,
    limit: Option<i64>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyPreviewResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    move_engine::apply_preview_moves(
        &mut connection,
        &settings,
        preset_name,
        limit.unwrap_or(80),
        approved,
    )
    .map_err(map_error)
}

#[tauri::command]
pub fn restore_snapshot(
    snapshot_id: i64,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<RestoreSnapshotResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let seed_pack = state.seed_pack();
    move_engine::restore_snapshot(&mut connection, &seed_pack, snapshot_id, approved)
        .map_err(map_error)
}

#[tauri::command]
pub fn apply_download_item(
    item_id: i64,
    preset_name: Option<String>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyPreviewResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let Some(item) = downloads_watcher::get_download_item_detail(&connection, item_id)
        .map_err(map_error)?
        .map(|detail| detail.item)
    else {
        return Err("Inbox item was not found.".to_owned());
    };
    if item.intake_mode != crate::models::DownloadIntakeMode::Standard {
        return Err(
            "This inbox item uses a special setup flow. Open its guided preview instead."
                .to_owned(),
        );
    }

    let settings = database::get_library_settings(&connection).map_err(map_error)?;
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
    Ok(result)
}

#[tauri::command]
pub fn get_download_item_guided_plan(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<GuidedInstallPlan>, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    downloads_watcher::get_download_item_guided_plan(&connection, &settings, &seed_pack, item_id)
        .map_err(map_error)
}

#[tauri::command]
pub fn get_download_item_review_plan(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<SpecialReviewPlan>, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    downloads_watcher::get_download_item_review_plan(&connection, &settings, &seed_pack, item_id)
        .map_err(map_error)
}

#[tauri::command]
pub fn apply_guided_download_item(
    item_id: i64,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyGuidedDownloadResult, String> {
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

    let result = move_engine::apply_guided_download_plan(
        &mut connection,
        &settings,
        &seed_pack,
        &state.app_data_dir,
        &plan,
        approved,
    )
    .map_err(map_error)?;
    downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
    Ok(result)
}

#[tauri::command]
pub fn apply_special_review_fix(
    item_id: i64,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplySpecialReviewFixResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();

    let result = move_engine::apply_special_review_fix(
        &mut connection,
        &settings,
        &seed_pack,
        &state.app_data_dir,
        item_id,
        approved,
    )
    .map_err(map_error)?;
    downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
    Ok(result)
}

#[tauri::command]
pub fn apply_review_plan_action(
    item_id: i64,
    action_kind: String,
    related_item_id: Option<i64>,
    url: Option<String>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyReviewPlanActionResult, String> {
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
    let action = find_review_action(&review_plan, &action_kind, related_item_id, url.as_deref())?;

    match action.kind {
        ReviewPlanActionKind::RepairSpecial => {
            let result = move_engine::apply_special_review_fix(
                &mut connection,
                &settings,
                &seed_pack,
                &state.app_data_dir,
                item_id,
                approved,
            )
            .map_err(map_error)?;
            downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
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
                    review_plan.profile_name.unwrap_or_else(|| "special mod".to_owned())
                ),
            })
        }
        ReviewPlanActionKind::InstallDependency => {
            let dependency_item_id = action
                .related_item_id
                .ok_or_else(|| "This dependency action is missing its inbox item.".to_owned())?;
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
            let result = move_engine::apply_guided_download_plan(
                &mut connection,
                &settings,
                &seed_pack,
                &state.app_data_dir,
                &plan,
                approved,
            )
            .map_err(map_error)?;
            downloads_watcher::refresh_download_item_status(&connection, dependency_item_id)
                .map_err(map_error)?;
            downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
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
        ReviewPlanActionKind::OpenOfficialSource => {
            let opened_url = action.url.clone().ok_or_else(|| {
                "This official page is missing its website address, so SimSuite could not open it."
                    .to_owned()
            })?;
            webbrowser::open(&opened_url).map_err(|error| {
                format!("SimSuite could not open the official page in your browser: {error}")
            })?;

            Ok(ApplyReviewPlanActionResult {
                action_kind: action.kind,
                focus_item_id: item_id,
                created_item_id: None,
                opened_url: Some(opened_url),
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
                return Err("Download was blocked because approval was not confirmed.".to_owned());
            }
            let fallback_name = format!(
                "{}.zip",
                action
                    .related_item_name
                    .clone()
                    .unwrap_or_else(|| "special-download".to_owned())
                    .replace(' ', "_")
            );
            let url = action
                .url
                .clone()
                .ok_or_else(|| "This action is missing its trusted download link.".to_owned())?;
            let downloaded_file =
                download_review_action_file(&url, &state.app_data_dir, &fallback_name)?;
            let imported_item_id = downloads_watcher::import_download_source(
                &mut connection,
                state.inner(),
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
            let profile_key = review_plan
                .profile_key
                .clone()
                .ok_or_else(|| "This inbox item does not have a matched special-mod profile.".to_owned())?;
            let Some((supported_ids, leftover_ids)) = install_profile_engine::collect_supported_subset_file_ids(
                &connection,
                &seed_pack,
                item_id,
                &profile_key,
            )
            .map_err(map_error)?
            else {
                return Err("SimSuite could not find a clean supported subset to split out.".to_owned());
            };
            let source = downloads_watcher::get_download_item_source(&connection, item_id)
                .map_err(map_error)?
                .ok_or_else(|| "This inbox item could not be found.".to_owned())?;
            let detail = downloads_watcher::get_download_item_detail(&connection, item_id)
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
                state.inner(),
                &source,
                &supported_root,
                supported_name,
                Some(item_id),
                vec!["Split out the supported special-mod files from a mixed batch.".to_owned()],
            )
            .map_err(map_error)?;
            let leftover_item_id = downloads_watcher::import_staged_batch(
                &mut connection,
                state.inner(),
                &source,
                &leftover_root,
                leftover_name,
                None,
                vec!["Leftover files from a mixed special-mod batch.".to_owned()],
            )
            .map_err(map_error)?;
            downloads_watcher::refresh_download_item_status(&connection, item_id).map_err(map_error)?;
            downloads_watcher::refresh_download_item_status(&connection, leftover_item_id)
                .map_err(map_error)?;

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
                message: "Split the supported special-mod files into their own clean inbox batch."
                    .to_owned(),
            })
        }
    }
}

#[tauri::command]
pub fn ignore_download_item(
    item_id: i64,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let mut connection = state.connection().map_err(map_error)?;
    downloads_watcher::ignore_download_item(&mut connection, item_id).map_err(map_error)?;
    Ok(true)
}

#[tauri::command]
pub fn list_library_files(
    query: LibraryQuery,
    state: State<'_, AppState>,
) -> Result<LibraryListResponse, String> {
    let connection = state.connection().map_err(map_error)?;
    library_index::list_library_files(&connection, query).map_err(map_error)
}

#[tauri::command]
pub fn get_creator_audit(
    query: Option<CreatorAuditQuery>,
    state: State<'_, AppState>,
) -> Result<CreatorAuditResponse, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    creator_audit::load_creator_audit(&connection, &settings, &seed_pack, query.unwrap_or_default())
        .map_err(map_error)
}

#[tauri::command]
pub fn get_category_audit(
    query: Option<CategoryAuditQuery>,
    state: State<'_, AppState>,
) -> Result<CategoryAuditResponse, String> {
    let connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let seed_pack = state.seed_pack();
    category_audit::load_category_audit(&connection, &settings, &seed_pack, query.unwrap_or_default())
        .map_err(map_error)
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
pub fn get_file_detail(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
    let connection = state.connection().map_err(map_error)?;
    library_index::get_file_detail(&connection, file_id).map_err(map_error)
}

#[tauri::command]
pub fn save_creator_learning(
    file_id: i64,
    creator_name: String,
    alias_name: Option<String>,
    lock_preference: Option<bool>,
    preferred_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
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

    library_index::get_file_detail(&connection, file_id).map_err(map_error)
}

#[tauri::command]
pub fn apply_creator_audit(
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

    Ok(ApplyCreatorAuditResult {
        creator_name,
        updated_count,
        cleared_review_count,
        locked_route: lock_preference.unwrap_or(false),
    })
}

#[tauri::command]
pub fn apply_category_audit(
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

    Ok(ApplyCategoryAuditResult {
        kind: normalized_kind,
        subtype,
        updated_count,
        cleared_review_count,
    })
}

#[tauri::command]
pub fn save_category_override(
    file_id: i64,
    kind: String,
    subtype: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<FileDetail>, String> {
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
    database::save_category_override(&mut connection, file_id, &normalized_kind, subtype.as_deref())
        .map_err(map_error)?;

    library_index::get_file_detail(&connection, file_id).map_err(map_error)
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
