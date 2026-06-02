use rusqlite::Row;

use crate::{
    error::{AppError, AppResult},
    models::{ApplyPlanListItem, PersistedApplyPlanItemStatus, PersistedApplyPlanStatus},
};

pub(crate) struct PlanRow {
    pub(crate) id: i64,
    pub(crate) source_staging_plan_id: Option<String>,
    pub(crate) source_plan_kind: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) status: String,
    pub(crate) would_touch_files: i64,
    pub(crate) confirmation_required: i64,
    pub(crate) backup_required: i64,
    pub(crate) restore_available: i64,
    pub(crate) total_items: i64,
    pub(crate) applyable_items: i64,
    pub(crate) blocked_items: i64,
    pub(crate) review_only_items: i64,
    pub(crate) caveats_json: String,
    pub(crate) source_scope_json: Option<String>,
    pub(crate) folder_config_json: Option<String>,
    pub(crate) context_trail_json: String,
    pub(crate) plan_hash: Option<String>,
    pub(crate) plan_hash_version: String,
    pub(crate) plan_hash_algorithm: String,
    pub(crate) plan_hash_created_at: Option<String>,
    pub(crate) preview_snapshot_id: Option<i64>,
    pub(crate) preview_snapshot_hash: Option<String>,
    pub(crate) plan_provenance_json: String,
    pub(crate) scan_session_id: Option<i64>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

pub(crate) fn apply_plan_list_item_from_row(row: &Row<'_>) -> rusqlite::Result<ApplyPlanListItem> {
    let status: String = row.get(5)?;
    Ok(ApplyPlanListItem {
        id: row.get(0)?,
        source_staging_plan_id: row.get(1)?,
        source_plan_kind: row.get(2)?,
        title: row.get(3)?,
        summary: row.get(4)?,
        status: parse_plan_status(&status).map_err(to_sql_error)?,
        would_touch_files: int_to_bool(row.get(6)?),
        confirmation_required: int_to_bool(row.get(7)?),
        backup_required: int_to_bool(row.get(8)?),
        restore_available: int_to_bool(row.get(9)?),
        total_items: row.get(10)?,
        applyable_items: row.get(11)?,
        blocked_items: row.get(12)?,
        review_only_items: row.get(13)?,
        plan_hash: row.get(14)?,
        plan_hash_version: row.get(15)?,
        plan_hash_algorithm: row.get(16)?,
        plan_hash_created_at: row.get(17)?,
        preview_snapshot_id: row.get(18)?,
        preview_snapshot_hash: row.get(19)?,
        created_at: row.get(20)?,
        updated_at: row.get(21)?,
    })
}

pub(crate) fn plan_row_from_row(row: &Row<'_>) -> rusqlite::Result<PlanRow> {
    Ok(PlanRow {
        id: row.get(0)?,
        source_staging_plan_id: row.get(1)?,
        source_plan_kind: row.get(2)?,
        title: row.get(3)?,
        summary: row.get(4)?,
        status: row.get(5)?,
        would_touch_files: row.get(6)?,
        confirmation_required: row.get(7)?,
        backup_required: row.get(8)?,
        restore_available: row.get(9)?,
        total_items: row.get(10)?,
        applyable_items: row.get(11)?,
        blocked_items: row.get(12)?,
        review_only_items: row.get(13)?,
        caveats_json: row.get(14)?,
        source_scope_json: row.get(15)?,
        folder_config_json: row.get(16)?,
        context_trail_json: row.get(17)?,
        plan_hash: row.get(18)?,
        plan_hash_version: row.get(19)?,
        plan_hash_algorithm: row.get(20)?,
        plan_hash_created_at: row.get(21)?,
        preview_snapshot_id: row.get(22)?,
        preview_snapshot_hash: row.get(23)?,
        plan_provenance_json: row.get(24)?,
        scan_session_id: row.get(25)?,
        created_at: row.get(26)?,
        updated_at: row.get(27)?,
    })
}

pub(crate) fn parse_plan_status(value: &str) -> AppResult<PersistedApplyPlanStatus> {
    match value {
        "draft" => Ok(PersistedApplyPlanStatus::Draft),
        "preview_only_source" => Ok(PersistedApplyPlanStatus::PreviewOnlySource),
        "blocked" => Ok(PersistedApplyPlanStatus::Blocked),
        "cancelled" => Ok(PersistedApplyPlanStatus::Cancelled),
        _ => Err(AppError::Message(format!(
            "Unsupported ApplyPlan status: {value}"
        ))),
    }
}

pub(crate) fn parse_item_status(value: &str) -> AppResult<PersistedApplyPlanItemStatus> {
    match value {
        "preview_only" => Ok(PersistedApplyPlanItemStatus::PreviewOnly),
        "blocked" => Ok(PersistedApplyPlanItemStatus::Blocked),
        "review_only" => Ok(PersistedApplyPlanItemStatus::ReviewOnly),
        "draft_candidate" => Ok(PersistedApplyPlanItemStatus::DraftCandidate),
        _ => Err(AppError::Message(format!(
            "Unsupported ApplyPlan item status: {value}"
        ))),
    }
}

pub(crate) fn int_to_bool(value: i64) -> bool {
    value != 0
}

fn to_sql_error(error: AppError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::models::PersistedApplyPlanStatus;

    use super::{apply_plan_list_item_from_row, plan_row_from_row};

    #[test]
    fn row_mappers_preserve_status_flags_and_json_columns() {
        let connection = Connection::open_in_memory().expect("memory db");

        let list_item = connection
            .query_row(
                "SELECT 42, 'source-1', 'backend_generated_sorting_preview',
                    'Plan title', 'Plan summary', 'blocked',
                    1, 0, 1, 0,
                    9, 5, 3, 1,
                    'abc123', 'apply_plan_hash_v1', 'sha256', '2026-01-02T03:04:05Z',
                    7, 'snapshot-hash', '2026-01-02T03:04:00Z', '2026-01-02T03:04:06Z'",
                [],
                apply_plan_list_item_from_row,
            )
            .expect("map ApplyPlan list item row");

        assert_eq!(list_item.id, 42);
        assert_eq!(list_item.status, PersistedApplyPlanStatus::Blocked);
        assert!(list_item.would_touch_files);
        assert!(!list_item.confirmation_required);
        assert!(list_item.backup_required);
        assert!(!list_item.restore_available);
        assert_eq!(list_item.plan_hash.as_deref(), Some("abc123"));
        assert_eq!(list_item.preview_snapshot_id, Some(7));

        let plan_row = connection
            .query_row(
                "SELECT 43, 'source-2', 'client_supplied_preview',
                    'Draft title', 'Draft summary', 'preview_only_source',
                    0, 1, 0, 1,
                    8, 4, 2, 2,
                    '[\"caveat\"]', '{\"scope\":true}', '{\"folder\":true}', '[]',
                    NULL, 'apply_plan_hash_v1', 'sha256', NULL,
                    NULL, NULL,
                    '{\"provenance\":true}', 11,
                    '2026-01-03T03:04:00Z', '2026-01-03T03:04:06Z'",
                [],
                plan_row_from_row,
            )
            .expect("map full ApplyPlan row");

        assert_eq!(plan_row.id, 43);
        assert_eq!(
            plan_row.source_scope_json.as_deref(),
            Some("{\"scope\":true}")
        );
        assert_eq!(
            plan_row.folder_config_json.as_deref(),
            Some("{\"folder\":true}")
        );
        assert_eq!(plan_row.context_trail_json, "[]");
        assert_eq!(plan_row.plan_provenance_json, "{\"provenance\":true}");
        assert_eq!(plan_row.scan_session_id, Some(11));
    }
}
