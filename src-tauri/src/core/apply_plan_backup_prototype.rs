#![cfg_attr(not(test), allow(dead_code))]

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::{
    core::apply_plan_results,
    error::{AppError, AppResult},
    models::{
        PersistedApplyPlanRestoreEntry, RecordApplyPlanRestoreEntryRequest,
        RecordApplyPlanResultLogRequest,
    },
};

const SUCCESS_SUMMARY: &str = "Fixture backup copied and verified. No user files changed.";
const RESTORE_SUCCESS_SUMMARY: &str =
    "Fixture restore copied and verified from backup. No user files changed.";

#[derive(Debug, Clone)]
pub(crate) struct FixtureBackupPrototypeRequest {
    pub fixture_mode: bool,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: Option<i64>,
    pub run_id: i64,
    pub fixture_root: PathBuf,
    pub source_path: PathBuf,
    pub backup_root: PathBuf,
    pub operation_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FixtureBackupPrototypeOutcome {
    Verified(FixtureBackupPrototypeSuccess),
    FailedBeforeChange(FixtureBackupPrototypeFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureBackupPrototypeSuccess {
    pub backup_path: PathBuf,
    pub source_size: u64,
    pub backup_size: u64,
    pub source_hash: String,
    pub backup_hash: String,
    pub backup_verified: bool,
    pub result_log_id: i64,
    pub restore_entry_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureBackupPrototypeFailure {
    pub result_log_id: i64,
    pub error_code: String,
    pub error_message: String,
    pub backup_created: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FixtureRestorePrototypeRequest {
    pub fixture_mode: bool,
    pub fixture_root: PathBuf,
    pub backup_path: PathBuf,
    pub restore_target_path: PathBuf,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: Option<i64>,
    pub run_id: i64,
    pub result_id: Option<i64>,
    pub restore_entry_id: Option<i64>,
    pub operation_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FixtureRestorePrototypeOutcome {
    Verified(FixtureRestorePrototypeSuccess),
    FailedBeforeChange(FixtureRestorePrototypeFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureRestorePrototypeSuccess {
    pub restored_path: PathBuf,
    pub backup_size: u64,
    pub restored_size: u64,
    pub backup_hash: String,
    pub restored_hash: String,
    pub restore_verified: bool,
    pub result_log_id: i64,
    pub source_restore_entry_id: i64,
    pub restore_entry_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureRestorePrototypeFailure {
    pub result_log_id: i64,
    pub error_code: String,
    pub error_message: String,
    pub restore_created: bool,
}

pub(crate) fn run_fixture_backup_prototype(
    connection: &Connection,
    request: FixtureBackupPrototypeRequest,
) -> AppResult<FixtureBackupPrototypeOutcome> {
    if !request.fixture_mode {
        return Err(AppError::Message(
            "Fixture backup prototype requires fixtureMode=true.".to_owned(),
        ));
    }
    let operation_kind = require_trimmed(&request.operation_kind, "operationKind")?;
    ensure_run_matches_plan(connection, request.run_id, request.apply_plan_id)?;

    let fixture_root = canonicalize_existing_dir(&request.fixture_root, "fixtureRoot")?;
    let backup_root = canonicalize_existing_dir(&request.backup_root, "backupRoot")?;
    ensure_under_root(&backup_root, &fixture_root, "backupRoot")?;

    let source_path = resolve_fixture_candidate(&request.source_path, "sourcePath")?;
    ensure_under_root(&source_path, &fixture_root, "sourcePath")?;
    if source_path.starts_with(&backup_root) {
        return Err(AppError::Message(
            "sourcePath must not be inside backupRoot.".to_owned(),
        ));
    }

    let file_name = source_path.file_name().ok_or_else(|| {
        AppError::Message("sourcePath must point to a named fixture file.".to_owned())
    })?;
    let item_segment = request
        .apply_plan_item_id
        .map(|id| format!("item-{id}"))
        .unwrap_or_else(|| "item-no-item".to_owned());
    let backup_dir = backup_root
        .join(format!("run-{}", request.run_id))
        .join(item_segment);
    let backup_path = backup_dir.join(file_name);
    ensure_under_root(&backup_dir, &backup_root, "backup destination")?;
    ensure_under_root(&backup_path, &backup_root, "backup destination")?;

    let source_metadata = match fs::metadata(&source_path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return record_failed_before_change(
                connection,
                &request,
                &operation_kind,
                &source_path,
                "source_not_file",
                "Fixture source path is not a file.",
            );
        }
        Err(error) => {
            return record_failed_before_change(
                connection,
                &request,
                &operation_kind,
                &source_path,
                "source_missing",
                &format!("Fixture source file could not be read: {error}"),
            );
        }
    };

    if backup_path.exists() {
        return record_failed_before_change(
            connection,
            &request,
            &operation_kind,
            &source_path,
            "backup_destination_exists",
            "Fixture backup destination already exists; no overwrite was attempted.",
        );
    }

    let source_hash = hash_file(&source_path)?;
    fs::create_dir_all(&backup_dir).map_err(|error| {
        AppError::Message(format!(
            "Fixture backup directory could not be created: {error}"
        ))
    })?;
    fs::copy(&source_path, &backup_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture backup copy failed before source changes: {error}"
        ))
    })?;

    let backup_metadata = fs::metadata(&backup_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture backup file could not be verified: {error}"
        ))
    })?;
    let backup_hash = hash_file(&backup_path)?;
    let source_size = source_metadata.len();
    let backup_size = backup_metadata.len();
    if source_size != backup_size || source_hash != backup_hash {
        return Err(AppError::Message(
            "Fixture backup verification failed after copying a temporary test file.".to_owned(),
        ));
    }

    let result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: request.apply_plan_item_id,
            operation_kind: operation_kind.clone(),
            result_status: "pending_log".to_owned(),
            source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
            destination_path_at_execution: None,
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            error_code: None,
            error_message: None,
            user_summary: SUCCESS_SUMMARY.to_owned(),
        },
    )?;
    let restore_entry = apply_plan_results::record_apply_plan_restore_entry(
        connection,
        RecordApplyPlanRestoreEntryRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_result_id: Some(result.id),
            apply_plan_item_id: request.apply_plan_item_id,
            original_source_path: source_path.to_string_lossy().to_string(),
            destination_path_at_execution: None,
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            file_hash_before: Some(source_hash.clone()),
            file_size_before: Some(source_size as i64),
            operation_kind,
            operation_result_status: "pending_log".to_owned(),
            restore_status: "design_only".to_owned(),
            restore_error_code: None,
            restore_error_message: None,
        },
    )?;

    Ok(FixtureBackupPrototypeOutcome::Verified(
        FixtureBackupPrototypeSuccess {
            backup_path,
            source_size,
            backup_size,
            source_hash,
            backup_hash,
            backup_verified: true,
            result_log_id: result.id,
            restore_entry_id: restore_entry.id,
        },
    ))
}

pub(crate) fn run_fixture_restore_prototype(
    connection: &Connection,
    request: FixtureRestorePrototypeRequest,
) -> AppResult<FixtureRestorePrototypeOutcome> {
    if !request.fixture_mode {
        return Err(AppError::Message(
            "Fixture restore prototype requires fixtureMode=true.".to_owned(),
        ));
    }
    let operation_kind = require_trimmed(&request.operation_kind, "operationKind")?;
    let source_restore_entry_id = request.restore_entry_id.ok_or_else(|| {
        AppError::Message(
            "Fixture restore prototype requires an existing restoreEntryId.".to_owned(),
        )
    })?;
    let source_restore_entry =
        load_scoped_restore_entry(connection, &request, source_restore_entry_id)?;

    let fixture_root = canonicalize_existing_dir(&request.fixture_root, "fixtureRoot")?;
    let backup_path = resolve_fixture_candidate(&request.backup_path, "backupPath")?;
    ensure_under_root(&backup_path, &fixture_root, "backupPath")?;

    let recorded_backup_path = source_restore_entry
        .backup_path
        .as_deref()
        .ok_or_else(|| {
            AppError::Message("Referenced restore entry does not contain a backupPath.".to_owned())
        })
        .and_then(|path| resolve_fixture_candidate(Path::new(path), "recorded backupPath"))?;
    ensure_under_root(&recorded_backup_path, &fixture_root, "recorded backupPath")?;
    if recorded_backup_path != backup_path {
        return Err(AppError::Message(
            "Fixture restore backupPath must match the referenced restore entry.".to_owned(),
        ));
    }

    let restore_target_path =
        resolve_fixture_candidate(&request.restore_target_path, "restoreTargetPath")?;
    ensure_under_root(&restore_target_path, &fixture_root, "restoreTargetPath")?;

    let backup_metadata = match fs::metadata(&backup_path) {
        Ok(metadata) if metadata.is_file() => metadata,
        Ok(_) => {
            return record_restore_failed_before_change(
                connection,
                &request,
                &operation_kind,
                &backup_path,
                &restore_target_path,
                "backup_not_file",
                "Fixture backup path is not a file.",
            );
        }
        Err(error) => {
            return record_restore_failed_before_change(
                connection,
                &request,
                &operation_kind,
                &backup_path,
                &restore_target_path,
                "backup_missing",
                &format!("Fixture backup file could not be read: {error}"),
            );
        }
    };

    if restore_target_path.exists() {
        return record_restore_failed_before_change(
            connection,
            &request,
            &operation_kind,
            &backup_path,
            &restore_target_path,
            "restore_target_exists",
            "Fixture restore target already exists; no overwrite was attempted.",
        );
    }

    let backup_size = backup_metadata.len();
    let backup_hash = hash_file(&backup_path)?;
    if let Some(expected_size) = source_restore_entry.file_size_before {
        if expected_size < 0 || expected_size as u64 != backup_size {
            return record_restore_failed_before_change(
                connection,
                &request,
                &operation_kind,
                &backup_path,
                &restore_target_path,
                "backup_size_mismatch",
                "Fixture backup size does not match the referenced restore entry.",
            );
        }
    }
    if let Some(expected_hash) = source_restore_entry.file_hash_before.as_deref() {
        if expected_hash != backup_hash {
            return record_restore_failed_before_change(
                connection,
                &request,
                &operation_kind,
                &backup_path,
                &restore_target_path,
                "backup_hash_mismatch",
                "Fixture backup hash does not match the referenced restore entry.",
            );
        }
    }

    fs::copy(&backup_path, &restore_target_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture restore copy failed before user-file changes: {error}"
        ))
    })?;

    let restored_metadata = fs::metadata(&restore_target_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture restored file could not be verified: {error}"
        ))
    })?;
    let restored_size = restored_metadata.len();
    let restored_hash = hash_file(&restore_target_path)?;
    if backup_size != restored_size || backup_hash != restored_hash {
        return Err(AppError::Message(
            "Fixture restore verification failed after copying a temporary test file.".to_owned(),
        ));
    }

    let result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: request.apply_plan_item_id,
            operation_kind: operation_kind.clone(),
            result_status: "pending_log".to_owned(),
            source_path_at_execution: Some(backup_path.to_string_lossy().to_string()),
            destination_path_at_execution: Some(restore_target_path.to_string_lossy().to_string()),
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            error_code: None,
            error_message: None,
            user_summary: RESTORE_SUCCESS_SUMMARY.to_owned(),
        },
    )?;
    let restore_entry = apply_plan_results::record_apply_plan_restore_entry(
        connection,
        RecordApplyPlanRestoreEntryRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_result_id: Some(result.id),
            apply_plan_item_id: request.apply_plan_item_id,
            original_source_path: source_restore_entry.original_source_path,
            destination_path_at_execution: Some(restore_target_path.to_string_lossy().to_string()),
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            file_hash_before: Some(backup_hash.clone()),
            file_size_before: Some(backup_size as i64),
            operation_kind,
            operation_result_status: "pending_log".to_owned(),
            restore_status: "design_only".to_owned(),
            restore_error_code: None,
            restore_error_message: None,
        },
    )?;

    Ok(FixtureRestorePrototypeOutcome::Verified(
        FixtureRestorePrototypeSuccess {
            restored_path: restore_target_path,
            backup_size,
            restored_size,
            backup_hash,
            restored_hash,
            restore_verified: true,
            result_log_id: result.id,
            source_restore_entry_id,
            restore_entry_id: restore_entry.id,
        },
    ))
}

fn record_failed_before_change(
    connection: &Connection,
    request: &FixtureBackupPrototypeRequest,
    operation_kind: &str,
    source_path: &Path,
    error_code: &str,
    error_message: &str,
) -> AppResult<FixtureBackupPrototypeOutcome> {
    let result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: request.apply_plan_item_id,
            operation_kind: operation_kind.to_owned(),
            result_status: "failed_before_change".to_owned(),
            source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
            destination_path_at_execution: None,
            backup_path: None,
            error_code: Some(error_code.to_owned()),
            error_message: Some(error_message.to_owned()),
            user_summary: format!("{error_message} No user files changed."),
        },
    )?;

    Ok(FixtureBackupPrototypeOutcome::FailedBeforeChange(
        FixtureBackupPrototypeFailure {
            result_log_id: result.id,
            error_code: error_code.to_owned(),
            error_message: error_message.to_owned(),
            backup_created: false,
        },
    ))
}

fn record_restore_failed_before_change(
    connection: &Connection,
    request: &FixtureRestorePrototypeRequest,
    operation_kind: &str,
    backup_path: &Path,
    restore_target_path: &Path,
    error_code: &str,
    error_message: &str,
) -> AppResult<FixtureRestorePrototypeOutcome> {
    let result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: request.apply_plan_item_id,
            operation_kind: operation_kind.to_owned(),
            result_status: "failed_before_change".to_owned(),
            source_path_at_execution: Some(backup_path.to_string_lossy().to_string()),
            destination_path_at_execution: Some(restore_target_path.to_string_lossy().to_string()),
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            error_code: Some(error_code.to_owned()),
            error_message: Some(error_message.to_owned()),
            user_summary: format!("{error_message} No user files changed."),
        },
    )?;

    Ok(FixtureRestorePrototypeOutcome::FailedBeforeChange(
        FixtureRestorePrototypeFailure {
            result_log_id: result.id,
            error_code: error_code.to_owned(),
            error_message: error_message.to_owned(),
            restore_created: false,
        },
    ))
}

pub(super) fn ensure_run_matches_plan(
    connection: &Connection,
    run_id: i64,
    apply_plan_id: i64,
) -> AppResult<()> {
    let detail = apply_plan_results::get_apply_plan_run_log(connection, run_id)?
        .ok_or_else(|| AppError::Message(format!("ApplyPlan run log {run_id} was not found.")))?;
    if detail.run.apply_plan_id != apply_plan_id {
        return Err(AppError::Message(
            "Fixture backup run does not belong to the requested ApplyPlan.".to_owned(),
        ));
    }
    Ok(())
}

fn load_scoped_restore_entry(
    connection: &Connection,
    request: &FixtureRestorePrototypeRequest,
    restore_entry_id: i64,
) -> AppResult<PersistedApplyPlanRestoreEntry> {
    let detail = apply_plan_results::get_apply_plan_run_log(connection, request.run_id)?
        .ok_or_else(|| {
            AppError::Message(format!(
                "ApplyPlan run log {} was not found.",
                request.run_id
            ))
        })?;
    if detail.run.apply_plan_id != request.apply_plan_id {
        return Err(AppError::Message(
            "Fixture restore run does not belong to the requested ApplyPlan.".to_owned(),
        ));
    }

    let restore_entry = detail
        .restore_entries
        .iter()
        .find(|entry| entry.id == restore_entry_id)
        .cloned()
        .ok_or_else(|| {
            AppError::Message(format!(
                "ApplyPlan restore entry {restore_entry_id} was not found for this run."
            ))
        })?;
    if restore_entry.apply_plan_id != request.apply_plan_id
        || restore_entry.apply_plan_run_id != request.run_id
    {
        return Err(AppError::Message(
            "Referenced restore entry is outside the requested ApplyPlan run.".to_owned(),
        ));
    }
    if restore_entry.apply_plan_item_id != request.apply_plan_item_id {
        return Err(AppError::Message(
            "Referenced restore entry item does not match the requested item.".to_owned(),
        ));
    }
    if let Some(result_id) = request.result_id {
        if restore_entry.apply_plan_result_id != Some(result_id) {
            return Err(AppError::Message(
                "Referenced restore entry result does not match the requested result.".to_owned(),
            ));
        }
    }
    if let Some(result_id) = restore_entry.apply_plan_result_id {
        let result = detail
            .results
            .iter()
            .find(|result| result.id == result_id)
            .ok_or_else(|| {
                AppError::Message(
                    "Referenced restore entry points to a missing result log.".to_owned(),
                )
            })?;
        if result.apply_plan_id != request.apply_plan_id
            || result.apply_plan_run_id != request.run_id
            || result.apply_plan_item_id != restore_entry.apply_plan_item_id
        {
            return Err(AppError::Message(
                "Referenced restore entry result is outside the requested scope.".to_owned(),
            ));
        }
    }

    Ok(restore_entry)
}

pub(super) fn canonicalize_existing_dir(path: &Path, field_name: &str) -> AppResult<PathBuf> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| AppError::Message(format!("{field_name} is not available: {error}")))?;
    if !canonical.is_dir() {
        return Err(AppError::Message(format!(
            "{field_name} must be a directory."
        )));
    }
    Ok(canonical)
}

pub(super) fn resolve_fixture_candidate(path: &Path, field_name: &str) -> AppResult<PathBuf> {
    if !path.is_absolute() {
        return Err(AppError::Message(format!("{field_name} must be absolute.")));
    }
    if path.exists() {
        return fs::canonicalize(path).map_err(|error| {
            AppError::Message(format!("{field_name} cannot be resolved: {error}"))
        });
    }
    let parent = path.parent().ok_or_else(|| {
        AppError::Message(format!(
            "{field_name} must include an existing parent directory."
        ))
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Message(format!("{field_name} must include a file name.")))?;
    let parent = fs::canonicalize(parent).map_err(|error| {
        AppError::Message(format!(
            "{field_name} parent must exist for fixture boundary checks: {error}"
        ))
    })?;
    Ok(parent.join(file_name))
}

pub(super) fn ensure_under_root(path: &Path, root: &Path, field_name: &str) -> AppResult<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "{field_name} must stay inside the supplied fixture boundary."
        )))
    }
}

pub(super) fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = fs::File::open(path)
        .map_err(|error| AppError::Message(format!("Fixture file could not be opened: {error}")))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let bytes = file.read(&mut buffer).map_err(|error| {
            AppError::Message(format!("Fixture file could not be read: {error}"))
        })?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn require_trimmed(value: &str, field_name: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(AppError::Message(format!("{field_name} is required.")))
    } else {
        Ok(trimmed.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rusqlite::{params, Connection};
    use tempfile::{tempdir, TempDir};

    use crate::{
        core::apply_plan_results::{
            create_apply_plan_run_log, list_apply_plan_restore_entries,
            list_apply_plan_result_logs, record_apply_plan_restore_entry,
        },
        database,
        models::{
            ApplyPlanRestoreEntryStatus, ApplyPlanResultLogStatus, CreateApplyPlanRunLogRequest,
            ListApplyPlanRestoreEntriesRequest, ListApplyPlanResultLogsRequest,
            RecordApplyPlanRestoreEntryRequest, RecordApplyPlanResultLogRequest,
        },
    };

    use super::*;

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn insert_saved_plan(connection: &Connection) -> i64 {
        connection
            .execute(
                "INSERT INTO apply_plans (
                    source_plan_kind, title, summary, status, would_touch_files,
                    confirmation_required, backup_required, restore_available, total_items,
                    applyable_items, blocked_items, review_only_items, caveats_json,
                    created_at, updated_at
                ) VALUES ('test', 'Test plan', 'Fixture backup test plan', 'preview_only_source',
                    0, 1, 1, 0, 1, 0, 0, 0, '[]', '2026-01-01', '2026-01-01')",
                [],
            )
            .expect("insert plan");
        connection.last_insert_rowid()
    }

    fn insert_plan_item(connection: &Connection, plan_id: i64) -> i64 {
        connection
            .execute(
                "INSERT INTO apply_plan_items (
                    apply_plan_id, file_name, current_path, current_root, destination_path,
                    destination_root, action_kind, evidence_level, bucket, confidence_label,
                    item_status, blocked, review_only, path_privacy_level, created_at, updated_at
                ) VALUES (?1, 'sample.package', 'C:/Fixture/source/sample.package', 'mods',
                    'C:/Fixture/backup/sample.package', 'mods', 'fixture_backup_prototype',
                    'strong', 'test', 'high', 'draft_candidate', 0, 0,
                    'local_full_path_required', '2026-01-01', '2026-01-01')",
                params![plan_id],
            )
            .expect("insert item");
        connection.last_insert_rowid()
    }

    fn create_run(connection: &Connection, plan_id: i64) -> i64 {
        create_apply_plan_run_log(
            connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: None,
                skipped_items: None,
                failed_items: None,
                summary: Some("Fixture-only backup prototype run.".to_owned()),
            },
        )
        .expect("create run")
        .id
    }

    fn request(
        plan_id: i64,
        item_id: Option<i64>,
        run_id: i64,
        fixture_root: &Path,
        source_path: &Path,
        backup_root: &Path,
    ) -> FixtureBackupPrototypeRequest {
        FixtureBackupPrototypeRequest {
            fixture_mode: true,
            apply_plan_id: plan_id,
            apply_plan_item_id: item_id,
            run_id,
            fixture_root: fixture_root.to_path_buf(),
            source_path: source_path.to_path_buf(),
            backup_root: backup_root.to_path_buf(),
            operation_kind: "fixture_backup_prototype".to_owned(),
        }
    }

    fn restore_request(
        plan_id: i64,
        item_id: Option<i64>,
        run_id: i64,
        result_id: Option<i64>,
        restore_entry_id: Option<i64>,
        fixture_root: &Path,
        backup_path: &Path,
        restore_target_path: &Path,
    ) -> FixtureRestorePrototypeRequest {
        FixtureRestorePrototypeRequest {
            fixture_mode: true,
            fixture_root: fixture_root.to_path_buf(),
            backup_path: backup_path.to_path_buf(),
            restore_target_path: restore_target_path.to_path_buf(),
            apply_plan_id: plan_id,
            apply_plan_item_id: item_id,
            run_id,
            result_id,
            restore_entry_id,
            operation_kind: "fixture_restore_prototype".to_owned(),
        }
    }

    fn verified_backup_fixture(
        connection: &Connection,
        source_bytes: &[u8],
    ) -> (TempDir, i64, i64, i64, PathBuf, PathBuf, i64, i64) {
        let plan_id = insert_saved_plan(connection);
        let item_id = insert_plan_item(connection, plan_id);
        let run_id = create_run(connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = source_dir.join("sample.package");
        fs::write(&source_path, source_bytes).expect("source fixture");

        let outcome = run_fixture_backup_prototype(
            connection,
            request(
                plan_id,
                Some(item_id),
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("fixture backup");
        let FixtureBackupPrototypeOutcome::Verified(success) = outcome else {
            panic!("expected verified backup");
        };

        (
            temp,
            plan_id,
            item_id,
            run_id,
            source_path,
            success.backup_path,
            success.result_log_id,
            success.restore_entry_id,
        )
    }

    struct VerifiedRecoveryChain {
        _temp: TempDir,
        plan_id: i64,
        item_id: i64,
        run_id: i64,
        source_path: PathBuf,
        backup_path: PathBuf,
        restore_target_path: PathBuf,
        backup_result_id: i64,
        backup_restore_entry_id: i64,
        restore_result_id: i64,
        restore_entry_id: i64,
        source_hash: String,
        backup_hash: String,
        restored_hash: String,
    }

    fn run_verified_backup_restore_chain(
        connection: &Connection,
        source_bytes: &[u8],
    ) -> VerifiedRecoveryChain {
        let plan_id = insert_saved_plan(connection);
        let item_id = insert_plan_item(connection, plan_id);
        let run_id = create_run(connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("backup");
        let restore_dir = fixture_root.join("restored");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(&backup_root).expect("backup dir");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let source_path = source_dir.join("sample.package");
        let restore_target_path = restore_dir.join("sample.package");
        fs::write(&source_path, source_bytes).expect("source fixture");
        let source_hash = hash_file(&source_path).expect("source hash");

        let backup_outcome = run_fixture_backup_prototype(
            connection,
            request(
                plan_id,
                Some(item_id),
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("fixture backup");
        let FixtureBackupPrototypeOutcome::Verified(backup_success) = backup_outcome else {
            panic!("expected verified backup");
        };
        assert!(backup_success.backup_verified);
        assert_eq!(backup_success.source_hash, source_hash);
        assert_eq!(backup_success.source_hash, backup_success.backup_hash);
        assert_eq!(
            fs::read(&source_path).expect("source after backup"),
            source_bytes
        );
        assert_eq!(
            fs::read(&backup_success.backup_path).expect("backup after backup"),
            source_bytes
        );

        let results_after_backup = list_apply_plan_result_logs(
            connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("backup results");
        assert_eq!(results_after_backup.len(), 1);
        assert_eq!(results_after_backup[0].id, backup_success.result_log_id);
        assert_eq!(
            results_after_backup[0].result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(results_after_backup[0].apply_plan_id, plan_id);
        assert_eq!(results_after_backup[0].apply_plan_item_id, Some(item_id));

        let entries_after_backup = list_apply_plan_restore_entries(
            connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("backup restore entries");
        assert_eq!(entries_after_backup.len(), 1);
        assert_eq!(entries_after_backup[0].id, backup_success.restore_entry_id);
        assert_eq!(
            entries_after_backup[0].restore_status,
            ApplyPlanRestoreEntryStatus::DesignOnly
        );
        assert_eq!(
            entries_after_backup[0].operation_result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(entries_after_backup[0].apply_plan_id, plan_id);
        assert_eq!(entries_after_backup[0].apply_plan_item_id, Some(item_id));
        assert_eq!(
            entries_after_backup[0].apply_plan_result_id,
            Some(backup_success.result_log_id)
        );

        let backup_bytes_before_restore =
            fs::read(&backup_success.backup_path).expect("backup before restore");
        let restore_outcome = run_fixture_restore_prototype(
            connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_success.result_log_id),
                Some(backup_success.restore_entry_id),
                fixture_root,
                &backup_success.backup_path,
                &restore_target_path,
            ),
        )
        .expect("fixture restore");
        let FixtureRestorePrototypeOutcome::Verified(restore_success) = restore_outcome else {
            panic!("expected verified restore");
        };
        assert!(restore_success.restore_verified);
        assert_eq!(
            restore_success.source_restore_entry_id,
            backup_success.restore_entry_id
        );
        assert_eq!(
            fs::read(&backup_success.backup_path).expect("backup after restore"),
            backup_bytes_before_restore
        );
        assert_eq!(
            fs::read(&restore_target_path).expect("restored target"),
            backup_bytes_before_restore
        );
        assert_eq!(restore_success.backup_hash, restore_success.restored_hash);
        assert_eq!(restore_success.backup_hash, backup_success.backup_hash);

        let results_after_restore = list_apply_plan_result_logs(
            connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("restore results");
        assert_eq!(results_after_restore.len(), 2);
        let restore_result = results_after_restore
            .iter()
            .find(|result| result.id == restore_success.result_log_id)
            .expect("restore result");
        assert_eq!(
            restore_result.result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(restore_result.apply_plan_id, plan_id);
        assert_eq!(restore_result.apply_plan_item_id, Some(item_id));

        let entries_after_restore = list_apply_plan_restore_entries(
            connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries_after_restore.len(), 2);
        let restore_entry = entries_after_restore
            .iter()
            .find(|entry| entry.id == restore_success.restore_entry_id)
            .expect("restore proof entry");
        assert_eq!(
            restore_entry.restore_status,
            ApplyPlanRestoreEntryStatus::DesignOnly
        );
        assert_eq!(
            restore_entry.operation_result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(restore_entry.apply_plan_id, plan_id);
        assert_eq!(restore_entry.apply_plan_item_id, Some(item_id));
        assert_eq!(
            restore_entry.apply_plan_result_id,
            Some(restore_success.result_log_id)
        );

        VerifiedRecoveryChain {
            _temp: temp,
            plan_id,
            item_id,
            run_id,
            source_path,
            backup_path: backup_success.backup_path,
            restore_target_path,
            backup_result_id: backup_success.result_log_id,
            backup_restore_entry_id: backup_success.restore_entry_id,
            restore_result_id: restore_success.result_log_id,
            restore_entry_id: restore_success.restore_entry_id,
            source_hash,
            backup_hash: backup_success.backup_hash,
            restored_hash: restore_success.restored_hash,
        }
    }

    fn assert_no_unsafe_fixture_statuses(connection: &Connection) {
        let result_statuses: Vec<String> = connection
            .prepare("SELECT result_status FROM apply_plan_results")
            .expect("prepare result statuses")
            .query_map([], |row| row.get(0))
            .expect("query result statuses")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect result statuses");
        for forbidden in [
            "applied",
            "restored",
            "moved",
            "copied",
            "restore_complete",
            "apply_complete",
        ] {
            assert!(
                !result_statuses.iter().any(|status| status == forbidden),
                "fixture proof must not record result status {forbidden}"
            );
        }

        let restore_statuses: Vec<String> = connection
            .prepare("SELECT restore_status FROM apply_plan_restore_entries")
            .expect("prepare restore statuses")
            .query_map([], |row| row.get(0))
            .expect("query restore statuses")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect restore statuses");
        for forbidden in [
            "applied",
            "restored",
            "moved",
            "copied",
            "restore_complete",
            "apply_complete",
        ] {
            assert!(
                !restore_statuses.iter().any(|status| status == forbidden),
                "fixture proof must not record restore status {forbidden}"
            );
        }
    }

    #[test]
    fn fixture_backup_copies_verifies_and_records_metadata() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let item_id = insert_plan_item(&connection, plan_id);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = source_dir.join("sample.package");
        fs::write(&source_path, b"fixture package bytes").expect("source fixture");

        let outcome = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                Some(item_id),
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("fixture backup");

        let FixtureBackupPrototypeOutcome::Verified(success) = outcome else {
            panic!("expected verified backup");
        };
        assert!(success.backup_verified);
        assert!(success
            .backup_path
            .starts_with(fs::canonicalize(&backup_root).expect("canonical backup root")));
        assert!(success.backup_path.exists());
        assert_eq!(
            fs::read(&source_path).expect("source"),
            b"fixture package bytes"
        );
        assert_eq!(
            fs::read(&success.backup_path).expect("backup"),
            b"fixture package bytes"
        );
        assert_eq!(success.source_size, success.backup_size);
        assert_eq!(success.source_hash, success.backup_hash);

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, success.result_log_id);
        assert_eq!(
            results[0].result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(results[0].user_summary, SUCCESS_SUMMARY);
        assert!(results[0].backup_path.is_some());

        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, success.restore_entry_id);
        assert_eq!(
            entries[0].restore_status,
            ApplyPlanRestoreEntryStatus::DesignOnly
        );
        assert_eq!(
            entries[0].operation_result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(entries[0].file_hash_before, Some(success.source_hash));
    }

    #[test]
    fn empty_file_backup_verifies_successfully() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = source_dir.join("empty.package");
        fs::write(&source_path, b"").expect("empty source");

        let outcome = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("fixture backup");

        let FixtureBackupPrototypeOutcome::Verified(success) = outcome else {
            panic!("expected verified backup");
        };
        assert_eq!(success.source_size, 0);
        assert_eq!(success.backup_size, 0);
        assert_eq!(success.source_hash, success.backup_hash);
    }

    #[test]
    fn missing_source_records_failed_before_change_without_restore_entry() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = source_dir.join("missing.package");

        let outcome = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("failure outcome");

        let FixtureBackupPrototypeOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "source_missing");
        assert!(!failure.backup_created);
        assert!(!backup_root.join("run-1").exists());

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, failure.result_log_id);
        assert_eq!(
            results[0].result_status,
            ApplyPlanResultLogStatus::FailedBeforeChange
        );
        assert_eq!(results[0].error_code.as_deref(), Some("source_missing"));
        assert!(results[0].backup_path.is_none());

        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("restore entries");
        assert!(entries.is_empty());
    }

    #[test]
    fn missing_backup_root_is_rejected_before_copy() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("missing-backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("sample.package");
        fs::write(&source_path, b"fixture").expect("source fixture");

        let error = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect_err("missing backup root rejected");
        assert!(error.to_string().contains("backupRoot"));
        assert_eq!(
            list_apply_plan_result_logs(
                &connection,
                ListApplyPlanResultLogsRequest {
                    apply_plan_run_id: run_id,
                },
            )
            .expect("results")
            .len(),
            0
        );
    }

    #[test]
    fn source_outside_fixture_root_is_rejected() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let fixture = tempdir().expect("fixture tempdir");
        let outside = tempdir().expect("outside tempdir");
        let backup_root = fixture.path().join("backup");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = outside.path().join("sample.package");
        fs::write(&source_path, b"fixture").expect("outside source");

        let error = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture.path(),
                &source_path,
                &backup_root,
            ),
        )
        .expect_err("outside source rejected");
        assert!(error.to_string().contains("sourcePath"));
    }

    #[test]
    fn backup_root_outside_fixture_root_is_rejected() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let fixture = tempdir().expect("fixture tempdir");
        let outside = tempdir().expect("outside tempdir");
        let source_dir = fixture.path().join("source");
        fs::create_dir_all(&source_dir).expect("source dir");
        let source_path = source_dir.join("sample.package");
        fs::write(&source_path, b"fixture").expect("source fixture");

        let error = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture.path(),
                &source_path,
                outside.path(),
            ),
        )
        .expect_err("outside backup root rejected");
        assert!(error.to_string().contains("backupRoot"));
    }

    #[test]
    fn source_under_backup_root_is_rejected() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = backup_root.join("sample.package");
        fs::write(&source_path, b"fixture").expect("source under backup");

        let error = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect_err("backup source rejected");
        assert!(error.to_string().contains("sourcePath"));
    }

    #[test]
    fn existing_backup_destination_is_not_overwritten() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let item_id = insert_plan_item(&connection, plan_id);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let source_dir = fixture_root.join("source");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("source dir");
        fs::create_dir_all(
            backup_root
                .join(format!("run-{run_id}"))
                .join(format!("item-{item_id}")),
        )
        .expect("backup dir");
        let source_path = source_dir.join("sample.package");
        fs::write(&source_path, b"source bytes").expect("source fixture");
        let existing_backup = backup_root
            .join(format!("run-{run_id}"))
            .join(format!("item-{item_id}"))
            .join("sample.package");
        fs::write(&existing_backup, b"existing backup").expect("existing backup");

        let outcome = run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                Some(item_id),
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("failure outcome");

        let FixtureBackupPrototypeOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "backup_destination_exists");
        assert_eq!(
            fs::read(&existing_backup).expect("existing backup"),
            b"existing backup"
        );
    }

    #[test]
    fn fixture_mode_is_required() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let backup_root = fixture_root.join("backup");
        let source_path = fixture_root.join("source.package");
        fs::create_dir_all(&backup_root).expect("backup dir");
        fs::write(&source_path, b"fixture").expect("source fixture");
        let mut request = request(
            plan_id,
            None,
            run_id,
            fixture_root,
            &source_path,
            &backup_root,
        );
        request.fixture_mode = false;

        let error = run_fixture_backup_prototype(&connection, request).expect_err("mode rejected");
        assert!(error.to_string().contains("fixtureMode=true"));
    }

    #[test]
    fn fixture_backup_does_not_record_applied_or_restored_statuses() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&backup_root).expect("backup dir");
        let source_path = fixture_root.join("source.package");
        fs::write(&source_path, b"fixture").expect("source fixture");

        run_fixture_backup_prototype(
            &connection,
            request(
                plan_id,
                None,
                run_id,
                fixture_root,
                &source_path,
                &backup_root,
            ),
        )
        .expect("fixture backup");

        let result_statuses: Vec<String> = connection
            .prepare("SELECT result_status FROM apply_plan_results")
            .expect("prepare results")
            .query_map([], |row| row.get(0))
            .expect("query results")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect results");
        assert!(!result_statuses.iter().any(|status| status == "applied"));

        let restore_statuses: Vec<String> = connection
            .prepare("SELECT restore_status FROM apply_plan_restore_entries")
            .expect("prepare restore")
            .query_map([], |row| row.get(0))
            .expect("query restore")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect restore");
        assert!(!restore_statuses.iter().any(|status| status == "restored"));
    }

    #[test]
    fn full_fixture_backup_restore_chain_verifies_files_and_metadata() {
        let connection = memory_connection();
        let source_bytes = b"fixture integration package bytes";

        let chain = run_verified_backup_restore_chain(&connection, source_bytes);

        let fixture_root = fs::canonicalize(chain._temp.path()).expect("fixture root");
        let canonical_source = fs::canonicalize(&chain.source_path).expect("canonical source");
        assert!(canonical_source.starts_with(fixture_root));
        assert!(chain.source_path.exists());
        assert!(chain.backup_path.exists());
        assert!(chain.restore_target_path.exists());
        assert_eq!(fs::read(&chain.source_path).expect("source"), source_bytes);
        assert_eq!(fs::read(&chain.backup_path).expect("backup"), source_bytes);
        assert_eq!(
            fs::read(&chain.restore_target_path).expect("restored"),
            source_bytes
        );
        assert_eq!(chain.source_hash, chain.backup_hash);
        assert_eq!(chain.backup_hash, chain.restored_hash);
        assert_ne!(chain.backup_result_id, chain.restore_result_id);
        assert_ne!(chain.backup_restore_entry_id, chain.restore_entry_id);

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: chain.run_id,
            },
        )
        .expect("results");
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|result| {
            result.apply_plan_id == chain.plan_id
                && result.apply_plan_item_id == Some(chain.item_id)
                && result.apply_plan_run_id == chain.run_id
                && result.result_status == ApplyPlanResultLogStatus::PendingLog
        }));

        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: chain.run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|entry| {
            entry.apply_plan_id == chain.plan_id
                && entry.apply_plan_item_id == Some(chain.item_id)
                && entry.apply_plan_run_id == chain.run_id
                && entry.restore_status == ApplyPlanRestoreEntryStatus::DesignOnly
                && entry.operation_result_status == ApplyPlanResultLogStatus::PendingLog
        }));
        assert_no_unsafe_fixture_statuses(&connection);
    }

    #[test]
    fn fixture_recovery_chain_rejects_mismatched_run_plan_item_or_result_scope() {
        let connection = memory_connection();
        let chain = run_verified_backup_restore_chain(&connection, b"fixture scope bytes");
        let restore_parent = chain
            .restore_target_path
            .parent()
            .expect("restore parent")
            .to_path_buf();
        let result_count_before = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: chain.run_id,
            },
        )
        .expect("results before")
        .len();
        let entry_count_before = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: chain.run_id,
            },
        )
        .expect("entries before")
        .len();

        let wrong_plan_id = insert_saved_plan(&connection);
        let wrong_plan_error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                wrong_plan_id,
                Some(chain.item_id),
                chain.run_id,
                Some(chain.backup_result_id),
                Some(chain.backup_restore_entry_id),
                chain._temp.path(),
                &chain.backup_path,
                &restore_parent.join("wrong-plan.package"),
            ),
        )
        .expect_err("wrong plan rejected");
        assert!(wrong_plan_error.to_string().contains("ApplyPlan"));

        let wrong_run_id = create_run(&connection, chain.plan_id);
        let wrong_run_error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                chain.plan_id,
                Some(chain.item_id),
                wrong_run_id,
                Some(chain.backup_result_id),
                Some(chain.backup_restore_entry_id),
                chain._temp.path(),
                &chain.backup_path,
                &restore_parent.join("wrong-run.package"),
            ),
        )
        .expect_err("wrong run rejected");
        assert!(wrong_run_error.to_string().contains("this run"));

        let wrong_item_error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                chain.plan_id,
                None,
                chain.run_id,
                Some(chain.backup_result_id),
                Some(chain.backup_restore_entry_id),
                chain._temp.path(),
                &chain.backup_path,
                &restore_parent.join("wrong-item.package"),
            ),
        )
        .expect_err("wrong item rejected");
        assert!(wrong_item_error.to_string().contains("item"));

        let wrong_result_error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                chain.plan_id,
                Some(chain.item_id),
                chain.run_id,
                Some(chain.backup_result_id + 100_000),
                Some(chain.backup_restore_entry_id),
                chain._temp.path(),
                &chain.backup_path,
                &restore_parent.join("wrong-result.package"),
            ),
        )
        .expect_err("wrong result rejected");
        assert!(wrong_result_error.to_string().contains("result"));

        assert_eq!(
            list_apply_plan_result_logs(
                &connection,
                ListApplyPlanResultLogsRequest {
                    apply_plan_run_id: chain.run_id,
                },
            )
            .expect("results after")
            .len(),
            result_count_before
        );
        assert_eq!(
            list_apply_plan_restore_entries(
                &connection,
                ListApplyPlanRestoreEntriesRequest {
                    apply_plan_run_id: chain.run_id,
                },
            )
            .expect("entries after")
            .len(),
            entry_count_before
        );
        assert_no_unsafe_fixture_statuses(&connection);
    }

    #[test]
    fn fixture_recovery_chain_refuses_restore_overwrite_after_verified_backup() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"backup content");
        let restore_dir = temp.path().join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");
        fs::write(&restore_target_path, b"existing target").expect("existing target");

        let outcome = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                temp.path(),
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect("overwrite refusal outcome");

        let FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "restore_target_exists");
        assert!(!failure.restore_created);
        assert_eq!(
            fs::read(&restore_target_path).expect("restore target"),
            b"existing target"
        );
        assert_no_unsafe_fixture_statuses(&connection);
    }

    #[test]
    fn fixture_restore_copies_verifies_and_records_metadata() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture package bytes");
        let fixture_root = temp.path();
        let restore_dir = fixture_root.join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");
        let backup_bytes_before = fs::read(&backup_path).expect("backup before");

        let outcome = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                fixture_root,
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect("fixture restore");

        let FixtureRestorePrototypeOutcome::Verified(success) = outcome else {
            panic!("expected verified restore");
        };
        assert!(success.restore_verified);
        assert_eq!(success.source_restore_entry_id, backup_restore_entry_id);
        let canonical_restore_target =
            fs::canonicalize(&restore_target_path).expect("canonical restore target");
        assert_eq!(success.restored_path, canonical_restore_target);
        assert!(restore_target_path.exists());
        assert_eq!(
            fs::read(&restore_target_path).expect("restored"),
            b"fixture package bytes"
        );
        assert_eq!(
            fs::read(&backup_path).expect("backup after"),
            backup_bytes_before
        );
        assert_eq!(success.backup_size, success.restored_size);
        assert_eq!(success.backup_hash, success.restored_hash);

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("results");
        assert_eq!(results.len(), 2);
        let restore_result = results
            .iter()
            .find(|result| result.id == success.result_log_id)
            .expect("restore result");
        assert_eq!(
            restore_result.result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(restore_result.user_summary, RESTORE_SUCCESS_SUMMARY);
        let restored_path_string = success.restored_path.to_string_lossy().to_string();
        assert_eq!(
            restore_result.destination_path_at_execution.as_deref(),
            Some(restored_path_string.as_str())
        );

        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 2);
        let restore_entry = entries
            .iter()
            .find(|entry| entry.id == success.restore_entry_id)
            .expect("new restore entry");
        assert_eq!(
            restore_entry.restore_status,
            ApplyPlanRestoreEntryStatus::DesignOnly
        );
        assert_eq!(
            restore_entry.operation_result_status,
            ApplyPlanResultLogStatus::PendingLog
        );
        assert_eq!(
            restore_entry.destination_path_at_execution.as_deref(),
            Some(restored_path_string.as_str())
        );
        assert_eq!(restore_entry.file_hash_before, Some(success.backup_hash));
    }

    #[test]
    fn missing_backup_records_failed_before_change_without_restore_entry() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection);
        let item_id = insert_plan_item(&connection, plan_id);
        let run_id = create_run(&connection, plan_id);
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let backup_root = fixture_root.join("backup");
        let restore_dir = fixture_root.join("restored");
        fs::create_dir_all(&backup_root).expect("backup dir");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let missing_backup = backup_root.join("missing.package");
        let restore_target_path = restore_dir.join("missing.package");

        let result = apply_plan_results::record_apply_plan_result_log(
            &connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: run_id,
                apply_plan_item_id: Some(item_id),
                operation_kind: "fixture_backup_prototype".to_owned(),
                result_status: "pending_log".to_owned(),
                source_path_at_execution: None,
                destination_path_at_execution: None,
                backup_path: Some(missing_backup.to_string_lossy().to_string()),
                error_code: None,
                error_message: None,
                user_summary: "Fixture metadata points to a missing backup.".to_owned(),
            },
        )
        .expect("result");
        let entry = record_apply_plan_restore_entry(
            &connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: run_id,
                apply_plan_result_id: Some(result.id),
                apply_plan_item_id: Some(item_id),
                original_source_path: "C:/Fixture/source/missing.package".to_owned(),
                destination_path_at_execution: None,
                backup_path: Some(missing_backup.to_string_lossy().to_string()),
                file_hash_before: None,
                file_size_before: None,
                operation_kind: "fixture_backup_prototype".to_owned(),
                operation_result_status: "pending_log".to_owned(),
                restore_status: "design_only".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )
        .expect("restore entry");

        let outcome = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(result.id),
                Some(entry.id),
                fixture_root,
                &missing_backup,
                &restore_target_path,
            ),
        )
        .expect("failure outcome");

        let FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "backup_missing");
        assert!(!failure.restore_created);
        assert!(!restore_target_path.exists());

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("results");
        assert_eq!(results.len(), 2);
        assert_eq!(
            results
                .iter()
                .find(|result| result.id == failure.result_log_id)
                .expect("failure result")
                .result_status,
            ApplyPlanResultLogStatus::FailedBeforeChange
        );
        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn existing_restore_target_is_refused_without_overwrite() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"source bytes");
        let fixture_root = temp.path();
        let restore_dir = fixture_root.join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");
        fs::write(&restore_target_path, b"existing restored file").expect("existing target");

        let outcome = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                fixture_root,
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect("failure outcome");

        let FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "restore_target_exists");
        assert_eq!(
            fs::read(&restore_target_path).expect("existing target"),
            b"existing restored file"
        );
    }

    #[test]
    fn restore_rejects_paths_outside_fixture_root() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture bytes");
        let outside = tempdir().expect("outside tempdir");
        let outside_backup = outside.path().join("sample.package");
        fs::write(&outside_backup, b"fixture bytes").expect("outside backup");
        let restore_dir = temp.path().join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");

        let error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                temp.path(),
                &outside_backup,
                &restore_target_path,
            ),
        )
        .expect_err("outside backup rejected");
        assert!(error.to_string().contains("backupPath"));

        let outside_target = outside.path().join("restored.package");
        let error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                temp.path(),
                &backup_path,
                &outside_target,
            ),
        )
        .expect_err("outside target rejected");
        assert!(error.to_string().contains("restoreTargetPath"));
    }

    #[test]
    fn restore_target_parent_must_exist_before_restore() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture bytes");
        let missing_parent_target = temp.path().join("missing-parent").join("sample.package");

        let error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                temp.path(),
                &backup_path,
                &missing_parent_target,
            ),
        )
        .expect_err("missing parent rejected");
        assert!(error.to_string().contains("restoreTargetPath"));
        assert!(!temp.path().join("missing-parent").exists());
    }

    #[test]
    fn fixture_restore_mode_is_required() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture bytes");
        let restore_dir = temp.path().join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");
        let mut request = restore_request(
            plan_id,
            Some(item_id),
            run_id,
            Some(backup_result_id),
            Some(backup_restore_entry_id),
            temp.path(),
            &backup_path,
            &restore_target_path,
        );
        request.fixture_mode = false;

        let error = run_fixture_restore_prototype(&connection, request).expect_err("mode rejected");
        assert!(error.to_string().contains("fixtureMode=true"));
    }

    #[test]
    fn restore_entry_scope_must_match_requested_run_plan_item_and_result() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture bytes");
        let restore_dir = temp.path().join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");

        let error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                None,
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                temp.path(),
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect_err("item mismatch rejected");
        assert!(error.to_string().contains("item"));

        let error = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id + 10_000),
                Some(backup_restore_entry_id),
                temp.path(),
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect_err("result mismatch rejected");
        assert!(error.to_string().contains("result"));
    }

    #[test]
    fn recorded_backup_hash_or_size_mismatch_fails_before_copy() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            _backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture bytes");
        let restore_dir = temp.path().join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");
        let mismatch_entry = record_apply_plan_restore_entry(
            &connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: run_id,
                apply_plan_result_id: Some(backup_result_id),
                apply_plan_item_id: Some(item_id),
                original_source_path: "C:/Fixture/source/sample.package".to_owned(),
                destination_path_at_execution: None,
                backup_path: Some(backup_path.to_string_lossy().to_string()),
                file_hash_before: Some("not-the-backup-hash".to_owned()),
                file_size_before: Some(123456),
                operation_kind: "fixture_backup_prototype".to_owned(),
                operation_result_status: "pending_log".to_owned(),
                restore_status: "design_only".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )
        .expect("mismatch entry");

        let outcome = run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(mismatch_entry.id),
                temp.path(),
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect("failure outcome");

        let FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) = outcome else {
            panic!("expected failed-before-change outcome");
        };
        assert_eq!(failure.error_code, "backup_size_mismatch");
        assert!(!restore_target_path.exists());
    }

    #[test]
    fn fixture_restore_does_not_record_applied_restored_moved_or_copied_statuses() {
        let connection = memory_connection();
        let (
            temp,
            plan_id,
            item_id,
            run_id,
            _source_path,
            backup_path,
            backup_result_id,
            backup_restore_entry_id,
        ) = verified_backup_fixture(&connection, b"fixture bytes");
        let restore_dir = temp.path().join("restored");
        fs::create_dir_all(&restore_dir).expect("restore dir");
        let restore_target_path = restore_dir.join("sample.package");

        run_fixture_restore_prototype(
            &connection,
            restore_request(
                plan_id,
                Some(item_id),
                run_id,
                Some(backup_result_id),
                Some(backup_restore_entry_id),
                temp.path(),
                &backup_path,
                &restore_target_path,
            ),
        )
        .expect("fixture restore");

        let result_statuses: Vec<String> = connection
            .prepare("SELECT result_status FROM apply_plan_results")
            .expect("prepare results")
            .query_map([], |row| row.get(0))
            .expect("query results")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect results");
        for forbidden in ["applied", "moved", "copied"] {
            assert!(
                !result_statuses.iter().any(|status| status == forbidden),
                "fixture restore must not record {forbidden}"
            );
        }

        let restore_statuses: Vec<String> = connection
            .prepare("SELECT restore_status FROM apply_plan_restore_entries")
            .expect("prepare restore")
            .query_map([], |row| row.get(0))
            .expect("query restore")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect restore");
        assert!(!restore_statuses.iter().any(|status| status == "restored"));
    }

    #[test]
    fn prototype_is_not_registered_as_a_tauri_command() {
        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("run_fixture_backup_prototype"));
        assert!(!commands_source.contains("fixture_backup_prototype"));
        assert!(!commands_source.contains("run_fixture_restore_prototype"));
        assert!(!commands_source.contains("fixture_restore_prototype"));
        assert!(!commands_source.contains("fixture_backup_restore_integration"));
    }

    #[test]
    fn prototype_does_not_call_forbidden_file_changing_systems() {
        let source = include_str!("apply_plan_backup_prototype.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");

        for forbidden in [
            "move_engine",
            "snapshot_manager",
            "downloads_watcher",
            "apply_preview",
            "commit_staging",
            "cleanup_staging",
            "apply_download",
            "reject_download",
            "restore_rejected",
            "restore_snapshot",
            "undo_applied",
            "remove_file",
            "remove_dir",
            "rename(",
            "std::process",
            "Command::",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "fixture backup prototype must not call {forbidden}"
            );
        }
    }
}
