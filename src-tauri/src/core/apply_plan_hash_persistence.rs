use rusqlite::{params, Connection};

use crate::{
    core::apply_plan_provenance::{self, ApplyPlanHashStamp},
    error::AppResult,
    models::PersistedApplyPlan,
};

pub(crate) fn build_and_store_apply_plan_hash_stamp(
    connection: &Connection,
    plan: &PersistedApplyPlan,
    plan_hash_created_at: &str,
) -> AppResult<ApplyPlanHashStamp> {
    let hash_stamp = apply_plan_provenance::build_hash_stamp(connection, plan)?;
    store_apply_plan_hash_stamp(connection, plan.id, &hash_stamp, plan_hash_created_at)?;
    Ok(hash_stamp)
}

pub(crate) fn store_apply_plan_hash_stamp(
    connection: &Connection,
    plan_id: i64,
    hash_stamp: &ApplyPlanHashStamp,
    plan_hash_created_at: &str,
) -> AppResult<()> {
    let plan_provenance_json = serde_json::to_string(&hash_stamp.provenance)?;
    connection.execute(
        "UPDATE apply_plans
         SET plan_hash = ?1,
             plan_hash_version = ?2,
             plan_hash_algorithm = ?3,
             plan_hash_created_at = ?4,
             plan_provenance_json = ?5
         WHERE id = ?6",
        params![
            hash_stamp.hash,
            hash_stamp.version,
            hash_stamp.algorithm,
            plan_hash_created_at,
            plan_provenance_json,
            plan_id,
        ],
    )?;
    Ok(())
}

pub(crate) fn verify_loaded_apply_plan_hash(
    connection: &Connection,
    plan: &PersistedApplyPlan,
) -> AppResult<apply_plan_provenance::ApplyPlanHashVerification> {
    apply_plan_provenance::verify_hash(connection, plan)
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::{apply_plan_loading, apply_plan_provenance, apply_plan_saves},
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            SaveApplyPlanPreviewRequest, StagingPlan,
        },
    };

    use super::{store_apply_plan_hash_stamp, verify_loaded_apply_plan_hash};

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

    fn save_sample_plan(
        connection: &mut Connection,
        source_scope: Option<serde_json::Value>,
    ) -> i64 {
        apply_plan_saves::save_apply_plan_preview_draft(
            connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: Some("organize".to_owned()),
                source_scope,
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
            None,
            None,
        )
        .expect("save")
        .plan_id
    }

    fn verify_plan_hash(
        connection: &Connection,
        plan_id: i64,
    ) -> apply_plan_provenance::ApplyPlanHashVerification {
        let plan = apply_plan_loading::load_saved_apply_plan(connection, plan_id)
            .expect("load plan for verification")
            .expect("saved plan exists");
        verify_loaded_apply_plan_hash(connection, &plan).expect("verify plan hash")
    }

    #[test]
    fn helper_persists_hash_columns_without_rewriting_updated_at() {
        let connection = memory_connection();
        connection
            .execute(
                "INSERT INTO apply_plans (
                    source_plan_kind, title, summary, status, total_items,
                    caveats_json, context_trail_json, plan_provenance_json,
                    created_at, updated_at
                ) VALUES ('client_supplied_preview', 'hash title', 'hash summary',
                    'preview_only_source', 0, '[]', '[]', '{}',
                    '2026-01-07T03:04:00Z', '2026-01-07T03:04:06Z')",
                [],
            )
            .expect("insert ApplyPlan shell");
        let plan_id = connection.last_insert_rowid();
        let stamp = apply_plan_provenance::ApplyPlanHashStamp {
            hash: "f".repeat(64),
            version: "apply_plan_hash_v1".to_owned(),
            algorithm: "sha256".to_owned(),
            provenance: serde_json::json!({
                "planHashVersion": "apply_plan_hash_v1",
                "stable": true,
            }),
        };

        store_apply_plan_hash_stamp(&connection, plan_id, &stamp, "2026-01-07T03:05:00Z")
            .expect("store hash stamp");

        let row = connection
            .query_row(
                "SELECT plan_hash, plan_hash_version, plan_hash_algorithm,
                    plan_hash_created_at, plan_provenance_json, updated_at
                 FROM apply_plans
                 WHERE id = ?1",
                [plan_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .expect("hash columns");

        assert_eq!(row.0.as_deref(), Some(stamp.hash.as_str()));
        assert_eq!(row.1, stamp.version);
        assert_eq!(row.2, stamp.algorithm);
        assert_eq!(row.3.as_deref(), Some("2026-01-07T03:05:00Z"));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&row.4).expect("stored provenance JSON"),
            stamp.provenance
        );
        assert_eq!(row.5, "2026-01-07T03:04:06Z");
    }

    #[test]
    fn helper_verification_detects_tampered_context_and_destination() {
        let mut connection = memory_connection();
        let plan_id = save_sample_plan(
            &mut connection,
            Some(serde_json::json!({"kind": "selected_files"})),
        );

        assert!(verify_plan_hash(&connection, plan_id).is_valid);

        connection
            .execute(
                "UPDATE apply_plans SET context_trail_json = '[{\"sourceSystem\":\"test\",\"signalKind\":\"tamper\",\"label\":\"Tampered\",\"value\":null,\"strength\":\"evidence\"}]' WHERE id = ?1",
                [plan_id],
            )
            .expect("tamper context");
        let context_check = verify_plan_hash(&connection, plan_id);
        assert!(!context_check.is_valid);
        assert_eq!(context_check.status, "mismatch");

        connection
            .execute(
                "UPDATE apply_plans SET context_trail_json = '[]' WHERE id = ?1",
                [plan_id],
            )
            .expect("restore context");
        connection
            .execute(
                "UPDATE apply_plan_items SET destination_path = 'C:/Sims/Mods/Tampered/a.package' WHERE apply_plan_id = ?1 AND destination_path IS NOT NULL",
                [plan_id],
            )
            .expect("tamper destination");
        let destination_check = verify_plan_hash(&connection, plan_id);
        assert!(!destination_check.is_valid);
        assert_eq!(destination_check.status, "mismatch");
    }

    #[test]
    fn helper_verification_detects_same_count_source_scope_tampering() {
        let mut connection = memory_connection();
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                ) VALUES
                (1, 'C:/Sims/Mods/a.package', 'a.package', 'package', 1, '2026-01-01', 'h1', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                (2, 'C:/Sims/Mods/b.package', 'b.package', 'package', 1, '2026-01-01', 'h2', 'Unknown', NULL, 0.10, '[]', '[\"parser_warning\"]', 'mods', 1),
                (999, 'C:/Sims/Mods/other-a.package', 'other-a.package', 'package', 1, '2026-01-01', 'other1', 'CAS', 'Skin', 0.95, '[]', '[]', 'mods', 1),
                (1000, 'C:/Sims/Mods/other-b.package', 'other-b.package', 'package', 1, '2026-01-01', 'other2', 'BuildBuy', NULL, 0.95, '[]', '[]', 'mods', 1)",
                [],
            )
            .expect("files");
        let plan_id = save_sample_plan(
            &mut connection,
            Some(serde_json::json!({"kind": "selected_files", "fileIds": [1, 2]})),
        );
        assert!(verify_plan_hash(&connection, plan_id).is_valid);

        connection
            .execute(
                "UPDATE apply_plans SET source_scope_json = '{\"kind\":\"selected_files\",\"fileIds\":[999,1000]}' WHERE id = ?1",
                [plan_id],
            )
            .expect("tamper source scope");
        let source_scope_check = verify_plan_hash(&connection, plan_id);
        assert!(!source_scope_check.is_valid);
        assert_eq!(source_scope_check.status, "mismatch");
    }

    #[test]
    fn helper_verification_detects_same_count_rule_template_edit() {
        let mut connection = memory_connection();
        connection
            .execute(
                "INSERT INTO rules (rule_name, rule_template, rule_priority, enabled)
                 VALUES ('provenance-test-rule', 'initial template', 1, 1)",
                [],
            )
            .expect("insert active rule");
        let plan_id = save_sample_plan(
            &mut connection,
            Some(serde_json::json!({"kind": "selected_files"})),
        );

        assert!(verify_plan_hash(&connection, plan_id).is_valid);
        connection
            .execute(
                "UPDATE rules
                 SET rule_template = rule_template || ' :: edited for provenance test'
                 WHERE id = (SELECT id FROM rules WHERE enabled = 1 ORDER BY rule_priority ASC, rule_name ASC LIMIT 1)",
                [],
            )
            .expect("edit one active rule");

        let check = verify_plan_hash(&connection, plan_id);
        assert!(!check.is_valid);
        assert_eq!(check.status, "mismatch");
    }
}
