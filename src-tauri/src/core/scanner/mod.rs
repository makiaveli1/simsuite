use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::{
    app_state::AppState,
    commands::emit_scan_progress,
    core::{
        bundle_detector, duplicate_detector,
        file_inspector::{inspect_file, InspectionOutcome},
        filename_parser::{detect_creator_hint, parse_filename, FilenameClassification},
    },
    database,
    error::{AppError, AppResult},
    models::{ScanMode, ScanPhase, ScanProgress, ScanSummary},
    seed::normalize_key,
};

#[derive(Debug, Clone, Copy)]
enum RootType {
    Mods,
    Tray,
}

#[derive(Debug, Clone)]
struct ScanRoot {
    root_type: RootType,
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoveredFile {
    pub(crate) root_path: PathBuf,
    pub(crate) source_location: String,
    pub(crate) path: PathBuf,
    pub(crate) filename: String,
    pub(crate) extension: String,
    pub(crate) size: i64,
    pub(crate) created_at: Option<String>,
    pub(crate) modified_at: Option<String>,
    pub(crate) relative_depth: i64,
}

#[derive(Debug, Clone)]
struct CachedFileRecord {
    path: String,
    filename: String,
    extension: String,
    hash: Option<String>,
    size: i64,
    created_at: Option<String>,
    modified_at: Option<String>,
    creator_id: Option<i64>,
    kind: String,
    subtype: Option<String>,
    confidence: f64,
    source_location: String,
    relative_depth: i64,
    safety_notes_json: String,
    parser_warnings_json: String,
    insights_json: String,
}

impl CachedFileRecord {
    fn matches(&self, file: &DiscoveredFile) -> bool {
        self.filename == file.filename
            && self.extension == file.extension
            && self.size == file.size
            && self.created_at == file.created_at
            && self.modified_at == file.modified_at
            && self.source_location == file.source_location
            && self.relative_depth == file.relative_depth
    }

    fn normalized_hash(&self) -> Option<String> {
        self.hash
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }
}

enum HashOutcome {
    Success { index: usize, hash: String },
    Failure { index: usize, error: String },
}

const MAX_HASH_WORKERS: usize = 4;
const MIN_PARALLEL_HASH_ITEMS: usize = 8;
const SCAN_CACHE_FINGERPRINT_KEY: &str = "scan_cache_fingerprint";
// Bump when stored inspection output meaning changes so unchanged files are re-inspected once.
// v18: THUM (0x3C1AF1F2) thumbnail extraction added — all cached files must be re-inspected
const SCAN_CACHE_VERSION: &str = "scanner-v18";

pub fn library_scan_needs_refresh(
    connection: &Connection,
    seed_pack: &crate::seed::SeedPack,
) -> AppResult<bool> {
    let expected_fingerprint = current_cache_fingerprint(connection, seed_pack)?;
    let saved_fingerprint = database::get_app_setting(connection, SCAN_CACHE_FINGERPRINT_KEY)?;

    if saved_fingerprint.as_deref() == Some(expected_fingerprint.as_str()) {
        return Ok(false);
    }

    let has_completed_scan: bool = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM scan_sessions
            WHERE completed_at IS NOT NULL
        )",
        [],
        |row| row.get::<_, bool>(0),
    )?;
    let has_library_rows: bool = connection.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM files
            WHERE source_location <> 'downloads'
        )",
        [],
        |row| row.get::<_, bool>(0),
    )?;

    Ok(has_completed_scan || has_library_rows)
}

pub fn scan_library(state: &AppState, app: &AppHandle) -> AppResult<ScanSummary> {
    scan_library_with_progress(state, |progress| {
        emit_scan_progress(app, &progress).map_err(AppError::Message)
    })
}

pub fn scan_library_with_progress<F>(state: &AppState, mut emit: F) -> AppResult<ScanSummary>
where
    F: FnMut(ScanProgress) -> AppResult<()>,
{
    let base_seed_pack = state.seed_pack();
    let setup_connection = state.connection()?;
    let settings = database::get_library_settings(&setup_connection)?;
    let creator_learning_version = database::get_creator_learning_version(&setup_connection)?;
    let category_override_version = database::get_category_override_version(&setup_connection)?;
    let runtime_seed_pack =
        database::load_runtime_seed_pack(&setup_connection, base_seed_pack.as_ref())?;
    let category_overrides = database::list_category_overrides(&setup_connection)?
        .into_iter()
        .map(|item| (normalize_override_key(&item.match_path), item))
        .collect::<HashMap<_, _>>();

    let scan_roots = collect_roots(&settings)?;
    if scan_roots.is_empty() {
        return Err(AppError::Message(
            "Select a Mods or Tray folder before starting a scan.".to_owned(),
        ));
    }

    let cached_files = load_cached_files(&setup_connection, &scan_roots)?;
    let cache_fingerprint = cache_fingerprint(
        &runtime_seed_pack,
        creator_learning_version.as_deref(),
        category_override_version.as_deref(),
    );
    let cache_enabled = database::get_app_setting(&setup_connection, SCAN_CACHE_FINGERPRINT_KEY)?
        .as_deref()
        == Some(cache_fingerprint.as_str())
        && !cached_files.is_empty();
    let scan_mode = if cache_enabled {
        ScanMode::Incremental
    } else {
        ScanMode::Full
    };

    info!(
        "Starting {} library scan",
        match scan_mode {
            ScanMode::Full => "full",
            ScanMode::Incremental => "incremental",
        }
    );
    let session_id = {
        setup_connection.execute(
            "INSERT INTO scan_sessions (scan_type, started_at) VALUES (?1, ?2)",
            params![scan_mode_label(&scan_mode), Utc::now().to_rfc3339()],
        )?;
        setup_connection.last_insert_rowid()
    };

    let mut errors = Vec::new();
    emit(ScanProgress {
        total_files: 0,
        processed_files: 0,
        current_item: "Walking configured library folders".to_owned(),
        phase: ScanPhase::Collecting,
    })?;

    let discovered =
        collect_supported_files_with_progress(&scan_roots, &mut errors, |count, path| {
            emit(ScanProgress {
                total_files: count,
                processed_files: 0,
                current_item: path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string()),
                phase: ScanPhase::Collecting,
            })
        })?;
    emit(ScanProgress {
        total_files: discovered.len(),
        processed_files: 0,
        current_item: match scan_mode {
            ScanMode::Full => "Full scan indexed".to_owned(),
            ScanMode::Incremental => "Incremental scan indexed".to_owned(),
        },
        phase: ScanPhase::Collecting,
    })?;

    let current_paths = discovered
        .iter()
        .map(|file| file.path.to_string_lossy().to_string())
        .collect::<HashSet<_>>();
    let removed_files = cached_files
        .keys()
        .filter(|path| !current_paths.contains(*path))
        .count();
    let hash_candidates = collect_hash_candidate_indices(&discovered, &cached_files, cache_enabled);
    let hashed_files = hash_candidates.len();
    let hashes = hash_selected_candidates_with_progress(
        &discovered,
        &hash_candidates,
        &mut errors,
        |progress| emit(progress),
    )?;

    let mut connection = state.connection()?;
    let mut review_items_created = 0_usize;
    let mut reused_files = 0_usize;
    let mut new_files = 0_usize;
    let mut updated_files = 0_usize;
    {
        let transaction = connection.transaction()?;
        clear_previous_scan_data(&transaction, &scan_roots)?;
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

        for (index, file) in discovered.iter().enumerate() {
            emit(ScanProgress {
                total_files: discovered.len(),
                processed_files: index + 1,
                current_item: file.filename.clone(),
                phase: ScanPhase::Classifying,
            })?;

            let path_key = file.path.to_string_lossy().to_string();
            let cached = cached_files
                .get(&path_key)
                .filter(|entry| cache_enabled && entry.matches(file));

            if let Some(cached) = cached {
                reused_files += 1;
                review_items_created += insert_cached_file(
                    &transaction,
                    &mut file_insert,
                    &mut review_insert,
                    session_id,
                    file,
                    cached,
                    hashes.get(&index).cloned(),
                )?;
                continue;
            }

            if cached_files.contains_key(&path_key) {
                updated_files += 1;
            } else {
                new_files += 1;
            }

            review_items_created += insert_parsed_file(
                &transaction,
                &mut creator_cache,
                &runtime_seed_pack,
                &category_overrides,
                &mut file_insert,
                &mut review_insert,
                Some(session_id),
                file,
                hashes.get(&index).cloned(),
            )?;
        }

        drop(review_insert);
        drop(file_insert);
        transaction.commit()?;
    }

    emit(ScanProgress {
        total_files: discovered.len(),
        processed_files: discovered.len(),
        current_item: "Rebuilding tray bundles".to_owned(),
        phase: ScanPhase::Bundling,
    })?;
    let bundles_detected = bundle_detector::rebuild_bundles(&mut connection)?;

    emit(ScanProgress {
        total_files: discovered.len(),
        processed_files: discovered.len(),
        current_item: "Rebuilding duplicate map".to_owned(),
        phase: ScanPhase::Duplicates,
    })?;
    let duplicates_detected = duplicate_detector::rebuild_duplicates(&mut connection)?;

    database::save_app_setting(
        &mut connection,
        SCAN_CACHE_FINGERPRINT_KEY,
        Some(&cache_fingerprint),
        "user",
    )?;

    connection.execute(
        "UPDATE scan_sessions
         SET completed_at = ?1, files_scanned = ?2, errors = ?3
         WHERE id = ?4",
        params![
            Utc::now().to_rfc3339(),
            discovered.len() as i64,
            errors.join("\n"),
            session_id
        ],
    )?;

    emit(ScanProgress {
        total_files: discovered.len(),
        processed_files: discovered.len(),
        current_item: "Scan complete".to_owned(),
        phase: ScanPhase::Done,
    })?;

    info!(
        "Scan finished: {} files, reused {}, hashed {}, bundles {}, duplicates {}",
        discovered.len(),
        reused_files,
        hashed_files,
        bundles_detected,
        duplicates_detected
    );

    Ok(ScanSummary {
        session_id,
        scan_mode,
        files_scanned: discovered.len(),
        reused_files,
        new_files,
        updated_files,
        removed_files,
        hashed_files,
        review_items_created,
        bundles_detected,
        duplicates_detected,
        errors,
    })
}

fn collect_roots(settings: &crate::models::LibrarySettings) -> AppResult<Vec<ScanRoot>> {
    let mut roots = Vec::new();

    if let Some(mods_path) = settings.mods_path.as_deref() {
        let path = PathBuf::from(mods_path);
        if path.exists() {
            roots.push(ScanRoot {
                root_type: RootType::Mods,
                path,
            });
        } else {
            return Err(AppError::Message(format!(
                "Mods path does not exist: {mods_path}"
            )));
        }
    }

    if let Some(tray_path) = settings.tray_path.as_deref() {
        let path = PathBuf::from(tray_path);
        if path.exists() {
            roots.push(ScanRoot {
                root_type: RootType::Tray,
                path,
            });
        } else {
            return Err(AppError::Message(format!(
                "Tray path does not exist: {tray_path}"
            )));
        }
    }

    Ok(roots)
}

fn collect_supported_files_with_progress<F>(
    scan_roots: &[ScanRoot],
    errors: &mut Vec<String>,
    mut on_progress: F,
) -> AppResult<Vec<DiscoveredFile>>
where
    F: FnMut(usize, &Path) -> AppResult<()>,
{
    let mut discovered = Vec::new();

    for root in scan_roots {
        for entry in WalkDir::new(&root.path).into_iter() {
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    let extension = normalize_extension(entry.path());
                    if extension.is_empty() || !is_supported_extension(root.root_type, &extension) {
                        continue;
                    }

                    let metadata = match entry.metadata() {
                        Ok(metadata) => Some(metadata),
                        Err(error) => {
                            warn!(
                                "Failed to read metadata for {}: {error}",
                                entry.path().display()
                            );
                            errors.push(format!(
                                "Failed to read metadata for {}: {error}",
                                entry.path().display()
                            ));
                            None
                        }
                    };
                    let path = entry.into_path();

                    let relative_depth = path
                        .strip_prefix(&root.path)
                        .ok()
                        .and_then(|relative| relative.parent().map(component_count))
                        .unwrap_or(0);
                    let progress_path = path.clone();

                    discovered.push(DiscoveredFile {
                        source_location: match root.root_type {
                            RootType::Mods => "mods".to_owned(),
                            RootType::Tray => "tray".to_owned(),
                        },
                        root_path: root.path.clone(),
                        filename: path
                            .file_name()
                            .map(|value| value.to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_owned()),
                        path,
                        extension,
                        size: metadata
                            .as_ref()
                            .map(|value| value.len() as i64)
                            .unwrap_or_default(),
                        created_at: metadata
                            .as_ref()
                            .and_then(|value| value.created().ok())
                            .map(system_time_to_rfc3339),
                        modified_at: metadata
                            .as_ref()
                            .and_then(|value| value.modified().ok())
                            .map(system_time_to_rfc3339),
                        relative_depth: relative_depth as i64,
                    });

                    if discovered.len() == 1 || discovered.len() % 150 == 0 {
                        on_progress(discovered.len(), &progress_path)?;
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    warn!("Failed while scanning: {error}");
                    errors.push(error.to_string());
                }
            }
        }
    }

    discovered.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(discovered)
}

fn load_cached_files(
    connection: &rusqlite::Connection,
    roots: &[ScanRoot],
) -> AppResult<HashMap<String, CachedFileRecord>> {
    let has_mods = roots
        .iter()
        .any(|root| matches!(root.root_type, RootType::Mods));
    let has_tray = roots
        .iter()
        .any(|root| matches!(root.root_type, RootType::Tray));

    let sql = match (has_mods, has_tray) {
        (true, true) => {
            "SELECT
                path,
                filename,
                extension,
                hash,
                size,
                created_at,
                modified_at,
                creator_id,
                kind,
                subtype,
                confidence,
                source_location,
                relative_depth,
                safety_notes,
                parser_warnings,
                insights
             FROM files
             WHERE source_location IN ('mods', 'tray')"
        }
        (true, false) => {
            "SELECT
                path,
                filename,
                extension,
                hash,
                size,
                created_at,
                modified_at,
                creator_id,
                kind,
                subtype,
                confidence,
                source_location,
                relative_depth,
                safety_notes,
                parser_warnings,
                insights
             FROM files
             WHERE source_location = 'mods'"
        }
        (false, true) => {
            "SELECT
                path,
                filename,
                extension,
                hash,
                size,
                created_at,
                modified_at,
                creator_id,
                kind,
                subtype,
                confidence,
                source_location,
                relative_depth,
                safety_notes,
                parser_warnings,
                insights
             FROM files
             WHERE source_location = 'tray'"
        }
        (false, false) => return Ok(HashMap::new()),
    };

    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map([], |row| {
            Ok(CachedFileRecord {
                path: row.get(0)?,
                filename: row.get(1)?,
                extension: row.get(2)?,
                hash: row.get(3)?,
                size: row.get(4)?,
                created_at: row.get(5)?,
                modified_at: row.get(6)?,
                creator_id: row.get(7)?,
                kind: row.get(8)?,
                subtype: row.get(9)?,
                confidence: row.get(10)?,
                source_location: row.get(11)?,
                relative_depth: row.get(12)?,
                safety_notes_json: row.get(13)?,
                parser_warnings_json: row.get(14)?,
                insights_json: row.get(15)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows
        .into_iter()
        .map(|row| (row.path.clone(), row))
        .collect::<HashMap<_, _>>())
}

fn clear_previous_scan_data(transaction: &Transaction<'_>, roots: &[ScanRoot]) -> AppResult<()> {
    transaction.execute("DELETE FROM duplicates", [])?;
    transaction.execute("DELETE FROM review_queue", [])?;
    transaction.execute("DELETE FROM bundles", [])?;
    transaction.execute("UPDATE files SET bundle_id = NULL", [])?;

    if roots
        .iter()
        .any(|root| matches!(root.root_type, RootType::Mods))
    {
        transaction.execute("DELETE FROM files WHERE source_location = 'mods'", [])?;
    }

    if roots
        .iter()
        .any(|root| matches!(root.root_type, RootType::Tray))
    {
        transaction.execute("DELETE FROM files WHERE source_location = 'tray'", [])?;
    }

    Ok(())
}

fn hash_selected_candidates_with_progress<F>(
    discovered: &[DiscoveredFile],
    candidate_indices: &[usize],
    errors: &mut Vec<String>,
    mut emit: F,
) -> AppResult<HashMap<usize, String>>
where
    F: FnMut(ScanProgress) -> AppResult<()>,
{
    if candidate_indices.is_empty() {
        return Ok(HashMap::new());
    }

    emit(ScanProgress {
        total_files: candidate_indices.len(),
        processed_files: 0,
        current_item: format!("Hashing {} duplicate candidates", candidate_indices.len()),
        phase: ScanPhase::Hashing,
    })?;

    if candidate_indices.len() < MIN_PARALLEL_HASH_ITEMS {
        return hash_candidates_sequential(discovered, candidate_indices, errors, emit);
    }

    let total_candidates = candidate_indices.len();
    let worker_count = hash_worker_count(total_candidates);
    let queue = Arc::new(Mutex::new(VecDeque::from(candidate_indices.to_vec())));
    let (sender, receiver) = mpsc::channel::<HashOutcome>();

    thread::scope(|scope| -> AppResult<HashMap<usize, String>> {
        for _ in 0..worker_count {
            let queue = Arc::clone(&queue);
            let sender = sender.clone();

            scope.spawn(move || loop {
                let next_index = {
                    let mut queue = match queue.lock() {
                        Ok(queue) => queue,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    queue.pop_front()
                };

                let Some(index) = next_index else {
                    break;
                };

                let outcome = match hash_file(&discovered[index].path) {
                    Ok(hash) => HashOutcome::Success { index, hash },
                    Err(error) => HashOutcome::Failure {
                        index,
                        error: error.to_string(),
                    },
                };

                if sender.send(outcome).is_err() {
                    break;
                }
            });
        }
        drop(sender);

        let mut hashes = HashMap::with_capacity(total_candidates);
        for completed in 0..total_candidates {
            let outcome = receiver.recv().map_err(|_| {
                AppError::Message("Hash worker pool stopped before completing the scan".to_owned())
            })?;
            let (index, hash) = match outcome {
                HashOutcome::Success { index, hash } => (index, Some(hash)),
                HashOutcome::Failure { index, error } => {
                    errors.push(format!(
                        "Failed to hash {}: {error}",
                        discovered[index].path.display()
                    ));
                    (index, None)
                }
            };

            if let Some(hash) = hash {
                hashes.insert(index, hash);
            }

            emit(ScanProgress {
                total_files: total_candidates,
                processed_files: completed + 1,
                current_item: discovered[index].filename.clone(),
                phase: ScanPhase::Hashing,
            })?;
        }

        Ok(hashes)
    })
}

fn hash_candidates_sequential<F>(
    discovered: &[DiscoveredFile],
    candidate_indices: &[usize],
    errors: &mut Vec<String>,
    mut emit: F,
) -> AppResult<HashMap<usize, String>>
where
    F: FnMut(ScanProgress) -> AppResult<()>,
{
    let mut hashes = HashMap::with_capacity(candidate_indices.len());

    for (completed, index) in candidate_indices.iter().copied().enumerate() {
        match hash_file(&discovered[index].path) {
            Ok(hash) => {
                hashes.insert(index, hash);
            }
            Err(error) => errors.push(format!(
                "Failed to hash {}: {error}",
                discovered[index].path.display()
            )),
        }

        emit(ScanProgress {
            total_files: candidate_indices.len(),
            processed_files: completed + 1,
            current_item: discovered[index].filename.clone(),
            phase: ScanPhase::Hashing,
        })?;
    }

    Ok(hashes)
}

fn collect_hash_candidate_indices(
    discovered: &[DiscoveredFile],
    cached_files: &HashMap<String, CachedFileRecord>,
    cache_enabled: bool,
) -> Vec<usize> {
    let mut size_counts = HashMap::new();
    for file in discovered {
        *size_counts.entry(file.size).or_insert(0_usize) += 1;
    }

    discovered
        .iter()
        .enumerate()
        .filter_map(|(index, file)| {
            let is_duplicate_candidate = size_counts
                .get(&file.size)
                .filter(|count| **count > 1)
                .is_some();
            if !is_duplicate_candidate {
                return None;
            }

            let path_key = file.path.to_string_lossy().to_string();
            let can_reuse_hash = cached_files
                .get(&path_key)
                .filter(|entry| cache_enabled && entry.matches(file))
                .and_then(CachedFileRecord::normalized_hash)
                .is_some();

            if can_reuse_hash {
                None
            } else {
                Some(index)
            }
        })
        .collect()
}

fn hash_worker_count(job_count: usize) -> usize {
    let available = thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(2);

    available.min(MAX_HASH_WORKERS).min(job_count.max(1))
}

fn ensure_creator(
    transaction: &Transaction<'_>,
    creator_cache: &mut HashMap<String, i64>,
    creator_name: &str,
) -> AppResult<i64> {
    if let Some(existing) = creator_cache.get(creator_name) {
        return Ok(*existing);
    }

    if let Some(existing) = transaction
        .query_row(
            "SELECT id FROM creators WHERE canonical_name = ?1",
            params![creator_name],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
    {
        creator_cache.insert(creator_name.to_owned(), existing);
        return Ok(existing);
    }

    transaction.execute(
        "INSERT INTO creators (canonical_name, notes, created_by_user)
         VALUES (?1, ?2, 0)",
        params![creator_name, "Discovered from filename parsing"],
    )?;

    let creator_id = transaction.last_insert_rowid();
    creator_cache.insert(creator_name.to_owned(), creator_id);
    Ok(creator_id)
}

fn insert_cached_file(
    transaction: &Transaction<'_>,
    file_insert: &mut rusqlite::Statement<'_>,
    review_insert: &mut rusqlite::Statement<'_>,
    session_id: i64,
    file: &DiscoveredFile,
    cached: &CachedFileRecord,
    hashed_override: Option<String>,
) -> AppResult<usize> {
    let safety_notes = parse_string_array(&cached.safety_notes_json);
    let parser_warnings = parse_string_array(&cached.parser_warnings_json);
    let hash = hashed_override.or_else(|| cached.normalized_hash());

    file_insert.execute(params![
        file.path.to_string_lossy().to_string(),
        &file.filename,
        &file.extension,
        hash.as_deref(),
        file.size,
        file.created_at.as_deref(),
        file.modified_at.as_deref(),
        cached.creator_id,
        &cached.kind,
        cached.subtype.as_deref(),
        cached.confidence,
        &file.source_location,
        session_id,
        file.relative_depth,
        &cached.safety_notes_json,
        &cached.parser_warnings_json,
        &cached.insights_json,
    ])?;
    let file_id = transaction.last_insert_rowid();

    queue_review_items(
        review_insert,
        file_id,
        cached.confidence,
        &parser_warnings,
        &safety_notes,
    )
}

pub(crate) fn insert_parsed_file(
    transaction: &Transaction<'_>,
    creator_cache: &mut HashMap<String, i64>,
    seed_pack: &crate::seed::SeedPack,
    category_overrides: &HashMap<String, database::UserCategoryOverride>,
    file_insert: &mut rusqlite::Statement<'_>,
    review_insert: &mut rusqlite::Statement<'_>,
    session_id: Option<i64>,
    file: &DiscoveredFile,
    hash: Option<String>,
) -> AppResult<usize> {
    let mut classification = parse_filename(&file.filename, seed_pack);
    let inspection = inspect_file(&file.path, &file.extension, seed_pack).unwrap_or_default();
    apply_folder_creator_hint(&mut classification, file, seed_pack);
    apply_inspection_hints(&mut classification, &inspection, seed_pack);
    apply_creator_profile_hints(&mut classification, seed_pack);
    apply_category_override(&mut classification, file, category_overrides);

    let creator_id = match classification.possible_creator.as_deref() {
        Some(name) => Some(ensure_creator(transaction, creator_cache, name)?),
        None => None,
    };
    let safety_notes = collect_safety_notes(file, &classification);
    let safety_notes_json = serde_json::to_string(&safety_notes)?;
    let parser_warnings_json = serde_json::to_string(&classification.warning_flags)?;
    let insights_json = serde_json::to_string(&inspection.insights)?;

    file_insert.execute(params![
        file.path.to_string_lossy().to_string(),
        &file.filename,
        &file.extension,
        hash.as_deref(),
        file.size,
        file.created_at.as_deref(),
        file.modified_at.as_deref(),
        creator_id,
        classification.kind,
        classification.subtype,
        classification.confidence,
        &file.source_location,
        session_id,
        file.relative_depth,
        &safety_notes_json,
        &parser_warnings_json,
        &insights_json,
    ])?;
    let file_id = transaction.last_insert_rowid();

    queue_review_items(
        review_insert,
        file_id,
        classification.confidence,
        &classification.warning_flags,
        &safety_notes,
    )
}

fn apply_folder_creator_hint(
    classification: &mut FilenameClassification,
    file: &DiscoveredFile,
    seed_pack: &crate::seed::SeedPack,
) {
    let path_hint = detect_creator_from_path(file, seed_pack);

    if let Some(folder_creator) = path_hint.as_deref() {
        apply_creator_signal(classification, folder_creator, 0.12, seed_pack);
    }
}

fn detect_creator_from_path(
    file: &DiscoveredFile,
    seed_pack: &crate::seed::SeedPack,
) -> Option<String> {
    let relative = file.path.strip_prefix(&file.root_path).ok()?;
    let parent = relative.parent()?;

    for segment in parent
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .rev()
        .take(4)
    {
        if is_generic_folder_name(&segment) {
            continue;
        }

        if let Some(creator) = detect_creator_hint(&segment, seed_pack) {
            if is_known_creator_name(&creator, seed_pack) {
                return Some(creator);
            }
        }
    }

    None
}

fn is_generic_folder_name(value: &str) -> bool {
    let normalized = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();

    matches!(
        normalized.as_str(),
        "mods"
            | "mod"
            | "downloads"
            | "review"
            | "unsorted"
            | "tray"
            | "importedtray"
            | "cas"
            | "hair"
            | "skin"
            | "skins"
            | "buildbuy"
            | "build"
            | "buy"
            | "gameplay"
            | "scriptmods"
            | "scripts"
            | "overrides"
            | "poses"
            | "presets"
            | "furniture"
            | "decor"
            | "clutter"
            | "lighting"
            | "kitchen"
            | "bathroom"
            | "bedroom"
            | "living"
            | "dining"
            | "misc"
            | "unknown"
    )
}

fn apply_inspection_hints(
    classification: &mut FilenameClassification,
    inspection: &InspectionOutcome,
    seed_pack: &crate::seed::SeedPack,
) {
    let inspection_creator_hints = if inspection.insights.creator_hints.is_empty() {
        inspection
            .creator_hint
            .as_deref()
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        inspection
            .insights
            .creator_hints
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    };

    if let Some(current) = classification.possible_creator.as_deref() {
        if let Some(matching_hint) = inspection_creator_hints
            .iter()
            .copied()
            .find(|hint| same_creator_identity(current, hint))
        {
            if is_known_creator_name(matching_hint, seed_pack) && current != matching_hint {
                classification.possible_creator = Some(matching_hint.to_owned());
            }
            classification.confidence += 0.04;
        } else if let Some(creator) = preferred_creator_signal(&inspection_creator_hints, seed_pack)
        {
            apply_creator_signal(
                classification,
                creator,
                inspection.confidence_boost,
                seed_pack,
            );
        }
    } else if let Some(creator) = preferred_creator_signal(&inspection_creator_hints, seed_pack) {
        apply_creator_signal(
            classification,
            creator,
            inspection.confidence_boost,
            seed_pack,
        );
    }

    if let Some(kind_hint) = inspection.kind_hint.as_deref() {
        let inspection_supported_kind = matches!(
            kind_hint,
            "CAS"
                | "BuildBuy"
                | "Gameplay"
                | "PresetsAndSliders"
                | "ScriptMods"
                | "OverridesAndDefaults"
        );
        let confirms_current_kind = classification.kind == kind_hint;
        let can_override = classification.kind == "Unknown"
            || (classification.confidence < 0.7 && inspection_supported_kind);

        if can_override {
            classification.kind = kind_hint.to_owned();
            classification.confidence += 0.08;
            if let Some(subtype) = inspection.subtype_hint.as_deref() {
                if classification.subtype.is_none() {
                    classification.subtype = Some(subtype.to_owned());
                }
            }
        }

        if inspection_supported_kind && (can_override || confirms_current_kind) {
            classification.confidence = classification
                .confidence
                .max(inspection.kind_confidence_floor);
            classification.warning_flags.retain(|warning| {
                warning != "no_category_detected" && warning != "conflicting_category_signals"
            });
        }
    }
}

fn apply_creator_signal(
    classification: &mut FilenameClassification,
    creator_signal: &str,
    set_boost: f64,
    seed_pack: &crate::seed::SeedPack,
) {
    match classification.possible_creator.as_deref() {
        None => {
            classification.possible_creator = Some(creator_signal.to_owned());
            classification.confidence += set_boost;
        }
        Some(current) if same_creator_identity(current, creator_signal) => {
            if is_known_creator_name(creator_signal, seed_pack) && current != creator_signal {
                classification.possible_creator = Some(creator_signal.to_owned());
            }
            classification.confidence += 0.04;
        }
        Some(current) => {
            let current_is_known = is_known_creator_name(current, seed_pack);
            let signal_is_known = is_known_creator_name(creator_signal, seed_pack);

            match (current_is_known, signal_is_known) {
                (false, true) => {
                    classification.possible_creator = Some(creator_signal.to_owned());
                    classification.confidence += (set_boost * 0.75).max(0.04);
                }
                (true, false) | (false, false) => {}
                (true, true) => push_creator_conflict_warning(classification),
            }
        }
    }
}

fn preferred_creator_signal<'a>(
    creator_signals: &[&'a str],
    seed_pack: &crate::seed::SeedPack,
) -> Option<&'a str> {
    creator_signals
        .iter()
        .copied()
        .find(|value| is_known_creator_name(value, seed_pack))
        .or_else(|| creator_signals.first().copied())
}

fn same_creator_identity(left: &str, right: &str) -> bool {
    normalize_key(left) == normalize_key(right)
}

fn is_known_creator_name(value: &str, seed_pack: &crate::seed::SeedPack) -> bool {
    seed_pack.creator_profiles.contains_key(value)
}

fn push_creator_conflict_warning(classification: &mut FilenameClassification) {
    if !classification
        .warning_flags
        .iter()
        .any(|flag| flag == "conflicting_creator_signals")
    {
        classification
            .warning_flags
            .push("conflicting_creator_signals".to_owned());
    }
}

fn apply_creator_profile_hints(
    classification: &mut FilenameClassification,
    seed_pack: &crate::seed::SeedPack,
) {
    let Some(creator) = classification.possible_creator.as_deref() else {
        return;
    };
    let Some(profile) = seed_pack.creator_profiles.get(creator) else {
        return;
    };

    if classification.kind == "Unknown" {
        if let Some(kind) = profile.likely_kinds.first() {
            classification.kind = kind.clone();
            classification.confidence += 0.16;
        }
    }

    if classification.subtype.is_none() {
        classification.subtype = profile.likely_subtypes.first().cloned();
    }
}

fn collect_safety_notes(
    file: &DiscoveredFile,
    classification: &FilenameClassification,
) -> Vec<String> {
    let mut notes = Vec::new();

    if file.extension == ".ts4script" && file.relative_depth > 1 {
        notes.push("unsafe_script_depth".to_owned());
    }

    if file.source_location == "mods" && classification.kind.starts_with("Tray") {
        notes.push("tray_file_in_mods_root".to_owned());
    }

    notes
}

fn apply_category_override(
    classification: &mut FilenameClassification,
    file: &DiscoveredFile,
    category_overrides: &HashMap<String, database::UserCategoryOverride>,
) {
    let path_key = normalize_override_key(&file.path.to_string_lossy());
    let Some(override_record) = category_overrides.get(&path_key) else {
        return;
    };

    classification.kind = override_record.kind.clone();
    classification.subtype = override_record.subtype.clone();
    classification.confidence = classification.confidence.max(0.82);
    classification
        .warning_flags
        .retain(|warning| warning != "no_category_detected");
}

fn queue_review_items(
    review_insert: &mut rusqlite::Statement<'_>,
    file_id: i64,
    confidence: f64,
    warning_flags: &[String],
    safety_notes: &[String],
) -> AppResult<usize> {
    let mut reasons = Vec::new();
    if confidence < 0.55 {
        reasons.push("low_confidence_parse".to_owned());
    }
    reasons.extend(warning_flags.iter().cloned());
    reasons.extend(safety_notes.iter().cloned());

    let mut created = 0;
    for reason in reasons {
        review_insert.execute(params![file_id, reason, confidence])?;
        created += 1;
    }

    Ok(created)
}

pub(crate) fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn normalize_extension(path: &Path) -> String {
    path.extension()
        .map(|value| format!(".{}", value.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

fn is_supported_extension(root_type: RootType, extension: &str) -> bool {
    match root_type {
        RootType::Mods => matches!(
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
        ),
        RootType::Tray => matches!(
            extension,
            ".trayitem"
                | ".blueprint"
                | ".bpi"
                | ".householdbinary"
                | ".hhi"
                | ".sgi"
                | ".room"
                | ".rmi"
        ),
    }
}

fn component_count(path: &Path) -> usize {
    path.components().count()
}

fn system_time_to_rfc3339(time: std::time::SystemTime) -> String {
    let date_time: DateTime<Utc> = time.into();
    date_time.to_rfc3339()
}

fn parse_string_array(value: &str) -> Vec<String> {
    serde_json::from_str(value).unwrap_or_default()
}

fn scan_mode_label(mode: &ScanMode) -> &'static str {
    match mode {
        ScanMode::Full => "full",
        ScanMode::Incremental => "incremental",
    }
}

fn cache_fingerprint(
    seed_pack: &crate::seed::SeedPack,
    creator_learning_version: Option<&str>,
    category_override_version: Option<&str>,
) -> String {
    format!(
        "{SCAN_CACHE_VERSION}:{}:{}:{}",
        seed_pack.seed_version,
        creator_learning_version.unwrap_or("none"),
        category_override_version.unwrap_or("none"),
    )
}

fn current_cache_fingerprint(
    connection: &Connection,
    seed_pack: &crate::seed::SeedPack,
) -> AppResult<String> {
    let creator_learning_version = database::get_creator_learning_version(connection)?;
    let category_override_version = database::get_category_override_version(connection)?;
    Ok(cache_fingerprint(
        seed_pack,
        creator_learning_version.as_deref(),
        category_override_version.as_deref(),
    ))
}

fn normalize_override_key(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, fs, sync::Arc};

    use tempfile::tempdir;

    use crate::{core::library_index, database, seed::load_seed_pack};

    use super::*;

    fn discovered_file(name: &str, size: i64) -> DiscoveredFile {
        DiscoveredFile {
            root_path: PathBuf::from("Mods"),
            source_location: "mods".to_owned(),
            path: PathBuf::from(name),
            filename: name.to_owned(),
            extension: ".package".to_owned(),
            size,
            created_at: None,
            modified_at: None,
            relative_depth: 0,
        }
    }

    fn build_state(
        temp: &tempfile::TempDir,
        seed_pack: crate::seed::SeedPack,
        configure: impl FnOnce(&mut rusqlite::Connection),
    ) -> AppState {
        let database_path = temp.path().join("test.sqlite3");
        let mut connection = rusqlite::Connection::open(&database_path).expect("db");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("fk");
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .expect("wal");
        database::initialize(&mut connection).expect("schema");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");
        configure(&mut connection);

        AppState {
            database_path,
            seed_pack: Arc::new(seed_pack),
            scan_status: Arc::new(Mutex::new(crate::models::ScanStatus::default())),
            downloads_status: Arc::new(
                Mutex::new(crate::models::DownloadsWatcherStatus::default()),
            ),
            keep_running_in_background: Arc::new(Mutex::new(false)),
            automatic_watch_checks: Arc::new(Mutex::new(false)),
            watch_check_interval_hours: Arc::new(Mutex::new(12)),
            downloads_watcher_control: Arc::new(Mutex::new(
                crate::app_state::DownloadsWatcherControl::default(),
            )),
            watch_polling_control: Arc::new(Mutex::new(
                crate::app_state::WatchPollingControl::default(),
            )),
            downloads_processing_lock: Arc::new(Mutex::new(())),
            app_data_dir: temp.path().to_path_buf(),
        }
    }

    #[test]
    fn stale_refresh_flag_stays_off_for_a_brand_new_database() {
        let temp = tempdir().expect("tempdir");
        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |_| {});
        let connection = state.connection().expect("connection");
        let seed_pack = state.seed_pack();

        let needs_refresh =
            library_scan_needs_refresh(&connection, seed_pack.as_ref()).expect("refresh flag");

        assert!(!needs_refresh);
    }

    #[test]
    fn stale_refresh_flag_turns_on_for_existing_data_with_an_old_fingerprint() {
        let temp = tempdir().expect("tempdir");
        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |connection| {
            connection
                .execute(
                    "INSERT INTO scan_sessions (scan_type, started_at, completed_at, files_scanned, errors)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        "full",
                        "2026-03-16T00:00:00+00:00",
                        "2026-03-16T00:10:00+00:00",
                        10_i64,
                        ""
                    ],
                )
                .expect("scan session");
            database::save_app_setting(
                connection,
                SCAN_CACHE_FINGERPRINT_KEY,
                Some("scanner-v14:test:test:test"),
                "user",
            )
            .expect("save old fingerprint");
        });
        let connection = state.connection().expect("connection");
        let seed_pack = state.seed_pack();

        let needs_refresh =
            library_scan_needs_refresh(&connection, seed_pack.as_ref()).expect("refresh flag");

        assert!(needs_refresh);
    }

    #[test]
    fn collect_supported_files_reports_progress_while_walking() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        fs::create_dir_all(mods.join("Nested")).expect("mods tree");
        fs::write(mods.join("alpha.package"), b"package").expect("package");
        fs::write(mods.join("Nested").join("beta.ts4script"), b"script").expect("script");

        let roots = vec![ScanRoot {
            root_type: RootType::Mods,
            path: mods,
        }];
        let mut errors = Vec::new();
        let seen_counts = RefCell::new(Vec::new());

        let discovered =
            collect_supported_files_with_progress(&roots, &mut errors, |count, _path| {
                seen_counts.borrow_mut().push(count);
                Ok(())
            })
            .expect("walk succeeds");

        assert!(errors.is_empty());
        assert_eq!(discovered.len(), 2);
        assert_eq!(*seen_counts.borrow(), vec![1]);
    }

    #[test]
    fn hash_candidates_only_include_shared_sizes() {
        let discovered = vec![
            discovered_file("alpha.package", 10),
            discovered_file("beta.package", 24),
            discovered_file("gamma.package", 24),
            discovered_file("delta.package", 61),
        ];

        assert_eq!(
            collect_hash_candidate_indices(&discovered, &HashMap::new(), false),
            vec![1, 2]
        );
    }

    #[test]
    fn hash_candidates_skip_unchanged_cached_hashes() {
        let discovered = vec![
            discovered_file("same.package", 24),
            discovered_file("other.package", 24),
        ];
        let mut cached = HashMap::new();
        cached.insert(
            "same.package".to_owned(),
            CachedFileRecord {
                path: "same.package".to_owned(),
                filename: "same.package".to_owned(),
                extension: ".package".to_owned(),
                hash: Some("cached-hash".to_owned()),
                size: 24,
                created_at: None,
                modified_at: None,
                creator_id: None,
                kind: "CAS".to_owned(),
                subtype: None,
                confidence: 0.8,
                source_location: "mods".to_owned(),
                relative_depth: 0,
                safety_notes_json: "[]".to_owned(),
                parser_warnings_json: "[]".to_owned(),
                insights_json: "{}".to_owned(),
            },
        );

        assert_eq!(
            collect_hash_candidate_indices(&discovered, &cached, true),
            vec![1]
        );
    }

    #[test]
    fn scan_records_tray_bundles_and_safety_notes() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        fs::create_dir_all(mods.join("Scripts").join("TooDeep")).expect("mods tree");
        fs::create_dir_all(&tray).expect("tray");

        fs::write(
            mods.join("Scripts").join("TooDeep").join("deep.ts4script"),
            b"test",
        )
        .expect("script");
        fs::write(mods.join("0x00112233.trayitem"), b"tray").expect("tray item");
        fs::write(tray.join("0x00112233.householdbinary"), b"household").expect("household");
        fs::write(tray.join("0x00112233.hhi"), b"hhi").expect("hhi");

        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |connection| {
            database::save_library_paths(
                connection,
                &crate::models::LibrarySettings {
                    mods_path: Some(mods.to_string_lossy().to_string()),
                    tray_path: Some(tray.to_string_lossy().to_string()),
                    downloads_path: None,
                    ..Default::default()
                },
            )
            .expect("save settings");
        });

        let summary = scan_library_with_progress(&state, |_| Ok(())).expect("scan");

        let connection = state.connection().expect("connection");
        let settings = database::get_library_settings(&connection).expect("settings");
        let seed_pack = state.seed_pack();
        let overview =
            library_index::get_home_overview(&connection, &settings, &seed_pack).expect("overview");
        assert_eq!(summary.scan_mode, ScanMode::Full);
        assert_eq!(overview.total_files, 4);
        assert!(overview.unsafe_count >= 1);
        assert!(overview.bundles_count >= 1);
    }

    #[test]
    fn scan_hashes_duplicate_candidates_without_missing_exact_duplicates() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        fs::create_dir_all(&mods).expect("mods");

        fs::write(mods.join("same_a.package"), b"same-bytes").expect("same_a");
        fs::write(mods.join("same_b.package"), b"same-bytes").expect("same_b");
        fs::write(mods.join("unique.package"), b"something-longer-and-unique").expect("unique");

        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |connection| {
            database::save_library_paths(
                connection,
                &crate::models::LibrarySettings {
                    mods_path: Some(mods.to_string_lossy().to_string()),
                    tray_path: None,
                    downloads_path: None,
                    ..Default::default()
                },
            )
            .expect("save settings");
        });

        let summary = scan_library_with_progress(&state, |_| Ok(())).expect("scan");

        let connection = state.connection().expect("connection");
        let duplicates: i64 = connection
            .query_row("SELECT COUNT(*) FROM duplicates", [], |row| row.get(0))
            .expect("duplicate count");
        let hashed_files: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM files WHERE hash IS NOT NULL AND hash <> ''",
                [],
                |row| row.get(0),
            )
            .expect("hash count");

        assert_eq!(summary.hashed_files, 2);
        assert_eq!(duplicates, 1);
        assert_eq!(hashed_files, 2);
    }

    #[test]
    fn scan_uses_folder_names_as_creator_hints() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        fs::create_dir_all(mods.join("LittleMsSam")).expect("mods tree");
        fs::write(
            mods.join("LittleMsSam").join("SendSimsToBed.package"),
            b"placeholder",
        )
        .expect("package");

        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |connection| {
            database::save_library_paths(
                connection,
                &crate::models::LibrarySettings {
                    mods_path: Some(mods.to_string_lossy().to_string()),
                    tray_path: None,
                    downloads_path: None,
                    ..Default::default()
                },
            )
            .expect("save settings");
        });

        scan_library_with_progress(&state, |_| Ok(())).expect("scan");

        let connection = state.connection().expect("connection");
        let listing =
            library_index::list_library_files(&connection, crate::models::LibraryQuery::default())
                .expect("listing");

        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].creator.as_deref(), Some("LittleMsSam"));
        assert_eq!(listing.items[0].kind, "Gameplay");
    }

    #[test]
    fn folder_creator_hint_prefers_known_folder_creator_over_weak_filename_creator() {
        let seed_pack = load_seed_pack().expect("seed");
        let mut classification = parse_filename("ESTATE_Brick_Wall.package", &seed_pack);
        let file = DiscoveredFile {
            root_path: PathBuf::from("C:/Mods"),
            source_location: "mods".to_owned(),
            path: PathBuf::from("C:/Mods/Felixandre ESTATE Part 1/ESTATE_Brick_Wall.package"),
            filename: "ESTATE_Brick_Wall.package".to_owned(),
            extension: ".package".to_owned(),
            size: 1,
            created_at: None,
            modified_at: None,
            relative_depth: 1,
        };

        apply_folder_creator_hint(&mut classification, &file, &seed_pack);

        assert_eq!(
            classification.possible_creator.as_deref(),
            Some("Felixandre")
        );
        assert!(!classification
            .warning_flags
            .contains(&"conflicting_creator_signals".to_owned()));
    }

    #[test]
    fn folder_creator_hint_skips_unknown_nearest_folder_names() {
        let seed_pack = load_seed_pack().expect("seed");
        let mut classification = parse_filename("LittleMsSam_SendSimsToBed.package", &seed_pack);
        let file = DiscoveredFile {
            root_path: PathBuf::from("C:/Mods"),
            source_location: "mods".to_owned(),
            path: PathBuf::from(
                "C:/Mods/LittleMsSam_Mods/SleepOverhaul/LittleMsSam_SendSimsToBed.package",
            ),
            filename: "LittleMsSam_SendSimsToBed.package".to_owned(),
            extension: ".package".to_owned(),
            size: 1,
            created_at: None,
            modified_at: None,
            relative_depth: 2,
        };

        apply_folder_creator_hint(&mut classification, &file, &seed_pack);

        assert_eq!(
            classification.possible_creator.as_deref(),
            Some("LittleMsSam")
        );
        assert!(!classification
            .warning_flags
            .contains(&"conflicting_creator_signals".to_owned()));
    }

    #[test]
    fn folder_creator_hint_keeps_conflict_for_two_known_creators() {
        let seed_pack = load_seed_pack().expect("seed");
        let mut classification = parse_filename("LittleMsSam_SendSimsToBed.package", &seed_pack);
        let file = DiscoveredFile {
            root_path: PathBuf::from("C:/Mods"),
            source_location: "mods".to_owned(),
            path: PathBuf::from("C:/Mods/Deaderpool_Collection/LittleMsSam_SendSimsToBed.package"),
            filename: "LittleMsSam_SendSimsToBed.package".to_owned(),
            extension: ".package".to_owned(),
            size: 1,
            created_at: None,
            modified_at: None,
            relative_depth: 1,
        };

        apply_folder_creator_hint(&mut classification, &file, &seed_pack);

        assert_eq!(
            classification.possible_creator.as_deref(),
            Some("LittleMsSam")
        );
        assert!(classification
            .warning_flags
            .contains(&"conflicting_creator_signals".to_owned()));
    }

    #[test]
    fn inspection_hints_clear_stale_category_warnings_and_raise_confidence() {
        let mut classification = crate::core::filename_parser::FilenameClassification {
            normalized: "aesthetic_walls".to_owned(),
            tokens: vec!["aesthetic".to_owned(), "walls".to_owned()],
            possible_creator: None,
            kind: "Unknown".to_owned(),
            subtype: None,
            set_name: None,
            version_label: None,
            support_tokens: Vec::new(),
            warning_flags: vec![
                "no_category_detected".to_owned(),
                "conflicting_category_signals".to_owned(),
            ],
            confidence: 0.18,
        };
        let inspection = crate::core::file_inspector::InspectionOutcome {
            kind_hint: Some("BuildBuy".to_owned()),
            subtype_hint: Some("Build Surfaces".to_owned()),
            kind_confidence_floor: 0.7,
            ..Default::default()
        };

        let seed_pack = load_seed_pack().expect("seed");
        apply_inspection_hints(&mut classification, &inspection, &seed_pack);

        assert_eq!(classification.kind, "BuildBuy");
        assert_eq!(classification.subtype.as_deref(), Some("Build Surfaces"));
        assert!(classification.confidence >= 0.7);
        assert!(!classification
            .warning_flags
            .contains(&"no_category_detected".to_owned()));
        assert!(!classification
            .warning_flags
            .contains(&"conflicting_category_signals".to_owned()));
    }

    #[test]
    fn inspection_hints_do_not_flag_conflict_when_current_creator_is_in_hint_list() {
        let mut classification = crate::core::filename_parser::FilenameClassification {
            normalized: "thepancake1_spiralstaircases".to_owned(),
            tokens: vec!["thepancake1".to_owned(), "spiralstaircases".to_owned()],
            possible_creator: Some("thepancake1".to_owned()),
            kind: "ScriptMods".to_owned(),
            subtype: Some("Utilities".to_owned()),
            set_name: None,
            version_label: None,
            support_tokens: Vec::new(),
            warning_flags: Vec::new(),
            confidence: 0.9,
        };
        let inspection = crate::core::file_inspector::InspectionOutcome {
            creator_hint: Some("MizoreYukii".to_owned()),
            confidence_boost: 0.16,
            insights: crate::models::FileInsights {
                creator_hints: vec!["MizoreYukii".to_owned(), "thepancake1".to_owned()],
                ..Default::default()
            },
            ..Default::default()
        };
        let seed_pack = load_seed_pack().expect("seed");

        apply_inspection_hints(&mut classification, &inspection, &seed_pack);

        assert_eq!(
            classification.possible_creator.as_deref(),
            Some("thepancake1")
        );
        assert!(!classification
            .warning_flags
            .contains(&"conflicting_creator_signals".to_owned()));
    }

    #[test]
    fn inspection_hints_can_promote_unknown_files_to_gameplay() {
        let mut classification = crate::core::filename_parser::FilenameClassification {
            normalized: "plumlace_mental_wellness".to_owned(),
            tokens: vec![
                "plumlace".to_owned(),
                "mental".to_owned(),
                "wellness".to_owned(),
            ],
            possible_creator: Some("Plumlace".to_owned()),
            kind: "Unknown".to_owned(),
            subtype: None,
            set_name: None,
            version_label: None,
            support_tokens: Vec::new(),
            warning_flags: vec!["no_category_detected".to_owned()],
            confidence: 0.24,
        };
        let inspection = crate::core::file_inspector::InspectionOutcome {
            kind_hint: Some("Gameplay".to_owned()),
            subtype_hint: Some("Gameplay".to_owned()),
            kind_confidence_floor: 0.58,
            ..Default::default()
        };

        let seed_pack = load_seed_pack().expect("seed");
        apply_inspection_hints(&mut classification, &inspection, &seed_pack);

        assert_eq!(classification.kind, "Gameplay");
        assert_eq!(classification.subtype.as_deref(), Some("Gameplay"));
        assert!(classification.confidence >= 0.58);
        assert!(!classification
            .warning_flags
            .contains(&"no_category_detected".to_owned()));
    }

    #[test]
    fn second_scan_reuses_unchanged_cached_entries() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        fs::create_dir_all(&mods).expect("mods");
        fs::write(mods.join("same_a.package"), b"same-bytes").expect("same_a");
        fs::write(mods.join("same_b.package"), b"same-bytes").expect("same_b");

        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |connection| {
            database::save_library_paths(
                connection,
                &crate::models::LibrarySettings {
                    mods_path: Some(mods.to_string_lossy().to_string()),
                    tray_path: None,
                    downloads_path: None,
                    ..Default::default()
                },
            )
            .expect("save settings");
        });

        let first = scan_library_with_progress(&state, |_| Ok(())).expect("first");
        let second = scan_library_with_progress(&state, |_| Ok(())).expect("second");

        assert_eq!(first.scan_mode, ScanMode::Full);
        assert_eq!(second.scan_mode, ScanMode::Incremental);
        assert_eq!(second.reused_files, 2);
        assert_eq!(second.hashed_files, 0);
    }

    #[test]
    fn category_override_is_applied_on_rescan() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        fs::create_dir_all(&mods).expect("mods");
        fs::write(mods.join("mystery.package"), b"placeholder").expect("package");

        let seed_pack = load_seed_pack().expect("seed");
        let state = build_state(&temp, seed_pack, |connection| {
            database::save_library_paths(
                connection,
                &crate::models::LibrarySettings {
                    mods_path: Some(mods.to_string_lossy().to_string()),
                    tray_path: None,
                    downloads_path: None,
                    ..Default::default()
                },
            )
            .expect("save settings");
        });

        scan_library_with_progress(&state, |_| Ok(())).expect("initial scan");
        {
            let mut connection = state.connection().expect("connection");
            let file_id: i64 = connection
                .query_row(
                    "SELECT id FROM files WHERE filename = ?1",
                    params!["mystery.package"],
                    |row| row.get(0),
                )
                .expect("file id");
            database::save_category_override(&mut connection, file_id, "Gameplay", Some("Utility"))
                .expect("save override");
        }

        let summary = scan_library_with_progress(&state, |_| Ok(())).expect("rescan");
        let connection = state.connection().expect("connection");
        let listing =
            library_index::list_library_files(&connection, crate::models::LibraryQuery::default())
                .expect("listing");

        assert_eq!(summary.scan_mode, ScanMode::Full);
        assert_eq!(listing.items[0].kind, "Gameplay");
        assert_eq!(listing.items[0].subtype.as_deref(), Some("Utility"));
    }
}
