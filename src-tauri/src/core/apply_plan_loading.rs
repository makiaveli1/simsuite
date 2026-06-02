use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::{apply_plan_items, apply_plan_persistence_json, apply_plan_persistence_rows},
    error::AppResult,
    models::{ApplyPlanContextSignal, ApplyPlanFolderConfig, PersistedApplyPlan},
};

pub(crate) fn load_saved_apply_plan(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<Option<PersistedApplyPlan>> {
    let plan = connection
        .query_row(
            "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
                would_touch_files, confirmation_required, backup_required, restore_available,
                total_items, applyable_items, blocked_items, review_only_items,
                caveats_json, source_scope_json, folder_config_json, context_trail_json,
                plan_hash, plan_hash_version, plan_hash_algorithm, plan_hash_created_at,
                preview_snapshot_id, preview_snapshot_hash,
                plan_provenance_json, scan_session_id, created_at, updated_at
             FROM apply_plans
             WHERE id = ?1",
            params![plan_id],
            apply_plan_persistence_rows::plan_row_from_row,
        )
        .optional()?;

    let Some(plan) = plan else {
        return Ok(None);
    };

    let items = apply_plan_items::load_apply_plan_items(connection, plan.id)?;
    let caveats: Vec<String> = apply_plan_persistence_json::parse_required_json_field(
        &plan.caveats_json,
        "caveats_json",
        "ApplyPlan",
        plan.id,
    )?;
    let source_scope: Option<serde_json::Value> =
        apply_plan_persistence_json::parse_optional_json_field(
            plan.source_scope_json.as_deref(),
            "source_scope_json",
            "ApplyPlan",
            plan.id,
        )?;
    let folder_config: Option<ApplyPlanFolderConfig> =
        apply_plan_persistence_json::parse_optional_json_field(
            plan.folder_config_json.as_deref(),
            "folder_config_json",
            "ApplyPlan",
            plan.id,
        )?;
    let context_trail: Vec<ApplyPlanContextSignal> =
        apply_plan_persistence_json::parse_required_json_field(
            &plan.context_trail_json,
            "context_trail_json",
            "ApplyPlan",
            plan.id,
        )?;
    let plan_provenance: serde_json::Value =
        apply_plan_persistence_json::parse_required_json_field(
            &plan.plan_provenance_json,
            "plan_provenance_json",
            "ApplyPlan",
            plan.id,
        )?;

    Ok(Some(PersistedApplyPlan {
        id: plan.id,
        source_staging_plan_id: plan.source_staging_plan_id,
        source_plan_kind: plan.source_plan_kind,
        title: plan.title,
        summary: plan.summary,
        status: apply_plan_persistence_rows::parse_plan_status(&plan.status)?,
        would_touch_files: apply_plan_persistence_rows::int_to_bool(plan.would_touch_files),
        confirmation_required: apply_plan_persistence_rows::int_to_bool(plan.confirmation_required),
        backup_required: apply_plan_persistence_rows::int_to_bool(plan.backup_required),
        restore_available: apply_plan_persistence_rows::int_to_bool(plan.restore_available),
        total_items: plan.total_items,
        applyable_items: plan.applyable_items,
        blocked_items: plan.blocked_items,
        review_only_items: plan.review_only_items,
        caveats,
        source_scope,
        folder_config,
        context_trail,
        plan_hash: plan.plan_hash,
        plan_hash_version: plan.plan_hash_version,
        plan_hash_algorithm: plan.plan_hash_algorithm,
        plan_hash_created_at: plan.plan_hash_created_at,
        preview_snapshot_id: plan.preview_snapshot_id,
        preview_snapshot_hash: plan.preview_snapshot_hash,
        plan_provenance,
        scan_session_id: plan.scan_session_id,
        created_at: plan.created_at,
        updated_at: plan.updated_at,
        items,
    }))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::{apply_plan_provenance, apply_plan_saves},
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            PersistedApplyPlanItemStatus, PersistedApplyPlanStatus, SaveApplyPlanPreviewRequest,
            StagingPlan,
        },
    };

    use super::load_saved_apply_plan;

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

    #[test]
    fn helper_assembles_items_json_and_provenance_without_mutating_record() {
        let mut connection = memory_connection();
        let result = apply_plan_saves::save_apply_plan_preview_draft(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: Some("organize".to_owned()),
                source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
            None,
            None,
        )
        .expect("save");
        let before = load_saved_apply_plan(&connection, result.plan_id)
            .expect("get before")
            .expect("saved before helper load");

        let loaded = load_saved_apply_plan(&connection, result.plan_id)
            .expect("load saved ApplyPlan through helper")
            .expect("saved plan should exist");

        assert_eq!(loaded.id, result.plan_id);
        assert_eq!(loaded.status, PersistedApplyPlanStatus::Blocked);
        assert_eq!(
            loaded.source_plan_kind,
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND
        );
        assert_eq!(
            loaded.source_scope,
            Some(serde_json::json!({"kind": "selected_files"}))
        );
        assert_eq!(loaded.items.len(), 2);
        assert!(loaded.items.iter().all(|item| !item.signals.is_empty()));
        assert!(loaded
            .items
            .iter()
            .flat_map(|item| item.blockers.iter())
            .any(|blocker| blocker.reason_code == "parser_warning"));
        assert!(loaded
            .items
            .iter()
            .any(|item| item.item_status == PersistedApplyPlanItemStatus::Blocked));
        assert_eq!(loaded.plan_hash, before.plan_hash);
        assert_eq!(loaded.plan_hash_version, before.plan_hash_version);
        assert_eq!(loaded.plan_hash_algorithm, before.plan_hash_algorithm);
        assert_eq!(loaded.plan_hash_created_at, before.plan_hash_created_at);
        assert_eq!(loaded.preview_snapshot_id, before.preview_snapshot_id);
        assert_eq!(loaded.preview_snapshot_hash, before.preview_snapshot_hash);
        assert_eq!(loaded.updated_at, before.updated_at);
        assert_eq!(loaded.plan_provenance, before.plan_provenance);

        let missing = load_saved_apply_plan(&connection, result.plan_id + 10_000)
            .expect("missing saved ApplyPlan lookup should not error");
        assert!(missing.is_none());
    }

    #[test]
    fn helper_malformed_persisted_plan_json_fails_closed() {
        for column in [
            "caveats_json",
            "source_scope_json",
            "folder_config_json",
            "context_trail_json",
        ] {
            let mut connection = memory_connection();
            let result = apply_plan_saves::save_apply_plan_preview_draft(
                &mut connection,
                SaveApplyPlanPreviewRequest {
                    source_plan: sample_plan(),
                    source_plan_kind: None,
                    source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                    folder_config: None,
                    context_trail: Vec::new(),
                    scan_session_id: None,
                },
                apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
                None,
                None,
            )
            .expect("save");
            connection
                .execute(
                    &format!("UPDATE apply_plans SET {column} = '{{' WHERE id = ?1"),
                    [result.plan_id],
                )
                .expect("corrupt json column");

            let error = load_saved_apply_plan(&connection, result.plan_id)
                .expect_err("malformed JSON should fail closed");
            assert!(error.to_string().contains(column));
        }

        let connection = memory_connection();
        connection
            .execute(
                "INSERT INTO apply_plans (
                    source_plan_kind, title, summary, status, total_items,
                    caveats_json, context_trail_json, plan_provenance_json
                ) VALUES ('client_supplied_preview', 'bad', 'bad', 'preview_only_source', 0, '[]', '[]', '{')",
                [],
            )
            .expect("insert malformed provenance");
        let error = load_saved_apply_plan(&connection, connection.last_insert_rowid())
            .expect_err("malformed provenance should fail closed");
        assert!(error.to_string().contains("plan_provenance_json"));
    }
}
