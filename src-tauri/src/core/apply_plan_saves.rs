use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{
    core::{apply_plan_hash_persistence, apply_plan_items, apply_plan_loading, apply_plan_records},
    error::{AppError, AppResult},
    models::{
        PersistedApplyPlanItemStatus, PersistedApplyPlanStatus, SaveApplyPlanPreviewRequest,
        SaveApplyPlanPreviewResult, StagingPlan, StagingPlanStatus,
    },
};

pub(crate) fn save_apply_plan_preview_draft(
    connection: &mut Connection,
    request: SaveApplyPlanPreviewRequest,
    source_plan_kind: &str,
    preview_snapshot_id: Option<i64>,
    preview_snapshot_hash: Option<String>,
) -> AppResult<SaveApplyPlanPreviewResult> {
    let source_plan = request.source_plan;
    reject_file_touching_source_plan(&source_plan)?;

    let source_plan_kind = source_plan_kind.to_owned();
    let source_scope_json = request
        .source_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let folder_config_json = request
        .folder_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let context_trail_json = serde_json::to_string(&request.context_trail)?;
    let caveats_json = serde_json::to_string(&source_plan.caveats)?;
    let now = Utc::now().to_rfc3339();
    let prepared_items = apply_plan_items::prepare_apply_plan_items(&source_plan)?;
    let blocked_items = prepared_items
        .iter()
        .filter(|item| item.item_status == PersistedApplyPlanItemStatus::Blocked)
        .count() as i64;
    let review_only_items = prepared_items
        .iter()
        .filter(|item| {
            item.item_status == PersistedApplyPlanItemStatus::ReviewOnly
                || (item.review_only && item.item_status != PersistedApplyPlanItemStatus::Blocked)
        })
        .count() as i64;
    let status = if source_plan.status == StagingPlanStatus::Blocked || blocked_items > 0 {
        PersistedApplyPlanStatus::Blocked
    } else {
        PersistedApplyPlanStatus::PreviewOnlySource
    };

    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO apply_plans (
            source_staging_plan_id,
            source_plan_kind,
            title,
            summary,
            status,
            would_touch_files,
            confirmation_required,
            backup_required,
            restore_available,
            total_items,
            applyable_items,
            blocked_items,
            review_only_items,
            caveats_json,
            source_scope_json,
            folder_config_json,
            context_trail_json,
            scan_session_id,
            preview_snapshot_id,
            preview_snapshot_hash,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, 1, 0, ?6, 0, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            source_plan.id,
            source_plan_kind,
            source_plan.title,
            source_plan.summary,
            apply_plan_records::plan_status_value(&status),
            prepared_items.len() as i64,
            blocked_items,
            review_only_items,
            caveats_json,
            source_scope_json,
            folder_config_json,
            context_trail_json,
            request.scan_session_id,
            preview_snapshot_id,
            preview_snapshot_hash,
            now,
            now,
        ],
    )?;
    let plan_id = transaction.last_insert_rowid();

    apply_plan_items::insert_apply_plan_items(&transaction, plan_id, prepared_items, &now)?;

    let persisted = apply_plan_loading::load_saved_apply_plan(&transaction, plan_id)?
        .ok_or_else(|| AppError::Message("saved ApplyPlan could not be reloaded".to_owned()))?;
    apply_plan_hash_persistence::build_and_store_apply_plan_hash_stamp(
        &transaction,
        &persisted,
        &now,
    )?;

    transaction.commit()?;
    let plan = apply_plan_records::get_apply_plan_list_item(connection, plan_id)?
        .ok_or_else(|| AppError::Message("saved ApplyPlan could not be reloaded".to_owned()))?;

    Ok(SaveApplyPlanPreviewResult { plan_id, plan })
}

pub(crate) fn reject_file_touching_source_plan(source_plan: &StagingPlan) -> AppResult<()> {
    if source_plan.would_touch_files {
        return Err(AppError::Message(
            "Cannot save an ApplyPlan preview from a source plan that would touch files."
                .to_owned(),
        ));
    }

    if source_plan.items.iter().any(|item| item.would_touch_files) {
        return Err(AppError::Message(
            "Cannot save an ApplyPlan preview from source items that would touch files.".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::{apply_plan_loading, apply_plan_provenance},
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            PersistedApplyPlanItemStatus, SaveApplyPlanPreviewRequest, StagingPlan,
            StagingPlanActionKind,
        },
    };

    use super::save_apply_plan_preview_draft;

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
    fn helper_rejects_source_plan_that_would_touch_files() {
        let mut connection = memory_connection();
        let mut plan = sample_plan();
        plan.would_touch_files = true;

        let error = save_apply_plan_preview_draft(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: plan,
                source_plan_kind: None,
                source_scope: None,
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
            None,
            None,
        )
        .expect_err("file-touching source plan should fail");

        assert!(error.to_string().contains("would touch files"));
    }

    #[test]
    fn helper_rejects_source_item_that_would_touch_files() {
        let mut connection = memory_connection();
        let mut plan = sample_plan();
        plan.items[0].would_touch_files = true;

        let error = save_apply_plan_preview_draft(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: plan,
                source_plan_kind: None,
                source_scope: None,
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
            None,
            None,
        )
        .expect_err("file-touching source item should fail");

        assert!(error
            .to_string()
            .contains("source items that would touch files"));
    }

    #[test]
    fn helper_keeps_draft_candidate_items_non_applyable_in_v1() {
        let mut connection = memory_connection();
        let mut plan = sample_plan();
        plan.items[0].action_kind = StagingPlanActionKind::SuggestMove;

        let result = save_apply_plan_preview_draft(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: plan,
                source_plan_kind: None,
                source_scope: None,
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
            None,
            None,
        )
        .expect("save");

        let saved = apply_plan_loading::load_saved_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved");
        assert_eq!(saved.applyable_items, 0);
        assert!(saved
            .items
            .iter()
            .any(|item| item.item_status == PersistedApplyPlanItemStatus::DraftCandidate));
    }

    #[test]
    fn helper_persists_draft_items_and_hash_metadata() {
        let mut connection = memory_connection();
        let result = save_apply_plan_preview_draft(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: Some("organize".to_owned()),
                source_scope: Some(serde_json::json!({"kind": "direct_helper"})),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND,
            None,
            None,
        )
        .expect("save through helper");

        assert!(result.plan_id > 0);
        assert_eq!(
            result.plan.source_plan_kind,
            apply_plan_provenance::CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND
        );
        assert_eq!(result.plan.total_items, 2);
        assert_eq!(result.plan.applyable_items, 0);

        let saved = apply_plan_loading::load_saved_apply_plan(&connection, result.plan_id)
            .expect("get saved plan")
            .expect("saved plan exists");
        assert_eq!(saved.preview_snapshot_id, None);
        assert_eq!(saved.preview_snapshot_hash, None);
        assert_eq!(
            saved.source_scope,
            Some(serde_json::json!({"kind": "direct_helper"}))
        );
        assert_eq!(saved.items.len(), 2);
        assert!(saved
            .plan_hash
            .as_ref()
            .is_some_and(|hash| hash.len() == 64));
        assert!(saved.plan_hash_created_at.is_some());
        assert_eq!(
            saved
                .plan_provenance
                .get("sourceScope")
                .and_then(|scope| scope.get("kind"))
                .and_then(|value| value.as_str()),
            Some("direct_helper")
        );
    }
}
