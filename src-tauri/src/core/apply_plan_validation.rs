use std::{collections::HashMap, path::Path};

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::{apply_plan_persistence, apply_plan_provenance},
    error::{AppError, AppResult},
    models::{
        ApplyPlanConflictStatus, ApplyPlanValidationItem, ApplyPlanValidationPreview,
        ApplyPlanValidationPreviewStatus, ApplyPlanValidationStatus, ApplyPlanValidationSummary,
        LibrarySettings, PersistedApplyPlan, PersistedApplyPlanBlocker, PersistedApplyPlanItem,
        PersistedApplyPlanItemStatus, PersistedApplyPlanStatus, PreviewApplyPlanValidationRequest,
    },
    platform::{
        current_platform,
        path_semantics::{
            PathComparisonKey, PathMetadataState, PathSemanticsError, RootIdentity,
        },
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

    let root_identities = probe_configured_root_identities(settings);
    let destination_conflicts = detect_destination_conflicts(&root_identities, &plan.items);
    let mut items = Vec::with_capacity(plan.items.len());
    for item in &plan.items {
        items.push(validate_item(
            connection,
            &root_identities,
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
    root_identities: &ConfiguredRootIdentities,
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
    } else if let Some(conflict_status) = destination_conflict {
        match conflict_status {
            ApplyPlanConflictStatus::CaseConflict => {
                reasons.push(
                    "Another item in this plan targets the same destination under the configured root's case-insensitive filesystem rules.".to_owned(),
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
        (ApplyPlanValidationStatus::Blocked, conflict_status)
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
            root_identities,
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
    root_identities: &ConfiguredRootIdentities,
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

    if contains_parent_dir_component(destination_path) {
        reasons.push(
            "Destination path contains a parent-directory traversal component.".to_owned(),
        );
        required_next_steps.push("Regenerate the preview with a safe destination path.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::UnsafeDestination,
            ApplyPlanConflictStatus::Unsupported,
        ));
    }

    let destination_root_identity =
        match configured_root_identity(root_identities, destination_root_name) {
            Some(Ok(identity)) => identity,
            None => {
                reasons.push(format!(
                    "Destination root `{destination_root_name}` is not configured."
                ));
                required_next_steps
                    .push("Configure the destination root before future validation.".to_owned());
                return Ok((
                    ApplyPlanValidationStatus::MissingDestinationRoot,
                    ApplyPlanConflictStatus::Unsupported,
                ));
            }
            Some(Err(error)) => {
                reasons.push(format!(
                    "Destination root `{destination_root_name}` could not be safely inspected: {}",
                    error.message
                ));
                required_next_steps.push(
                    if error.missing_root {
                        "Choose an existing destination root before future validation."
                    } else {
                        "Review destination root permissions and filesystem capabilities before future validation."
                    }
                    .to_owned(),
                );
                return Ok(if error.missing_root {
                    (
                        ApplyPlanValidationStatus::MissingDestinationRoot,
                        ApplyPlanConflictStatus::Unsupported,
                    )
                } else {
                    (
                        ApplyPlanValidationStatus::Error,
                        ApplyPlanConflictStatus::PermissionUnknown,
                    )
                });
            }
        };

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

    let current_source_identity = match destination_root_identity
        .identify_path("legacy-library-settings", Path::new(&current_file.path))
    {
        Ok(identity) => identity,
        Err(error) => {
            let stale_source = source_path_error_is_stale(&error);
            reasons.push(format!(
                "Current indexed source path cannot be proven under its configured root: {error}"
            ));
            required_next_steps.push(
                if stale_source {
                    "Rescan or review the current Library location before future validation."
                } else {
                    "Review source permissions and filesystem capabilities before future validation."
                }
                .to_owned(),
            );
            return Ok(if stale_source {
                (
                    ApplyPlanValidationStatus::StaleSource,
                    ApplyPlanConflictStatus::NotChecked,
                )
            } else {
                (
                    ApplyPlanValidationStatus::Error,
                    ApplyPlanConflictStatus::PermissionUnknown,
                )
            });
        }
    };

    let saved_source_identity = match destination_root_identity
        .identify_path("legacy-library-settings", Path::new(&item.current_path))
    {
        Ok(identity) => identity,
        Err(error) => {
            let stale_source = source_path_error_is_stale(&error);
            reasons.push(format!(
                "Saved source path cannot be proven under the configured root: {error}"
            ));
            required_next_steps.push(
                if stale_source {
                    "Regenerate the preview from the current Library location."
                } else {
                    "Review source permissions and filesystem capabilities before future validation."
                }
                .to_owned(),
            );
            return Ok(if stale_source {
                (
                    ApplyPlanValidationStatus::StaleSource,
                    ApplyPlanConflictStatus::NotChecked,
                )
            } else {
                (
                    ApplyPlanValidationStatus::Error,
                    ApplyPlanConflictStatus::PermissionUnknown,
                )
            });
        }
    };

    if current_source_identity.comparison_key != saved_source_identity.comparison_key {
        reasons.push("Saved source path no longer matches the current Library index.".to_owned());
        required_next_steps
            .push("Review the current Library location before future validation.".to_owned());
        return Ok((
            ApplyPlanValidationStatus::StaleSource,
            ApplyPlanConflictStatus::NotChecked,
        ));
    }

    let destination_identity = match destination_root_identity
        .identify_path("legacy-library-settings", Path::new(destination_path))
    {
        Ok(identity) => identity,
        Err(error) => {
            let unsafe_destination = matches!(
                &error,
                PathSemanticsError::CandidateMustBeAbsolute(_)
                    | PathSemanticsError::RelativePathRequired(_)
                    | PathSemanticsError::ParentTraversal(_)
                    | PathSemanticsError::UnsupportedPathComponent(_)
                    | PathSemanticsError::ExistingAncestorNotDirectory(_)
                    | PathSemanticsError::OutsideRoot { .. }
            );
            reasons.push(format!(
                "Destination path cannot be proven under the configured Mods/Tray root: {error}"
            ));
            required_next_steps.push(
                if unsafe_destination {
                    "Regenerate the preview with a safe destination path."
                } else {
                    "Review destination permissions and filesystem capabilities before future validation."
                }
                .to_owned(),
            );
            return Ok(if unsafe_destination {
                (
                    ApplyPlanValidationStatus::UnsafeDestination,
                    ApplyPlanConflictStatus::Unsupported,
                )
            } else {
                (
                    ApplyPlanValidationStatus::Error,
                    ApplyPlanConflictStatus::PermissionUnknown,
                )
            });
        }
    };

    match destination_identity.metadata_state {
        PathMetadataState::File
        | PathMetadataState::Directory
        | PathMetadataState::Symlink
        | PathMetadataState::Other => {
            reasons
                .push("A file or folder already exists at the saved destination path.".to_owned());
            required_next_steps
                .push("Resolve the destination conflict before future validation.".to_owned());
            Ok((
                ApplyPlanValidationStatus::DestinationExists,
                ApplyPlanConflictStatus::DestinationExists,
            ))
        }
        PathMetadataState::Missing => {
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
        PathMetadataState::Unreadable(error_kind) => {
            reasons.push(format!(
                "Destination conflict could not be checked safely: {error_kind}"
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

fn detect_destination_conflicts(
    root_identities: &ConfiguredRootIdentities,
    items: &[PersistedApplyPlanItem],
) -> HashMap<i64, ApplyPlanConflictStatus> {
    let mut by_identity: HashMap<(String, PathComparisonKey), Vec<(i64, String)>> =
        HashMap::new();
    for item in items {
        if item.blocked
            || item.review_only
            || item.item_status == PersistedApplyPlanItemStatus::Blocked
            || item.item_status == PersistedApplyPlanItemStatus::ReviewOnly
        {
            continue;
        }
        let Some(destination_path) = item
            .destination_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(destination_root_name) = item
            .destination_root
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(Ok(root_identity)) =
            configured_root_identity(root_identities, destination_root_name)
        else {
            continue;
        };
        let Ok(identity) = root_identity
            .identify_path("legacy-library-settings", Path::new(destination_path))
        else {
            continue;
        };

        by_identity
            .entry((identity.root_id, identity.comparison_key))
            .or_default()
            .push((item.id, normalize_separators_for_display(destination_path)));
    }

    let mut conflicts = HashMap::new();
    for destinations in by_identity.values() {
        if destinations.len() < 2 {
            continue;
        }
        let first_path = &destinations[0].1;
        let paths_differ = destinations.iter().any(|(_, path)| path != first_path);
        let conflict_status = if paths_differ
            && destinations
                .iter()
                .all(|(_, path)| path.to_lowercase() == first_path.to_lowercase())
        {
            ApplyPlanConflictStatus::CaseConflict
        } else {
            ApplyPlanConflictStatus::SameNameConflict
        };
        for (item_id, _) in destinations {
            conflicts.insert(*item_id, conflict_status.clone());
        }
    }
    conflicts
}

fn normalize_separators_for_display(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_owned()
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

type ConfiguredRootIdentities =
    HashMap<String, Result<RootIdentity, ConfiguredRootProbeFailure>>;

#[derive(Debug)]
struct ConfiguredRootProbeFailure {
    message: String,
    missing_root: bool,
}

fn probe_configured_root_identities(settings: &LibrarySettings) -> ConfiguredRootIdentities {
    let mut identities = HashMap::new();
    for (root_name, root_path) in [
        ("mods", settings.mods_path.as_deref()),
        ("tray", settings.tray_path.as_deref()),
    ] {
        let Some(root_path) = root_path else {
            continue;
        };
        let result = RootIdentity::probe_existing_root(
            root_name,
            current_platform(),
            Path::new(root_path),
        )
        .map_err(|error| ConfiguredRootProbeFailure {
            missing_root: root_error_is_missing(&error),
            message: error.to_string(),
        });
        identities.insert(root_name.to_owned(), result);
    }
    identities
}

fn configured_root_identity<'a>(
    identities: &'a ConfiguredRootIdentities,
    root_name: &str,
) -> Option<&'a Result<RootIdentity, ConfiguredRootProbeFailure>> {
    identities.get(&root_name.trim().to_ascii_lowercase())
}

fn root_error_is_missing(error: &PathSemanticsError) -> bool {
    match error {
        PathSemanticsError::RootMustBeAbsolute(_) | PathSemanticsError::RootIsNotDirectory(_) => {
            true
        }
        PathSemanticsError::RootUnavailable { source, .. } => {
            source.kind() == std::io::ErrorKind::NotFound
        }
        _ => false,
    }
}

fn contains_parent_dir_component(path: &str) -> bool {
    path.replace('\\', "/")
        .split('/')
        .any(|component| component == "..")
}

fn source_path_error_is_stale(error: &PathSemanticsError) -> bool {
    matches!(
        error,
        PathSemanticsError::CandidateMustBeAbsolute(_)
            | PathSemanticsError::RelativePathRequired(_)
            | PathSemanticsError::ParentTraversal(_)
            | PathSemanticsError::UnsupportedPathComponent(_)
            | PathSemanticsError::ExistingAncestorNotDirectory(_)
            | PathSemanticsError::OutsideRoot { .. }
    )
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
        platform::path_semantics::CaseSensitivity,
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
    fn canonical_destination_validation_follows_root_case_policy() {
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

        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let root_identities = probe_configured_root_identities(&settings);
        let case_sensitivity = configured_root_identity(&root_identities, "mods")
            .expect("configured root")
            .as_ref()
            .expect("root probe")
            .capabilities
            .case_sensitivity;
        let preview = preview_apply_plan_validation(
            &connection,
            &settings,
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        match case_sensitivity {
            CaseSensitivity::Insensitive => {
                assert_eq!(preview.summary.conflict_items, 2);
                assert!(preview.items.iter().all(|item| {
                    item.conflict_status == ApplyPlanConflictStatus::CaseConflict
                }));
                assert!(preview.items.iter().all(|item| item.blocked));
            }
            CaseSensitivity::Sensitive => {
                assert_eq!(preview.summary.conflict_items, 0);
                assert!(preview.items.iter().all(|item| {
                    item.conflict_status == ApplyPlanConflictStatus::None
                        && item.validation_status == ApplyPlanValidationStatus::ValidPreviewOnly
                }));
            }
            CaseSensitivity::Unknown => {
                assert!(preview.items.iter().all(|item| {
                    item.validation_status == ApplyPlanValidationStatus::Error
                        && item.conflict_status == ApplyPlanConflictStatus::PermissionUnknown
                }));
            }
        }
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
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(21),
                path_string(&source_path),
                Some("C:\\Sims\\Mods\\..\\Elsewhere\\hair.package".to_owned()),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some("C:\\Sims\\Mods".to_owned()),
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
    fn source_path_drift_follows_root_case_policy() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("CaseSource.package");
        let saved_source_path = mods_root.join("caseSource.package");
        let destination_path = mods_root.join("CAS").join("CaseSource.package");
        fs::create_dir_all(&mods_root).expect("mods root");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 64, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(64),
                path_string(&saved_source_path),
                Some(path_string(&destination_path)),
            )],
        );
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let root_identities = probe_configured_root_identities(&settings);
        let case_sensitivity = configured_root_identity(&root_identities, "mods")
            .expect("configured root")
            .as_ref()
            .expect("root probe")
            .capabilities
            .case_sensitivity;

        let preview = preview_apply_plan_validation(
            &connection,
            &settings,
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        match case_sensitivity {
            CaseSensitivity::Sensitive => {
                assert_eq!(
                    preview.items[0].validation_status,
                    ApplyPlanValidationStatus::StaleSource
                );
            }
            CaseSensitivity::Insensitive => {
                assert_eq!(
                    preview.items[0].validation_status,
                    ApplyPlanValidationStatus::ValidPreviewOnly
                );
                assert_eq!(
                    preview.items[0].conflict_status,
                    ApplyPlanConflictStatus::None
                );
            }
            CaseSensitivity::Unknown => {
                assert_eq!(
                    preview.items[0].validation_status,
                    ApplyPlanValidationStatus::Error
                );
                assert_eq!(
                    preview.items[0].conflict_status,
                    ApplyPlanConflictStatus::PermissionUnknown
                );
            }
        }
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
    fn configured_destination_root_must_exist_on_disk() {
        let temp = tempdir().expect("tempdir");
        let missing_mods_root = temp.path().join("MissingMods");
        let source_path = temp.path().join("source.package");
        let destination_path = missing_mods_root.join("CAS").join("source.package");
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
                mods_path: Some(path_string(&missing_mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::MissingDestinationRoot
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("could not be safely inspected")));
    }

    #[test]
    fn configured_destination_root_must_be_absolute() {
        let temp = tempdir().expect("tempdir");
        let source_path = temp.path().join("source.package");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 63, &path_string(&source_path), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(63),
                path_string(&source_path),
                Some("relative/Mods/CAS/source.package".to_owned()),
            )],
        );

        let preview = preview_apply_plan_validation(
            &connection,
            &LibrarySettings {
                mods_path: Some("relative/Mods".to_owned()),
                ..Default::default()
            },
            PreviewApplyPlanValidationRequest { plan_id },
        )
        .expect("preview");

        assert_eq!(
            preview.items[0].validation_status,
            ApplyPlanValidationStatus::MissingDestinationRoot
        );
        assert!(preview.items[0]
            .reasons
            .iter()
            .any(|reason| reason.contains("must be an absolute path")));
    }

    #[test]
    fn root_probe_errors_distinguish_missing_paths_from_permission_failures() {
        let missing = PathSemanticsError::RootUnavailable {
            path: std::path::PathBuf::from("/missing/Mods"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        let denied = PathSemanticsError::RootUnavailable {
            path: std::path::PathBuf::from("/restricted/Mods"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };

        assert!(root_error_is_missing(&missing));
        assert!(!root_error_is_missing(&denied));
    }

    #[cfg(unix)]
    #[test]
    fn destination_symlinks_must_remain_inside_the_configured_root() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let inside = mods_root.join("Inside");
        let outside = temp.path().join("Outside");
        let source_path = mods_root.join("source.package");
        fs::create_dir_all(&inside).expect("inside directory");
        fs::create_dir(&outside).expect("outside directory");
        fs::write(&source_path, b"package").expect("source file");
        symlink(&outside, mods_root.join("EscapeLink")).expect("escape symlink");
        symlink(&inside, mods_root.join("InsideLink")).expect("inside symlink");

        let mut connection = memory_connection();
        insert_file(&connection, 62, &path_string(&source_path), "mods");

        let escape_plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(62),
                path_string(&source_path),
                Some(path_string(
                    &mods_root.join("EscapeLink").join("escaped.package"),
                )),
            )],
        );
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let escape_preview = preview_apply_plan_validation(
            &connection,
            &settings,
            PreviewApplyPlanValidationRequest {
                plan_id: escape_plan_id,
            },
        )
        .expect("escape preview");
        assert_eq!(
            escape_preview.items[0].validation_status,
            ApplyPlanValidationStatus::UnsafeDestination
        );

        let internal_plan_id = save_plan(
            &mut connection,
            vec![candidate_item(
                Some(62),
                path_string(&source_path),
                Some(path_string(
                    &mods_root.join("InsideLink").join("internal.package"),
                )),
            )],
        );
        let internal_preview = preview_apply_plan_validation(
            &connection,
            &settings,
            PreviewApplyPlanValidationRequest {
                plan_id: internal_plan_id,
            },
        )
        .expect("internal preview");
        assert_eq!(
            internal_preview.items[0].validation_status,
            ApplyPlanValidationStatus::ValidPreviewOnly
        );
        assert_eq!(
            internal_preview.items[0].conflict_status,
            ApplyPlanConflictStatus::None
        );
    }

    #[cfg(unix)]
    #[test]
    fn internal_aliases_to_one_destination_are_not_mislabeled_as_case_conflicts() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let inside = mods_root.join("Inside");
        let alias = mods_root.join("InsideAlias");
        let source_path_a = mods_root.join("a.package");
        let source_path_b = mods_root.join("b.package");
        fs::create_dir_all(&inside).expect("inside directory");
        fs::write(&source_path_a, b"a").expect("source a");
        fs::write(&source_path_b, b"b").expect("source b");
        symlink(&inside, &alias).expect("inside alias");

        let mut connection = memory_connection();
        insert_file(&connection, 65, &path_string(&source_path_a), "mods");
        insert_file(&connection, 66, &path_string(&source_path_b), "mods");
        let plan_id = save_plan(
            &mut connection,
            vec![
                candidate_item(
                    Some(65),
                    path_string(&source_path_a),
                    Some(path_string(&inside.join("shared.package"))),
                ),
                candidate_item(
                    Some(66),
                    path_string(&source_path_b),
                    Some(path_string(&alias.join("shared.package"))),
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
        assert!(preview.items.iter().all(|item| {
            item.conflict_status == ApplyPlanConflictStatus::SameNameConflict && item.blocked
        }));
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
