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
            FixtureBackupPrototypeSuccess, FixtureRestorePrototypeOutcome,
            FixtureRestorePrototypeRequest,
        },
        apply_plan_persistence, apply_plan_results,
        move_engine,
        move_engine_membership_prototype::{
            apply_fixture_membership_action, load_fixture_membership_state,
            FixtureMembershipAction, FixtureMembershipState,
        },
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

#[derive(Debug, Clone, PartialEq, Eq)]
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

const FIXTURE_ATTEMPT_STRATEGY_HARD_LINK: &str = "hard_link_no_replace";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureAttemptCapabilityProof {
    strategy: String,
    platform: String,
    same_filesystem: bool,
    hard_link_supported: bool,
    native_runtime_proven: bool,
    evidence_label: String,
}

impl FixtureAttemptCapabilityProof {
    fn validate_for_fixture_attempt(&self) -> AppResult<()> {
        if self.strategy != FIXTURE_ATTEMPT_STRATEGY_HARD_LINK
            || self.platform.trim().is_empty()
            || !self.same_filesystem
            || !self.hard_link_supported
            || !self.native_runtime_proven
            || self.evidence_label.trim().is_empty()
        {
            return Err(AppError::Message(
                "Fixture attempt requires complete explicit no-replace capability evidence."
                    .to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureAttemptState {
    PreparedBeforeChange,
    DestinationClaimObserved,
    DestinationVerified,
    SourceReleaseCompleted,
    Committed,
    BlockedBeforeChange,
    FailedBeforeChange,
    RecoveryRequired,
}

impl FixtureAttemptState {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreparedBeforeChange => "prepared_before_change",
            Self::DestinationClaimObserved => "destination_claim_observed",
            Self::DestinationVerified => "destination_verified",
            Self::SourceReleaseCompleted => "source_release_completed",
            Self::Committed => "committed",
            Self::BlockedBeforeChange => "blocked_before_change",
            Self::FailedBeforeChange => "failed_before_change",
            Self::RecoveryRequired => "recovery_required",
        }
    }

    fn parse(value: &str) -> AppResult<Self> {
        match value {
            "prepared_before_change" => Ok(Self::PreparedBeforeChange),
            "destination_claim_observed" => Ok(Self::DestinationClaimObserved),
            "destination_verified" => Ok(Self::DestinationVerified),
            "source_release_completed" => Ok(Self::SourceReleaseCompleted),
            "committed" => Ok(Self::Committed),
            "blocked_before_change" => Ok(Self::BlockedBeforeChange),
            "failed_before_change" => Ok(Self::FailedBeforeChange),
            "recovery_required" => Ok(Self::RecoveryRequired),
            other => Err(AppError::Message(format!(
                "Unknown fixture attempt state: {other}"
            ))),
        }
    }

    fn transition_allowed(self, next: Self) -> bool {
        matches!(
            (self, next),
            (
                Self::PreparedBeforeChange,
                Self::DestinationClaimObserved
                    | Self::BlockedBeforeChange
                    | Self::FailedBeforeChange
            ) | (
                Self::DestinationClaimObserved,
                Self::DestinationVerified | Self::RecoveryRequired
            ) | (
                Self::DestinationVerified,
                Self::SourceReleaseCompleted | Self::RecoveryRequired
            ) | (
                Self::SourceReleaseCompleted,
                Self::RecoveryRequired
            )
        )
    }

    fn is_nonterminal(self) -> bool {
        matches!(
            self,
            Self::PreparedBeforeChange
                | Self::DestinationClaimObserved
                | Self::DestinationVerified
                | Self::SourceReleaseCompleted
                | Self::RecoveryRequired
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureAttemptRecord {
    attempt_id: String,
    apply_plan_run_id: i64,
    apply_plan_id: i64,
    apply_plan_item_id: i64,
    source_path: String,
    destination_path: String,
    expected_hash: String,
    expected_size: u64,
    initial_source_location: String,
    initial_download_item_id: Option<i64>,
    backup_result_id: i64,
    backup_restore_entry_id: i64,
    capability: FixtureAttemptCapabilityProof,
    source_regular_file: bool,
    source_entry_non_symlink: bool,
    destination_entry_non_symlink: bool,
    state: FixtureAttemptState,
}

fn ensure_fixture_attempt_journal_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS fixture_apply_plan_attempts (
            attempt_id TEXT PRIMARY KEY NOT NULL,
            apply_plan_run_id INTEGER NOT NULL,
            apply_plan_id INTEGER NOT NULL,
            apply_plan_item_id INTEGER NOT NULL,
            source_path TEXT NOT NULL,
            destination_path TEXT NOT NULL,
            expected_hash TEXT NOT NULL,
            expected_size INTEGER NOT NULL CHECK(expected_size >= 0),
            initial_source_location TEXT NOT NULL,
            initial_download_item_id INTEGER,
            backup_result_id INTEGER NOT NULL,
            backup_restore_entry_id INTEGER NOT NULL,
            strategy TEXT NOT NULL,
            capability_platform TEXT NOT NULL,
            same_filesystem INTEGER NOT NULL CHECK(same_filesystem IN (0, 1)),
            hard_link_supported INTEGER NOT NULL CHECK(hard_link_supported IN (0, 1)),
            native_runtime_proven INTEGER NOT NULL CHECK(native_runtime_proven IN (0, 1)),
            capability_evidence_label TEXT NOT NULL,
            source_regular_file INTEGER NOT NULL CHECK(source_regular_file IN (0, 1)),
            source_entry_non_symlink INTEGER NOT NULL CHECK(source_entry_non_symlink IN (0, 1)),
            destination_entry_non_symlink INTEGER NOT NULL CHECK(destination_entry_non_symlink IN (0, 1)),
            state TEXT NOT NULL CHECK(state IN (
                'prepared_before_change',
                'destination_claim_observed',
                'destination_verified',
                'source_release_completed',
                'committed',
                'blocked_before_change',
                'failed_before_change',
                'recovery_required'
            )),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_fixture_apply_attempt_active_item
        ON fixture_apply_plan_attempts(apply_plan_run_id, apply_plan_item_id)
        WHERE state IN (
            'prepared_before_change',
            'destination_claim_observed',
            'destination_verified',
            'source_release_completed',
            'recovery_required'
        );",
    )?;
    Ok(())
}

fn load_fixture_attempt(
    connection: &Connection,
    attempt_id: &str,
) -> AppResult<Option<FixtureAttemptRecord>> {
    ensure_fixture_attempt_journal_schema(connection)?;
    let row = connection
        .query_row(
            "SELECT
                attempt_id,
                apply_plan_run_id,
                apply_plan_id,
                apply_plan_item_id,
                source_path,
                destination_path,
                expected_hash,
                expected_size,
                initial_source_location,
                initial_download_item_id,
                backup_result_id,
                backup_restore_entry_id,
                strategy,
                capability_platform,
                same_filesystem,
                hard_link_supported,
                native_runtime_proven,
                capability_evidence_label,
                source_regular_file,
                source_entry_non_symlink,
                destination_entry_non_symlink,
                state
             FROM fixture_apply_plan_attempts
             WHERE attempt_id = ?1",
            params![attempt_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<i64>>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, i64>(14)?,
                    row.get::<_, i64>(15)?,
                    row.get::<_, i64>(16)?,
                    row.get::<_, String>(17)?,
                    row.get::<_, i64>(18)?,
                    row.get::<_, i64>(19)?,
                    row.get::<_, i64>(20)?,
                    row.get::<_, String>(21)?,
                ))
            },
        )
        .optional()?;

    let Some((
        attempt_id,
        apply_plan_run_id,
        apply_plan_id,
        apply_plan_item_id,
        source_path,
        destination_path,
        expected_hash,
        expected_size,
        initial_source_location,
        initial_download_item_id,
        backup_result_id,
        backup_restore_entry_id,
        strategy,
        capability_platform,
        same_filesystem,
        hard_link_supported,
        native_runtime_proven,
        capability_evidence_label,
        source_regular_file,
        source_entry_non_symlink,
        destination_entry_non_symlink,
        state,
    )) = row
    else {
        return Ok(None);
    };
    if expected_size < 0 {
        return Err(AppError::Message(
            "Fixture attempt stored a negative expected size.".to_owned(),
        ));
    }

    Ok(Some(FixtureAttemptRecord {
        attempt_id,
        apply_plan_run_id,
        apply_plan_id,
        apply_plan_item_id,
        source_path,
        destination_path,
        expected_hash,
        expected_size: expected_size as u64,
        initial_source_location,
        initial_download_item_id,
        backup_result_id,
        backup_restore_entry_id,
        capability: FixtureAttemptCapabilityProof {
            strategy,
            platform: capability_platform,
            same_filesystem: same_filesystem != 0,
            hard_link_supported: hard_link_supported != 0,
            native_runtime_proven: native_runtime_proven != 0,
            evidence_label: capability_evidence_label,
        },
        source_regular_file: source_regular_file != 0,
        source_entry_non_symlink: source_entry_non_symlink != 0,
        destination_entry_non_symlink: destination_entry_non_symlink != 0,
        state: FixtureAttemptState::parse(&state)?,
    }))
}

fn prepare_fixture_attempt(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    attempt_id: &str,
    capability: &FixtureAttemptCapabilityProof,
) -> AppResult<FixtureAttemptRecord> {
    if attempt_id.trim().is_empty() {
        return Err(AppError::Message(
            "Fixture attempt id must not be empty.".to_owned(),
        ));
    }
    capability.validate_for_fixture_attempt()?;

    let assessment = assess_fixture_reconciliation_state(connection, request)?;
    if assessment.state != FixtureReconciliationState::ResumeFromVerifiedBackupRequired {
        return Err(AppError::Message(format!(
            "Fixture attempt can only be prepared from ResumeFromVerifiedBackupRequired, found {:?}.",
            assessment.state
        )));
    }
    let evidence = assessment.evidence.ok_or_else(|| {
        AppError::Message("Fixture attempt lost verified backup evidence.".to_owned())
    })?;
    let item = load_plan_item_scope(connection, request.apply_plan_id, request.apply_plan_item_id)?;
    let initial_membership = load_fixture_membership_state(connection, item.file_id)?
        .ok_or_else(|| AppError::Message("Fixture attempt requires current Library membership evidence.".to_owned()))?;
    let initial_membership_path = resolve_fixture_candidate(
        Path::new(&initial_membership.path),
        "initial membership path",
    )?;
    if initial_membership.file_id != item.file_id || initial_membership_path != evidence.source_path {
        return Err(AppError::Message(
            "Fixture attempt Library membership does not match the verified source identity."
                .to_owned(),
        ));
    }
    let source_path = Path::new(&item.current_path);
    let destination_path = Path::new(&item.destination_path);
    let source_metadata = fs::symlink_metadata(source_path).map_err(|error| {
        AppError::Message(format!(
            "Fixture attempt could not inspect source path entry: {error}"
        ))
    })?;
    let source_regular_file = source_metadata.file_type().is_file();
    let source_entry_non_symlink = !source_metadata.file_type().is_symlink();
    let destination_entry_non_symlink = !existing_path_entry_is_symlink(destination_path)?;
    if !source_regular_file || !source_entry_non_symlink || !destination_entry_non_symlink {
        return Err(AppError::Message(
            "Fixture attempt requires a regular non-symlink source and non-symlink destination entry."
                .to_owned(),
        ));
    }

    ensure_fixture_attempt_journal_schema(connection)?;
    let existing_active = connection
        .query_row(
            "SELECT attempt_id
             FROM fixture_apply_plan_attempts
             WHERE apply_plan_run_id = ?1
               AND apply_plan_item_id = ?2
               AND state IN (
                   'prepared_before_change',
                   'destination_claim_observed',
                   'destination_verified',
                   'source_release_completed',
                   'recovery_required'
               )
             LIMIT 1",
            params![request.run_id, request.apply_plan_item_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if let Some(existing_attempt_id) = existing_active {
        return Err(AppError::Message(format!(
            "Fixture item already has an active attempt: {existing_attempt_id}"
        )));
    }

    connection.execute(
        "INSERT INTO fixture_apply_plan_attempts (
            attempt_id,
            apply_plan_run_id,
            apply_plan_id,
            apply_plan_item_id,
            source_path,
            destination_path,
            expected_hash,
            expected_size,
            initial_source_location,
            initial_download_item_id,
            backup_result_id,
            backup_restore_entry_id,
            strategy,
            capability_platform,
            same_filesystem,
            hard_link_supported,
            native_runtime_proven,
            capability_evidence_label,
            source_regular_file,
            source_entry_non_symlink,
            destination_entry_non_symlink,
            state
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
        )",
        params![
            attempt_id,
            request.run_id,
            request.apply_plan_id,
            request.apply_plan_item_id,
            evidence.source_path.to_string_lossy().to_string(),
            evidence.destination_path.to_string_lossy().to_string(),
            evidence.expected_hash,
            evidence.expected_size as i64,
            initial_membership.source_location,
            initial_membership.download_item_id,
            evidence.backup_result_id,
            evidence.backup_restore_entry_id,
            capability.strategy,
            capability.platform,
            i64::from(capability.same_filesystem),
            i64::from(capability.hard_link_supported),
            i64::from(capability.native_runtime_proven),
            capability.evidence_label,
            i64::from(source_regular_file),
            i64::from(source_entry_non_symlink),
            i64::from(destination_entry_non_symlink),
            FixtureAttemptState::PreparedBeforeChange.as_str(),
        ],
    )?;

    load_fixture_attempt(connection, attempt_id)?.ok_or_else(|| {
        AppError::Message("Fixture attempt was not readable after preparation.".to_owned())
    })
}

fn transition_fixture_attempt(
    connection: &Connection,
    attempt_id: &str,
    next: FixtureAttemptState,
) -> AppResult<FixtureAttemptRecord> {
    let current = load_fixture_attempt(connection, attempt_id)?.ok_or_else(|| {
        AppError::Message(format!("Fixture attempt not found: {attempt_id}"))
    })?;
    if !current.state.transition_allowed(next) {
        return Err(AppError::Message(format!(
            "Fixture attempt transition is not allowed: {} -> {}",
            current.state.as_str(),
            next.as_str()
        )));
    }

    let changed = connection.execute(
        "UPDATE fixture_apply_plan_attempts
         SET state = ?1, updated_at = CURRENT_TIMESTAMP
         WHERE attempt_id = ?2 AND state = ?3",
        params![next.as_str(), attempt_id, current.state.as_str()],
    )?;
    if changed != 1 {
        return Err(AppError::Message(
            "Fixture attempt changed concurrently before transition.".to_owned(),
        ));
    }

    load_fixture_attempt(connection, attempt_id)?.ok_or_else(|| {
        AppError::Message("Fixture attempt disappeared after transition.".to_owned())
    })
}

#[derive(Debug, Clone)]
struct FixtureMembershipHandoff {
    attempt_id: String,
    action: FixtureMembershipAction,
}

fn validate_fixture_forward_membership_action(
    item: &FixtureMovePlanItemScope,
    attempt: &FixtureAttemptRecord,
    action: &FixtureMembershipAction,
) -> AppResult<()> {
    let attempt_source =
        resolve_fixture_candidate(Path::new(&attempt.source_path), "attempt source path")?;
    match action {
        FixtureMembershipAction::Transition {
            expected,
            final_state,
        } => {
            let expected_path =
                resolve_fixture_candidate(Path::new(&expected.path), "membership expected path")?;
            let final_path = resolve_fixture_candidate(
                Path::new(&final_state.path),
                "membership final path",
            )?;
            let attempt_destination = resolve_fixture_candidate(
                Path::new(&attempt.destination_path),
                "attempt destination path",
            )?;
            if expected.file_id != item.file_id
                || final_state.file_id != item.file_id
                || expected_path != attempt_source
                || final_path != attempt_destination
                || expected.source_location != attempt.initial_source_location
                || expected.download_item_id != attempt.initial_download_item_id
            {
                return Err(AppError::Message(
                    "Fixture membership transition does not match the durable attempt file identity and paths."
                        .to_owned(),
                ));
            }
        }
        FixtureMembershipAction::Delete { expected } => {
            let expected_path =
                resolve_fixture_candidate(Path::new(&expected.path), "membership delete path")?;
            if expected.file_id != item.file_id
                || expected_path != attempt_source
                || expected.source_location != attempt.initial_source_location
                || expected.download_item_id != attempt.initial_download_item_id
            {
                return Err(AppError::Message(
                    "Fixture membership delete does not match the durable attempt file identity and source path."
                        .to_owned(),
                ));
            }
        }
        FixtureMembershipAction::InsertRestored { .. } => {
            return Err(AppError::Message(
                "Fixture forward file-operation handoff cannot insert restored membership."
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn fixture_forward_membership_action_matches_item(
    item: &FixtureMovePlanItemScope,
    attempt: &FixtureAttemptRecord,
    action: &FixtureMembershipAction,
) -> bool {
    match action {
        FixtureMembershipAction::Transition {
            expected,
            final_state,
        } => {
            expected.file_id == item.file_id
                && final_state.file_id == item.file_id
                && expected.source_location == attempt.initial_source_location
                && expected.download_item_id == attempt.initial_download_item_id
        }
        FixtureMembershipAction::Delete { expected } => {
            expected.file_id == item.file_id
                && expected.source_location == attempt.initial_source_location
                && expected.download_item_id == attempt.initial_download_item_id
        }
        FixtureMembershipAction::InsertRestored { .. } => false,
    }
}

#[derive(Debug, Clone)]
struct PreparedFixtureMembershipHandoff {
    handoff: FixtureMembershipHandoff,
    attempt: FixtureAttemptRecord,
    item: FixtureMovePlanItemScope,
}

fn prepare_fixture_membership_handoff(
    connection: &Connection,
    handoff: &FixtureMembershipHandoff,
) -> AppResult<PreparedFixtureMembershipHandoff> {
    let attempt = load_fixture_attempt(connection, &handoff.attempt_id)?.ok_or_else(|| {
        AppError::Message(format!(
            "Fixture membership handoff attempt not found: {}",
            handoff.attempt_id
        ))
    })?;
    if !matches!(
        attempt.state,
        FixtureAttemptState::SourceReleaseCompleted | FixtureAttemptState::Committed
    ) {
        return Err(AppError::Message(format!(
            "Fixture membership handoff requires source_release_completed or committed attempt state, found {}.",
            attempt.state.as_str()
        )));
    }

    let item = load_plan_item_scope(
        connection,
        attempt.apply_plan_id,
        attempt.apply_plan_item_id,
    )?;
    validate_fixture_forward_membership_action(&item, &attempt, &handoff.action)?;

    if observe_expected_file(
        Path::new(&attempt.source_path),
        attempt.expected_size,
        &attempt.expected_hash,
    )? != ObservedFileState::Missing
        || observe_expected_file(
            Path::new(&attempt.destination_path),
            attempt.expected_size,
            &attempt.expected_hash,
        )? != ObservedFileState::Exact
    {
        return Err(AppError::Message(
            "Fixture membership handoff requires the source to be released and the destination bytes to match the durable attempt evidence."
                .to_owned(),
        ));
    }

    Ok(PreparedFixtureMembershipHandoff {
        handoff: handoff.clone(),
        attempt,
        item,
    })
}

fn revalidate_prepared_membership_handoff_in_transaction(
    connection: &Connection,
    prepared: &PreparedFixtureMembershipHandoff,
) -> AppResult<FixtureAttemptRecord> {
    let current_attempt = load_fixture_attempt(connection, &prepared.handoff.attempt_id)?
        .ok_or_else(|| {
            AppError::Message(format!(
                "Fixture membership handoff attempt disappeared: {}",
                prepared.handoff.attempt_id
            ))
        })?;
    if current_attempt != prepared.attempt {
        return Err(AppError::Message(
            "Fixture membership handoff attempt evidence changed before database commit."
                .to_owned(),
        ));
    }

    let current_item = load_plan_item_scope(
        connection,
        prepared.attempt.apply_plan_id,
        prepared.attempt.apply_plan_item_id,
    )?;
    if current_item != prepared.item {
        return Err(AppError::Message(
            "Fixture membership handoff plan evidence changed before database commit."
                .to_owned(),
        ));
    }

    if !fixture_forward_membership_action_matches_item(
        &current_item,
        &current_attempt,
        &prepared.handoff.action,
    ) {
        return Err(AppError::Message(
            "Fixture membership handoff file identity changed before database commit."
                .to_owned(),
        ));
    }

    Ok(current_attempt)
}

fn commit_fixture_membership_handoffs(
    connection: &Connection,
    handoffs: &[FixtureMembershipHandoff],
) -> AppResult<()> {
    let prepared = handoffs
        .iter()
        .map(|handoff| prepare_fixture_membership_handoff(connection, handoff))
        .collect::<AppResult<Vec<_>>>()?;

    let transaction = connection.unchecked_transaction()?;
    for prepared_handoff in &prepared {
        let attempt = revalidate_prepared_membership_handoff_in_transaction(
            &transaction,
            prepared_handoff,
        )?;
        apply_fixture_membership_action(&transaction, &prepared_handoff.handoff.action)?;
        if attempt.state == FixtureAttemptState::SourceReleaseCompleted {
            commit_fixture_attempt_after_membership(
                &transaction,
                &prepared_handoff.handoff.attempt_id,
            )?;
        }
    }
    transaction.commit()?;
    Ok(())
}

#[derive(Debug, Clone)]
struct FixtureReverseMembershipHandoff {
    attempt_id: String,
    action: FixtureMembershipAction,
}

fn validate_fixture_reverse_membership_action(
    item: &FixtureMovePlanItemScope,
    attempt: &FixtureAttemptRecord,
    action: &FixtureMembershipAction,
) -> AppResult<()> {
    let attempt_source =
        resolve_fixture_candidate(Path::new(&attempt.source_path), "attempt source path")?;
    let attempt_destination = resolve_fixture_candidate(
        Path::new(&attempt.destination_path),
        "attempt destination path",
    )?;
    match action {
        FixtureMembershipAction::Transition {
            expected,
            final_state,
        } => {
            let expected_path = resolve_fixture_candidate(
                Path::new(&expected.path),
                "reverse membership expected path",
            )?;
            let final_path = resolve_fixture_candidate(
                Path::new(&final_state.path),
                "reverse membership final path",
            )?;
            if expected.file_id != item.file_id
                || final_state.file_id != item.file_id
                || expected_path != attempt_destination
                || final_path != attempt_source
                || final_state.source_location != attempt.initial_source_location
                || final_state.download_item_id != attempt.initial_download_item_id
            {
                return Err(AppError::Message(
                    "Fixture reverse membership transition does not restore the durable original membership."
                        .to_owned(),
                ));
            }
        }
        FixtureMembershipAction::InsertRestored { final_state } => {
            let final_path = resolve_fixture_candidate(
                Path::new(&final_state.path),
                "restored membership path",
            )?;
            if final_path != attempt_source
                || final_state.source_location != attempt.initial_source_location
                || final_state.download_item_id != attempt.initial_download_item_id
            {
                return Err(AppError::Message(
                    "Fixture restored membership does not match the durable original membership."
                        .to_owned(),
                ));
            }
        }
        FixtureMembershipAction::Delete { .. } => {
            return Err(AppError::Message(
                "Fixture reverse membership handoff cannot delete membership."
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn fixture_reverse_membership_action_matches_item(
    item: &FixtureMovePlanItemScope,
    attempt: &FixtureAttemptRecord,
    action: &FixtureMembershipAction,
) -> bool {
    match action {
        FixtureMembershipAction::Transition {
            expected,
            final_state,
        } => {
            expected.file_id == item.file_id
                && final_state.file_id == item.file_id
                && final_state.source_location == attempt.initial_source_location
                && final_state.download_item_id == attempt.initial_download_item_id
        }
        FixtureMembershipAction::InsertRestored { final_state } => {
            final_state.source_location == attempt.initial_source_location
                && final_state.download_item_id == attempt.initial_download_item_id
        }
        FixtureMembershipAction::Delete { .. } => false,
    }
}

#[derive(Debug, Clone)]
struct PreparedFixtureReverseMembershipHandoff {
    handoff: FixtureReverseMembershipHandoff,
    attempt: FixtureAttemptRecord,
    item: FixtureMovePlanItemScope,
}

fn prepare_fixture_reverse_membership_handoff(
    connection: &Connection,
    handoff: &FixtureReverseMembershipHandoff,
) -> AppResult<PreparedFixtureReverseMembershipHandoff> {
    let attempt = load_fixture_attempt(connection, &handoff.attempt_id)?.ok_or_else(|| {
        AppError::Message(format!(
            "Fixture reverse membership attempt not found: {}",
            handoff.attempt_id
        ))
    })?;
    if attempt.state != FixtureAttemptState::Committed {
        return Err(AppError::Message(format!(
            "Fixture reverse membership handoff requires a committed attempt, found {}.",
            attempt.state.as_str()
        )));
    }
    let item = load_plan_item_scope(
        connection,
        attempt.apply_plan_id,
        attempt.apply_plan_item_id,
    )?;
    validate_fixture_reverse_membership_action(&item, &attempt, &handoff.action)?;

    if observe_expected_file(
        Path::new(&attempt.source_path),
        attempt.expected_size,
        &attempt.expected_hash,
    )? != ObservedFileState::Exact
        || observe_expected_file(
            Path::new(&attempt.destination_path),
            attempt.expected_size,
            &attempt.expected_hash,
        )? != ObservedFileState::Missing
    {
        return Err(AppError::Message(
            "Fixture reverse membership handoff requires the exact source to be restored and the moved destination to be absent."
                .to_owned(),
        ));
    }

    Ok(PreparedFixtureReverseMembershipHandoff {
        handoff: handoff.clone(),
        attempt,
        item,
    })
}

fn revalidate_prepared_reverse_membership_handoff_in_transaction(
    connection: &Connection,
    prepared: &PreparedFixtureReverseMembershipHandoff,
) -> AppResult<()> {
    let current_attempt = load_fixture_attempt(connection, &prepared.handoff.attempt_id)?
        .ok_or_else(|| {
            AppError::Message(format!(
                "Fixture reverse membership attempt disappeared: {}",
                prepared.handoff.attempt_id
            ))
        })?;
    if current_attempt != prepared.attempt {
        return Err(AppError::Message(
            "Fixture reverse membership attempt evidence changed before database commit."
                .to_owned(),
        ));
    }
    let current_item = load_plan_item_scope(
        connection,
        prepared.attempt.apply_plan_id,
        prepared.attempt.apply_plan_item_id,
    )?;
    if current_item != prepared.item
        || !fixture_reverse_membership_action_matches_item(
            &current_item,
            &current_attempt,
            &prepared.handoff.action,
        )
    {
        return Err(AppError::Message(
            "Fixture reverse membership plan or membership identity changed before database commit."
                .to_owned(),
        ));
    }
    Ok(())
}

fn commit_fixture_reverse_membership_handoffs(
    connection: &Connection,
    handoffs: &[FixtureReverseMembershipHandoff],
) -> AppResult<()> {
    let prepared = handoffs
        .iter()
        .map(|handoff| prepare_fixture_reverse_membership_handoff(connection, handoff))
        .collect::<AppResult<Vec<_>>>()?;

    let transaction = connection.unchecked_transaction()?;
    for prepared_handoff in &prepared {
        revalidate_prepared_reverse_membership_handoff_in_transaction(
            &transaction,
            prepared_handoff,
        )?;
        apply_fixture_membership_action(&transaction, &prepared_handoff.handoff.action)?;
    }
    transaction.commit()?;
    Ok(())
}

fn commit_fixture_attempt_after_membership(
    connection: &Connection,
    attempt_id: &str,
) -> AppResult<FixtureAttemptRecord> {
    let current = load_fixture_attempt(connection, attempt_id)?.ok_or_else(|| {
        AppError::Message(format!("Fixture attempt not found: {attempt_id}"))
    })?;
    if current.state == FixtureAttemptState::Committed {
        return Ok(current);
    }
    if current.state != FixtureAttemptState::SourceReleaseCompleted {
        return Err(AppError::Message(format!(
            "Fixture membership commit requires source_release_completed attempt state, found {}.",
            current.state.as_str()
        )));
    }

    let changed = connection.execute(
        "UPDATE fixture_apply_plan_attempts
         SET state = ?1, updated_at = CURRENT_TIMESTAMP
         WHERE attempt_id = ?2 AND state = ?3",
        params![
            FixtureAttemptState::Committed.as_str(),
            attempt_id,
            FixtureAttemptState::SourceReleaseCompleted.as_str(),
        ],
    )?;
    if changed != 1 {
        return Err(AppError::Message(
            "Fixture attempt changed concurrently before membership commit.".to_owned(),
        ));
    }

    load_fixture_attempt(connection, attempt_id)?.ok_or_else(|| {
        AppError::Message("Fixture attempt disappeared after membership commit.".to_owned())
    })
}

fn fixture_attempt_owns_exact_hard_link_pair(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    attempt_id: &str,
) -> AppResult<bool> {
    let Some(attempt) = load_fixture_attempt(connection, attempt_id)? else {
        return Ok(false);
    };
    if attempt.apply_plan_run_id != request.run_id
        || attempt.apply_plan_id != request.apply_plan_id
        || attempt.apply_plan_item_id != request.apply_plan_item_id
        || !attempt.state.is_nonterminal()
    {
        return Ok(false);
    }

    let assessment = assess_fixture_reconciliation_state(connection, request)?;
    if assessment.state != FixtureReconciliationState::ExactHardLinkPairNeedsReview {
        return Ok(false);
    }
    let Some(evidence) = assessment.evidence else {
        return Ok(false);
    };
    if attempt.source_path != evidence.source_path.to_string_lossy()
        || attempt.destination_path != evidence.destination_path.to_string_lossy()
        || attempt.expected_hash != evidence.expected_hash
        || attempt.expected_size != evidence.expected_size
        || attempt.backup_result_id != evidence.backup_result_id
        || attempt.backup_restore_entry_id != evidence.backup_restore_entry_id
        || !attempt.source_regular_file
        || !attempt.source_entry_non_symlink
        || !attempt.destination_entry_non_symlink
    {
        return Ok(false);
    }
    attempt.capability.validate_for_fixture_attempt()?;

    Ok(same_physical_file_identity(
        Path::new(&attempt.source_path),
        Path::new(&attempt.destination_path),
    )? == Some(true))
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureSortingAuthorization {
    plan_hash: String,
    source_path: String,
    destination_path: String,
    source_hash: String,
    source_size: u64,
    backup_result_id: i64,
    backup_restore_entry_id: i64,
}

fn ensure_fixture_sorting_authorization_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS fixture_sorting_authorizations (
            apply_plan_id INTEGER NOT NULL,
            apply_plan_item_id INTEGER NOT NULL,
            plan_hash TEXT NOT NULL,
            source_path TEXT NOT NULL,
            destination_path TEXT NOT NULL,
            source_hash TEXT NOT NULL,
            source_size INTEGER NOT NULL CHECK(source_size >= 0),
            backup_result_id INTEGER NOT NULL,
            backup_restore_entry_id INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (apply_plan_id, apply_plan_item_id)
        );",
    )?;
    Ok(())
}

fn record_fixture_sorting_authorization(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    backup_result_id: i64,
    backup_restore_entry_id: i64,
) -> AppResult<FixtureSortingAuthorization> {
    ensure_fixture_sorting_authorization_schema(connection)?;
    let item = load_plan_item_scope(connection, request.apply_plan_id, request.apply_plan_item_id)?;
    if item.blocked || item.review_only || item.action_kind != "suggest_move" {
        return Err(AppError::Message(
            "Fixture sorting authorization requires one unblocked suggest_move item.".to_owned(),
        ));
    }
    let hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, request.apply_plan_id)?;
    if !hash_check.is_valid {
        return Err(AppError::Message(
            "Fixture sorting authorization requires an unchanged saved ApplyPlan fingerprint."
                .to_owned(),
        ));
    }
    let plan = apply_plan_persistence::get_apply_plan(connection, request.apply_plan_id)?
        .ok_or_else(|| AppError::Message("Fixture sorting ApplyPlan disappeared.".to_owned()))?;
    let plan_hash = plan
        .plan_hash
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Message("Fixture sorting ApplyPlan has no fingerprint.".to_owned()))?;
    let indexed = load_indexed_source_scope(connection, item.file_id)?;
    let source_hash = indexed
        .hash
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Message("Fixture sorting source has no indexed fingerprint.".to_owned()))?;
    if indexed.size < 0 {
        return Err(AppError::Message(
            "Fixture sorting source has an invalid indexed size.".to_owned(),
        ));
    }
    let source_path = resolve_fixture_candidate(Path::new(&item.current_path), "sorting authorization source")?;
    let indexed_path = resolve_fixture_candidate(Path::new(&indexed.path), "sorting authorization indexed source")?;
    let destination_path = resolve_fixture_candidate(
        Path::new(&item.destination_path),
        "sorting authorization destination",
    )?;
    if source_path != indexed_path {
        return Err(AppError::Message(
            "Fixture sorting authorization source no longer matches Library ownership."
                .to_owned(),
        ));
    }

    let authorization = FixtureSortingAuthorization {
        plan_hash,
        source_path: source_path.to_string_lossy().to_string(),
        destination_path: destination_path.to_string_lossy().to_string(),
        source_hash,
        source_size: indexed.size as u64,
        backup_result_id,
        backup_restore_entry_id,
    };
    let inserted = connection.execute(
        "INSERT INTO fixture_sorting_authorizations (
            apply_plan_id, apply_plan_item_id, plan_hash, source_path, destination_path,
            source_hash, source_size, backup_result_id, backup_restore_entry_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            request.apply_plan_id,
            request.apply_plan_item_id,
            authorization.plan_hash,
            authorization.source_path,
            authorization.destination_path,
            authorization.source_hash,
            authorization.source_size as i64,
            authorization.backup_result_id,
            authorization.backup_restore_entry_id,
        ],
    )?;
    if inserted != 1 {
        return Err(AppError::Message(
            "Fixture sorting authorization could not be recorded exactly once.".to_owned(),
        ));
    }
    Ok(authorization)
}

fn load_valid_fixture_sorting_authorization(
    connection: &Connection,
    request: &FixtureReconciliationRequest,
    item: &FixtureMovePlanItemScope,
) -> AppResult<Option<FixtureSortingAuthorization>> {
    let table_exists = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'fixture_sorting_authorizations'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    if !table_exists {
        return Ok(None);
    }
    let authorization = connection
        .query_row(
            "SELECT plan_hash, source_path, destination_path, source_hash, source_size,
                    backup_result_id, backup_restore_entry_id
             FROM fixture_sorting_authorizations
             WHERE apply_plan_id = ?1 AND apply_plan_item_id = ?2",
            params![request.apply_plan_id, request.apply_plan_item_id],
            |row| {
                let source_size = row.get::<_, i64>(4)?;
                if source_size < 0 {
                    return Err(rusqlite::Error::IntegralValueOutOfRange(4, source_size));
                }
                Ok(FixtureSortingAuthorization {
                    plan_hash: row.get(0)?,
                    source_path: row.get(1)?,
                    destination_path: row.get(2)?,
                    source_hash: row.get(3)?,
                    source_size: source_size as u64,
                    backup_result_id: row.get(5)?,
                    backup_restore_entry_id: row.get(6)?,
                })
            },
        )
        .optional()?;
    let Some(authorization) = authorization else {
        return Ok(None);
    };
    let hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, request.apply_plan_id)?;
    if !hash_check.is_valid {
        return Ok(None);
    }
    let plan = apply_plan_persistence::get_apply_plan(connection, request.apply_plan_id)?
        .ok_or_else(|| AppError::Message("Fixture sorting ApplyPlan disappeared.".to_owned()))?;
    let Some(current_plan_hash) = plan.plan_hash.as_deref() else {
        return Ok(None);
    };
    let indexed = load_indexed_source_scope(connection, item.file_id)?;
    let Some(indexed_hash) = indexed.hash.as_deref() else {
        return Ok(None);
    };
    if indexed.size < 0 {
        return Ok(None);
    }
    let source_path = resolve_fixture_candidate(Path::new(&item.current_path), "authorized sorting source")?;
    let indexed_path = resolve_fixture_candidate(Path::new(&indexed.path), "authorized indexed sorting source")?;
    let destination_path = resolve_fixture_candidate(
        Path::new(&item.destination_path),
        "authorized sorting destination",
    )?;
    let authorized_source = resolve_fixture_candidate(
        Path::new(&authorization.source_path),
        "recorded sorting authorization source",
    )?;
    let authorized_destination = resolve_fixture_candidate(
        Path::new(&authorization.destination_path),
        "recorded sorting authorization destination",
    )?;
    if item.blocked
        || item.review_only
        || item.action_kind != "suggest_move"
        || authorization.plan_hash != current_plan_hash
        || authorization.source_hash != indexed_hash
        || authorization.source_size != indexed.size as u64
        || source_path != indexed_path
        || authorized_source != source_path
        || authorized_destination != destination_path
    {
        return Ok(None);
    }
    Ok(Some(authorization))
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
    let sorting_authorization = if item.action_kind == "suggest_move" {
        load_valid_fixture_sorting_authorization(connection, request, &item)?
    } else {
        None
    };
    if item.blocked
        || item.review_only
        || (item.action_kind != "move" && sorting_authorization.is_none())
    {
        return Ok(ambiguous_reconciliation(
            "Persisted ApplyPlan item is not an unblocked move or a fixture-authorized sorting suggestion.",
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
    if let Some(authorization) = sorting_authorization.as_ref() {
        if authorization.backup_result_id != backup_result.id
            || authorization.backup_restore_entry_id != backup_entry.id
        {
            return Ok(ambiguous_reconciliation(
                "Fixture sorting authorization does not match the verified backup chain.",
            ));
        }
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
        core::{
            apply_plan_dry_run, apply_plan_persistence, apply_plan_provenance,
            apply_plan_results::{
                create_apply_plan_run_log, list_apply_plan_restore_entries,
                list_apply_plan_result_logs,
            },
            apply_plan_validation,
        },
        database,
        models::{
            ApplyPlanDryRunItemStatus, ApplyPlanRestoreEntryStatus, ApplyPlanResultLogStatus,
            ApplyPlanValidationPreviewStatus, ApplyPlanValidationStatus,
            BuildApplyPlanFromStagingPlanRequest, CreateApplyPlanRunLogRequest,
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            ListApplyPlanRestoreEntriesRequest, ListApplyPlanResultLogsRequest,
            PreviewApplyPlanDryRunRequest, PreviewApplyPlanValidationRequest,
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

    #[cfg(target_os = "macos")]
    struct SortingFixtureContext {
        context: FixtureContext,
        settings: LibrarySettings,
        file_id: i64,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SortingBatchFixtureItem {
        file_id: i64,
        item_id: i64,
        source_path: PathBuf,
        destination_path: PathBuf,
        source_hash: String,
        source_size: u64,
    }

    #[cfg(target_os = "macos")]
    struct SortingBatchFixtureContext {
        _temp: TempDir,
        fixture_root: PathBuf,
        mods_root: PathBuf,
        backup_root: PathBuf,
        destination_dir: PathBuf,
        settings: LibrarySettings,
        plan_id: i64,
        run_id: i64,
        items: Vec<SortingBatchFixtureItem>,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FixtureSortingBatchReceiptItem {
        item_order: usize,
        apply_plan_item_id: i64,
        file_id: i64,
        source_path: PathBuf,
        destination_path: PathBuf,
        source_hash: String,
        source_size: u64,
        backup_result_id: i64,
        backup_restore_entry_id: i64,
        backup_path: PathBuf,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FixtureSortingBatchReceipt {
        run_id: i64,
        apply_plan_id: i64,
        plan_hash: String,
        batch_hash: String,
        items: Vec<FixtureSortingBatchReceiptItem>,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FixtureSortingBatchApplyOutcome {
        completed_item_ids: Vec<i64>,
        pending_item_ids: Vec<i64>,
        stopped_before_item_id: Option<i64>,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FixtureSortingBatchDirectoryRecord {
        apply_plan_run_id: i64,
        apply_plan_id: i64,
        batch_hash: String,
        directory_path: PathBuf,
        parent_path: PathBuf,
        device_id: u64,
        inode: u64,
        state: FixtureSortingDirectoryState,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FixtureSortingDirectoryState {
        PreparedBeforeCreate,
        CreatedVerified,
        OwnershipUnknownKeep,
        RetainedNonEmpty,
        RetainedIdentityChanged,
        CleanupComplete,
    }

    #[cfg(target_os = "macos")]
    impl FixtureSortingDirectoryState {
        fn as_str(self) -> &'static str {
            match self {
                Self::PreparedBeforeCreate => "prepared_before_create",
                Self::CreatedVerified => "created_verified",
                Self::OwnershipUnknownKeep => "ownership_unknown_keep",
                Self::RetainedNonEmpty => "retained_non_empty",
                Self::RetainedIdentityChanged => "retained_identity_changed",
                Self::CleanupComplete => "cleanup_complete",
            }
        }

        fn from_db(value: &str) -> AppResult<Self> {
            match value {
                "prepared_before_create" => Ok(Self::PreparedBeforeCreate),
                "created_verified" => Ok(Self::CreatedVerified),
                "ownership_unknown_keep" => Ok(Self::OwnershipUnknownKeep),
                "retained_non_empty" => Ok(Self::RetainedNonEmpty),
                "retained_identity_changed" => Ok(Self::RetainedIdentityChanged),
                "cleanup_complete" => Ok(Self::CleanupComplete),
                other => Err(AppError::Message(format!(
                    "Unknown fixture sorting directory state: {other}"
                ))),
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct FixtureSortingDirectoryRecord {
        plan_hash: String,
        backup_result_id: i64,
        backup_restore_entry_id: i64,
        directory_path: PathBuf,
        parent_path: PathBuf,
        device_id: Option<u64>,
        inode: Option<u64>,
        state: FixtureSortingDirectoryState,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FixtureSortingDirectoryFaultPoint {
        None,
        AfterCreateBeforeOwnershipFinalize,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FixtureSortingDirectoryCreateOutcome {
        ExistingNotOwned,
        CreatedOwned,
        OwnershipUnknownKeep,
    }

    #[cfg(target_os = "macos")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FixtureSortingDirectoryCleanupOutcome {
        NotOwned,
        Removed,
        AlreadyAbsent,
        OwnershipUnknownKeep,
        RetainedNonEmpty,
        RetainedIdentityChanged,
    }

    #[cfg(target_os = "macos")]
    fn ensure_fixture_sorting_directory_schema(connection: &Connection) -> AppResult<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS fixture_sorting_created_directories (
                apply_plan_id INTEGER NOT NULL,
                apply_plan_item_id INTEGER NOT NULL,
                plan_hash TEXT NOT NULL,
                backup_result_id INTEGER NOT NULL,
                backup_restore_entry_id INTEGER NOT NULL,
                directory_path TEXT NOT NULL,
                parent_path TEXT NOT NULL,
                device_id INTEGER,
                inode INTEGER,
                state TEXT NOT NULL CHECK(state IN (
                    'prepared_before_create',
                    'created_verified',
                    'ownership_unknown_keep',
                    'retained_non_empty',
                    'retained_identity_changed',
                    'cleanup_complete'
                )),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (apply_plan_id, apply_plan_item_id)
            );",
        )?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn checked_sqlite_identity(value: u64, label: &str) -> AppResult<i64> {
        i64::try_from(value).map_err(|_| {
            AppError::Message(format!(
                "Fixture sorting directory {label} is too large for SQLite evidence."
            ))
        })
    }

    #[cfg(target_os = "macos")]
    fn load_fixture_sorting_directory_record(
        connection: &Connection,
        plan_id: i64,
        item_id: i64,
    ) -> AppResult<Option<FixtureSortingDirectoryRecord>> {
        ensure_fixture_sorting_directory_schema(connection)?;
        connection
            .query_row(
                "SELECT plan_hash, backup_result_id, backup_restore_entry_id,
                        directory_path, parent_path, device_id, inode, state
                 FROM fixture_sorting_created_directories
                 WHERE apply_plan_id = ?1 AND apply_plan_item_id = ?2",
                params![plan_id, item_id],
                |row| {
                    let device_id = row.get::<_, Option<i64>>(5)?;
                    let inode = row.get::<_, Option<i64>>(6)?;
                    if let Some(value) = device_id.filter(|value| *value < 0) {
                        return Err(rusqlite::Error::IntegralValueOutOfRange(5, value));
                    }
                    if let Some(value) = inode.filter(|value| *value < 0) {
                        return Err(rusqlite::Error::IntegralValueOutOfRange(6, value));
                    }
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        device_id.map(|value| value as u64),
                        inode.map(|value| value as u64),
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()?
            .map(
                |(
                    plan_hash,
                    backup_result_id,
                    backup_restore_entry_id,
                    directory_path,
                    parent_path,
                    device_id,
                    inode,
                    state,
                )| {
                    Ok(FixtureSortingDirectoryRecord {
                        plan_hash,
                        backup_result_id,
                        backup_restore_entry_id,
                        directory_path: PathBuf::from(directory_path),
                        parent_path: PathBuf::from(parent_path),
                        device_id,
                        inode,
                        state: FixtureSortingDirectoryState::from_db(&state)?,
                    })
                },
            )
            .transpose()
    }

    #[cfg(target_os = "macos")]
    fn mark_fixture_sorting_directory_state(
        connection: &Connection,
        plan_id: i64,
        item_id: i64,
        expected: FixtureSortingDirectoryState,
        next: FixtureSortingDirectoryState,
    ) -> AppResult<()> {
        let changed = connection.execute(
            "UPDATE fixture_sorting_created_directories
             SET state = ?1, updated_at = CURRENT_TIMESTAMP
             WHERE apply_plan_id = ?2 AND apply_plan_item_id = ?3 AND state = ?4",
            params![next.as_str(), plan_id, item_id, expected.as_str()],
        )?;
        if changed != 1 {
            return Err(AppError::Message(
                "Fixture sorting directory state changed unexpectedly.".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn fixture_directory_identity(path: &Path) -> AppResult<(u64, u64)> {
        use std::os::unix::fs::MetadataExt;

        let metadata = fs::symlink_metadata(path).map_err(|error| {
            AppError::Message(format!(
                "Fixture sorting directory could not be inspected: {error}"
            ))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(AppError::Message(
                "Fixture sorting destination parent must be a real directory, not a symlink or other entry."
                    .to_owned(),
            ));
        }
        Ok((metadata.dev(), metadata.ino()))
    }

    #[cfg(target_os = "macos")]
    fn sorting_destination_directory_scope(
        connection: &Connection,
        sorting: &SortingFixtureContext,
    ) -> AppResult<(String, PathBuf, PathBuf)> {
        let context = &sorting.context;
        let hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, context.plan_id)?;
        if !hash_check.is_valid {
            return Err(AppError::Message(
                "Fixture sorting directory requires an unchanged ApplyPlan fingerprint."
                    .to_owned(),
            ));
        }
        let plan = apply_plan_persistence::get_apply_plan(connection, context.plan_id)?
            .ok_or_else(|| AppError::Message("Fixture sorting ApplyPlan disappeared.".to_owned()))?;
        let plan_hash = plan
            .plan_hash
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                AppError::Message("Fixture sorting ApplyPlan has no fingerprint.".to_owned())
            })?;
        let item = load_plan_item_scope(connection, context.plan_id, context.item_id)?;
        if item.blocked || item.review_only || item.action_kind != "suggest_move" {
            return Err(AppError::Message(
                "Fixture sorting directory requires one unblocked suggest_move item.".to_owned(),
            ));
        }
        let mods_root_text = sorting
            .settings
            .mods_path
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::Message("Fixture sorting Mods root is missing.".to_owned()))?;
        let mods_root = canonicalize_existing_dir(Path::new(mods_root_text), "sorting Mods root")?;
        let fixture_root = canonicalize_existing_dir(&context.fixture_root, "sorting fixture root")?;
        let destination_path = PathBuf::from(&item.destination_path);
        let raw_directory = destination_path.parent().ok_or_else(|| {
            AppError::Message("Fixture sorting destination has no parent directory.".to_owned())
        })?;
        let directory_path =
            resolve_fixture_candidate(raw_directory, "sorting destination directory")?;
        ensure_under_root(&directory_path, &mods_root, "sorting destination directory")?;
        ensure_under_root(&directory_path, &fixture_root, "sorting destination directory")?;
        let parent_path = directory_path.parent().ok_or_else(|| {
            AppError::Message("Fixture sorting destination directory has no parent.".to_owned())
        })?;
        if parent_path != mods_root {
            return Err(AppError::Message(
                "Fixture sorting folder proof only permits one immediate organization folder under Mods."
                    .to_owned(),
            ));
        }
        Ok((plan_hash, directory_path, mods_root))
    }

    #[cfg(target_os = "macos")]
    fn ensure_fixture_sorting_destination_directory_with_fault(
        connection: &Connection,
        sorting: &SortingFixtureContext,
        backup: &FixtureBackupPrototypeSuccess,
        fault: FixtureSortingDirectoryFaultPoint,
    ) -> AppResult<FixtureSortingDirectoryCreateOutcome> {
        ensure_fixture_sorting_directory_schema(connection)?;
        let context = &sorting.context;
        let (plan_hash, directory_path, parent_path) =
            sorting_destination_directory_scope(connection, sorting)?;

        if let Some(record) = load_fixture_sorting_directory_record(
            connection,
            context.plan_id,
            context.item_id,
        )? {
            if record.plan_hash != plan_hash
                || record.backup_result_id != backup.result_log_id
                || record.backup_restore_entry_id != backup.restore_entry_id
                || record.directory_path != directory_path
                || record.parent_path != parent_path
            {
                return Err(AppError::Message(
                    "Fixture sorting directory evidence no longer matches this plan and backup."
                        .to_owned(),
                ));
            }
            match record.state {
                FixtureSortingDirectoryState::PreparedBeforeCreate => {
                    if directory_path.exists() {
                        fixture_directory_identity(&directory_path)?;
                        mark_fixture_sorting_directory_state(
                            connection,
                            context.plan_id,
                            context.item_id,
                            FixtureSortingDirectoryState::PreparedBeforeCreate,
                            FixtureSortingDirectoryState::OwnershipUnknownKeep,
                        )?;
                        return Ok(FixtureSortingDirectoryCreateOutcome::OwnershipUnknownKeep);
                    }
                }
                FixtureSortingDirectoryState::CreatedVerified => {
                    let (device_id, inode) = fixture_directory_identity(&directory_path)?;
                    if record.device_id == Some(device_id) && record.inode == Some(inode) {
                        return Ok(FixtureSortingDirectoryCreateOutcome::CreatedOwned);
                    }
                    return Err(AppError::Message(
                        "Fixture sorting directory identity changed before move execution."
                            .to_owned(),
                    ));
                }
                FixtureSortingDirectoryState::OwnershipUnknownKeep
                | FixtureSortingDirectoryState::RetainedNonEmpty
                | FixtureSortingDirectoryState::RetainedIdentityChanged => {
                    fixture_directory_identity(&directory_path)?;
                    return Ok(FixtureSortingDirectoryCreateOutcome::OwnershipUnknownKeep);
                }
                FixtureSortingDirectoryState::CleanupComplete => {
                    return Err(AppError::Message(
                        "Fixture sorting directory cleanup already completed for this plan item."
                            .to_owned(),
                    ));
                }
            }
        } else if directory_path.exists() {
            fixture_directory_identity(&directory_path)?;
            return Ok(FixtureSortingDirectoryCreateOutcome::ExistingNotOwned);
        } else {
            let inserted = connection.execute(
                "INSERT INTO fixture_sorting_created_directories (
                    apply_plan_id, apply_plan_item_id, plan_hash, backup_result_id,
                    backup_restore_entry_id, directory_path, parent_path, state
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    context.plan_id,
                    context.item_id,
                    plan_hash,
                    backup.result_log_id,
                    backup.restore_entry_id,
                    directory_path.to_string_lossy().to_string(),
                    parent_path.to_string_lossy().to_string(),
                    FixtureSortingDirectoryState::PreparedBeforeCreate.as_str(),
                ],
            )?;
            if inserted != 1 {
                return Err(AppError::Message(
                    "Fixture sorting directory intent could not be recorded exactly once."
                        .to_owned(),
                ));
            }
        }

        if directory_path.exists() {
            return Err(AppError::Message(
                "Fixture sorting destination directory appeared before guarded creation."
                    .to_owned(),
            ));
        }
        fs::create_dir(&directory_path).map_err(|error| {
            AppError::Message(format!(
                "Fixture sorting destination directory could not be created: {error}"
            ))
        })?;
        if fault == FixtureSortingDirectoryFaultPoint::AfterCreateBeforeOwnershipFinalize {
            return Err(AppError::Message(
                "Injected interruption after fixture sorting directory creation before ownership finalization."
                    .to_owned(),
            ));
        }

        let (device_id, inode) = fixture_directory_identity(&directory_path)?;
        let changed = connection.execute(
            "UPDATE fixture_sorting_created_directories
             SET device_id = ?1, inode = ?2, state = ?3, updated_at = CURRENT_TIMESTAMP
             WHERE apply_plan_id = ?4 AND apply_plan_item_id = ?5 AND state = ?6",
            params![
                checked_sqlite_identity(device_id, "device id")?,
                checked_sqlite_identity(inode, "inode")?,
                FixtureSortingDirectoryState::CreatedVerified.as_str(),
                context.plan_id,
                context.item_id,
                FixtureSortingDirectoryState::PreparedBeforeCreate.as_str(),
            ],
        )?;
        if changed != 1 {
            return Err(AppError::Message(
                "Fixture sorting directory ownership could not be finalized exactly once."
                    .to_owned(),
            ));
        }
        Ok(FixtureSortingDirectoryCreateOutcome::CreatedOwned)
    }

    #[cfg(target_os = "macos")]
    fn fixture_directory_has_indexed_children(
        connection: &Connection,
        directory_path: &Path,
    ) -> AppResult<bool> {
        let mut statement = connection.prepare("SELECT path FROM files")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            let path = PathBuf::from(row?);
            if path.starts_with(directory_path) {
                return Ok(true);
            }
            if let Ok(resolved_path) =
                resolve_fixture_candidate(&path, "sorting indexed child path")
            {
                if resolved_path.starts_with(directory_path) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    #[cfg(target_os = "macos")]
    fn cleanup_fixture_sorting_destination_directory(
        connection: &Connection,
        sorting: &SortingFixtureContext,
    ) -> AppResult<FixtureSortingDirectoryCleanupOutcome> {
        let context = &sorting.context;
        let Some(record) = load_fixture_sorting_directory_record(
            connection,
            context.plan_id,
            context.item_id,
        )? else {
            return Ok(FixtureSortingDirectoryCleanupOutcome::NotOwned);
        };
        let (plan_hash, directory_path, parent_path) =
            sorting_destination_directory_scope(connection, sorting)?;
        if record.plan_hash != plan_hash
            || record.directory_path != directory_path
            || record.parent_path != parent_path
        {
            return Err(AppError::Message(
                "Fixture sorting directory cleanup evidence no longer matches the saved plan."
                    .to_owned(),
            ));
        }

        match record.state {
            FixtureSortingDirectoryState::PreparedBeforeCreate
            | FixtureSortingDirectoryState::OwnershipUnknownKeep => {
                return Ok(FixtureSortingDirectoryCleanupOutcome::OwnershipUnknownKeep);
            }
            FixtureSortingDirectoryState::RetainedNonEmpty => {
                return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty);
            }
            FixtureSortingDirectoryState::RetainedIdentityChanged => {
                return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedIdentityChanged);
            }
            FixtureSortingDirectoryState::CleanupComplete => {
                return Ok(FixtureSortingDirectoryCleanupOutcome::AlreadyAbsent);
            }
            FixtureSortingDirectoryState::CreatedVerified => {}
        }

        if !directory_path.exists() {
            mark_fixture_sorting_directory_state(
                connection,
                context.plan_id,
                context.item_id,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::CleanupComplete,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::AlreadyAbsent);
        }

        let current_identity = fixture_directory_identity(&directory_path)?;
        if record.device_id != Some(current_identity.0) || record.inode != Some(current_identity.1) {
            mark_fixture_sorting_directory_state(
                connection,
                context.plan_id,
                context.item_id,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::RetainedIdentityChanged,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedIdentityChanged);
        }
        if fixture_directory_has_indexed_children(connection, &directory_path)?
            || fs::read_dir(&directory_path)
                .map_err(|error| {
                    AppError::Message(format!(
                        "Fixture sorting destination directory could not be read for cleanup: {error}"
                    ))
                })?
                .next()
                .is_some()
        {
            mark_fixture_sorting_directory_state(
                connection,
                context.plan_id,
                context.item_id,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::RetainedNonEmpty,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty);
        }

        let final_identity = fixture_directory_identity(&directory_path)?;
        if final_identity != current_identity {
            mark_fixture_sorting_directory_state(
                connection,
                context.plan_id,
                context.item_id,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::RetainedIdentityChanged,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedIdentityChanged);
        }
        if let Err(error) = fs::remove_dir(&directory_path) {
            if fs::read_dir(&directory_path)
                .ok()
                .and_then(|mut entries| entries.next())
                .is_some()
            {
                mark_fixture_sorting_directory_state(
                    connection,
                    context.plan_id,
                    context.item_id,
                    FixtureSortingDirectoryState::CreatedVerified,
                    FixtureSortingDirectoryState::RetainedNonEmpty,
                )?;
                return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty);
            }
            return Err(AppError::Message(format!(
                "Fixture sorting destination directory could not be removed safely: {error}"
            )));
        }
        mark_fixture_sorting_directory_state(
            connection,
            context.plan_id,
            context.item_id,
            FixtureSortingDirectoryState::CreatedVerified,
            FixtureSortingDirectoryState::CleanupComplete,
        )?;
        Ok(FixtureSortingDirectoryCleanupOutcome::Removed)
    }

    #[cfg(target_os = "macos")]
    fn setup_sorting_fixture(
        connection: &mut Connection,
        source_bytes: &[u8],
    ) -> SortingFixtureContext {
        setup_sorting_fixture_with_destination_folder(connection, source_bytes, true)
    }

    #[cfg(target_os = "macos")]
    fn setup_sorting_fixture_with_destination_folder(
        connection: &mut Connection,
        source_bytes: &[u8],
        destination_folder_exists: bool,
    ) -> SortingFixtureContext {
        let temp = tempdir().expect("sorting tempdir");
        let fixture_root = temp.path().to_path_buf();
        let mods_root = fixture_root.join("Mods");
        let source_dir = mods_root.join("Unsorted");
        let destination_dir = mods_root.join("Gameplay");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("sorting source dir");
        if destination_folder_exists {
            fs::create_dir(&destination_dir).expect("sorting destination dir");
        }
        fs::create_dir_all(&backup_root).expect("sorting backup dir");
        let source_path = source_dir.join("sample.package");
        fs::write(&source_path, source_bytes).expect("sorting source fixture");

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, hash, size, modified_at, kind, confidence,
                    source_location, relative_depth, safety_notes, parser_warnings, insights
                 ) VALUES (?1, 'sample.package', 'package', ?2, ?3, '2026-08-08T20:00:00Z',
                    'Gameplay', 0.95, 'mods', 1, '[]', '[]', '{}')",
                params![
                    source_path.to_string_lossy().to_string(),
                    bytes_hash(source_bytes),
                    source_bytes.len() as i64,
                ],
            )
            .expect("insert sorting source");
        let file_id = connection.last_insert_rowid();

        let settings = LibrarySettings {
            mods_path: Some(mods_root.to_string_lossy().to_string()),
            ..Default::default()
        };
        let saved = apply_plan_persistence::build_apply_plan_from_staging_plan(
            connection,
            &settings,
            BuildApplyPlanFromStagingPlanRequest {
                preview_request: GenerateSortingPreviewPlanRequest {
                    scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                        file_ids: vec![file_id],
                    },
                    folder_config: None,
                    context_trail: Vec::new(),
                },
                source_plan_kind: None,
                folder_config: None,
                context_trail: Vec::new(),
            },
        )
        .expect("build sorting ApplyPlan");
        let plan_id = saved.plan_id;
        let item_id = connection
            .query_row(
                "SELECT id FROM apply_plan_items WHERE apply_plan_id = ?1 ORDER BY id ASC LIMIT 1",
                params![plan_id],
                |row| row.get::<_, i64>(0),
            )
            .expect("sorting ApplyPlan item id");
        let item = load_plan_item_scope(connection, plan_id, item_id)
            .expect("sorting ApplyPlan item scope");
        assert_eq!(item.file_id, file_id);
        assert_eq!(item.action_kind, "suggest_move");
        assert!(!item.blocked);
        assert!(!item.review_only);
        assert_eq!(
            resolve_fixture_candidate(Path::new(&item.current_path), "sorting source")
                .expect("resolve sorting source"),
            resolve_fixture_candidate(&source_path, "sorting fixture source")
                .expect("resolve sorting fixture source")
        );
        let expected_destination = mods_root.join("Gameplay").join("sample.package");
        let destination_path = PathBuf::from(&item.destination_path);
        assert_eq!(destination_path, expected_destination);

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
                summary: Some("Fixture-only sorting Apply/Undo proof.".to_owned()),
            },
        )
        .expect("create sorting run")
        .id;

        SortingFixtureContext {
            context: FixtureContext {
                _temp: temp,
                fixture_root,
                backup_root,
                source_path,
                destination_path,
                plan_id,
                item_id,
                run_id,
            },
            settings,
            file_id,
        }
    }

    #[cfg(target_os = "macos")]
    fn run_verified_sorting_preflight_and_backup(
        connection: &Connection,
        sorting: &SortingFixtureContext,
    ) -> AppResult<FixtureBackupPrototypeSuccess> {
        let context = &sorting.context;
        let plan = apply_plan_persistence::get_apply_plan(connection, context.plan_id)?
            .ok_or_else(|| AppError::Message("Sorting fixture ApplyPlan disappeared.".to_owned()))?;
        if plan.source_plan_kind
            != apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
            || plan.preview_snapshot_id.is_none()
            || plan
                .preview_snapshot_hash
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AppError::Message(
                "Sorting fixture requires a backend-generated preview snapshot.".to_owned(),
            ));
        }
        let hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, plan.id)?;
        if !hash_check.is_valid {
            return Err(AppError::Message(format!(
                "Sorting fixture ApplyPlan provenance is stale: {}",
                hash_check.message
            )));
        }

        let validation = apply_plan_validation::preview_apply_plan_validation(
            connection,
            &sorting.settings,
            PreviewApplyPlanValidationRequest {
                plan_id: context.plan_id,
            },
        )?;
        let validation_item = validation
            .items
            .iter()
            .find(|item| item.item_id == context.item_id)
            .ok_or_else(|| {
                AppError::Message("Sorting fixture validation omitted the saved item.".to_owned())
            })?;
        let backup_is_only_plan_blocker = validation.status == ApplyPlanValidationPreviewStatus::Blocked
            && !validation.can_proceed_to_confirmation
            && validation.summary.total_items == 1
            && validation.summary.blocked_items == 0
            && validation.summary.review_only_items == 0
            && validation.summary.conflict_items == 0
            && validation.summary.stale_items == 0
            && validation.summary.missing_source_items == 0
            && validation.summary.destination_conflict_items == 0
            && validation.summary.backup_blocked_items == 1;
        if !backup_is_only_plan_blocker
            || validation_item.validation_status != ApplyPlanValidationStatus::ValidPreviewOnly
            || validation_item.blocked
            || validation_item.review_only
        {
            return Err(AppError::Message(format!(
                "Sorting fixture did not reach the expected backup-only validation stop: preview={:?}, item={:?}, summary=blocked:{} review:{} conflicts:{} stale:{} missing:{} destination:{} backup:{}, reasons={:?}",
                validation.status,
                validation_item.validation_status,
                validation.summary.blocked_items,
                validation.summary.review_only_items,
                validation.summary.conflict_items,
                validation.summary.stale_items,
                validation.summary.missing_source_items,
                validation.summary.destination_conflict_items,
                validation.summary.backup_blocked_items,
                validation_item.reasons
            )));
        }

        let dry_run = apply_plan_dry_run::preview_apply_plan_dry_run(
            connection,
            &sorting.settings,
            PreviewApplyPlanDryRunRequest {
                plan_id: context.plan_id,
            },
        )?;
        let dry_run_item = dry_run
            .items
            .iter()
            .find(|item| item.item_id == context.item_id)
            .ok_or_else(|| {
                AppError::Message("Sorting fixture dry-run omitted the saved item.".to_owned())
            })?;
        if dry_run_item.dry_run_status != ApplyPlanDryRunItemStatus::WouldRequireBackup
            || dry_run_item.can_apply
        {
            return Err(AppError::Message(
                "Sorting fixture must remain blocked on backup before fixture execution."
                    .to_owned(),
            ));
        }

        let item = load_plan_item_scope(connection, context.plan_id, context.item_id)?;
        if item.action_kind != "suggest_move" || item.blocked || item.review_only {
            return Err(AppError::Message(
                "Sorting fixture requires one unblocked backend suggest_move item.".to_owned(),
            ));
        }
        let source_path = resolve_fixture_candidate(Path::new(&item.current_path), "sorting currentPath")?;
        let indexed = load_indexed_source_scope(connection, item.file_id)?;
        let indexed_path = resolve_fixture_candidate(Path::new(&indexed.path), "sorting indexed source")?;
        if indexed_path != source_path {
            return Err(AppError::Message(
                "Sorting fixture indexed source no longer matches the saved plan.".to_owned(),
            ));
        }
        let source_metadata = fs::metadata(&source_path).map_err(|error| {
            AppError::Message(format!("Sorting fixture source could not be inspected: {error}"))
        })?;
        let source_hash = hash_file(&source_path)?;
        let indexed_hash = indexed.hash.as_deref().ok_or_else(|| {
            AppError::Message("Sorting fixture requires an indexed source hash.".to_owned())
        })?;
        if !source_metadata.is_file()
            || indexed.size < 0
            || indexed.size as u64 != source_metadata.len()
            || indexed_hash != source_hash
        {
            return Err(AppError::Message(
                "Sorting fixture source changed after the sorting preview; regenerate before applying."
                    .to_owned(),
            ));
        }

        match run_fixture_backup_prototype(
            connection,
            FixtureBackupPrototypeRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: Some(context.item_id),
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                source_path: context.source_path.clone(),
                backup_root: context.backup_root.clone(),
                operation_kind: BACKUP_OPERATION_KIND.to_owned(),
            },
        )? {
            FixtureBackupPrototypeOutcome::Verified(success) => Ok(success),
            FixtureBackupPrototypeOutcome::FailedBeforeChange(failure) => Err(AppError::Message(
                format!("Sorting fixture backup failed before change: {}", failure.error_message),
            )),
        }
    }

    #[cfg(target_os = "macos")]
    fn run_verified_sorting_backup_gate(
        connection: &Connection,
        sorting: &SortingFixtureContext,
    ) -> AppResult<FixtureBackupPrototypeSuccess> {
        let backup = run_verified_sorting_preflight_and_backup(connection, sorting)?;
        ensure_fixture_sorting_destination_directory_with_fault(
            connection,
            sorting,
            &backup,
            FixtureSortingDirectoryFaultPoint::None,
        )?;
        record_fixture_sorting_authorization(
            connection,
            &reconciliation_request(&sorting.context),
            backup.result_log_id,
            backup.restore_entry_id,
        )?;
        Ok(backup)
    }

    #[cfg(target_os = "macos")]
    fn sorting_batch_reconciliation_request(
        batch: &SortingBatchFixtureContext,
        item: &SortingBatchFixtureItem,
    ) -> FixtureReconciliationRequest {
        FixtureReconciliationRequest {
            fixture_mode: true,
            apply_plan_id: batch.plan_id,
            apply_plan_item_id: item.item_id,
            run_id: batch.run_id,
            fixture_root: batch.fixture_root.clone(),
        }
    }

    #[cfg(target_os = "macos")]
    fn setup_sorting_batch_fixture(connection: &mut Connection) -> SortingBatchFixtureContext {
        let temp = tempdir().expect("sorting batch tempdir");
        let fixture_root = temp.path().to_path_buf();
        let mods_root = fixture_root.join("Mods");
        let source_dir = mods_root.join("Unsorted");
        let destination_dir = mods_root.join("Gameplay");
        let backup_root = fixture_root.join("backup");
        fs::create_dir_all(&source_dir).expect("sorting batch source dir");
        fs::create_dir_all(&backup_root).expect("sorting batch backup dir");
        assert!(!destination_dir.exists());

        let fixtures: [(&str, &[u8]); 3] = [
            ("alpha.package", b"sorting batch alpha gameplay package"),
            ("bravo.package", b"sorting batch bravo gameplay package"),
            ("charlie.package", b"sorting batch charlie gameplay package"),
        ];
        let mut source_rows = Vec::new();
        for (filename, bytes) in fixtures {
            let source_path = source_dir.join(filename);
            fs::write(&source_path, bytes).expect("sorting batch source fixture");
            let source_hash = bytes_hash(bytes);
            connection
                .execute(
                    "INSERT INTO files (
                        path, filename, extension, hash, size, modified_at, kind, confidence,
                        source_location, relative_depth, safety_notes, parser_warnings, insights
                     ) VALUES (?1, ?2, 'package', ?3, ?4, '2026-08-09T00:00:00Z',
                        'Gameplay', 0.95, 'mods', 1, '[]', '[]', '{}')",
                    params![
                        source_path.to_string_lossy().to_string(),
                        filename,
                        source_hash,
                        bytes.len() as i64,
                    ],
                )
                .expect("insert sorting batch source");
            source_rows.push((
                connection.last_insert_rowid(),
                source_path,
                source_hash,
                bytes.len() as u64,
            ));
        }

        let settings = LibrarySettings {
            mods_path: Some(mods_root.to_string_lossy().to_string()),
            ..Default::default()
        };
        let saved = apply_plan_persistence::build_apply_plan_from_staging_plan(
            connection,
            &settings,
            BuildApplyPlanFromStagingPlanRequest {
                preview_request: GenerateSortingPreviewPlanRequest {
                    scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                        file_ids: source_rows.iter().map(|row| row.0).collect(),
                    },
                    folder_config: None,
                    context_trail: Vec::new(),
                },
                source_plan_kind: None,
                folder_config: None,
                context_trail: Vec::new(),
            },
        )
        .expect("build sorting batch ApplyPlan");
        let plan_id = saved.plan_id;
        let hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, plan_id)
            .expect("verify sorting batch plan hash");
        assert!(hash_check.is_valid);

        let item_ids = {
            let mut statement = connection
                .prepare(
                    "SELECT id FROM apply_plan_items WHERE apply_plan_id = ?1 ORDER BY id ASC",
                )
                .expect("prepare sorting batch item ids");
            statement
                .query_map(params![plan_id], |row| row.get::<_, i64>(0))
                .expect("query sorting batch item ids")
                .collect::<Result<Vec<_>, _>>()
                .expect("collect sorting batch item ids")
        };
        assert_eq!(item_ids.len(), source_rows.len());

        let mut items = Vec::new();
        for item_id in item_ids {
            let item = load_plan_item_scope(connection, plan_id, item_id)
                .expect("sorting batch plan item scope");
            assert_eq!(item.action_kind, "suggest_move");
            assert!(!item.blocked);
            assert!(!item.review_only);
            let (file_id, source_path, source_hash, source_size) = source_rows
                .iter()
                .find(|row| row.0 == item.file_id)
                .cloned()
                .expect("sorting batch plan item maps to fixture source");
            assert_eq!(PathBuf::from(&item.current_path), source_path);
            let filename = source_path.file_name().expect("sorting batch filename");
            let destination_path = destination_dir.join(filename);
            assert_eq!(PathBuf::from(&item.destination_path), destination_path);
            items.push(SortingBatchFixtureItem {
                file_id,
                item_id,
                source_path,
                destination_path,
                source_hash,
                source_size,
            });
        }

        let run_id = create_apply_plan_run_log(
            connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: Some(items.len() as i64),
                skipped_items: None,
                failed_items: None,
                summary: Some("Fixture-only multi-file sorting safety proof.".to_owned()),
            },
        )
        .expect("create sorting batch run")
        .id;

        SortingBatchFixtureContext {
            _temp: temp,
            fixture_root,
            mods_root,
            backup_root,
            destination_dir,
            settings,
            plan_id,
            run_id,
            items,
        }
    }

    #[cfg(target_os = "macos")]
    fn verify_sorting_batch_read_only_preflight(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
    ) -> AppResult<()> {
        let plan = apply_plan_persistence::get_apply_plan(connection, batch.plan_id)?
            .ok_or_else(|| AppError::Message("Sorting batch ApplyPlan disappeared.".to_owned()))?;
        if plan.source_plan_kind
            != apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
            || plan.preview_snapshot_id.is_none()
            || plan
                .preview_snapshot_hash
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AppError::Message(
                "Sorting batch requires one backend-generated preview snapshot.".to_owned(),
            ));
        }
        let hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, batch.plan_id)?;
        if !hash_check.is_valid {
            return Err(AppError::Message(format!(
                "Sorting batch ApplyPlan provenance is stale: {}",
                hash_check.message
            )));
        }
        if plan.items.len() != batch.items.len() {
            return Err(AppError::Message(
                "Sorting batch item count changed after the preview was saved.".to_owned(),
            ));
        }

        let validation = apply_plan_validation::preview_apply_plan_validation(
            connection,
            &batch.settings,
            PreviewApplyPlanValidationRequest {
                plan_id: batch.plan_id,
            },
        )?;
        let expected_count = batch.items.len() as i64;
        if validation.status != ApplyPlanValidationPreviewStatus::Blocked
            || validation.can_proceed_to_confirmation
            || validation.summary.total_items != expected_count
            || validation.summary.blocked_items != 0
            || validation.summary.review_only_items != 0
            || validation.summary.conflict_items != 0
            || validation.summary.stale_items != 0
            || validation.summary.missing_source_items != 0
            || validation.summary.destination_conflict_items != 0
            || validation.summary.backup_blocked_items != expected_count
            || validation.items.iter().any(|item| {
                item.validation_status != ApplyPlanValidationStatus::ValidPreviewOnly
                    || item.blocked
                    || item.review_only
            })
        {
            return Err(AppError::Message(format!(
                "Sorting batch did not reach the expected all-items backup-only validation stop: preview={:?}, summary=blocked:{} review:{} conflicts:{} stale:{} missing:{} destination:{} backup:{}",
                validation.status,
                validation.summary.blocked_items,
                validation.summary.review_only_items,
                validation.summary.conflict_items,
                validation.summary.stale_items,
                validation.summary.missing_source_items,
                validation.summary.destination_conflict_items,
                validation.summary.backup_blocked_items,
            )));
        }

        let dry_run = apply_plan_dry_run::preview_apply_plan_dry_run(
            connection,
            &batch.settings,
            PreviewApplyPlanDryRunRequest {
                plan_id: batch.plan_id,
            },
        )?;
        if dry_run.items.len() != batch.items.len()
            || dry_run.items.iter().any(|item| {
                item.dry_run_status != ApplyPlanDryRunItemStatus::WouldRequireBackup
                    || item.can_apply
            })
        {
            return Err(AppError::Message(
                "Sorting batch dry-run must keep every item blocked on backup before execution."
                    .to_owned(),
            ));
        }

        let canonical_mods_root = canonicalize_existing_dir(&batch.mods_root, "sorting batch Mods root")?;
        let destination_parent = batch.destination_dir.parent().ok_or_else(|| {
            AppError::Message(
                "Sorting batch destination directory has no parent folder.".to_owned(),
            )
        })?;
        let canonical_destination_parent =
            canonicalize_existing_dir(destination_parent, "sorting batch destination parent")?;
        if canonical_destination_parent != canonical_mods_root {
            return Err(AppError::Message(
                "Sorting batch proof only permits one immediate organization folder under Mods."
                    .to_owned(),
            ));
        }

        let mut source_paths = Vec::new();
        let mut destination_paths = Vec::new();
        for expected in &batch.items {
            let item = load_plan_item_scope(connection, batch.plan_id, expected.item_id)?;
            if item.file_id != expected.file_id
                || item.action_kind != "suggest_move"
                || item.blocked
                || item.review_only
                || PathBuf::from(&item.current_path) != expected.source_path
                || PathBuf::from(&item.destination_path) != expected.destination_path
                || expected.destination_path.parent() != Some(batch.destination_dir.as_path())
            {
                return Err(AppError::Message(
                    "Sorting batch saved item evidence changed before execution.".to_owned(),
                ));
            }

            let indexed = load_indexed_source_scope(connection, expected.file_id)?;
            let indexed_hash = indexed.hash.as_deref().ok_or_else(|| {
                AppError::Message("Sorting batch requires an indexed source hash.".to_owned())
            })?;
            if indexed.size < 0
                || PathBuf::from(&indexed.path) != expected.source_path
                || indexed.size as u64 != expected.source_size
                || indexed_hash != expected.source_hash
            {
                return Err(AppError::Message(
                    "Sorting batch Library evidence changed after the preview; regenerate before applying."
                        .to_owned(),
                ));
            }

            let source_metadata = fs::symlink_metadata(&expected.source_path).map_err(|error| {
                AppError::Message(format!(
                    "Sorting batch source could not be inspected before execution: {error}"
                ))
            })?;
            if !source_metadata.file_type().is_file()
                || source_metadata.file_type().is_symlink()
                || source_metadata.len() != expected.source_size
                || hash_file(&expected.source_path)? != expected.source_hash
            {
                return Err(AppError::Message(
                    "Sorting batch source bytes changed after the preview; regenerate before applying."
                        .to_owned(),
                ));
            }
            if expected.destination_path.exists()
                || existing_path_entry_is_symlink(&expected.destination_path)?
            {
                return Err(AppError::Message(
                    "Sorting batch destination is no longer empty; no batch changes were started."
                        .to_owned(),
                ));
            }

            if source_paths.contains(&expected.source_path)
                || destination_paths.contains(&expected.destination_path)
            {
                return Err(AppError::Message(
                    "Sorting batch contains a duplicate source or destination path.".to_owned(),
                ));
            }
            source_paths.push(expected.source_path.clone());
            destination_paths.push(expected.destination_path.clone());
        }

        if destination_paths
            .iter()
            .any(|destination| source_paths.contains(destination))
        {
            return Err(AppError::Message(
                "Sorting batch source/destination chains are not supported by this safety proof."
                    .to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn load_existing_sorting_batch_backup(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        item: &SortingBatchFixtureItem,
    ) -> AppResult<Option<FixtureBackupPrototypeSuccess>> {
        let detail = apply_plan_results::get_apply_plan_run_log(connection, batch.run_id)?
            .ok_or_else(|| AppError::Message("Sorting batch run log disappeared.".to_owned()))?;
        let entries = detail
            .restore_entries
            .iter()
            .filter(|entry| {
                entry.apply_plan_item_id == Some(item.item_id)
                    && entry.operation_kind == BACKUP_OPERATION_KIND
                    && entry.operation_result_status == ApplyPlanResultLogStatus::PendingLog
                    && entry.restore_status == ApplyPlanRestoreEntryStatus::DesignOnly
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Ok(None);
        }
        if entries.len() != 1 {
            return Err(AppError::Message(
                "Sorting batch found more than one verified backup chain for one item."
                    .to_owned(),
            ));
        }
        let entry = entries[0];
        let result_id = entry.apply_plan_result_id.ok_or_else(|| {
            AppError::Message("Sorting batch backup restore entry lost its result id.".to_owned())
        })?;
        let result = detail
            .results
            .iter()
            .find(|result| result.id == result_id)
            .ok_or_else(|| {
                AppError::Message("Sorting batch backup result disappeared.".to_owned())
            })?;
        if result.apply_plan_item_id != Some(item.item_id)
            || result.operation_kind != BACKUP_OPERATION_KIND
            || result.result_status != ApplyPlanResultLogStatus::PendingLog
            || entry.file_hash_before.as_deref() != Some(item.source_hash.as_str())
            || entry.file_size_before != Some(item.source_size as i64)
        {
            return Err(AppError::Message(
                "Sorting batch existing backup metadata no longer matches the saved item."
                    .to_owned(),
            ));
        }
        let recorded_source = resolve_fixture_candidate(
            Path::new(&entry.original_source_path),
            "sorting batch recorded backup source",
        )?;
        let expected_source = resolve_fixture_candidate(
            &item.source_path,
            "sorting batch expected backup source",
        )?;
        if recorded_source != expected_source
            || resolve_optional_recorded_path(
                result.source_path_at_execution.as_deref(),
                "sorting batch backup result source",
            )? != Some(expected_source)
        {
            return Err(AppError::Message(
                "Sorting batch existing backup source path no longer matches the saved item."
                    .to_owned(),
            ));
        }
        let backup_path = resolve_fixture_candidate(
            Path::new(entry.backup_path.as_deref().ok_or_else(|| {
                AppError::Message("Sorting batch backup entry lost its backup path.".to_owned())
            })?),
            "sorting batch existing backup path",
        )?;
        let canonical_backup_root = canonicalize_existing_dir(&batch.backup_root, "sorting batch backup root")?;
        ensure_under_root(&backup_path, &canonical_backup_root, "sorting batch existing backup")?;
        if resolve_optional_recorded_path(
            result.backup_path.as_deref(),
            "sorting batch backup result path",
        )? != Some(backup_path.clone())
            || observe_expected_file(&backup_path, item.source_size, &item.source_hash)?
                != ObservedFileState::Exact
        {
            return Err(AppError::Message(
                "Sorting batch existing backup path or bytes changed; it will not be reused."
                    .to_owned(),
            ));
        }
        Ok(Some(FixtureBackupPrototypeSuccess {
            backup_path,
            source_size: item.source_size,
            backup_size: item.source_size,
            source_hash: item.source_hash.clone(),
            backup_hash: item.source_hash.clone(),
            backup_verified: true,
            result_log_id: result.id,
            restore_entry_id: entry.id,
        }))
    }

    #[cfg(target_os = "macos")]
    fn ensure_sorting_batch_backups(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
    ) -> AppResult<Vec<FixtureBackupPrototypeSuccess>> {
        let mut backups = Vec::with_capacity(batch.items.len());
        for item in &batch.items {
            if let Some(existing) = load_existing_sorting_batch_backup(connection, batch, item)? {
                backups.push(existing);
                continue;
            }
            let outcome = run_fixture_backup_prototype(
                connection,
                FixtureBackupPrototypeRequest {
                    fixture_mode: true,
                    apply_plan_id: batch.plan_id,
                    apply_plan_item_id: Some(item.item_id),
                    run_id: batch.run_id,
                    fixture_root: batch.fixture_root.clone(),
                    source_path: item.source_path.clone(),
                    backup_root: batch.backup_root.clone(),
                    operation_kind: BACKUP_OPERATION_KIND.to_owned(),
                },
            )?;
            match outcome {
                FixtureBackupPrototypeOutcome::Verified(success) => backups.push(success),
                FixtureBackupPrototypeOutcome::FailedBeforeChange(failure) => {
                    return Err(AppError::Message(format!(
                        "Sorting batch backup failed before source changes: {}",
                        failure.error_message
                    )));
                }
            }
        }
        Ok(backups)
    }

    #[cfg(target_os = "macos")]
    fn ensure_sorting_batch_receipt_schema(connection: &Connection) -> AppResult<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS fixture_sorting_batch_items (
                apply_plan_run_id INTEGER NOT NULL,
                apply_plan_id INTEGER NOT NULL,
                item_order INTEGER NOT NULL CHECK(item_order >= 0),
                apply_plan_item_id INTEGER NOT NULL,
                file_id INTEGER NOT NULL,
                plan_hash TEXT NOT NULL,
                batch_hash TEXT NOT NULL,
                source_path TEXT NOT NULL,
                destination_path TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                source_size INTEGER NOT NULL CHECK(source_size >= 0),
                backup_result_id INTEGER NOT NULL,
                backup_restore_entry_id INTEGER NOT NULL,
                backup_path TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (apply_plan_run_id, apply_plan_item_id),
                UNIQUE (apply_plan_run_id, item_order)
            );",
        )?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn sorting_batch_hash(plan_hash: &str, items: &[FixtureSortingBatchReceiptItem]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"simsuite-sorting-batch-receipt-v1\0");
        hasher.update(plan_hash.as_bytes());
        hasher.update(b"\0");
        for item in items {
            for field in [
                item.item_order.to_string(),
                item.apply_plan_item_id.to_string(),
                item.file_id.to_string(),
                item.source_path.to_string_lossy().to_string(),
                item.destination_path.to_string_lossy().to_string(),
                item.source_hash.clone(),
                item.source_size.to_string(),
                item.backup_result_id.to_string(),
                item.backup_restore_entry_id.to_string(),
                item.backup_path.to_string_lossy().to_string(),
            ] {
                hasher.update(field.as_bytes());
                hasher.update(b"\0");
            }
        }
        hex::encode(hasher.finalize())
    }

    #[cfg(target_os = "macos")]
    fn load_sorting_batch_receipt(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
    ) -> AppResult<Option<FixtureSortingBatchReceipt>> {
        ensure_sorting_batch_receipt_schema(connection)?;
        let mut statement = connection.prepare(
            "SELECT item_order, apply_plan_item_id, file_id, plan_hash, batch_hash,
                    source_path, destination_path, source_hash, source_size,
                    backup_result_id, backup_restore_entry_id, backup_path, apply_plan_id
             FROM fixture_sorting_batch_items
             WHERE apply_plan_run_id = ?1
             ORDER BY item_order ASC",
        )?;
        let rows = statement
            .query_map(params![batch.run_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, String>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if rows.is_empty() {
            return Ok(None);
        }

        let first_plan_hash = rows[0].3.clone();
        let first_batch_hash = rows[0].4.clone();
        let first_plan_id = rows[0].12;
        let mut items = Vec::with_capacity(rows.len());
        for (
            item_order,
            apply_plan_item_id,
            file_id,
            plan_hash,
            batch_hash,
            source_path,
            destination_path,
            source_hash,
            source_size,
            backup_result_id,
            backup_restore_entry_id,
            backup_path,
            apply_plan_id,
        ) in rows
        {
            if item_order < 0
                || source_size < 0
                || plan_hash != first_plan_hash
                || batch_hash != first_batch_hash
                || apply_plan_id != first_plan_id
            {
                return Err(AppError::Message(
                    "Sorting batch receipt rows disagree with each other.".to_owned(),
                ));
            }
            items.push(FixtureSortingBatchReceiptItem {
                item_order: item_order as usize,
                apply_plan_item_id,
                file_id,
                source_path: PathBuf::from(source_path),
                destination_path: PathBuf::from(destination_path),
                source_hash,
                source_size: source_size as u64,
                backup_result_id,
                backup_restore_entry_id,
                backup_path: PathBuf::from(backup_path),
            });
        }
        Ok(Some(FixtureSortingBatchReceipt {
            run_id: batch.run_id,
            apply_plan_id: first_plan_id,
            plan_hash: first_plan_hash,
            batch_hash: first_batch_hash,
            items,
        }))
    }

    #[cfg(target_os = "macos")]
    fn validate_sorting_batch_receipt(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        receipt: &FixtureSortingBatchReceipt,
    ) -> AppResult<()> {
        if receipt.run_id != batch.run_id
            || receipt.apply_plan_id != batch.plan_id
            || receipt.items.len() != batch.items.len()
        {
            return Err(AppError::Message(
                "Sorting batch receipt no longer matches the requested run and item count."
                    .to_owned(),
            ));
        }
        let plan = apply_plan_persistence::get_apply_plan(connection, batch.plan_id)?
            .ok_or_else(|| AppError::Message("Sorting batch ApplyPlan disappeared.".to_owned()))?;
        let stored_provenance_bytes = serde_json::to_vec(&plan.plan_provenance)?;
        let stored_provenance_hash = hex::encode(Sha256::digest(stored_provenance_bytes));
        if plan.plan_hash.as_deref() != Some(receipt.plan_hash.as_str())
            || stored_provenance_hash != receipt.plan_hash
            || plan.source_plan_kind
                != apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
            || plan.preview_snapshot_id.is_none()
            || plan
                .preview_snapshot_hash
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AppError::Message(
                "Sorting batch receipt is bound to a different or damaged saved ApplyPlan fingerprint."
                    .to_owned(),
            ));
        }
        if sorting_batch_hash(&receipt.plan_hash, &receipt.items) != receipt.batch_hash {
            return Err(AppError::Message(
                "Sorting batch receipt fingerprint no longer matches its item evidence."
                    .to_owned(),
            ));
        }

        for (expected, recorded) in batch.items.iter().zip(receipt.items.iter()) {
            if recorded.apply_plan_item_id != expected.item_id
                || recorded.file_id != expected.file_id
                || recorded.source_path != expected.source_path
                || recorded.destination_path != expected.destination_path
                || recorded.source_hash != expected.source_hash
                || recorded.source_size != expected.source_size
            {
                return Err(AppError::Message(
                    "Sorting batch receipt item identity changed after authorization.".to_owned(),
                ));
            }
            let item = load_plan_item_scope(connection, batch.plan_id, expected.item_id)?;
            if item.file_id != expected.file_id
                || item.action_kind != "suggest_move"
                || item.blocked
                || item.review_only
                || PathBuf::from(item.current_path) != expected.source_path
                || PathBuf::from(item.destination_path) != expected.destination_path
            {
                return Err(AppError::Message(
                    "Sorting batch persisted plan item changed after receipt creation.".to_owned(),
                ));
            }
            let indexed = load_indexed_source_scope(connection, expected.file_id)?;
            if indexed.size < 0
                || indexed.size as u64 != expected.source_size
                || indexed.hash.as_deref() != Some(expected.source_hash.as_str())
                || !matches!(
                    PathBuf::from(indexed.path),
                    path if path == expected.source_path || path == expected.destination_path
                )
            {
                return Err(AppError::Message(
                    "Sorting batch Library identity moved somewhere outside the authorized source/destination pair."
                        .to_owned(),
                ));
            }
            let canonical_backup_root = canonicalize_existing_dir(&batch.backup_root, "sorting batch backup root")?;
            let backup_path = resolve_fixture_candidate(&recorded.backup_path, "sorting batch receipt backup")?;
            ensure_under_root(&backup_path, &canonical_backup_root, "sorting batch receipt backup")?;
            if backup_path != recorded.backup_path
                || observe_expected_file(&backup_path, expected.source_size, &expected.source_hash)?
                    != ObservedFileState::Exact
            {
                return Err(AppError::Message(
                    "Sorting batch receipt backup bytes changed after authorization.".to_owned(),
                ));
            }
            let existing_backup = load_existing_sorting_batch_backup(connection, batch, expected)?
                .ok_or_else(|| {
                    AppError::Message(
                        "Sorting batch receipt lost its verified backup chain.".to_owned(),
                    )
                })?;
            if existing_backup.result_log_id != recorded.backup_result_id
                || existing_backup.restore_entry_id != recorded.backup_restore_entry_id
                || existing_backup.backup_path != recorded.backup_path
            {
                return Err(AppError::Message(
                    "Sorting batch receipt backup identifiers changed after authorization."
                        .to_owned(),
                ));
            }
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn ensure_sorting_batch_receipt(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
    ) -> AppResult<FixtureSortingBatchReceipt> {
        if let Some(receipt) = load_sorting_batch_receipt(connection, batch)? {
            validate_sorting_batch_receipt(connection, batch, &receipt)?;
            return Ok(receipt);
        }

        verify_sorting_batch_read_only_preflight(connection, batch)?;
        let backups = ensure_sorting_batch_backups(connection, batch)?;
        verify_sorting_batch_read_only_preflight(connection, batch)?;
        if backups.len() != batch.items.len() {
            return Err(AppError::Message(
                "Sorting batch did not produce exactly one verified backup per item."
                    .to_owned(),
            ));
        }
        let plan = apply_plan_persistence::get_apply_plan(connection, batch.plan_id)?
            .ok_or_else(|| AppError::Message("Sorting batch ApplyPlan disappeared.".to_owned()))?;
        let plan_hash = plan
            .plan_hash
            .clone()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::Message("Sorting batch ApplyPlan has no fingerprint.".to_owned()))?;
        let receipt_items = batch
            .items
            .iter()
            .zip(backups.iter())
            .enumerate()
            .map(|(item_order, (item, backup))| FixtureSortingBatchReceiptItem {
                item_order,
                apply_plan_item_id: item.item_id,
                file_id: item.file_id,
                source_path: item.source_path.clone(),
                destination_path: item.destination_path.clone(),
                source_hash: item.source_hash.clone(),
                source_size: item.source_size,
                backup_result_id: backup.result_log_id,
                backup_restore_entry_id: backup.restore_entry_id,
                backup_path: backup.backup_path.clone(),
            })
            .collect::<Vec<_>>();
        let batch_hash = sorting_batch_hash(&plan_hash, &receipt_items);

        ensure_sorting_batch_receipt_schema(connection)?;
        let transaction = connection.unchecked_transaction()?;
        for item in &receipt_items {
            transaction.execute(
                "INSERT INTO fixture_sorting_batch_items (
                    apply_plan_run_id, apply_plan_id, item_order, apply_plan_item_id,
                    file_id, plan_hash, batch_hash, source_path, destination_path,
                    source_hash, source_size, backup_result_id, backup_restore_entry_id,
                    backup_path
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    batch.run_id,
                    batch.plan_id,
                    item.item_order as i64,
                    item.apply_plan_item_id,
                    item.file_id,
                    plan_hash,
                    batch_hash,
                    item.source_path.to_string_lossy().to_string(),
                    item.destination_path.to_string_lossy().to_string(),
                    item.source_hash,
                    item.source_size as i64,
                    item.backup_result_id,
                    item.backup_restore_entry_id,
                    item.backup_path.to_string_lossy().to_string(),
                ],
            )?;
        }
        transaction.commit()?;
        let receipt = load_sorting_batch_receipt(connection, batch)?.ok_or_else(|| {
            AppError::Message("Sorting batch receipt disappeared after commit.".to_owned())
        })?;
        validate_sorting_batch_receipt(connection, batch, &receipt)?;
        Ok(receipt)
    }

    #[cfg(target_os = "macos")]
    fn ensure_sorting_batch_directory_schema(connection: &Connection) -> AppResult<()> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS fixture_sorting_batch_directories (
                apply_plan_run_id INTEGER NOT NULL,
                apply_plan_id INTEGER NOT NULL,
                batch_hash TEXT NOT NULL,
                directory_path TEXT NOT NULL,
                parent_path TEXT NOT NULL,
                device_id INTEGER NOT NULL CHECK(device_id >= 0),
                inode INTEGER NOT NULL CHECK(inode >= 0),
                state TEXT NOT NULL CHECK(state IN (
                    'prepared_before_create',
                    'created_verified',
                    'ownership_unknown_keep',
                    'retained_non_empty',
                    'retained_identity_changed',
                    'cleanup_complete'
                )),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (apply_plan_run_id, directory_path)
            );",
        )?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn load_sorting_batch_directory_record(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
    ) -> AppResult<Option<FixtureSortingBatchDirectoryRecord>> {
        ensure_sorting_batch_directory_schema(connection)?;
        let row = connection
            .query_row(
                "SELECT apply_plan_run_id, apply_plan_id, batch_hash, directory_path,
                        parent_path, device_id, inode, state
                 FROM fixture_sorting_batch_directories
                 WHERE apply_plan_run_id = ?1 AND directory_path = ?2",
                params![
                    batch.run_id,
                    batch.destination_dir.to_string_lossy().to_string(),
                ],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()?;
        let Some((run_id, plan_id, batch_hash, directory_path, parent_path, device_id, inode, state)) = row else {
            return Ok(None);
        };
        if device_id < 0 || inode < 0 {
            return Err(AppError::Message(
                "Sorting batch directory identity contains a negative filesystem id.".to_owned(),
            ));
        }
        Ok(Some(FixtureSortingBatchDirectoryRecord {
            apply_plan_run_id: run_id,
            apply_plan_id: plan_id,
            batch_hash,
            directory_path: PathBuf::from(directory_path),
            parent_path: PathBuf::from(parent_path),
            device_id: device_id as u64,
            inode: inode as u64,
            state: FixtureSortingDirectoryState::from_db(&state)?,
        }))
    }

    #[cfg(target_os = "macos")]
    fn update_sorting_batch_directory_state(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        expected: FixtureSortingDirectoryState,
        next: FixtureSortingDirectoryState,
    ) -> AppResult<()> {
        let changed = connection.execute(
            "UPDATE fixture_sorting_batch_directories
             SET state = ?1, updated_at = CURRENT_TIMESTAMP
             WHERE apply_plan_run_id = ?2 AND directory_path = ?3 AND state = ?4",
            params![
                next.as_str(),
                batch.run_id,
                batch.destination_dir.to_string_lossy().to_string(),
                expected.as_str(),
            ],
        )?;
        if changed != 1 {
            return Err(AppError::Message(
                "Sorting batch directory ownership state changed unexpectedly.".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn ensure_sorting_batch_destination_directory(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        receipt: &FixtureSortingBatchReceipt,
    ) -> AppResult<FixtureSortingDirectoryCreateOutcome> {
        validate_sorting_batch_receipt(connection, batch, receipt)?;
        let parent = canonicalize_existing_dir(&batch.mods_root, "sorting batch Mods root")?;
        let destination_parent = batch.destination_dir.parent().ok_or_else(|| {
            AppError::Message(
                "Sorting batch destination directory has no parent folder.".to_owned(),
            )
        })?;
        let canonical_destination_parent =
            canonicalize_existing_dir(destination_parent, "sorting batch destination parent")?;
        if canonical_destination_parent != parent {
            return Err(AppError::Message(
                "Sorting batch destination directory must be one immediate child of Mods."
                    .to_owned(),
            ));
        }
        if receipt
            .items
            .iter()
            .any(|item| item.destination_path.parent() != Some(batch.destination_dir.as_path()))
        {
            return Err(AppError::Message(
                "Sorting batch receipt contains destinations outside its one shared folder."
                    .to_owned(),
            ));
        }

        if let Some(record) = load_sorting_batch_directory_record(connection, batch)? {
            if record.apply_plan_run_id != batch.run_id
                || record.apply_plan_id != batch.plan_id
                || record.batch_hash != receipt.batch_hash
                || record.directory_path != batch.destination_dir
                || record.parent_path != parent
            {
                return Err(AppError::Message(
                    "Sorting batch directory ownership record no longer matches the batch receipt."
                        .to_owned(),
                ));
            }
            return match record.state {
                FixtureSortingDirectoryState::PreparedBeforeCreate => {
                    if batch.destination_dir.exists() {
                        update_sorting_batch_directory_state(
                            connection,
                            batch,
                            FixtureSortingDirectoryState::PreparedBeforeCreate,
                            FixtureSortingDirectoryState::OwnershipUnknownKeep,
                        )?;
                        Ok(FixtureSortingDirectoryCreateOutcome::OwnershipUnknownKeep)
                    } else {
                        Err(AppError::Message(
                            "Sorting batch directory creation was prepared but never completed; retry must explicitly reconcile it."
                                .to_owned(),
                        ))
                    }
                }
                FixtureSortingDirectoryState::CreatedVerified => {
                    let (device_id, inode) = fixture_directory_identity(&batch.destination_dir)?;
                    if device_id == record.device_id && inode == record.inode {
                        Ok(FixtureSortingDirectoryCreateOutcome::CreatedOwned)
                    } else {
                        Err(AppError::Message(
                            "Sorting batch destination folder identity changed after SimSuite created it."
                                .to_owned(),
                        ))
                    }
                }
                FixtureSortingDirectoryState::OwnershipUnknownKeep => {
                    Ok(FixtureSortingDirectoryCreateOutcome::OwnershipUnknownKeep)
                }
                FixtureSortingDirectoryState::RetainedNonEmpty
                | FixtureSortingDirectoryState::RetainedIdentityChanged
                | FixtureSortingDirectoryState::CleanupComplete => Err(AppError::Message(
                    "Sorting batch directory ownership is already in a post-Undo state."
                        .to_owned(),
                )),
            };
        }

        if batch.destination_dir.exists() {
            let metadata = fs::symlink_metadata(&batch.destination_dir).map_err(|error| {
                AppError::Message(format!(
                    "Sorting batch existing destination folder could not be inspected: {error}"
                ))
            })?;
            if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
                return Err(AppError::Message(
                    "Sorting batch existing destination entry is not a safe real directory."
                        .to_owned(),
                ));
            }
            return Ok(FixtureSortingDirectoryCreateOutcome::ExistingNotOwned);
        }

        ensure_sorting_batch_directory_schema(connection)?;
        connection.execute(
            "INSERT INTO fixture_sorting_batch_directories (
                apply_plan_run_id, apply_plan_id, batch_hash, directory_path, parent_path,
                device_id, inode, state
             ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6)",
            params![
                batch.run_id,
                batch.plan_id,
                receipt.batch_hash,
                batch.destination_dir.to_string_lossy().to_string(),
                parent.to_string_lossy().to_string(),
                FixtureSortingDirectoryState::PreparedBeforeCreate.as_str(),
            ],
        )?;
        fs::create_dir(&batch.destination_dir).map_err(|error| {
            AppError::Message(format!(
                "Sorting batch destination folder could not be created non-recursively: {error}"
            ))
        })?;
        let (device_id, inode) = fixture_directory_identity(&batch.destination_dir)?;
        let changed = connection.execute(
            "UPDATE fixture_sorting_batch_directories
             SET device_id = ?1, inode = ?2, state = ?3, updated_at = CURRENT_TIMESTAMP
             WHERE apply_plan_run_id = ?4 AND directory_path = ?5 AND state = ?6",
            params![
                device_id as i64,
                inode as i64,
                FixtureSortingDirectoryState::CreatedVerified.as_str(),
                batch.run_id,
                batch.destination_dir.to_string_lossy().to_string(),
                FixtureSortingDirectoryState::PreparedBeforeCreate.as_str(),
            ],
        )?;
        if changed != 1 {
            return Err(AppError::Message(
                "Sorting batch created its destination folder but could not finalize ownership evidence."
                    .to_owned(),
            ));
        }
        Ok(FixtureSortingDirectoryCreateOutcome::CreatedOwned)
    }

    #[cfg(target_os = "macos")]
    fn load_sorting_batch_authorization_row(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        item: &SortingBatchFixtureItem,
    ) -> AppResult<Option<FixtureSortingAuthorization>> {
        ensure_fixture_sorting_authorization_schema(connection)?;
        connection
            .query_row(
                "SELECT plan_hash, source_path, destination_path, source_hash, source_size,
                        backup_result_id, backup_restore_entry_id
                 FROM fixture_sorting_authorizations
                 WHERE apply_plan_id = ?1 AND apply_plan_item_id = ?2",
                params![batch.plan_id, item.item_id],
                |row| {
                    let source_size = row.get::<_, i64>(4)?;
                    if source_size < 0 {
                        return Err(rusqlite::Error::IntegralValueOutOfRange(4, source_size));
                    }
                    Ok(FixtureSortingAuthorization {
                        plan_hash: row.get(0)?,
                        source_path: row.get(1)?,
                        destination_path: row.get(2)?,
                        source_hash: row.get(3)?,
                        source_size: source_size as u64,
                        backup_result_id: row.get(5)?,
                        backup_restore_entry_id: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    #[cfg(target_os = "macos")]
    fn ensure_sorting_batch_item_authorization(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        receipt: &FixtureSortingBatchReceipt,
        item: &SortingBatchFixtureItem,
        receipt_item: &FixtureSortingBatchReceiptItem,
    ) -> AppResult<()> {
        if let Some(authorization) = load_sorting_batch_authorization_row(connection, batch, item)? {
            let authorized_source = resolve_fixture_candidate(
                Path::new(&authorization.source_path),
                "sorting batch recorded authorization source",
            )?;
            let expected_source = resolve_fixture_candidate(
                &item.source_path,
                "sorting batch expected authorization source",
            )?;
            let authorized_destination = resolve_fixture_candidate(
                Path::new(&authorization.destination_path),
                "sorting batch recorded authorization destination",
            )?;
            let expected_destination = resolve_fixture_candidate(
                &item.destination_path,
                "sorting batch expected authorization destination",
            )?;
            if authorization.plan_hash == receipt.plan_hash
                && authorization.source_hash == item.source_hash
                && authorization.source_size == item.source_size
                && authorized_source == expected_source
                && authorized_destination == expected_destination
                && authorization.backup_result_id == receipt_item.backup_result_id
                && authorization.backup_restore_entry_id == receipt_item.backup_restore_entry_id
            {
                return Ok(());
            }
            return Err(AppError::Message(
                "Sorting batch existing item authorization no longer matches the sealed batch receipt."
                    .to_owned(),
            ));
        }

        // A missing row can only be created through the original strict single-item gate.
        // That gate re-verifies the live ApplyPlan, so after any batch membership has moved a
        // missing authorization fails closed instead of being minted from partial state.
        let request = sorting_batch_reconciliation_request(batch, item);
        let authorization = record_fixture_sorting_authorization(
            connection,
            &request,
            receipt_item.backup_result_id,
            receipt_item.backup_restore_entry_id,
        )?;
        if authorization.plan_hash != receipt.plan_hash
            || authorization.source_hash != item.source_hash
            || authorization.source_size != item.source_size
            || authorization.backup_result_id != receipt_item.backup_result_id
            || authorization.backup_restore_entry_id != receipt_item.backup_restore_entry_id
        {
            return Err(AppError::Message(
                "Sorting batch item authorization did not bind the sealed receipt evidence."
                    .to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn sorting_batch_attempt_id(batch: &SortingBatchFixtureContext, item: &SortingBatchFixtureItem) -> String {
        format!("sorting-batch-run-{}-item-{}", batch.run_id, item.item_id)
    }

    #[cfg(target_os = "macos")]
    fn prepare_sorting_batch_attempt(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        receipt: &FixtureSortingBatchReceipt,
        item: &SortingBatchFixtureItem,
        receipt_item: &FixtureSortingBatchReceiptItem,
        attempt_id: &str,
        capability: &FixtureAttemptCapabilityProof,
    ) -> AppResult<FixtureAttemptRecord> {
        if attempt_id.trim().is_empty() {
            return Err(AppError::Message(
                "Sorting batch attempt id must not be empty.".to_owned(),
            ));
        }
        capability.validate_for_fixture_attempt()?;
        ensure_run_matches_plan(connection, batch.run_id, batch.plan_id)?;
        validate_sorting_batch_receipt(connection, batch, receipt)?;
        ensure_sorting_batch_item_authorization(
            connection,
            batch,
            receipt,
            item,
            receipt_item,
        )?;

        let plan_item = load_plan_item_scope(connection, batch.plan_id, item.item_id)?;
        if plan_item.file_id != item.file_id
            || plan_item.action_kind != "suggest_move"
            || plan_item.blocked
            || plan_item.review_only
        {
            return Err(AppError::Message(
                "Sorting batch attempt requires the same unblocked suggest_move item sealed in the receipt."
                    .to_owned(),
            ));
        }
        let source_path = resolve_fixture_candidate(
            &item.source_path,
            "sorting batch attempt source",
        )?;
        let destination_path = resolve_fixture_candidate(
            &item.destination_path,
            "sorting batch attempt destination",
        )?;
        let initial_membership = load_fixture_membership_state(connection, item.file_id)?
            .ok_or_else(|| {
                AppError::Message(
                    "Sorting batch attempt requires current Library membership evidence."
                        .to_owned(),
                )
            })?;
        let initial_membership_path = resolve_fixture_candidate(
            Path::new(&initial_membership.path),
            "sorting batch attempt initial membership",
        )?;
        if initial_membership.file_id != item.file_id || initial_membership_path != source_path {
            return Err(AppError::Message(
                "Sorting batch pending item Library membership is no longer at its authorized source."
                    .to_owned(),
            ));
        }
        if observe_expected_file(&source_path, item.source_size, &item.source_hash)?
            != ObservedFileState::Exact
            || observe_expected_file(
                &destination_path,
                item.source_size,
                &item.source_hash,
            )? != ObservedFileState::Missing
        {
            return Err(AppError::Message(
                "Sorting batch pending item changed after authorization; retry is blocked."
                    .to_owned(),
            ));
        }
        let source_metadata = fs::symlink_metadata(&source_path).map_err(|error| {
            AppError::Message(format!(
                "Sorting batch attempt could not inspect source path entry: {error}"
            ))
        })?;
        let source_regular_file = source_metadata.file_type().is_file();
        let source_entry_non_symlink = !source_metadata.file_type().is_symlink();
        let destination_entry_non_symlink = !existing_path_entry_is_symlink(&destination_path)?;
        if !source_regular_file || !source_entry_non_symlink || !destination_entry_non_symlink {
            return Err(AppError::Message(
                "Sorting batch attempt requires a regular non-symlink source and empty non-symlink destination."
                    .to_owned(),
            ));
        }
        let backup = load_existing_sorting_batch_backup(connection, batch, item)?
            .ok_or_else(|| {
                AppError::Message(
                    "Sorting batch attempt lost the verified backup sealed in its receipt."
                        .to_owned(),
                )
            })?;
        if backup.result_log_id != receipt_item.backup_result_id
            || backup.restore_entry_id != receipt_item.backup_restore_entry_id
            || backup.backup_path != receipt_item.backup_path
        {
            return Err(AppError::Message(
                "Sorting batch attempt backup evidence no longer matches the sealed receipt."
                    .to_owned(),
            ));
        }

        ensure_fixture_attempt_journal_schema(connection)?;
        let existing_active = connection
            .query_row(
                "SELECT attempt_id
                 FROM fixture_apply_plan_attempts
                 WHERE apply_plan_run_id = ?1
                   AND apply_plan_item_id = ?2
                   AND state IN (
                       'prepared_before_change',
                       'destination_claim_observed',
                       'destination_verified',
                       'source_release_completed',
                       'recovery_required'
                   )
                 LIMIT 1",
                params![batch.run_id, item.item_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(existing_attempt_id) = existing_active {
            return Err(AppError::Message(format!(
                "Sorting batch item already has an active attempt: {existing_attempt_id}"
            )));
        }

        connection.execute(
            "INSERT INTO fixture_apply_plan_attempts (
                attempt_id,
                apply_plan_run_id,
                apply_plan_id,
                apply_plan_item_id,
                source_path,
                destination_path,
                expected_hash,
                expected_size,
                initial_source_location,
                initial_download_item_id,
                backup_result_id,
                backup_restore_entry_id,
                strategy,
                capability_platform,
                same_filesystem,
                hard_link_supported,
                native_runtime_proven,
                capability_evidence_label,
                source_regular_file,
                source_entry_non_symlink,
                destination_entry_non_symlink,
                state
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22
            )",
            params![
                attempt_id,
                batch.run_id,
                batch.plan_id,
                item.item_id,
                source_path.to_string_lossy().to_string(),
                destination_path.to_string_lossy().to_string(),
                item.source_hash,
                item.source_size as i64,
                initial_membership.source_location,
                initial_membership.download_item_id,
                receipt_item.backup_result_id,
                receipt_item.backup_restore_entry_id,
                capability.strategy,
                capability.platform,
                i64::from(capability.same_filesystem),
                i64::from(capability.hard_link_supported),
                i64::from(capability.native_runtime_proven),
                capability.evidence_label,
                i64::from(source_regular_file),
                i64::from(source_entry_non_symlink),
                i64::from(destination_entry_non_symlink),
                FixtureAttemptState::PreparedBeforeChange.as_str(),
            ],
        )?;
        load_fixture_attempt(connection, attempt_id)?.ok_or_else(|| {
            AppError::Message(
                "Sorting batch attempt was not readable after preparation.".to_owned(),
            )
        })
    }

    #[cfg(target_os = "macos")]
    fn release_sorting_batch_item_attempt(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        receipt: &FixtureSortingBatchReceipt,
        item: &SortingBatchFixtureItem,
        receipt_item: &FixtureSortingBatchReceiptItem,
        attempt_id: &str,
    ) -> AppResult<()> {
        let attempt = prepare_sorting_batch_attempt(
            connection,
            batch,
            receipt,
            item,
            receipt_item,
            attempt_id,
            &fixture_attempt_capability_proof(),
        )?;
        claim_fixture_destination_no_replace(&item.source_path, &item.destination_path)?;
        transition_fixture_attempt(
            connection,
            attempt_id,
            FixtureAttemptState::DestinationClaimObserved,
        )?;
        if observe_expected_file(
            &item.destination_path,
            attempt.expected_size,
            &attempt.expected_hash,
        )? != ObservedFileState::Exact
        {
            return Err(AppError::Message(
                "Sorting batch destination did not match the prepared attempt after claim."
                    .to_owned(),
            ));
        }
        transition_fixture_attempt(
            connection,
            attempt_id,
            FixtureAttemptState::DestinationVerified,
        )?;
        fs::remove_file(&item.source_path).map_err(|error| {
            AppError::Message(format!(
                "Sorting batch destination was verified but source release failed: {error}"
            ))
        })?;
        transition_fixture_attempt(
            connection,
            attempt_id,
            FixtureAttemptState::SourceReleaseCompleted,
        )?;
        if observe_expected_file(&item.source_path, item.source_size, &item.source_hash)?
            != ObservedFileState::Missing
            || observe_expected_file(&item.destination_path, item.source_size, &item.source_hash)?
                != ObservedFileState::Exact
        {
            return Err(AppError::Message(
                "Sorting batch item ended source release in an unexpected filesystem state."
                    .to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn sorting_batch_forward_handoff(
        item: &SortingBatchFixtureItem,
        attempt_id: &str,
    ) -> FixtureMembershipHandoff {
        FixtureMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: FixtureMembershipState {
                    file_id: item.file_id,
                    path: item.source_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: None,
                },
                final_state: FixtureMembershipState {
                    file_id: item.file_id,
                    path: item.destination_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: None,
                },
            },
        }
    }

    #[cfg(target_os = "macos")]
    fn run_sorting_batch_apply(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        stop_before_index: Option<usize>,
    ) -> AppResult<FixtureSortingBatchApplyOutcome> {
        let receipt = ensure_sorting_batch_receipt(connection, batch)?;
        ensure_sorting_batch_destination_directory(connection, batch, &receipt)?;

        for (item, receipt_item) in batch.items.iter().zip(receipt.items.iter()) {
            let attempt_id = sorting_batch_attempt_id(batch, item);
            let already_finished = load_fixture_attempt(connection, &attempt_id)?
                .is_some_and(|attempt| {
                    matches!(
                        attempt.state,
                        FixtureAttemptState::SourceReleaseCompleted | FixtureAttemptState::Committed
                    )
                });
            if !already_finished {
                ensure_sorting_batch_item_authorization(
                    connection,
                    batch,
                    &receipt,
                    item,
                    receipt_item,
                )?;
            }
        }

        let mut handoffs = Vec::new();
        let mut stopped_before_item_id = None;
        for (index, item) in batch.items.iter().enumerate() {
            let attempt_id = sorting_batch_attempt_id(batch, item);
            match load_fixture_attempt(connection, &attempt_id)? {
                Some(attempt) if attempt.state == FixtureAttemptState::Committed => {
                    if observe_expected_file(&item.source_path, item.source_size, &item.source_hash)?
                        != ObservedFileState::Missing
                        || observe_expected_file(
                            &item.destination_path,
                            item.source_size,
                            &item.source_hash,
                        )? != ObservedFileState::Exact
                    {
                        return Err(AppError::Message(
                            "Sorting batch committed item no longer matches its durable filesystem evidence."
                                .to_owned(),
                        ));
                    }
                    let membership = load_fixture_membership_state(connection, item.file_id)?
                        .ok_or_else(|| {
                            AppError::Message(
                                "Sorting batch committed item disappeared from Library membership."
                                    .to_owned(),
                            )
                        })?;
                    if PathBuf::from(membership.path) != item.destination_path
                        || membership.source_location != "mods"
                    {
                        return Err(AppError::Message(
                            "Sorting batch committed item Library membership is not at its destination."
                                .to_owned(),
                        ));
                    }
                    continue;
                }
                Some(attempt) if attempt.state == FixtureAttemptState::SourceReleaseCompleted => {
                    handoffs.push(sorting_batch_forward_handoff(item, &attempt_id));
                    continue;
                }
                Some(attempt) => {
                    return Err(AppError::Message(format!(
                        "Sorting batch item requires per-file recovery before batch retry: {}",
                        attempt.state.as_str()
                    )));
                }
                None => {}
            }

            if stop_before_index == Some(index) {
                stopped_before_item_id = Some(item.item_id);
                break;
            }
            release_sorting_batch_item_attempt(
                connection,
                batch,
                &receipt,
                item,
                &receipt.items[index],
                &attempt_id,
            )?;
            handoffs.push(sorting_batch_forward_handoff(item, &attempt_id));
        }

        if !handoffs.is_empty() {
            commit_fixture_membership_handoffs(connection, &handoffs)?;
        }

        let mut completed_item_ids = Vec::new();
        let mut pending_item_ids = Vec::new();
        for item in &batch.items {
            let attempt_id = sorting_batch_attempt_id(batch, item);
            match load_fixture_attempt(connection, &attempt_id)? {
                Some(attempt) if attempt.state == FixtureAttemptState::Committed => {
                    completed_item_ids.push(item.item_id)
                }
                _ => pending_item_ids.push(item.item_id),
            }
        }
        Ok(FixtureSortingBatchApplyOutcome {
            completed_item_ids,
            pending_item_ids,
            stopped_before_item_id,
        })
    }

    #[cfg(target_os = "macos")]
    fn sorting_batch_reverse_handoff(
        item: &SortingBatchFixtureItem,
        attempt_id: &str,
    ) -> FixtureReverseMembershipHandoff {
        FixtureReverseMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: FixtureMembershipState {
                    file_id: item.file_id,
                    path: item.destination_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: None,
                },
                final_state: FixtureMembershipState {
                    file_id: item.file_id,
                    path: item.source_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: None,
                },
            },
        }
    }

    #[cfg(target_os = "macos")]
    fn restore_sorting_batch_item_from_receipt(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        item: &SortingBatchFixtureItem,
        receipt_item: &FixtureSortingBatchReceiptItem,
    ) -> AppResult<()> {
        match run_fixture_restore_prototype(
            connection,
            FixtureRestorePrototypeRequest {
                fixture_mode: true,
                fixture_root: batch.fixture_root.clone(),
                backup_path: receipt_item.backup_path.clone(),
                restore_target_path: item.source_path.clone(),
                apply_plan_id: batch.plan_id,
                apply_plan_item_id: Some(item.item_id),
                run_id: batch.run_id,
                result_id: Some(receipt_item.backup_result_id),
                restore_entry_id: Some(receipt_item.backup_restore_entry_id),
                operation_kind: UNDO_OPERATION_KIND.to_owned(),
            },
        )? {
            FixtureRestorePrototypeOutcome::Verified(_) => {}
            FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) => {
                return Err(AppError::Message(format!(
                    "Sorting batch Undo restore failed before change: {}",
                    failure.error_message
                )));
            }
        }
        if observe_expected_file(&item.source_path, item.source_size, &item.source_hash)?
            != ObservedFileState::Exact
            || observe_expected_file(&item.destination_path, item.source_size, &item.source_hash)?
                != ObservedFileState::Exact
        {
            return Err(AppError::Message(
                "Sorting batch Undo could not verify restored source and moved copy before cleanup."
                    .to_owned(),
            ));
        }
        fs::remove_file(&item.destination_path).map_err(|error| {
            AppError::Message(format!(
                "Sorting batch Undo restored source but could not remove moved copy: {error}"
            ))
        })?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn cleanup_sorting_batch_destination_directory(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
        receipt: &FixtureSortingBatchReceipt,
    ) -> AppResult<FixtureSortingDirectoryCleanupOutcome> {
        let Some(record) = load_sorting_batch_directory_record(connection, batch)? else {
            return Ok(FixtureSortingDirectoryCleanupOutcome::NotOwned);
        };
        if record.state != FixtureSortingDirectoryState::CreatedVerified {
            return Ok(FixtureSortingDirectoryCleanupOutcome::NotOwned);
        }
        validate_sorting_batch_receipt(connection, batch, receipt)?;
        for item in &batch.items {
            if observe_expected_file(&item.source_path, item.source_size, &item.source_hash)?
                != ObservedFileState::Exact
                || observe_expected_file(
                    &item.destination_path,
                    item.source_size,
                    &item.source_hash,
                )? != ObservedFileState::Missing
            {
                return Err(AppError::Message(
                    "Sorting batch folder cleanup requires every batch file to be fully undone first."
                        .to_owned(),
                ));
            }
            let membership = load_fixture_membership_state(connection, item.file_id)?
                .ok_or_else(|| {
                    AppError::Message(
                        "Sorting batch folder cleanup requires restored Library membership."
                            .to_owned(),
                    )
                })?;
            let membership_path = resolve_fixture_candidate(
                Path::new(&membership.path),
                "sorting batch cleanup membership path",
            )?;
            let source_path = resolve_fixture_candidate(
                &item.source_path,
                "sorting batch cleanup source path",
            )?;
            if membership_path != source_path || membership.source_location != "mods" {
                return Err(AppError::Message(
                    "Sorting batch folder cleanup requires Library membership back at every original source."
                        .to_owned(),
                ));
            }
        }

        let current_identity = fixture_directory_identity(&batch.destination_dir)?;
        if current_identity != (record.device_id, record.inode) {
            update_sorting_batch_directory_state(
                connection,
                batch,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::RetainedIdentityChanged,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedIdentityChanged);
        }
        if fixture_directory_has_indexed_children(connection, &batch.destination_dir)?
            || fs::read_dir(&batch.destination_dir)
                .map_err(|error| {
                    AppError::Message(format!(
                        "Sorting batch destination folder could not be read for cleanup: {error}"
                    ))
                })?
                .next()
                .is_some()
        {
            update_sorting_batch_directory_state(
                connection,
                batch,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::RetainedNonEmpty,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty);
        }
        if fixture_directory_identity(&batch.destination_dir)? != current_identity {
            update_sorting_batch_directory_state(
                connection,
                batch,
                FixtureSortingDirectoryState::CreatedVerified,
                FixtureSortingDirectoryState::RetainedIdentityChanged,
            )?;
            return Ok(FixtureSortingDirectoryCleanupOutcome::RetainedIdentityChanged);
        }
        fs::remove_dir(&batch.destination_dir).map_err(|error| {
            AppError::Message(format!(
                "Sorting batch destination folder could not be removed safely: {error}"
            ))
        })?;
        update_sorting_batch_directory_state(
            connection,
            batch,
            FixtureSortingDirectoryState::CreatedVerified,
            FixtureSortingDirectoryState::CleanupComplete,
        )?;
        Ok(FixtureSortingDirectoryCleanupOutcome::Removed)
    }

    #[cfg(target_os = "macos")]
    fn run_sorting_batch_undo(
        connection: &Connection,
        batch: &SortingBatchFixtureContext,
    ) -> AppResult<FixtureSortingDirectoryCleanupOutcome> {
        let receipt = ensure_sorting_batch_receipt(connection, batch)?;
        let mut reverse_handoffs = Vec::new();

        // Preflight every committed item before restoring the first one. Known tampering therefore
        // blocks the whole Undo before this helper changes any file.
        for item in &batch.items {
            let attempt_id = sorting_batch_attempt_id(batch, item);
            let Some(attempt) = load_fixture_attempt(connection, &attempt_id)? else {
                continue;
            };
            if attempt.state != FixtureAttemptState::Committed {
                return Err(AppError::Message(
                    "Sorting batch Undo requires every started item to have committed Library membership first."
                        .to_owned(),
                ));
            }
            let source_state = observe_expected_file(&item.source_path, item.source_size, &item.source_hash)?;
            let destination_state =
                observe_expected_file(&item.destination_path, item.source_size, &item.source_hash)?;
            if !matches!(
                (source_state, destination_state),
                (ObservedFileState::Missing, ObservedFileState::Exact)
                    | (ObservedFileState::Exact, ObservedFileState::Missing)
            ) {
                return Err(AppError::Message(
                    "Sorting batch Undo found changed or ambiguous file bytes; no batch Undo changes were started."
                        .to_owned(),
                ));
            }
        }

        for (item, receipt_item) in batch.items.iter().zip(receipt.items.iter()) {
            let attempt_id = sorting_batch_attempt_id(batch, item);
            let Some(attempt) = load_fixture_attempt(connection, &attempt_id)? else {
                continue;
            };
            let source_state = observe_expected_file(&item.source_path, item.source_size, &item.source_hash)?;
            let destination_state =
                observe_expected_file(&item.destination_path, item.source_size, &item.source_hash)?;
            if source_state == ObservedFileState::Missing
                && destination_state == ObservedFileState::Exact
            {
                restore_sorting_batch_item_from_receipt(
                    connection,
                    batch,
                    item,
                    receipt_item,
                )?;
            }
            let membership = load_fixture_membership_state(connection, item.file_id)?
                .ok_or_else(|| {
                    AppError::Message("Sorting batch Undo lost Library membership.".to_owned())
                })?;
            if PathBuf::from(&membership.path) == item.destination_path {
                reverse_handoffs.push(sorting_batch_reverse_handoff(item, &attempt_id));
            } else if PathBuf::from(&membership.path) != item.source_path {
                return Err(AppError::Message(
                    "Sorting batch Undo Library membership moved outside its authorized pair."
                        .to_owned(),
                ));
            }
            if attempt.state != FixtureAttemptState::Committed {
                return Err(AppError::Message(
                    "Sorting batch Undo attempt state changed unexpectedly.".to_owned(),
                ));
            }
        }

        if !reverse_handoffs.is_empty() {
            commit_fixture_reverse_membership_handoffs(connection, &reverse_handoffs)?;
        }
        cleanup_sorting_batch_destination_directory(connection, batch, &receipt)
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

    fn fixture_attempt_capability_proof() -> FixtureAttemptCapabilityProof {
        FixtureAttemptCapabilityProof {
            strategy: FIXTURE_ATTEMPT_STRATEGY_HARD_LINK.to_owned(),
            platform: std::env::consts::OS.to_owned(),
            same_filesystem: true,
            hard_link_supported: true,
            native_runtime_proven: true,
            evidence_label: "explicit-fixture-capability-proof-v1".to_owned(),
        }
    }

    fn setup_verified_backup_only_state(
        connection: &Connection,
        source_bytes: &[u8],
    ) -> FixtureContext {
        let context = setup_fixture(connection, source_bytes);
        run_fixture_move_transaction_with_fault(
            connection,
            move_request(&context),
            FixtureFaultPoint::AfterVerifiedBackupBeforeMove,
        )
        .expect_err("interrupt after verified backup");
        assert_eq!(
            classify_fixture_reconciliation_state(connection, &reconciliation_request(&context))
                .expect("classify verified-backup state"),
            FixtureReconciliationState::ResumeFromVerifiedBackupRequired
        );
        context
    }

    #[cfg(target_os = "macos")]
    fn attach_fixture_download_owner(
        connection: &Connection,
        context: &FixtureContext,
        download_item_id: i64,
    ) -> i64 {
        connection
            .execute(
                "INSERT INTO download_items (
                    id, source_path, display_name, source_kind, source_size,
                    detected_file_count, status, notes
                 ) VALUES (?1, ?2, ?3, 'archive', 100, 0, 'pending', '[]')",
                params![
                    download_item_id,
                    format!("/fixture/download-{download_item_id}.zip"),
                    format!("Fixture download {download_item_id}"),
                ],
            )
            .expect("fixture download owner");
        let file_id = load_plan_item_scope(connection, context.plan_id, context.item_id)
            .expect("fixture plan item")
            .file_id;
        connection
            .execute(
                "UPDATE files SET download_item_id = ?1 WHERE id = ?2",
                params![download_item_id, file_id],
            )
            .expect("attach fixture download owner");
        file_id
    }

    #[cfg(target_os = "macos")]
    fn release_fixture_attempt(
        connection: &Connection,
        context: &FixtureContext,
        attempt_id: &str,
    ) {
        let request = reconciliation_request(context);
        let attempt = prepare_fixture_attempt(
            connection,
            &request,
            attempt_id,
            &fixture_attempt_capability_proof(),
        )
        .expect("prepare fixture handoff attempt");

        claim_fixture_destination_no_replace(&context.source_path, &context.destination_path)
            .expect("claim fixture handoff destination");
        transition_fixture_attempt(
            connection,
            attempt_id,
            FixtureAttemptState::DestinationClaimObserved,
        )
        .expect("record destination claim");
        assert_eq!(
            observe_expected_file(
                &context.destination_path,
                attempt.expected_size,
                &attempt.expected_hash,
            )
            .expect("verify claimed destination"),
            ObservedFileState::Exact
        );
        transition_fixture_attempt(
            connection,
            attempt_id,
            FixtureAttemptState::DestinationVerified,
        )
        .expect("record verified destination");
        fs::remove_file(&context.source_path).expect("release fixture source");
        transition_fixture_attempt(
            connection,
            attempt_id,
            FixtureAttemptState::SourceReleaseCompleted,
        )
        .expect("record released source");

        assert!(!context.source_path.exists());
        assert_eq!(
            observe_expected_file(
                &context.destination_path,
                attempt.expected_size,
                &attempt.expected_hash,
            )
            .expect("verify released destination"),
            ObservedFileState::Exact
        );
    }

    #[cfg(target_os = "macos")]
    fn fixture_membership_handoff(
        context: &FixtureContext,
        file_id: i64,
        download_item_id: i64,
        attempt_id: &str,
    ) -> FixtureMembershipHandoff {
        FixtureMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: FixtureMembershipState {
                    file_id,
                    path: context.source_path.to_string_lossy().to_string(),
                    source_location: "downloads".to_owned(),
                    download_item_id: Some(download_item_id),
                },
                final_state: FixtureMembershipState {
                    file_id,
                    path: context.destination_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: Some(download_item_id),
                },
            },
        }
    }

    #[cfg(target_os = "macos")]
    fn undo_sorting_fixture_files_from_backup(
        connection: &Connection,
        context: &FixtureContext,
        backup: &FixtureBackupPrototypeSuccess,
    ) -> AppResult<()> {
        if observe_expected_file(
            &context.source_path,
            backup.source_size,
            &backup.source_hash,
        )? != ObservedFileState::Missing
            || observe_expected_file(
                &context.destination_path,
                backup.source_size,
                &backup.source_hash,
            )? != ObservedFileState::Exact
        {
            return Err(AppError::Message(
                "Sorting fixture Undo requires the exact moved file and an absent original source."
                    .to_owned(),
            ));
        }

        match run_fixture_restore_prototype(
            connection,
            FixtureRestorePrototypeRequest {
                fixture_mode: true,
                fixture_root: context.fixture_root.clone(),
                backup_path: backup.backup_path.clone(),
                restore_target_path: context.source_path.clone(),
                apply_plan_id: context.plan_id,
                apply_plan_item_id: Some(context.item_id),
                run_id: context.run_id,
                result_id: Some(backup.result_log_id),
                restore_entry_id: Some(backup.restore_entry_id),
                operation_kind: UNDO_OPERATION_KIND.to_owned(),
            },
        )? {
            FixtureRestorePrototypeOutcome::Verified(_) => {}
            FixtureRestorePrototypeOutcome::FailedBeforeChange(failure) => {
                return Err(AppError::Message(format!(
                    "Sorting fixture Undo restore failed before change: {}",
                    failure.error_message
                )));
            }
        }

        if observe_expected_file(
            &context.source_path,
            backup.source_size,
            &backup.source_hash,
        )? != ObservedFileState::Exact
            || observe_expected_file(
                &context.destination_path,
                backup.source_size,
                &backup.source_hash,
            )? != ObservedFileState::Exact
        {
            return Err(AppError::Message(
                "Sorting fixture Undo restore verification failed before destination cleanup."
                    .to_owned(),
            ));
        }
        fs::remove_file(&context.destination_path).map_err(|error| {
            AppError::Message(format!(
                "Sorting fixture Undo restored the source but could not remove the verified moved copy: {error}"
            ))
        })?;
        if context.destination_path.exists() {
            return Err(AppError::Message(
                "Sorting fixture Undo destination still exists after verified cleanup.".to_owned(),
            ));
        }
        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn sorting_fixture_membership_states(
        sorting: &SortingFixtureContext,
    ) -> (FixtureMembershipState, FixtureMembershipState) {
        let context = &sorting.context;
        (
            FixtureMembershipState {
                file_id: sorting.file_id,
                path: context.source_path.to_string_lossy().to_string(),
                source_location: "mods".to_owned(),
                download_item_id: None,
            },
            FixtureMembershipState {
                file_id: sorting.file_id,
                path: context.destination_path.to_string_lossy().to_string(),
                source_location: "mods".to_owned(),
                download_item_id: None,
            },
        )
    }

    #[cfg(target_os = "macos")]
    fn apply_sorting_fixture_membership(
        connection: &Connection,
        sorting: &SortingFixtureContext,
        attempt_id: &str,
    ) -> (FixtureMembershipState, FixtureMembershipState) {
        release_fixture_attempt(connection, &sorting.context, attempt_id);
        let (original_membership, sorted_membership) = sorting_fixture_membership_states(sorting);
        let handoff = FixtureMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: original_membership.clone(),
                final_state: sorted_membership.clone(),
            },
        };
        commit_fixture_membership_handoffs(connection, std::slice::from_ref(&handoff))
            .expect("commit fixture sorting Library membership");
        (original_membership, sorted_membership)
    }

    #[cfg(target_os = "macos")]
    fn undo_sorting_fixture_membership(
        connection: &Connection,
        sorting: &SortingFixtureContext,
        backup: &FixtureBackupPrototypeSuccess,
        attempt_id: &str,
        original_membership: FixtureMembershipState,
        sorted_membership: FixtureMembershipState,
    ) {
        undo_sorting_fixture_files_from_backup(connection, &sorting.context, backup)
            .expect("restore fixture sorting bytes from verified backup");
        let handoff = FixtureReverseMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: sorted_membership,
                final_state: original_membership,
            },
        };
        commit_fixture_reverse_membership_handoffs(connection, std::slice::from_ref(&handoff))
            .expect("restore fixture sorting Library membership");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_batch_existing_destination_collision_blocks_before_backups_or_moves() {
        let mut connection = memory_connection();
        let batch = setup_sorting_batch_fixture(&mut connection);
        fs::create_dir(&batch.destination_dir).expect("create pre-existing Gameplay folder");
        let collision_path = batch.items[1].destination_path.clone();
        fs::write(&collision_path, b"pre-existing player file")
            .expect("create destination collision before batch Apply");

        let error = run_sorting_batch_apply(&connection, &batch, None)
            .expect_err("existing destination collision must block whole batch preflight");
        assert!(format!("{error:?}").contains("expected all-items backup-only validation stop"));
        assert_eq!(
            fs::read(&collision_path).expect("collision file remains untouched"),
            b"pre-existing player file"
        );
        for item in &batch.items {
            assert!(item.source_path.exists());
            if item.destination_path != collision_path {
                assert!(!item.destination_path.exists());
            }
        }
        let run = apply_plan_results::get_apply_plan_run_log(&connection, batch.run_id)
            .expect("load collision batch run")
            .expect("collision batch run exists");
        assert_eq!(
            run.results
                .iter()
                .filter(|result| result.operation_kind == BACKUP_OPERATION_KIND)
                .count(),
            0
        );
        assert_eq!(
            run.restore_entries
                .iter()
                .filter(|entry| entry.operation_kind == BACKUP_OPERATION_KIND)
                .count(),
            0
        );
        assert!(
            load_sorting_batch_receipt(&connection, &batch)
                .expect("load absent collision batch receipt")
                .is_none()
        );
        assert!(
            load_sorting_batch_directory_record(&connection, &batch)
                .expect("load absent collision folder ownership")
                .is_none()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_batch_stale_source_blocks_before_backups_folder_or_moves() {
        let mut connection = memory_connection();
        let batch = setup_sorting_batch_fixture(&mut connection);
        let stale_item = &batch.items[1];
        fs::write(&stale_item.source_path, b"changed after sorting preview")
            .expect("change one batch source after preview");

        let error = run_sorting_batch_apply(&connection, &batch, None)
            .expect_err("stale batch source must block before execution");
        assert!(format!("{error:?}").contains("changed"));
        assert!(!batch.destination_dir.exists());
        for item in &batch.items {
            assert!(item.source_path.exists());
            assert!(!item.destination_path.exists());
        }
        let run = apply_plan_results::get_apply_plan_run_log(&connection, batch.run_id)
            .expect("load stale batch run")
            .expect("stale batch run exists");
        assert_eq!(
            run.results
                .iter()
                .filter(|result| result.operation_kind == BACKUP_OPERATION_KIND)
                .count(),
            0
        );
        assert_eq!(
            run.restore_entries
                .iter()
                .filter(|entry| entry.operation_kind == BACKUP_OPERATION_KIND)
                .count(),
            0
        );
        assert!(
            load_sorting_batch_receipt(&connection, &batch)
                .expect("load absent stale batch receipt")
                .is_none()
        );
        assert!(
            load_sorting_batch_directory_record(&connection, &batch)
                .expect("load absent stale batch folder record")
                .is_none()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_batch_partial_prefix_retry_reuses_backups_and_skips_completed_item() {
        let mut connection = memory_connection();
        let batch = setup_sorting_batch_fixture(&mut connection);

        let partial = run_sorting_batch_apply(&connection, &batch, Some(1))
            .expect("stop sorting batch before second item");
        assert_eq!(partial.completed_item_ids, vec![batch.items[0].item_id]);
        assert_eq!(partial.pending_item_ids.len(), 2);
        assert_eq!(partial.stopped_before_item_id, Some(batch.items[1].item_id));
        assert_eq!(
            observe_expected_file(
                &batch.items[0].destination_path,
                batch.items[0].source_size,
                &batch.items[0].source_hash,
            )
            .expect("partial first destination"),
            ObservedFileState::Exact
        );
        assert_eq!(
            observe_expected_file(
                &batch.items[1].source_path,
                batch.items[1].source_size,
                &batch.items[1].source_hash,
            )
            .expect("partial second source"),
            ObservedFileState::Exact
        );
        let first_attempt_id = sorting_batch_attempt_id(&batch, &batch.items[0]);
        let first_attempt_before = load_fixture_attempt(&connection, &first_attempt_id)
            .expect("load partial first attempt")
            .expect("partial first attempt exists");
        assert_eq!(first_attempt_before.state, FixtureAttemptState::Committed);
        let before_retry = apply_plan_results::get_apply_plan_run_log(&connection, batch.run_id)
            .expect("load partial batch run")
            .expect("partial batch run exists");
        let backup_ids_before = before_retry
            .results
            .iter()
            .filter(|result| result.operation_kind == BACKUP_OPERATION_KIND)
            .map(|result| result.id)
            .collect::<Vec<_>>();
        assert_eq!(backup_ids_before.len(), 3);

        let completed = run_sorting_batch_apply(&connection, &batch, None)
            .expect("resume sorting batch without duplicating first item");
        assert_eq!(completed.completed_item_ids.len(), 3);
        assert!(completed.pending_item_ids.is_empty());
        assert_eq!(completed.stopped_before_item_id, None);
        let after_retry = apply_plan_results::get_apply_plan_run_log(&connection, batch.run_id)
            .expect("load resumed batch run")
            .expect("resumed batch run exists");
        let backup_ids_after = after_retry
            .results
            .iter()
            .filter(|result| result.operation_kind == BACKUP_OPERATION_KIND)
            .map(|result| result.id)
            .collect::<Vec<_>>();
        assert_eq!(backup_ids_after, backup_ids_before);
        let first_attempt_after = load_fixture_attempt(&connection, &first_attempt_id)
            .expect("reload first attempt after retry")
            .expect("first attempt still exists after retry");
        assert_eq!(first_attempt_after.attempt_id, first_attempt_before.attempt_id);
        assert_eq!(first_attempt_after.state, FixtureAttemptState::Committed);
        assert_eq!(
            run_sorting_batch_undo(&connection, &batch).expect("undo resumed sorting batch"),
            FixtureSortingDirectoryCleanupOutcome::Removed
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_batch_undo_restores_files_but_keeps_folder_with_player_content() {
        let mut connection = memory_connection();
        let batch = setup_sorting_batch_fixture(&mut connection);
        run_sorting_batch_apply(&connection, &batch, None)
            .expect("apply batch before player-content Undo test");
        let player_path = batch.destination_dir.join("player-added.package");
        fs::write(&player_path, b"player content SimSuite does not own")
            .expect("add player content to batch-owned folder");

        assert_eq!(
            run_sorting_batch_undo(&connection, &batch)
                .expect("undo batch while preserving player content"),
            FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty
        );
        assert!(batch.destination_dir.is_dir());
        assert_eq!(
            fs::read(&player_path).expect("player content survives Undo"),
            b"player content SimSuite does not own"
        );
        for item in &batch.items {
            assert_eq!(
                observe_expected_file(&item.source_path, item.source_size, &item.source_hash)
                    .expect("player-content restored source"),
                ObservedFileState::Exact
            );
            assert_eq!(
                observe_expected_file(
                    &item.destination_path,
                    item.source_size,
                    &item.source_hash,
                )
                .expect("player-content cleaned destination"),
                ObservedFileState::Missing
            );
            let membership = load_fixture_membership_state(&connection, item.file_id)
                .expect("load player-content restored membership")
                .expect("player-content membership exists");
            assert_eq!(PathBuf::from(membership.path), item.source_path);
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_batch_undo_tamper_blocks_before_restoring_any_other_file() {
        let mut connection = memory_connection();
        let batch = setup_sorting_batch_fixture(&mut connection);
        run_sorting_batch_apply(&connection, &batch, None)
            .expect("apply batch before tamper Undo test");
        fs::write(&batch.items[1].destination_path, b"tampered moved package")
            .expect("tamper one moved batch destination");

        let error = run_sorting_batch_undo(&connection, &batch)
            .expect_err("tampered batch Undo must fail before restoring any source");
        assert!(format!("{error:?}").contains("changed or ambiguous"));
        for (index, item) in batch.items.iter().enumerate() {
            assert_eq!(
                observe_expected_file(&item.source_path, item.source_size, &item.source_hash)
                    .expect("tamper test source remains absent"),
                ObservedFileState::Missing
            );
            let destination_state = observe_expected_file(
                &item.destination_path,
                item.source_size,
                &item.source_hash,
            )
            .expect("tamper test destination state");
            if index == 1 {
                assert_eq!(destination_state, ObservedFileState::Different);
            } else {
                assert_eq!(destination_state, ObservedFileState::Exact);
            }
            let membership = load_fixture_membership_state(&connection, item.file_id)
                .expect("load tamper test membership")
                .expect("tamper test membership exists");
            assert_eq!(PathBuf::from(membership.path), item.destination_path);
        }
        assert!(batch.destination_dir.is_dir());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn backend_sorting_batch_three_files_share_one_folder_and_undo_exactly() {
        let mut connection = memory_connection();
        let batch = setup_sorting_batch_fixture(&mut connection);
        assert_eq!(batch.items.len(), 3);
        assert!(!batch.destination_dir.exists());
        verify_sorting_batch_read_only_preflight(&connection, &batch)
            .expect("batch read-only preflight");
        for item in &batch.items {
            assert_eq!(
                observe_expected_file(&item.source_path, item.source_size, &item.source_hash)
                    .expect("initial batch source"),
                ObservedFileState::Exact
            );
            assert!(!item.destination_path.exists());
        }

        let outcome = run_sorting_batch_apply(&connection, &batch, None)
            .expect("apply sorting batch through fixture journal");
        assert_eq!(outcome.completed_item_ids.len(), 3);
        assert!(outcome.pending_item_ids.is_empty());
        assert_eq!(outcome.stopped_before_item_id, None);
        assert!(batch.destination_dir.is_dir());

        let receipt = load_sorting_batch_receipt(&connection, &batch)
            .expect("load batch receipt")
            .expect("batch receipt exists");
        assert_eq!(receipt.items.len(), 3);
        assert!(!receipt.batch_hash.is_empty());
        validate_sorting_batch_receipt(&connection, &batch, &receipt)
            .expect("validate sealed batch receipt");
        let directory_rows: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM fixture_sorting_batch_directories WHERE apply_plan_run_id = ?1",
                params![batch.run_id],
                |row| row.get(0),
            )
            .expect("count batch directory ownership rows");
        assert_eq!(directory_rows, 1);
        let directory = load_sorting_batch_directory_record(&connection, &batch)
            .expect("load batch directory record")
            .expect("batch directory record exists");
        assert_eq!(directory.state, FixtureSortingDirectoryState::CreatedVerified);

        for item in &batch.items {
            assert_eq!(
                observe_expected_file(&item.source_path, item.source_size, &item.source_hash)
                    .expect("moved batch source"),
                ObservedFileState::Missing
            );
            assert_eq!(
                observe_expected_file(
                    &item.destination_path,
                    item.source_size,
                    &item.source_hash,
                )
                .expect("moved batch destination"),
                ObservedFileState::Exact
            );
            let membership = load_fixture_membership_state(&connection, item.file_id)
                .expect("load moved batch membership")
                .expect("moved batch membership exists");
            assert_eq!(PathBuf::from(membership.path), item.destination_path);
            assert_eq!(membership.source_location, "mods");
            let attempt = load_fixture_attempt(
                &connection,
                &sorting_batch_attempt_id(&batch, item),
            )
            .expect("load batch attempt")
            .expect("batch attempt exists");
            assert_eq!(attempt.state, FixtureAttemptState::Committed);
        }

        assert_eq!(
            run_sorting_batch_undo(&connection, &batch).expect("undo sorting batch"),
            FixtureSortingDirectoryCleanupOutcome::Removed
        );
        assert!(!batch.destination_dir.exists());
        for item in &batch.items {
            assert_eq!(
                observe_expected_file(&item.source_path, item.source_size, &item.source_hash)
                    .expect("restored batch source"),
                ObservedFileState::Exact
            );
            assert_eq!(
                observe_expected_file(
                    &item.destination_path,
                    item.source_size,
                    &item.source_hash,
                )
                .expect("cleaned batch destination"),
                ObservedFileState::Missing
            );
            let membership = load_fixture_membership_state(&connection, item.file_id)
                .expect("load restored batch membership")
                .expect("restored batch membership exists");
            assert_eq!(PathBuf::from(membership.path), item.source_path);
            assert_eq!(membership.source_location, "mods");
        }
        assert_eq!(
            run_sorting_batch_undo(&connection, &batch).expect("repeat batch Undo safely"),
            FixtureSortingDirectoryCleanupOutcome::NotOwned
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn backend_sorting_plan_applies_and_undoes_through_verified_fixture_journal() {
        let mut connection = memory_connection();
        let source_bytes = b"sorting fixture gameplay package";
        let sorting = setup_sorting_fixture(&mut connection, source_bytes);
        let context = &sorting.context;

        let backup = run_verified_sorting_backup_gate(&connection, &sorting)
            .expect("sorting plan should pass preview, validation, stale-file, and backup gates");
        assert_eq!(
            classify_fixture_reconciliation_state(&connection, &reconciliation_request(context))
                .expect("sorting verified-backup reconciliation"),
            FixtureReconciliationState::ResumeFromVerifiedBackupRequired
        );

        let attempt_id = "attempt-sorting-apply-undo";
        release_fixture_attempt(&connection, context, attempt_id);
        let original_membership = FixtureMembershipState {
            file_id: sorting.file_id,
            path: context.source_path.to_string_lossy().to_string(),
            source_location: "mods".to_owned(),
            download_item_id: None,
        };
        let sorted_membership = FixtureMembershipState {
            file_id: sorting.file_id,
            path: context.destination_path.to_string_lossy().to_string(),
            source_location: "mods".to_owned(),
            download_item_id: None,
        };
        let forward_handoff = FixtureMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: original_membership.clone(),
                final_state: sorted_membership.clone(),
            },
        };
        commit_fixture_membership_handoffs(&connection, std::slice::from_ref(&forward_handoff))
            .expect("commit sorted Library membership after verified fixture move");

        assert_eq!(
            load_fixture_membership_state(&connection, sorting.file_id)
                .expect("sorted Library membership"),
            Some(sorted_membership.clone())
        );
        assert!(!context.source_path.exists());
        assert_eq!(fs::read(&context.destination_path).expect("sorted file"), source_bytes);
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("sorting attempt")
                .expect("sorting attempt row")
                .state,
            FixtureAttemptState::Committed
        );

        undo_sorting_fixture_files_from_backup(&connection, context, &backup)
            .expect("restore exact pre-sort bytes from verified backup");
        let reverse_handoff = FixtureReverseMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: sorted_membership,
                final_state: original_membership.clone(),
            },
        };
        commit_fixture_reverse_membership_handoffs(
            &connection,
            std::slice::from_ref(&reverse_handoff),
        )
        .expect("restore original Library membership after verified Undo");

        assert_eq!(
            load_fixture_membership_state(&connection, sorting.file_id)
                .expect("restored sorting membership"),
            Some(original_membership)
        );
        assert_eq!(fs::read(&context.source_path).expect("restored source"), source_bytes);
        assert!(!context.destination_path.exists());
        let final_hash_check = apply_plan_persistence::verify_apply_plan_hash(
            &connection,
            context.plan_id,
        )
        .expect("recheck original sorting plan provenance after Undo");
        assert!(final_hash_check.is_valid);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn backend_sorting_missing_folder_is_owned_and_removed_only_after_verified_undo() {
        let mut connection = memory_connection();
        let source_bytes = b"sorting fixture creates gameplay folder";
        let sorting = setup_sorting_fixture_with_destination_folder(
            &mut connection,
            source_bytes,
            false,
        );
        let destination_dir = sorting
            .context
            .destination_path
            .parent()
            .expect("sorting destination parent")
            .to_path_buf();
        assert!(!destination_dir.exists());

        let backup = run_verified_sorting_backup_gate(&connection, &sorting)
            .expect("sorting gate should create and own the missing organization folder");
        assert!(destination_dir.is_dir());
        let directory_record = load_fixture_sorting_directory_record(
            &connection,
            sorting.context.plan_id,
            sorting.context.item_id,
        )
        .expect("load sorting directory record")
        .expect("sorting directory record");
        assert_eq!(
            directory_record.state,
            FixtureSortingDirectoryState::CreatedVerified
        );
        assert!(directory_record.device_id.is_some());
        assert!(directory_record.inode.is_some());

        let attempt_id = "attempt-sorting-created-folder";
        let (original_membership, sorted_membership) =
            apply_sorting_fixture_membership(&connection, &sorting, attempt_id);
        assert_eq!(
            load_fixture_membership_state(&connection, sorting.file_id)
                .expect("sorted membership after created-folder move"),
            Some(sorted_membership.clone())
        );
        undo_sorting_fixture_membership(
            &connection,
            &sorting,
            &backup,
            attempt_id,
            original_membership.clone(),
            sorted_membership,
        );
        assert_eq!(
            load_fixture_membership_state(&connection, sorting.file_id)
                .expect("restored membership after created-folder Undo"),
            Some(original_membership)
        );

        assert_eq!(
            cleanup_fixture_sorting_destination_directory(&connection, &sorting)
                .expect("remove exact empty SimSuite-owned sorting folder"),
            FixtureSortingDirectoryCleanupOutcome::Removed
        );
        assert!(!destination_dir.exists());
        assert_eq!(
            fs::read(&sorting.context.source_path).expect("restored source after folder cleanup"),
            source_bytes
        );
        assert_eq!(
            cleanup_fixture_sorting_destination_directory(&connection, &sorting)
                .expect("repeat folder cleanup"),
            FixtureSortingDirectoryCleanupOutcome::AlreadyAbsent
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_undo_keeps_owned_folder_when_player_added_content() {
        let mut connection = memory_connection();
        let sorting = setup_sorting_fixture_with_destination_folder(
            &mut connection,
            b"sorting fixture player content guard",
            false,
        );
        let destination_dir = sorting
            .context
            .destination_path
            .parent()
            .expect("sorting destination parent")
            .to_path_buf();
        let backup = run_verified_sorting_backup_gate(&connection, &sorting)
            .expect("sorting gate with missing folder");
        let attempt_id = "attempt-sorting-player-content";
        let (original_membership, sorted_membership) =
            apply_sorting_fixture_membership(&connection, &sorting, attempt_id);
        undo_sorting_fixture_membership(
            &connection,
            &sorting,
            &backup,
            attempt_id,
            original_membership,
            sorted_membership,
        );

        let player_file = destination_dir.join("player-added.txt");
        fs::write(&player_file, b"player content").expect("player-added fixture content");
        assert_eq!(
            cleanup_fixture_sorting_destination_directory(&connection, &sorting)
                .expect("cleanup should preserve non-empty folder"),
            FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty
        );
        assert!(destination_dir.is_dir());
        assert_eq!(
            fs::read(&player_file).expect("player-added content survives"),
            b"player content"
        );
        assert_eq!(
            load_fixture_sorting_directory_record(
                &connection,
                sorting.context.plan_id,
                sorting.context.item_id,
            )
            .expect("load retained directory record")
            .expect("retained directory record")
            .state,
            FixtureSortingDirectoryState::RetainedNonEmpty
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_undo_keeps_owned_empty_folder_when_library_still_claims_a_child() {
        let mut connection = memory_connection();
        let sorting = setup_sorting_fixture_with_destination_folder(
            &mut connection,
            b"sorting fixture indexed child guard",
            false,
        );
        let destination_dir = sorting
            .context
            .destination_path
            .parent()
            .expect("sorting destination parent")
            .to_path_buf();
        let backup = run_verified_sorting_backup_gate(&connection, &sorting)
            .expect("sorting gate with missing folder");
        let attempt_id = "attempt-sorting-indexed-child";
        let (original_membership, sorted_membership) =
            apply_sorting_fixture_membership(&connection, &sorting, attempt_id);
        undo_sorting_fixture_membership(
            &connection,
            &sorting,
            &backup,
            attempt_id,
            original_membership,
            sorted_membership,
        );
        assert!(fs::read_dir(&destination_dir)
            .expect("empty destination directory")
            .next()
            .is_none());

        let indexed_child = destination_dir.join("library-owned.package");
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, hash, size, kind, confidence,
                    source_location, relative_depth, safety_notes, parser_warnings, insights
                 ) VALUES (?1, 'library-owned.package', 'package', ?2, 1,
                    'Gameplay', 1.0, 'mods', 1, '[]', '[]', '{}')",
                params![
                    indexed_child.to_string_lossy().to_string(),
                    bytes_hash(b"x"),
                ],
            )
            .expect("insert Library-only child claim");

        assert_eq!(
            cleanup_fixture_sorting_destination_directory(&connection, &sorting)
                .expect("Library-owned child must veto folder cleanup"),
            FixtureSortingDirectoryCleanupOutcome::RetainedNonEmpty
        );
        assert!(destination_dir.is_dir());
        assert!(fs::read_dir(&destination_dir)
            .expect("destination remains physically empty")
            .next()
            .is_none());
        assert_eq!(
            load_fixture_sorting_directory_record(
                &connection,
                sorting.context.plan_id,
                sorting.context.item_id,
            )
            .expect("load Library-vetoed directory record")
            .expect("Library-vetoed directory record")
            .state,
            FixtureSortingDirectoryState::RetainedNonEmpty
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn sorting_undo_keeps_replacement_folder_with_same_name() {
        let mut connection = memory_connection();
        let sorting = setup_sorting_fixture_with_destination_folder(
            &mut connection,
            b"sorting fixture replacement folder guard",
            false,
        );
        let destination_dir = sorting
            .context
            .destination_path
            .parent()
            .expect("sorting destination parent")
            .to_path_buf();
        let backup = run_verified_sorting_backup_gate(&connection, &sorting)
            .expect("sorting gate with owned destination folder");
        let attempt_id = "attempt-sorting-replaced-folder";
        let (original_membership, sorted_membership) =
            apply_sorting_fixture_membership(&connection, &sorting, attempt_id);
        undo_sorting_fixture_membership(
            &connection,
            &sorting,
            &backup,
            attempt_id,
            original_membership,
            sorted_membership,
        );

        let displaced_owned_dir = destination_dir
            .parent()
            .expect("Mods root")
            .join("Gameplay-original-owned");
        fs::rename(&destination_dir, &displaced_owned_dir)
            .expect("move exact owned directory out of expected path");
        fs::create_dir(&destination_dir).expect("create replacement directory with same name");
        assert_ne!(
            fixture_directory_identity(&destination_dir).expect("replacement identity"),
            fixture_directory_identity(&displaced_owned_dir).expect("original owned identity")
        );

        assert_eq!(
            cleanup_fixture_sorting_destination_directory(&connection, &sorting)
                .expect("cleanup should preserve replacement directory"),
            FixtureSortingDirectoryCleanupOutcome::RetainedIdentityChanged
        );
        assert!(destination_dir.is_dir());
        assert!(displaced_owned_dir.is_dir());
        assert_eq!(
            load_fixture_sorting_directory_record(
                &connection,
                sorting.context.plan_id,
                sorting.context.item_id,
            )
            .expect("load identity-changed directory record")
            .expect("identity-changed directory record")
            .state,
            FixtureSortingDirectoryState::RetainedIdentityChanged
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn interrupted_sorting_folder_create_never_recovers_delete_ownership_by_guessing() {
        let mut connection = memory_connection();
        let sorting = setup_sorting_fixture_with_destination_folder(
            &mut connection,
            b"sorting fixture interrupted folder ownership",
            false,
        );
        let destination_dir = sorting
            .context
            .destination_path
            .parent()
            .expect("sorting destination parent")
            .to_path_buf();
        let backup = run_verified_sorting_preflight_and_backup(&connection, &sorting)
            .expect("sorting preflight and backup before folder interruption");

        let error = ensure_fixture_sorting_destination_directory_with_fault(
            &connection,
            &sorting,
            &backup,
            FixtureSortingDirectoryFaultPoint::AfterCreateBeforeOwnershipFinalize,
        )
        .expect_err("injected interruption after folder creation");
        assert!(error.to_string().contains("ownership finalization"));
        assert!(destination_dir.is_dir());
        let prepared = load_fixture_sorting_directory_record(
            &connection,
            sorting.context.plan_id,
            sorting.context.item_id,
        )
        .expect("load interrupted directory intent")
        .expect("interrupted directory intent");
        assert_eq!(
            prepared.state,
            FixtureSortingDirectoryState::PreparedBeforeCreate
        );
        assert_eq!(prepared.device_id, None);
        assert_eq!(prepared.inode, None);

        assert_eq!(
            ensure_fixture_sorting_destination_directory_with_fault(
                &connection,
                &sorting,
                &backup,
                FixtureSortingDirectoryFaultPoint::None,
            )
            .expect("retry interrupted folder creation conservatively"),
            FixtureSortingDirectoryCreateOutcome::OwnershipUnknownKeep
        );
        let recovered = load_fixture_sorting_directory_record(
            &connection,
            sorting.context.plan_id,
            sorting.context.item_id,
        )
        .expect("load conservatively recovered directory")
        .expect("conservatively recovered directory");
        assert_eq!(
            recovered.state,
            FixtureSortingDirectoryState::OwnershipUnknownKeep
        );
        assert_eq!(recovered.device_id, None);
        assert_eq!(recovered.inode, None);

        record_fixture_sorting_authorization(
            &connection,
            &reconciliation_request(&sorting.context),
            backup.result_log_id,
            backup.restore_entry_id,
        )
        .expect("sorting authorization can proceed while folder remains non-owned");
        assert_eq!(
            classify_fixture_reconciliation_state(
                &connection,
                &reconciliation_request(&sorting.context),
            )
            .expect("reconciliation after conservative folder recovery"),
            FixtureReconciliationState::ResumeFromVerifiedBackupRequired
        );
        assert_eq!(
            cleanup_fixture_sorting_destination_directory(&connection, &sorting)
                .expect("cleanup must not claim interrupted folder"),
            FixtureSortingDirectoryCleanupOutcome::OwnershipUnknownKeep
        );
        assert!(destination_dir.is_dir());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn backend_sorting_plan_blocks_changed_source_before_backup_or_move() {
        let mut connection = memory_connection();
        let sorting = setup_sorting_fixture(&mut connection, b"0123456789");
        fs::write(&sorting.context.source_path, b"abcdefghij")
            .expect("change fixture file after sorting preview");

        let error = run_verified_sorting_backup_gate(&connection, &sorting)
            .expect_err("changed source must block sorting before backup");
        assert!(error
            .to_string()
            .contains("source changed after the sorting preview"));
        assert!(sorting.context.source_path.exists());
        assert!(!sorting.context.destination_path.exists());
        assert_eq!(
            list_apply_plan_result_logs(
                &connection,
                ListApplyPlanResultLogsRequest {
                    apply_plan_run_id: sorting.context.run_id,
                },
            )
            .expect("sorting result logs after stale source")
            .len(),
            0
        );
        assert!(fs::read_dir(&sorting.context.backup_root)
            .expect("sorting backup root")
            .next()
            .is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn backend_sorting_suggestion_with_backup_but_without_fixture_authorization_stays_blocked() {
        let mut connection = memory_connection();
        let sorting = setup_sorting_fixture(&mut connection, b"sorting suggestion only");
        let context = &sorting.context;

        let backup = run_fixture_backup_prototype(
            &connection,
            FixtureBackupPrototypeRequest {
                fixture_mode: true,
                apply_plan_id: context.plan_id,
                apply_plan_item_id: Some(context.item_id),
                run_id: context.run_id,
                fixture_root: context.fixture_root.clone(),
                source_path: context.source_path.clone(),
                backup_root: context.backup_root.clone(),
                operation_kind: BACKUP_OPERATION_KIND.to_owned(),
            },
        )
        .expect("fixture backup without sorting authorization");
        assert!(matches!(backup, FixtureBackupPrototypeOutcome::Verified(_)));

        let state = classify_fixture_reconciliation_state(
            &connection,
            &reconciliation_request(context),
        )
        .expect("sorting suggestion reconciliation without authorization");
        assert!(matches!(
            state,
            FixtureReconciliationState::Ambiguous(ref reason)
                if reason.contains("fixture-authorized sorting suggestion")
        ));
        assert!(context.source_path.exists());
        assert!(!context.destination_path.exists());
        assert!(load_fixture_attempt(&connection, "attempt-without-sorting-authorization")
            .expect("attempt lookup")
            .is_none());
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
    fn membership_handoff_commits_only_after_verified_release_and_repeats_safely() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(
            &connection,
            b"membership handoff success bytes",
        );
        let download_item_id = 7_001;
        let file_id = attach_fixture_download_owner(&connection, &context, download_item_id);
        let attempt_id = "attempt-membership-success";
        let handoff = fixture_membership_handoff(
            &context,
            file_id,
            download_item_id,
            attempt_id,
        );
        let expected = match &handoff.action {
            FixtureMembershipAction::Transition { expected, .. } => expected.clone(),
            _ => unreachable!(),
        };
        let final_state = match &handoff.action {
            FixtureMembershipAction::Transition { final_state, .. } => final_state.clone(),
            _ => unreachable!(),
        };

        release_fixture_attempt(&connection, &context, attempt_id);
        assert_eq!(
            load_fixture_membership_state(&connection, file_id).expect("membership before handoff"),
            Some(expected)
        );
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("attempt before handoff")
                .expect("attempt")
                .state,
            FixtureAttemptState::SourceReleaseCompleted
        );

        commit_fixture_membership_handoffs(&connection, std::slice::from_ref(&handoff))
            .expect("commit fixture membership handoff");
        assert_eq!(
            load_fixture_membership_state(&connection, file_id).expect("membership after handoff"),
            Some(final_state.clone())
        );
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("attempt after handoff")
                .expect("attempt")
                .state,
            FixtureAttemptState::Committed
        );

        commit_fixture_membership_handoffs(&connection, std::slice::from_ref(&handoff))
            .expect("repeat committed handoff");
        assert_eq!(
            load_fixture_membership_state(&connection, file_id).expect("repeated membership"),
            Some(final_state)
        );
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("repeated attempt")
                .expect("attempt")
                .state,
            FixtureAttemptState::Committed
        );
        transition_fixture_attempt(
            &connection,
            attempt_id,
            FixtureAttemptState::RecoveryRequired,
        )
        .expect_err("committed handoff attempt must remain immutable");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn membership_handoff_can_delete_old_membership_after_verified_file_release() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(
            &connection,
            b"membership replacement old installed bytes",
        );
        let file_id = load_plan_item_scope(&connection, context.plan_id, context.item_id)
            .expect("old replacement plan item")
            .file_id;
        connection
            .execute(
                "UPDATE files
                 SET source_location = 'mods', download_item_id = NULL
                 WHERE id = ?1",
                params![file_id],
            )
            .expect("mark old replacement row as installed");
        let expected = FixtureMembershipState {
            file_id,
            path: context.source_path.to_string_lossy().to_string(),
            source_location: "mods".to_owned(),
            download_item_id: None,
        };
        let attempt_id = "attempt-membership-delete-old";
        let handoff = FixtureMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Delete {
                expected: expected.clone(),
            },
        };

        release_fixture_attempt(&connection, &context, attempt_id);
        assert_eq!(
            load_fixture_membership_state(&connection, file_id)
                .expect("old membership before delete handoff"),
            Some(expected)
        );
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());

        commit_fixture_membership_handoffs(&connection, std::slice::from_ref(&handoff))
            .expect("delete old membership after verified release");
        assert!(load_fixture_membership_state(&connection, file_id)
            .expect("old membership removed")
            .is_none());
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("delete attempt after handoff")
                .expect("delete attempt")
                .state,
            FixtureAttemptState::Committed
        );
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());

        commit_fixture_membership_handoffs(&connection, std::slice::from_ref(&handoff))
            .expect("repeat delete handoff safely");
        assert!(load_fixture_membership_state(&connection, file_id)
            .expect("old membership remains removed")
            .is_none());
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("repeated delete attempt")
                .expect("delete attempt")
                .state,
            FixtureAttemptState::Committed
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn membership_handoff_replacement_batch_commits_old_delete_and_incoming_install_together() {
        let connection = memory_connection();
        let old = setup_verified_backup_only_state(
            &connection,
            b"replacement batch old installed bytes",
        );
        let incoming = setup_verified_backup_only_state(
            &connection,
            b"replacement batch incoming bytes",
        );

        let old_file_id = load_plan_item_scope(&connection, old.plan_id, old.item_id)
            .expect("old replacement item")
            .file_id;
        connection
            .execute(
                "UPDATE files
                 SET source_location = 'mods', download_item_id = NULL
                 WHERE id = ?1",
                params![old_file_id],
            )
            .expect("mark old row installed");
        let old_expected = FixtureMembershipState {
            file_id: old_file_id,
            path: old.source_path.to_string_lossy().to_string(),
            source_location: "mods".to_owned(),
            download_item_id: None,
        };
        let old_attempt_id = "attempt-replacement-old";
        let old_handoff = FixtureMembershipHandoff {
            attempt_id: old_attempt_id.to_owned(),
            action: FixtureMembershipAction::Delete {
                expected: old_expected,
            },
        };

        let incoming_download_id = 7_051;
        let incoming_file_id =
            attach_fixture_download_owner(&connection, &incoming, incoming_download_id);
        let incoming_attempt_id = "attempt-replacement-incoming";
        let incoming_handoff = fixture_membership_handoff(
            &incoming,
            incoming_file_id,
            incoming_download_id,
            incoming_attempt_id,
        );
        let incoming_final = match &incoming_handoff.action {
            FixtureMembershipAction::Transition { final_state, .. } => final_state.clone(),
            _ => unreachable!(),
        };

        release_fixture_attempt(&connection, &old, old_attempt_id);
        release_fixture_attempt(&connection, &incoming, incoming_attempt_id);
        commit_fixture_membership_handoffs(
            &connection,
            &[old_handoff.clone(), incoming_handoff.clone()],
        )
        .expect("commit replacement membership batch");

        assert!(load_fixture_membership_state(&connection, old_file_id)
            .expect("old replacement membership")
            .is_none());
        assert_eq!(
            load_fixture_membership_state(&connection, incoming_file_id)
                .expect("incoming installed membership"),
            Some(incoming_final.clone())
        );
        for attempt_id in [old_attempt_id, incoming_attempt_id] {
            assert_eq!(
                load_fixture_attempt(&connection, attempt_id)
                    .expect("replacement attempt")
                    .expect("attempt")
                    .state,
                FixtureAttemptState::Committed
            );
        }
        assert!(!old.source_path.exists());
        assert!(old.destination_path.exists());
        assert!(!incoming.source_path.exists());
        assert!(incoming.destination_path.exists());

        commit_fixture_membership_handoffs(
            &connection,
            &[old_handoff, incoming_handoff],
        )
        .expect("repeat replacement membership batch safely");
        assert!(load_fixture_membership_state(&connection, old_file_id)
            .expect("old replacement stays absent")
            .is_none());
        assert_eq!(
            load_fixture_membership_state(&connection, incoming_file_id)
                .expect("incoming replacement stays installed"),
            Some(incoming_final)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reverse_replacement_membership_restores_original_semantics_with_new_old_row_id() {
        let connection = memory_connection();
        let old = setup_verified_backup_only_state(
            &connection,
            b"reverse replacement old installed bytes",
        );
        let incoming = setup_verified_backup_only_state(
            &connection,
            b"reverse replacement incoming bytes",
        );

        let old_file_id = load_plan_item_scope(&connection, old.plan_id, old.item_id)
            .expect("old reverse item")
            .file_id;
        connection
            .execute(
                "UPDATE files SET source_location = 'mods', download_item_id = NULL WHERE id = ?1",
                params![old_file_id],
            )
            .expect("mark old reverse row installed");
        let old_attempt_id = "attempt-reverse-replacement-old";
        let old_forward = FixtureMembershipHandoff {
            attempt_id: old_attempt_id.to_owned(),
            action: FixtureMembershipAction::Delete {
                expected: FixtureMembershipState {
                    file_id: old_file_id,
                    path: old.source_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: None,
                },
            },
        };

        let incoming_download_id = 7_061;
        let incoming_file_id =
            attach_fixture_download_owner(&connection, &incoming, incoming_download_id);
        let incoming_attempt_id = "attempt-reverse-replacement-incoming";
        let incoming_forward = fixture_membership_handoff(
            &incoming,
            incoming_file_id,
            incoming_download_id,
            incoming_attempt_id,
        );
        let incoming_installed = match &incoming_forward.action {
            FixtureMembershipAction::Transition { final_state, .. } => final_state.clone(),
            _ => unreachable!(),
        };

        release_fixture_attempt(&connection, &old, old_attempt_id);
        release_fixture_attempt(&connection, &incoming, incoming_attempt_id);
        commit_fixture_membership_handoffs(
            &connection,
            &[old_forward, incoming_forward],
        )
        .expect("commit forward replacement before reverse");

        move_engine::move_single_file(&old.destination_path, &old.source_path)
            .expect("restore old fixture bytes");
        move_engine::move_single_file(&incoming.destination_path, &incoming.source_path)
            .expect("return incoming fixture bytes");

        let restored_old_id = old_file_id + 10_000;
        let old_reverse = FixtureReverseMembershipHandoff {
            attempt_id: old_attempt_id.to_owned(),
            action: FixtureMembershipAction::InsertRestored {
                final_state: FixtureMembershipState {
                    file_id: restored_old_id,
                    path: old.source_path.to_string_lossy().to_string(),
                    source_location: "mods".to_owned(),
                    download_item_id: None,
                },
            },
        };
        let incoming_reverse = FixtureReverseMembershipHandoff {
            attempt_id: incoming_attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: incoming_installed,
                final_state: FixtureMembershipState {
                    file_id: incoming_file_id,
                    path: incoming.source_path.to_string_lossy().to_string(),
                    source_location: "downloads".to_owned(),
                    download_item_id: Some(incoming_download_id),
                },
            },
        };

        commit_fixture_reverse_membership_handoffs(
            &connection,
            &[old_reverse.clone(), incoming_reverse.clone()],
        )
        .expect("restore replacement membership batch");

        assert!(load_fixture_membership_state(&connection, old_file_id)
            .expect("historical old id")
            .is_none());
        assert_eq!(
            load_fixture_membership_state(&connection, restored_old_id)
                .expect("restored old membership"),
            Some(FixtureMembershipState {
                file_id: restored_old_id,
                path: old.source_path.to_string_lossy().to_string(),
                source_location: "mods".to_owned(),
                download_item_id: None,
            })
        );
        assert_eq!(
            load_fixture_membership_state(&connection, incoming_file_id)
                .expect("returned incoming membership"),
            Some(FixtureMembershipState {
                file_id: incoming_file_id,
                path: incoming.source_path.to_string_lossy().to_string(),
                source_location: "downloads".to_owned(),
                download_item_id: Some(incoming_download_id),
            })
        );
        assert!(old.source_path.exists());
        assert!(!old.destination_path.exists());
        assert!(incoming.source_path.exists());
        assert!(!incoming.destination_path.exists());

        commit_fixture_reverse_membership_handoffs(
            &connection,
            &[old_reverse, incoming_reverse],
        )
        .expect("repeat reverse replacement safely");
        assert!(load_fixture_membership_state(&connection, restored_old_id)
            .expect("restored old row remains")
            .is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn reverse_membership_handoff_blocks_tampered_restored_source_without_database_claim() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(
            &connection,
            b"reverse tamper original bytes",
        );
        let download_item_id = 7_071;
        let file_id = attach_fixture_download_owner(&connection, &context, download_item_id);
        let attempt_id = "attempt-reverse-tamper";
        let forward = fixture_membership_handoff(
            &context,
            file_id,
            download_item_id,
            attempt_id,
        );
        let installed = match &forward.action {
            FixtureMembershipAction::Transition { final_state, .. } => final_state.clone(),
            _ => unreachable!(),
        };

        release_fixture_attempt(&connection, &context, attempt_id);
        commit_fixture_membership_handoffs(&connection, std::slice::from_ref(&forward))
            .expect("commit forward membership before reverse tamper");
        move_engine::move_single_file(&context.destination_path, &context.source_path)
            .expect("return fixture source before tamper");
        fs::write(&context.source_path, b"tampered restored bytes")
            .expect("tamper restored fixture source");

        let reverse = FixtureReverseMembershipHandoff {
            attempt_id: attempt_id.to_owned(),
            action: FixtureMembershipAction::Transition {
                expected: installed.clone(),
                final_state: FixtureMembershipState {
                    file_id,
                    path: context.source_path.to_string_lossy().to_string(),
                    source_location: "downloads".to_owned(),
                    download_item_id: Some(download_item_id),
                },
            },
        };
        let error = commit_fixture_reverse_membership_handoffs(
            &connection,
            std::slice::from_ref(&reverse),
        )
        .expect_err("tampered restored bytes must block reverse membership");
        assert!(error
            .to_string()
            .contains("exact source to be restored"));
        assert_eq!(
            load_fixture_membership_state(&connection, file_id)
                .expect("membership remains at committed forward state"),
            Some(installed)
        );
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("tampered reverse attempt")
                .expect("attempt")
                .state,
            FixtureAttemptState::Committed
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn membership_handoff_second_commit_failure_rolls_back_all_database_claims() {
        let connection = memory_connection();
        let first = setup_verified_backup_only_state(
            &connection,
            b"membership handoff batch first bytes",
        );
        let second = setup_verified_backup_only_state(
            &connection,
            b"membership handoff batch second bytes",
        );
        let first_download_id = 7_101;
        let second_download_id = 7_102;
        let first_file_id =
            attach_fixture_download_owner(&connection, &first, first_download_id);
        let second_file_id =
            attach_fixture_download_owner(&connection, &second, second_download_id);
        let first_attempt_id = "attempt-membership-batch-first";
        let second_attempt_id = "attempt-membership-batch-second";
        let first_handoff = fixture_membership_handoff(
            &first,
            first_file_id,
            first_download_id,
            first_attempt_id,
        );
        let second_handoff = fixture_membership_handoff(
            &second,
            second_file_id,
            second_download_id,
            second_attempt_id,
        );
        let first_expected = match &first_handoff.action {
            FixtureMembershipAction::Transition { expected, .. } => expected.clone(),
            _ => unreachable!(),
        };
        let second_expected = match &second_handoff.action {
            FixtureMembershipAction::Transition { expected, .. } => expected.clone(),
            _ => unreachable!(),
        };

        release_fixture_attempt(&connection, &first, first_attempt_id);
        release_fixture_attempt(&connection, &second, second_attempt_id);
        connection
            .execute_batch(
                "CREATE TRIGGER reject_second_membership_handoff_commit
                 BEFORE UPDATE OF state ON fixture_apply_plan_attempts
                 WHEN OLD.attempt_id = 'attempt-membership-batch-second'
                  AND NEW.state = 'committed'
                 BEGIN
                    SELECT RAISE(ABORT, 'forced second membership handoff failure');
                 END;",
            )
            .expect("install second handoff failure");

        let error = commit_fixture_membership_handoffs(
            &connection,
            &[first_handoff.clone(), second_handoff.clone()],
        )
        .expect_err("second handoff DB failure must roll back the full DB batch");
        assert!(error
            .to_string()
            .contains("forced second membership handoff failure"));
        assert_eq!(
            load_fixture_membership_state(&connection, first_file_id)
                .expect("first membership rolled back"),
            Some(first_expected)
        );
        assert_eq!(
            load_fixture_membership_state(&connection, second_file_id)
                .expect("second membership rolled back"),
            Some(second_expected)
        );
        for attempt_id in [first_attempt_id, second_attempt_id] {
            assert_eq!(
                load_fixture_attempt(&connection, attempt_id)
                    .expect("rolled-back attempt")
                    .expect("attempt")
                    .state,
                FixtureAttemptState::SourceReleaseCompleted
            );
        }
        assert!(!first.source_path.exists());
        assert!(first.destination_path.exists());
        assert!(!second.source_path.exists());
        assert!(second.destination_path.exists());

        connection
            .execute_batch("DROP TRIGGER reject_second_membership_handoff_commit;")
            .expect("remove forced handoff failure");
        commit_fixture_membership_handoffs(
            &connection,
            &[first_handoff, second_handoff],
        )
        .expect("retry complete handoff batch");
        for attempt_id in [first_attempt_id, second_attempt_id] {
            assert_eq!(
                load_fixture_attempt(&connection, attempt_id)
                    .expect("retried attempt")
                    .expect("attempt")
                    .state,
                FixtureAttemptState::Committed
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn membership_handoff_changed_destination_blocks_before_database_commit() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(
            &connection,
            b"membership handoff changed destination bytes",
        );
        let download_item_id = 7_151;
        let file_id = attach_fixture_download_owner(&connection, &context, download_item_id);
        let attempt_id = "attempt-membership-destination-changed";
        let handoff = fixture_membership_handoff(
            &context,
            file_id,
            download_item_id,
            attempt_id,
        );
        let expected = match &handoff.action {
            FixtureMembershipAction::Transition { expected, .. } => expected.clone(),
            _ => unreachable!(),
        };
        release_fixture_attempt(&connection, &context, attempt_id);
        fs::write(&context.destination_path, b"changed after verified release")
            .expect("change fixture destination before membership handoff");

        let error = commit_fixture_membership_handoffs(
            &connection,
            std::slice::from_ref(&handoff),
        )
        .expect_err("changed destination must block membership handoff");
        assert!(error
            .to_string()
            .contains("destination bytes to match the durable attempt evidence"));
        assert_eq!(
            load_fixture_membership_state(&connection, file_id)
                .expect("membership remains uncommitted"),
            Some(expected)
        );
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("changed destination attempt")
                .expect("attempt")
                .state,
            FixtureAttemptState::SourceReleaseCompleted
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn membership_handoff_stale_library_ownership_fails_closed_without_false_commit() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(
            &connection,
            b"membership handoff stale ownership bytes",
        );
        let download_item_id = 7_201;
        let file_id = attach_fixture_download_owner(&connection, &context, download_item_id);
        let attempt_id = "attempt-membership-stale";
        let handoff = fixture_membership_handoff(
            &context,
            file_id,
            download_item_id,
            attempt_id,
        );
        release_fixture_attempt(&connection, &context, attempt_id);

        let newer_state = FixtureMembershipState {
            file_id,
            path: context
                .fixture_root
                .join("newer-owned.package")
                .to_string_lossy()
                .to_string(),
            source_location: "tray".to_owned(),
            download_item_id: None,
        };
        connection
            .execute(
                "UPDATE files
                 SET path = ?1, source_location = ?2, download_item_id = ?3
                 WHERE id = ?4",
                params![
                    newer_state.path,
                    newer_state.source_location,
                    newer_state.download_item_id,
                    newer_state.file_id,
                ],
            )
            .expect("simulate newer membership owner");

        let error = commit_fixture_membership_handoffs(
            &connection,
            std::slice::from_ref(&handoff),
        )
        .expect_err("stale Library ownership must block handoff commit");
        assert!(error.to_string().contains("changed before transition"));
        assert_eq!(
            load_fixture_membership_state(&connection, file_id).expect("newer ownership survives"),
            Some(newer_state)
        );
        assert_eq!(
            load_fixture_attempt(&connection, attempt_id)
                .expect("stale attempt remains recoverable")
                .expect("attempt")
                .state,
            FixtureAttemptState::SourceReleaseCompleted
        );
        assert!(!context.source_path.exists());
        assert!(context.destination_path.exists());
    }

    #[test]
    fn attempt_journal_prepares_exact_record_and_rejects_duplicate_active_attempt() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(&connection, b"attempt journal exact bytes");
        let request = reconciliation_request(&context);
        let capability = fixture_attempt_capability_proof();
        let assessment = assess_fixture_reconciliation_state(&connection, &request)
            .expect("verified-backup assessment");
        let evidence = assessment.evidence.expect("verified-backup evidence");

        let attempt = prepare_fixture_attempt(&connection, &request, "attempt-exact-1", &capability)
            .expect("prepare fixture attempt");
        assert_eq!(attempt.attempt_id, "attempt-exact-1");
        assert_eq!(attempt.apply_plan_run_id, context.run_id);
        assert_eq!(attempt.apply_plan_id, context.plan_id);
        assert_eq!(attempt.apply_plan_item_id, context.item_id);
        assert_eq!(attempt.source_path, evidence.source_path.to_string_lossy().to_string());
        assert_eq!(
            attempt.destination_path,
            evidence.destination_path.to_string_lossy().to_string()
        );
        assert_eq!(attempt.expected_hash, evidence.expected_hash);
        assert_eq!(attempt.expected_size, evidence.expected_size);
        assert_eq!(attempt.backup_result_id, evidence.backup_result_id);
        assert_eq!(
            attempt.backup_restore_entry_id,
            evidence.backup_restore_entry_id
        );
        assert_eq!(attempt.capability, capability);
        assert!(attempt.source_regular_file);
        assert!(attempt.source_entry_non_symlink);
        assert!(attempt.destination_entry_non_symlink);
        assert_eq!(attempt.state, FixtureAttemptState::PreparedBeforeChange);

        let duplicate = prepare_fixture_attempt(
            &connection,
            &request,
            "attempt-exact-2",
            &fixture_attempt_capability_proof(),
        )
        .expect_err("second active attempt must be rejected");
        assert!(duplicate.to_string().contains("already has an active attempt"));
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM fixture_apply_plan_attempts",
                [],
                |row| row.get(0),
            )
            .expect("attempt row count");
        assert_eq!(count, 1);
    }

    #[test]
    fn attempt_journal_rejects_incomplete_capability_scope_and_evidence_drift() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(&connection, b"capability rejection bytes");
        let request = reconciliation_request(&context);
        let mut incomplete = fixture_attempt_capability_proof();
        incomplete.hard_link_supported = false;
        prepare_fixture_attempt(&connection, &request, "attempt-bad-capability", &incomplete)
            .expect_err("incomplete capability must fail");
        assert!(load_fixture_attempt(&connection, "attempt-bad-capability")
            .expect("load rejected attempt")
            .is_none());

        let other = setup_fixture(&connection, b"other run bytes");
        let mut cross_scoped = request.clone();
        cross_scoped.run_id = other.run_id;
        prepare_fixture_attempt(
            &connection,
            &cross_scoped,
            "attempt-cross-scope",
            &fixture_attempt_capability_proof(),
        )
        .expect_err("cross-scoped run must fail");
        assert!(load_fixture_attempt(&connection, "attempt-cross-scope")
            .expect("load cross-scope attempt")
            .is_none());

        let drift_connection = memory_connection();
        let drift_context =
            setup_verified_backup_only_state(&drift_connection, b"source drift original bytes");
        fs::write(&drift_context.source_path, b"source drift changed bytes")
            .expect("tamper source after backup");
        prepare_fixture_attempt(
            &drift_connection,
            &reconciliation_request(&drift_context),
            "attempt-drift",
            &fixture_attempt_capability_proof(),
        )
        .expect_err("drifted source must fail preparation");
        assert!(load_fixture_attempt(&drift_connection, "attempt-drift")
            .expect("load drifted attempt")
            .is_none());
    }

    #[test]
    fn attempt_journal_enforces_monotonic_transitions_and_reserves_commit_for_membership_handoff() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(&connection, b"state-machine bytes");
        let request = reconciliation_request(&context);
        prepare_fixture_attempt(
            &connection,
            &request,
            "attempt-state-1",
            &fixture_attempt_capability_proof(),
        )
        .expect("prepare state-machine attempt");

        transition_fixture_attempt(
            &connection,
            "attempt-state-1",
            FixtureAttemptState::DestinationVerified,
        )
        .expect_err("state skipping must fail");
        assert_eq!(
            load_fixture_attempt(&connection, "attempt-state-1")
                .expect("load after rejected skip")
                .expect("attempt")
                .state,
            FixtureAttemptState::PreparedBeforeChange
        );

        for next in [
            FixtureAttemptState::DestinationClaimObserved,
            FixtureAttemptState::DestinationVerified,
            FixtureAttemptState::SourceReleaseCompleted,
        ] {
            let record = transition_fixture_attempt(&connection, "attempt-state-1", next)
                .expect("allowed monotonic transition");
            assert_eq!(record.state, next);
        }

        transition_fixture_attempt(
            &connection,
            "attempt-state-1",
            FixtureAttemptState::Committed,
        )
        .expect_err("generic state transition must not bypass the membership handoff");
        transition_fixture_attempt(
            &connection,
            "attempt-state-1",
            FixtureAttemptState::PreparedBeforeChange,
        )
        .expect_err("backward transition must fail");
        assert_eq!(
            load_fixture_attempt(&connection, "attempt-state-1")
                .expect("load released attempt")
                .expect("attempt")
                .state,
            FixtureAttemptState::SourceReleaseCompleted
        );
    }

    #[test]
    fn attempt_journal_terminal_before_change_allows_later_historical_attempt() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(&connection, b"historical attempt bytes");
        let request = reconciliation_request(&context);
        prepare_fixture_attempt(
            &connection,
            &request,
            "attempt-history-1",
            &fixture_attempt_capability_proof(),
        )
        .expect("prepare first historical attempt");
        let blocked = transition_fixture_attempt(
            &connection,
            "attempt-history-1",
            FixtureAttemptState::BlockedBeforeChange,
        )
        .expect("block first attempt before change");
        assert_eq!(blocked.state, FixtureAttemptState::BlockedBeforeChange);

        let second = prepare_fixture_attempt(
            &connection,
            &request,
            "attempt-history-2",
            &fixture_attempt_capability_proof(),
        )
        .expect("terminal historical attempt must allow later retry");
        assert_eq!(second.state, FixtureAttemptState::PreparedBeforeChange);
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM fixture_apply_plan_attempts
                 WHERE apply_plan_run_id = ?1 AND apply_plan_item_id = ?2",
                params![context.run_id, context.item_id],
                |row| row.get(0),
            )
            .expect("historical attempt count");
        assert_eq!(count, 2);
    }

    #[test]
    fn attempt_journal_recovery_required_blocks_replacement_and_is_frozen() {
        let connection = memory_connection();
        let context = setup_verified_backup_only_state(&connection, b"recovery-required bytes");
        let request = reconciliation_request(&context);
        prepare_fixture_attempt(
            &connection,
            &request,
            "attempt-recovery-1",
            &fixture_attempt_capability_proof(),
        )
        .expect("prepare recovery attempt");
        transition_fixture_attempt(
            &connection,
            "attempt-recovery-1",
            FixtureAttemptState::DestinationClaimObserved,
        )
        .expect("record destination claim");
        let recovery = transition_fixture_attempt(
            &connection,
            "attempt-recovery-1",
            FixtureAttemptState::RecoveryRequired,
        )
        .expect("enter recovery-required state");
        assert_eq!(recovery.state, FixtureAttemptState::RecoveryRequired);

        prepare_fixture_attempt(
            &connection,
            &request,
            "attempt-recovery-2",
            &fixture_attempt_capability_proof(),
        )
        .expect_err("recovery-required attempt must block replacement");
        transition_fixture_attempt(
            &connection,
            "attempt-recovery-1",
            FixtureAttemptState::Committed,
        )
        .expect_err("recovery-required state is frozen in this prototype");
        assert_eq!(
            load_fixture_attempt(&connection, "attempt-recovery-1")
                .expect("load recovery attempt")
                .expect("attempt")
                .state,
            FixtureAttemptState::RecoveryRequired
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn attempt_journal_requires_matching_prepared_attempt_to_attribute_hard_link_pair() {
        let unowned_connection = memory_connection();
        let unowned_context = setup_verified_backup_only_state(
            &unowned_connection,
            b"unowned hard-link pair bytes",
        );
        claim_fixture_destination_no_replace(
            &unowned_context.source_path,
            &unowned_context.destination_path,
        )
        .expect("create unowned hard-link pair");
        let unowned_request = reconciliation_request(&unowned_context);
        assert_eq!(
            classify_fixture_reconciliation_state(&unowned_connection, &unowned_request)
                .expect("classify unowned pair"),
            FixtureReconciliationState::ExactHardLinkPairNeedsReview
        );
        assert!(!fixture_attempt_owns_exact_hard_link_pair(
            &unowned_connection,
            &unowned_request,
            "missing-attempt",
        )
        .expect("unowned pair attribution"));

        let owned_connection = memory_connection();
        let owned_context =
            setup_verified_backup_only_state(&owned_connection, b"owned hard-link pair bytes");
        let owned_request = reconciliation_request(&owned_context);
        prepare_fixture_attempt(
            &owned_connection,
            &owned_request,
            "attempt-owned-1",
            &fixture_attempt_capability_proof(),
        )
        .expect("prepare ownership attempt");
        claim_fixture_destination_no_replace(
            &owned_context.source_path,
            &owned_context.destination_path,
        )
        .expect("create owned hard-link pair after durable intent");
        assert!(fixture_attempt_owns_exact_hard_link_pair(
            &owned_connection,
            &owned_request,
            "attempt-owned-1",
        )
        .expect("prepared attempt should attribute exact pair"));
        let claim_observed = transition_fixture_attempt(
            &owned_connection,
            "attempt-owned-1",
            FixtureAttemptState::DestinationClaimObserved,
        )
        .expect("record claim observation");
        assert_eq!(
            claim_observed.state,
            FixtureAttemptState::DestinationClaimObserved
        );
        assert!(fixture_attempt_owns_exact_hard_link_pair(
            &owned_connection,
            &owned_request,
            "attempt-owned-1",
        )
        .expect("claim-observed attempt should retain attribution"));
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
        assert!(!commands_source.contains("prepare_fixture_attempt"));
        assert!(!commands_source.contains("transition_fixture_attempt"));
        assert!(!commands_source.contains("fixture_attempt_owns_exact_hard_link_pair"));
        assert!(!commands_source.contains("fixture_apply_plan_attempts"));
        assert!(!commands_source.contains("fixture_move_transaction"));

        let core_source = include_str!("mod.rs");
        assert!(core_source.contains("#[cfg(test)]\npub mod apply_plan_fixture_transaction_prototype;"));
        let transaction_source = include_str!("apply_plan_fixture_transaction_prototype.rs");
        assert!(transaction_source.starts_with("#![cfg(test)]"));
    }
}
