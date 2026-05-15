use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::{
    error::{AppError, AppResult},
    models::{
        ApplyPlanListItem, DeleteDraftApplyPlanResult, ListSavedApplyPlansRequest,
        PersistedApplyPlan, PersistedApplyPlanBlocker, PersistedApplyPlanItem,
        PersistedApplyPlanItemStatus, PersistedApplyPlanSignal, PersistedApplyPlanStatus,
        SaveApplyPlanPreviewRequest, SaveApplyPlanPreviewResult, StagingPlan,
        StagingPlanActionKind, StagingPlanBucket, StagingPlanConfidenceLabel,
        StagingPlanEvidenceLevel, StagingPlanStatus,
    },
};

const PATH_PRIVACY_LEVEL: &str = "local_full_path_required";
const SOURCE_SYSTEM: &str = "staging_plan";

pub fn save_apply_plan_preview(
    connection: &mut Connection,
    request: SaveApplyPlanPreviewRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    let source_plan = request.source_plan;
    reject_file_touching_source_plan(&source_plan)?;

    let source_plan_kind = request
        .source_plan_kind
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(enum_value(&source_plan.source)?);
    let source_scope_json = request
        .source_scope
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let caveats_json = serde_json::to_string(&source_plan.caveats)?;
    let now = Utc::now().to_rfc3339();
    let prepared_items = prepare_items(&source_plan)?;
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
            scan_session_id,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 1, 1, 0, ?6, 0, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
        params![
            source_plan.id,
            source_plan_kind,
            source_plan.title,
            source_plan.summary,
            plan_status_value(&status),
            prepared_items.len() as i64,
            blocked_items,
            review_only_items,
            caveats_json,
            source_scope_json,
            request.scan_session_id,
            now,
        ],
    )?;
    let plan_id = transaction.last_insert_rowid();

    {
        let mut item_insert = transaction.prepare(
            "INSERT INTO apply_plan_items (
                apply_plan_id,
                source_item_id,
                file_id,
                file_name,
                current_path,
                current_root,
                destination_path,
                destination_root,
                action_kind,
                evidence_level,
                bucket,
                confidence_label,
                item_status,
                blocked,
                review_only,
                validation_status,
                conflict_status,
                path_privacy_level,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, NULL, NULL, ?16, ?17, ?17)",
        )?;
        let mut signal_insert = transaction.prepare(
            "INSERT INTO apply_plan_item_signals (
                apply_plan_item_id,
                signal_kind,
                signal_label,
                signal_value,
                evidence_level,
                source_system,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )?;
        let mut blocker_insert = transaction.prepare(
            "INSERT INTO apply_plan_item_blockers (
                apply_plan_item_id,
                blocker_kind,
                reason_code,
                message,
                source_system,
                created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;

        for item in prepared_items {
            item_insert.execute(params![
                plan_id,
                item.source_item_id,
                item.file_id,
                item.file_name,
                item.current_path,
                item.current_root,
                item.destination_path,
                item.destination_root,
                item.action_kind,
                item.evidence_level,
                item.bucket,
                item.confidence_label,
                item_status_value(&item.item_status),
                bool_to_int(item.blocked),
                bool_to_int(item.review_only),
                PATH_PRIVACY_LEVEL,
                now,
            ])?;
            let item_id = transaction.last_insert_rowid();

            for signal in item.signals {
                signal_insert.execute(params![
                    item_id,
                    "source_signal",
                    signal,
                    Option::<String>::None,
                    item.evidence_level,
                    SOURCE_SYSTEM,
                    now,
                ])?;
            }

            for blocker in item.blockers {
                blocker_insert.execute(params![
                    item_id,
                    blocker.kind,
                    blocker.code,
                    blocker.message,
                    SOURCE_SYSTEM,
                    now,
                ])?;
            }
        }
    }

    transaction.commit()?;
    let plan = get_apply_plan_list_item(connection, plan_id)?
        .ok_or_else(|| AppError::Message("saved ApplyPlan could not be reloaded".to_owned()))?;

    Ok(SaveApplyPlanPreviewResult { plan_id, plan })
}

pub fn list_saved_apply_plans(
    connection: &Connection,
    request: ListSavedApplyPlansRequest,
) -> AppResult<Vec<ApplyPlanListItem>> {
    let limit = request.limit.unwrap_or(50).clamp(1, 200);
    let include_cancelled = request.include_cancelled.unwrap_or(false);
    let sql = if include_cancelled {
        "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
            would_touch_files, confirmation_required, backup_required, restore_available,
            total_items, applyable_items, blocked_items, review_only_items, created_at, updated_at
         FROM apply_plans
         ORDER BY created_at DESC, id DESC
         LIMIT ?1"
    } else {
        "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
            would_touch_files, confirmation_required, backup_required, restore_available,
            total_items, applyable_items, blocked_items, review_only_items, created_at, updated_at
         FROM apply_plans
         WHERE status != 'cancelled'
         ORDER BY created_at DESC, id DESC
         LIMIT ?1"
    };

    let mut statement = connection.prepare(sql)?;
    let rows = statement
        .query_map(params![limit], apply_plan_list_item_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

pub fn get_apply_plan(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<Option<PersistedApplyPlan>> {
    let plan = connection
        .query_row(
            "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
                would_touch_files, confirmation_required, backup_required, restore_available,
                total_items, applyable_items, blocked_items, review_only_items,
                caveats_json, source_scope_json, scan_session_id, created_at, updated_at
             FROM apply_plans
             WHERE id = ?1",
            params![plan_id],
            |row| {
                let caveats_json: String = row.get(14)?;
                let source_scope_json: Option<String> = row.get(15)?;
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
                    caveats_json,
                    source_scope_json,
                    scan_session_id: row.get(16)?,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            },
        )
        .optional()?;

    let Some(plan) = plan else {
        return Ok(None);
    };

    let items = load_apply_plan_items(connection, plan.id)?;
    let caveats = serde_json::from_str::<Vec<String>>(&plan.caveats_json).unwrap_or_default();
    let source_scope = match plan.source_scope_json {
        Some(value) => serde_json::from_str::<serde_json::Value>(&value).ok(),
        None => None,
    };

    Ok(Some(PersistedApplyPlan {
        id: plan.id,
        source_staging_plan_id: plan.source_staging_plan_id,
        source_plan_kind: plan.source_plan_kind,
        title: plan.title,
        summary: plan.summary,
        status: parse_plan_status(&plan.status)?,
        would_touch_files: int_to_bool(plan.would_touch_files),
        confirmation_required: int_to_bool(plan.confirmation_required),
        backup_required: int_to_bool(plan.backup_required),
        restore_available: int_to_bool(plan.restore_available),
        total_items: plan.total_items,
        applyable_items: plan.applyable_items,
        blocked_items: plan.blocked_items,
        review_only_items: plan.review_only_items,
        caveats,
        source_scope,
        scan_session_id: plan.scan_session_id,
        created_at: plan.created_at,
        updated_at: plan.updated_at,
        items,
    }))
}

pub fn delete_draft_apply_plan(
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
    let status = parse_plan_status(&status)?;
    if !matches!(
        status,
        PersistedApplyPlanStatus::Draft
            | PersistedApplyPlanStatus::PreviewOnlySource
            | PersistedApplyPlanStatus::Blocked
            | PersistedApplyPlanStatus::Cancelled
    ) {
        return Err(AppError::Message(
            "Only draft or preview-only ApplyPlan records can be cancelled".to_owned(),
        ));
    }

    let now = Utc::now().to_rfc3339();
    connection.execute(
        "UPDATE apply_plans SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
        params![now, plan_id],
    )?;

    Ok(DeleteDraftApplyPlanResult {
        plan_id,
        cancelled: true,
        status: PersistedApplyPlanStatus::Cancelled,
    })
}

fn get_apply_plan_list_item(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<Option<ApplyPlanListItem>> {
    connection
        .query_row(
            "SELECT id, source_staging_plan_id, source_plan_kind, title, summary, status,
                would_touch_files, confirmation_required, backup_required, restore_available,
                total_items, applyable_items, blocked_items, review_only_items, created_at, updated_at
             FROM apply_plans
             WHERE id = ?1",
            params![plan_id],
            apply_plan_list_item_from_row,
        )
        .optional()
        .map_err(AppError::from)
}

fn load_apply_plan_items(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<Vec<PersistedApplyPlanItem>> {
    let mut statement = connection.prepare(
        "SELECT id, apply_plan_id, source_item_id, file_id, file_name, current_path,
            current_root, destination_path, destination_root, action_kind, evidence_level,
            bucket, confidence_label, item_status, blocked, review_only, validation_status,
            conflict_status, path_privacy_level, created_at, updated_at
         FROM apply_plan_items
         WHERE apply_plan_id = ?1
         ORDER BY id ASC",
    )?;

    let rows = statement
        .query_map(params![plan_id], |row| {
            let item_id: i64 = row.get(0)?;
            Ok(ItemRow {
                id: item_id,
                apply_plan_id: row.get(1)?,
                source_item_id: row.get(2)?,
                file_id: row.get(3)?,
                file_name: row.get(4)?,
                current_path: row.get(5)?,
                current_root: row.get(6)?,
                destination_path: row.get(7)?,
                destination_root: row.get(8)?,
                action_kind: row.get(9)?,
                evidence_level: row.get(10)?,
                bucket: row.get(11)?,
                confidence_label: row.get(12)?,
                item_status: row.get(13)?,
                blocked: row.get(14)?,
                review_only: row.get(15)?,
                validation_status: row.get(16)?,
                conflict_status: row.get(17)?,
                path_privacy_level: row.get(18)?,
                created_at: row.get(19)?,
                updated_at: row.get(20)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let signals = load_apply_plan_item_signals(connection, row.id)?;
        let blockers = load_apply_plan_item_blockers(connection, row.id)?;
        items.push(PersistedApplyPlanItem {
            id: row.id,
            apply_plan_id: row.apply_plan_id,
            source_item_id: row.source_item_id,
            file_id: row.file_id,
            file_name: row.file_name,
            current_path: row.current_path,
            current_root: row.current_root,
            destination_path: row.destination_path,
            destination_root: row.destination_root,
            action_kind: row.action_kind,
            evidence_level: row.evidence_level,
            bucket: row.bucket,
            confidence_label: row.confidence_label,
            item_status: parse_item_status(&row.item_status)?,
            blocked: int_to_bool(row.blocked),
            review_only: int_to_bool(row.review_only),
            validation_status: row.validation_status,
            conflict_status: row.conflict_status,
            path_privacy_level: row.path_privacy_level,
            created_at: row.created_at,
            updated_at: row.updated_at,
            signals,
            blockers,
        });
    }

    Ok(items)
}

fn load_apply_plan_item_signals(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Vec<PersistedApplyPlanSignal>> {
    let mut statement = connection.prepare(
        "SELECT id, apply_plan_item_id, signal_kind, signal_label, signal_value,
            evidence_level, source_system, created_at
         FROM apply_plan_item_signals
         WHERE apply_plan_item_id = ?1
         ORDER BY id ASC",
    )?;
    let rows = statement
        .query_map(params![item_id], |row| {
            Ok(PersistedApplyPlanSignal {
                id: row.get(0)?,
                apply_plan_item_id: row.get(1)?,
                signal_kind: row.get(2)?,
                signal_label: row.get(3)?,
                signal_value: row.get(4)?,
                evidence_level: row.get(5)?,
                source_system: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn load_apply_plan_item_blockers(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Vec<PersistedApplyPlanBlocker>> {
    let mut statement = connection.prepare(
        "SELECT id, apply_plan_item_id, blocker_kind, reason_code, message,
            source_system, created_at
         FROM apply_plan_item_blockers
         WHERE apply_plan_item_id = ?1
         ORDER BY id ASC",
    )?;
    let rows = statement
        .query_map(params![item_id], |row| {
            Ok(PersistedApplyPlanBlocker {
                id: row.get(0)?,
                apply_plan_item_id: row.get(1)?,
                blocker_kind: row.get(2)?,
                reason_code: row.get(3)?,
                message: row.get(4)?,
                source_system: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn prepare_items(source_plan: &StagingPlan) -> AppResult<Vec<PreparedItem>> {
    source_plan
        .items
        .iter()
        .map(|item| {
            let mut blockers = item
                .blocked_reasons
                .iter()
                .map(|reason| PreparedBlocker {
                    kind: "blocked".to_owned(),
                    code: reason.clone(),
                    message: reason.clone(),
                })
                .collect::<Vec<_>>();

            let missing_current_path = item
                .current_path
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty();
            if missing_current_path {
                blockers.push(PreparedBlocker {
                    kind: "blocked".to_owned(),
                    code: "missing_current_path".to_owned(),
                    message: "Current path was not present in the preview source.".to_owned(),
                });
            }
            if source_plan.status == StagingPlanStatus::Blocked {
                blockers.push(PreparedBlocker {
                    kind: "blocked".to_owned(),
                    code: "source_plan_blocked".to_owned(),
                    message: "Source preview plan was blocked.".to_owned(),
                });
            }

            let review_only = is_review_only_item(item);
            if review_only {
                blockers.push(PreparedBlocker {
                    kind: "review_only".to_owned(),
                    code: "review_only_evidence".to_owned(),
                    message: "Item requires manual review before any future Apply.".to_owned(),
                });
            }

            let blocked = blockers.iter().any(|blocker| blocker.kind == "blocked");
            let item_status = if blocked {
                PersistedApplyPlanItemStatus::Blocked
            } else if review_only {
                PersistedApplyPlanItemStatus::ReviewOnly
            } else if matches!(
                item.action_kind,
                StagingPlanActionKind::SuggestMove | StagingPlanActionKind::SuggestGroup
            ) && item.suggested_destination_path.is_some()
            {
                PersistedApplyPlanItemStatus::DraftCandidate
            } else {
                PersistedApplyPlanItemStatus::PreviewOnly
            };

            Ok(PreparedItem {
                source_item_id: Some(item.id.clone()),
                file_id: item.file_id,
                file_name: item.file_name.clone(),
                current_path: item.current_path.clone().unwrap_or_default(),
                current_root: enum_value(&item.current_root)?,
                destination_path: item.suggested_destination_path.clone(),
                destination_root: destination_root_for_bucket(&item.bucket).map(str::to_owned),
                action_kind: enum_value(&item.action_kind)?,
                evidence_level: enum_value(&item.evidence_level)?,
                bucket: Some(enum_value(&item.bucket)?),
                confidence_label: Some(enum_value(&item.confidence_label)?),
                item_status,
                blocked,
                review_only,
                signals: item.source_signals.clone(),
                blockers,
            })
        })
        .collect()
}

fn reject_file_touching_source_plan(source_plan: &StagingPlan) -> AppResult<()> {
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

fn is_review_only_item(item: &crate::models::StagingPlanItem) -> bool {
    matches!(item.evidence_level, StagingPlanEvidenceLevel::ReviewOnly)
        || matches!(
            item.confidence_label,
            StagingPlanConfidenceLabel::ReviewOnly
        )
        || matches!(item.action_kind, StagingPlanActionKind::SuggestReview)
}

fn destination_root_for_bucket(bucket: &StagingPlanBucket) -> Option<&'static str> {
    match bucket {
        StagingPlanBucket::Tray => Some("tray"),
        StagingPlanBucket::ScriptMods
        | StagingPlanBucket::Cas
        | StagingPlanBucket::BuildBuy
        | StagingPlanBucket::Gameplay
        | StagingPlanBucket::PresetsSliders
        | StagingPlanBucket::OverridesDefaults => Some("mods"),
        StagingPlanBucket::NeedsReview | StagingPlanBucket::UnknownLeaveInPlace => None,
    }
}

fn apply_plan_list_item_from_row(row: &Row<'_>) -> rusqlite::Result<ApplyPlanListItem> {
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
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn parse_plan_status(value: &str) -> AppResult<PersistedApplyPlanStatus> {
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

fn parse_item_status(value: &str) -> AppResult<PersistedApplyPlanItemStatus> {
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

fn plan_status_value(status: &PersistedApplyPlanStatus) -> &'static str {
    match status {
        PersistedApplyPlanStatus::Draft => "draft",
        PersistedApplyPlanStatus::PreviewOnlySource => "preview_only_source",
        PersistedApplyPlanStatus::Blocked => "blocked",
        PersistedApplyPlanStatus::Cancelled => "cancelled",
    }
}

fn item_status_value(status: &PersistedApplyPlanItemStatus) -> &'static str {
    match status {
        PersistedApplyPlanItemStatus::PreviewOnly => "preview_only",
        PersistedApplyPlanItemStatus::Blocked => "blocked",
        PersistedApplyPlanItemStatus::ReviewOnly => "review_only",
        PersistedApplyPlanItemStatus::DraftCandidate => "draft_candidate",
    }
}

fn enum_value<T: serde::Serialize>(value: &T) -> AppResult<String> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(value) => Ok(value),
        _ => Err(AppError::Message(
            "Could not serialize enum value as string".to_owned(),
        )),
    }
}

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

fn to_sql_error(error: AppError) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

struct PreparedItem {
    source_item_id: Option<String>,
    file_id: Option<i64>,
    file_name: String,
    current_path: String,
    current_root: String,
    destination_path: Option<String>,
    destination_root: Option<String>,
    action_kind: String,
    evidence_level: String,
    bucket: Option<String>,
    confidence_label: Option<String>,
    item_status: PersistedApplyPlanItemStatus,
    blocked: bool,
    review_only: bool,
    signals: Vec<String>,
    blockers: Vec<PreparedBlocker>,
}

struct PreparedBlocker {
    kind: String,
    code: String,
    message: String,
}

struct PlanRow {
    id: i64,
    source_staging_plan_id: Option<String>,
    source_plan_kind: String,
    title: String,
    summary: String,
    status: String,
    would_touch_files: i64,
    confirmation_required: i64,
    backup_required: i64,
    restore_available: i64,
    total_items: i64,
    applyable_items: i64,
    blocked_items: i64,
    review_only_items: i64,
    caveats_json: String,
    source_scope_json: Option<String>,
    scan_session_id: Option<i64>,
    created_at: String,
    updated_at: String,
}

struct ItemRow {
    id: i64,
    apply_plan_id: i64,
    source_item_id: Option<String>,
    file_id: Option<i64>,
    file_name: String,
    current_path: String,
    current_root: String,
    destination_path: Option<String>,
    destination_root: Option<String>,
    action_kind: String,
    evidence_level: String,
    bucket: Option<String>,
    confidence_label: Option<String>,
    item_status: String,
    blocked: i64,
    review_only: i64,
    validation_status: Option<String>,
    conflict_status: Option<String>,
    path_privacy_level: String,
    created_at: String,
    updated_at: String,
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use crate::{
        database,
        models::{
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
            StagingPlanActionKind,
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
            },
        )
        .expect("preview plan")
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
                scan_session_id: None,
            },
        )
        .expect("save");

        assert!(result.plan_id > 0);
        assert_eq!(result.plan.would_touch_files, false);
        assert_eq!(result.plan.applyable_items, 0);
        assert_eq!(result.plan.total_items, 2);

        let saved = get_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved plan");
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
    fn save_rejects_source_plan_that_would_touch_files() {
        let mut connection = memory_connection();
        let mut plan = sample_plan();
        plan.would_touch_files = true;

        let error = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: plan,
                source_plan_kind: None,
                source_scope: None,
                scan_session_id: None,
            },
        )
        .expect_err("file-touching source plan should fail");

        assert!(error.to_string().contains("would touch files"));
    }

    #[test]
    fn save_rejects_source_item_that_would_touch_files() {
        let mut connection = memory_connection();
        let mut plan = sample_plan();
        plan.items[0].would_touch_files = true;

        let error = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: plan,
                source_plan_kind: None,
                source_scope: None,
                scan_session_id: None,
            },
        )
        .expect_err("file-touching source item should fail");

        assert!(error
            .to_string()
            .contains("source items that would touch files"));
    }

    #[test]
    fn list_summaries_exclude_paths_and_cancelled_plans_by_default() {
        let mut connection = memory_connection();
        let result = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: None,
                source_scope: None,
                scan_session_id: None,
            },
        )
        .expect("save");

        let plans = list_saved_apply_plans(&connection, ListSavedApplyPlansRequest::default())
            .expect("list");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].id, result.plan_id);

        let summary_json = serde_json::to_string(&plans[0]).expect("summary json");
        assert!(!summary_json.contains("C:/Sims/Mods"));

        let cancelled =
            delete_draft_apply_plan(&mut connection, result.plan_id).expect("cancel draft");
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
    fn draft_candidate_items_are_not_applyable_in_v1() {
        let mut connection = memory_connection();
        let mut plan = sample_plan();
        plan.items[0].action_kind = StagingPlanActionKind::SuggestMove;

        let result = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: plan,
                source_plan_kind: None,
                source_scope: None,
                scan_session_id: None,
            },
        )
        .expect("save");

        let saved = get_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved");
        assert_eq!(saved.applyable_items, 0);
        assert!(saved
            .items
            .iter()
            .any(|item| item.item_status == PersistedApplyPlanItemStatus::DraftCandidate));
    }
}
