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
    core::{
        apply_plan_backup_prototype::{
            run_fixture_backup_prototype, FixtureBackupPrototypeOutcome,
            FixtureBackupPrototypeRequest,
        },
        apply_plan_operation_preview, apply_plan_results,
    },
    error::{AppError, AppResult},
    models::{
        LibrarySettings, PreviewApplyPlanOperationsRequest, RecordApplyPlanRestoreEntryRequest,
        RecordApplyPlanResultLogRequest,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct FixtureApplyExecutorPrototypeRequest {
    pub fixture_mode: bool,
    pub plan_id: i64,
    pub confirmation_token: String,
    pub expected_plan_hash: Option<String>,
    pub expected_operation_set_hash: Option<String>,
    pub fixture_root: PathBuf,
    pub backup_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureApplyExecutorPrototypeOutcome {
    pub fixture_mode: bool,
    pub can_execute_real_files: bool,
    pub plan_id: i64,
    pub token_run_id: i64,
    pub operation_set_hash: String,
    pub executed_operations: Vec<FixtureApplyExecutorOperationSuccess>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureApplyExecutorOperationSuccess {
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

struct TokenRecord {
    run_id: i64,
    plan_hash: String,
    operation_set_hash: String,
    allowed_operation_count: i64,
}

pub(crate) fn run_fixture_apply_executor_prototype(
    connection: &Connection,
    settings: &LibrarySettings,
    request: FixtureApplyExecutorPrototypeRequest,
) -> AppResult<FixtureApplyExecutorPrototypeOutcome> {
    if !request.fixture_mode {
        return Err(AppError::Message(
            "Fixture Apply executor prototype requires fixtureMode=true.".to_owned(),
        ));
    }

    let token = require_trimmed(&request.confirmation_token, "confirmationToken")?;
    let fixture_root = canonicalize_existing_dir(&request.fixture_root, "fixtureRoot")?;
    fs::create_dir_all(&request.backup_root).map_err(|error| {
        AppError::Message(format!("Fixture backup root could not be created: {error}"))
    })?;
    let backup_root = canonicalize_existing_dir(&request.backup_root, "backupRoot")?;
    ensure_under_root(&backup_root, &fixture_root, "backupRoot")?;

    let token_record = load_token_record(connection, request.plan_id, &token)?;
    if token_already_has_results(connection, token_record.run_id)? {
        return Err(AppError::Message(
            "Confirmation token has already been used by the fixture executor prototype."
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
            "Current operation-set hash no longer matches the backend-issued confirmation token. Reload and re-confirm before any fixture execution proof.".to_owned(),
        ));
    }
    if operation_preview.operations.is_empty() {
        return Err(AppError::Message(
            "Fixture Apply executor prototype requires at least one backend-owned operation candidate.".to_owned(),
        ));
    }
    if operation_preview.operations.len() as i64 != token_record.allowed_operation_count {
        return Err(AppError::Message(
            "Current operation count no longer matches the backend-issued confirmation token."
                .to_owned(),
        ));
    }

    let mut executed_operations = Vec::with_capacity(operation_preview.operations.len());
    for operation in operation_preview.operations {
        let source_path = operation.source_path.as_deref().ok_or_else(|| {
            AppError::Message("Fixture operation is missing a source path.".to_owned())
        })?;
        let destination_path = operation.destination_path.as_deref().ok_or_else(|| {
            AppError::Message("Fixture operation is missing a destination path.".to_owned())
        })?;
        let source_path = canonicalize_existing_file(Path::new(source_path), "sourcePath")?;
        ensure_under_root(&source_path, &fixture_root, "sourcePath")?;
        let destination_path = resolve_new_file_under_root(
            Path::new(destination_path),
            &fixture_root,
            "destinationPath",
        )?;
        if source_path == destination_path {
            return Err(AppError::Message(
                "Fixture source and destination must be different paths.".to_owned(),
            ));
        }

        let source_hash = hash_file(&source_path)?;
        let source_size = fs::metadata(&source_path)?.len();
        let backup_outcome = run_fixture_backup_prototype(
            connection,
            FixtureBackupPrototypeRequest {
                fixture_mode: true,
                apply_plan_id: request.plan_id,
                apply_plan_item_id: Some(operation.item_id),
                run_id: token_record.run_id,
                fixture_root: fixture_root.clone(),
                source_path: source_path.clone(),
                backup_root: backup_root.clone(),
                operation_kind: "fixture_backup_before_move".to_owned(),
            },
        )?;
        let backup_success = match backup_outcome {
            FixtureBackupPrototypeOutcome::Verified(success) => success,
            FixtureBackupPrototypeOutcome::FailedBeforeChange(failure) => {
                return Err(AppError::Message(format!(
                    "Fixture backup failed before move: {} ({})",
                    failure.error_message, failure.error_code
                )));
            }
        };

        fs::rename(&source_path, &destination_path).map_err(|error| {
            AppError::Message(format!("Fixture move failed after backup proof: {error}"))
        })?;
        if source_path.exists() {
            return Err(AppError::Message(
                "Fixture move verification failed: source still exists after move.".to_owned(),
            ));
        }
        let destination_hash = hash_file(&destination_path)?;
        let destination_size = fs::metadata(&destination_path)?.len();
        if destination_hash != source_hash || destination_size != source_size {
            return Err(AppError::Message(
                "Fixture move verification failed after temporary test-file move.".to_owned(),
            ));
        }

        let move_result = apply_plan_results::record_apply_plan_result_log(
            connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: token_record.run_id,
                apply_plan_item_id: Some(operation.item_id),
                operation_kind: "fixture_move_only".to_owned(),
                result_status: "pending_log".to_owned(),
                source_path_at_execution: Some(source_path.to_string_lossy().to_string()),
                destination_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
                backup_path: Some(backup_success.backup_path.to_string_lossy().to_string()),
                error_code: None,
                error_message: None,
                user_summary:
                    "Fixture move completed inside a temporary fixture root. No user files changed."
                        .to_owned(),
            },
        )?;
        let restore_entry = apply_plan_results::record_apply_plan_restore_entry(
            connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: token_record.run_id,
                apply_plan_result_id: Some(move_result.id),
                apply_plan_item_id: Some(operation.item_id),
                original_source_path: source_path.to_string_lossy().to_string(),
                destination_path_at_execution: Some(destination_path.to_string_lossy().to_string()),
                backup_path: Some(backup_success.backup_path.to_string_lossy().to_string()),
                file_hash_before: Some(source_hash.clone()),
                file_size_before: Some(source_size as i64),
                operation_kind: "fixture_move_only".to_owned(),
                operation_result_status: "pending_log".to_owned(),
                restore_status: "design_only".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )?;

        executed_operations.push(FixtureApplyExecutorOperationSuccess {
            item_id: operation.item_id,
            source_path,
            destination_path,
            backup_path: backup_success.backup_path,
            source_hash,
            destination_hash,
            backup_hash: backup_success.backup_hash,
            source_size,
            destination_size,
            backup_size: backup_success.backup_size,
            backup_verified: backup_success.backup_verified,
            move_verified: true,
            backup_result_log_id: backup_success.result_log_id,
            move_result_log_id: move_result.id,
            restore_entry_id: restore_entry.id,
        });
    }

    Ok(FixtureApplyExecutorPrototypeOutcome {
        fixture_mode: true,
        can_execute_real_files: false,
        plan_id: request.plan_id,
        token_run_id: token_record.run_id,
        operation_set_hash: token_record.operation_set_hash,
        executed_operations,
        caveats: vec![
            "Hidden fixture-only executor prototype; no Tauri command is registered.".to_owned(),
            "All file changes are constrained to the caller-provided temporary fixture root."
                .to_owned(),
            "Result and restore rows remain prototype metadata and do not unlock real Apply."
                .to_owned(),
        ],
    })
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
            "{field_name} must be an existing fixture directory."
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
            "{field_name} must be an existing fixture file."
        )));
    }
    Ok(canonical)
}

fn resolve_new_file_under_root(path: &Path, root: &Path, field_name: &str) -> AppResult<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        AppError::Message(format!("{field_name} must include a destination parent."))
    })?;
    let canonical_parent = canonicalize_existing_dir(parent, &format!("{field_name} parent"))?;
    ensure_under_root(&canonical_parent, root, &format!("{field_name} parent"))?;
    let file_name = path.file_name().ok_or_else(|| {
        AppError::Message(format!("{field_name} must point to a named fixture file."))
    })?;
    let candidate = canonical_parent.join(file_name);
    ensure_under_root(&candidate, root, field_name)?;
    if candidate.exists() {
        return Err(AppError::Message(format!(
            "{field_name} already exists; fixture executor will not overwrite."
        )));
    }
    Ok(candidate)
}

fn ensure_under_root(path: &Path, root: &Path, field_name: &str) -> AppResult<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "{field_name} must stay inside the fixture root."
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
    fn fixture_executor_moves_only_temp_fixture_after_token_backup_and_hash_revalidation() {
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let mods_root = fixture_root.join("Mods");
        let backup_root = fixture_root.join("__fixture_backups");
        let source_path = mods_root.join("fixture-executor.package");
        let destination_path = mods_root.join("CAS").join("fixture-executor.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::create_dir_all(&backup_root).expect("backup root");
        fs::write(&source_path, b"fixture executor package bytes").expect("source file");

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

        let expected_source_path = source_path.canonicalize().expect("expected source path");
        let expected_destination_path = destination_path
            .parent()
            .expect("destination parent")
            .canonicalize()
            .expect("expected destination parent")
            .join(destination_path.file_name().expect("destination file name"));

        let outcome = super::run_fixture_apply_executor_prototype(
            &connection,
            &settings,
            super::FixtureApplyExecutorPrototypeRequest {
                fixture_mode: true,
                plan_id,
                confirmation_token: token.token.clone(),
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
                fixture_root: fixture_root.to_path_buf(),
                backup_root: backup_root.clone(),
            },
        )
        .expect("fixture executor");

        assert!(outcome.fixture_mode);
        assert!(!outcome.can_execute_real_files);
        assert_eq!(outcome.plan_id, plan_id);
        assert_eq!(outcome.executed_operations.len(), 1);
        assert_eq!(
            outcome.executed_operations[0].source_path,
            expected_source_path
        );
        assert_eq!(
            outcome.executed_operations[0].destination_path,
            expected_destination_path
        );
        assert!(outcome.executed_operations[0].backup_verified);
        assert!(outcome.executed_operations[0].move_verified);
        assert!(
            !source_path.exists(),
            "fixture source should move inside temp root"
        );
        assert_eq!(
            fs::read(&destination_path).expect("destination bytes"),
            b"fixture executor package bytes"
        );
        assert_eq!(
            fs::read(&outcome.executed_operations[0].backup_path).expect("backup bytes"),
            b"fixture executor package bytes"
        );

        let result_statuses: Vec<String> = connection
            .prepare("SELECT result_status FROM apply_plan_results ORDER BY id")
            .expect("prepare result statuses")
            .query_map([], |row| row.get(0))
            .expect("query result statuses")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect result statuses");
        assert_eq!(result_statuses, vec!["pending_log", "pending_log"]);
        for forbidden in ["applied", "restored", "moved", "copied", "restore_complete"] {
            assert!(!result_statuses.iter().any(|status| status == forbidden));
        }

        let second_attempt_error = super::run_fixture_apply_executor_prototype(
            &connection,
            &settings,
            super::FixtureApplyExecutorPrototypeRequest {
                fixture_mode: true,
                plan_id,
                confirmation_token: token.token,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash),
                fixture_root: fixture_root.to_path_buf(),
                backup_root,
            },
        )
        .expect_err("fixture token must be single-use");
        assert!(second_attempt_error
            .to_string()
            .contains("already been used"));
    }

    #[test]
    fn fixture_executor_rejects_non_fixture_mode_before_file_or_result_changes() {
        let temp = tempdir().expect("tempdir");
        let fixture_root = temp.path();
        let mods_root = fixture_root.join("Mods");
        let backup_root = fixture_root.join("__fixture_backups");
        let source_path = mods_root.join("reject-non-fixture.package");
        fs::create_dir_all(mods_root.join("CAS")).expect("destination parent");
        fs::create_dir_all(&backup_root).expect("backup root");
        fs::write(&source_path, b"must stay put").expect("source file");

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

        let error = super::run_fixture_apply_executor_prototype(
            &connection,
            &settings,
            super::FixtureApplyExecutorPrototypeRequest {
                fixture_mode: false,
                plan_id,
                confirmation_token: token.token,
                expected_plan_hash: plan.plan_hash.clone(),
                expected_operation_set_hash: Some(operation_preview.operation_set_hash),
                fixture_root: fixture_root.to_path_buf(),
                backup_root,
            },
        )
        .expect_err("non-fixture execution must fail");

        assert!(error.to_string().contains("fixtureMode=true"));
        assert_eq!(
            fs::read(&source_path).expect("source bytes"),
            b"must stay put"
        );
        let result_rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM apply_plan_results", [], |row| {
                row.get(0)
            })
            .expect("count result rows");
        assert_eq!(result_rows, 0);
    }
}
