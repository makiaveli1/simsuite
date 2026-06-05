use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::{apply_plan_path_validation, apply_plan_persistence, apply_plan_provenance},
    error::{AppError, AppResult},
    models::{
        ApplyPlanConflictStatus, ApplyPlanValidationItem, ApplyPlanValidationPreview,
        ApplyPlanValidationPreviewStatus, ApplyPlanValidationStatus, ApplyPlanValidationSummary,
        LibrarySettings, PersistedApplyPlan, PersistedApplyPlanBlocker, PersistedApplyPlanItem,
        PersistedApplyPlanItemStatus, PersistedApplyPlanStatus, PreviewApplyPlanValidationRequest,
    },
};

pub fn preview_apply_plan_validation(
    connection: &Connection,
    settings: &LibrarySettings,
    request: PreviewApplyPlanValidationRequest,
) -> AppResult<ApplyPlanValidationPreview> {
    let plan =
        apply_plan_persistence::get_apply_plan(connection, request.plan_id)?.ok_or_else(|| {
            AppError::Message(format!("Saved ApplyPlan {} was not found", request.plan_id))
        })?;

    let mut caveats = plan.caveats.clone();
    push_unique_caveat(
        &mut caveats,
        "No files changed. This validation preview is read-only.",
    );
    push_unique_caveat(
        &mut caveats,
        "Backup/restore is required before any future confirmation; Apply is not ready yet.",
    );

    if plan.status == PersistedApplyPlanStatus::Cancelled {
        push_unique_caveat(
            &mut caveats,
            "Cancelled draft records cannot proceed to future confirmation.",
        );
    }
    if plan.items.is_empty() {
        push_unique_caveat(&mut caveats, "This saved draft has no items to validate.");
    }

    let plan_hash_check = apply_plan_persistence::verify_apply_plan_hash(connection, plan.id)?;
    if !plan_hash_check.is_valid {
        push_unique_caveat(
            &mut caveats,
            &format!("{} ({})", plan_hash_check.message, plan_hash_check.status),
        );
    }
    let backend_generated_provenance = has_backend_generated_provenance(&plan);
    if !backend_generated_provenance {
        push_unique_caveat(
            &mut caveats,
            "Client-supplied or legacy ApplyPlan previews are review/audit-only and cannot proceed to future confirmation. Regenerate from a backend-generated sorting preview.",
        );
    }

    let destination_conflicts =
        apply_plan_path_validation::detect_destination_conflicts(&plan.items);
    let mut items = Vec::with_capacity(plan.items.len());
    for item in &plan.items {
        items.push(validate_item(
            connection,
            settings,
            item,
            destination_conflicts.get(&item.id).cloned(),
        )?);
    }

    let summary = summarize_items(&plan, &items);
    let status = if plan.status == PersistedApplyPlanStatus::Cancelled
        || plan.items.is_empty()
        || !plan_hash_check.is_valid
        || !backend_generated_provenance
        || summary.blocked_items > 0
        || summary.conflict_items > 0
        || summary.backup_blocked_items > 0
    {
        ApplyPlanValidationPreviewStatus::Blocked
    } else {
        ApplyPlanValidationPreviewStatus::ValidPreviewOnly
    };

    Ok(ApplyPlanValidationPreview {
        plan_id: plan.id,
        plan_hash: plan.plan_hash.clone(),
        source_plan_kind: plan.source_plan_kind.clone(),
        status,
        can_proceed_to_confirmation: false,
        checked_at: Utc::now().to_rfc3339(),
        summary,
        caveats,
        items,
    })
}

fn validate_item(
    connection: &Connection,
    settings: &LibrarySettings,
    item: &PersistedApplyPlanItem,
    destination_conflict: Option<ApplyPlanConflictStatus>,
) -> AppResult<ApplyPlanValidationItem> {
    let mut reasons = Vec::new();
    let mut required_next_steps = Vec::new();

    let (validation_status, conflict_status) = if has_duplicate_review_signal_or_blocker(item) {
        reasons.push(
            "Duplicate-related evidence still requires manual review before future confirmation."
                .to_owned(),
        );
        required_next_steps.push("Review duplicate evidence before future validation.".to_owned());
        (
            ApplyPlanValidationStatus::DuplicateReviewBlocked,
            ApplyPlanConflictStatus::Unsupported,
        )
    } else if item.blocked
        || item.item_status == PersistedApplyPlanItemStatus::Blocked
        || has_non_review_blocker(&item.blockers)
    {
        for blocker in &item.blockers {
            reasons.push(format!("{}: {}", blocker.reason_code, blocker.message));
        }
        if reasons.is_empty() {
            reasons.push("Saved draft item is blocked.".to_owned());
        }
        required_next_steps.push("Resolve the blocker before future validation.".to_owned());
        (
            ApplyPlanValidationStatus::Blocked,
            ApplyPlanConflictStatus::NotChecked,
        )
    } else if item.review_only || item.item_status == PersistedApplyPlanItemStatus::ReviewOnly {
        reasons.push("Saved draft item is review-only.".to_owned());
        for blocker in &item.blockers {
            reasons.push(format!("{}: {}", blocker.reason_code, blocker.message));
        }
        required_next_steps.push("Review this item manually before future validation.".to_owned());
        (
            ApplyPlanValidationStatus::ReviewOnlyBlocked,
            ApplyPlanConflictStatus::NotChecked,
        )
    } else if !item.action_kind.eq_ignore_ascii_case("suggest_move") {
        reasons.push(
            "Only direct move suggestions are eligible for future Apply validation; grouped, review, leave-in-place, and no-action rows stay review-only."
                .to_owned(),
        );
        required_next_steps.push(
            "Regenerate the preview with direct move suggestions before future validation."
                .to_owned(),
        );
        (
            ApplyPlanValidationStatus::ReviewOnlyBlocked,
            ApplyPlanConflictStatus::Unsupported,
        )
    } else if item.evidence_level.eq_ignore_ascii_case("heuristic")
        || item
            .confidence_label
            .as_deref()
            .is_some_and(|label| label.eq_ignore_ascii_case("heuristic"))
    {
        reasons.push(
            "Heuristic-only evidence is review-only and cannot proceed to future confirmation."
                .to_owned(),
        );
        required_next_steps.push(
            "Use deterministic or evidence-backed signals before future validation.".to_owned(),
        );
        (
            ApplyPlanValidationStatus::ReviewOnlyBlocked,
            ApplyPlanConflictStatus::Unsupported,
        )
    } else if item.file_id.is_none() {
        if item.current_path.trim().is_empty() {
            reasons.push("No saved source path or Library file id is available.".to_owned());
            required_next_steps
                .push("Regenerate the preview from current Library data.".to_owned());
            (
                ApplyPlanValidationStatus::MissingSource,
                ApplyPlanConflictStatus::SourceMissing,
            )
        } else {
            reasons.push(
                "No Library file id is attached; SimSuite will not infer identity by path."
                    .to_owned(),
            );
            required_next_steps
                .push("Regenerate the preview from current Library data.".to_owned());
            (
                ApplyPlanValidationStatus::ReviewOnlyBlocked,
                ApplyPlanConflictStatus::NotChecked,
            )
        }
    } else {
        validate_item_paths(
            connection,
            settings,
            item,
            destination_conflict,
            &mut reasons,
            &mut required_next_steps,
        )?
    };

    let blocked = item.blocked
        || item.item_status == PersistedApplyPlanItemStatus::Blocked
        || validation_status != ApplyPlanValidationStatus::ValidPreviewOnly;
    let review_only = item.review_only
        || item.item_status == PersistedApplyPlanItemStatus::ReviewOnly
        || validation_status == ApplyPlanValidationStatus::ReviewOnlyBlocked;

    Ok(ApplyPlanValidationItem {
        item_id: item.id,
        file_id: item.file_id,
        file_name: item.file_name.clone(),
        validation_status,
        conflict_status,
        blocked,
        review_only,
        can_apply_later: false,
        reasons,
        required_next_steps,
    })
}

fn validate_item_paths(
    connection: &Connection,
    settings: &LibrarySettings,
    item: &PersistedApplyPlanItem,
    destination_conflict: Option<ApplyPlanConflictStatus>,
    reasons: &mut Vec<String>,
    required_next_steps: &mut Vec<String>,
) -> AppResult<(ApplyPlanValidationStatus, ApplyPlanConflictStatus)> {
    let file_id = item.file_id.expect("file_id should be present");
    let current_file = load_current_file(connection, file_id)?;
    let Some(current_file) = current_file else {
        reasons.push("Library no longer has a row for this saved file id.".to_owned());
        required_next_steps.push("Regenerate the preview from current Library data.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::MissingSource,
            ApplyPlanConflictStatus::SourceMissing,
        ));
    };

    if apply_plan_path_validation::normalized_path_key(&current_file.path)
        != apply_plan_path_validation::normalized_path_key(&item.current_path)
    {
        reasons.push("Saved source path no longer matches the current Library index.".to_owned());
        required_next_steps
            .push("Review the current Library location before future validation.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::StaleSource,
            ApplyPlanConflictStatus::NotChecked,
        ));
    }

    if !is_supported_apply_plan_file_type(&current_file.extension, &current_file.kind) {
        reasons.push(format!(
            "Unsupported file type for future Apply validation: extension `{}`, kind `{}`.",
            current_file.extension, current_file.kind
        ));
        required_next_steps.push(
            "Regenerate the preview with supported Sims package/script/tray files only.".to_owned(),
        );
        return Ok((
            ApplyPlanValidationStatus::ReviewOnlyBlocked,
            ApplyPlanConflictStatus::Unsupported,
        ));
    }

    let source_root =
        apply_plan_path_validation::configured_root(settings, current_file.source_location.trim());
    let source_outcome =
        apply_plan_path_validation::validate_source_under_root(&current_file.path, source_root);
    if source_outcome.validation_status != ApplyPlanValidationStatus::ValidPreviewOnly {
        reasons.push(source_outcome.reason);
        required_next_steps.push(source_outcome.required_next_step);
        return Ok((
            source_outcome.validation_status,
            source_outcome.conflict_status,
        ));
    }

    let Some(destination_path) = item
        .destination_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        reasons.push("No saved destination preview path is available.".to_owned());
        required_next_steps
            .push("Regenerate the preview with a destination suggestion.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::ReviewOnlyBlocked,
            ApplyPlanConflictStatus::NotChecked,
        ));
    };

    let Some(destination_root_name) = item
        .destination_root
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        reasons.push("No destination root was saved for this preview item.".to_owned());
        required_next_steps
            .push("Regenerate the preview from current Library settings.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::MissingDestinationRoot,
            ApplyPlanConflictStatus::Unsupported,
        ));
    };

    if let Some(conflict_status) = destination_conflict {
        match conflict_status {
            ApplyPlanConflictStatus::CaseConflict => {
                reasons.push(
                    "Another item in this plan targets the same destination under Windows case-insensitive path rules.".to_owned(),
                );
            }
            ApplyPlanConflictStatus::SameNameConflict => {
                reasons.push(
                    "Another item in this plan targets the same canonical destination path."
                        .to_owned(),
                );
            }
            _ => {
                reasons
                    .push("Another item in this plan conflicts with this destination.".to_owned());
            }
        }
        required_next_steps.push(
            "Regenerate or edit the preview so each destination is unique before future validation."
                .to_owned(),
        );
        return Ok((ApplyPlanValidationStatus::Blocked, conflict_status));
    }

    let destination_outcome = apply_plan_path_validation::validate_destination(
        apply_plan_path_validation::DestinationValidationInput {
            settings,
            destination_path,
            destination_root_name,
            indexed_source_root_name: current_file.source_location.trim(),
            saved_current_root_name: item.current_root.trim(),
        },
    );
    reasons.push(destination_outcome.reason);
    required_next_steps.push(destination_outcome.required_next_step);
    Ok((
        destination_outcome.validation_status,
        destination_outcome.conflict_status,
    ))
}

fn summarize_items(
    plan: &PersistedApplyPlan,
    items: &[ApplyPlanValidationItem],
) -> ApplyPlanValidationSummary {
    ApplyPlanValidationSummary {
        total_items: items.len() as i64,
        blocked_items: items
            .iter()
            .filter(|item| item.validation_status != ApplyPlanValidationStatus::ValidPreviewOnly)
            .count() as i64,
        review_only_items: items
            .iter()
            .filter(|item| {
                item.review_only
                    || item.validation_status == ApplyPlanValidationStatus::ReviewOnlyBlocked
            })
            .count() as i64,
        conflict_items: items
            .iter()
            .filter(|item| {
                !matches!(
                    item.conflict_status,
                    ApplyPlanConflictStatus::None | ApplyPlanConflictStatus::NotChecked
                )
            })
            .count() as i64,
        stale_items: items
            .iter()
            .filter(|item| item.validation_status == ApplyPlanValidationStatus::StaleSource)
            .count() as i64,
        missing_source_items: items
            .iter()
            .filter(|item| {
                matches!(
                    item.validation_status,
                    ApplyPlanValidationStatus::MissingSource
                        | ApplyPlanValidationStatus::MissingSourceRoot
                        | ApplyPlanValidationStatus::UnsafeSource
                )
            })
            .count() as i64,
        destination_conflict_items: items
            .iter()
            .filter(|item| {
                item.validation_status == ApplyPlanValidationStatus::DestinationExists
                    || item.conflict_status == ApplyPlanConflictStatus::DestinationExists
            })
            .count() as i64,
        backup_blocked_items: if plan.backup_required && !plan.restore_available {
            items.len() as i64
        } else {
            0
        },
    }
}

fn load_current_file(connection: &Connection, file_id: i64) -> AppResult<Option<CurrentFileRow>> {
    connection
        .query_row(
            "SELECT path, source_location, extension, kind FROM files WHERE id = ?1",
            params![file_id],
            |row| {
                Ok(CurrentFileRow {
                    path: row.get(0)?,
                    source_location: row.get(1)?,
                    extension: row.get(2)?,
                    kind: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
}

fn has_duplicate_review_signal_or_blocker(item: &PersistedApplyPlanItem) -> bool {
    item.blockers.iter().any(|blocker| {
        contains_duplicate_text(&blocker.reason_code)
            || contains_duplicate_text(&blocker.message)
            || contains_duplicate_text(&blocker.blocker_kind)
    }) || item.signals.iter().any(|signal| {
        contains_duplicate_text(&signal.signal_kind)
            || contains_duplicate_text(&signal.signal_label)
            || signal
                .signal_value
                .as_deref()
                .is_some_and(contains_duplicate_text)
    })
}

fn has_backend_generated_provenance(plan: &PersistedApplyPlan) -> bool {
    let expected = apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND;
    let Some(preview_snapshot_hash) = plan.preview_snapshot_hash.as_deref() else {
        return false;
    };

    plan.preview_snapshot_id.is_some()
        && plan.source_plan_kind == expected
        && plan
            .plan_provenance
            .get("sourceKind")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|source_kind| source_kind == expected)
        && plan
            .plan_provenance
            .get("previewSnapshotHash")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|provenance_hash| provenance_hash == preview_snapshot_hash)
}

fn has_non_review_blocker(blockers: &[PersistedApplyPlanBlocker]) -> bool {
    blockers.iter().any(|blocker| {
        !blocker.blocker_kind.eq_ignore_ascii_case("review_only")
            && !blocker
                .reason_code
                .eq_ignore_ascii_case("review_only_evidence")
    })
}

fn contains_duplicate_text(value: &str) -> bool {
    value.to_ascii_lowercase().contains("duplicate")
}

fn push_unique_caveat(caveats: &mut Vec<String>, caveat: &str) {
    if !caveats.iter().any(|existing| existing == caveat) {
        caveats.push(caveat.to_owned());
    }
}

fn is_supported_apply_plan_file_type(extension: &str, _kind: &str) -> bool {
    let extension = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "package"
            | "ts4script"
            | "trayitem"
            | "blueprint"
            | "bpi"
            | "hhi"
            | "householdbinary"
            | "sgi"
            | "rmi"
            | "room"
    )
}

struct CurrentFileRow {
    path: String,
    source_location: String,
    extension: String,
    kind: String,
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::*;
    use crate::{
        core::apply_plan_persistence::{delete_draft_apply_plan, save_apply_plan_preview},
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

    fn insert_file(connection: &Connection, file_id: i64, path: &str, source_location: &str) {
        insert_file_with_extension(connection, file_id, path, source_location, "package", "CAS");
    }

    fn insert_file_with_extension(
        connection: &Connection,
        file_id: i64,
        path: &str,
        source_location: &str,
        extension: &str,
        kind: &str,
    ) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES (?1, ?2, ?3, ?4, 1, '2026-01-01', ?5, ?6, 'Hair', 0.95, '[]', '[]', ?7, 1)",
                params![file_id, path, file_name(path), extension, format!("h{file_id}"), kind, source_location],
            )
            .expect("insert file");
    }

    fn file_name(path: &str) -> String {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sample.package")
            .to_owned()
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
            id: "validation-source-plan".to_owned(),
            created_at: "2026-05-15T00:00:00Z".to_owned(),
            source: StagingPlanSource::Organize,
            status: StagingPlanStatus::PreviewOnly,
            title: "Validation source plan".to_owned(),
            summary: "Preview-only validation source.".to_owned(),
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

    #[test]
    fn validation_preview_reports_valid_items_but_blocks_confirmation_for_backup() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
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

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(preview.status, ApplyPlanValidationPreviewStatus::Blocked);
        assert!(!preview.can_proceed_to_confirmation);
        assert_eq!(preview.summary.backup_blocked_items, 1);
        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ValidPreviewOnly
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::None
        );
        assert!(!preview.items[0].can_apply_later);
        assert!(preview.caveats.iter().any(|caveat| caveat
            .contains("Client-supplied or legacy ApplyPlan previews are review/audit-only")));
    }

    #[test]
    fn canonical_destination_validation_rejects_duplicate_destinations() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path_a = mods_root.join("a.package");
        let source_path_b = mods_root.join("b.package");
        let destination_path = mods_root.join("CAS").join("shared.package");
        fs::create_dir_all(source_path_a.parent().unwrap()).expect("source parent");
        fs::write(&source_path_a, b"package-a").expect("source file a");
        fs::write(&source_path_b, b"package-b").expect("source file b");

        let mut connection = memory_connection();
        insert_file(&connection, 31, &path_string(&source_path_a), "mods");
        insert_file(&connection, 32, &path_string(&source_path_b), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![
                candidate_item(
                    Some(31),
                    path_string(&source_path_a),
                    Some(path_string(&destination_path)),
                ),
                candidate_item(
                    Some(32),
                    path_string(&source_path_b),
                    Some(path_string(&destination_path)),
                ),
            ],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(preview.summary.conflict_items, 2);
        assert!(preview
            .items
            .iter()
            .all(|item| item.conflict_status == ApplyPlanConflictStatus::SameNameConflict));
        assert!(preview.items.iter().all(|item| item.blocked));
    }

    #[test]
    fn canonical_destination_validation_rejects_case_only_conflicts() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path_a = mods_root.join("a.package");
        let source_path_b = mods_root.join("b.package");
        let destination_path_a = mods_root.join("CAS").join("Shared.package");
        let destination_path_b = mods_root.join("CAS").join("shared.package");
        fs::create_dir_all(source_path_a.parent().unwrap()).expect("source parent");
        fs::write(&source_path_a, b"package-a").expect("source file a");
        fs::write(&source_path_b, b"package-b").expect("source file b");

        let mut connection = memory_connection();
        insert_file(&connection, 41, &path_string(&source_path_a), "mods");
        insert_file(&connection, 42, &path_string(&source_path_b), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![
                candidate_item(
                    Some(41),
                    path_string(&source_path_a),
                    Some(path_string(&destination_path_a)),
                ),
                candidate_item(
                    Some(42),
                    path_string(&source_path_b),
                    Some(path_string(&destination_path_b)),
                ),
            ],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(preview.summary.conflict_items, 2);
        assert!(preview
            .items
            .iter()
            .all(|item| item.conflict_status == ApplyPlanConflictStatus::CaseConflict));
        assert!(preview.items.iter().all(|item| item.blocked));
    }

    #[test]
    fn canonical_destination_validation_rejects_missing_destination_parent() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        let destination_path = mods_root.join("MissingParent").join("hair.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 61, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(61),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::FolderMissing
        );
    }

    #[test]
    fn canonical_destination_validation_rejects_source_outside_configured_root() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let outside_root = temp.path().join("Elsewhere");
        let source_path = outside_root.join("hair.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 62, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(62),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeSource
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::Unsupported
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_destination_validation_rejects_source_symlink_escape() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let outside_root = temp.path().join("Elsewhere");
        let outside_source = outside_root.join("linked.package");
        let symlink_source = mods_root.join("linked.package");
        let destination_path = mods_root.join("CAS").join("linked.package");
        fs::create_dir_all(&mods_root).expect("mods root");
        fs::create_dir_all(outside_source.parent().unwrap()).expect("outside parent");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&outside_source, b"package").expect("outside source file");
        std::os::unix::fs::symlink(&outside_source, &symlink_source).expect("source symlink");

        let mut connection = memory_connection();
        insert_file(&connection, 65, &path_string(&symlink_source), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(65),
                path_string(&symlink_source),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeSource
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("symlink")));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_destination_validation_rejects_destination_symlink_escape() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let outside_root = temp.path().join("Elsewhere");
        let source_path = mods_root.join("hair.package");
        let linked_parent = mods_root.join("LinkedCas");
        let destination_path = linked_parent.join("hair.package");
        fs::create_dir_all(&mods_root).expect("mods root");
        fs::create_dir_all(&outside_root).expect("outside root");
        fs::write(&source_path, b"package").expect("source file");
        std::os::unix::fs::symlink(&outside_root, &linked_parent).expect("destination symlink");

        let mut connection = memory_connection();
        insert_file(&connection, 66, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(66),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("symlink")));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_destination_validation_rejects_dangling_destination_symlink_leaf() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        let missing_target = temp.path().join("missing-target.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");
        std::os::unix::fs::symlink(&missing_target, &destination_path)
            .expect("dangling destination symlink");

        let mut connection = memory_connection();
        insert_file(&connection, 67, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(67),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("symlink")));
    }

    #[cfg(unix)]
    #[test]
    fn canonical_destination_validation_rejects_configured_root_symlink() {
        let temp = tempdir().expect("tempdir");
        let real_mods_root = temp.path().join("RealMods");
        let linked_mods_root = temp.path().join("ModsLink");
        let source_path = linked_mods_root.join("hair.package");
        let destination_path = linked_mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(real_mods_root.join("CAS")).expect("real destination parent");
        fs::write(real_mods_root.join("hair.package"), b"package").expect("source file");
        std::os::unix::fs::symlink(&real_mods_root, &linked_mods_root)
            .expect("configured root symlink");

        let mut connection = memory_connection();
        insert_file(&connection, 68, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(68),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&linked_mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeSource
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("root") && reason.contains("symlink")));
    }

    #[test]
    fn canonical_destination_validation_ignores_ineligible_rows_for_destination_conflicts() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let first_source_path = mods_root.join("hair.package");
        let heuristic_source_path = mods_root.join("heuristic.package");
        let group_source_path = mods_root.join("group.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&first_source_path, b"package").expect("first source file");
        fs::write(&heuristic_source_path, b"package").expect("heuristic source file");
        fs::write(&group_source_path, b"package").expect("group source file");

        let mut heuristic_item = candidate_item(
            Some(69),
            path_string(&heuristic_source_path),
            Some(path_string(&destination_path)),
        );
        heuristic_item.evidence_level = StagingPlanEvidenceLevel::Heuristic;
        heuristic_item.confidence_label = StagingPlanConfidenceLabel::Heuristic;

        let mut group_item = candidate_item(
            Some(70),
            path_string(&group_source_path),
            Some(path_string(&destination_path)),
        );
        group_item.action_kind = StagingPlanActionKind::SuggestGroup;

        let mut connection = memory_connection();
        insert_file(&connection, 1, &path_string(&first_source_path), "mods");
        insert_file(
            &connection,
            69,
            &path_string(&heuristic_source_path),
            "mods",
        );
        insert_file(&connection, 70, &path_string(&group_source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![
                candidate_item(
                    Some(1),
                    path_string(&first_source_path),
                    Some(path_string(&destination_path)),
                ),
                heuristic_item,
                group_item,
            ],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ValidPreviewOnly
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::None
        );
        assert_eq!(
            preview.items[1].validation_status,
            ApplyPlanValidationStatus::ReviewOnlyBlocked
        );
        assert_eq!(
            preview.items[2].validation_status,
            ApplyPlanValidationStatus::ReviewOnlyBlocked
        );
    }

    #[test]
    fn canonical_destination_validation_blocks_unsupported_file_types() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("notes.txt");
        let destination_path = mods_root.join("CAS").join("notes.txt");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"notes").expect("source file");

        let mut connection = memory_connection();
        insert_file_with_extension(
            &connection,
            71,
            &path_string(&source_path),
            "mods",
            "txt",
            "Document",
        );
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(71),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ReviewOnlyBlocked
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::Unsupported
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("Unsupported file type")));
    }

    #[test]
    fn canonical_destination_validation_blocks_unsupported_group_actions() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("grouped.package");
        let destination_path = mods_root.join("CAS").join("grouped.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 63, &path_string(&source_path), "mods");
        let mut item = candidate_item(
            Some(63),
            path_string(&source_path),
            Some(path_string(&destination_path)),
        );
        item.action_kind = StagingPlanActionKind::SuggestGroup;
        let plan_id = save_plan(&mut connection, vec![item]);

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ReviewOnlyBlocked
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("Only direct move suggestions")));
    }

    #[test]
    fn canonical_destination_validation_blocks_heuristic_only_items() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("heuristic.package");
        let destination_path = mods_root.join("CAS").join("heuristic.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 64, &path_string(&source_path), "mods");
        let mut item = candidate_item(
            Some(64),
            path_string(&source_path),
            Some(path_string(&destination_path)),
        );
        item.evidence_level = StagingPlanEvidenceLevel::Heuristic;
        item.confidence_label = StagingPlanConfidenceLabel::Heuristic;
        let plan_id = save_plan(&mut connection, vec![item]);

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ReviewOnlyBlocked
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("Heuristic-only")));
    }

    #[test]
    fn canonical_destination_validation_includes_plan_hash_and_source_kind() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 51, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(51),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(preview.source_plan_kind, "client_supplied_preview");
        assert_eq!(preview.plan_hash.as_deref().map(str::len), Some(64));
        assert!(!preview.can_proceed_to_confirmation);
    }

    #[test]
    fn windows_style_parent_destination_is_rejected_before_prefix_match() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 21, &path_string(&source_path), "mods");
        let destination_path = format!("{}\\..\\Elsewhere\\hair.package", path_string(&mods_root));
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(21),
                path_string(&source_path),
                Some(destination_path),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );
    }

    #[test]
    fn tampered_plan_hash_blocks_validation_preview() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hair.package");
        let destination_path = mods_root.join("CAS").join("hair.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 10, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(10),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );
        connection
            .execute(
                "UPDATE apply_plans SET context_trail_json = '[{\"sourceSystem\":\"test\",\"signalKind\":\"tamper\",\"label\":\"Tampered\",\"value\":null,\"strength\":\"evidence\"}]' WHERE id = ?1",
                [plan_id],
            )
            .expect("tamper context");

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(preview.status, ApplyPlanValidationPreviewStatus::Blocked);
        assert!(!preview.can_proceed_to_confirmation);
        assert!(preview
            .caveats
            .iter()
            .any(|caveat| caveat.contains("Saved ApplyPlan hash/provenance no longer matches")));
    }

    #[test]
    fn missing_or_empty_plan_provenance_blocks_validation_preview() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let connection = memory_connection();

        for (plan_hash, plan_provenance_json, expected) in [
            (None, "{}", "Saved ApplyPlan hash/provenance is missing"),
            (
                Some("0".repeat(64)),
                "{}",
                "Saved ApplyPlan hash/provenance is missing",
            ),
        ] {
            connection
                .execute(
                    "INSERT INTO apply_plans (
                        source_plan_kind, title, summary, status, total_items,
                        caveats_json, context_trail_json, plan_hash, plan_provenance_json
                    ) VALUES ('client_supplied_preview', 'legacy', 'legacy', 'preview_only_source', 0, '[]', '[]', ?1, ?2)",
                    params![plan_hash, plan_provenance_json],
                )
                .expect("insert legacy/malformed provenance plan");
            let plan_id = connection.last_insert_rowid();

            let preview = preview_apply_plan_validation(
                &connection,
                &LibrarySettings {
                    mods_path: Some(path_string(&mods_root)),
                    ..Default::default()
                },
                PreviewApplyPlanValidationRequest { plan_id },
            )
            .expect("preview");

            assert_eq!(preview.status, ApplyPlanValidationPreviewStatus::Blocked);
            assert!(!preview.can_proceed_to_confirmation);
            assert!(preview
                .caveats
                .iter()
                .any(|caveat| caveat.contains(expected)));
        }
    }

    #[test]
    fn malformed_plan_provenance_fails_validation_closed() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let connection = memory_connection();
        connection
            .execute(
                "INSERT INTO apply_plans (
                    source_plan_kind, title, summary, status, total_items,
                    caveats_json, context_trail_json, plan_hash, plan_provenance_json
                ) VALUES ('client_supplied_preview', 'bad', 'bad', 'preview_only_source', 0, '[]', '[]', ?1, '{')",
                params![Some("0".repeat(64))],
            )
            .expect("insert malformed provenance plan");
        let plan_id = connection.last_insert_rowid();

        let error = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect_err("malformed provenance should fail closed");

        assert!(error.to_string().contains("plan_provenance_json"));
    }

    #[test]
    fn review_only_item_returns_review_only_blocked() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("unknown.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 2, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![review_only_item(Some(2), path_string(&source_path))],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ReviewOnlyBlocked
        );
        assert!(!preview.items[0].can_apply_later);
    }

    #[test]
    fn saved_blockers_make_item_blocked() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("blocked.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 3, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![blocked_item(
                Some(3),
                path_string(&source_path),
                "parser_warning",
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::Blocked
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("parser_warning")));
    }

    #[test]
    fn missing_library_row_returns_missing_source() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("missing-row.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 4, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(4),
                path_string(&source_path),
                Some(path_string(
                    &mods_root.join("CAS").join("missing-row.package"),
                )),
            )],
        );
        connection
            .execute("DELETE FROM files WHERE id = 4", [])
            .expect("delete indexed row");

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::MissingSource
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::SourceMissing
        );
    }

    #[test]
    fn changed_current_path_returns_stale_source() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("old.package");
        let new_path = mods_root.join("new.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");
        fs::write(&new_path, b"package").expect("new file");

        let mut connection = memory_connection();
        insert_file(&connection, 5, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(5),
                path_string(&source_path),
                Some(path_string(&mods_root.join("CAS").join("old.package"))),
            )],
        );
        connection
            .execute(
                "UPDATE files SET path = ?1 WHERE id = 5",
                params![path_string(&new_path)],
            )
            .expect("update indexed path");

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::StaleSource
        );
    }

    #[test]
    fn destination_outside_configured_root_returns_unsafe_destination() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let outside_root = temp.path().join("Elsewhere");
        let source_path = mods_root.join("unsafe.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 6, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(6),
                path_string(&source_path),
                Some(path_string(&outside_root.join("unsafe.package"))),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );
    }

    #[test]
    fn destination_exists_returns_destination_exists() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("source.package");
        let destination_path = mods_root.join("CAS").join("source.package");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");
        fs::write(&destination_path, b"existing").expect("destination file");

        let mut connection = memory_connection();
        insert_file(&connection, 7, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(7),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::DestinationExists
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::DestinationExists
        );
    }

    #[test]
    fn missing_destination_root_returns_missing_destination_root() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("root.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 8, &path_string(&source_path), "mods");
        let mut item = candidate_item(
            Some(8),
            path_string(&source_path),
            Some(path_string(&temp.path().join("Tray").join("root.trayitem"))),
        );
        item.bucket = StagingPlanBucket::Tray;
        let plan_id = save_plan(&mut connection, vec![item]);

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::MissingDestinationRoot
        );
    }

    #[test]
    fn cancelled_and_empty_plans_return_blocked_previews() {
        let mut connection = memory_connection();
        let empty_plan_id = save_plan(&mut connection, vec![]);
        let empty_preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings::default(),
            PreviewApplyPlanValidationRequest {
                plan_id: empty_plan_id,
            },
        )
        .expect("empty preview");
        assert_eq!(
            empty_preview.status,
            ApplyPlanValidationPreviewStatus::Blocked
        );

        let cancelled_plan_id = save_plan(
            &mut connection,
            vec![candidate_item(None, String::new(), None)],
        );
        delete_draft_apply_plan(&mut connection, cancelled_plan_id).expect("cancel");
        let cancelled_preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings::default(),
            PreviewApplyPlanValidationRequest {
                plan_id: cancelled_plan_id,
            },
        )
        .expect("cancelled preview");
        assert_eq!(
            cancelled_preview.status,
            ApplyPlanValidationPreviewStatus::Blocked
        );
        assert!(cancelled_preview
            .caveats
            .iter()
            .any(|caveat| caveat.contains("Cancelled draft")));
    }

    #[test]
    fn validation_preview_does_not_persist_statuses() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("persist.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 9, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(9),
                path_string(&source_path),
                Some(path_string(&mods_root.join("CAS").join("persist.package"))),
            )],
        );

        let _ = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        let changed_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM apply_plan_items
                 WHERE apply_plan_id = ?1
                   AND (validation_status IS NOT NULL OR conflict_status IS NOT NULL)",
                params![plan_id],
                |row| row.get(0),
            )
            .expect("status count");
        assert_eq!(changed_count, 0);
    }

    #[test]
    fn validation_module_production_code_avoids_mutating_file_calls() {
        let production_source = [
            include_str!("apply_plan_validation.rs")
                .split("#[cfg(test)]")
                .next()
                .expect("validation production source"),
            include_str!("apply_plan_path_validation.rs")
                .split("#[cfg(test)]")
                .next()
                .expect("path validation production source"),
        ]
        .join("\n");

        for forbidden in [
            concat!("apply_", "preview_organization"),
            concat!("apply_", "preview_moves"),
            concat!("apply_", "download_item"),
            concat!("commit_", "staging_area"),
            concat!("cleanup_", "staging_areas"),
            concat!("reject_", "download_item"),
            concat!("restore_", "snapshot"),
            concat!("undo_", "applied_item"),
            "move_engine::",
            "create_dir",
            "write(",
            "copy(",
            "rename(",
            "remove_file",
            "remove_dir",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "validation preview production code must not call {forbidden}"
            );
        }
    }
}
