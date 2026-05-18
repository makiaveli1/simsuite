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
    models::{RecordApplyPlanRestoreEntryRequest, RecordApplyPlanResultLogRequest},
};

const SUCCESS_SUMMARY: &str = "Fixture backup copied and verified. No user files changed.";

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

fn ensure_run_matches_plan(
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

fn canonicalize_existing_dir(path: &Path, field_name: &str) -> AppResult<PathBuf> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| AppError::Message(format!("{field_name} is not available: {error}")))?;
    if !canonical.is_dir() {
        return Err(AppError::Message(format!(
            "{field_name} must be a directory."
        )));
    }
    Ok(canonical)
}

fn resolve_fixture_candidate(path: &Path, field_name: &str) -> AppResult<PathBuf> {
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

fn ensure_under_root(path: &Path, root: &Path, field_name: &str) -> AppResult<()> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "{field_name} must stay inside the supplied fixture boundary."
        )))
    }
}

fn hash_file(path: &Path) -> AppResult<String> {
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
    use tempfile::tempdir;

    use crate::{
        core::apply_plan_results::{
            create_apply_plan_run_log, list_apply_plan_restore_entries, list_apply_plan_result_logs,
        },
        database,
        models::{
            ApplyPlanRestoreEntryStatus, ApplyPlanResultLogStatus, CreateApplyPlanRunLogRequest,
            ListApplyPlanRestoreEntriesRequest, ListApplyPlanResultLogsRequest,
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
    fn prototype_is_not_registered_as_a_tauri_command() {
        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("run_fixture_backup_prototype"));
        assert!(!commands_source.contains("fixture_backup_prototype"));
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
