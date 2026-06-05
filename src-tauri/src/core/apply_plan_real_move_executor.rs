#![cfg(feature = "apply-executor-real-move-spike")]
#![cfg_attr(not(test), allow(dead_code))]

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    core::{apply_plan_operation_preview, apply_plan_results},
    error::{AppError, AppResult},
    models::{
        ApplyPlanDryRunActionPreview, LibrarySettings, PreviewApplyPlanOperationsRequest,
        RecordApplyPlanRestoreEntryRequest, RecordApplyPlanResultLogRequest,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct HiddenRealMoveExecutorRequest {
    pub real_move_feature_gate_enabled: bool,
    pub plan_id: i64,
    pub confirmation_token: String,
    pub expected_plan_hash: Option<String>,
    pub expected_operation_set_hash: Option<String>,
    pub backup_root: PathBuf,
    #[cfg(test)]
    pub force_rename_failure_after_backup: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenRealMoveExecutorOutcome {
    pub feature_gate_enabled: bool,
    pub plan_id: i64,
    pub token_run_id: i64,
    pub operation_set_hash: String,
    pub executed_operations: Vec<HiddenRealMoveOperationSuccess>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenRealMoveOperationSuccess {
    pub item_id: i64,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub backup_path: PathBuf,
    pub source_hash: String,
    pub destination_hash: String,
    pub backup_hash: String,
    pub source_size: u64,
    pub destination_size: u64,
    pub backup_size: u64,
    pub backup_verified: bool,
    pub move_verified: bool,
    pub backup_result_log_id: i64,
    pub move_result_log_id: i64,
    pub restore_entry_id: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct HiddenRunScopedRestoreRequest {
    pub real_move_feature_gate_enabled: bool,
    pub apply_plan_run_id: i64,
    pub confirmation_token: String,
    pub restore_entry_ids: Vec<i64>,
    pub backup_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenRunScopedRestoreOutcome {
    pub feature_gate_enabled: bool,
    pub apply_plan_run_id: i64,
    pub restored_entries: Vec<HiddenRunScopedRestoreSuccess>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HiddenRunScopedRestoreSuccess {
    pub restore_entry_id: i64,
    pub result_log_id: i64,
    pub item_id: Option<i64>,
    pub original_source_path: PathBuf,
    pub destination_path: PathBuf,
    pub backup_path: PathBuf,
    pub restored_hash: String,
    pub restored_size: u64,
}

struct TokenRecord {
    run_id: i64,
    plan_hash: String,
    operation_set_hash: String,
    allowed_operation_count: i64,
}

pub(crate) fn run_hidden_real_move_executor_spike(
    connection: &Connection,
    settings: &LibrarySettings,
    request: HiddenRealMoveExecutorRequest,
) -> AppResult<HiddenRealMoveExecutorOutcome> {
    if !request.real_move_feature_gate_enabled {
        return Err(AppError::Message(
            "Hidden real move executor spike requires the runtime feature gate to be enabled."
                .to_owned(),
        ));
    }

    let token = require_trimmed(&request.confirmation_token, "confirmationToken")?;
    let roots = ConfiguredRoots::from_settings(settings)?;
    fs::create_dir_all(&request.backup_root).map_err(|error| {
        AppError::Message(format!(
            "Hidden real executor backup root could not be created: {error}"
        ))
    })?;
    let backup_root = canonicalize_existing_dir(&request.backup_root, "backupRoot")?;

    let token_record = load_token_record(connection, request.plan_id, &token)?;
    if token_already_has_results(connection, token_record.run_id)? {
        return Err(AppError::Message(
            "Confirmation token has already been used by the hidden real move executor spike."
                .to_owned(),
        ));
    }

    if let Some(expected_plan_hash) = request
        .expected_plan_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if expected_plan_hash != token_record.plan_hash {
            return Err(AppError::Message(
                "Expected ApplyPlan hash did not match the backend-issued confirmation token."
                    .to_owned(),
            ));
        }
    }

    if let Some(expected_operation_set_hash) = request
        .expected_operation_set_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if expected_operation_set_hash != token_record.operation_set_hash {
            return Err(AppError::Message(
                "Expected operation-set hash did not match the backend-issued confirmation token."
                    .to_owned(),
            ));
        }
    }

    let operation_preview = apply_plan_operation_preview::preview_apply_plan_operations(
        connection,
        settings,
        PreviewApplyPlanOperationsRequest {
            plan_id: request.plan_id,
            expected_plan_hash: Some(token_record.plan_hash.clone()),
        },
    )?;

    if operation_preview.operation_set_hash != token_record.operation_set_hash {
        return Err(AppError::Message(
            "Current operation-set hash no longer matches the backend-issued confirmation token. Reload and re-confirm before hidden real execution.".to_owned(),
        ));
    }
    if operation_preview.operations.is_empty() {
        return Err(AppError::Message(
            "Hidden real move executor spike requires at least one backend-owned operation candidate."
                .to_owned(),
        ));
    }
    let operation_count = operation_preview.operations.len();
    if operation_count as i64 != token_record.allowed_operation_count {
        return Err(AppError::Message(
            "Current operation count no longer matches the backend-issued confirmation token."
                .to_owned(),
        ));
    }

    apply_plan_results::mark_apply_plan_run_applying(
        connection,
        token_record.run_id,
        operation_count as i64,
    )?;

    let mut executed_operations = Vec::with_capacity(operation_count);
    for operation in operation_preview.operations {
        if operation.action_preview != ApplyPlanDryRunActionPreview::WouldMoveLater {
            return Err(AppError::Message(
                "Hidden real executor spike accepts move-only operation previews.".to_owned(),
            ));
        }
        let source_path = operation.source_path.as_deref().ok_or_else(|| {
            AppError::Message("Hidden real move operation is missing a source path.".to_owned())
        })?;
        let destination_path = operation.destination_path.as_deref().ok_or_else(|| {
            AppError::Message(
                "Hidden real move operation is missing a destination path.".to_owned(),
            )
        })?;
        let source_path = canonicalize_existing_file(Path::new(source_path), "sourcePath")?;
        let source_root = roots.matching_root(&source_path, "sourcePath")?;
        if source_path.starts_with(&backup_root) {
            return Err(AppError::Message(
                "sourcePath must not be inside backupRoot.".to_owned(),
            ));
        }
        let destination_path = resolve_new_file_under_configured_root(
            Path::new(destination_path),
            &roots,
            source_root,
            "destinationPath",
        )?;
        if source_path == destination_path {
            return Err(AppError::Message(
                "Hidden real move source and destination must be different paths.".to_owned(),
            ));
        }

        let source_hash = hash_file(&source_path)?;
        let source_size = fs::metadata(&source_path)?.len();
        let backup = create_backup_first(
            connection,
            token_record.run_id,
            operation.item_id,
            &backup_root,
            &source_path,
            &source_hash,
            source_size,
        )?;

        #[cfg(test)]
        if request.force_rename_failure_after_backup {
            let error = std::io::Error::other("forced test rename failure after verified backup");
            record_failed_move_after_backup(
                connection,
                FailedMoveAfterBackupRecord {
                    run_id: token_record.run_id,
                    item_id: operation.item_id,
                    source_path: &source_path,
                    destination_path: &destination_path,
                    backup: &backup,
                    source_hash: &source_hash,
                    source_size,
                    error_message: error.to_string(),
                },
            )?;
            mark_run_apply_failed_after_verified_backup(
                connection,
                token_record.run_id,
                executed_operations.len() as i64,
                operation_count as i64,
            )?;
            return Err(AppError::Message(format!(
                "Hidden real move failed after backup proof; restore from backup is required before retry: {error}. Recovery metadata was recorded from backend-observed paths."
            )));
        }

        if let Err(error) = fs::rename(&source_path, &destination_path) {
            let recovery_error = record_failed_move_after_backup(
                connection,
                FailedMoveAfterBackupRecord {
                    run_id: token_record.run_id,
                    item_id: operation.item_id,
                    source_path: &source_path,
                    destination_path: &destination_path,
                    backup: &backup,
                    source_hash: &source_hash,
                    source_size,
                    error_message: error.to_string(),
                },
            );
            let recovery_note = match recovery_error {
                Ok(()) => {
                    mark_run_apply_failed_after_verified_backup(
                        connection,
                        token_record.run_id,
                        executed_operations.len() as i64,
                        operation_count as i64,
                    )?;
                    "Recovery metadata was recorded from backend-observed paths.".to_owned()
                }
                Err(recovery_error) => format!(
                    "Recovery metadata could not be recorded and requires manual audit: {recovery_error}"
                ),
            };
            return Err(AppError::Message(format!(
                "Hidden real move failed after backup proof; restore from backup is required before retry: {error}. {recovery_note}"
            )));
        }
        if source_path.exists() {
            return Err(AppError::Message(
                "Hidden real move verification failed: source still exists after move.".to_owned(),
            ));
        }
        let destination_hash = hash_file(&destination_path)?;
        let destination_size = fs::metadata(&destination_path)?.len();
        if destination_hash != source_hash || destination_size != source_size {
            return Err(AppError::Message(
                "Hidden real move verification failed after move.".to_owned(),
            ));
        }

        let move_result = apply_plan_results::record_backend_apply_plan_result_log(
            connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: token_record.run_id,
                apply_plan_item_id: Some(operation.item_id),
                operation_kind: "hidden_real_move_only".to_owned(),
                result_status: "applied".to_owned(),
                source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
                destination_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
                backup_path: Some(backup.backup_path.to_string_lossy().to_string()),
                error_code: None,
                error_message: None,
                user_summary: "Hidden feature-gated real move spike completed after backup verification. Not exposed to the UI.".to_owned(),
            },
        )?;
        let restore_entry = apply_plan_results::record_backend_apply_plan_restore_entry(
            connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: token_record.run_id,
                apply_plan_result_id: Some(move_result.id),
                apply_plan_item_id: Some(operation.item_id),
                original_source_path: source_path.to_string_lossy().to_string(),
                destination_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
                backup_path: Some(backup.backup_path.to_string_lossy().to_string()),
                file_hash_before: Some(source_hash.clone()),
                file_size_before: Some(source_size as i64),
                operation_kind: "hidden_real_move_only".to_owned(),
                operation_result_status: "applied".to_owned(),
                restore_status: "not_restored".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )?;

        executed_operations.push(HiddenRealMoveOperationSuccess {
            item_id: operation.item_id,
            source_path,
            destination_path,
            backup_path: backup.backup_path,
            source_hash,
            destination_hash,
            backup_hash: backup.backup_hash,
            source_size,
            destination_size,
            backup_size: backup.backup_size,
            backup_verified: backup.backup_verified,
            move_verified: true,
            backup_result_log_id: backup.result_log_id,
            move_result_log_id: move_result.id,
            restore_entry_id: restore_entry.id,
        });
    }

    apply_plan_results::mark_apply_plan_run_applied(
        connection,
        token_record.run_id,
        executed_operations.len() as i64,
        0,
        0,
        "Hidden feature-gated real move executor completed backend-observed moves.",
    )?;

    Ok(HiddenRealMoveExecutorOutcome {
        feature_gate_enabled: true,
        plan_id: request.plan_id,
        token_run_id: token_record.run_id,
        operation_set_hash: token_record.operation_set_hash,
        executed_operations,
        caveats: vec![
            "Hidden real move executor spike is compiled behind apply-executor-real-move-spike."
                .to_owned(),
            "No public Tauri command is registered; UI Apply remains locked.".to_owned(),
            "Only move operations from backend-owned operation previews are attempted.".to_owned(),
            "Delete, quarantine, replace, update replacement, folder creation, and AI-only actions remain blocked.".to_owned(),
        ],
    })
}

pub(crate) fn run_hidden_run_scoped_restore_spike(
    connection: &Connection,
    settings: &LibrarySettings,
    request: HiddenRunScopedRestoreRequest,
) -> AppResult<HiddenRunScopedRestoreOutcome> {
    if !request.real_move_feature_gate_enabled {
        return Err(AppError::Message(
            "Hidden run-scoped restore spike requires the runtime feature gate to be enabled."
                .to_owned(),
        ));
    }
    if request.restore_entry_ids.is_empty() {
        return Err(AppError::Message(
            "Hidden run-scoped restore spike requires backend restore entry ids.".to_owned(),
        ));
    }

    let token = require_trimmed(&request.confirmation_token, "confirmationToken")?;
    validate_run_confirmation_token(connection, request.apply_plan_run_id, &token)?;
    let roots = ConfiguredRoots::from_settings(settings)?;
    let backup_root = canonicalize_existing_dir(&request.backup_root, "backupRoot")?;

    let mut restored_entries = Vec::with_capacity(request.restore_entry_ids.len());
    for restore_entry_id in request.restore_entry_ids {
        let entry =
            load_hidden_restore_entry(connection, request.apply_plan_run_id, restore_entry_id)?;
        if entry.operation_kind != "hidden_real_move_only" {
            return Err(AppError::Message(
                "Hidden run-scoped restore only accepts backend real-move restore entries."
                    .to_owned(),
            ));
        }
        if entry.restore_status != "not_restored" {
            return Err(AppError::Message(
                "Hidden run-scoped restore only accepts not_restored entries.".to_owned(),
            ));
        }
        let destination_path = entry
            .destination_path_at_execution
            .as_deref()
            .ok_or_else(|| {
                AppError::Message(
                    "Hidden run-scoped restore entry is missing destination path metadata."
                        .to_owned(),
                )
            })?;
        let backup_path = entry.backup_path.as_deref().ok_or_else(|| {
            AppError::Message(
                "Hidden run-scoped restore entry is missing backup path metadata.".to_owned(),
            )
        })?;
        let file_hash_before = entry.file_hash_before.as_deref().ok_or_else(|| {
            AppError::Message(
                "Hidden run-scoped restore entry is missing pre-move hash metadata.".to_owned(),
            )
        })?;
        let file_size_before = entry.file_size_before.ok_or_else(|| {
            AppError::Message(
                "Hidden run-scoped restore entry is missing pre-move size metadata.".to_owned(),
            )
        })?;
        if file_size_before < 0 {
            return Err(AppError::Message(
                "Hidden run-scoped restore entry has invalid size metadata.".to_owned(),
            ));
        }
        let file_size_before = file_size_before as u64;

        let destination_path =
            canonicalize_existing_file(Path::new(destination_path), "restore destinationPath")?;
        let destination_root = roots.matching_root(&destination_path, "restore destinationPath")?;
        let original_source_path = resolve_new_file_under_configured_root(
            Path::new(&entry.original_source_path),
            &roots,
            destination_root,
            "restore originalSourcePath",
        )?;
        let backup_path = canonicalize_existing_file(Path::new(backup_path), "restore backupPath")?;
        ensure_under_root(&backup_path, &backup_root, "restore backupPath")?;

        let destination_hash = hash_file(&destination_path)?;
        let destination_size = fs::metadata(&destination_path)?.len();
        if destination_hash != file_hash_before || destination_size != file_size_before {
            return Err(AppError::Message(
                "Hidden run-scoped restore refused because destination bytes no longer match the backend restore entry.".to_owned(),
            ));
        }
        let backup_hash = hash_file(&backup_path)?;
        let backup_size = fs::metadata(&backup_path)?.len();
        if backup_hash != file_hash_before || backup_size != file_size_before {
            return Err(AppError::Message(
                "Hidden run-scoped restore refused because backup bytes no longer match the backend restore entry.".to_owned(),
            ));
        }

        fs::rename(&destination_path, &original_source_path).map_err(|error| {
            AppError::Message(format!(
                "Hidden run-scoped restore failed while moving destination back to the original source path: {error}"
            ))
        })?;
        if destination_path.exists() {
            return Err(AppError::Message(
                "Hidden run-scoped restore verification failed: destination still exists after restore.".to_owned(),
            ));
        }
        let restored_hash = hash_file(&original_source_path)?;
        let restored_size = fs::metadata(&original_source_path)?.len();
        if restored_hash != file_hash_before || restored_size != file_size_before {
            return Err(AppError::Message(
                "Hidden run-scoped restore verification failed after restore move.".to_owned(),
            ));
        }

        let result = apply_plan_results::record_backend_apply_plan_result_log(
            connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: request.apply_plan_run_id,
                apply_plan_item_id: entry.apply_plan_item_id,
                operation_kind: "hidden_real_restore_move_only".to_owned(),
                result_status: "restored".to_owned(),
                source_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
                destination_path_at_execution: Some(original_source_path.to_string_lossy().to_string()),
                backup_path: Some(backup_path.to_string_lossy().to_string()),
                error_code: None,
                error_message: None,
                user_summary: "Hidden feature-gated run-scoped restore moved a backend-recorded destination back to its original source path. Not exposed to the UI.".to_owned(),
            },
        )?;

        apply_plan_results::mark_apply_plan_restore_entry_restored(connection, entry.id)?;

        restored_entries.push(HiddenRunScopedRestoreSuccess {
            restore_entry_id: entry.id,
            result_log_id: result.id,
            item_id: entry.apply_plan_item_id,
            original_source_path,
            destination_path,
            backup_path,
            restored_hash,
            restored_size,
        });
    }

    apply_plan_results::mark_apply_plan_run_restored(
        connection,
        request.apply_plan_run_id,
        restored_entries.len() as i64,
        "Hidden feature-gated run-scoped restore completed backend-observed restore entries.",
    )?;

    Ok(HiddenRunScopedRestoreOutcome {
        feature_gate_enabled: true,
        apply_plan_run_id: request.apply_plan_run_id,
        restored_entries,
        caveats: vec![
            "Hidden run-scoped restore spike is compiled behind apply-executor-real-move-spike."
                .to_owned(),
            "No public Tauri command is registered; UI Restore remains locked.".to_owned(),
            "Restore paths are loaded only from backend restore entries for the requested run.".to_owned(),
            "Restore status rows are updated from backend-observed restore entries after byte verification.".to_owned(),
        ],
    })
}

struct BackupProof {
    backup_path: PathBuf,
    backup_hash: String,
    backup_size: u64,
    backup_verified: bool,
    result_log_id: i64,
}

fn create_backup_first(
    connection: &Connection,
    run_id: i64,
    item_id: i64,
    backup_root: &Path,
    source_path: &Path,
    source_hash: &str,
    source_size: u64,
) -> AppResult<BackupProof> {
    let file_name = source_path.file_name().ok_or_else(|| {
        AppError::Message("sourcePath must point to a named file before backup.".to_owned())
    })?;
    let backup_dir = backup_root
        .join(format!("run-{run_id}"))
        .join(format!("item-{item_id}"));
    let backup_path = backup_dir.join(file_name);
    ensure_under_root(&backup_dir, backup_root, "backup destination")?;
    ensure_under_root(&backup_path, backup_root, "backup destination")?;
    if backup_path.exists() {
        return Err(AppError::Message(
            "Hidden real backup destination already exists; no overwrite was attempted.".to_owned(),
        ));
    }

    fs::create_dir_all(&backup_dir).map_err(|error| {
        AppError::Message(format!(
            "Hidden real backup directory could not be created: {error}"
        ))
    })?;
    fs::copy(source_path, &backup_path).map_err(|error| {
        AppError::Message(format!(
            "Hidden real backup copy failed before source changes: {error}"
        ))
    })?;
    let backup_hash = hash_file(&backup_path)?;
    let backup_size = fs::metadata(&backup_path)?.len();
    if backup_hash != source_hash || backup_size != source_size {
        return Err(AppError::Message(
            "Hidden real backup verification failed before move; source was not changed."
                .to_owned(),
        ));
    }

    let backup_result = apply_plan_results::record_backend_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: run_id,
            apply_plan_item_id: Some(item_id),
            operation_kind: "hidden_real_backup_before_move".to_owned(),
            result_status: "applied".to_owned(),
            source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
            destination_path_at_execution: None,
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            error_code: None,
            error_message: None,
            user_summary: "Hidden real executor copied and verified backup before moving."
                .to_owned(),
        },
    )?;
    apply_plan_results::record_backend_apply_plan_restore_entry(
        connection,
        RecordApplyPlanRestoreEntryRequest {
            apply_plan_run_id: run_id,
            apply_plan_result_id: Some(backup_result.id),
            apply_plan_item_id: Some(item_id),
            original_source_path: source_path.to_string_lossy().to_string(),
            destination_path_at_execution: None,
            backup_path: Some(backup_path.to_string_lossy().to_string()),
            file_hash_before: Some(source_hash.to_owned()),
            file_size_before: Some(source_size as i64),
            operation_kind: "hidden_real_backup_before_move".to_owned(),
            operation_result_status: "applied".to_owned(),
            restore_status: "not_restored".to_owned(),
            restore_error_code: None,
            restore_error_message: None,
        },
    )?;

    Ok(BackupProof {
        backup_path,
        backup_hash,
        backup_size,
        backup_verified: true,
        result_log_id: backup_result.id,
    })
}

fn mark_run_apply_failed_after_verified_backup(
    connection: &Connection,
    run_id: i64,
    completed_moves: i64,
    operation_count: i64,
) -> AppResult<()> {
    let skipped_items = operation_count
        .saturating_sub(completed_moves)
        .saturating_sub(1);
    apply_plan_results::mark_apply_plan_run_applied(
        connection,
        run_id,
        completed_moves,
        1,
        skipped_items,
        "Hidden feature-gated real move executor failed after verified backup; backend recovery metadata is available.",
    )?;
    Ok(())
}

struct FailedMoveAfterBackupRecord<'a> {
    run_id: i64,
    item_id: i64,
    source_path: &'a Path,
    destination_path: &'a Path,
    backup: &'a BackupProof,
    source_hash: &'a str,
    source_size: u64,
    error_message: String,
}

fn record_failed_move_after_backup(
    connection: &Connection,
    record: FailedMoveAfterBackupRecord<'_>,
) -> AppResult<()> {
    let failed_result = apply_plan_results::record_backend_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: record.run_id,
            apply_plan_item_id: Some(record.item_id),
            operation_kind: "hidden_real_move_failed_after_backup".to_owned(),
            result_status: "failed_after_change".to_owned(),
            source_path_at_execution: Some(record.source_path.to_string_lossy().to_string()),
            destination_path_at_execution: Some(record.destination_path.to_string_lossy().to_string()),
            backup_path: Some(record.backup.backup_path.to_string_lossy().to_string()),
            error_code: Some("hidden_real_move_rename_failed_after_backup".to_owned()),
            error_message: Some(record.error_message),
            user_summary: "Hidden feature-gated real move failed after verified backup and before a completed source move. Backend recovery metadata was preserved.".to_owned(),
        },
    )?;
    apply_plan_results::record_backend_apply_plan_restore_entry(
        connection,
        RecordApplyPlanRestoreEntryRequest {
            apply_plan_run_id: record.run_id,
            apply_plan_result_id: Some(failed_result.id),
            apply_plan_item_id: Some(record.item_id),
            original_source_path: record.source_path.to_string_lossy().to_string(),
            destination_path_at_execution: Some(
                record.destination_path.to_string_lossy().to_string(),
            ),
            backup_path: Some(record.backup.backup_path.to_string_lossy().to_string()),
            file_hash_before: Some(record.source_hash.to_owned()),
            file_size_before: Some(record.source_size as i64),
            operation_kind: "hidden_real_move_failed_after_backup".to_owned(),
            operation_result_status: "failed_after_change".to_owned(),
            restore_status: "not_restored".to_owned(),
            restore_error_code: Some("move_failed_after_backup".to_owned()),
            restore_error_message: Some(
                "Backup was verified and retained; source path should be audited before retry."
                    .to_owned(),
            ),
        },
    )?;
    Ok(())
}

struct ConfiguredRoots {
    mods_root: Option<PathBuf>,
    tray_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RootKind {
    Mods,
    Tray,
}

impl ConfiguredRoots {
    fn from_settings(settings: &LibrarySettings) -> AppResult<Self> {
        Ok(Self {
            mods_root: settings
                .mods_path
                .as_deref()
                .map(Path::new)
                .map(|path| canonicalize_existing_dir(path, "modsPath"))
                .transpose()?,
            tray_root: settings
                .tray_path
                .as_deref()
                .map(Path::new)
                .map(|path| canonicalize_existing_dir(path, "trayPath"))
                .transpose()?,
        })
    }

    fn matching_root(&self, path: &Path, field_name: &str) -> AppResult<RootKind> {
        if self
            .mods_root
            .as_ref()
            .is_some_and(|root| path.starts_with(root))
        {
            return Ok(RootKind::Mods);
        }
        if self
            .tray_root
            .as_ref()
            .is_some_and(|root| path.starts_with(root))
        {
            return Ok(RootKind::Tray);
        }
        Err(AppError::Message(format!(
            "{field_name} must stay inside a configured Mods or Tray root."
        )))
    }

    fn root_for(&self, kind: RootKind) -> Option<&PathBuf> {
        match kind {
            RootKind::Mods => self.mods_root.as_ref(),
            RootKind::Tray => self.tray_root.as_ref(),
        }
    }
}

fn load_token_record(
    connection: &Connection,
    plan_id: i64,
    confirmation_token: &str,
) -> AppResult<TokenRecord> {
    let row = connection
        .query_row(
            "SELECT id, summary FROM apply_plan_runs
             WHERE apply_plan_id = ?1 AND confirmation_token = ?2 AND status != 'cancelled'
             ORDER BY id DESC
             LIMIT 1",
            params![plan_id, confirmation_token],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let Some((run_id, summary)) = row else {
        return Err(AppError::Message(
            "Backend-issued confirmation token was not found for this ApplyPlan.".to_owned(),
        ));
    };
    let summary = summary.ok_or_else(|| {
        AppError::Message("Confirmation token metadata summary is missing.".to_owned())
    })?;
    let summary: Value = serde_json::from_str(&summary).map_err(|error| {
        AppError::Message(format!(
            "Confirmation token metadata summary is malformed: {error}"
        ))
    })?;
    if summary.get("kind").and_then(Value::as_str) != Some("confirmation_token_v1") {
        return Err(AppError::Message(
            "Confirmation token metadata is not confirmation_token_v1.".to_owned(),
        ));
    }
    let plan_hash = required_summary_string(&summary, "planHash")?;
    let operation_set_hash = required_summary_string(&summary, "operationSetHash")?;
    let allowed_operation_count = summary
        .get("allowedOperationCount")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            AppError::Message(
                "Confirmation token metadata is missing allowedOperationCount.".to_owned(),
            )
        })?;
    if allowed_operation_count <= 0 {
        return Err(AppError::Message(
            "Confirmation token metadata does not allow any operations.".to_owned(),
        ));
    }
    Ok(TokenRecord {
        run_id,
        plan_hash,
        operation_set_hash,
        allowed_operation_count,
    })
}

struct HiddenRestoreEntry {
    id: i64,
    apply_plan_item_id: Option<i64>,
    original_source_path: String,
    destination_path_at_execution: Option<String>,
    backup_path: Option<String>,
    file_hash_before: Option<String>,
    file_size_before: Option<i64>,
    operation_kind: String,
    restore_status: String,
}

fn validate_run_confirmation_token(
    connection: &Connection,
    run_id: i64,
    confirmation_token: &str,
) -> AppResult<()> {
    let stored_token: Option<String> = connection
        .query_row(
            "SELECT confirmation_token FROM apply_plan_runs
             WHERE id = ?1 AND status != 'cancelled'",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Message(format!("ApplyPlan run {run_id} was not found.")))?;
    let stored_token = stored_token.ok_or_else(|| {
        AppError::Message("ApplyPlan run does not have a backend confirmation token.".to_owned())
    })?;
    if stored_token != confirmation_token {
        return Err(AppError::Message(
            "Confirmation token does not match the requested ApplyPlan run.".to_owned(),
        ));
    }
    Ok(())
}

fn load_hidden_restore_entry(
    connection: &Connection,
    run_id: i64,
    restore_entry_id: i64,
) -> AppResult<HiddenRestoreEntry> {
    connection
        .query_row(
            "SELECT id, apply_plan_item_id, original_source_path, destination_path_at_execution,
                backup_path, file_hash_before, file_size_before, operation_kind, restore_status
             FROM apply_plan_restore_entries
             WHERE id = ?1 AND apply_plan_run_id = ?2",
            params![restore_entry_id, run_id],
            |row| {
                Ok(HiddenRestoreEntry {
                    id: row.get(0)?,
                    apply_plan_item_id: row.get(1)?,
                    original_source_path: row.get(2)?,
                    destination_path_at_execution: row.get(3)?,
                    backup_path: row.get(4)?,
                    file_hash_before: row.get(5)?,
                    file_size_before: row.get(6)?,
                    operation_kind: row.get(7)?,
                    restore_status: row.get(8)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            AppError::Message(
                "Backend restore entry was not found for the requested ApplyPlan run.".to_owned(),
            )
        })
}

fn token_already_has_results(connection: &Connection, run_id: i64) -> AppResult<bool> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM apply_plan_results WHERE apply_plan_run_id = ?1",
        [run_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn required_summary_string(summary: &Value, key: &str) -> AppResult<String> {
    summary
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::Message(format!("Confirmation token metadata is missing {key}.")))
}

fn canonicalize_existing_dir(path: &Path, field_name: &str) -> AppResult<PathBuf> {
    let canonical = path.canonicalize().map_err(|error| {
        AppError::Message(format!("{field_name} could not be canonicalized: {error}"))
    })?;
    if !canonical.is_dir() {
        return Err(AppError::Message(format!(
            "{field_name} must be an existing directory."
        )));
    }
    Ok(canonical)
}

fn canonicalize_existing_file(path: &Path, field_name: &str) -> AppResult<PathBuf> {
    let canonical = path.canonicalize().map_err(|error| {
        AppError::Message(format!("{field_name} could not be canonicalized: {error}"))
    })?;
    if !canonical.is_file() {
        return Err(AppError::Message(format!(
            "{field_name} must be an existing file."
        )));
    }
    Ok(canonical)
}

fn resolve_new_file_under_configured_root(
    path: &Path,
    roots: &ConfiguredRoots,
    expected_root: RootKind,
    field_name: &str,
) -> AppResult<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Message(format!("{field_name} must include a destination parent."))
    })?;
    let canonical_parent = canonicalize_existing_dir(parent, &format!("{field_name} parent"))?;
    let expected_root_path = roots.root_for(expected_root).ok_or_else(|| {
        AppError::Message(
            "Expected configured root is missing during destination validation.".to_owned(),
        )
    })?;
    ensure_under_root(
        &canonical_parent,
        expected_root_path,
        &format!("{field_name} parent"),
    )?;
    let actual_root = roots.matching_root(&canonical_parent, &format!("{field_name} parent"))?;
    if actual_root != expected_root {
        return Err(AppError::Message(
            "Hidden real move executor spike will not perform cross-root moves.".to_owned(),
        ));
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| AppError::Message(format!("{field_name} must point to a named file.")))?;
    let candidate = canonical_parent.join(file_name);
    ensure_under_root(&candidate, expected_root_path, field_name)?;
    if candidate.exists() {
        return Err(AppError::Message(format!(
            "{field_name} already exists; hidden real executor will not overwrite."
        )));
    }
    Ok(candidate)
}

fn ensure_under_root(path: &Path, root: &Path, field_name: &str) -> AppResult<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "{field_name} must stay inside its configured root."
        )))
    }
}

fn require_trimmed(value: &str, field_name: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(AppError::Message(format!("{field_name} is required.")))
    } else {
        Ok(trimmed.to_owned())
    }
}

fn hash_file(path: &Path) -> AppResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use rusqlite::{params, Connection};
    use tempfile::tempdir;

    use crate::{
        core::{
            apply_plan_confirmation_token, apply_plan_operation_preview,
            apply_plan_persistence::{
                generate_sorting_preview_snapshot, get_apply_plan,
                save_apply_plan_from_preview_snapshot,
            },
        },
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope,
            IssueApplyPlanConfirmationTokenRequest, LibrarySettings,
            PreviewApplyPlanOperationsRequest, SaveApplyPlanFromPreviewSnapshotRequest,
        },
    };

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory connection");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn path_string(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    fn file_name(path: &Path) -> String {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sample.package")
            .to_owned()
    }

    fn insert_file(connection: &Connection, file_id: i64, path: &Path, source_location: &str) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES (?1, ?2, ?3, 'package', 1, '2026-01-01', ?4, 'CAS', 'Hair', 0.95, '[]', '[]', ?5, 1)",
                params![file_id, path_string(path), file_name(path), format!("h{file_id}"), source_location],
            )
            .expect("insert file");
    }

    fn save_backend_generated_plan(connection: &mut Connection, settings: &LibrarySettings) -> i64 {
        let preview = generate_sorting_preview_snapshot(
            connection,
            settings,
            GenerateSortingPreviewPlanRequest {
                scope: GenerateSortingPreviewPlanScope::SelectedFiles { file_ids: vec![70] },
                folder_config: None,
                context_trail: Vec::new(),
            },
        )
        .expect("generate backend preview snapshot");

        save_apply_plan_from_preview_snapshot(
            connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: preview.preview_snapshot_id,
                preview_snapshot_hash: preview.preview_snapshot_hash,
            },
        )
        .expect("save backend generated plan")
        .plan_id
    }

    #[test]
    fn hidden_real_move_executor_requires_runtime_gate_before_file_or_result_changes() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let backup_root = temp.path().join("RealExecutorBackups");
        let source_path = mods_root.join("gate-required.package");
        fs::create_dir_all(mods_root.join("CAS")).expect("destination parent");
        fs::create_dir_all(&backup_root).expect("backup root");
        fs::write(&source_path, b"must not move without runtime gate").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &source_path, "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let plan = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan");
        let operation_preview = apply_plan_operation_preview::preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
            },
        )
        .expect("operation preview");
        let token = apply_plan_confirmation_token::issue_apply_plan_confirmation_token(
            &connection,
            &settings,
            IssueApplyPlanConfirmationTokenRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
            },
        )
        .expect("token");

        let error = super::run_hidden_real_move_executor_spike(
            &connection,
            &settings,
            super::HiddenRealMoveExecutorRequest {
                real_move_feature_gate_enabled: false,
                plan_id,
                confirmation_token: token.token,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash),
                backup_root,
                force_rename_failure_after_backup: false,
            },
        )
        .expect_err("hidden real executor must fail closed without runtime gate");

        assert!(error.to_string().contains("feature gate"));
        assert_eq!(
            fs::read(&source_path).expect("source bytes"),
            b"must not move without runtime gate"
        );
        let result_rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM apply_plan_results", [], |row| {
                row.get(0)
            })
            .expect("count result rows");
        assert_eq!(result_rows, 0);
    }

    #[test]
    fn hidden_real_move_executor_moves_only_with_token_hash_rerun_validation_and_backup_first() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let backup_root = temp.path().join("RealExecutorBackups");
        let source_path = mods_root.join("real-move-spike.package");
        let destination_path = mods_root.join("CAS").join("real-move-spike.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::create_dir_all(&backup_root).expect("backup root");
        fs::write(&source_path, b"hidden real move spike bytes").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &source_path, "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let plan = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan");
        let operation_preview = apply_plan_operation_preview::preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
            },
        )
        .expect("operation preview");
        let token = apply_plan_confirmation_token::issue_apply_plan_confirmation_token(
            &connection,
            &settings,
            IssueApplyPlanConfirmationTokenRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
            },
        )
        .expect("token");

        let outcome = super::run_hidden_real_move_executor_spike(
            &connection,
            &settings,
            super::HiddenRealMoveExecutorRequest {
                real_move_feature_gate_enabled: true,
                plan_id,
                confirmation_token: token.token.clone(),
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
                backup_root: backup_root.clone(),
                force_rename_failure_after_backup: false,
            },
        )
        .expect("hidden real move executor spike");

        assert!(outcome.feature_gate_enabled);
        assert_eq!(outcome.plan_id, plan_id);
        assert_eq!(outcome.token_run_id, token.token_id);
        assert_eq!(
            outcome.operation_set_hash,
            operation_preview.operation_set_hash
        );
        assert_eq!(outcome.executed_operations.len(), 1);
        assert!(outcome.executed_operations[0].backup_verified);
        assert!(outcome.executed_operations[0].move_verified);
        assert!(
            !source_path.exists(),
            "source should have moved after backup proof"
        );
        assert_eq!(
            fs::read(&destination_path).expect("destination bytes"),
            b"hidden real move spike bytes"
        );
        assert_eq!(
            fs::read(&outcome.executed_operations[0].backup_path).expect("backup bytes"),
            b"hidden real move spike bytes"
        );

        let run_status_and_counts: (String, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT status, total_items, applied_items, failed_items, restored_items
                 FROM apply_plan_runs
                 WHERE id = ?1",
                [token.token_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("run status and counters");
        assert_eq!(
            run_status_and_counts,
            ("applied".to_owned(), 1, 1, 0, 0),
            "hidden backend executor should mark the token run applied from observed results"
        );

        let result_rows: Vec<(String, String)> = connection
            .prepare("SELECT operation_kind, result_status FROM apply_plan_results ORDER BY id")
            .expect("prepare result rows")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query result rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect result rows");
        assert_eq!(
            result_rows,
            vec![
                (
                    "hidden_real_backup_before_move".to_owned(),
                    "applied".to_owned(),
                ),
                ("hidden_real_move_only".to_owned(), "applied".to_owned()),
            ]
        );

        let restore_statuses: Vec<String> = connection
            .prepare("SELECT restore_status FROM apply_plan_restore_entries ORDER BY id")
            .expect("prepare restore statuses")
            .query_map([], |row| row.get(0))
            .expect("query restore statuses")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect restore statuses");
        assert_eq!(restore_statuses, vec!["not_restored", "not_restored"]);

        let second_attempt_error = super::run_hidden_real_move_executor_spike(
            &connection,
            &settings,
            super::HiddenRealMoveExecutorRequest {
                real_move_feature_gate_enabled: true,
                plan_id,
                confirmation_token: token.token,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash),
                backup_root,
                force_rename_failure_after_backup: false,
            },
        )
        .expect_err("hidden real executor token must be single-use");
        assert!(second_attempt_error
            .to_string()
            .contains("already been used"));
    }

    #[test]
    fn hidden_real_move_executor_records_recovery_state_when_move_fails_after_verified_backup() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let backup_root = temp.path().join("RealExecutorBackups");
        let source_path = mods_root.join("rename-fails-after-backup.package");
        let destination_parent = mods_root.join("CAS");
        let destination_path = destination_parent.join("rename-fails-after-backup.package");
        fs::create_dir_all(&destination_parent).expect("destination parent");
        fs::create_dir_all(&backup_root).expect("backup root");
        fs::write(&source_path, b"backup exists before failed move").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &source_path, "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let plan = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan");
        let operation_preview = apply_plan_operation_preview::preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
            },
        )
        .expect("operation preview");
        let token = apply_plan_confirmation_token::issue_apply_plan_confirmation_token(
            &connection,
            &settings,
            IssueApplyPlanConfirmationTokenRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
            },
        )
        .expect("token");

        let error = super::run_hidden_real_move_executor_spike(
            &connection,
            &settings,
            super::HiddenRealMoveExecutorRequest {
                real_move_feature_gate_enabled: true,
                plan_id,
                confirmation_token: token.token.clone(),
                expected_plan_hash: plan.plan_hash,
                expected_operation_set_hash: Some(operation_preview.operation_set_hash),
                backup_root: backup_root.clone(),
                force_rename_failure_after_backup: true,
            },
        )
        .expect_err("rename must fail after backup proof");

        assert!(error.to_string().contains("after backup proof"));
        assert_eq!(
            fs::read(&source_path).expect("source remains after failed rename"),
            b"backup exists before failed move"
        );
        assert!(!destination_path.exists());
        let backup_paths: Vec<String> = connection
            .prepare("SELECT backup_path FROM apply_plan_results WHERE operation_kind = 'hidden_real_backup_before_move'")
            .expect("prepare backup paths")
            .query_map([], |row| row.get(0))
            .expect("query backup paths")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect backup paths");
        assert_eq!(backup_paths.len(), 1);
        assert_eq!(
            fs::read(&backup_paths[0]).expect("verified backup remains available"),
            b"backup exists before failed move"
        );

        let rows: Vec<(String, String, Option<String>)> = connection
            .prepare(
                "SELECT operation_kind, result_status, error_code
                 FROM apply_plan_results
                 ORDER BY id",
            )
            .expect("prepare result rows")
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .expect("query result rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect result rows");
        assert_eq!(
            rows,
            vec![
                (
                    "hidden_real_backup_before_move".to_owned(),
                    "applied".to_owned(),
                    None,
                ),
                (
                    "hidden_real_move_failed_after_backup".to_owned(),
                    "failed_after_change".to_owned(),
                    Some("hidden_real_move_rename_failed_after_backup".to_owned()),
                ),
            ]
        );

        let run_status_and_counts: (String, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT status, total_items, applied_items, failed_items, restored_items
                 FROM apply_plan_runs
                 WHERE id = ?1",
                [token.token_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("run status and counters after failed move");
        assert_eq!(
            run_status_and_counts,
            ("apply_failed".to_owned(), 1, 0, 1, 0),
            "hidden backend executor should mark failed-after-backup runs from observed result rows"
        );

        let restore_rows: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM apply_plan_restore_entries",
                [],
                |row| row.get(0),
            )
            .expect("count restore rows");
        assert_eq!(
            restore_rows, 2,
            "backup and failed move recovery rows exist"
        );
    }

    #[test]
    fn hidden_run_scoped_restore_uses_backend_restore_entry_without_path_input() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let backup_root = temp.path().join("RealExecutorBackups");
        let source_path = mods_root.join("restore-from-entry.package");
        let destination_path = mods_root.join("CAS").join("restore-from-entry.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::create_dir_all(&backup_root).expect("backup root");
        fs::write(&source_path, b"restore me from backend metadata").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &source_path, "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let plan = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan");
        let operation_preview = apply_plan_operation_preview::preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
            },
        )
        .expect("operation preview");
        let token = apply_plan_confirmation_token::issue_apply_plan_confirmation_token(
            &connection,
            &settings,
            IssueApplyPlanConfirmationTokenRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
            },
        )
        .expect("token");

        let move_outcome = super::run_hidden_real_move_executor_spike(
            &connection,
            &settings,
            super::HiddenRealMoveExecutorRequest {
                real_move_feature_gate_enabled: true,
                plan_id,
                confirmation_token: token.token.clone(),
                expected_plan_hash: plan.plan_hash,
                expected_operation_set_hash: Some(operation_preview.operation_set_hash),
                backup_root: backup_root.clone(),
                force_rename_failure_after_backup: false,
            },
        )
        .expect("hidden real move executor spike");
        assert!(!source_path.exists());
        assert!(destination_path.exists());

        let restore_entry_id = move_outcome.executed_operations[0].restore_entry_id;
        let restore_outcome = super::run_hidden_run_scoped_restore_spike(
            &connection,
            &settings,
            super::HiddenRunScopedRestoreRequest {
                real_move_feature_gate_enabled: true,
                apply_plan_run_id: token.token_id,
                confirmation_token: token.token,
                restore_entry_ids: vec![restore_entry_id],
                backup_root,
            },
        )
        .expect("hidden run-scoped restore");

        assert_eq!(restore_outcome.apply_plan_run_id, token.token_id);
        assert_eq!(restore_outcome.restored_entries.len(), 1);
        assert_eq!(
            restore_outcome.restored_entries[0].restore_entry_id,
            restore_entry_id
        );
        assert_eq!(
            fs::read(&source_path).expect("source restored from backend restore entry"),
            b"restore me from backend metadata"
        );
        assert!(!destination_path.exists());
        assert_eq!(
            fs::read(&restore_outcome.restored_entries[0].backup_path)
                .expect("backup retained after restore"),
            b"restore me from backend metadata"
        );

        let run_status_and_counts: (String, i64, i64, i64, i64) = connection
            .query_row(
                "SELECT status, total_items, applied_items, failed_items, restored_items
                 FROM apply_plan_runs
                 WHERE id = ?1",
                [token.token_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .expect("run status and counters after restore");
        assert_eq!(
            run_status_and_counts,
            ("restored".to_owned(), 1, 1, 0, 1),
            "hidden backend restore should mark only restored backend entries and keep apply counters"
        );

        let restored_entry_status: String = connection
            .query_row(
                "SELECT restore_status
                 FROM apply_plan_restore_entries
                 WHERE id = ?1",
                [restore_entry_id],
                |row| row.get(0),
            )
            .expect("restore entry status after backend restore");
        assert_eq!(restored_entry_status, "restored");
    }
}
