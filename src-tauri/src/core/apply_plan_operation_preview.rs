use rusqlite::Connection;
use serde_json::json;
use sha2::Digest;

use crate::{
    core::{
        apply_plan_dry_run, apply_plan_persistence, apply_plan_provenance, apply_plan_validation,
    },
    error::AppResult,
    models::{
        ApplyPlanDryRunActionPreview, ApplyPlanDryRunItem, ApplyPlanDryRunItemStatus,
        ApplyPlanOperationPreview, ApplyPlanOperationPreviewItem, ApplyPlanOperationPreviewStatus,
        ApplyPlanOperationPreviewSummary, LibrarySettings, PreviewApplyPlanDryRunRequest,
        PreviewApplyPlanOperationsRequest, PreviewApplyPlanValidationRequest,
    },
};

pub const OPERATION_SET_HASH_VERSION: &str = "apply_plan_operation_set_v1";
pub const OPERATION_SET_HASH_ALGORITHM: &str = apply_plan_provenance::PLAN_HASH_ALGORITHM;

pub fn preview_apply_plan_operations(
    connection: &Connection,
    settings: &LibrarySettings,
    request: PreviewApplyPlanOperationsRequest,
) -> AppResult<ApplyPlanOperationPreview> {
    let validation = apply_plan_validation::preview_apply_plan_validation(
        connection,
        settings,
        PreviewApplyPlanValidationRequest {
            plan_id: request.plan_id,
        },
    )?;
    let dry_run = apply_plan_dry_run::preview_apply_plan_dry_run(
        connection,
        settings,
        PreviewApplyPlanDryRunRequest {
            plan_id: request.plan_id,
        },
    )?;

    let mut caveats = validation.caveats;
    for caveat in dry_run.caveats {
        push_unique(&mut caveats, &caveat);
    }
    push_unique(
        &mut caveats,
        "No files changed. This operation-set preview is read-only.",
    );
    push_unique(
        &mut caveats,
        "Operation-set preview is not Apply; confirmation, backup, restore map, result log, and executor proof are still required.",
    );

    let backend_generated = validation.source_plan_kind
        == apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND;
    if !backend_generated {
        push_unique(
            &mut caveats,
            "Client-supplied or legacy ApplyPlan previews are review/audit-only and cannot produce operation-set candidates.",
        );
    }

    let plan_hash_check =
        apply_plan_persistence::verify_apply_plan_hash(connection, request.plan_id)?;
    if !plan_hash_check.is_valid {
        push_unique(
            &mut caveats,
            &format!("{} ({})", plan_hash_check.message, plan_hash_check.status),
        );
    }

    let expected_plan_hash_matches = expected_plan_hash_matches(
        request.expected_plan_hash.as_deref(),
        validation.plan_hash.as_deref(),
        &mut caveats,
    );

    let operations = if backend_generated && plan_hash_check.is_valid && expected_plan_hash_matches
    {
        dry_run
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.dry_run_status,
                    ApplyPlanDryRunItemStatus::CandidateAfterFutureSafetyGates
                        | ApplyPlanDryRunItemStatus::WouldRequireBackup
                )
            })
            .map(operation_from_dry_run_item)
            .collect()
    } else {
        Vec::new()
    };

    let summary = ApplyPlanOperationPreviewSummary {
        total_items: dry_run.summary.total_items,
        candidate_operations: operations.len() as i64,
        blocked_items: dry_run.summary.blocked_items,
        skipped_items: dry_run.summary.skipped_items,
        review_only_items: dry_run.summary.review_only_items,
        conflict_items: dry_run.summary.conflict_items,
        backup_required_items: dry_run.summary.backup_required_items,
    };
    let status = if operations.is_empty() {
        ApplyPlanOperationPreviewStatus::Blocked
    } else {
        ApplyPlanOperationPreviewStatus::PreviewOnly
    };

    let operation_set_hash = operation_set_hash(
        request.plan_id,
        validation.plan_hash.as_deref(),
        &operations,
    )?;

    Ok(ApplyPlanOperationPreview {
        plan_id: request.plan_id,
        status,
        can_proceed_to_apply: false,
        can_proceed_to_confirmation: false,
        checked_at: validation.checked_at,
        operation_set_hash,
        operation_set_hash_algorithm: OPERATION_SET_HASH_ALGORITHM.to_owned(),
        operation_set_hash_version: OPERATION_SET_HASH_VERSION.to_owned(),
        summary,
        caveats,
        operations,
    })
}

fn expected_plan_hash_matches(
    expected_plan_hash: Option<&str>,
    actual_plan_hash: Option<&str>,
    caveats: &mut Vec<String>,
) -> bool {
    let Some(expected) = expected_plan_hash
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return true;
    };

    if Some(expected) == actual_plan_hash {
        true
    } else {
        push_unique(
            caveats,
            "Expected ApplyPlan hash did not match the saved plan hash. Regenerate or reload the saved plan before previewing operations.",
        );
        false
    }
}

fn operation_set_hash(
    plan_id: i64,
    plan_hash: Option<&str>,
    operations: &[ApplyPlanOperationPreviewItem],
) -> AppResult<String> {
    let payload = json!({
        "operationSetHashVersion": OPERATION_SET_HASH_VERSION,
        "operationSetHashAlgorithm": OPERATION_SET_HASH_ALGORITHM,
        "planId": plan_id,
        "planHash": plan_hash,
        "operations": operations,
        "readOnlyBoundary": {
            "canProceedToApply": false,
            "canProceedToConfirmation": false,
            "canExecute": false,
            "note": "Operation-set hash binds preview rows only; it does not authorize file mutation."
        }
    });
    Ok(hex::encode(sha2::Sha256::digest(serde_json::to_vec(
        &payload,
    )?)))
}

fn operation_from_dry_run_item(item: &ApplyPlanDryRunItem) -> ApplyPlanOperationPreviewItem {
    ApplyPlanOperationPreviewItem {
        item_id: item.item_id,
        file_id: item.file_id,
        file_name: item.file_name.clone(),
        source_path: item.source_path.clone(),
        destination_path: item.destination_path.clone(),
        action_preview: operation_action_preview(item),
        reasons: item.reasons.clone(),
        required_before_apply: item.required_before_apply.clone(),
        can_apply: false,
    }
}

fn operation_action_preview(item: &ApplyPlanDryRunItem) -> ApplyPlanDryRunActionPreview {
    if item.dry_run_status == ApplyPlanDryRunItemStatus::WouldRequireBackup
        && item.source_path.is_some()
        && item.destination_path.is_some()
    {
        ApplyPlanDryRunActionPreview::WouldMoveLater
    } else {
        item.action_preview.clone()
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
        core::apply_plan_persistence::{
            generate_sorting_preview_snapshot, get_apply_plan,
            save_apply_plan_from_preview_snapshot, save_apply_plan_preview,
        },
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope,
            SaveApplyPlanFromPreviewSnapshotRequest, SaveApplyPlanPreviewRequest, StagingPlan,
            StagingPlanActionKind, StagingPlanBucket, StagingPlanConfidenceLabel,
            StagingPlanCurrentRoot, StagingPlanEvidenceLevel, StagingPlanItem, StagingPlanSource,
            StagingPlanStatus,
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

    fn source_plan(items: Vec<StagingPlanItem>) -> StagingPlan {
        StagingPlan {
            id: "operation-preview-source-plan".to_owned(),
            created_at: "2026-05-24T00:00:00Z".to_owned(),
            source: StagingPlanSource::Organize,
            status: StagingPlanStatus::PreviewOnly,
            title: "Operation preview source plan".to_owned(),
            summary: "Preview-only operation preview source.".to_owned(),
            item_count: items.len(),
            would_touch_files: false,
            caveats: vec!["No files changed. Source plan is preview-only.".to_owned()],
            items,
        }
    }

    fn save_client_supplied_plan(connection: &mut Connection, items: Vec<StagingPlanItem>) -> i64 {
        save_apply_plan_preview(
            connection,
            SaveApplyPlanPreviewRequest {
                source_plan: source_plan(items),
                source_plan_kind: Some("ignored_client_kind".to_owned()),
                source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
        )
        .expect("save client-supplied plan")
        .plan_id
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
    fn backend_generated_valid_preview_returns_read_only_operation_candidates() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("backend-generated.package");
        let destination_parent = mods_root.join("CAS");
        fs::create_dir_all(&destination_parent).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &path_string(&source_path), "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let plan = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan");

        let preview = preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
            },
        )
        .expect("operation preview");

        assert_eq!(preview.status, ApplyPlanOperationPreviewStatus::PreviewOnly);
        assert!(!preview.can_proceed_to_confirmation);
        assert!(!preview.can_proceed_to_apply);
        assert_eq!(preview.summary.total_items, 1);
        assert_eq!(preview.summary.candidate_operations, 1);
        assert_eq!(preview.summary.backup_required_items, 1);
        assert_eq!(preview.operations.len(), 1);
        assert_eq!(preview.operations[0].file_id, Some(70));
        assert_eq!(preview.operations[0].file_name, "backend-generated.package");
        assert_eq!(
            preview.operations[0].action_preview,
            crate::models::ApplyPlanDryRunActionPreview::WouldMoveLater
        );
        assert_eq!(
            preview.operations[0].destination_path.as_deref(),
            plan.items[0].destination_path.as_deref()
        );
        assert!(!preview.operations[0].can_apply);
        assert!(preview.operations[0]
            .required_before_apply
            .iter()
            .any(|step| step.contains("Backup required")));
        assert!(preview
            .caveats
            .iter()
            .any(|caveat| caveat.contains("operation-set preview is read-only")));

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
    fn operation_preview_blocks_expected_plan_hash_mismatch() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("hash-mismatch.package");
        fs::create_dir_all(mods_root.join("CAS")).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &path_string(&source_path), "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);

        let preview = preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: Some("0".repeat(64)),
            },
        )
        .expect("operation preview");

        assert_eq!(preview.status, ApplyPlanOperationPreviewStatus::Blocked);
        assert_eq!(preview.summary.candidate_operations, 0);
        assert!(preview.operations.is_empty());
        assert!(preview
            .caveats
            .iter()
            .any(|caveat| caveat.contains("Expected ApplyPlan hash did not match")));
    }

    #[test]
    fn operation_preview_blocks_tampered_plan_provenance() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("tampered.package");
        fs::create_dir_all(mods_root.join("CAS")).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &path_string(&source_path), "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let original_hash = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan")
            .plan_hash;
        connection
            .execute(
                "UPDATE apply_plans SET context_trail_json = '[{\"sourceSystem\":\"test\",\"signalKind\":\"tamper\",\"label\":\"Tampered\",\"value\":null,\"strength\":\"evidence\"}]' WHERE id = ?1",
                [plan_id],
            )
            .expect("tamper context");

        let preview = preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: original_hash,
            },
        )
        .expect("operation preview");

        assert_eq!(preview.status, ApplyPlanOperationPreviewStatus::Blocked);
        assert_eq!(preview.summary.candidate_operations, 0);
        assert!(preview.operations.is_empty());
        assert!(preview
            .caveats
            .iter()
            .any(|caveat| caveat.contains("hash/provenance no longer matches")));
    }

    #[test]
    fn operation_preview_rejects_client_supplied_preview_without_writing_rows() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("client-supplied.package");
        let destination_path = mods_root.join("CAS").join("client-supplied.package");
        fs::create_dir_all(source_path.parent().unwrap()).expect("source parent");
        fs::create_dir_all(destination_path.parent().unwrap()).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 60, &path_string(&source_path), "mods");
        let plan_id = save_client_supplied_plan(
            &mut connection,
            vec![candidate_item(
                Some(60),
                path_string(&source_path),
                Some(path_string(&destination_path)),
            )],
        );

        let preview = preview_apply_plan_operations(
            &connection,
            &LibrarySettings {
                mods_path: Some(path_string(&mods_root)),
                ..Default::default()
            },
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: None,
            },
        )
        .expect("operation preview");

        assert_eq!(preview.status, ApplyPlanOperationPreviewStatus::Blocked);
        assert!(!preview.can_proceed_to_confirmation);
        assert!(!preview.can_proceed_to_apply);
        assert_eq!(preview.summary.candidate_operations, 0);
        assert!(preview.operations.is_empty());
        assert!(preview.caveats.iter().any(|caveat| {
            caveat.contains("Client-supplied") && caveat.contains("review/audit-only")
        }));

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
    fn confirmation_token_issue_binds_backend_plan_hash_and_current_operation_set() {
        let temp = tempdir().expect("tempdir");
        let mods_root = temp.path().join("Mods");
        let source_path = mods_root.join("confirmation-candidate.package");
        fs::create_dir_all(mods_root.join("CAS")).expect("destination parent");
        fs::write(&source_path, b"package").expect("source file");

        let mut connection = memory_connection();
        insert_file(&connection, 70, &path_string(&source_path), "mods");
        let settings = LibrarySettings {
            mods_path: Some(path_string(&mods_root)),
            ..Default::default()
        };
        let plan_id = save_backend_generated_plan(&mut connection, &settings);
        let plan = get_apply_plan(&connection, plan_id)
            .expect("load plan")
            .expect("saved plan");
        let operation_preview = preview_apply_plan_operations(
            &connection,
            &settings,
            PreviewApplyPlanOperationsRequest {
                plan_id,
                expected_plan_hash: plan.plan_hash.clone(),
            },
        )
        .expect("operation preview");

        let token =
            crate::core::apply_plan_confirmation_token::issue_apply_plan_confirmation_token(
                &connection,
                &settings,
                crate::models::IssueApplyPlanConfirmationTokenRequest {
                    plan_id,
                    expected_plan_hash: plan.plan_hash.clone(),
                    expected_operation_set_hash: Some(operation_preview.operation_set_hash.clone()),
                },
            )
            .expect("issue confirmation token");

        assert_eq!(token.plan_id, plan_id);
        assert_eq!(token.plan_hash, plan.plan_hash.expect("plan hash"));
        assert_eq!(
            token.operation_set_hash,
            operation_preview.operation_set_hash
        );
        assert_eq!(token.allowed_operation_count, 1);
        assert_eq!(token.source_plan_kind, "backend_generated_sorting_preview");
        assert_eq!(
            token.single_use_state,
            crate::models::ApplyPlanConfirmationTokenUseState::Unused
        );
        assert!(!token.can_proceed_to_apply);
        assert!(!token.can_execute);
        assert!(!token.token.trim().is_empty());
        assert!(token
            .caveats
            .iter()
            .any(|caveat| caveat.contains("Apply executor is still locked")));
    }
}
