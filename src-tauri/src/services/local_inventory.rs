use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::error::{AppError, AppResult};
use crate::models::{LocalFile, LocalMod, TrackingMode, UpdateStatus};

pub struct LocalInventory;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalModScanResult {
    pub mods_found: i64,
    pub files_processed: i64,
    pub new_mods: i64,
    pub updated_mods: i64,
}

impl LocalInventory {
    pub fn scan_and_update_local_mods(
        conn: &Connection,
        mods_path: &Path,
    ) -> AppResult<LocalModScanResult> {
        let mut mods_found = 0i64;
        let mut files_processed = 0i64;
        let mut new_mods = 0i64;
        let mut updated_mods = 0i64;

        if !mods_path.exists() {
            tracing::info!("Mods path does not exist: {:?}", mods_path);
            return Ok(LocalModScanResult {
                mods_found,
                files_processed,
                new_mods,
                updated_mods,
            });
        }

        tracing::info!("Starting scan of mods directory: {:?}", mods_path);

        let mut folder_to_files: std::collections::HashMap<String, Vec<std::path::PathBuf>> =
            std::collections::HashMap::new();

        for entry in WalkDir::new(mods_path)
            .max_depth(1)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() {
                folder_to_files.insert(path.to_string_lossy().to_string(), Vec::new());
            }
        }

        let walkdir_count = WalkDir::new(mods_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .count();
        tracing::debug!("WalkDir found {} entries in {:?}", walkdir_count, mods_path);

        for entry in WalkDir::new(mods_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if ext != "package" && ext != "ts4script" {
                continue;
            }
            if let Some(parent) = path.parent() {
                let parent_str = parent.to_string_lossy().to_string();
                if let Some(folder_path) = mods_path
                    .canonicalize()
                    .ok()
                    .and_then(|mp| {
                        parent.canonicalize().ok().and_then(|p| {
                            p.strip_prefix(mp)
                                .ok()
                                .map(|s| s.to_string_lossy().to_string())
                        })
                    })
                    .or_else(|| {
                        if parent_str.starts_with(mods_path.to_string_lossy().as_ref()) {
                            Some(parent_str.clone())
                        } else {
                            None
                        }
                    })
                {
                    folder_to_files
                        .entry(folder_path)
                        .or_insert_with(Vec::new)
                        .push(path.to_path_buf());
                }
            }
        }

        tracing::info!("Found {} mod folders to process", folder_to_files.len());

        for (folder_path, files) in folder_to_files {
            if files.is_empty() {
                continue;
            }

            let folder_name = std::path::Path::new(&folder_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let new_fingerprint =
                Self::compute_folder_fingerprint(std::path::Path::new(&folder_path))?;

            if let Some(existing_mod_id) = Self::detect_folder_rename(
                conn,
                std::path::Path::new(&folder_path),
                &new_fingerprint,
            )? {
                Self::update_local_mod_path(
                    conn,
                    &existing_mod_id,
                    std::path::Path::new(&folder_path),
                )?;
                tracing::info!(
                    "Updated path for mod {} to {}",
                    existing_mod_id,
                    folder_path
                );
                updated_mods += 1;
                mods_found += 1;
                for file_path in &files {
                    if let Some(file_record) =
                        Self::process_file(conn, &existing_mod_id, file_path)?
                    {
                        files_processed += 1;
                        let _ = file_record;
                    }
                }
                continue;
            }

            let existing_id = Self::find_local_mod_by_folder(conn, &folder_path)?;

            let mod_id = if let Some(id) = existing_id {
                updated_mods += 1;
                id
            } else {
                new_mods += 1;
                Self::create_local_mod(conn, &folder_name, &folder_path)?
            };

            mods_found += 1;

            for file_path in &files {
                if let Some(file_record) = Self::process_file(conn, &mod_id, file_path)? {
                    files_processed += 1;
                    let _ = file_record;
                }
            }
        }

        tracing::info!(
            "Scan complete: {} mods found ({} new, {} updated), {} files processed",
            mods_found,
            new_mods,
            updated_mods,
            files_processed
        );

        Ok(LocalModScanResult {
            mods_found,
            files_processed,
            new_mods,
            updated_mods,
        })
    }

    pub fn get_or_create_local_mod(
        conn: &Connection,
        display_name: &str,
        folder_path: &str,
    ) -> AppResult<String> {
        if folder_path.is_empty() {
            return Err(AppError::Message("folder_path cannot be empty".into()));
        }

        if let Some(existing_id) = Self::find_local_mod_by_folder(conn, folder_path)? {
            tracing::debug!(
                "Found existing local mod {} for folder {}",
                existing_id,
                folder_path
            );
            return Ok(existing_id);
        }

        Self::create_local_mod(conn, display_name, folder_path)
    }

    pub fn get_local_mod(conn: &Connection, mod_id: &str) -> AppResult<Option<LocalMod>> {
        if mod_id.is_empty() {
            return Err(AppError::Message("mod_id cannot be empty".into()));
        }

        let result = conn
            .query_row(
                "SELECT id, display_name, normalized_name, creator_name, category,
                        local_root_path, tracking_mode, source_confidence, confirmed_source_id,
                        current_status, last_checked_at, created_at, updated_at
                 FROM local_mods
                 WHERE id = ?1",
                params![mod_id],
                |row| {
                    Ok(LocalMod {
                        id: row.get(0)?,
                        display_name: row.get(1)?,
                        normalized_name: row.get(2)?,
                        creator_name: row.get(3)?,
                        category: row.get(4)?,
                        local_root_path: row.get(5)?,
                        tracking_mode: parse_tracking_mode(&row.get::<_, String>(6)?),
                        source_confidence: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                        confirmed_source_id: row.get(8)?,
                        current_status: parse_update_status(&row.get::<_, String>(9)?),
                        last_checked_at: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn get_local_files(conn: &Connection, mod_id: &str) -> AppResult<Vec<LocalFile>> {
        if mod_id.is_empty() {
            return Err(AppError::Message("mod_id cannot be empty".into()));
        }

        let mut statement = conn.prepare(
            "SELECT id, local_mod_id, file_path, file_name, file_ext,
                    file_size, sha256, modified_at
             FROM local_files
             WHERE local_mod_id = ?1
             ORDER BY file_name",
        )?;

        let files = statement
            .query_map(params![mod_id], |row| {
                Ok(LocalFile {
                    id: row.get(0)?,
                    local_mod_id: row.get(1)?,
                    file_path: row.get(2)?,
                    file_name: row.get(3)?,
                    file_ext: row.get(4)?,
                    file_size: row.get(5)?,
                    sha256: row.get(6)?,
                    modified_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    pub fn update_mod_status(
        conn: &Connection,
        mod_id: &str,
        status: UpdateStatus,
        source_confidence: f64,
        confirmed_source_id: Option<&str>,
    ) -> AppResult<()> {
        if mod_id.is_empty() {
            return Err(AppError::Message("mod_id cannot be empty".into()));
        }

        if source_confidence < 0.0 || source_confidence > 1.0 {
            tracing::warn!(
                "source_confidence {} is outside valid range [0, 1]",
                source_confidence
            );
        }

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE local_mods
             SET current_status = ?1,
                 source_confidence = ?2,
                 confirmed_source_id = ?3,
                 last_checked_at = ?4,
                 updated_at = ?4
             WHERE id = ?5",
            params![
                update_status_label(&status),
                source_confidence,
                confirmed_source_id,
                now,
                mod_id
            ],
        )?;
        Ok(())
    }

    pub fn detect_folder_rename(
        connection: &Connection,
        new_folder_path: &Path,
        new_fingerprint: &str,
    ) -> AppResult<Option<String>> {
        let mut stmt = connection.prepare(
            "SELECT lm.id, lm.local_root_path, lm.display_name
             FROM local_mods lm
             WHERE lm.local_root_path != ?1",
        )?;

        let existing_mods: Vec<(String, String, String)> = stmt
            .query_map([new_folder_path.to_string_lossy().to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (mod_id, old_path, display_name) in existing_mods {
            if let Ok(old_fingerprint) = Self::compute_folder_fingerprint(Path::new(&old_path)) {
                if old_fingerprint == new_fingerprint {
                    tracing::info!(
                        "Detected folder rename for mod {}: {} -> {}",
                        display_name,
                        old_path,
                        new_folder_path.display()
                    );
                    return Ok(Some(mod_id));
                }
            }
        }

        Ok(None)
    }

    pub fn update_local_mod_path(
        connection: &Connection,
        mod_id: &str,
        new_folder_path: &Path,
    ) -> AppResult<()> {
        connection.execute(
            "UPDATE local_mods SET local_root_path = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            rusqlite::params![new_folder_path.to_string_lossy().to_string(), mod_id],
        )?;
        Ok(())
    }

    fn find_local_mod_by_folder(conn: &Connection, folder_path: &str) -> AppResult<Option<String>> {
        conn.query_row(
            "SELECT id FROM local_mods WHERE local_root_path = ?1",
            params![folder_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    fn create_local_mod(
        conn: &Connection,
        display_name: &str,
        folder_path: &str,
    ) -> AppResult<String> {
        if display_name.is_empty() {
            return Err(AppError::Message("display_name cannot be empty".into()));
        }
        if folder_path.is_empty() {
            return Err(AppError::Message("folder_path cannot be empty".into()));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let normalized_name = normalize_mod_name(display_name);

        tracing::debug!(
            "Creating new local mod: {} at {}",
            display_name,
            folder_path
        );

        conn.execute(
            "INSERT INTO local_mods (
                id, display_name, normalized_name, local_root_path,
                tracking_mode, source_confidence, current_status,
                created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                display_name,
                normalized_name,
                folder_path,
                tracking_mode_label(&TrackingMode::DetectedOnly),
                0.0_f64,
                update_status_label(&UpdateStatus::Untracked),
                now,
                now
            ],
        )?;

        Ok(id)
    }

    fn process_file(
        conn: &Connection,
        mod_id: &str,
        file_path: &Path,
    ) -> AppResult<Option<LocalFile>> {
        if mod_id.is_empty() {
            return Err(AppError::Message("mod_id cannot be empty".into()));
        }

        let metadata = file_path.metadata().map_err(|e| {
            tracing::warn!("Failed to read metadata for {:?}: {}", file_path, e);
            AppError::Io(e)
        })?;

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        let file_size = metadata.len() as i64;
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339());

        let sha256 = match Self::compute_file_hash(file_path) {
            Ok(hash) => Some(hash),
            Err(e) => {
                tracing::warn!("Failed to compute hash for {:?}: {}", file_path, e);
                None
            }
        };

        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM local_files WHERE local_mod_id = ?1 AND file_path = ?2",
                params![mod_id, file_path.to_string_lossy().to_string()],
                |row| row.get(0),
            )
            .optional()?;

        let id = if let Some(existing_id) = existing {
            conn.execute(
                "UPDATE local_files
                 SET file_name = ?1, file_ext = ?2, file_size = ?3,
                     sha256 = ?4, modified_at = ?5
                 WHERE id = ?6",
                params![
                    file_name,
                    file_ext,
                    file_size,
                    sha256,
                    modified_at,
                    existing_id
                ],
            )?;
            existing_id
        } else {
            let new_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO local_files (
                    id, local_mod_id, file_path, file_name, file_ext,
                    file_size, sha256, modified_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    new_id,
                    mod_id,
                    file_path.to_string_lossy().to_string(),
                    file_name,
                    file_ext,
                    file_size,
                    sha256,
                    modified_at
                ],
            )?;
            new_id
        };

        Ok(Some(LocalFile {
            id,
            local_mod_id: mod_id.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            file_name,
            file_ext,
            file_size,
            sha256,
            modified_at,
        }))
    }

    fn compute_file_hash(file_path: &Path) -> AppResult<String> {
        use std::io::{BufReader, Read};

        let file = std::fs::File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    pub fn compute_folder_fingerprint(folder_path: &Path) -> AppResult<String> {
        let mut hasher = Sha256::new();
        let mut files: Vec<(String, u64)> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    if let Ok(metadata) = entry.metadata() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let size = metadata.len();
                        files.push((name, size));
                    }
                }
            }
        }

        files.sort();

        for (name, size) in files {
            hasher.update(format!("{}:{}", name, size));
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn compute_file_fingerprints(folder_path: &Path) -> AppResult<HashMap<String, String>> {
        let mut fingerprints = HashMap::new();

        if let Ok(entries) = std::fs::read_dir(folder_path) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    if let Ok(metadata) = entry.metadata() {
                        let fingerprint = format!("{}:{}", name, metadata.len());
                        let hash = format!("{:x}", Sha256::digest(fingerprint.as_bytes()));
                        fingerprints.insert(name, hash);
                    }
                }
            }
        }

        Ok(fingerprints)
    }
}

fn normalize_mod_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect::<String>()
        .trim()
        .to_string()
}

#[expect(clippy::unneeded_struct_pattern)]
fn tracking_mode_label(mode: &TrackingMode) -> &'static str {
    match mode {
        TrackingMode::DetectedOnly => "detected_only",
        TrackingMode::Auto => "auto",
        TrackingMode::Manual => "manual",
        TrackingMode::Ignored => "ignored",
    }
}

fn parse_tracking_mode(value: &str) -> TrackingMode {
    match value {
        "auto" => TrackingMode::Auto,
        "manual" => TrackingMode::Manual,
        "ignored" => TrackingMode::Ignored,
        _ => TrackingMode::DetectedOnly,
    }
}

#[expect(clippy::unneeded_struct_pattern)]
fn update_status_label(status: &UpdateStatus) -> &'static str {
    match status {
        UpdateStatus::Untracked => "untracked",
        UpdateStatus::UpToDate => "up_to_date",
        UpdateStatus::ConfirmedUpdate => "confirmed_update",
        UpdateStatus::ProbableUpdate => "probable_update",
        UpdateStatus::SourceActivity => "source_activity",
        UpdateStatus::SourceUnreachable => "source_unreachable",
        UpdateStatus::NeedsGameUpdate => "needs_game_update",
    }
}

fn parse_update_status(value: &str) -> UpdateStatus {
    match value {
        "up_to_date" => UpdateStatus::UpToDate,
        "confirmed_update" => UpdateStatus::ConfirmedUpdate,
        "probable_update" => UpdateStatus::ProbableUpdate,
        "source_activity" => UpdateStatus::SourceActivity,
        "source_unreachable" => UpdateStatus::SourceUnreachable,
        "needs_game_update" => UpdateStatus::NeedsGameUpdate,
        _ => UpdateStatus::Untracked,
    }
}
