use chrono::Utc;
use std::path::{Path, PathBuf};

use dirs::document_dir;
use tauri::{AppHandle, Emitter, State};

use crate::{
    app_state::AppState,
    core::{
        category_audit, creator_audit, downloads_watcher, library_index, move_engine,
        rule_engine, scanner, snapshot_manager,
    },
    database,
    error::AppError,
    models::{
        ApplyCategoryAuditResult, ApplyCreatorAuditResult, ApplyPreviewResult, CategoryAuditQuery,
        CategoryAuditFile, CategoryAuditResponse, CreatorAuditFile, CreatorAuditQuery,
        CreatorAuditResponse, DetectedLibraryPaths, DownloadInboxDetail, DownloadsInboxQuery,
        DownloadsInboxResponse, DownloadsWatcherStatus, DuplicateOverview, DuplicatePair,
        FileDetail, HomeOverview, LibraryFacets, LibraryListResponse, LibraryQuery,
        LibrarySettings, OrganizationPreview, RestoreSnapshotResult, ReviewQueueItem,
        RulePreset, ScanPhase, ScanRuntimeState, ScanStatus, ScanSummary, SnapshotSummary,
    },
};

fn map_error(error: AppError) -> String {
    error.to_string()
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
    rule_engine::build_preview(&connection, &settings, preset_name, limit.unwrap_or(40))
        .map_err(map_error)
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
    move_engine::restore_snapshot(&mut connection, snapshot_id, approved).map_err(map_error)
}

#[tauri::command]
pub fn apply_download_item(
    item_id: i64,
    preset_name: Option<String>,
    approved: bool,
    state: State<'_, AppState>,
) -> Result<ApplyPreviewResult, String> {
    let mut connection = state.connection().map_err(map_error)?;
    let settings = database::get_library_settings(&connection).map_err(map_error)?;
    let file_ids = downloads_watcher::load_active_file_ids(&connection, item_id).map_err(map_error)?;
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
