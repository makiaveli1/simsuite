use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use chrono::{DateTime, Utc};
use notify::{recommended_watcher, RecursiveMode, Watcher};
use rusqlite::{params, Connection, OptionalExtension};
use tauri::AppHandle;
use walkdir::WalkDir;

use crate::{
    app_state::AppState,
    commands::emit_downloads_status,
    core::{
        bundle_detector, duplicate_detector, rule_engine,
        scanner::{self, DiscoveredFile},
    },
    database,
    error::{AppError, AppResult},
    models::{
        DownloadInboxDetail, DownloadInboxFile, DownloadsInboxItem, DownloadsInboxOverview,
        DownloadsInboxQuery, DownloadsInboxResponse, DownloadsWatcherState,
        DownloadsWatcherStatus, LibrarySettings, OrganizationPreview,
    },
};

const WATCHER_DEBOUNCE_MS: u64 = 900;

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

pub fn restart_watcher(app: &AppHandle, state: &AppState) -> AppResult<()> {
    stop_watcher(state)?;

    let connection = state.connection()?;
    let settings = database::get_library_settings(&connection)?;
    let Some(downloads_path) = settings.downloads_path.filter(|value| !value.trim().is_empty()) else {
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
    process_downloads_once(
        app,
        state,
        Some("Manual inbox refresh".to_owned()),
        true,
    )
}

pub fn list_download_items(
    connection: &Connection,
    settings: &LibrarySettings,
    query: DownloadsInboxQuery,
) -> AppResult<DownloadsInboxResponse> {
    let overview = load_overview(connection, settings)?;
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

    if let Some(status) = query
        .status
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        let index = params.len() + 1;
        sql.push_str(&format!(" AND di.status = ?{index}"));
        params.push(status.to_owned());
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
                first_seen_at: row.get(8)?,
                last_seen_at: row.get(9)?,
                updated_at: row.get(10)?,
                error_message: row.get(11)?,
                notes: parse_string_array(row.get::<_, String>(12)?),
                active_file_count: row.get(13)?,
                applied_file_count: row.get(14)?,
                review_file_count: row.get(15)?,
                sample_files: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut items = Vec::new();
    for mut item in rows {
        item.sample_files = load_item_sample_names(connection, item.id)?;
        items.push(item);
    }

    Ok(DownloadsInboxResponse { overview, items })
}

pub fn get_download_item_detail(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Option<DownloadInboxDetail>> {
    let Some(item) = load_item_by_id(connection, item_id)? else {
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

pub fn preview_download_item(
    connection: &Connection,
    settings: &LibrarySettings,
    item_id: i64,
    preset_name: Option<String>,
) -> AppResult<OrganizationPreview> {
    let file_ids = load_active_file_ids(connection, item_id)?;
    if file_ids.is_empty() {
        return Err(AppError::Message(
            "This inbox item has no active files left to preview.".to_owned(),
        ));
    }

    rule_engine::build_preview_for_files(connection, settings, preset_name, &file_ids)
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
    let Some(current_status) = connection
        .query_row(
            "SELECT status FROM download_items WHERE id = ?1",
            params![item_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    else {
        return Ok(());
    };

    if current_status == "ignored" || current_status == "error" {
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

    let _ = process_downloads_once(
        &app,
        &state,
        Some("Initial inbox refresh".to_owned()),
        false,
    );

    loop {
        if stop.try_recv().is_ok() {
            break;
        }

        match event_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(event)) => {
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
                while event_rx.try_recv().is_ok() {}

                let _ = process_downloads_once(&app, &state, Some(current_item), false);
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
    let mut connection = state.connection()?;
    let settings = database::get_library_settings(&connection)?;
    let Some(downloads_path) = settings.downloads_path.clone().filter(|value| !value.trim().is_empty()) else {
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
        },
    )?;

    let base_seed = state.seed_pack();
    let runtime_seed_pack = database::load_runtime_seed_pack(&connection, base_seed.as_ref())?;
    let category_overrides = database::list_category_overrides(&connection)?
        .into_iter()
        .map(|item| (normalize_path_key(&item.match_path), item))
        .collect::<HashMap<_, _>>();
    let observed = collect_observed_sources(&watched_root)?;
    let existing = load_existing_items(&connection)?;
    let mut changed = false;

    for source in &observed {
        let key = normalize_path_key(&source.path.to_string_lossy());
        let existing_item = existing.get(&key);
        let unchanged = existing_item.is_some_and(|item| {
            item.source_size == source.source_size
                && item.source_modified_at == source.source_modified_at
                && ((item.status == "applied" || item.status == "ignored")
                    || item.active_file_count > 0)
        });

        if unchanged {
            update_last_seen(&connection, existing_item.expect("existing item").id)?;
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
        )?;
        changed = true;
    }

    mark_missing_direct_sources(&connection, &existing, &observed)?;

    if changed || manual {
        bundle_detector::rebuild_bundles(&mut connection)?;
        duplicate_detector::rebuild_duplicates(&mut connection)?;
    }

    recompute_item_statuses(&connection)?;
    let status = summarize_status(&connection, Some(watched_root.to_string_lossy().to_string()))?;
    store_status(state, app, status.clone())?;
    Ok(status)
}

fn process_source(
    connection: &mut Connection,
    state: &AppState,
    watched_root: &Path,
    seed_pack: &crate::seed::SeedPack,
    category_overrides: &HashMap<String, database::UserCategoryOverride>,
    source: &ObservedSource,
    existing: Option<&ExistingDownloadItem>,
) -> AppResult<()> {
    let item_id = upsert_download_item(connection, source, existing.map(|item| item.id))?;

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

    let mut notes = Vec::new();
    let mut staged_root = None;
    let discovered = if source.source_kind == "file" {
        vec![build_discovered_file(watched_root, &source.path)?]
    } else {
        let next_root = state
            .app_data_dir
            .join("downloads_inbox")
            .join(item_id.to_string())
            .join(Utc::now().format("%Y%m%d%H%M%S").to_string());
        let extracted = extract_archive(source, &next_root, &mut notes)?;
        staged_root = Some(next_root);
        extracted
    };

    if discovered.is_empty() {
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
                serde_json::to_string(&notes)?,
                staged_root.map(|value| value.to_string_lossy().to_string()),
                Utc::now().to_rfc3339()
            ],
        )?;
        return Ok(());
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
            source_modified_at: metadata
                .modified()
                .ok()
                .map(system_time_to_rfc3339),
        });
    }

    observed.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(observed)
}

fn load_existing_items(connection: &Connection) -> AppResult<HashMap<String, ExistingDownloadItem>> {
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
            return Err(AppError::Message(
                "Unsupported archive format.".to_owned(),
            ));
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
        notes.push(format!("Ignored {ignored_entries} unsupported archive entries."));
    }

    Ok(discovered)
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
        if !is_supported_content_extension(&extension) {
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
        notes.push(format!("Ignored {ignored_entries} unsupported zip entries."));
    }

    Ok(())
}

fn mark_missing_direct_sources(
    connection: &Connection,
    existing: &HashMap<String, ExistingDownloadItem>,
    observed: &[ObservedSource],
) -> AppResult<()> {
    let observed_paths = observed
        .iter()
        .map(|item| normalize_path_key(&item.path.to_string_lossy()))
        .collect::<HashSet<_>>();
    let now = Utc::now().to_rfc3339();

    for item in existing.values() {
        if observed_paths.contains(&normalize_path_key(&item.source_path)) {
            continue;
        }
        if item.source_kind != "file" || item.status == "applied" || item.status == "ignored" {
            continue;
        }

        connection.execute(
            "UPDATE download_items
             SET status = 'error',
                 error_message = ?2,
                 updated_at = ?3
             WHERE id = ?1",
            params![item.id, "Source file is missing from Downloads.", now],
        )?;
    }

    Ok(())
}

fn recompute_item_statuses(connection: &Connection) -> AppResult<()> {
    let mut statement = connection.prepare("SELECT id, status FROM download_items")?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?
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
    let (active_file_count, applied_file_count, review_file_count): (i64, i64, i64) =
        connection.query_row(
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
                )",
            params![item_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

    if active_file_count == 0 && applied_file_count > 0 {
        return Ok("applied".to_owned());
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
    let (total_items, ready_items, needs_review_items, applied_items, error_items, active_files): (
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = connection.query_row(
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
            )
         FROM download_items",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(4)?.unwrap_or_default(),
                row.get::<_, i64>(5)?,
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
    })
}

fn load_item_by_id(connection: &Connection, item_id: i64) -> AppResult<Option<DownloadsInboxItem>> {
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
                    first_seen_at: row.get(8)?,
                    last_seen_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    error_message: row.get(11)?,
                    notes: parse_string_array(row.get::<_, String>(12)?),
                    active_file_count: row.get(13)?,
                    applied_file_count: row.get(14)?,
                    review_file_count: row.get(15)?,
                    sample_files: Vec::new(),
                })
            },
        )
        .optional()
        .map_err(Into::into)
        .and_then(|item| {
            if let Some(mut item) = item {
                item.sample_files = load_item_sample_names(connection, item.id)?;
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

fn normalize_extension(path: &Path) -> String {
    path.extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
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

fn parse_string_array(value: String) -> Vec<String> {
    serde_json::from_str(&value).unwrap_or_default()
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
    use super::{derive_item_status, summarize_status};
    use crate::database::initialize;
    use rusqlite::{params, Connection};

    fn setup_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
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
}
