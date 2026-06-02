use rusqlite::Connection;

use crate::{
    core::{
        apply_plan_builders, apply_plan_hash_persistence, apply_plan_loading,
        apply_plan_preview_snapshot_consumption, apply_plan_preview_snapshots,
        apply_plan_provenance, apply_plan_records, apply_plan_saves,
    },
    error::{AppError, AppResult},
    models::{
        ApplyPlanListItem, BuildApplyPlanFromStagingPlanRequest, DeleteDraftApplyPlanResult,
        GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanResult, LibrarySettings,
        ListSavedApplyPlansRequest, PersistedApplyPlan, SaveApplyPlanFromPreviewSnapshotRequest,
        SaveApplyPlanPreviewRequest, SaveApplyPlanPreviewResult,
    },
};

pub fn generate_sorting_preview_snapshot(
    connection: &mut Connection,
    settings: &LibrarySettings,
    request: GenerateSortingPreviewPlanRequest,
) -> AppResult<GenerateSortingPreviewPlanResult> {
    apply_plan_preview_snapshots::generate_sorting_preview_snapshot(connection, settings, request)
}

pub fn build_apply_plan_from_staging_plan(
    connection: &mut Connection,
    settings: &LibrarySettings,
    request: BuildApplyPlanFromStagingPlanRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    apply_plan_builders::build_apply_plan_from_staging_plan(connection, settings, request)
}

pub fn save_apply_plan_preview(
    connection: &mut Connection,
    request: SaveApplyPlanPreviewRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    apply_plan_saves::save_apply_plan_preview_draft(
        connection,
        request,
        apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
        None,
        None,
    )
}

pub fn save_apply_plan_from_preview_snapshot(
    connection: &mut Connection,
    request: SaveApplyPlanFromPreviewSnapshotRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    apply_plan_preview_snapshot_consumption::save_apply_plan_from_preview_snapshot(
        connection, request,
    )
}

pub fn list_saved_apply_plans(
    connection: &Connection,
    request: ListSavedApplyPlansRequest,
) -> AppResult<Vec<ApplyPlanListItem>> {
    apply_plan_records::list_saved_apply_plans(connection, request)
}

pub fn get_apply_plan(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<Option<PersistedApplyPlan>> {
    apply_plan_loading::load_saved_apply_plan(connection, plan_id)
}

pub fn verify_apply_plan_hash(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<apply_plan_provenance::ApplyPlanHashVerification> {
    let plan = get_apply_plan(connection, plan_id)?
        .ok_or_else(|| AppError::Message(format!("Saved ApplyPlan {plan_id} was not found")))?;
    apply_plan_hash_persistence::verify_loaded_apply_plan_hash(connection, &plan)
}

pub fn delete_draft_apply_plan(
    connection: &mut Connection,
    plan_id: i64,
) -> AppResult<DeleteDraftApplyPlanResult> {
    apply_plan_records::cancel_draft_apply_plan(connection, plan_id)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        database,
        models::{
            BuildApplyPlanFromStagingPlanRequest, GenerateSortingPreviewPlanRequest,
            GenerateSortingPreviewPlanScope, LibrarySettings, PersistedApplyPlanItemStatus,
            SaveApplyPlanFromPreviewSnapshotRequest, StagingPlan,
        },
    };

    use super::*;

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn sample_plan() -> StagingPlan {
        let connection = memory_connection();
        let settings = LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        };
        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES
                ('C:/Sims/Mods/a.package', 'a.package', 'package', 1, '2026-01-01', 'h1', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                ('C:/Sims/Mods/b.package', 'b.package', 'package', 1, '2026-01-01', 'h2', 'Unknown', NULL, 0.10, '[]', '[\"parser_warning\"]', 'mods', 1)",
                [],
            )
            .expect("files");

        crate::core::rule_engine::sorting_plan::generate_sorting_preview_plan(
            &connection,
            &settings,
            GenerateSortingPreviewPlanRequest {
                scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                    file_ids: vec![1, 2],
                },
                folder_config: None,
                context_trail: Vec::new(),
            },
        )
        .expect("preview plan")
    }

    fn insert_snapshot_preview_files(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES
                (1, 'C:/Sims/Mods/a.package', 'a.package', 'package', 1, '2026-01-01', 'h1', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                (2, 'C:/Sims/Mods/b.package', 'b.package', 'package', 1, '2026-01-01', 'h2', 'Unknown', NULL, 0.10, '[]', '[\"parser_warning\"]', 'mods', 1)",
                [],
            )
            .expect("files");
    }

    fn snapshot_settings() -> LibrarySettings {
        LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        }
    }

    fn snapshot_preview_request() -> GenerateSortingPreviewPlanRequest {
        GenerateSortingPreviewPlanRequest {
            scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                file_ids: vec![1, 2],
            },
            folder_config: None,
            context_trail: Vec::new(),
        }
    }

    #[test]
    fn preview_snapshot_save_binds_backend_snapshot_and_rejects_reuse() {
        let mut connection = memory_connection();
        insert_snapshot_preview_files(&connection);
        let preview = generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("preview snapshot");

        assert!(preview.preview_snapshot_id > 0);
        assert_eq!(preview.preview_snapshot_hash.len(), 64);
        assert!(preview
            .preview_snapshot_hash
            .chars()
            .all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(
            preview.preview_snapshot_hash_version,
            apply_plan_provenance::PREVIEW_SNAPSHOT_HASH_VERSION
        );
        assert_eq!(preview.preview_snapshot_hash_algorithm, "sha256");

        let saved = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: preview.preview_snapshot_id,
                preview_snapshot_hash: preview.preview_snapshot_hash.clone(),
            },
        )
        .expect("save from snapshot");
        let persisted = get_apply_plan(&connection, saved.plan_id)
            .expect("get")
            .expect("persisted plan");

        assert_eq!(
            persisted.source_plan_kind,
            apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
        );
        assert_eq!(
            persisted.preview_snapshot_id,
            Some(preview.preview_snapshot_id)
        );
        assert_eq!(
            persisted.preview_snapshot_hash.as_deref(),
            Some(preview.preview_snapshot_hash.as_str())
        );
        assert_eq!(
            persisted
                .plan_provenance
                .get("previewSnapshotHash")
                .and_then(|value| value.as_str()),
            Some(preview.preview_snapshot_hash.as_str())
        );

        let consumed_apply_plan_id: Option<i64> = connection
            .query_row(
                "SELECT consumed_apply_plan_id FROM apply_plan_preview_snapshots WHERE id = ?1",
                [preview.preview_snapshot_id],
                |row| row.get(0),
            )
            .expect("consumed snapshot id");
        assert_eq!(consumed_apply_plan_id, Some(saved.plan_id));

        let reuse = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: preview.preview_snapshot_id,
                preview_snapshot_hash: preview.preview_snapshot_hash,
            },
        )
        .expect_err("snapshot reuse must fail closed");
        assert!(reuse.to_string().contains("already been saved"));
    }

    #[test]
    fn generated_preview_snapshots_saved_twice_have_same_plan_hash() {
        let mut connection = memory_connection();
        insert_snapshot_preview_files(&connection);

        let first_preview = generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("first preview snapshot");
        let first_saved = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: first_preview.preview_snapshot_id,
                preview_snapshot_hash: first_preview.preview_snapshot_hash.clone(),
            },
        )
        .expect("first save from snapshot");

        let second_preview = generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("second preview snapshot");
        let second_saved = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: second_preview.preview_snapshot_id,
                preview_snapshot_hash: second_preview.preview_snapshot_hash.clone(),
            },
        )
        .expect("second save from snapshot");

        let first_plan = get_apply_plan(&connection, first_saved.plan_id)
            .expect("get first")
            .expect("first plan");
        let second_plan = get_apply_plan(&connection, second_saved.plan_id)
            .expect("get second")
            .expect("second plan");

        assert_eq!(
            first_preview.preview_snapshot_hash,
            second_preview.preview_snapshot_hash
        );
        assert_eq!(first_plan.plan_hash, second_plan.plan_hash);
    }

    #[test]
    fn preview_snapshot_save_rejects_hash_mismatch() {
        let mut connection = memory_connection();
        insert_snapshot_preview_files(&connection);
        let preview = generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("preview snapshot");

        let err = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: preview.preview_snapshot_id,
                preview_snapshot_hash: "0".repeat(64),
            },
        )
        .expect_err("hash mismatch must fail closed");
        assert!(err.to_string().contains("hash mismatch"));

        let saved_plan_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM apply_plans", [], |row| row.get(0))
            .expect("saved plan count");
        assert_eq!(saved_plan_count, 0);
    }

    #[test]
    fn preview_snapshot_save_rejects_rule_drift_before_persistence() {
        let mut connection = memory_connection();
        insert_snapshot_preview_files(&connection);
        let preview = generate_sorting_preview_snapshot(
            &mut connection,
            &snapshot_settings(),
            snapshot_preview_request(),
        )
        .expect("preview snapshot");

        connection
            .execute(
                "INSERT INTO rules (rule_name, rule_template, rule_priority, enabled)
                 VALUES ('snapshot-drift', 'changed after preview', 1, 1)",
                [],
            )
            .expect("rule drift");

        let err = save_apply_plan_from_preview_snapshot(
            &mut connection,
            SaveApplyPlanFromPreviewSnapshotRequest {
                preview_snapshot_id: preview.preview_snapshot_id,
                preview_snapshot_hash: preview.preview_snapshot_hash,
            },
        )
        .expect_err("rule drift must fail closed");
        assert!(err.to_string().contains("provenance no longer matches"));
    }

    #[test]
    fn saving_preview_plan_creates_plan_items_signals_and_blockers() {
        let mut connection = memory_connection();
        let result = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: Some("organize".to_owned()),
                source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
        )
        .expect("save");

        assert!(result.plan_id > 0);
        assert_eq!(
            result.plan.source_plan_kind,
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND
        );
        assert!(!result.plan.would_touch_files);
        assert_eq!(result.plan.applyable_items, 0);
        assert_eq!(result.plan.total_items, 2);

        let saved = get_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved plan");
        let plan_hash = saved.plan_hash.as_ref().expect("backend plan hash");
        assert_eq!(saved.plan_hash_version, "apply_plan_hash_v1");
        assert_eq!(saved.plan_hash_algorithm, "sha256");
        assert_eq!(plan_hash.len(), 64);
        assert!(plan_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(
            saved
                .plan_provenance
                .get("sourceScope")
                .and_then(|scope| scope.get("kind"))
                .and_then(|value| value.as_str()),
            Some("selected_files")
        );
        assert_eq!(
            saved
                .plan_provenance
                .get("items")
                .and_then(|items| items.as_array())
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(saved.items.len(), 2);
        assert!(saved.items.iter().all(|item| !item.signals.is_empty()));
        assert!(saved
            .items
            .iter()
            .any(|item| item.item_status == PersistedApplyPlanItemStatus::Blocked));
        assert!(saved
            .items
            .iter()
            .flat_map(|item| item.blockers.iter())
            .any(|blocker| blocker.reason_code == "parser_warning"));
    }

    #[test]
    fn no_execution_status_can_be_inserted_by_schema() {
        let connection = memory_connection();
        let result = connection.execute(
            "INSERT INTO apply_plans (
                source_plan_kind, title, summary, status, total_items, caveats_json
            ) VALUES ('organize', 'bad', 'bad', 'applied', 0, '[]')",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn backend_generated_preview_saved_twice_has_same_hash() {
        let mut connection = memory_connection();
        let settings = LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        };
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES
                (10, 'C:/Sims/Mods/Shared.package', 'shared.package', 'package', 1, '2026-01-01', 'h10', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                (11, 'C:/Sims/Mods/Nested/shared.package', 'shared.package', 'package', 1, '2026-01-01', 'h11', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 2)",
                [],
            )
            .expect("files");

        let request = BuildApplyPlanFromStagingPlanRequest {
            preview_request: GenerateSortingPreviewPlanRequest {
                scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                    file_ids: vec![10, 11],
                },
                folder_config: None,
                context_trail: Vec::new(),
            },
            source_plan_kind: Some("malicious_client_override".to_owned()),
            folder_config: None,
            context_trail: Vec::new(),
        };

        let first = build_apply_plan_from_staging_plan(&mut connection, &settings, request.clone())
            .expect("first save");
        let second = build_apply_plan_from_staging_plan(&mut connection, &settings, request)
            .expect("second save");
        let first_plan = get_apply_plan(&connection, first.plan_id)
            .expect("get first")
            .expect("first plan");
        let second_plan = get_apply_plan(&connection, second.plan_id)
            .expect("get second")
            .expect("second plan");

        assert_eq!(
            first_plan.source_plan_kind,
            apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
        );
        assert_eq!(first_plan.plan_hash, second_plan.plan_hash);
    }

    #[test]
    fn persisted_plan_serialization_exposes_hash_metadata_not_full_provenance() {
        let mut connection = memory_connection();
        let result = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: Some("ignored_client_kind".to_owned()),
                source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
        )
        .expect("save");
        let saved = get_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved");

        let serialized = serde_json::to_string(&saved).expect("serialize plan");
        assert!(serialized.contains("planHash"));
        assert!(serialized.contains("planHashVersion"));
        assert!(!serialized.contains("planProvenance"));
        assert!(!serialized.contains("plan_provenance"));
        assert!(!serialized.contains("sourceFileSnapshot"));
    }
}
