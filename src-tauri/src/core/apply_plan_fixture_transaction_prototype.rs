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
    models::{
        ApplyPlanRestoreEntryStatus, ApplyPlanResultLogStatus, RecordApplyPlanRestoreEntryRequest,
        RecordApplyPlanResultLogRequest,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureFaultPoint {
    None,
    BeforeBackup,
    AfterVerifiedBackupBeforeMove,
    AfterMoveBeforeResult,
    AfterMoveResultBeforeRestoreEntry,
    UndoAfterRestoreBeforeCleanup,
}

fn injected_interruption(point: FixtureFaultPoint) -> AppError {
    AppError::Message(format!("Injected fixture interruption at {point:?}."))
}

#[derive(Debug, Clone)]
pub(crate) struct FixtureReconciliationRequest {
    pub fixture_mode: bool,
    pub apply_plan_id: i64,
    pub apply_plan_item_id: i64,
    pub run_id: i64,
    pub fixture_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FixtureReconciliationState {
    CleanStart,
    ResumeFromVerifiedBackupRequired,
    ExactHardLinkPairNeedsReview,
    MovedMissingResult,
    MoveResultMissingRestoreEntry,
    MoveRecordedAwaitingUndo,
    UndoCleanupPending,
    UndoComplete,
    Ambiguous(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObservedFileState {
    Missing,
    Exact,
    Different,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureReconciliationEvidence {
    source_path: PathBuf,
    destination_path: PathBuf,
    backup_path: PathBuf,
    expected_hash: String,
    expected_size: u64,
    backup_result_id: i64,
    backup_restore_entry_id: i64,
    move_result_id: Option<i64>,
}

#[derive(Debug)]
struct FixtureReconciliationAssessment {
    state: FixtureReconciliationState,
    evidence: Option<FixtureReconciliationEvidence>,
}

pub(crate) fn classify_fixture_reconciliation_state(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
) -> AppResult<FixtureReconciliationState> {
    Ok(assess_fixture_reconciliation_state(connection, request)?.state)
}

pub(crate) fn reconcile_fixture_transaction(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
) -> AppResult<FixtureReconciliationState> {
    let assessment = assess_fixture_reconciliation_state(connection, request)?;
    let state = assessment.state.clone();
    let Some(evidence) = assessment.evidence else {
        return Ok(state);
    };

    match state {
        FixtureReconciliationState::MovedMissingResult => {
            let move_result = apply_plan_results::record_apply_plan_result_log(
                connection,
                RecordApplyPlanResultLogRequest {
                    apply_plan_run_id: request.run_id,
                    apply_plan_item_id: Some(request.apply_plan_item_id),
                    operation_kind: MOVE_OPERATION_KIND.to_owned(),
                    result_status: "pending_log".to_owned(),
                    source_path_at_execution: Some(
                        evidence.source_path.to_string_lossy().to_string(),
                    ),
                    destination_path_at_execution: Some(
                        evidence.destination_path.to_string_lossy().to_string(),
                    ),
                    backup_path: Some(evidence.backup_path.to_string_lossy().to_string()),
                    error_code: None,
                    error_message: None,
                    user_summary: MOVE_SUCCESS_SUMMARY.to_owned(),
                },
            )?;
            record_move_restore_entry(connection, request, &evidence, move_result.id)?;
            classify_fixture_reconciliation_state(connection, request)
        }
        FixtureReconciliationState::MoveResultMissingRestoreEntry => {
            let move_result_id = evidence.move_result_id.ok_or_else(|| {
                AppError::Message(
                    "Fixture reconciliation lost the verified move-result identity.".to_owned(),
                )
            })?;
            record_move_restore_entry(connection, request, &evidence, move_result_id)?;
            classify_fixture_reconciliation_state(connection, request)
        }
        FixtureReconciliationState::UndoCleanupPending => {
            if observe_expected_file(
                &evidence.source_path,
                evidence.expected_size,
                &evidence.expected_hash,
            )? != ObservedFileState::Exact
                || observe_expected_file(
                    &evidence.destination_path,
                    evidence.expected_size,
                    &evidence.expected_hash,
                )? != ObservedFileState::Exact
            {
                return Ok(FixtureReconciliationState::Ambiguous(
                    "Fixture bytes changed before reconciliation cleanup.".to_owned(),
                ));
            }
            fs::remove_file(&evidence.destination_path).map_err(|error| {
                AppError::Message(format!(
                    "Fixture reconciliation could not remove the already-restored duplicate: {error}"
                ))
            })?;
            classify_fixture_reconciliation_state(connection, request)
        }
        _ => Ok(state),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureResumeRacePoint {
    None,
    DestinationAppearsAfterClassification,
    SourceChangesAfterClassification,
    BackupChangesAfterClassification,
    MoveMetadataAppearsAfterClassification,
}

pub(crate) fn resume_fixture_transaction_from_verified_backup(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
) -> AppResult<FixtureReconciliationState> {
    resume_fixture_transaction_from_verified_backup_with_race(
        connection,
        request,
        FixtureResumeRacePoint::None,
    )
}

fn resume_fixture_transaction_from_verified_backup_with_race(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    race_point: FixtureResumeRacePoint,
) -> AppResult<FixtureReconciliationState> {
    let initial_assessment = assess_fixture_reconciliation_state(connection, request)?;
    if initial_assessment.state != FixtureReconciliationState::ResumeFromVerifiedBackupRequired {
        return Ok(initial_assessment.state);
    }
    let initial_evidence = initial_assessment.evidence.ok_or_else(|| {
        AppError::Message(
            "Fixture resume lost the verified backup evidence required for execution.".to_owned(),
        )
    })?;

    inject_fixture_resume_race(connection, request, &initial_evidence, race_point)?;

    // Re-read the complete DB + filesystem evidence after the possible race and immediately
    // before the final mutation preflight. This intentionally does not trust the first snapshot.
    let final_assessment = assess_fixture_reconciliation_state(connection, request)?;
    if final_assessment.state != FixtureReconciliationState::ResumeFromVerifiedBackupRequired {
        return Ok(final_assessment.state);
    }
    let final_evidence = final_assessment.evidence.ok_or_else(|| {
        AppError::Message(
            "Fixture resume lost the final verified backup evidence required for execution."
                .to_owned(),
        )
    })?;
    if final_evidence != initial_evidence {
        return Ok(FixtureReconciliationState::Ambiguous(
            "Verified backup-chain evidence changed between resume classification and execution."
                .to_owned(),
        ));
    }

    // Keep these checks adjacent to the move primitive. They deliberately re-hash both source
    // and backup and require the destination to still be absent after the full reclassification.
    if observe_expected_file(
        &final_evidence.backup_path,
        final_evidence.expected_size,
        &final_evidence.expected_hash,
    )? != ObservedFileState::Exact
    {
        return Ok(FixtureReconciliationState::Ambiguous(
            "Verified backup bytes changed before resume execution.".to_owned(),
        ));
    }
    // Hash source after the backup check, then make destination absence the final filesystem
    // probe immediately before the move call. This avoids widening the no-overwrite race window
    // with another potentially expensive file hash after the destination check.
    if observe_expected_file(
        &final_evidence.source_path,
        final_evidence.expected_size,
        &final_evidence.expected_hash,
    )? != ObservedFileState::Exact
    {
        return Ok(FixtureReconciliationState::Ambiguous(
            "Fixture source bytes changed before resume execution.".to_owned(),
        ));
    }
    if observe_expected_file(
        &final_evidence.destination_path,
        final_evidence.expected_size,
        &final_evidence.expected_hash,
    )? != ObservedFileState::Missing
    {
        return Ok(FixtureReconciliationState::Ambiguous(
            "Fixture destination appeared before resume execution; no overwrite was attempted."
                .to_owned(),
        ));
    }

    if let Err(error) = move_engine::move_single_file(
        &final_evidence.source_path,
        &final_evidence.destination_path,
    ) {
        return Err(AppError::Message(format!(
            "Fixture resume move primitive failed; reconciliation must inspect the isolated fixture before another attempt: {error}"
        )));
    }

    if observe_expected_file(
        &final_evidence.source_path,
        final_evidence.expected_size,
        &final_evidence.expected_hash,
    )? != ObservedFileState::Missing
        || observe_expected_file(
            &final_evidence.destination_path,
            final_evidence.expected_size,
            &final_evidence.expected_hash,
        )? != ObservedFileState::Exact
    {
        return Err(AppError::Message(
            "Fixture resume move completed with an unexpected source/destination state."
                .to_owned(),
        ));
    }

    let move_result = apply_plan_results::record_apply_plan_result_log(
        connection,
        RecordApplyPlanResultLogRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_item_id: Some(request.apply_plan_item_id),
            operation_kind: MOVE_OPERATION_KIND.to_owned(),
            result_status: "pending_log".to_owned(),
            source_path_at_execution: Some(
                final_evidence.source_path.to_string_lossy().to_string(),
            ),
            destination_path_at_execution: Some(
                final_evidence.destination_path.to_string_lossy().to_string(),
            ),
            backup_path: Some(final_evidence.backup_path.to_string_lossy().to_string()),
            error_code: None,
            error_message: None,
            user_summary: MOVE_SUCCESS_SUMMARY.to_owned(),
        },
    )?;
    record_move_restore_entry(connection, request, &final_evidence, move_result.id)?;

    classify_fixture_reconciliation_state(connection, request)
}

fn inject_fixture_resume_race(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    evidence: &FixtureReconciliationEvidence,
    race_point: FixtureResumeRacePoint,
) -> AppResult<()> {
    match race_point {
        FixtureResumeRacePoint::None => Ok(()),
        FixtureResumeRacePoint::DestinationAppearsAfterClassification => {
            fs::write(&evidence.destination_path, b"fixture destination race bytes")?;
            Ok(())
        }
        FixtureResumeRacePoint::SourceChangesAfterClassification => {
            fs::write(&evidence.source_path, b"fixture source race bytes")?;
            Ok(())
        }
        FixtureResumeRacePoint::BackupChangesAfterClassification => {
            fs::write(&evidence.backup_path, b"fixture backup race bytes")?;
            Ok(())
        }
        FixtureResumeRacePoint::MoveMetadataAppearsAfterClassification => {
            apply_plan_results::record_apply_plan_result_log(
                connection,
                RecordApplyPlanResultLogRequest {
                    apply_plan_run_id: request.run_id,
                    apply_plan_item_id: Some(request.apply_plan_item_id),
                    operation_kind: MOVE_OPERATION_KIND.to_owned(),
                    result_status: "pending_log".to_owned(),
                    source_path_at_execution: Some(
                        evidence.source_path.to_string_lossy().to_string(),
                    ),
                    destination_path_at_execution: Some(
                        evidence.destination_path.to_string_lossy().to_string(),
                    ),
                    backup_path: Some(evidence.backup_path.to_string_lossy().to_string()),
                    error_code: None,
                    error_message: None,
                    user_summary: "Injected fixture metadata race for resume proof.".to_owned(),
                },
            )?;
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureHardLinkMoveFault {
    None,
    InterruptAfterDestinationClaim,
    ForceVerificationFailureAfterDestinationClaim,
}

fn claim_fixture_destination_no_replace(source: &Path, destination: &Path) -> AppResult<()> {
    fs::hard_link(source, destination).map_err(|error| {
        AppError::Message(format!(
            "Fixture no-replace destination claim failed for {} -> {}: {error}",
            source.display(),
            destination.display()
        ))
    })
}

fn move_fixture_file_via_hard_link_no_replace(
    source: &Path,
    destination: &Path,
    expected_size: u64,
    expected_hash: &str,
    fault: FixtureHardLinkMoveFault,
) -> AppResult<()> {
    if observe_expected_file(source, expected_size, expected_hash)? != ObservedFileState::Exact {
        return Err(AppError::Message(
            "Fixture hard-link move requires an exact verified source before claiming a destination."
                .to_owned(),
        ));
    }

    claim_fixture_destination_no_replace(source, destination)?;

    if fault == FixtureHardLinkMoveFault::InterruptAfterDestinationClaim {
        return Err(AppError::Message(
            "Injected fixture interruption after exclusive hard-link destination claim.".to_owned(),
        ));
    }

    let destination_verified = fault != FixtureHardLinkMoveFault::ForceVerificationFailureAfterDestinationClaim
        && observe_expected_file(destination, expected_size, expected_hash)? == ObservedFileState::Exact;
    if !destination_verified {
        if same_physical_file_identity(source, destination)? == Some(true) {
            fs::remove_file(destination).map_err(|error| {
                AppError::Message(format!(
                    "Fixture hard-link verification failed and the claimed destination link could not be removed safely: {error}"
                ))
            })?;
        }
        return Err(AppError::Message(
            "Fixture hard-link destination verification failed; source was retained and no move was completed."
                .to_owned(),
        ));
    }

    fs::remove_file(source).map_err(|error| {
        AppError::Message(format!(
            "Fixture hard-link destination verified but source unlink failed; reconciliation must inspect both names: {error}"
        ))
    })?;

    if observe_expected_file(source, expected_size, expected_hash)? != ObservedFileState::Missing
        || observe_expected_file(destination, expected_size, expected_hash)?
            != ObservedFileState::Exact
    {
        return Err(AppError::Message(
            "Fixture hard-link move finished with an unexpected source/destination state."
                .to_owned(),
        ));
    }
    Ok(())
}

fn existing_path_entry_is_symlink(path: &Path) -> AppResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::Message(format!(
            "Fixture path-entry type could not inspect {}: {error}",
            path.display()
        ))),
    }
}

fn same_physical_file_identity(first: &Path, second: &Path) -> AppResult<Option<bool>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let first_metadata = fs::symlink_metadata(first).map_err(|error| {
            AppError::Message(format!(
                "Fixture physical-file identity could not inspect {}: {error}",
                first.display()
            ))
        })?;
        let second_metadata = fs::symlink_metadata(second).map_err(|error| {
            AppError::Message(format!(
                "Fixture physical-file identity could not inspect {}: {error}",
                second.display()
            ))
        })?;
        return Ok(Some(
            first_metadata.dev() == second_metadata.dev()
                && first_metadata.ino() == second_metadata.ino(),
        ));
    }

    #[cfg(not(unix))]
    {
        let _ = (first, second);
        Ok(None)
    }
}

fn record_move_restore_entry(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    evidence: &FixtureReconciliationEvidence,
    move_result_id: i64,
) -> AppResult<()> {
    apply_plan_results::record_apply_plan_restore_entry(
        connection,
        RecordApplyPlanRestoreEntryRequest {
            apply_plan_run_id: request.run_id,
            apply_plan_result_id: Some(move_result_id),
            apply_plan_item_id: Some(request.apply_plan_item_id),
            original_source_path: evidence.source_path.to_string_lossy().to_string(),
            destination_path_at_execution: Some(
                evidence.destination_path.to_string_lossy().to_string(),
            ),
            backup_path: Some(evidence.backup_path.to_string_lossy().to_string()),
            file_hash_before: Some(evidence.expected_hash.clone()),
            file_size_before: Some(evidence.expected_size as i64),
            operation_kind: MOVE_OPERATION_KIND.to_owned(),
            operation_result_status: "pending_log".to_owned(),
            restore_status: "design_only".to_owned(),
            restore_error_code: None,
            restore_error_message: None,
        },
    )?;
    Ok(())
}

fn assess_fixture_reconciliation_state(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
) -> AppResult<FixtureReconciliationAssessment> {
    if !request.fixture_mode {
        return Err(AppError::Message(
            "Fixture reconciliation requires fixtureMode=true.".to_owned(),
        ));
    }
    ensure_run_matches_plan(connection, request.run_id, request.apply_plan_id)?;
    let fixture_root = canonicalize_existing_dir(&request.fixture_root, "fixtureRoot")?;
    let item = load_plan_item_scope(connection, request.apply_plan_id, request.apply_plan_item_id)?;
    if item.blocked || item.review_only || item.action_kind != "move" {
        return Ok(ambiguous_reconciliation(
            "Persisted ApplyPlan item is no longer an unblocked move candidate.",
        ));
    }

    let source_entry_is_symlink = existing_path_entry_is_symlink(Path::new(&item.current_path))?;
    let destination_entry_is_symlink =
        existing_path_entry_is_symlink(Path::new(&item.destination_path))?;
    let source_path = resolve_fixture_candidate(Path::new(&item.current_path), "currentPath")?;
    let destination_path =
        resolve_fixture_candidate(Path::new(&item.destination_path), "destinationPath")?;
    ensure_under_root(&source_path, &fixture_root, "currentPath")?;
    ensure_under_root(&destination_path, &fixture_root, "destinationPath")?;

    let indexed = load_indexed_source_scope(connection, item.file_id)?;
    let indexed_path = resolve_fixture_candidate(Path::new(&indexed.path), "indexed source path")?;
    if indexed_path != source_path || indexed.size < 0 || indexed.hash.is_none() {
        return Ok(ambiguous_reconciliation(
            "Indexed source evidence no longer matches the persisted ApplyPlan source.",
        ));
    }
    let indexed_hash = indexed.hash.as_deref().expect("checked indexed hash");
    let indexed_size = indexed.size as u64;

    let detail = apply_plan_results::get_apply_plan_run_log(connection, request.run_id)?
        .ok_or_else(|| AppError::Message("Fixture reconciliation run log was not found.".to_owned()))?;

    let backup_entries = detail
        .restore_entries
        .iter()
        .filter(|entry| {
            entry.apply_plan_item_id == Some(request.apply_plan_item_id)
                && entry.operation_kind == BACKUP_OPERATION_KIND
                && entry.operation_result_status == ApplyPlanResultLogStatus::PendingLog
                && entry.restore_status == ApplyPlanRestoreEntryStatus::DesignOnly
        })
        .collect::<Vec<_>>();
    if backup_entries.is_empty() {
        let source_state = observe_expected_file(&source_path, indexed_size, indexed_hash)?;
        let destination_exists = destination_path.exists();
        if source_state == ObservedFileState::Exact
            && !destination_exists
            && !detail.results.iter().any(|result| {
                result.apply_plan_item_id == Some(request.apply_plan_item_id)
                    && matches!(result.operation_kind.as_str(), MOVE_OPERATION_KIND | UNDO_OPERATION_KIND)
            })
        {
            return Ok(FixtureReconciliationAssessment {
                state: FixtureReconciliationState::CleanStart,
                evidence: None,
            });
        }
        return Ok(ambiguous_reconciliation(
            "No single verified backup chain exists for the observed fixture state.",
        ));
    }
    if backup_entries.len() != 1 {
        return Ok(ambiguous_reconciliation(
            "Multiple verified backup entries exist for one fixture plan item.",
        ));
    }
    let backup_entry = backup_entries[0];
    let Some(backup_result_id) = backup_entry.apply_plan_result_id else {
        return Ok(ambiguous_reconciliation(
            "Verified backup entry is not linked to its backup result.",
        ));
    };
    let Some(backup_result) = detail.results.iter().find(|result| result.id == backup_result_id)
    else {
        return Ok(ambiguous_reconciliation(
            "Verified backup result is missing from the requested run.",
        ));
    };
    if backup_result.apply_plan_item_id != Some(request.apply_plan_item_id)
        || backup_result.operation_kind != BACKUP_OPERATION_KIND
        || backup_result.result_status != ApplyPlanResultLogStatus::PendingLog
    {
        return Ok(ambiguous_reconciliation(
            "Verified backup result does not match the requested item scope.",
        ));
    }

    let Some(expected_hash) = backup_entry.file_hash_before.as_deref() else {
        return Ok(ambiguous_reconciliation(
            "Verified backup entry is missing its pre-move hash.",
        ));
    };
    let Some(expected_size_i64) = backup_entry.file_size_before else {
        return Ok(ambiguous_reconciliation(
            "Verified backup entry is missing its pre-move size.",
        ));
    };
    if expected_size_i64 < 0
        || expected_size_i64 as u64 != indexed_size
        || expected_hash != indexed_hash
    {
        return Ok(ambiguous_reconciliation(
            "Verified backup evidence disagrees with the indexed source snapshot.",
        ));
    }
    let expected_size = expected_size_i64 as u64;
    let Some(backup_path_text) = backup_entry.backup_path.as_deref() else {
        return Ok(ambiguous_reconciliation(
            "Verified backup entry is missing its backup path.",
        ));
    };
    let backup_path = resolve_fixture_candidate(Path::new(backup_path_text), "backupPath")?;
    ensure_under_root(&backup_path, &fixture_root, "backupPath")?;
    let recorded_source =
        resolve_fixture_candidate(Path::new(&backup_entry.original_source_path), "originalSourcePath")?;
    if recorded_source != source_path
        || resolve_optional_recorded_path(
            backup_result.source_path_at_execution.as_deref(),
            "backup result source",
        )? != Some(source_path.clone())
        || resolve_optional_recorded_path(
            backup_result.backup_path.as_deref(),
            "backup result backup",
        )? != Some(backup_path.clone())
        || observe_expected_file(&backup_path, expected_size, expected_hash)?
            != ObservedFileState::Exact
    {
        return Ok(ambiguous_reconciliation(
            "Verified backup path, source path, or bytes no longer agree.",
        ));
    }

    let move_results = detail
        .results
        .iter()
        .filter(|result| {
            result.apply_plan_item_id == Some(request.apply_plan_item_id)
                && result.operation_kind == MOVE_OPERATION_KIND
                && result.result_status == ApplyPlanResultLogStatus::PendingLog
        })
        .collect::<Vec<_>>();
    let move_entries = detail
        .restore_entries
        .iter()
        .filter(|entry| {
            entry.apply_plan_item_id == Some(request.apply_plan_item_id)
                && entry.operation_kind == MOVE_OPERATION_KIND
                && entry.operation_result_status == ApplyPlanResultLogStatus::PendingLog
                && entry.restore_status == ApplyPlanRestoreEntryStatus::DesignOnly
        })
        .collect::<Vec<_>>();
    let undo_results = detail
        .results
        .iter()
        .filter(|result| {
            result.apply_plan_item_id == Some(request.apply_plan_item_id)
                && result.operation_kind == UNDO_OPERATION_KIND
                && result.result_status == ApplyPlanResultLogStatus::PendingLog
        })
        .collect::<Vec<_>>();
    let undo_entries = detail
        .restore_entries
        .iter()
        .filter(|entry| {
            entry.apply_plan_item_id == Some(request.apply_plan_item_id)
                && entry.operation_kind == UNDO_OPERATION_KIND
                && entry.operation_result_status == ApplyPlanResultLogStatus::PendingLog
                && entry.restore_status == ApplyPlanRestoreEntryStatus::DesignOnly
        })
        .collect::<Vec<_>>();
    if move_results.len() > 1 || move_entries.len() > 1 || undo_results.len() > 1 || undo_entries.len() > 1 {
        return Ok(ambiguous_reconciliation(
            "Multiple successful transaction records exist for one fixture item.",
        ));
    }

    let move_result = move_results.first().copied();
    let move_entry = move_entries.first().copied();
    if let Some(result) = move_result {
        if resolve_optional_recorded_path(result.source_path_at_execution.as_deref(), "move source")?
            != Some(source_path.clone())
            || resolve_optional_recorded_path(
                result.destination_path_at_execution.as_deref(),
                "move destination",
            )? != Some(destination_path.clone())
            || resolve_optional_recorded_path(result.backup_path.as_deref(), "move backup")?
                != Some(backup_path.clone())
        {
            return Ok(ambiguous_reconciliation(
                "Recorded move-result paths disagree with the persisted plan and verified backup.",
            ));
        }
    }
    if let Some(entry) = move_entry {
        if move_result.map(|result| result.id) != entry.apply_plan_result_id
            || resolve_fixture_candidate(Path::new(&entry.original_source_path), "move original source")?
                != source_path
            || resolve_optional_recorded_path(
                entry.destination_path_at_execution.as_deref(),
                "move restore destination",
            )? != Some(destination_path.clone())
            || resolve_optional_recorded_path(entry.backup_path.as_deref(), "move restore backup")?
                != Some(backup_path.clone())
            || entry.file_hash_before.as_deref() != Some(expected_hash)
            || entry.file_size_before != Some(expected_size_i64)
        {
            return Ok(ambiguous_reconciliation(
                "Recorded move restore-map entry disagrees with verified transaction evidence.",
            ));
        }
    }
    if move_result.is_none() && move_entry.is_some() {
        return Ok(ambiguous_reconciliation(
            "Move restore-map entry exists without its move-result record.",
        ));
    }

    let undo_result = undo_results.first().copied();
    let undo_entry = undo_entries.first().copied();
    if undo_result.is_some() != undo_entry.is_some() {
        return Ok(ambiguous_reconciliation(
            "Undo metadata is only partially recorded.",
        ));
    }
    if let (Some(result), Some(entry)) = (undo_result, undo_entry) {
        if entry.apply_plan_result_id != Some(result.id)
            || resolve_optional_recorded_path(result.source_path_at_execution.as_deref(), "undo backup")?
                != Some(backup_path.clone())
            || resolve_optional_recorded_path(
                result.destination_path_at_execution.as_deref(),
                "undo destination",
            )? != Some(source_path.clone())
            || resolve_optional_recorded_path(result.backup_path.as_deref(), "undo backup reference")?
                != Some(backup_path.clone())
            || resolve_fixture_candidate(Path::new(&entry.original_source_path), "undo original source")?
                != source_path
            || resolve_optional_recorded_path(
                entry.destination_path_at_execution.as_deref(),
                "undo restore destination",
            )? != Some(source_path.clone())
            || resolve_optional_recorded_path(entry.backup_path.as_deref(), "undo restore backup")?
                != Some(backup_path.clone())
            || entry.file_hash_before.as_deref() != Some(expected_hash)
            || entry.file_size_before != Some(expected_size_i64)
        {
            return Ok(ambiguous_reconciliation(
                "Recorded undo metadata disagrees with verified transaction evidence.",
            ));
        }
    }

    let source_state = observe_expected_file(&source_path, expected_size, expected_hash)?;
    let destination_state =
        observe_expected_file(&destination_path, expected_size, expected_hash)?;
    if source_state == ObservedFileState::Different || destination_state == ObservedFileState::Different {
        return Ok(ambiguous_reconciliation(
            "Observed source or destination bytes differ from the verified backup.",
        ));
    }

    let exact_pair_same_physical_file = if source_state == ObservedFileState::Exact
        && destination_state == ObservedFileState::Exact
    {
        same_physical_file_identity(&source_path, &destination_path)?
    } else {
        None
    };

    let evidence = FixtureReconciliationEvidence {
        source_path,
        destination_path,
        backup_path,
        expected_hash: expected_hash.to_owned(),
        expected_size,
        backup_result_id,
        backup_restore_entry_id: backup_entry.id,
        move_result_id: move_result.map(|result| result.id),
    };
    let state = match (source_state, destination_state) {
        (ObservedFileState::Exact, ObservedFileState::Exact)
            if move_result.is_none()
                && move_entry.is_none()
                && undo_result.is_none()
                && !source_entry_is_symlink
                && !destination_entry_is_symlink
                && exact_pair_same_physical_file == Some(true) =>
        {
            FixtureReconciliationState::ExactHardLinkPairNeedsReview
        }
        (ObservedFileState::Exact, ObservedFileState::Missing)
            if move_result.is_none() && move_entry.is_none() && undo_result.is_none() =>
        {
            FixtureReconciliationState::ResumeFromVerifiedBackupRequired
        }
        (ObservedFileState::Missing, ObservedFileState::Exact)
            if move_result.is_none() && move_entry.is_none() && undo_result.is_none() =>
        {
            FixtureReconciliationState::MovedMissingResult
        }
        (ObservedFileState::Missing, ObservedFileState::Exact)
            if move_result.is_some() && move_entry.is_none() && undo_result.is_none() =>
        {
            FixtureReconciliationState::MoveResultMissingRestoreEntry
        }
        (ObservedFileState::Missing, ObservedFileState::Exact)
            if move_result.is_some() && move_entry.is_some() && undo_result.is_none() =>
        {
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        }
        (ObservedFileState::Exact, ObservedFileState::Exact)
            if move_result.is_some() && move_entry.is_some() && undo_result.is_some() =>
        {
            FixtureReconciliationState::UndoCleanupPending
        }
        (ObservedFileState::Exact, ObservedFileState::Missing)
            if move_result.is_some() && move_entry.is_some() && undo_result.is_some() =>
        {
            FixtureReconciliationState::UndoComplete
        }
        _ => FixtureReconciliationState::Ambiguous(
            "Observed files and persisted transaction records do not form one known safe state."
                .to_owned(),
        ),
    };
    Ok(FixtureReconciliationAssessment {
        state,
        evidence: Some(evidence),
    })
}

fn resolve_optional_recorded_path(
    value: Option<&str>,
    field_name: &str,
) -> AppResult<Option<PathBuf>> {
    value
        .map(|value| resolve_fixture_candidate(Path::new(value), field_name))
        .transpose()
}

fn observe_expected_file(
    path: &Path,
    expected_size: u64,
    expected_hash: &str,
) -> AppResult<ObservedFileState> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => {
            if metadata.len() == expected_size && hash_file(path)? == expected_hash {
                Ok(ObservedFileState::Exact)
            } else {
                Ok(ObservedFileState::Different)
            }
        }
        Ok(_) => Ok(ObservedFileState::Different),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ObservedFileState::Missing),
        Err(error) => Err(AppError::Message(format!(
            "Fixture reconciliation could not inspect {}: {error}",
            path.display()
        ))),
    }
}

fn ambiguous_reconciliation(reason: &str) -> FixtureReconciliationAssessment {
    FixtureReconciliationAssessment {
        state: FixtureReconciliationState::Ambiguous(reason.to_owned()),
        evidence: None,
    }
}

pub(crate) fn run_fixture_move_transaction(
    connection: &Connection,
    request: FixtureMoveTransactionRequest,
) -> AppResult<FixtureMoveTransactionOutcome> {
    run_fixture_move_transaction_with_fault(connection, request, FixtureFaultPoint::None)
}

fn run_fixture_move_transaction_with_fault(
    connection: &Connection,
    request: FixtureMoveTransactionRequest,
    fault_point: FixtureFaultPoint,
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

    if fault_point == FixtureFaultPoint::BeforeBackup {
        return Err(injected_interruption(fault_point));
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

    if fault_point == FixtureFaultPoint::AfterVerifiedBackupBeforeMove {
        return Err(injected_interruption(fault_point));
    }

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

    if fault_point == FixtureFaultPoint::AfterMoveBeforeResult {
        return Err(injected_interruption(fault_point));
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
    if fault_point == FixtureFaultPoint::AfterMoveResultBeforeRestoreEntry {
        return Err(injected_interruption(fault_point));
    }

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
    run_fixture_move_undo_with_fault(connection, request, FixtureFaultPoint::None)
}

fn run_fixture_move_undo_with_fault(
    connection: &Connection,
    request: FixtureMoveUndoRequest,
    fault_point: FixtureFaultPoint,
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
    if fault_point == FixtureFaultPoint::UndoAfterRestoreBeforeCleanup {
        return Err(injected_interruption(fault_point));
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

    fn reconciliation_request(context: &FixtureContext) -> FixtureReconciliationRequest {
        FixtureReconciliationRequest {
            fixture_mode: true,
            apply_plan_id: context.plan_id,
            apply_plan_item_id: context.item_id,
            run_id: context.run_id,
            fixture_root: context.fixture_root.clone(),
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
    fn interruption_before_backup_is_cleanly_retryable() {
        let connection = memory_connection();
        let source_bytes = b"before backup bytes";
        let context = setup_fixture(&connection, source_bytes);

        let error = run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::BeforeBackup,
        )
        .expect_err("injected interruption before backup");
        assert!(error.to_string().contains("BeforeBackup"));
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
        assert!(fs::read_dir(&context.backup_root)
            .expect("backup root")
            .next()
            .is_none());
        assert!(list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("result logs")
        .is_empty());
        assert!(list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("restore entries")
        .is_empty());

        let retry = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("retry after pre-backup interruption");
        assert!(matches!(retry, FixtureMoveTransactionOutcome::Verified(_)));
    }

    #[test]
    fn interruption_after_verified_backup_preserves_recovery_material_but_full_retry_is_blocked() {
        let connection = memory_connection();
        let source_bytes = b"after backup bytes";
        let context = setup_fixture(&connection, source_bytes);

        let error = run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("injected interruption after backup");
        assert!(error
            .to_string()
            .contains("AfterVerifiedBackupBeforeMove"));
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("result logs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].operation_kind, BACKUP_OPERATION_KIND);
        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 1);
        let backup_path = PathBuf::from(
            entries[0]
                .backup_path
                .as_deref()
                .expect("verified backup path"),
        );
        assert!(backup_path.exists());
        assert_eq!(fs::read(&backup_path).expect("verified backup"), source_bytes);

        let retry = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("retry after verified-backup interruption");
        let FixtureMoveTransactionOutcome::FailedBeforeChange(retry_failure) = retry else {
            panic!("full retry should remain blocked once the verified backup already exists");
        };
        assert_eq!(retry_failure.error_code, "backup_destination_exists");
        assert!(!retry_failure.backup_created);
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
        assert!(backup_path.exists());
    }

    #[test]
    fn interruption_after_move_before_result_exposes_orphaned_move_recovery_gap() {
        let connection = memory_connection();
        let source_bytes = b"orphaned move bytes";
        let context = setup_fixture(&connection, source_bytes);

        let error = run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterMoveBeforeResult,
        )
        .expect_err("injected interruption after move");
        assert!(error.to_string().contains("AfterMoveBeforeResult"));
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());
        assert_eq!(
            fs::read(&context.destination_path).expect("moved destination"),
            source_bytes
        );

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("result logs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].operation_kind, BACKUP_OPERATION_KIND);
        assert!(!results
            .iter()
            .any(|result| result.operation_kind == MOVE_OPERATION_KIND));
        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].operation_kind, BACKUP_OPERATION_KIND);
        let backup_path = PathBuf::from(
            entries[0]
                .backup_path
                .as_deref()
                .expect("verified backup path"),
        );
        assert!(backup_path.exists());
        assert_eq!(fs::read(&backup_path).expect("verified backup"), source_bytes);
    }

    #[test]
    fn interruption_after_move_result_before_restore_entry_exposes_incomplete_recovery_link() {
        let connection = memory_connection();
        let source_bytes = b"incomplete restore link bytes";
        let context = setup_fixture(&connection, source_bytes);

        let error = run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterMoveResultBeforeRestoreEntry,
        )
        .expect_err("injected interruption after move result");
        assert!(error
            .to_string()
            .contains("AfterMoveResultBeforeRestoreEntry"));
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("result logs");
        assert_eq!(results.len(), 2);
        let move_result = results
            .iter()
            .find(|result| result.operation_kind == MOVE_OPERATION_KIND)
            .expect("move result exists");
        assert_eq!(move_result.result_status, ApplyPlanResultLogStatus::PendingLog);
        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: context.run_id,
            },
        )
        .expect("restore entries");
        assert_eq!(entries.len(), 1);
        assert!(entries
            .iter()
            .all(|entry| entry.operation_kind == BACKUP_OPERATION_KIND));
        assert!(!entries.iter().any(|entry| {
            entry.apply_plan_result_id == Some(move_result.id)
                && entry.operation_kind == MOVE_OPERATION_KIND
        }));
    }

    #[test]
    fn interruption_during_undo_after_restore_exposes_verified_duplicate_state_and_retry_block() {
        let connection = memory_connection();
        let source_bytes = b"undo interruption bytes";
        let context = setup_fixture(&connection, source_bytes);
        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("fixture move transaction");
        let FixtureMoveTransactionOutcome::Verified(success) = outcome else {
            panic!("expected verified fixture move");
        };
        let undo_request = FixtureMoveUndoRequest {
            fixture_mode: true,
            apply_plan_id: context.plan_id,
            apply_plan_item_id: context.item_id,
            run_id: context.run_id,
            fixture_root: context.fixture_root.clone(),
            move_result_log_id: success.move_result_log_id,
            move_restore_entry_id: success.move_restore_entry_id,
        };

        let error = run_fixture_move_undo_with_fault(
            &connection,
            undo_request.clone(),
            FixtureFaultPoint::UndoAfterRestoreBeforeCleanup,
        )
        .expect_err("injected interruption during undo");
        assert!(error
            .to_string()
            .contains("UndoAfterRestoreBeforeCleanup"));
        assert!(context.source_path.exists());
        assert!(context.destination_path.exists());
        assert_eq!(fs::read(&context.source_path).expect("restored source"), source_bytes);
        assert_eq!(
            fs::read(&context.destination_path).expect("still moved destination"),
            source_bytes
        );

        let retry_error = run_fixture_move_undo(&connection, undo_request)
            .expect_err("ordinary retry remains blocked by no-overwrite guard");
        assert!(retry_error
            .to_string()
            .contains("undo target already exists"));
    }

    #[test]
    fn undo_blocks_when_moved_destination_was_tampered_after_execution() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"original move bytes");
        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("fixture move transaction");
        let FixtureMoveTransactionOutcome::Verified(success) = outcome else {
            panic!("expected verified fixture move");
        };
        fs::write(&context.destination_path, b"tampered destination bytes")
            .expect("tamper moved destination");

        let error = run_fixture_move_undo(
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
        .expect_err("tampered destination blocks undo");
        assert!(error
            .to_string()
            .contains("changed after execution"));
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());
        assert!(success.backup_path.exists());
    }

    #[test]
    fn reconciliation_classifies_verified_backup_as_resume_required_without_moving_file() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"resume required bytes");
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");

        let request = reconciliation_request(&context);
        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify backup-only state"),
            FixtureReconciliationState::ResumeFromVerifiedBackupRequired
        );
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("reconcile backup-only state"),
            FixtureReconciliationState::ResumeFromVerifiedBackupRequired
        );
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
    }

    #[test]
    fn resume_from_verified_backup_moves_once_records_metadata_and_remains_undoable() {
        let connection = memory_connection();
        let source_bytes = b"resume verified backup bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        let request = reconciliation_request(&context);
        let initial = assess_fixture_reconciliation_state(&connection, &request)
            .expect("initial resume assessment");
        let backup_path = initial
            .evidence
            .as_ref()
            .expect("verified backup evidence")
            .backup_path
            .clone();

        assert_eq!(
            resume_fixture_transaction_from_verified_backup(&connection, &request)
                .expect("resume from verified backup"),
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        );
        assert!(!context.source_path.exists());
        assert_eq!(
            fs::read(&context.destination_path).expect("resumed destination"),
            source_bytes
        );
        assert_eq!(fs::read(&backup_path).expect("verified backup"), source_bytes);

        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        let move_results = detail
            .results
            .iter()
            .filter(|result| result.operation_kind == MOVE_OPERATION_KIND)
            .collect::<Vec<_>>();
        let move_entries = detail
            .restore_entries
            .iter()
            .filter(|entry| entry.operation_kind == MOVE_OPERATION_KIND)
            .collect::<Vec<_>>();
        assert_eq!(move_results.len(), 1);
        assert_eq!(move_entries.len(), 1);

        assert_eq!(
            resume_fixture_transaction_from_verified_backup(&connection, &request)
                .expect("repeated resume stays a no-op"),
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        );
        let repeated_detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("repeated run detail")
            .expect("run");
        assert_eq!(
            repeated_detail
                .results
                .iter()
                .filter(|result| result.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            1
        );
        assert_eq!(
            repeated_detail
                .restore_entries
                .iter()
                .filter(|entry| entry.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            1
        );

        run_fixture_move_undo(
            &connection,
            FixtureMoveUndoRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: context.item_id,
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                move_result_log_id: move_results[0].id,
                move_restore_entry_id: move_entries[0].id,
            },
        )
        .expect("undo resumed move");
        assert_eq!(fs::read(&context.source_path).expect("restored source"), source_bytes);
        assert!(!context.destination_path.exists());
        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify completed resumed undo"),
            FixtureReconciliationState::UndoComplete
        );
    }

    #[test]
    fn resume_blocks_destination_that_appears_after_classification_without_overwrite() {
        let connection = memory_connection();
        let source_bytes = b"destination race source bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        let request = reconciliation_request(&context);

        let state = resume_fixture_transaction_from_verified_backup_with_race(
            &connection,
            &request,
            FixtureResumeRacePoint::DestinationAppearsAfterClassification,
        )
        .expect("destination race classification");
        assert!(matches!(state, FixtureReconciliationState::Ambiguous(_)));
        assert_eq!(fs::read(&context.source_path).expect("source untouched"), source_bytes);
        assert_eq!(
            fs::read(&context.destination_path).expect("competing destination untouched"),
            b"fixture destination race bytes"
        );
        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        assert_eq!(
            detail
                .results
                .iter()
                .filter(|result| result.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            0
        );
    }

    #[test]
    fn resume_blocks_source_change_after_classification_without_moving_it() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"source race original bytes");
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        let request = reconciliation_request(&context);

        let state = resume_fixture_transaction_from_verified_backup_with_race(
            &connection,
            &request,
            FixtureResumeRacePoint::SourceChangesAfterClassification,
        )
        .expect("source race classification");
        assert!(matches!(state, FixtureReconciliationState::Ambiguous(_)));
        assert_eq!(
            fs::read(&context.source_path).expect("changed source remains in place"),
            b"fixture source race bytes"
        );
        assert!(!context.destination_path.exists());
        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        assert_eq!(
            detail
                .results
                .iter()
                .filter(|result| result.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            0
        );
    }

    #[test]
    fn resume_blocks_backup_change_after_classification_without_moving_source() {
        let connection = memory_connection();
        let source_bytes = b"backup race original bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        let request = reconciliation_request(&context);
        let initial = assess_fixture_reconciliation_state(&connection, &request)
            .expect("initial resume assessment");
        let backup_path = initial
            .evidence
            .as_ref()
            .expect("verified backup evidence")
            .backup_path
            .clone();

        let state = resume_fixture_transaction_from_verified_backup_with_race(
            &connection,
            &request,
            FixtureResumeRacePoint::BackupChangesAfterClassification,
        )
        .expect("backup race classification");
        assert!(matches!(state, FixtureReconciliationState::Ambiguous(_)));
        assert_eq!(fs::read(&context.source_path).expect("source untouched"), source_bytes);
        assert!(!context.destination_path.exists());
        assert_eq!(
            fs::read(&backup_path).expect("changed backup remains for inspection"),
            b"fixture backup race bytes"
        );
        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        assert_eq!(
            detail
                .results
                .iter()
                .filter(|result| result.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            0
        );
    }

    #[test]
    fn resume_blocks_move_metadata_that_appears_after_classification_without_moving_bytes() {
        let connection = memory_connection();
        let source_bytes = b"metadata race original bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        let request = reconciliation_request(&context);

        let state = resume_fixture_transaction_from_verified_backup_with_race(
            &connection,
            &request,
            FixtureResumeRacePoint::MoveMetadataAppearsAfterClassification,
        )
        .expect("metadata race classification");
        assert!(matches!(state, FixtureReconciliationState::Ambiguous(_)));
        assert_eq!(fs::read(&context.source_path).expect("source untouched"), source_bytes);
        assert!(!context.destination_path.exists());
        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        assert_eq!(
            detail
                .results
                .iter()
                .filter(|result| result.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            1,
            "only the injected competing metadata record should exist"
        );
        assert_eq!(
            detail
                .restore_entries
                .iter()
                .filter(|entry| entry.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            0
        );
    }

    #[test]
    fn resume_from_clean_start_is_a_noop() {
        let connection = memory_connection();
        let source_bytes = b"clean start resume bytes";
        let context = setup_fixture(&connection, source_bytes);
        let request = reconciliation_request(&context);

        assert_eq!(
            resume_fixture_transaction_from_verified_backup(&connection, &request)
                .expect("clean start remains a no-op"),
            FixtureReconciliationState::CleanStart
        );
        assert_eq!(fs::read(&context.source_path).expect("source untouched"), source_bytes);
        assert!(!context.destination_path.exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hard_link_destination_claim_refuses_existing_destination_without_overwrite() {
        let temp = tempdir().expect("temp fixture");
        let source = temp.path().join("source.package");
        let destination = temp.path().join("destination.package");
        fs::write(&source, b"source bytes").expect("source");
        fs::write(&destination, b"existing destination bytes").expect("destination");

        claim_fixture_destination_no_replace(&source, &destination)
            .expect_err("existing destination must be refused");

        assert_eq!(fs::read(&source).expect("source unchanged"), b"source bytes");
        assert_eq!(
            fs::read(&destination).expect("destination unchanged"),
            b"existing destination bytes"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn concurrent_hard_link_destination_claims_allow_exactly_one_winner() {
        use std::sync::{Arc, Barrier};

        let temp = tempdir().expect("temp fixture");
        let source = temp.path().join("source.package");
        let destination = temp.path().join("destination.package");
        let source_bytes = b"concurrent hard-link bytes";
        fs::write(&source, source_bytes).expect("source");

        const CONTENDERS: usize = 16;
        let barrier = Arc::new(Barrier::new(CONTENDERS));
        let handles = (0..CONTENDERS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let source = source.clone();
                let destination = destination.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    claim_fixture_destination_no_replace(&source, &destination).is_ok()
                })
            })
            .collect::<Vec<_>>();

        let successful_claims = handles
            .into_iter()
            .map(|handle| usize::from(handle.join().expect("claim thread")))
            .sum::<usize>();

        assert_eq!(successful_claims, 1);
        assert_eq!(fs::read(&source).expect("source remains"), source_bytes);
        assert_eq!(fs::read(&destination).expect("claimed destination"), source_bytes);
        assert_eq!(
            same_physical_file_identity(&source, &destination)
                .expect("physical identity"),
            Some(true)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hard_link_no_replace_move_verifies_destination_before_unlinking_source() {
        let temp = tempdir().expect("temp fixture");
        let source = temp.path().join("source.package");
        let destination = temp.path().join("destination.package");
        let source_bytes = b"verified hard-link move bytes";
        let expected_hash = bytes_hash(source_bytes);
        fs::write(&source, source_bytes).expect("source");

        move_fixture_file_via_hard_link_no_replace(
            &source,
            &destination,
            source_bytes.len() as u64,
            &expected_hash,
            FixtureHardLinkMoveFault::None,
        )
        .expect("hard-link no-replace move");

        assert!(!source.exists());
        assert_eq!(fs::read(&destination).expect("destination"), source_bytes);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn hard_link_verification_failure_removes_only_claimed_link_and_retains_source() {
        let temp = tempdir().expect("temp fixture");
        let source = temp.path().join("source.package");
        let destination = temp.path().join("destination.package");
        let source_bytes = b"verification failure source bytes";
        let expected_hash = bytes_hash(source_bytes);
        fs::write(&source, source_bytes).expect("source");

        move_fixture_file_via_hard_link_no_replace(
            &source,
            &destination,
            source_bytes.len() as u64,
            &expected_hash,
            FixtureHardLinkMoveFault::ForceVerificationFailureAfterDestinationClaim,
        )
        .expect_err("forced verification failure");

        assert_eq!(fs::read(&source).expect("source retained"), source_bytes);
        assert!(!destination.exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn interrupted_forward_hard_link_claim_is_explicit_but_not_auto_reconciled() {
        let connection = memory_connection();
        let source_bytes = b"forward hard-link interruption bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        let request = reconciliation_request(&context);
        let initial = assess_fixture_reconciliation_state(&connection, &request)
            .expect("initial assessment");
        let evidence = initial.evidence.expect("verified backup evidence");

        move_fixture_file_via_hard_link_no_replace(
            &evidence.source_path,
            &evidence.destination_path,
            evidence.expected_size,
            &evidence.expected_hash,
            FixtureHardLinkMoveFault::InterruptAfterDestinationClaim,
        )
        .expect_err("interrupt after destination claim");

        assert_eq!(
            same_physical_file_identity(&context.source_path, &context.destination_path)
                .expect("physical identity"),
            Some(true)
        );
        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify interrupted hard-link claim"),
            FixtureReconciliationState::ExactHardLinkPairNeedsReview
        );
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("reconciler must leave forward claim untouched"),
            FixtureReconciliationState::ExactHardLinkPairNeedsReview
        );
        assert_eq!(fs::read(&context.source_path).expect("source retained"), source_bytes);
        assert_eq!(
            fs::read(&context.destination_path).expect("destination retained"),
            source_bytes
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn equal_copied_files_are_not_misclassified_as_forward_hard_link_claim() {
        let connection = memory_connection();
        let source_bytes = b"equal copy negative control bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        fs::copy(&context.source_path, &context.destination_path).expect("ordinary copy");
        let request = reconciliation_request(&context);

        assert_eq!(
            same_physical_file_identity(&context.source_path, &context.destination_path)
                .expect("physical identity"),
            Some(false)
        );
        assert!(matches!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify equal copied files"),
            FixtureReconciliationState::Ambiguous(_)
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn symlink_alias_is_not_misclassified_as_exact_hard_link_pair() {
        use std::os::unix::fs::symlink;

        let connection = memory_connection();
        let source_bytes = b"symlink alias negative control bytes";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        symlink(&context.source_path, &context.destination_path).expect("destination symlink");
        let request = reconciliation_request(&context);

        assert!(existing_path_entry_is_symlink(&context.destination_path)
            .expect("destination entry type"));
        assert!(matches!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify symlink alias"),
            FixtureReconciliationState::Ambiguous(_)
        ));
    }

    #[test]
    fn reconciliation_repairs_missing_move_result_and_restore_entry_without_moving_bytes_again() {
        let connection = memory_connection();
        let source_bytes = b"repair missing move metadata";
        let context = setup_fixture(&connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterMoveBeforeResult,
        )
        .expect_err("interrupt after move before result");
        let request = reconciliation_request(&context);

        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify moved missing result"),
            FixtureReconciliationState::MovedMissingResult
        );
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("repair missing move metadata"),
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        );
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("idempotent metadata reconciliation"),
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        );
        assert!(!context.source_path.exists());
        assert_eq!(
            fs::read(&context.destination_path).expect("moved destination"),
            source_bytes
        );

        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        let move_result = detail
            .results
            .iter()
            .find(|result| result.operation_kind == MOVE_OPERATION_KIND)
            .expect("repaired move result");
        let move_entry = detail
            .restore_entries
            .iter()
            .find(|entry| entry.operation_kind == MOVE_OPERATION_KIND)
            .expect("repaired move restore entry");
        run_fixture_move_undo(
            &connection,
            FixtureMoveUndoRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: context.item_id,
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                move_result_log_id: move_result.id,
                move_restore_entry_id: move_entry.id,
            },
        )
        .expect("undo after repaired metadata");
        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify completed undo"),
            FixtureReconciliationState::UndoComplete
        );
    }

    #[test]
    fn reconciliation_repairs_missing_move_restore_entry_and_is_idempotent() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"repair restore link bytes");
        run_fixture_move_transaction_with_fault(
            &connection,
            move_request(&context),
            FixtureFaultPoint::AfterMoveResultBeforeRestoreEntry,
        )
        .expect_err("interrupt after move result");
        let request = reconciliation_request(&context);

        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify missing restore entry"),
            FixtureReconciliationState::MoveResultMissingRestoreEntry
        );
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("repair restore entry"),
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        );
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("idempotent restore-entry reconciliation"),
            FixtureReconciliationState::MoveRecordedAwaitingUndo
        );
        let detail = apply_plan_results::get_apply_plan_run_log(&connection, context.run_id)
            .expect("run detail")
            .expect("run");
        assert_eq!(
            detail
                .restore_entries
                .iter()
                .filter(|entry| entry.operation_kind == MOVE_OPERATION_KIND)
                .count(),
            1
        );
    }

    #[test]
    fn reconciliation_finishes_interrupted_undo_cleanup_and_then_is_idempotent() {
        let connection = memory_connection();
        let source_bytes = b"cleanup pending bytes";
        let context = setup_fixture(&connection, source_bytes);
        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("fixture move transaction");
        let FixtureMoveTransactionOutcome::Verified(success) = outcome else {
            panic!("expected verified fixture move");
        };
        run_fixture_move_undo_with_fault(
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
            FixtureFaultPoint::UndoAfterRestoreBeforeCleanup,
        )
        .expect_err("interrupt undo before cleanup");
        let request = reconciliation_request(&context);

        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &request)
                .expect("classify cleanup-pending state"),
            FixtureReconciliationState::UndoCleanupPending
        );
        assert!(context.source_path.exists());
        assert!(context.destination_path.exists());
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("finish cleanup"),
            FixtureReconciliationState::UndoComplete
        );
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
        assert_eq!(fs::read(&context.source_path).expect("restored source"), source_bytes);
        assert_eq!(
            reconcile_fixture_transaction(&connection, &request)
                .expect("idempotent completed reconciliation"),
            FixtureReconciliationState::UndoComplete
        );
    }

    #[test]
    fn reconciliation_fails_closed_for_tampered_or_cross_scoped_evidence() {
        let connection = memory_connection();
        let context = setup_fixture(&connection, b"reconciliation tamper bytes");
        let outcome = run_fixture_move_transaction(&connection, move_request(&context))
            .expect("fixture move transaction");
        let FixtureMoveTransactionOutcome::Verified(_success) = outcome else {
            panic!("expected verified fixture move");
        };
        fs::write(&context.destination_path, b"tampered bytes")
            .expect("tamper destination");
        let request = reconciliation_request(&context);
        let state = classify_fixture_reconciliation_state(&connection, &request)
            .expect("classify tampered state");
        assert!(matches!(state, FixtureReconciliationState::Ambiguous(_)));
        let reconciled = reconcile_fixture_transaction(&connection, &request)
            .expect("ambiguous reconciliation remains a no-op");
        assert!(matches!(reconciled, FixtureReconciliationState::Ambiguous(_)));
        assert!(!context.source_path.exists());
        assert_eq!(
            fs::read(&context.destination_path).expect("tampered destination"),
            b"tampered bytes"
        );

        let other = setup_fixture(&connection, b"other scope bytes");
        let mut cross_scoped = request.clone();
        cross_scoped.run_id = other.run_id;
        assert!(classify_fixture_reconciliation_state(&connection, &cross_scoped).is_err());
        assert_eq!(
            fs::read(&context.destination_path).expect("unchanged tampered destination"),
            b"tampered bytes"
        );
    }

    #[test]
    fn fixture_transaction_is_not_registered_or_compiled_as_a_normal_command() {
        let commands_source = include_str!("../commands/mod.rs");
        assert!(!commands_source.contains("run_fixture_move_transaction"));
        assert!(!commands_source.contains("run_fixture_move_undo"));
        assert!(!commands_source.contains("classify_fixture_reconciliation_state"));
        assert!(!commands_source.contains("reconcile_fixture_transaction"));
        assert!(!commands_source.contains("resume_fixture_transaction_from_verified_backup"));
        assert!(!commands_source.contains("fixture_move_transaction"));

        let core_source = include_str!("mod.rs");
        assert!(core_source.contains("#[cfg(test)]\npub mod apply_plan_fixture_transaction_prototype;"));
        let transaction_source = include_str!("apply_plan_fixture_transaction_prototype.rs");
        assert!(transaction_source.starts_with("#![cfg(test)]"));
    }
}
