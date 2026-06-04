use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::{
    error::{AppError, AppResult},
    models::{
        ApplyPlanRestoreEntryStatus, ApplyPlanResultLogStatus, ApplyPlanRunLogDetail,
        ApplyPlanRunLogStatus, CreateApplyPlanRunLogRequest, ListApplyPlanRestoreEntriesRequest,
        ListApplyPlanResultLogsRequest, ListApplyPlanRunLogsRequest,
        PersistedApplyPlanRestoreEntry, PersistedApplyPlanResult, PersistedApplyPlanRun,
        RecordApplyPlanRestoreEntryRequest, RecordApplyPlanResultLogRequest,
    },
};

const DEFAULT_BACKUP_STRATEGY: &str = "copy_backup_first";

pub fn create_apply_plan_run_log(
    connection: &Connection,
    request: CreateApplyPlanRunLogRequest,
) -> AppResult<PersistedApplyPlanRun> {
    let plan = load_plan_summary(connection, request.apply_plan_id)?;
    if plan.status == "cancelled" {
        return Err(AppError::Message(
            "Cannot create a result-log foundation row for a cancelled ApplyPlan.".to_owned(),
        ));
    }

    let status = parse_run_status(request.status.as_deref().unwrap_or("draft_log"))?;
    ensure_create_run_status_allowed(&status)?;
    let backup_strategy = validate_backup_strategy(request.backup_strategy.as_deref())?;
    let total_items = non_negative_or(request.total_items, plan.total_items, "totalItems")?;
    let skipped_items = non_negative_or(request.skipped_items, 0, "skippedItems")?;
    let failed_items = non_negative_or(request.failed_items, 0, "failedItems")?;
    let summary = trim_optional(request.summary);
    let confirmation_token = trim_optional(request.confirmation_token);
    let now = Utc::now().to_rfc3339();

    connection.execute(
        "INSERT INTO apply_plan_runs (
            apply_plan_id,
            status,
            backup_strategy,
            confirmation_token,
            confirmed_at,
            started_at,
            finished_at,
            total_items,
            skipped_items,
            applied_items,
            failed_items,
            restored_items,
            summary,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, ?5, ?6, 0, ?7, 0, ?8, ?9, ?9)",
        params![
            request.apply_plan_id,
            run_status_value(&status),
            backup_strategy,
            confirmation_token,
            total_items,
            skipped_items,
            failed_items,
            summary,
            now,
        ],
    )?;

    let run_id = connection.last_insert_rowid();
    get_apply_plan_run(connection, run_id)?
        .ok_or_else(|| AppError::Message("ApplyPlan run log could not be reloaded.".to_owned()))
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
pub(crate) fn mark_apply_plan_run_applying(
    connection: &Connection,
    run_id: i64,
    total_items: i64,
) -> AppResult<PersistedApplyPlanRun> {
    if total_items < 0 {
        return Err(AppError::Message(
            "totalItems must be zero or greater for backend run transition.".to_owned(),
        ));
    }
    ensure_run_transition_allowed(connection, run_id)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE apply_plan_runs
         SET status = 'applying',
             confirmed_at = COALESCE(confirmed_at, ?2),
             started_at = COALESCE(started_at, ?2),
             total_items = ?3,
             updated_at = ?2
         WHERE id = ?1",
        params![run_id, now, total_items],
    )?;
    get_apply_plan_run(connection, run_id)?
        .ok_or_else(|| AppError::Message("ApplyPlan run log could not be reloaded.".to_owned()))
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
pub(crate) fn mark_apply_plan_run_applied(
    connection: &Connection,
    run_id: i64,
    applied_items: i64,
    failed_items: i64,
    skipped_items: i64,
    _summary: &str,
) -> AppResult<PersistedApplyPlanRun> {
    ensure_non_negative_transition_count(applied_items, "appliedItems")?;
    ensure_non_negative_transition_count(failed_items, "failedItems")?;
    ensure_non_negative_transition_count(skipped_items, "skippedItems")?;
    ensure_run_transition_allowed(connection, run_id)?;
    let status = if failed_items > 0 {
        "apply_failed"
    } else {
        "applied"
    };
    let total_items = applied_items + failed_items + skipped_items;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE apply_plan_runs
         SET status = ?2,
             finished_at = ?3,
             total_items = ?4,
             skipped_items = ?5,
             applied_items = ?6,
             failed_items = ?7,
             updated_at = ?3
         WHERE id = ?1",
        params![
            run_id,
            status,
            now,
            total_items,
            skipped_items,
            applied_items,
            failed_items,
        ],
    )?;
    get_apply_plan_run(connection, run_id)?
        .ok_or_else(|| AppError::Message("ApplyPlan run log could not be reloaded.".to_owned()))
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
pub(crate) fn mark_apply_plan_run_restored(
    connection: &Connection,
    run_id: i64,
    restored_items: i64,
    _summary: &str,
) -> AppResult<PersistedApplyPlanRun> {
    ensure_non_negative_transition_count(restored_items, "restoredItems")?;
    ensure_run_transition_allowed(connection, run_id)?;
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE apply_plan_runs
         SET status = 'restored',
             restored_items = ?2,
             updated_at = ?3
         WHERE id = ?1",
        params![run_id, restored_items, now],
    )?;
    get_apply_plan_run(connection, run_id)?
        .ok_or_else(|| AppError::Message("ApplyPlan run log could not be reloaded.".to_owned()))
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
pub(crate) fn mark_apply_plan_restore_entry_restored(
    connection: &Connection,
    restore_entry_id: i64,
) -> AppResult<PersistedApplyPlanRestoreEntry> {
    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE apply_plan_restore_entries
         SET restore_status = 'restored',
             restored_at = ?2,
             updated_at = ?2
         WHERE id = ?1 AND restore_status = 'not_restored'",
        params![restore_entry_id, now],
    )?;
    get_apply_plan_restore_entry(connection, restore_entry_id)?.ok_or_else(|| {
        AppError::Message("ApplyPlan restore entry could not be reloaded.".to_owned())
    })
}

pub fn list_apply_plan_run_logs(
    connection: &Connection,
    request: ListApplyPlanRunLogsRequest,
) -> AppResult<Vec<PersistedApplyPlanRun>> {
    let limit = request.limit.unwrap_or(50).clamp(1, 200);
    let include_cancelled = request.include_cancelled.unwrap_or(false);

    match (request.apply_plan_id, include_cancelled) {
        (Some(plan_id), true) => query_run_logs(
            connection,
            "SELECT id, apply_plan_id, status, backup_strategy, confirmation_token,
                confirmed_at, started_at, finished_at, total_items, skipped_items,
                applied_items, failed_items, restored_items, summary, created_at, updated_at
             FROM apply_plan_runs
             WHERE apply_plan_id = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
            params![plan_id, limit],
        ),
        (Some(plan_id), false) => query_run_logs(
            connection,
            "SELECT id, apply_plan_id, status, backup_strategy, confirmation_token,
                confirmed_at, started_at, finished_at, total_items, skipped_items,
                applied_items, failed_items, restored_items, summary, created_at, updated_at
             FROM apply_plan_runs
             WHERE apply_plan_id = ?1 AND status != 'cancelled'
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
            params![plan_id, limit],
        ),
        (None, true) => query_run_logs(
            connection,
            "SELECT id, apply_plan_id, status, backup_strategy, confirmation_token,
                confirmed_at, started_at, finished_at, total_items, skipped_items,
                applied_items, failed_items, restored_items, summary, created_at, updated_at
             FROM apply_plan_runs
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
            params![limit],
        ),
        (None, false) => query_run_logs(
            connection,
            "SELECT id, apply_plan_id, status, backup_strategy, confirmation_token,
                confirmed_at, started_at, finished_at, total_items, skipped_items,
                applied_items, failed_items, restored_items, summary, created_at, updated_at
             FROM apply_plan_runs
             WHERE status != 'cancelled'
             ORDER BY created_at DESC, id DESC
             LIMIT ?1",
            params![limit],
        ),
    }
}

pub fn get_apply_plan_run_log(
    connection: &Connection,
    run_id: i64,
) -> AppResult<Option<ApplyPlanRunLogDetail>> {
    let Some(run) = get_apply_plan_run(connection, run_id)? else {
        return Ok(None);
    };
    let results = list_apply_plan_result_logs(
        connection,
        ListApplyPlanResultLogsRequest {
            apply_plan_run_id: run_id,
        },
    )?;
    let restore_entries = list_apply_plan_restore_entries(
        connection,
        ListApplyPlanRestoreEntriesRequest {
            apply_plan_run_id: run_id,
        },
    )?;

    Ok(Some(ApplyPlanRunLogDetail {
        run,
        results,
        restore_entries,
    }))
}

pub fn record_apply_plan_result_log(
    connection: &Connection,
    request: RecordApplyPlanResultLogRequest,
) -> AppResult<PersistedApplyPlanResult> {
    record_apply_plan_result_log_with_policy(connection, request, false)
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
pub(crate) fn record_backend_apply_plan_result_log(
    connection: &Connection,
    request: RecordApplyPlanResultLogRequest,
) -> AppResult<PersistedApplyPlanResult> {
    record_apply_plan_result_log_with_policy(connection, request, true)
}

fn record_apply_plan_result_log_with_policy(
    connection: &Connection,
    request: RecordApplyPlanResultLogRequest,
    allow_backend_execution_statuses: bool,
) -> AppResult<PersistedApplyPlanResult> {
    let run = load_run_reference(connection, request.apply_plan_run_id)?;
    if run.status == "cancelled" {
        return Err(AppError::Message(
            "Cannot record result-log rows for a cancelled ApplyPlan run log.".to_owned(),
        ));
    }
    if let Some(item_id) = request.apply_plan_item_id {
        ensure_item_belongs_to_plan(connection, item_id, run.apply_plan_id)?;
    }

    let result_status = parse_result_status(&request.result_status)?;
    if !allow_backend_execution_statuses {
        ensure_record_result_status_allowed(&result_status)?;
    }
    let operation_kind = require_trimmed(request.operation_kind, "operationKind")?;
    let user_summary = require_trimmed(request.user_summary, "userSummary")?;
    let now = Utc::now().to_rfc3339();

    connection.execute(
        "INSERT INTO apply_plan_results (
            apply_plan_run_id,
            apply_plan_id,
            apply_plan_item_id,
            operation_kind,
            result_status,
            source_path_at_execution,
            destination_path_at_execution,
            backup_path,
            error_code,
            error_message,
            user_summary,
            started_at,
            finished_at,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, NULL, ?12, ?12)",
        params![
            request.apply_plan_run_id,
            run.apply_plan_id,
            request.apply_plan_item_id,
            operation_kind,
            result_status_value(&result_status),
            trim_optional(request.source_path_at_execution),
            trim_optional(request.destination_path_at_execution),
            trim_optional(request.backup_path),
            trim_optional(request.error_code),
            trim_optional(request.error_message),
            user_summary,
            now,
        ],
    )?;

    let result_id = connection.last_insert_rowid();
    get_apply_plan_result(connection, result_id)?
        .ok_or_else(|| AppError::Message("ApplyPlan result log could not be reloaded.".to_owned()))
}

pub fn list_apply_plan_result_logs(
    connection: &Connection,
    request: ListApplyPlanResultLogsRequest,
) -> AppResult<Vec<PersistedApplyPlanResult>> {
    let mut statement = connection.prepare(
        "SELECT id, apply_plan_run_id, apply_plan_id, apply_plan_item_id, operation_kind,
            result_status, source_path_at_execution, destination_path_at_execution, backup_path,
            error_code, error_message, user_summary, started_at, finished_at, created_at, updated_at
         FROM apply_plan_results
         WHERE apply_plan_run_id = ?1
         ORDER BY id ASC",
    )?;
    let rows = statement
        .query_map(
            params![request.apply_plan_run_id],
            apply_plan_result_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

pub fn record_apply_plan_restore_entry(
    connection: &Connection,
    request: RecordApplyPlanRestoreEntryRequest,
) -> AppResult<PersistedApplyPlanRestoreEntry> {
    record_apply_plan_restore_entry_with_policy(connection, request, false)
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
pub(crate) fn record_backend_apply_plan_restore_entry(
    connection: &Connection,
    request: RecordApplyPlanRestoreEntryRequest,
) -> AppResult<PersistedApplyPlanRestoreEntry> {
    record_apply_plan_restore_entry_with_policy(connection, request, true)
}

fn record_apply_plan_restore_entry_with_policy(
    connection: &Connection,
    request: RecordApplyPlanRestoreEntryRequest,
    allow_backend_execution_statuses: bool,
) -> AppResult<PersistedApplyPlanRestoreEntry> {
    let run = load_run_reference(connection, request.apply_plan_run_id)?;
    if run.status == "cancelled" {
        return Err(AppError::Message(
            "Cannot record restore-map rows for a cancelled ApplyPlan run log.".to_owned(),
        ));
    }

    let result_ref = match request.apply_plan_result_id {
        Some(result_id) => Some(load_result_reference(
            connection,
            result_id,
            request.apply_plan_run_id,
        )?),
        None => None,
    };
    if let Some(result_ref) = &result_ref {
        if result_ref.apply_plan_id != run.apply_plan_id {
            return Err(AppError::Message(
                "Result log belongs to a different ApplyPlan.".to_owned(),
            ));
        }
    }

    let apply_plan_item_id = request.apply_plan_item_id.or_else(|| {
        result_ref
            .as_ref()
            .and_then(|result| result.apply_plan_item_id)
    });
    if let (Some(request_item_id), Some(result_ref)) =
        (request.apply_plan_item_id, result_ref.as_ref())
    {
        if let Some(result_item_id) = result_ref.apply_plan_item_id {
            if request_item_id != result_item_id {
                return Err(AppError::Message(
                    "Restore entry item does not match the referenced result log.".to_owned(),
                ));
            }
        }
    }
    if let Some(item_id) = apply_plan_item_id {
        ensure_item_belongs_to_plan(connection, item_id, run.apply_plan_id)?;
    }

    let operation_result_status = parse_result_status(&request.operation_result_status)?;
    let restore_status = parse_restore_status(&request.restore_status)?;
    if !allow_backend_execution_statuses {
        ensure_record_result_status_allowed(&operation_result_status)?;
        ensure_record_restore_status_allowed(&restore_status)?;
    }
    let original_source_path = require_trimmed(request.original_source_path, "originalSourcePath")?;
    let operation_kind = require_trimmed(request.operation_kind, "operationKind")?;
    if matches!(request.file_size_before, Some(value) if value < 0) {
        return Err(AppError::Message(
            "fileSizeBefore must be zero or greater when provided.".to_owned(),
        ));
    }
    let now = Utc::now().to_rfc3339();

    connection.execute(
        "INSERT INTO apply_plan_restore_entries (
            apply_plan_run_id,
            apply_plan_result_id,
            apply_plan_id,
            apply_plan_item_id,
            original_source_path,
            destination_path_at_execution,
            backup_path,
            file_hash_before,
            file_size_before,
            operation_kind,
            operation_result_status,
            restore_status,
            restore_error_code,
            restore_error_message,
            created_at,
            updated_at,
            restored_at,
            failed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15, NULL, NULL)",
        params![
            request.apply_plan_run_id,
            request.apply_plan_result_id,
            run.apply_plan_id,
            apply_plan_item_id,
            original_source_path,
            trim_optional(request.destination_path_at_execution),
            trim_optional(request.backup_path),
            trim_optional(request.file_hash_before),
            request.file_size_before,
            operation_kind,
            result_status_value(&operation_result_status),
            restore_status_value(&restore_status),
            trim_optional(request.restore_error_code),
            trim_optional(request.restore_error_message),
            now,
        ],
    )?;

    let entry_id = connection.last_insert_rowid();
    get_apply_plan_restore_entry(connection, entry_id)?.ok_or_else(|| {
        AppError::Message("ApplyPlan restore entry could not be reloaded.".to_owned())
    })
}

pub fn list_apply_plan_restore_entries(
    connection: &Connection,
    request: ListApplyPlanRestoreEntriesRequest,
) -> AppResult<Vec<PersistedApplyPlanRestoreEntry>> {
    let mut statement = connection.prepare(
        "SELECT id, apply_plan_run_id, apply_plan_result_id, apply_plan_id, apply_plan_item_id,
            original_source_path, destination_path_at_execution, backup_path, file_hash_before,
            file_size_before, operation_kind, operation_result_status, restore_status,
            restore_error_code, restore_error_message, created_at, updated_at, restored_at, failed_at
         FROM apply_plan_restore_entries
         WHERE apply_plan_run_id = ?1
         ORDER BY id ASC",
    )?;
    let rows = statement
        .query_map(
            params![request.apply_plan_run_id],
            apply_plan_restore_entry_from_row,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

fn get_apply_plan_run(
    connection: &Connection,
    run_id: i64,
) -> AppResult<Option<PersistedApplyPlanRun>> {
    connection
        .query_row(
            "SELECT id, apply_plan_id, status, backup_strategy, confirmation_token,
                confirmed_at, started_at, finished_at, total_items, skipped_items,
                applied_items, failed_items, restored_items, summary, created_at, updated_at
             FROM apply_plan_runs
             WHERE id = ?1",
            params![run_id],
            apply_plan_run_from_row,
        )
        .optional()
        .map_err(AppError::from)
}

fn get_apply_plan_result(
    connection: &Connection,
    result_id: i64,
) -> AppResult<Option<PersistedApplyPlanResult>> {
    connection
        .query_row(
            "SELECT id, apply_plan_run_id, apply_plan_id, apply_plan_item_id, operation_kind,
                result_status, source_path_at_execution, destination_path_at_execution, backup_path,
                error_code, error_message, user_summary, started_at, finished_at, created_at, updated_at
             FROM apply_plan_results
             WHERE id = ?1",
            params![result_id],
            apply_plan_result_from_row,
        )
        .optional()
        .map_err(AppError::from)
}

fn get_apply_plan_restore_entry(
    connection: &Connection,
    entry_id: i64,
) -> AppResult<Option<PersistedApplyPlanRestoreEntry>> {
    connection
        .query_row(
            "SELECT id, apply_plan_run_id, apply_plan_result_id, apply_plan_id, apply_plan_item_id,
                original_source_path, destination_path_at_execution, backup_path, file_hash_before,
                file_size_before, operation_kind, operation_result_status, restore_status,
                restore_error_code, restore_error_message, created_at, updated_at, restored_at, failed_at
             FROM apply_plan_restore_entries
             WHERE id = ?1",
            params![entry_id],
            apply_plan_restore_entry_from_row,
        )
        .optional()
        .map_err(AppError::from)
}

fn query_run_logs<P>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> AppResult<Vec<PersistedApplyPlanRun>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(params, apply_plan_run_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn apply_plan_run_from_row(row: &Row<'_>) -> rusqlite::Result<PersistedApplyPlanRun> {
    let status: String = row.get(2)?;
    Ok(PersistedApplyPlanRun {
        id: row.get(0)?,
        apply_plan_id: row.get(1)?,
        status: parse_run_status(&status).map_err(to_sql_error)?,
        backup_strategy: row.get(3)?,
        confirmation_token: row.get(4)?,
        confirmed_at: row.get(5)?,
        started_at: row.get(6)?,
        finished_at: row.get(7)?,
        total_items: row.get(8)?,
        skipped_items: row.get(9)?,
        applied_items: row.get(10)?,
        failed_items: row.get(11)?,
        restored_items: row.get(12)?,
        summary: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn apply_plan_result_from_row(row: &Row<'_>) -> rusqlite::Result<PersistedApplyPlanResult> {
    let status: String = row.get(5)?;
    Ok(PersistedApplyPlanResult {
        id: row.get(0)?,
        apply_plan_run_id: row.get(1)?,
        apply_plan_id: row.get(2)?,
        apply_plan_item_id: row.get(3)?,
        operation_kind: row.get(4)?,
        result_status: parse_result_status(&status).map_err(to_sql_error)?,
        source_path_at_execution: row.get(6)?,
        destination_path_at_execution: row.get(7)?,
        backup_path: row.get(8)?,
        error_code: row.get(9)?,
        error_message: row.get(10)?,
        user_summary: row.get(11)?,
        started_at: row.get(12)?,
        finished_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn apply_plan_restore_entry_from_row(
    row: &Row<'_>,
) -> rusqlite::Result<PersistedApplyPlanRestoreEntry> {
    let operation_result_status: String = row.get(11)?;
    let restore_status: String = row.get(12)?;
    Ok(PersistedApplyPlanRestoreEntry {
        id: row.get(0)?,
        apply_plan_run_id: row.get(1)?,
        apply_plan_result_id: row.get(2)?,
        apply_plan_id: row.get(3)?,
        apply_plan_item_id: row.get(4)?,
        original_source_path: row.get(5)?,
        destination_path_at_execution: row.get(6)?,
        backup_path: row.get(7)?,
        file_hash_before: row.get(8)?,
        file_size_before: row.get(9)?,
        operation_kind: row.get(10)?,
        operation_result_status: parse_result_status(&operation_result_status)
            .map_err(to_sql_error)?,
        restore_status: parse_restore_status(&restore_status).map_err(to_sql_error)?,
        restore_error_code: row.get(13)?,
        restore_error_message: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        restored_at: row.get(17)?,
        failed_at: row.get(18)?,
    })
}

fn load_plan_summary(connection: &Connection, plan_id: i64) -> AppResult<PlanReference> {
    connection
        .query_row(
            "SELECT status, total_items FROM apply_plans WHERE id = ?1",
            params![plan_id],
            |row| {
                Ok(PlanReference {
                    status: row.get(0)?,
                    total_items: row.get(1)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Message(format!("Saved ApplyPlan {plan_id} was not found.")))
}

fn load_run_reference(connection: &Connection, run_id: i64) -> AppResult<RunReference> {
    connection
        .query_row(
            "SELECT apply_plan_id, status FROM apply_plan_runs WHERE id = ?1",
            params![run_id],
            |row| {
                Ok(RunReference {
                    apply_plan_id: row.get(0)?,
                    status: row.get(1)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Message(format!("ApplyPlan run log {run_id} was not found.")))
}

fn load_result_reference(
    connection: &Connection,
    result_id: i64,
    run_id: i64,
) -> AppResult<ResultReference> {
    connection
        .query_row(
            "SELECT apply_plan_run_id, apply_plan_id, apply_plan_item_id
             FROM apply_plan_results
             WHERE id = ?1",
            params![result_id],
            |row| {
                Ok(ResultReference {
                    apply_plan_run_id: row.get(0)?,
                    apply_plan_id: row.get(1)?,
                    apply_plan_item_id: row.get(2)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| {
            AppError::Message(format!("ApplyPlan result log {result_id} was not found."))
        })
        .and_then(|result| {
            if result.apply_plan_run_id != run_id {
                Err(AppError::Message(
                    "Result log belongs to a different ApplyPlan run.".to_owned(),
                ))
            } else {
                Ok(result)
            }
        })
}

fn ensure_item_belongs_to_plan(
    connection: &Connection,
    item_id: i64,
    plan_id: i64,
) -> AppResult<()> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM apply_plan_items WHERE id = ?1 AND apply_plan_id = ?2",
            params![item_id, plan_id],
            |_row| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    if !exists {
        return Err(AppError::Message(
            "ApplyPlan item does not belong to the referenced saved plan.".to_owned(),
        ));
    }

    Ok(())
}

fn parse_run_status(value: &str) -> AppResult<ApplyPlanRunLogStatus> {
    match value {
        "draft_log" => Ok(ApplyPlanRunLogStatus::DraftLog),
        "confirmed" => Ok(ApplyPlanRunLogStatus::Confirmed),
        "applying" => Ok(ApplyPlanRunLogStatus::Applying),
        "applied" => Ok(ApplyPlanRunLogStatus::Applied),
        "apply_failed" => Ok(ApplyPlanRunLogStatus::ApplyFailed),
        "restored" => Ok(ApplyPlanRunLogStatus::Restored),
        "restore_failed" => Ok(ApplyPlanRunLogStatus::RestoreFailed),
        "blocked" => Ok(ApplyPlanRunLogStatus::Blocked),
        "cancelled" => Ok(ApplyPlanRunLogStatus::Cancelled),
        _ => Err(AppError::Message(format!(
            "Unsupported DB-only ApplyPlan run status for v1: {value}"
        ))),
    }
}

fn ensure_create_run_status_allowed(status: &ApplyPlanRunLogStatus) -> AppResult<()> {
    if matches!(
        status,
        ApplyPlanRunLogStatus::DraftLog
            | ApplyPlanRunLogStatus::Blocked
            | ApplyPlanRunLogStatus::Cancelled
    ) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "ApplyPlan run status {} can only be set by backend-observed executor transitions.",
            run_status_value(status)
        )))
    }
}

fn ensure_record_result_status_allowed(status: &ApplyPlanResultLogStatus) -> AppResult<()> {
    if matches!(
        status,
        ApplyPlanResultLogStatus::PendingLog
            | ApplyPlanResultLogStatus::Skipped
            | ApplyPlanResultLogStatus::Blocked
            | ApplyPlanResultLogStatus::FailedBeforeChange
    ) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "ApplyPlan result status {} can only be set by backend-observed executor transitions.",
            result_status_value(status)
        )))
    }
}

fn ensure_record_restore_status_allowed(status: &ApplyPlanRestoreEntryStatus) -> AppResult<()> {
    if matches!(
        status,
        ApplyPlanRestoreEntryStatus::NotAvailable
            | ApplyPlanRestoreEntryStatus::DesignOnly
            | ApplyPlanRestoreEntryStatus::NotRestored
    ) {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "ApplyPlan restore status {} can only be set by backend-observed executor transitions.",
            restore_status_value(status)
        )))
    }
}

fn parse_result_status(value: &str) -> AppResult<ApplyPlanResultLogStatus> {
    match value {
        "pending_log" => Ok(ApplyPlanResultLogStatus::PendingLog),
        "skipped" => Ok(ApplyPlanResultLogStatus::Skipped),
        "blocked" => Ok(ApplyPlanResultLogStatus::Blocked),
        "failed_before_change" => Ok(ApplyPlanResultLogStatus::FailedBeforeChange),
        "applied" => Ok(ApplyPlanResultLogStatus::Applied),
        "failed_after_change" => Ok(ApplyPlanResultLogStatus::FailedAfterChange),
        "restored" => Ok(ApplyPlanResultLogStatus::Restored),
        _ => Err(AppError::Message(format!(
            "Unsupported DB-only ApplyPlan result status for v1: {value}"
        ))),
    }
}

fn parse_restore_status(value: &str) -> AppResult<ApplyPlanRestoreEntryStatus> {
    match value {
        "not_available" => Ok(ApplyPlanRestoreEntryStatus::NotAvailable),
        "design_only" => Ok(ApplyPlanRestoreEntryStatus::DesignOnly),
        "not_restored" => Ok(ApplyPlanRestoreEntryStatus::NotRestored),
        "restored" => Ok(ApplyPlanRestoreEntryStatus::Restored),
        "restore_failed" => Ok(ApplyPlanRestoreEntryStatus::RestoreFailed),
        _ => Err(AppError::Message(format!(
            "Unsupported DB-only ApplyPlan restore status for v1: {value}"
        ))),
    }
}

fn run_status_value(status: &ApplyPlanRunLogStatus) -> &'static str {
    match status {
        ApplyPlanRunLogStatus::DraftLog => "draft_log",
        ApplyPlanRunLogStatus::Confirmed => "confirmed",
        ApplyPlanRunLogStatus::Applying => "applying",
        ApplyPlanRunLogStatus::Applied => "applied",
        ApplyPlanRunLogStatus::ApplyFailed => "apply_failed",
        ApplyPlanRunLogStatus::Restored => "restored",
        ApplyPlanRunLogStatus::RestoreFailed => "restore_failed",
        ApplyPlanRunLogStatus::Blocked => "blocked",
        ApplyPlanRunLogStatus::Cancelled => "cancelled",
    }
}

fn result_status_value(status: &ApplyPlanResultLogStatus) -> &'static str {
    match status {
        ApplyPlanResultLogStatus::PendingLog => "pending_log",
        ApplyPlanResultLogStatus::Skipped => "skipped",
        ApplyPlanResultLogStatus::Blocked => "blocked",
        ApplyPlanResultLogStatus::FailedBeforeChange => "failed_before_change",
        ApplyPlanResultLogStatus::Applied => "applied",
        ApplyPlanResultLogStatus::FailedAfterChange => "failed_after_change",
        ApplyPlanResultLogStatus::Restored => "restored",
    }
}

fn restore_status_value(status: &ApplyPlanRestoreEntryStatus) -> &'static str {
    match status {
        ApplyPlanRestoreEntryStatus::NotAvailable => "not_available",
        ApplyPlanRestoreEntryStatus::DesignOnly => "design_only",
        ApplyPlanRestoreEntryStatus::NotRestored => "not_restored",
        ApplyPlanRestoreEntryStatus::Restored => "restored",
        ApplyPlanRestoreEntryStatus::RestoreFailed => "restore_failed",
    }
}

fn validate_backup_strategy(value: Option<&str>) -> AppResult<String> {
    let value = value.unwrap_or(DEFAULT_BACKUP_STRATEGY).trim();
    match value {
        "copy_backup_first" | "design_only" => Ok(value.to_owned()),
        _ => Err(AppError::Message(format!(
            "Unsupported DB-only ApplyPlan backup strategy for v1: {value}"
        ))),
    }
}

fn non_negative_or(value: Option<i64>, default: i64, field_name: &str) -> AppResult<i64> {
    let value = value.unwrap_or(default);
    if value < 0 {
        Err(AppError::Message(format!(
            "{field_name} must be zero or greater."
        )))
    } else {
        Ok(value)
    }
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
fn ensure_non_negative_transition_count(value: i64, field_name: &str) -> AppResult<()> {
    if value < 0 {
        Err(AppError::Message(format!(
            "{field_name} must be zero or greater for backend run transition."
        )))
    } else {
        Ok(())
    }
}

#[cfg_attr(not(feature = "apply-executor-real-move-spike"), allow(dead_code))]
fn ensure_run_transition_allowed(connection: &Connection, run_id: i64) -> AppResult<()> {
    let run = load_run_reference(connection, run_id)?;
    if run.status == "cancelled" {
        Err(AppError::Message(
            "Cannot transition a cancelled ApplyPlan run log.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn require_trimmed(value: String, field_name: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(AppError::Message(format!("{field_name} is required.")))
    } else {
        Ok(trimmed.to_owned())
    }
}

fn trim_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_owned();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn to_sql_error(error: AppError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

struct PlanReference {
    status: String,
    total_items: i64,
}

struct RunReference {
    apply_plan_id: i64,
    status: String,
}

struct ResultReference {
    apply_plan_run_id: i64,
    apply_plan_id: i64,
    apply_plan_item_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use crate::{core::apply_plan_persistence, database};

    use super::*;

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn insert_saved_plan(connection: &Connection, status: &str) -> i64 {
        connection
            .execute(
                "INSERT INTO apply_plans (
                    source_plan_kind, title, summary, status, would_touch_files,
                    confirmation_required, backup_required, restore_available, total_items,
                    applyable_items, blocked_items, review_only_items, caveats_json,
                    created_at, updated_at
                ) VALUES ('test', 'Test plan', 'DB-only test plan', ?1, 0, 1, 1, 0, 2, 0, 1, 1, '[]', '2026-01-01', '2026-01-01')",
                params![status],
            )
            .expect("insert plan");
        connection.last_insert_rowid()
    }

    fn insert_plan_item(connection: &Connection, plan_id: i64) -> i64 {
        connection
            .execute(
                "INSERT INTO apply_plan_items (
                    apply_plan_id, file_name, current_path, current_root, destination_path,
                    destination_root, action_kind, evidence_level, bucket, confidence_label,
                    item_status, blocked, review_only, path_privacy_level, created_at, updated_at
                ) VALUES (?1, 'sample.package', 'C:/Sims/Mods/sample.package', 'mods',
                    'C:/Sims/Mods/CAS/sample.package', 'mods', 'suggest_move', 'strong',
                    'cas', 'high', 'draft_candidate', 0, 0, 'local_full_path_required',
                    '2026-01-01', '2026-01-01')",
                params![plan_id],
            )
            .expect("insert item");
        connection.last_insert_rowid()
    }

    fn create_run(connection: &Connection, plan_id: i64) -> PersistedApplyPlanRun {
        create_apply_plan_run_log(
            connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: None,
                skipped_items: None,
                failed_items: None,
                summary: Some("Foundation log only.".to_owned()),
            },
        )
        .expect("create run")
    }

    #[test]
    fn create_list_and_get_run_log_is_db_only() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");

        let run = create_run(&connection, plan_id);
        assert_eq!(run.apply_plan_id, plan_id);
        assert_eq!(run.status, ApplyPlanRunLogStatus::DraftLog);
        assert_eq!(run.backup_strategy, "copy_backup_first");
        assert_eq!(run.applied_items, 0);
        assert_eq!(run.restored_items, 0);

        let runs = list_apply_plan_run_logs(
            &connection,
            ListApplyPlanRunLogsRequest {
                apply_plan_id: Some(plan_id),
                include_cancelled: None,
                limit: Some(10),
            },
        )
        .expect("list runs");
        assert_eq!(runs.len(), 1);

        let detail = get_apply_plan_run_log(&connection, run.id)
            .expect("get run")
            .expect("run detail");
        assert_eq!(detail.run.id, run.id);
        assert!(detail.results.is_empty());
        assert!(detail.restore_entries.is_empty());
    }

    #[test]
    fn run_log_rejects_cancelled_plan_and_future_strategy() {
        let connection = memory_connection();
        let cancelled_plan_id = insert_saved_plan(&connection, "cancelled");
        let error = create_apply_plan_run_log(
            &connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: cancelled_plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: None,
                skipped_items: None,
                failed_items: None,
                summary: None,
            },
        )
        .expect_err("cancelled plan rejected");
        assert!(error.to_string().contains("cancelled ApplyPlan"));

        let active_plan_id = insert_saved_plan(&connection, "preview_only_source");
        let error = create_apply_plan_run_log(
            &connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: active_plan_id,
                status: Some("draft_log".to_owned()),
                backup_strategy: Some("journal_only".to_owned()),
                confirmation_token: None,
                total_items: None,
                skipped_items: None,
                failed_items: None,
                summary: None,
            },
        )
        .expect_err("journal strategy rejected");
        assert!(error.to_string().contains("backup strategy"));
    }

    #[test]
    fn run_log_rejects_execution_statuses() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");

        for status in ["confirmed", "applying", "applied", "restored"] {
            let error = create_apply_plan_run_log(
                &connection,
                CreateApplyPlanRunLogRequest {
                    apply_plan_id: plan_id,
                    status: Some(status.to_owned()),
                    backup_strategy: None,
                    confirmation_token: None,
                    total_items: None,
                    skipped_items: None,
                    failed_items: None,
                    summary: None,
                },
            )
            .expect_err("execution status rejected");
            assert!(error.to_string().contains(status));
        }
    }

    #[test]
    fn result_logs_accept_only_non_execution_statuses() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");
        let item_id = insert_plan_item(&connection, plan_id);
        let run = create_run(&connection, plan_id);

        for status in ["pending_log", "skipped", "blocked", "failed_before_change"] {
            let result = record_apply_plan_result_log(
                &connection,
                RecordApplyPlanResultLogRequest {
                    apply_plan_run_id: run.id,
                    apply_plan_item_id: Some(item_id),
                    operation_kind: "future_move_preview".to_owned(),
                    result_status: status.to_owned(),
                    source_path_at_execution: None,
                    destination_path_at_execution: None,
                    backup_path: None,
                    error_code: None,
                    error_message: None,
                    user_summary: format!("Recorded {status} without changing files."),
                },
            )
            .expect("record result");
            assert_eq!(result.apply_plan_run_id, run.id);
            assert_eq!(result.apply_plan_item_id, Some(item_id));
        }

        let results = list_apply_plan_result_logs(
            &connection,
            ListApplyPlanResultLogsRequest {
                apply_plan_run_id: run.id,
            },
        )
        .expect("list results");
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn result_log_rejects_applied_status_and_empty_summary() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");
        let run = create_run(&connection, plan_id);

        let error = record_apply_plan_result_log(
            &connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: run.id,
                apply_plan_item_id: None,
                operation_kind: "future_move_preview".to_owned(),
                result_status: "applied".to_owned(),
                source_path_at_execution: None,
                destination_path_at_execution: None,
                backup_path: None,
                error_code: None,
                error_message: None,
                user_summary: "Should not save.".to_owned(),
            },
        )
        .expect_err("applied rejected");
        assert!(error.to_string().contains("applied"));

        let error = record_apply_plan_result_log(
            &connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: run.id,
                apply_plan_item_id: None,
                operation_kind: "future_move_preview".to_owned(),
                result_status: "blocked".to_owned(),
                source_path_at_execution: None,
                destination_path_at_execution: None,
                backup_path: None,
                error_code: None,
                error_message: None,
                user_summary: " ".to_owned(),
            },
        )
        .expect_err("summary required");
        assert!(error.to_string().contains("userSummary"));
    }

    #[test]
    fn restore_entries_accept_only_design_safe_statuses() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");
        let item_id = insert_plan_item(&connection, plan_id);
        let run = create_run(&connection, plan_id);
        let result = record_apply_plan_result_log(
            &connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: run.id,
                apply_plan_item_id: Some(item_id),
                operation_kind: "future_move_preview".to_owned(),
                result_status: "blocked".to_owned(),
                source_path_at_execution: None,
                destination_path_at_execution: None,
                backup_path: None,
                error_code: None,
                error_message: None,
                user_summary: "Blocked before any file change.".to_owned(),
            },
        )
        .expect("record result");

        for status in ["not_available", "design_only", "not_restored"] {
            let entry = record_apply_plan_restore_entry(
                &connection,
                RecordApplyPlanRestoreEntryRequest {
                    apply_plan_run_id: run.id,
                    apply_plan_result_id: Some(result.id),
                    apply_plan_item_id: Some(item_id),
                    original_source_path: "C:/Sims/Mods/sample.package".to_owned(),
                    destination_path_at_execution: None,
                    backup_path: None,
                    file_hash_before: None,
                    file_size_before: Some(10),
                    operation_kind: "future_move_preview".to_owned(),
                    operation_result_status: "blocked".to_owned(),
                    restore_status: status.to_owned(),
                    restore_error_code: None,
                    restore_error_message: None,
                },
            )
            .expect("record restore entry");
            assert_eq!(entry.apply_plan_run_id, run.id);
            assert_eq!(entry.apply_plan_result_id, Some(result.id));
        }

        let entries = list_apply_plan_restore_entries(
            &connection,
            ListApplyPlanRestoreEntriesRequest {
                apply_plan_run_id: run.id,
            },
        )
        .expect("list entries");
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn restore_entry_rejects_restored_status_and_mismatched_result() {
        let connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");
        let item_id = insert_plan_item(&connection, plan_id);
        let run = create_run(&connection, plan_id);
        let other_plan_id = insert_saved_plan(&connection, "preview_only_source");
        let other_run = create_run(&connection, other_plan_id);
        let other_result = record_apply_plan_result_log(
            &connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: other_run.id,
                apply_plan_item_id: None,
                operation_kind: "future_move_preview".to_owned(),
                result_status: "blocked".to_owned(),
                source_path_at_execution: None,
                destination_path_at_execution: None,
                backup_path: None,
                error_code: None,
                error_message: None,
                user_summary: "Other run.".to_owned(),
            },
        )
        .expect("other result");

        let error = record_apply_plan_restore_entry(
            &connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: run.id,
                apply_plan_result_id: None,
                apply_plan_item_id: Some(item_id),
                original_source_path: "C:/Sims/Mods/sample.package".to_owned(),
                destination_path_at_execution: None,
                backup_path: None,
                file_hash_before: None,
                file_size_before: None,
                operation_kind: "future_move_preview".to_owned(),
                operation_result_status: "blocked".to_owned(),
                restore_status: "restored".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )
        .expect_err("restored rejected");
        assert!(error.to_string().contains("restored"));

        let error = record_apply_plan_restore_entry(
            &connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: run.id,
                apply_plan_result_id: Some(other_result.id),
                apply_plan_item_id: None,
                original_source_path: "C:/Sims/Mods/sample.package".to_owned(),
                destination_path_at_execution: None,
                backup_path: None,
                file_hash_before: None,
                file_size_before: None,
                operation_kind: "future_move_preview".to_owned(),
                operation_result_status: "blocked".to_owned(),
                restore_status: "design_only".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )
        .expect_err("mismatched result rejected");
        assert!(error.to_string().contains("different ApplyPlan run"));
    }

    #[test]
    fn cancelling_draft_plan_does_not_remove_result_restore_rows() {
        let mut connection = memory_connection();
        let plan_id = insert_saved_plan(&connection, "preview_only_source");
        let item_id = insert_plan_item(&connection, plan_id);
        let run = create_run(&connection, plan_id);
        let result = record_apply_plan_result_log(
            &connection,
            RecordApplyPlanResultLogRequest {
                apply_plan_run_id: run.id,
                apply_plan_item_id: Some(item_id),
                operation_kind: "future_move_preview".to_owned(),
                result_status: "blocked".to_owned(),
                source_path_at_execution: None,
                destination_path_at_execution: None,
                backup_path: None,
                error_code: None,
                error_message: None,
                user_summary: "Blocked before any file change.".to_owned(),
            },
        )
        .expect("record result");
        record_apply_plan_restore_entry(
            &connection,
            RecordApplyPlanRestoreEntryRequest {
                apply_plan_run_id: run.id,
                apply_plan_result_id: Some(result.id),
                apply_plan_item_id: Some(item_id),
                original_source_path: "C:/Sims/Mods/sample.package".to_owned(),
                destination_path_at_execution: None,
                backup_path: None,
                file_hash_before: None,
                file_size_before: None,
                operation_kind: "future_move_preview".to_owned(),
                operation_result_status: "blocked".to_owned(),
                restore_status: "design_only".to_owned(),
                restore_error_code: None,
                restore_error_message: None,
            },
        )
        .expect("record restore entry");

        apply_plan_persistence::delete_draft_apply_plan(&mut connection, plan_id)
            .expect("soft cancel plan");

        assert_eq!(
            list_apply_plan_result_logs(
                &connection,
                ListApplyPlanResultLogsRequest {
                    apply_plan_run_id: run.id,
                },
            )
            .expect("list results")
            .len(),
            1
        );
        assert_eq!(
            list_apply_plan_restore_entries(
                &connection,
                ListApplyPlanRestoreEntriesRequest {
                    apply_plan_run_id: run.id,
                },
            )
            .expect("list restore entries")
            .len(),
            1
        );
    }

    #[test]
    fn apply_plan_results_module_does_not_call_file_changing_helpers() {
        let source = include_str!("apply_plan_results.rs");
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");

        for forbidden in [
            "std::fs",
            "fs::",
            "move_engine",
            "snapshot_manager",
            "downloads_watcher",
            "apply_preview",
            "commit_staging",
            "cleanup_staging",
            "apply_download",
            "reject_download",
            "restore_rejected",
            "restore_snapshot",
            "undo_applied",
            "create_dir",
            "remove_file",
            "rename(",
            "copy(",
        ] {
            assert!(
                !production_source.contains(forbidden),
                "DB-only result/restore foundation must not call {forbidden}"
            );
        }
    }
}
