use std::path::{Component, Path};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::apply_plan_persistence,
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

    let mut items = Vec::with_capacity(plan.items.len());
    for item in &plan.items {
        items.push(validate_item(connection, settings, item)?);
    }

    let summary = summarize_items(&plan, &items);
    let status = if plan.status == PersistedApplyPlanStatus::Cancelled
        || plan.items.is_empty()
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

    if normalized_path_key(&current_file.path) != normalized_path_key(&item.current_path) {
        reasons.push("Saved source path no longer matches the current Library index.".to_owned());
        required_next_steps
            .push("Review the current Library location before future validation.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::StaleSource,
            ApplyPlanConflictStatus::NotChecked,
        ));
    }

    match read_only_path_exists(&current_file.path) {
        Ok(true) => {}
        Ok(false) => {
            reasons.push("The indexed source file was not found on disk.".to_owned());
            required_next_steps
                .push("Rescan or regenerate the preview from current Library data.".to_owned());
            return Ok((
                ApplyPlanValidationStatus::MissingSource,
                ApplyPlanConflictStatus::SourceMissing,
            ));
        }
        Err(message) => {
            reasons.push(format!("Source existence could not be checked: {message}"));
            required_next_steps
                .push("Review source file permissions before future validation.".to_owned());
            return Ok((
                ApplyPlanValidationStatus::Error,
                ApplyPlanConflictStatus::PermissionUnknown,
            ));
        }
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

    let Some(destination_root) = configured_root(settings, destination_root_name) else {
        reasons.push(format!(
            "Destination root `{destination_root_name}` is not configured."
        ));
        required_next_steps
            .push("Configure the destination root before future validation.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::MissingDestinationRoot,
            ApplyPlanConflictStatus::Unsupported,
        ));
    };

    if contains_parent_dir_component(destination_path)
        || !path_is_under_root(destination_path, destination_root)
    {
        reasons.push(
            "Destination path cannot be proven under the configured Mods/Tray root.".to_owned(),
        );
        required_next_steps.push("Regenerate the preview with a safe destination path.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::UnsafeDestination,
            ApplyPlanConflictStatus::Unsupported,
        ));
    }

    if !current_file
        .source_location
        .eq_ignore_ascii_case(destination_root_name)
        || (!item.current_root.eq_ignore_ascii_case("unknown")
            && !item
                .current_root
                .eq_ignore_ascii_case(destination_root_name))
    {
        reasons.push("Cross-root movement is not supported by the validation preview.".to_owned());
        required_next_steps
            .push("Keep future validation within one configured Library root.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::UnsupportedCrossRoot,
            ApplyPlanConflictStatus::CrossRootBlocked,
        ));
    }

    match read_only_path_exists(destination_path) {
        Ok(true) => {
            reasons
                .push("A file or folder already exists at the saved destination path.".to_owned());
            required_next_steps
                .push("Resolve the destination conflict before future validation.".to_owned());
            Ok((
                ApplyPlanValidationStatus::DestinationExists,
                ApplyPlanConflictStatus::DestinationExists,
            ))
        }
        Ok(false) => {
            reasons
                .push("No current validation blocker was found for this preview item.".to_owned());
            required_next_steps.push(
                "Backup/restore and confirmation work must exist before this could go further."
                    .to_owned(),
            );
            Ok((
                ApplyPlanValidationStatus::ValidPreviewOnly,
                ApplyPlanConflictStatus::None,
            ))
        }
        Err(message) => {
            reasons.push(format!(
                "Destination conflict could not be checked: {message}"
            ));
            required_next_steps
                .push("Review destination permissions before future validation.".to_owned());
            Ok((
                ApplyPlanValidationStatus::Error,
                ApplyPlanConflictStatus::PermissionUnknown,
            ))
        }
    }
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
            .filter(|item| item.validation_status == ApplyPlanValidationStatus::MissingSource)
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
            "SELECT path, source_location FROM files WHERE id = ?1",
            params![file_id],
            |row| {
                Ok(CurrentFileRow {
                    path: row.get(0)?,
                    source_location: row.get(1)?,
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

fn configured_root<'a>(settings: &'a LibrarySettings, root_name: &str) -> Option<&'a str> {
    match root_name {
        value if value.eq_ignore_ascii_case("mods") => settings.mods_path.as_deref(),
        value if value.eq_ignore_ascii_case("tray") => settings.tray_path.as_deref(),
        _ => None,
    }
}

fn contains_parent_dir_component(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn path_is_under_root(path: &str, root: &str) -> bool {
    let path = normalized_path_key(path);
    let root = normalized_path_key(root);
    path == root || path.starts_with(&format!("{root}/"))
}

fn normalized_path_key(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn read_only_path_exists(path: &str) -> Result<bool, String> {
    Path::new(path)
        .try_exists()
        .map_err(|error| error.to_string())
}

fn push_unique_caveat(caveats: &mut Vec<String>, caveat: &str) {
    if !caveats.iter().any(|existing| existing == caveat) {
        caveats.push(caveat.to_owned());
    }
}

struct CurrentFileRow {
    path: String,
    source_location: String,
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
        assert_eq!(preview.can_proceed_to_confirmation, false);
        assert_eq!(preview.summary.backup_blocked_items, 1);
        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::ValidPreviewOnly
        );
        assert_eq!(
            preview.items[0].conflict_status,
            ApplyPlanConflictStatus::None
        );
        assert_eq!(preview.items[0].can_apply_later, false);
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
        assert_eq!(preview.items[0].can_apply_later, false);
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
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(8),
                path_string(&source_path),
                Some(path_string(&mods_root.join("CAS").join("root.package"))),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings::default(),
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
        let source = include_str!("apply_plan_validation.rs");
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
