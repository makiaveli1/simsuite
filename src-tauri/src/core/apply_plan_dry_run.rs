use std::collections::HashMap;

use rusqlite::Connection;

use crate::{
    core::{apply_plan_persistence, apply_plan_validation},
    error::{AppError, AppResult},
    models::{
        ApplyPlanConflictStatus, ApplyPlanDryRunActionPreview, ApplyPlanDryRunItem,
        ApplyPlanDryRunItemStatus, ApplyPlanDryRunPreview, ApplyPlanDryRunPreviewStatus,
        ApplyPlanDryRunSummary, ApplyPlanValidationItem, ApplyPlanValidationStatus,
        LibrarySettings, PersistedApplyPlan, PersistedApplyPlanBlocker, PersistedApplyPlanItem,
        PersistedApplyPlanItemStatus, PersistedApplyPlanStatus, PreviewApplyPlanDryRunRequest,
        PreviewApplyPlanValidationRequest,
    },
};

const VALIDATION_STEP: &str = "Validation proof required";
const BACKUP_STEP: &str = "Backup required";
const RESTORE_MAP_STEP: &str = "Restore map required";
const RESULT_LOG_STEP: &str = "Result log required";
const CONFIRMATION_STEP: &str = "Explicit confirmation required";
const EXECUTOR_STEP: &str = "Apply executor proof required";

pub fn preview_apply_plan_dry_run(
    connection: &Connection,
    settings: &LibrarySettings,
    request: PreviewApplyPlanDryRunRequest,
) -> AppResult<ApplyPlanDryRunPreview> {
    let plan =
        apply_plan_persistence::get_apply_plan(connection, request.plan_id)?.ok_or_else(|| {
            AppError::Message(format!("Saved ApplyPlan {} was not found", request.plan_id))
        })?;

    let validation = apply_plan_validation::preview_apply_plan_validation(
        connection,
        settings,
        PreviewApplyPlanValidationRequest {
            plan_id: request.plan_id,
        },
    )?;
    let validation_by_item_id: HashMap<i64, ApplyPlanValidationItem> = validation
        .items
        .into_iter()
        .map(|item| (item.item_id, item))
        .collect();

    let mut caveats = validation.caveats;
    push_unique(
        &mut caveats,
        "No files changed. This dry-run preview is read-only.",
    );
    push_unique(
        &mut caveats,
        "Dry-run preview is not Apply or confirmation; Apply is not ready yet.",
    );
    push_unique(
        &mut caveats,
        "Restore is not ready yet. Future Apply still requires backup, restore map, result log, confirmation, and executor proof.",
    );
    if plan.status == PersistedApplyPlanStatus::Cancelled {
        push_unique(
            &mut caveats,
            "Cancelled draft records cannot become future Apply candidates.",
        );
    }
    if plan.items.is_empty() {
        push_unique(&mut caveats, "This saved draft has no items to dry-run.");
    }

    let items: Vec<ApplyPlanDryRunItem> = plan
        .items
        .iter()
        .map(|item| {
            let validation_item = validation_by_item_id.get(&item.id);
            classify_item(&plan, item, validation_item)
        })
        .collect();
    let summary = summarize_items(&items);
    let status = if plan.status == PersistedApplyPlanStatus::Cancelled
        || plan.items.is_empty()
        || summary.blocked_items > 0
        || summary.skipped_items > 0
        || summary.review_only_items > 0
        || summary.conflict_items > 0
        || summary.backup_required_items > 0
    {
        ApplyPlanDryRunPreviewStatus::Blocked
    } else {
        ApplyPlanDryRunPreviewStatus::PreviewOnly
    };

    Ok(ApplyPlanDryRunPreview {
        plan_id: plan.id,
        status,
        can_proceed_to_apply: false,
        can_proceed_to_confirmation: false,
        checked_at: validation.checked_at,
        summary,
        caveats,
        items,
    })
}

fn classify_item(
    plan: &PersistedApplyPlan,
    item: &PersistedApplyPlanItem,
    validation_item: Option<&ApplyPlanValidationItem>,
) -> ApplyPlanDryRunItem {
    let mut reasons = validation_item
        .map(|validation| validation.reasons.clone())
        .unwrap_or_else(|| {
            vec![
                "SimSuite has limited information here; validation did not return this saved item."
                    .to_owned(),
            ]
        });
    let blockers = blockers_for_item(&item.blockers);
    let mut required_before_apply = validation_item
        .map(|validation| validation.required_next_steps.clone())
        .unwrap_or_default();
    push_required_steps(&mut required_before_apply);

    let (dry_run_status, action_preview, dry_run_reason) =
        classify_status(plan, item, validation_item);
    push_unique(&mut reasons, dry_run_reason);

    ApplyPlanDryRunItem {
        item_id: item.id,
        file_id: item.file_id,
        file_name: item.file_name.clone(),
        dry_run_status,
        action_preview,
        source_path: optional_trimmed(&item.current_path),
        destination_path: item.destination_path.as_deref().and_then(optional_trimmed),
        reasons,
        blockers,
        required_before_apply,
        can_apply: false,
    }
}

fn classify_status(
    plan: &PersistedApplyPlan,
    item: &PersistedApplyPlanItem,
    validation_item: Option<&ApplyPlanValidationItem>,
) -> (
    ApplyPlanDryRunItemStatus,
    ApplyPlanDryRunActionPreview,
    &'static str,
) {
    if plan.status == PersistedApplyPlanStatus::Cancelled {
        return (
            ApplyPlanDryRunItemStatus::Blocked,
            ApplyPlanDryRunActionPreview::NoAction,
            "Cancelled draft records are blocked before any future dry-run action.",
        );
    }

    if item.blocked
        || item.item_status == PersistedApplyPlanItemStatus::Blocked
        || validation_item
            .map(|validation| validation.validation_status == ApplyPlanValidationStatus::Blocked)
            .unwrap_or(false)
    {
        return (
            ApplyPlanDryRunItemStatus::Blocked,
            ApplyPlanDryRunActionPreview::NoAction,
            "Saved blockers remain blocked in dry-run.",
        );
    }

    if item.review_only
        || item.item_status == PersistedApplyPlanItemStatus::ReviewOnly
        || validation_item
            .map(|validation| {
                matches!(
                    validation.validation_status,
                    ApplyPlanValidationStatus::ReviewOnlyBlocked
                        | ApplyPlanValidationStatus::DuplicateReviewBlocked
                )
            })
            .unwrap_or(false)
    {
        return (
            ApplyPlanDryRunItemStatus::WouldRequireReview,
            ApplyPlanDryRunActionPreview::WouldSkip,
            "Manual review is required before this item can be discussed for future Apply.",
        );
    }

    let Some(validation_item) = validation_item else {
        return (
            ApplyPlanDryRunItemStatus::Error,
            ApplyPlanDryRunActionPreview::NoAction,
            "Validation output was incomplete for this saved item.",
        );
    };

    match validation_item.validation_status {
        ApplyPlanValidationStatus::MissingSource
        | ApplyPlanValidationStatus::MissingSourceRoot
        | ApplyPlanValidationStatus::UnsafeSource
        | ApplyPlanValidationStatus::StaleSource => (
            ApplyPlanDryRunItemStatus::WouldSkip,
            ApplyPlanDryRunActionPreview::WouldSkip,
            "The saved source evidence is missing or stale, so dry-run would skip this item.",
        ),
        ApplyPlanValidationStatus::MissingDestinationRoot
        | ApplyPlanValidationStatus::UnsafeDestination
        | ApplyPlanValidationStatus::DestinationExists
        | ApplyPlanValidationStatus::UnsupportedCrossRoot => (
            ApplyPlanDryRunItemStatus::WouldRequireDestinationReview,
            ApplyPlanDryRunActionPreview::WouldSkip,
            "Destination evidence requires review before any future file-changing workflow.",
        ),
        ApplyPlanValidationStatus::BackupRequired => (
            ApplyPlanDryRunItemStatus::WouldRequireBackup,
            ApplyPlanDryRunActionPreview::NoAction,
            "Backup and restore-map proof are required before any future Apply.",
        ),
        ApplyPlanValidationStatus::Error => (
            ApplyPlanDryRunItemStatus::Error,
            ApplyPlanDryRunActionPreview::NoAction,
            "Validation returned an error for this item.",
        ),
        ApplyPlanValidationStatus::NotValidated => (
            ApplyPlanDryRunItemStatus::WouldSkip,
            ApplyPlanDryRunActionPreview::WouldSkip,
            "This item has not been validated for dry-run classification.",
        ),
        ApplyPlanValidationStatus::ValidPreviewOnly => {
            if has_destination_conflict(&validation_item.conflict_status) {
                (
                    ApplyPlanDryRunItemStatus::WouldRequireDestinationReview,
                    ApplyPlanDryRunActionPreview::WouldSkip,
                    "Destination conflict evidence requires review before any future Apply.",
                )
            } else if plan.backup_required && !plan.restore_available {
                (
                    ApplyPlanDryRunItemStatus::WouldRequireBackup,
                    ApplyPlanDryRunActionPreview::NoAction,
                    "No current validation blocker was found, but backup, restore map, result log, confirmation, and executor proof are still required.",
                )
            } else {
                (
                    ApplyPlanDryRunItemStatus::CandidateAfterFutureSafetyGates,
                    ApplyPlanDryRunActionPreview::WouldMoveLater,
                    "This item is only a candidate after future safety gates.",
                )
            }
        }
        ApplyPlanValidationStatus::Blocked
        | ApplyPlanValidationStatus::ReviewOnlyBlocked
        | ApplyPlanValidationStatus::DuplicateReviewBlocked => (
            ApplyPlanDryRunItemStatus::Blocked,
            ApplyPlanDryRunActionPreview::NoAction,
            "Validation blocked this item before future Apply can be discussed.",
        ),
    }
}

fn has_destination_conflict(status: &ApplyPlanConflictStatus) -> bool {
    matches!(
        status,
        ApplyPlanConflictStatus::DestinationExists
            | ApplyPlanConflictStatus::SameNameConflict
            | ApplyPlanConflictStatus::CaseConflict
            | ApplyPlanConflictStatus::FolderMissing
            | ApplyPlanConflictStatus::PermissionUnknown
            | ApplyPlanConflictStatus::PathTooLong
            | ApplyPlanConflictStatus::CrossRootBlocked
            | ApplyPlanConflictStatus::Unsupported
    )
}

fn summarize_items(items: &[ApplyPlanDryRunItem]) -> ApplyPlanDryRunSummary {
    ApplyPlanDryRunSummary {
        total_items: items.len() as i64,
        candidate_items: items
            .iter()
            .filter(|item| {
                item.dry_run_status == ApplyPlanDryRunItemStatus::CandidateAfterFutureSafetyGates
            })
            .count() as i64,
        skipped_items: items
            .iter()
            .filter(|item| {
                matches!(
                    item.dry_run_status,
                    ApplyPlanDryRunItemStatus::Blocked
                        | ApplyPlanDryRunItemStatus::WouldSkip
                        | ApplyPlanDryRunItemStatus::WouldRequireReview
                        | ApplyPlanDryRunItemStatus::WouldRequireDestinationReview
                        | ApplyPlanDryRunItemStatus::Error
                )
            })
            .count() as i64,
        blocked_items: items
            .iter()
            .filter(|item| item.dry_run_status == ApplyPlanDryRunItemStatus::Blocked)
            .count() as i64,
        review_only_items: items
            .iter()
            .filter(|item| item.dry_run_status == ApplyPlanDryRunItemStatus::WouldRequireReview)
            .count() as i64,
        conflict_items: items
            .iter()
            .filter(|item| {
                item.dry_run_status == ApplyPlanDryRunItemStatus::WouldRequireDestinationReview
            })
            .count() as i64,
        backup_required_items: items
            .iter()
            .filter(|item| item.dry_run_status == ApplyPlanDryRunItemStatus::WouldRequireBackup)
            .count() as i64,
    }
}

fn blockers_for_item(blockers: &[PersistedApplyPlanBlocker]) -> Vec<String> {
    blockers
        .iter()
        .map(|blocker| format!("{}: {}", blocker.reason_code, blocker.message))
        .collect()
}

fn push_required_steps(steps: &mut Vec<String>) {
    for step in [
        VALIDATION_STEP,
        BACKUP_STEP,
        RESTORE_MAP_STEP,
        RESULT_LOG_STEP,
        CONFIRMATION_STEP,
        EXECUTOR_STEP,
    ] {
        push_unique(steps, step);
    }
}

fn optional_trimmed(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use rusqlite::{params, Connection};
    use tempfile::tempdir;

    use super::*;
    use crate::{
        core::apply_plan_persistence::save_apply_plan_preview,
        database,
        models::{
            SaveApplyPlanPreviewRequest, StagingPlan, StagingPlanActionKind, StagingPlanBucket,
            StagingPlanConfidenceLabel, StagingPlanCurrentRoot, StagingPlanEvidenceLevel,
            StagingPlanItem, StagingPlanSource, StagingPlanStatus,
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

    fn file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sample.package")
            .to_owned()
    }

    fn insert_file(connection: &Connection, file_id: i64, path: &str, source_location: &str) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES (?1, ?2, ?3, 'package', 1, '2026-01-01', ?4, 'CAS', 'Hair', 0.95, '[]', '[]', ?5, 1)",
                params![file_id, path, file_name(path), format!("h{file_id}"), source_location],
            )
            .expect("insert file");
    }

    fn candidate_item(
        file_id: Option<i64>,
        current_path: String,
        destination_path: Option<String>,
    ) -> StagingPlanItem {
        StagingPlanItem {
            id: format!("item-{}", file_id.unwrap_or(0)),
            file_id,
            file_name: if current_path.is_empty() {
                "missing.package".to_owned()
            } else {
                file_name(&current_path)
            },
            current_path: if current_path.is_empty() {
                None
            } else {
                Some(current_path)
            },
            suggested_destination_path: destination_path,
            action_kind: StagingPlanActionKind::SuggestMove,
            evidence_level: StagingPlanEvidenceLevel::Deterministic,
            reason: "Deterministic package category evidence.".to_owned(),
            caveats: vec![],
            source_signals: vec!["kind: CAS".to_owned()],
            blocked_reasons: vec![],
            bucket: StagingPlanBucket::Cas,
            confidence_label: StagingPlanConfidenceLabel::Deterministic,
            current_root: StagingPlanCurrentRoot::Mods,
            would_touch_files: false,
        }
    }

    fn review_only_item(file_id: Option<i64>, current_path: String) -> StagingPlanItem {
        StagingPlanItem {
            evidence_level: StagingPlanEvidenceLevel::ReviewOnly,
            confidence_label: StagingPlanConfidenceLabel::ReviewOnly,
            action_kind: StagingPlanActionKind::SuggestReview,
            bucket: StagingPlanBucket::NeedsReview,
            suggested_destination_path: None,
            ..candidate_item(file_id, current_path, None)
        }
    }

    fn blocked_item(file_id: Option<i64>, current_path: String, reason: &str) -> StagingPlanItem {
        StagingPlanItem {
            blocked_reasons: vec![reason.to_owned()],
            ..candidate_item(file_id, current_path, None)
        }
    }

    fn source_plan(items: Vec<StagingPlanItem>) -> StagingPlan {
        StagingPlan {
            id: "dry-run-source-plan".to_owned(),
            created_at: "2026-05-24T00:00:00Z".to_owned(),
            source: StagingPlanSource::Organize,
            status: StagingPlanStatus::PreviewOnly,
            title: "Dry-run source plan".to_owned(),
            summary: "Preview-only dry-run source.".to_owned(),
            item_count: items.len(),
            would_touch_files: false,
            caveats: vec!["No files changed. Source plan is preview-only.".to_owned()],
            items,
        }
    }

    fn save_plan(connection: &mut Connection, items: Vec<StagingPlanItem>) -> i64 {
        save_apply_plan_preview(
            connection,
            SaveApplyPlanPreviewRequest {
                source_plan: source_plan(items),
                source_plan_kind: Some("sorting_preview".to_owned()),
                source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
        )
        .expect("save plan")
        .plan_id
    }

    fn preview(connection: &Connection, mods_root: &Path, plan_id: i64) -> ApplyPlanDryRunPreview {
        preview_apply_plan_dry_run(
            connection,
            &LibrarySettings {
                mods_path: Some(path_string(mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanDryRunRequest { plan_id },
        )
        .expect("dry-run preview")
    }

    #[test]
    fn dry_run_returns_false_progression_flags_and_item_flags() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 1, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(1),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview(&connection, &mods_root, plan_id);

        assert!(!preview.can_proceed_to_apply);
        assert!(!preview.can_proceed_to_confirmation);
        assert!(preview.items.iter().all(|item| !item.can_apply));
        assert_eq!(
            preview.items[0].dry_run_status,
            ApplyPlanDryRunItemStatus::WouldRequireBackup
        );
        assert!(preview.items[0]
            .required_before_apply
            .iter()
            .any(|step| step == BACKUP_STEP));
        assert_eq!(preview.summary.backup_required_items, 1);
    }

    #[test]
    fn dry_run_keeps_blocked_and_review_only_items_out_of_candidate_status() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let blocked_path = mods_root.join("blocked.package");
        let review_path = mods_root.join("review.package");
        fs::create_dir_all(blocked_path.parent().unwrap()).expect("source parent");
        fs::write(&blocked_path, b"package").expect("blocked file");
        fs::write(&review_path, b"package").expect("review file");

        let mut connection = memory_connection();
        insert_file(&connection, 2, &path_string(&blocked_path), "mods");
        insert_file(&connection, 3, &path_string(&review_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![
                blocked_item(Some(2), path_string(&blocked_path), "parser_warning"),
                review_only_item(Some(3), path_string(&review_path)),
            ],
        );

        let preview = preview(&connection, &mods_root, plan_id);

        assert_eq!(preview.summary.blocked_items, 1);
        assert_eq!(preview.summary.review_only_items, 1);
        assert!(preview.items.iter().all(|item| {
            item.dry_run_status != ApplyPlanDryRunItemStatus::CandidateAfterFutureSafetyGates
        }));
    }

    #[test]
    fn dry_run_skips_missing_and_stale_source_items() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let missing_row_path = mods_root.join("missing-row.package");
        let stale_path = mods_root.join("old.package");
        let new_path = mods_root.join("new.package");
        fs::create_dir_all(stale_path.parent().unwrap()).expect("source parent");
        fs::write(&missing_row_path, b"package").expect("missing row source");
        fs::write(&stale_path, b"package").expect("old file");
        fs::write(&new_path, b"package").expect("new file");

        let mut connection = memory_connection();
        insert_file(&connection, 40, &path_string(&missing_row_path), "mods");
        insert_file(&connection, 4, &path_string(&stale_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![
                candidate_item(
                    Some(40),
                    path_string(&missing_row_path),
                    Some(path_string(
                        &mods_root.join("CAS").join("missing-row.package"),
                    )),
                ),
                candidate_item(
                    Some(4),
                    path_string(&stale_path),
                    Some(path_string(&mods_root.join("CAS").join("old.package"))),
                ),
            ],
        );
        connection
            .execute("DELETE FROM files WHERE id = 40", [])
            .expect("delete indexed row");
        connection
            .execute(
                "UPDATE files SET path = ?1 WHERE id = 4",
                params![path_string(&new_path)],
            )
            .expect("update indexed path");

        let preview = preview(&connection, &mods_root, plan_id);

        assert_eq!(preview.summary.skipped_items, 2);
        assert!(preview
            .items
            .iter()
            .all(|item| item.dry_run_status == ApplyPlanDryRunItemStatus::WouldSkip));
    }

    #[test]
    fn dry_run_requires_destination_review_for_destination_conflicts() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("source.package");
        let destination_path = mods_root.join("CAS").join("source.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");
        fs::write(&destination_path, b"existing").expect("destination file");

        let mut connection = memory_connection();
        insert_file(&connection, 5, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(5),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview(&connection, &mods_root, plan_id);

        assert_eq!(preview.summary.conflict_items, 1);
        assert_eq!(
            preview.items[0].dry_run_status,
            ApplyPlanDryRunItemStatus::WouldRequireDestinationReview
        );
        assert_eq!(
            preview.items[0].action_preview,
            ApplyPlanDryRunActionPreview::WouldSkip
        );
    }

    #[test]
    fn dry_run_does_not_create_result_or_restore_rows() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("persist.package");
        let destination_path = mods_root.join("CAS").join("persist.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 6, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(6),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let _ = preview(&connection, &mods_root, plan_id);

        let result_rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM apply_plan_results", [], |row| {
                row.get(0)
            })
            .expect("count results");
        let restore_rows: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM apply_plan_restore_entries",
                [],
                |row| row.get(0),
            )
            .expect("count restore entries");
        assert_eq!(result_rows, 0);
        assert_eq!(restore_rows, 0);
    }

    #[test]
    fn dry_run_source_avoids_file_changing_and_fixture_helpers() {
        let source = include_str!("apply_plan_dry_run.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");

        for forbidden in [
            concat!("apply_", "preview_organization"),
            concat!("apply_", "preview_moves"),
            concat!("apply_", "download_item"),
            concat!("commit_", "staging_area"),
            concat!("cleanup_", "staging_areas"),
            concat!("reject_", "download_item"),
            concat!("restore_", "snapshot"),
            concat!("undo_", "applied_item"),
            "apply_plan_backup_prototype",
            "run_fixture_backup_prototype",
            "run_fixture_restore_prototype",
            "move_engine::",
            "record_apply_plan_result_log",
            "record_apply_plan_restore_entry",
            "create_apply_plan_run_log",
            "create_dir",
            "write(",
            "copy(",
            "rename(",
            "remove_file",
            "remove_dir",
            "ready_to_apply",
            "safe_to_move",
            "approved",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "dry-run production code must not contain {forbidden}"
            );
        }
    }
}
