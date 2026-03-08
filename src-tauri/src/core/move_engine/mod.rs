use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::Digest;

use crate::{
    core::{rule_engine, snapshot_manager},
    database,
    error::{AppError, AppResult},
    models::{ApplyPreviewResult, LibrarySettings, RestoreSnapshotResult},
};

pub fn apply_preview_moves(
    connection: &mut Connection,
    settings: &LibrarySettings,
    preset_name: Option<String>,
    limit: i64,
    approved: bool,
) -> AppResult<ApplyPreviewResult> {
    if !approved {
        return Err(AppError::Message(
            "Apply was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    let preview = rule_engine::build_preview(connection, settings, preset_name.clone(), limit)?;
    let actionable = preview
        .suggestions
        .into_iter()
        .filter(|item| !item.review_required)
        .filter_map(|item| {
            item.final_absolute_path
                .clone()
                .filter(|destination| destination != &item.current_path)
                .map(|destination| PlannedMove {
                    file_id: item.file_id,
                    current_path: PathBuf::from(item.current_path),
                    final_path: PathBuf::from(destination),
                    kind: item.kind,
                })
        })
        .collect::<Vec<_>>();

    if actionable.is_empty() {
        return Err(AppError::Message(
            "No safe preview moves are ready to apply.".to_owned(),
        ));
    }

    preflight_moves(&actionable)?;

    let snapshot_name = format!(
        "{} Preview {}",
        preset_name.unwrap_or_else(|| "Category First".to_owned()),
        Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    let snapshot = snapshot_manager::create_snapshot(
        connection,
        &snapshot_name,
        Some("Auto-created before approved preview batch"),
        &actionable
            .iter()
            .map(|item| {
                Ok(snapshot_manager::SnapshotItemRecord {
                    file_id: item.file_id,
                    original_path: item.current_path.to_string_lossy().to_string(),
                    original_hash: Some(file_hash(&item.current_path)?),
                })
            })
            .collect::<AppResult<Vec<_>>>()?,
    )?;

    let mut applied_moves = Vec::new();
    for item in &actionable {
        if let Err(error) = move_single_file(&item.current_path, &item.final_path) {
            rollback_applied_moves(connection, settings, &applied_moves)?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }

        applied_moves.push(item.clone());
        if let Err(error) = update_file_record_after_move(connection, settings, item) {
            rollback_applied_moves(connection, settings, &applied_moves)?;
            delete_snapshot(connection, snapshot.id)?;
            return Err(error);
        }
    }

    let moved_count = applied_moves.len() as i64;
    Ok(ApplyPreviewResult {
        snapshot_id: snapshot.id,
        moved_count,
        deferred_review_count: preview.review_count,
        skipped_count: preview.total_considered - moved_count - preview.review_count,
        snapshot_name: snapshot.snapshot_name,
    })
}

pub fn restore_snapshot(
    connection: &mut Connection,
    snapshot_id: i64,
    approved: bool,
) -> AppResult<RestoreSnapshotResult> {
    if !approved {
        return Err(AppError::Message(
            "Rollback was blocked because approval was not confirmed.".to_owned(),
        ));
    }

    rollback_snapshot_internal(connection, snapshot_id)
}

fn rollback_snapshot_internal(
    connection: &mut Connection,
    snapshot_id: i64,
) -> AppResult<RestoreSnapshotResult> {
    let settings = database::get_library_settings(connection)?;
    let mut statement = connection.prepare(
        "SELECT file_id, original_path, original_hash
         FROM snapshot_items
         WHERE snapshot_id = ?1
         ORDER BY id DESC",
    )?;
    let items = statement
        .query_map(params![snapshot_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);

    let mut restored_count = 0_i64;
    let mut skipped_count = 0_i64;

    for (file_id, original_path, original_hash) in items {
        let current_path: Option<String> = connection
            .query_row(
                "SELECT path FROM files WHERE id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .optional()?;

        let Some(current_path) = current_path else {
            skipped_count += 1;
            continue;
        };

        if current_path == original_path {
            skipped_count += 1;
            continue;
        }

        let current = PathBuf::from(&current_path);
        let original = PathBuf::from(&original_path);

        if !current.exists() {
            skipped_count += 1;
            continue;
        }

        if let Some(expected_hash) = original_hash.as_deref() {
            let current_hash = file_hash(&current)?;
            if current_hash != expected_hash {
                return Err(AppError::Message(format!(
                    "Rollback blocked because file contents changed for {}",
                    current.display()
                )));
            }
        }

        if original.exists() && original != current {
            return Err(AppError::Message(format!(
                "Rollback blocked because original destination already exists: {}",
                original.display()
            )));
        }

        move_single_file(&current, &original)?;
        update_file_record_on_restore(connection, &settings, file_id, &original)?;
        restored_count += 1;
    }

    Ok(RestoreSnapshotResult {
        snapshot_id,
        restored_count,
        skipped_count,
    })
}

fn rollback_applied_moves(
    connection: &Connection,
    settings: &LibrarySettings,
    applied_moves: &[PlannedMove],
) -> AppResult<()> {
    for item in applied_moves.iter().rev() {
        if item.final_path.exists() && !item.current_path.exists() {
            move_single_file(&item.final_path, &item.current_path)?;
        }

        update_file_record_on_restore(connection, settings, item.file_id, &item.current_path)?;
    }

    Ok(())
}

fn preflight_moves(moves: &[PlannedMove]) -> AppResult<()> {
    for item in moves {
        if !item.current_path.exists() {
            return Err(AppError::Message(format!(
                "Move blocked because source file is missing: {}",
                item.current_path.display()
            )));
        }

        if item.final_path.exists() {
            return Err(AppError::Message(format!(
                "Move blocked because destination already exists: {}",
                item.final_path.display()
            )));
        }
    }

    Ok(())
}

fn update_file_record_after_move(
    connection: &Connection,
    settings: &LibrarySettings,
    item: &PlannedMove,
) -> AppResult<()> {
    let from_path = item.current_path.to_string_lossy().to_string();
    let to_path = item.final_path.to_string_lossy().to_string();
    connection.execute(
        "UPDATE files
         SET path = ?1,
             source_location = ?2,
             relative_depth = ?3,
             indexed_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![
            &to_path,
            target_source_location(item),
            relative_depth(settings, &item.final_path, &item.kind),
            item.file_id
        ],
    )?;
    database::sync_category_override_path(connection, &from_path, &to_path)?;

    Ok(())
}

fn update_file_record_on_restore(
    connection: &Connection,
    settings: &LibrarySettings,
    file_id: i64,
    restored_path: &Path,
) -> AppResult<()> {
    let previous_path: String = connection.query_row(
        "SELECT path FROM files WHERE id = ?1",
        params![file_id],
        |row| row.get(0),
    )?;
    let source_location = if is_tray_path(restored_path) {
        "tray"
    } else {
        "mods"
    };
    let kind: String = connection.query_row(
        "SELECT kind FROM files WHERE id = ?1",
        params![file_id],
        |row| row.get(0),
    )?;

    connection.execute(
        "UPDATE files
         SET path = ?1,
             source_location = ?2,
             relative_depth = ?3,
             indexed_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![
            restored_path.to_string_lossy().to_string(),
            source_location,
            relative_depth(&settings, restored_path, &kind),
            file_id
        ],
    )?;
    database::sync_category_override_path(
        connection,
        &previous_path,
        &restored_path.to_string_lossy(),
    )?;

    Ok(())
}

fn delete_snapshot(connection: &Connection, snapshot_id: i64) -> AppResult<()> {
    connection.execute("DELETE FROM snapshots WHERE id = ?1", params![snapshot_id])?;
    Ok(())
}

fn move_single_file(source: &Path, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(source, destination)?;
            fs::remove_file(source)?;
            Ok(())
        }
    }
}

fn file_hash(path: &Path) -> AppResult<String> {
    let bytes = fs::read(path)?;
    Ok(hex::encode(sha2::Sha256::digest(bytes)))
}

fn relative_depth(settings: &LibrarySettings, path: &Path, kind: &str) -> i64 {
    let root = if kind.starts_with("Tray") || is_tray_path(path) {
        settings.tray_path.as_deref()
    } else {
        settings.mods_path.as_deref()
    };

    root.and_then(|root| {
        path.strip_prefix(root).ok().and_then(|relative| {
            relative
                .parent()
                .map(|parent| parent.components().count() as i64)
        })
    })
    .unwrap_or_default()
}

fn target_source_location(item: &PlannedMove) -> &'static str {
    if item.kind.starts_with("Tray") || is_tray_path(&item.final_path) {
        "tray"
    } else {
        "mods"
    }
}

fn is_tray_path(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("Tray")
    }) || path
        .to_string_lossy()
        .to_ascii_lowercase()
        .contains("\\tray\\")
        || path
            .to_string_lossy()
            .to_ascii_lowercase()
            .contains("/tray/")
}

#[derive(Debug, Clone)]
struct PlannedMove {
    file_id: i64,
    current_path: PathBuf,
    final_path: PathBuf,
    kind: String,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::params;
    use tempfile::tempdir;

    use crate::{database, models::LibrarySettings, seed::load_seed_pack};

    use super::{apply_preview_moves, restore_snapshot};

    #[test]
    fn apply_and_rollback_preview_moves_are_reversible() {
        let temp = tempdir().expect("tempdir");
        let mods = temp.path().join("Mods");
        let tray = temp.path().join("Tray");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&tray).expect("tray");

        let source_file = mods.join("messy_hair.package");
        fs::write(&source_file, b"hair-data").expect("source file");

        let mut connection = rusqlite::Connection::open_in_memory().expect("in-memory db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");
        database::save_library_paths(
            &mut connection,
            &LibrarySettings {
                mods_path: Some(mods.to_string_lossy().to_string()),
                tray_path: Some(tray.to_string_lossy().to_string()),
            },
        )
        .expect("settings");

        let creator_id: i64 = connection
            .query_row(
                "SELECT id FROM creators WHERE canonical_name = ?1",
                params!["Simstrouble"],
                |row| row.get(0),
            )
            .expect("creator");
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, subtype, confidence, source_location, creator_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    source_file.to_string_lossy().to_string(),
                    "messy_hair.package",
                    ".package",
                    "CAS",
                    "Hair",
                    0.92_f64,
                    "mods",
                    creator_id
                ],
            )
            .expect("file");

        let apply_result = apply_preview_moves(
            &mut connection,
            &LibrarySettings {
                mods_path: Some(mods.to_string_lossy().to_string()),
                tray_path: Some(tray.to_string_lossy().to_string()),
            },
            Some("Category First".to_owned()),
            20,
            true,
        )
        .expect("apply");

        assert_eq!(apply_result.moved_count, 1);
        let moved_path = mods
            .join("CAS")
            .join("Hair")
            .join("Simstrouble")
            .join("messy_hair.package");
        assert!(moved_path.exists());
        assert!(!source_file.exists());

        let rollback_result =
            restore_snapshot(&mut connection, apply_result.snapshot_id, true).expect("rollback");
        assert_eq!(rollback_result.restored_count, 1);
        assert!(source_file.exists());
        assert!(!moved_path.exists());
    }
}
