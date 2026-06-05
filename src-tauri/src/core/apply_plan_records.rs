use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    core::apply_plan_persistence_rows,
    error::{AppError, AppResult},
    models::{
        ApplyPlanListItem, DeleteDraftApplyPlanResult, ListSavedApplyPlansRequest,
        PersistedApplyPlanStatus,
    },
};

pub(crate) fn list_saved_apply_plans(
    connection: &Connection,
    request: ListSavedApplyPlansRequest,
) -> AppResult<Vec<ApplyPlanListItem>> {
    let limit = request.limit.unwrap_or(50).clamp(1, 200);
    let include_cancelled = request.include_cancelled.unwrap_or(false);
    let sql = if include_cancelled {
        "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
            would_touch_files, confirmation_required, backup_required, restore_available,
            total_items, applyable_items, blocked_items, review_only_items,
            plan_hash, plan_hash_version, plan_hash_algorithm, plan_hash_created_at,
            preview_snapshot_id, preview_snapshot_hash,
            created_at, updated_at
         FROM apply_plans
         ORDER BY created_at DESC, id DESC
         LIMIT ?1"
    } else {
        "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
            would_touch_files, confirmation_required, backup_required, restore_available,
            total_items, applyable_items, blocked_items, review_only_items,
            plan_hash, plan_hash_version, plan_hash_algorithm, plan_hash_created_at,
            preview_snapshot_id, preview_snapshot_hash,
            created_at, updated_at
         FROM apply_plans
         WHERE status != 'cancelled'
         ORDER BY created_at DESC, id DESC
         LIMIT ?1"
    };

    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(
            params![limit],
            apply_plan_persistence_rows::apply_plan_list_item_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

pub(crate) fn get_apply_plan_list_item(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<Option<ApplyPlanListItem>> {
    connection
        .query_row(
            "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
                would_touch_files, confirmation_required, backup_required, restore_available,
                total_items, applyable_items, blocked_items, review_only_items,
                plan_hash, plan_hash_version, plan_hash_algorithm, plan_hash_created_at,
                preview_snapshot_id, preview_snapshot_hash,
                created_at, updated_at
             FROM apply_plans
             WHERE id = ?1",
            params![plan_id],
            apply_plan_persistence_rows::apply_plan_list_item_from_row,
        )
        .optional()
        .map_err(AppError::from)
}

pub(crate) fn update_apply_plan_status(
    connection: &Connection,
    plan_id: i64,
    status: PersistedApplyPlanStatus,
    updated_at: &str,
) -> AppResult<usize> {
    let rows = connection.execute(
        "UPDATE apply_plans SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![plan_status_value(&status), updated_at, plan_id],
    )?;
    Ok(rows)
}

pub(crate) fn cancel_draft_apply_plan(
    connection: &mut Connection,
    plan_id: i64,
) -> AppResult<DeleteDraftApplyPlanResult> {
    let status: Option<String> = connection
        .query_row(
            "SELECT status FROM apply_plans WHERE id = ?1",
            params![plan_id],
            |row| row.get(0),
        )
        .optional()?;

    let Some(status) = status else {
        return Err(AppError::Message(format!(
            "Draft ApplyPlan {plan_id} was not found"
        )));
    };
    let status = apply_plan_persistence_rows::parse_plan_status(&status)?;
    if !is_cancellable_apply_plan_status(&status) {
        return Err(AppError::Message(
            "Only draft or preview-only ApplyPlan records can be cancelled".to_owned(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    update_apply_plan_status(
        connection,
        plan_id,
        PersistedApplyPlanStatus::Cancelled,
        &now,
    )?;

    Ok(DeleteDraftApplyPlanResult {
        plan_id,
        cancelled: true,
        status: PersistedApplyPlanStatus::Cancelled,
    })
}

pub(crate) fn plan_status_value(status: &PersistedApplyPlanStatus) -> &'static str {
    match status {
        PersistedApplyPlanStatus::Draft => "draft",
        PersistedApplyPlanStatus::PreviewOnlySource => "preview_only_source",
        PersistedApplyPlanStatus::Blocked => "blocked",
        PersistedApplyPlanStatus::Cancelled => "cancelled",
    }
}

fn is_cancellable_apply_plan_status(status: &PersistedApplyPlanStatus) -> bool {
    matches!(
        status,
        PersistedApplyPlanStatus::Draft
            | PersistedApplyPlanStatus::PreviewOnlySource
            | PersistedApplyPlanStatus::Blocked
            | PersistedApplyPlanStatus::Cancelled
    )
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        core::{apply_plan_loading, apply_plan_provenance, apply_plan_saves},
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            ListSavedApplyPlansRequest, PersistedApplyPlanStatus, SaveApplyPlanPreviewRequest,
            StagingPlan,
        },
    };

    use super::{cancel_draft_apply_plan, list_saved_apply_plans, update_apply_plan_status};

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

    fn save_sample_plan(connection: &mut Connection) -> i64 {
        apply_plan_saves::save_apply_plan_preview_draft(
            connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
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
        .expect("save")
        .plan_id
    }

    #[test]
    fn helper_cancels_draft_without_rewriting_immutable_plan_hash() {
        let mut connection = memory_connection();
        let plan_id = save_sample_plan(&mut connection);
        let before = apply_plan_loading::load_saved_apply_plan(&connection, plan_id)
            .expect("get")
            .expect("saved")
            .plan_hash;

        cancel_draft_apply_plan(&mut connection, plan_id).expect("cancel draft");

        let after = apply_plan_loading::load_saved_apply_plan(&connection, plan_id)
            .expect("get after cancel")
            .expect("saved after cancel")
            .plan_hash;
        assert_eq!(before, after);
    }

    #[test]
    fn helper_updates_status_timestamp_without_rewriting_hash() {
        let mut connection = memory_connection();
        let plan_id = save_sample_plan(&mut connection);
        let before = apply_plan_loading::load_saved_apply_plan(&connection, plan_id)
            .expect("get")
            .expect("saved");
        let before_hash = before.plan_hash.clone();
        let before_hash_created_at = before.plan_hash_created_at.clone();

        update_apply_plan_status(
            &connection,
            plan_id,
            PersistedApplyPlanStatus::Cancelled,
            "2026-06-01T10:00:00Z",
        )
        .expect("update status");

        let after = apply_plan_loading::load_saved_apply_plan(&connection, plan_id)
            .expect("get after update")
            .expect("saved after update");
        assert_eq!(after.status, PersistedApplyPlanStatus::Cancelled);
        assert_eq!(after.updated_at, "2026-06-01T10:00:00Z");
        assert_eq!(after.plan_hash, before_hash);
        assert_eq!(after.plan_hash_created_at, before_hash_created_at);
    }

    #[test]
    fn helper_list_summaries_exclude_paths_and_cancelled_plans_by_default() {
        let mut connection = memory_connection();
        let plan_id = save_sample_plan(&mut connection);

        let plans = list_saved_apply_plans(&connection, ListSavedApplyPlansRequest::default())
            .expect("list");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].id, plan_id);

        let summary_json = serde_json::to_string(&plans[0]).expect("summary json");
        assert!(!summary_json.contains("C:/Sims/Mods"));

        let cancelled = cancel_draft_apply_plan(&mut connection, plan_id).expect("cancel draft");
        assert!(cancelled.cancelled);

        let visible = list_saved_apply_plans(&connection, ListSavedApplyPlansRequest::default())
            .expect("list visible");
        assert!(visible.is_empty());

        let with_cancelled = list_saved_apply_plans(
            &connection,
            ListSavedApplyPlansRequest {
                include_cancelled: Some(true),
                limit: Some(10),
            },
        )
        .expect("list cancelled");
        assert_eq!(with_cancelled.len(), 1);
        assert_eq!(
            with_cancelled[0].status,
            PersistedApplyPlanStatus::Cancelled
        );
    }
}
