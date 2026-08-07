#![cfg(test)]

//! Hidden fixture-only transaction proof. This module must never become a normal
//! application command or user-file execution path without a separate safety design.

use std::{fs, path::{Path, PathBuf}};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::{
        apply_plan_backup_prototype::{
            canonicalize_existing_dir, ensure_run_matches_plan, ensure_under_root, hash_file,
            resolve_fixture_candidate, run_fixture_backup_prototype, run_fixture_restore_prototype,
            FixtureBackupPrototypeOutcome, FixtureBackupPrototypeRequest,
            FixtureRestorePrototypeOutcome, FixtureRestorePrototypeRequest,
        },
        apply_plan_results, move_engine,
    },
    error::{AppError, AppResult},
    models::{RecordApplyPlanRestoreEntryRequest, RecordApplyPlanResultLogRequest},
};

const MOVE_OPERATION_KIND: &str = "fixture_move_transaction";
const BACKUP_OPERATION_KIND: &str = "fixture_move_backup";
const UNDO_OPERATION_KIND: &str = "fixture_move_undo_restore";
const MOVE_SUCCESS_SUMMARY: &str =
    "Fixture move completed and verified after a verified backup. No user files changed.";

#[derive(Debug, Clone)]
pub(crate) struct FixtureMoveTransactionRequest {
    pub fixture_mode: bool,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: i64,
    pub run_id: i64,
    pub fixture_root: PathBuf,
    pub backup_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FixtureMoveTransactionOutcome {
    Verified(FixtureMoveTransactionSuccess),
    FailedBeforeChange(FixtureMoveTransactionFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureMoveTransactionSuccess {
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub backup_path: PathBuf,
    pub source_size: u64,
    pub destination_size: u64,
    pub source_hash: String,
    pub destination_hash: String,
    pub backup_result_log_id: i64,
    pub backup_restore_entry_id: i64,
    pub move_result_log_id: i64,
    pub move_restore_entry_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureMoveTransactionFailure {
    pub result_log_id: i64,
    pub error_code: String,
    pub error_message: String,
    pub backup_created: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FixtureMoveUndoRequest {
    pub fixture_mode: bool,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: i64,
    pub run_id: i64,
    pub fixture_root: PathBuf,
    pub move_result_log_id: i64,
    pub move_restore_entry_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureMoveUndoSuccess {
    pub restored_source_path: PathBuf,
    pub removed_destination_path: PathBuf,
    pub restored_hash: String,
    pub restore_result_log_id: i64,
    pub restore_entry_id: i64,
}

#[derive(Debug)]
struct FixtureMovePlanItemScope {
    file_id: i64,
    current_path: String,
    destination_path: String,
    action_kind: String,
    blocked: bool,
    review_only: bool,
}

#[derive(Debug)]
struct IndexedSourceScope {
    path: String,
    size: i64,
    hash: Option<String>,
}

pub(crate) fn run_fixture_move_transaction(
    connection: &Connection,
    request: FixtureMoveTransactionRequest,
) -> AppResult<FixtureMoveTransactionOutcome> {
    if !request.fixture_mode {
        return Err(AppError::Message(
            "Fixture move transaction requires fixtureMode=true.".to_owned(),
        ));
    }
    ensure_run_matches_plan(connection, request.run_id, request.apply_plan_id)?;

    let fixture_root = canonicalize_existing_dir(&request.fixture_root, "fixtureRoot")?;
    let backup_root = canonicalize_existing_dir(&request.backup_root, "backupRoot")?;
    ensure_under_root(&backup_root, &fixture_root, "backupRoot")?;

    let item = load_plan_item_scope(connection, request.apply_plan_id, request.apply_plan_item_id)?;
    if item.blocked || item.review_only {
        return Err(AppError::Message(
            "Fixture move transaction requires one unblocked, non-review-only ApplyPlan item."
                .to_owned(),
        ));
    }
    if item.action_kind != "move" {
        return Err(AppError::Message(
            "Fixture move transaction only supports persisted actionKind=move.".to_owned(),
        ));
    }

    let source_path = resolve_fixture_candidate(Path::new(&item.current_path), "currentPath")?;
    let destination_path =
        resolve_fixture_candidate(Path::new(&item.destination_path), "destinationPath")?;
    ensure_under_root(&source_path, &fixture_root, "currentPath")?;
    ensure_under_root(&destination_path, &fixture_root, "destinationPath")?;
    if source_path.starts_with(&backup_root) || destination_path.starts_with(&backup_root) {
        return Err(AppError::Message(
            "Fixture move source and destination must stay outside backupRoot.".to_owned(),
        ));
    }
    if source_path == destination_path {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            None,
            "same_path",
            "Fixture move source and destination resolve to the same path.",
            false,
        );
    }
    if destination_path.exists() {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            None,
            "destination_exists",
            "Fixture move destination already exists; no overwrite was attempted.",
            false,
        );
    }

    let indexed = load_indexed_source_scope(connection, item.file_id)?;
    let indexed_path = resolve_fixture_candidate(Path::new(&indexed.path), "indexed source path")?;
    if indexed_path != source_path {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            None,
            "indexed_source_path_changed",
            "Indexed source path no longer matches the persisted ApplyPlan item.",
            false,
        );
    }

    let source_metadata = match fs::metadata(&source_path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return record_failure(
                connection,
                &request,
                &source_path,
                Some(&destination_path),
                None,
                "source_not_file",
                "Fixture move source is not a file.",
                false,
            );
        }
        Err(_) => {
            return record_failure(
                connection,
                &request,
                &source_path,
                Some(&destination_path),
                None,
                "source_missing",
                "Fixture move source file is missing.",
                false,
            );
        }
    };
    let source_size = source_metadata.len();
    let source_hash = hash_file(&source_path)?;
    let Some(indexed_hash) = indexed.hash.as_deref() else {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            None,
            "source_hash_unavailable",
            "Fixture move requires an indexed source hash before execution.",
            false,
        );
    };
    if indexed.size < 0 || indexed.size as u64 != source_size || indexed_hash != source_hash {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            None,
            "source_changed",
            "Fixture source size or hash changed after the ApplyPlan snapshot.",
            false,
        );
    }

    let backup = run_fixture_backup_prototype(
        connection,
        FixtureBackupPrototypeRequest {
            fixture_mode: true,
            apply_plan_id: request.apply_plan_id,
            apply_plan_item_id: Some(request.apply_plan_item_id),
            run_id: request.run_id,
            fixture_root: fixture_root.clone(),
            source_path: source_path.clone(),
            backup_root: backup_root.clone(),
            operation_kind: BACKUP_OPERATION_KIND.to_owned(),
        },
    )?;
    let FixtureBackupPrototypeOutcome::Verified(backup) = backup else {
        let FixtureBackupPrototypeOutcome::FailedBeforeChange(failure) = backup else {
            unreachable!();
        };
        return Ok(FixtureMoveTransactionOutcome::FailedBeforeChange(
            FixtureMoveTransactionFailure {
                result_log_id: failure.result_log_id,
                error_code: failure.error_code,
                error_message: failure.error_message,
                backup_created: failure.backup_created,
            },
        ));
    };

    let current_source_metadata = fs::metadata(&source_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture source became unavailable after backup verification: {error}"
        ))
    })?;
    let current_source_hash = hash_file(&source_path)?;
    if current_source_metadata.len() != backup.source_size || current_source_hash != backup.source_hash
    {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            Some(&backup.backup_path),
            "source_changed_after_backup",
            "Fixture source changed after its backup was verified; move was not attempted.",
            true,
        );
    }
    if destination_path.exists() {
        return record_failure(
            connection,
            &request,
            &source_path,
            Some(&destination_path),
            Some(&backup.backup_path),
            "destination_appeared_after_backup",
            "Fixture destination appeared after backup verification; move was not attempted.",
            true,
        );
    }

    if let Err(error) = move_engine::move_single_file(&source_path, &destination_path) {
        if source_path.exists() && !destination_path.exists() {
            return record_failure(
                connection,
                &request,
                &source_path,
                Some(&destination_path),
                Some(&backup.backup_path),
                "move_failed_before_change",
                &format!("Fixture move failed before any observed path change: {error}"),
                true,
            );
        }
        return Err(AppError::Message(format!(
            "Fixture move primitive returned an error after path state changed. The isolated fixture must be inspected before retrying: {error}"
        )));
    }

    if source_path.exists() {
        return Err(AppError::Message(
            "Fixture move verification failed because the source still exists after the move."
                .to_owned(),
        ));
    }
    let destination_metadata = fs::metadata(&destination_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture move destination could not be verified after the move: {error}"
        ))
    })?;
    if !destination_metadata.is_file() {
        return Err(AppError::Message(
            "Fixture move destination is not a file after the move.".to_owned(),
        ));
    }
    let destination_size = destination_metadata.len();
    let destination_hash = hash_file(&destination_path)?;
    if destination_size != backup.source_size || destination_hash != backup.source_hash {
        return Err(AppError::Message(
            "Fixture move destination does not match the verified pre-move backup.".to_owned(),
        ));
    }

    let move_result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: Some(request.apply_plan_item_id),
            operation_kind: MOVE_OPERATION_KIND.to_owned(),
            result_status: "pending_log".to_owned(),
            source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
            destination_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
            backup_path: Some(backup.backup_path.to_string_lossy().to_string()),
            error_code: None,
            error_message: None,
            user_summary: MOVE_SUCCESS_SUMMARY.to_owned(),
        },
    )?;
    let move_restore_entry = apply_plan_results::record_apply_plan_restore_entry(
        connection,
        RecordApplyPlanRestoreEntryRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_result_id: Some(move_result.id),
            apply_plan_item_id: Some(request.apply_plan_item_id),
            original_source_path: source_path.to_string_lossy().to_string(),
            destination_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
            backup_path: Some(backup.backup_path.to_string_lossy().to_string()),
            file_hash_before: Some(backup.source_hash.clone()),
            file_size_before: Some(backup.source_size as i64),
            operation_kind: MOVE_OPERATION_KIND.to_owned(),
            operation_result_status: "pending_log".to_owned(),
            restore_status: "design_only".to_owned(),
            restore_error_code: None,
            restore_error_message: None,
        },
    )?;

    Ok(FixtureMoveTransactionOutcome::Verified(
        FixtureMoveTransactionSuccess {
            source_path,
            destination_path,
            backup_path: backup.backup_path,
            source_size: backup.source_size,
            destination_size,
            source_hash: backup.source_hash,
            destination_hash,
            backup_result_log_id: backup.result_log_id,
            backup_restore_entry_id: backup.restore_entry_id,
            move_result_log_id: move_result.id,
            move_restore_entry_id: move_restore_entry.id,
        },
    ))
}

pub(crate) fn run_fixture_move_undo(
    connection: &Connection,
    request: FixtureMoveUndoRequest,
) -> AppResult<FixtureMoveUndoSuccess> {
    if !request.fixture_mode {
        return Err(AppError::Message(
            "Fixture move undo requires fixtureMode=true.".to_owned(),
        ));
    }
    ensure_run_matches_plan(connection, request.run_id, request.apply_plan_id)?;
    let fixture_root = canonicalize_existing_dir(&request.fixture_root, "fixtureRoot")?;

    let detail = apply_plan_results::get_apply_plan_run_log(connection, request.run_id)?
        .ok_or_else(|| AppError::Message("Fixture move run log was not found.".to_owned()))?;
    let move_result = detail
        .results
        .iter()
        .find(|result| result.id == request.move_result_log_id)
        .ok_or_else(|| {
            AppError::Message("Fixture move result is outside the requested run.".to_owned())
        })?;
    if move_result.apply_plan_id != request.apply_plan_id
        || move_result.apply_plan_item_id != Some(request.apply_plan_item_id)
        || move_result.operation_kind != MOVE_OPERATION_KIND
    {
        return Err(AppError::Message(
            "Fixture move result does not match the requested plan/item scope.".to_owned(),
        ));
    }
    let move_restore_entry = detail
        .restore_entries
        .iter()
        .find(|entry| entry.id == request.move_restore_entry_id)
        .ok_or_else(|| {
            AppError::Message("Fixture move restore entry is outside the requested run.".to_owned())
        })?;
    if move_restore_entry.apply_plan_id != request.apply_plan_id
        || move_restore_entry.apply_plan_item_id != Some(request.apply_plan_item_id)
        || move_restore_entry.apply_plan_result_id != Some(request.move_result_log_id)
        || move_restore_entry.operation_kind != MOVE_OPERATION_KIND
    {
        return Err(AppError::Message(
            "Fixture move restore entry does not match the requested move result scope."
                .to_owned(),
        ));
    }

    let original_source_path = resolve_fixture_candidate(
        Path::new(&move_restore_entry.original_source_path),
        "originalSourcePath",
    )?;
    let destination_path = resolve_fixture_candidate(
        Path::new(
            move_restore_entry
                .destination_path_at_execution
                .as_deref()
                .ok_or_else(|| {
                    AppError::Message(
                        "Fixture move restore entry is missing its destination path.".to_owned(),
                    )
                })?,
        ),
        "destinationPathAtExecution",
    )?;
    let backup_path = resolve_fixture_candidate(
        Path::new(move_restore_entry.backup_path.as_deref().ok_or_else(|| {
            AppError::Message("Fixture move restore entry is missing its backup path.".to_owned())
        })?),
        "backupPath",
    )?;
    ensure_under_root(&original_source_path, &fixture_root, "originalSourcePath")?;
    ensure_under_root(&destination_path, &fixture_root, "destinationPathAtExecution")?;
    ensure_under_root(&backup_path, &fixture_root, "backupPath")?;

    if original_source_path.exists() {
        return Err(AppError::Message(
            "Fixture undo target already exists; no overwrite was attempted.".to_owned(),
        ));
    }
    let expected_hash = move_restore_entry.file_hash_before.as_deref().ok_or_else(|| {
        AppError::Message("Fixture move restore entry is missing its pre-move hash.".to_owned())
    })?;
    let expected_size = move_restore_entry.file_size_before.ok_or_else(|| {
        AppError::Message("Fixture move restore entry is missing its pre-move size.".to_owned())
    })?;
    if expected_size < 0 {
        return Err(AppError::Message(
            "Fixture move restore entry contains an invalid pre-move size.".to_owned(),
        ));
    }
    let destination_metadata = fs::metadata(&destination_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture moved destination is unavailable for undo: {error}"
        ))
    })?;
    if !destination_metadata.is_file()
        || destination_metadata.len() != expected_size as u64
        || hash_file(&destination_path)? != expected_hash
    {
        return Err(AppError::Message(
            "Fixture moved destination changed after execution; undo was blocked.".to_owned(),
        ));
    }

    let restore = run_fixture_restore_prototype(
        connection,
        FixtureRestorePrototypeRequest {
            fixture_mode: true,
            fixture_root: fixture_root.clone(),
            backup_path: backup_path.clone(),
            restore_target_path: original_source_path.clone(),
            apply_plan_id: request.apply_plan_id,
            apply_plan_item_id: Some(request.apply_plan_item_id),
            run_id: request.run_id,
            result_id: Some(request.move_result_log_id),
            restore_entry_id: Some(request.move_restore_entry_id),
            operation_kind: UNDO_OPERATION_KIND.to_owned(),
        },
    )?;
    let FixtureRestorePrototypeOutcome::Verified(restore) = restore else {
        let FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) = restore else {
            unreachable!();
        };
        return Err(AppError::Message(format!(
            "Fixture undo restore failed before change: {} ({})",
            failure.error_message, failure.error_code
        )));
    };

    let restored_hash = hash_file(&original_source_path)?;
    if restored_hash != expected_hash || restore.restored_size != expected_size as u64 {
        return Err(AppError::Message(
            "Fixture undo restored source does not match the recorded pre-move state."
                .to_owned(),
        ));
    }
    if hash_file(&destination_path)? != expected_hash {
        return Err(AppError::Message(
            "Fixture destination changed during undo; destination cleanup was blocked."
                .to_owned(),
        ));
    }
    fs::remove_file(&destination_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture source was restored, but the verified moved destination could not be removed: {error}"
        ))
    })?;
    if destination_path.exists() {
        return Err(AppError::Message(
            "Fixture destination still exists after bounded undo cleanup.".to_owned(),
        ));
    }

    Ok(FixtureMoveUndoSuccess {
        restored_source_path: original_source_path,
        removed_destination_path: destination_path,
        restored_hash,
        restore_result_log_id: restore.result_log_id,
        restore_entry_id: restore.restore_entry_id,
    })
}

fn load_plan_item_scope(
    connection: &Connection,
    apply_plan_id: i64,
    apply_plan_item_id: i64,
) -> AppResult<FixtureMovePlanItemScope> {
    connection
        .query_row(
            "SELECT file_id, current_path, destination_path, action_kind, blocked, review_only\n\
             FROM apply_plan_items\n\
             WHERE id = ?1 AND apply_plan_id = ?2",
            params![apply_plan_item_id, apply_plan_id],
            |row| {
                let file_id = row.get::<_, Option<i64>>(0)?.ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        0,
                        "file_id".to_owned(),
                        rusqlite::types::Type::Null,
                    )
                })?;
                let destination_path = row.get::<_, Option<String>>(2)?.ok_or_else(|| {
                    rusqlite::Error::InvalidColumnType(
                        2,
                        "destination_path".to_owned(),
                        rusqlite::types::Type::Null,
                    )
                })?;
                Ok(FixtureMovePlanItemScope {
                    file_id,
                    current_path: row.get(1)?,
                    destination_path,
                    action_kind: row.get(3)?,
                    blocked: row.get::<_, i64>(4)? != 0,
                    review_only: row.get::<_, i64>(5)? != 0,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            AppError::Message(
                "Fixture move requires a persisted ApplyPlan item in the requested plan."
                    .to_owned(),
            )
        })
}

fn load_indexed_source_scope(connection: &Connection, file_id: i64) -> AppResult<IndexedSourceScope> {
    connection
        .query_row(
            "SELECT path, size, hash FROM files WHERE id = ?1",
            params![file_id],
            |row| {
                Ok(IndexedSourceScope {
                    path: row.get(0)?,
                    size: row.get(1)?,
                    hash: row.get(2)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            AppError::Message("Fixture move source file is no longer indexed.".to_owned())
        })
}

fn record_failure(
    connection: &Connection,
    request: &FixtureMoveTransactionRequest,
    source_path: &Path,
    destination_path: Option<&Path>,
    backup_path: Option<&Path>,
    error_code: &str,
    error_message: &str,
    backup_created: bool,
) -> AppResult<FixtureMoveTransactionOutcome> {
    let result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: Some(request.apply_plan_item_id),
            operation_kind: MOVE_OPERATION_KIND.to_owned(),
            result_status: "failed_before_change".to_owned(),
            source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
            destination_path_at_execution: destination_path
                .map(|path| path.to_string_lossy().to_string()),
            backup_path: backup_path.map(|path| path.to_string_lossy().to_string()),
            error_code: Some(error_code.to_owned()),
            error_message: Some(error_message.to_owned()),
            user_summary: format!("{error_message} No user files changed."),
        },
    )?;
    Ok(FixtureMoveTransactionOutcome::FailedBeforeChange(
        FixtureMoveTransactionFailure {
            result_log_id: result.id,
            error_code: error_code.to_owned(),
            error_message: error_message.to_owned(),
            backup_created,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        core::apply_plan_results::{
            create_apply_plan_run_log, list_apply_plan_restore_entries,
            list_apply_plan_result_logs,
        },
        database,
        models::{
            ApplyPlanRestoreEntryStatus, ApplyPlanResultLogStatus, CreateApplyPlanRunLogRequest,
            ListApplyPlanRestoreEntriesRequest, ListApplyPlanResultLogsRequest,
        },
    };
    use sha2::{Digest, Sha256};
    use tempfile::{tempdir, TempDir};

    struct FixtureContext {
        _temp: TempDir,
        fixture_root: PathBuf,
        backup_root: PathBuf,
        source_path: PathBuf,
        destination_path: PathBuf,
        plan_id: i64,
        item_id: i64,
        run_id: i64,
    }

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn bytes_hash(bytes: &[u8]) -> String {
        hex::encode(Sha256::digest(bytes))
    }

    fn setup_fixture(connection: &Connection, source_bytes: &[u8]) -> FixtureContext {
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path().to_path_buf();
        let source_dir = fixture_root.join("source");
        let destination_dir = fixture_root.join("destination");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(&destination_dir).expect("destination dir");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = source_dir.join("sample.package");
        let destination_path = destination_dir.join("sample.package");
        fs::write(&source_path, source_bytes).expect("source fixture");

        connection
            .execute(
                "INSERT INTO files (path, filename, extension, hash, size, kind, confidence,\n\
                    source_location, relative_depth, parser_warnings, insights)\n\
                 VALUES (?1, 'sample.package', '.package', ?2, ?3, 'Gameplay', 1.0,\n\
                    'downloads', 0, '[]', '{}')",
                params![
                    source_path.to_string_lossy().to_string(),
                    bytes_hash(source_bytes),
                    source_bytes.len() as i64,
                ],
            )
            .expect("insert indexed source");
        let file_id = connection.last_insert_rowid();

        connection
            .execute(
                "INSERT INTO apply_plans (\n\
                    source_plan_kind, title, summary, status, would_touch_files,\n\
                    confirmation_required, backup_required, restore_available, total_items,\n\
                    applyable_items, blocked_items, review_only_items, caveats_json,\n\
                    created_at, updated_at\n\
                 ) VALUES ('fixture_test', 'Fixture move plan', 'Fixture-only transaction proof',\n\
                    'preview_only_source', 0, 1, 1, 0, 1, 1, 0, 0, '[]',\n\
                    '2026-01-01', '2026-01-01')",
                [],
            )
            .expect("insert plan");
        let plan_id = connection.last_insert_rowid();

        connection
            .execute(
                "INSERT INTO apply_plan_items (\n\
                    apply_plan_id, file_id, file_name, current_path, current_root,\n\
                    destination_path, destination_root, action_kind, evidence_level, bucket,\n\
                    confidence_label, item_status, blocked, review_only, path_privacy_level,\n\
                    created_at, updated_at\n\
                 ) VALUES (?1, ?2, 'sample.package', ?3, 'fixture_source', ?4,\n\
                    'fixture_destination', 'move', 'strong', 'fixture', 'high',\n\
                    'draft_candidate', 0, 0, 'local_full_path_required',\n\
                    '2026-01-01', '2026-01-01')",
                params![
                    plan_id,
                    file_id,
                    source_path.to_string_lossy().to_string(),
                    destination_path.to_string_lossy().to_string(),
                ],
            )
            .expect("insert plan item");
        let item_id = connection.last_insert_rowid();

        let run_id = create_apply_plan_run_log(
            connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: None,
                skipped_items: None,
                failed_items: None,
                summary: Some("Fixture move transaction proof.".to_owned()),
            },
        )
        .expect("create run")
        .id;

        FixtureContext {
            _temp: temp,
            fixture_root,
            backup_root,
            source_path,
            destination_path,
            plan_id,
            item_id,
            run_id,
        }
    }

    fn move_request(context: &FixtureContext) -> FixtureMoveTransactionRequest {
        FixtureMoveTransactionRequest {
            fixture_mode: true,
            apply_plan_id: context.plan_id,
            apply_plan_item_id: context.item_id,
            run_id: context.run_id,
            fixture_root: context.fixture_root.clone(),
            backup_root: context.backup_root.clone(),
        }
    }

    #[test]
    fn fixture_move_transaction_backs_up_moves_verifies_logs_and_undoes_exact_file_state() {
        let connection = memory_connection();
        let source_bytes = b"fixture transaction package bytes";
        let context = setup_fixture(&connection, source_bytes);

        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("fixture move transaction");
        let FixtureMoveTransactionOutcome::Verified(success) = outcome else {
            panic!("expected verified fixture move");
        };

        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());
        assert!(success.backup_path.exists());
        assert_eq!(fs::read(&context.destination_path).expect("destination"), source_bytes);
        assert_eq!(fs::read(&success.backup_path).expect("backup"), source_bytes);
        assert_eq!(success.source_hash, success.destination_hash);
        assert_eq!(success.source_size, success.destination_size);

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("result logs");
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|result| result.result_status == ApplyPlanResultLogStatus::PendingLog));
        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| {
            entry.restore_status == ApplyPlanRestoreEntryStatus::DesignOnly
                && entry.operation_result_status == ApplyPlanResultLogStatus::PendingLog
        }));

        let undo = run_fixture_move_undo(
            &connection,
            FixtureMoveUndoRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: context.item_id,
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                move_result_log_id: success.move_result_log_id,
                move_restore_entry_id: success.move_restore_entry_id,
            },
        )
        .expect("fixture undo");

        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
        assert_eq!(fs::read(&context.source_path).expect("restored source"), source_bytes);
        assert_eq!(undo.restored_hash, bytes_hash(source_bytes));
        assert!(success.backup_path.exists(), "verified backup is intentionally retained");

        let results_after = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("result logs after undo");
        assert_eq!(results_after.len(), 3);
        assert!(results_after
            .iter()
            .all(|result| result.result_status == ApplyPlanResultLogStatus::PendingLog));
    }

    #[test]
    fn destination_collision_blocks_before_backup_or_move() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"source bytes");
        fs::write(&context.destination_path, b"existing destination").expect("destination");

        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("collision outcome");
        let FixtureMoveTransactionOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "destination_exists");
        assert!(!failure.backup_created);
        assert!(context.source_path.exists());
        assert_eq!(
            fs::read(&context.destination_path).expect("destination"),
            b"existing destination"
        );
        assert!(fs::read_dir(&context.backup_root)
            .expect("backup root")
            .next()
            .is_none());
    }

    #[test]
    fn missing_source_blocks_before_backup_or_move() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"source bytes");
        fs::remove_file(&context.source_path).expect("remove source fixture");

        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("missing source outcome");
        let FixtureMoveTransactionOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "source_missing");
        assert!(!failure.backup_created);
        assert!(!context.destination_path.exists());
    }

    #[test]
    fn changed_source_hash_blocks_before_backup_or_move() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"original bytes");
        fs::write(&context.source_path, b"changed bytes with a different hash")
            .expect("change source fixture");

        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("changed source outcome");
        let FixtureMoveTransactionOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "source_changed");
        assert!(!failure.backup_created);
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
    }

    #[test]
    fn fixture_move_transaction_requires_fixture_mode() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"fixture mode bytes");
        let mut request = move_request(&context);
        request.fixture_mode = false;

        let error = run_fixture_move_transaction(&connection, request)
            .expect_err("fixture mode gate rejected");
        assert!(error.to_string().contains("fixtureMode=true"));
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
        assert!(fs::read_dir(&context.backup_root)
            .expect("backup root")
            .next()
            .is_none());
    }

    #[test]
    fn destination_outside_fixture_root_is_rejected_before_backup_or_move() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"bounded fixture bytes");
        let outside = tempdir().expect("outside tempdir");
        let outside_destination = outside.path().join("outside.package");
        connection
            .execute(
                "UPDATE apply_plan_items SET destination_path = ?1 WHERE id = ?2",
                params![
                    outside_destination.to_string_lossy().to_string(),
                    context.item_id,
                ],
            )
            .expect("set outside destination");

        let error = run_fixture_move_transaction(&connection, move_request(&context))
            .expect_err("outside fixture destination rejected");
        assert!(error.to_string().contains("supplied fixture boundary"));
        assert!(context.source_path.exists());
        assert!(!outside_destination.exists());
        assert!(fs::read_dir(&context.backup_root)
            .expect("backup root")
            .next()
            .is_none());
    }

    #[test]
    fn undo_rejects_restore_scope_mismatch_without_changing_moved_file() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"scope bytes");
        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("fixture move transaction");
        let FixtureMoveTransactionOutcome::Verified(success) = outcome else {
            panic!("expected verified fixture move");
        };

        let error = run_fixture_move_undo(
            &connection,
            FixtureMoveUndoRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: context.item_id,
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                move_result_log_id: success.backup_result_log_id,
                move_restore_entry_id: success.move_restore_entry_id,
            },
        )
        .expect_err("mismatched result scope rejected");
        assert!(error.to_string().contains("Fixture move result"));
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());

        run_fixture_move_undo(
            &connection,
            FixtureMoveUndoRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: context.item_id,
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                move_result_log_id: success.move_result_log_id,
                move_restore_entry_id: success.move_restore_entry_id,
            },
        )
        .expect("correctly scoped undo");
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
    }

    #[test]
    fn fixture_transaction_is_not_registered_or_compiled_as_a_normal_command() {
        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("run_fixture_move_transaction"));
        assert!(!commands_source.contains("run_fixture_move_undo"));
        assert!(!commands_source.contains("fixture_move_transaction"));

        let core_source = include_str!("mod.rs");
        assert!(core_source.contains("#[cfg(test)]\npub mod apply_plan_fixture_transaction_prototype;"));
        let transaction_source = include_str!("apply_plan_fixture_transaction_prototype.rs");
        assert!(transaction_source.starts_with("#![cfg(test)]"));
    }
}
