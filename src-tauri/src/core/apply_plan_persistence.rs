use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use sha2::Digest;

use crate::{
    core::apply_plan_provenance,
    error::{AppError, AppResult},
    models::{
        ApplyPlanContextSignal, ApplyPlanFolderConfig, ApplyPlanListItem,
        BuildApplyPlanFromStagingPlanRequest, DeleteDraftApplyPlanResult,
        GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanResult, LibrarySettings,
        ListSavedApplyPlansRequest, PersistedApplyPlan, PersistedApplyPlanBlocker,
        PersistedApplyPlanItem, PersistedApplyPlanItemStatus, PersistedApplyPlanSignal,
        PersistedApplyPlanStatus, SaveApplyPlanFromPreviewSnapshotRequest,
        SaveApplyPlanPreviewRequest, SaveApplyPlanPreviewResult, StagingPlan,
        StagingPlanActionKind, StagingPlanBucket, StagingPlanConfidenceLabel,
        StagingPlanEvidenceLevel, StagingPlanStatus,
    },
};

const PATH_PRIVACY_LEVEL: &str = "local_full_path_required";
const SOURCE_SYSTEM: &str = "staging_plan";

pub fn generate_sorting_preview_snapshot(
    connection: &mut Connection,
    settings: &LibrarySettings,
    request: GenerateSortingPreviewPlanRequest,
) -> AppResult<GenerateSortingPreviewPlanResult> {
    let source_scope = serde_json::to_value(&request.scope)?;
    let folder_config = request.folder_config.clone();
    let context_trail = request.context_trail.clone();
    let source_plan = crate::core::rule_engine::sorting_plan::generate_sorting_preview_plan(
        connection, settings, request,
    )?;
    reject_file_touching_source_plan(&source_plan)?;

    let seed = serde_json::to_vec(&source_plan)?;
    let seed_hash = hex::encode(sha2::Sha256::digest(seed));
    let snapshot_id = format!(
        "apply-preview-{}-{}",
        Utc::now().timestamp_millis(),
        &seed_hash[..12]
    );

    let source_plan_kind = apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND;
    let hash_stamp = apply_plan_provenance::build_preview_snapshot_hash_stamp(
        connection,
        &source_plan,
        source_plan_kind,
        &Some(source_scope.clone()),
        &folder_config,
        &context_trail,
        None,
    )?;
    let source_plan_json = serde_json::to_string(&source_plan)?;
    let source_scope_json = serde_json::to_string(&source_scope)?;
    let folder_config_json = folder_config
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let context_trail_json = serde_json::to_string(&context_trail)?;
    let provenance_json = serde_json::to_string(&hash_stamp.provenance)?;
    let created_at = Utc::now().to_rfc3339();

    connection.execute(
        "INSERT INTO apply_plan_preview_snapshots (
            snapshot_id,
            source_plan_kind,
            source_plan_json,
            source_scope_json,
            folder_config_json,
            context_trail_json,
            preview_snapshot_hash,
            preview_snapshot_hash_version,
            preview_snapshot_hash_algorithm,
            preview_snapshot_provenance_json,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            snapshot_id,
            source_plan_kind,
            source_plan_json,
            source_scope_json,
            folder_config_json,
            context_trail_json,
            hash_stamp.hash,
            hash_stamp.version,
            hash_stamp.algorithm,
            provenance_json,
            created_at,
        ],
    )?;
    let preview_snapshot_id = connection.last_insert_rowid();

    Ok(GenerateSortingPreviewPlanResult {
        plan: source_plan,
        preview_snapshot_id,
        preview_snapshot_hash: hash_stamp.hash,
        preview_snapshot_hash_version: hash_stamp.version,
        preview_snapshot_hash_algorithm: hash_stamp.algorithm,
        preview_snapshot_created_at: created_at,
    })
}

pub fn build_apply_plan_from_staging_plan(
    connection: &mut Connection,
    settings: &LibrarySettings,
    request: BuildApplyPlanFromStagingPlanRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    let mut preview_request = request.preview_request;
    preview_request.folder_config = request.folder_config.or(preview_request.folder_config);
    if !request.context_trail.is_empty() {
        preview_request.context_trail = request.context_trail;
    }

    let preview = generate_sorting_preview_snapshot(connection, settings, preview_request)?;
    save_apply_plan_from_preview_snapshot(
        connection,
        SaveApplyPlanFromPreviewSnapshotRequest {
            preview_snapshot_id: preview.preview_snapshot_id,
            preview_snapshot_hash: preview.preview_snapshot_hash,
        },
    )
}

pub fn save_apply_plan_preview(
    connection: &mut Connection,
    request: SaveApplyPlanPreviewRequest,
) -> AppResult<SaveApplyPlanPreviewResult> {
    save_apply_plan_preview_with_source_kind(
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
    let snapshot = load_preview_snapshot(connection, request.preview_snapshot_id)?
        .ok_or_else(|| AppError::Message("Preview snapshot was not found.".to_owned()))?;

    if snapshot.consumed_apply_plan_id.is_some() {
        return Err(AppError::Message(
            "Preview snapshot has already been saved as an ApplyPlan draft.".to_owned(),
        ));
    }

    if snapshot.preview_snapshot_hash != request.preview_snapshot_hash {
        return Err(AppError::Message(
            "Preview snapshot hash mismatch; regenerate the preview before saving.".to_owned(),
        ));
    }

    if snapshot.preview_snapshot_hash_version
        != apply_plan_provenance::PREVIEW_SNAPSHOT_HASH_VERSION
        || snapshot.preview_snapshot_hash_algorithm != apply_plan_provenance::PLAN_HASH_ALGORITHM
    {
        return Err(AppError::Message(
            "Preview snapshot uses an unsupported hash version or algorithm.".to_owned(),
        ));
    }

    let source_plan: StagingPlan =
        serde_json::from_str(&snapshot.source_plan_json).map_err(|error| {
            AppError::Message(format!(
                "Invalid source_plan_json for preview snapshot {}: {error}",
                snapshot.id
            ))
        })?;
    let source_scope: Option<serde_json::Value> = snapshot
        .source_scope_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| {
            AppError::Message(format!(
                "Invalid source_scope_json for preview snapshot {}: {error}",
                snapshot.id
            ))
        })?;
    let folder_config: Option<ApplyPlanFolderConfig> = snapshot
        .folder_config_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| {
            AppError::Message(format!(
                "Invalid folder_config_json for preview snapshot {}: {error}",
                snapshot.id
            ))
        })?;
    let context_trail: Vec<ApplyPlanContextSignal> =
        serde_json::from_str(&snapshot.context_trail_json).map_err(|error| {
            AppError::Message(format!(
                "Invalid context_trail_json for preview snapshot {}: {error}",
                snapshot.id
            ))
        })?;
    let stored_provenance: serde_json::Value =
        serde_json::from_str(&snapshot.preview_snapshot_provenance_json).map_err(|error| {
            AppError::Message(format!(
                "Invalid preview_snapshot_provenance_json for preview snapshot {}: {error}",
                snapshot.id
            ))
        })?;
    let recomputed = apply_plan_provenance::build_preview_snapshot_hash_stamp(
        connection,
        &source_plan,
        &snapshot.source_plan_kind,
        &source_scope,
        &folder_config,
        &context_trail,
        snapshot.scan_session_id,
    )?;
    if recomputed.hash != snapshot.preview_snapshot_hash
        || recomputed.provenance != stored_provenance
    {
        return Err(AppError::Message(
            "Preview snapshot provenance no longer matches stored preview content.".to_owned(),
        ));
    }

    let result = save_apply_plan_preview_with_source_kind(
        connection,
        SaveApplyPlanPreviewRequest {
            source_plan,
            source_plan_kind: None,
            source_scope,
            folder_config,
            context_trail,
            scan_session_id: snapshot.scan_session_id,
        },
        &snapshot.source_plan_kind,
        Some(snapshot.id),
        Some(snapshot.preview_snapshot_hash.clone()),
    )?;
    let consumed_rows = connection.execute(
        "UPDATE apply_plan_preview_snapshots
         SET consumed_apply_plan_id = ?1
         WHERE id = ?2 AND consumed_apply_plan_id IS NULL",
        params![result.plan_id, snapshot.id],
    )?;
    if consumed_rows != 1 {
        return Err(AppError::Message(
            "Preview snapshot has already been saved as an ApplyPlan draft.".to_owned(),
        ));
    }

    Ok(result)
}

fn save_apply_plan_preview_with_source_kind(
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
            plan_status_value(&status),
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

    let persisted = get_apply_plan(&transaction, plan_id)?
        .ok_or_else(|| AppError::Message("saved ApplyPlan could not be reloaded".to_owned()))?;
    let hash_stamp = apply_plan_provenance::build_hash_stamp(&transaction, &persisted)?;
    let plan_provenance_json = serde_json::to_string(&hash_stamp.provenance)?;
    transaction.execute(
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
            now,
            plan_provenance_json,
            plan_id,
        ],
    )?;

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
                caveats_json, source_scope_json, folder_config_json, context_trail_json,
                plan_hash, plan_hash_version, plan_hash_algorithm, plan_hash_created_at,
                preview_snapshot_id, preview_snapshot_hash,
                plan_provenance_json, scan_session_id, created_at, updated_at
             FROM apply_plans
             WHERE id = ?1",
            params![plan_id],
            |row| {
                let caveats_json: String = row.get(14)?;
                let source_scope_json: Option<String> = row.get(15)?;
                let folder_config_json: Option<String> = row.get(16)?;
                let context_trail_json: String = row.get(17)?;
                let plan_provenance_json: String = row.get(24)?;
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
                    folder_config_json,
                    context_trail_json,
                    plan_hash: row.get(18)?,
                    plan_hash_version: row.get(19)?,
                    plan_hash_algorithm: row.get(20)?,
                    plan_hash_created_at: row.get(21)?,
                    preview_snapshot_id: row.get(22)?,
                    preview_snapshot_hash: row.get(23)?,
                    plan_provenance_json,
                    scan_session_id: row.get(25)?,
                    created_at: row.get(26)?,
                    updated_at: row.get(27)?,
                })
            },
        )
        .optional()?;

    let Some(plan) = plan else {
        return Ok(None);
    };

    let items = load_apply_plan_items(connection, plan.id)?;
    let caveats = serde_json::from_str::<Vec<String>>(&plan.caveats_json).map_err(|error| {
        AppError::Message(format!(
            "Invalid caveats_json for ApplyPlan {}: {error}",
            plan.id
        ))
    })?;
    let source_scope = plan
        .source_scope_json
        .as_deref()
        .map(serde_json::from_str::<serde_json::Value>)
        .transpose()
        .map_err(|error| {
            AppError::Message(format!(
                "Invalid source_scope_json for ApplyPlan {}: {error}",
                plan.id
            ))
        })?;
    let folder_config = plan
        .folder_config_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|error| {
            AppError::Message(format!(
                "Invalid folder_config_json for ApplyPlan {}: {error}",
                plan.id
            ))
        })?;
    let context_trail = serde_json::from_str(&plan.context_trail_json).map_err(|error| {
        AppError::Message(format!(
            "Invalid context_trail_json for ApplyPlan {}: {error}",
            plan.id
        ))
    })?;
    let plan_provenance = serde_json::from_str(&plan.plan_provenance_json).map_err(|error| {
        AppError::Message(format!(
            "Invalid plan_provenance_json for ApplyPlan {}: {error}",
            plan.id
        ))
    })?;

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

pub fn verify_apply_plan_hash(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<apply_plan_provenance::ApplyPlanHashVerification> {
    let plan = get_apply_plan(connection, plan_id)?
        .ok_or_else(|| AppError::Message(format!("Saved ApplyPlan {plan_id} was not found")))?;
    apply_plan_provenance::verify_hash(connection, &plan)
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
                total_items, applyable_items, blocked_items, review_only_items,
            plan_hash, plan_hash_version, plan_hash_algorithm, plan_hash_created_at,
            preview_snapshot_id, preview_snapshot_hash,
            created_at, updated_at
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

fn load_preview_snapshot(
    connection: &Connection,
    preview_snapshot_id: i64,
) -> AppResult<Option<PreviewSnapshotRow>> {
    connection
        .query_row(
            "SELECT id, snapshot_id, source_plan_kind, source_plan_json, source_scope_json,
                folder_config_json, context_trail_json, preview_snapshot_hash,
                preview_snapshot_hash_version, preview_snapshot_hash_algorithm,
                preview_snapshot_provenance_json, scan_session_id, consumed_apply_plan_id,
                created_at, expires_at
             FROM apply_plan_preview_snapshots
             WHERE id = ?1",
            params![preview_snapshot_id],
            |row| {
                Ok(PreviewSnapshotRow {
                    id: row.get(0)?,
                    snapshot_id: row.get(1)?,
                    source_plan_kind: row.get(2)?,
                    source_plan_json: row.get(3)?,
                    source_scope_json: row.get(4)?,
                    folder_config_json: row.get(5)?,
                    context_trail_json: row.get(6)?,
                    preview_snapshot_hash: row.get(7)?,
                    preview_snapshot_hash_version: row.get(8)?,
                    preview_snapshot_hash_algorithm: row.get(9)?,
                    preview_snapshot_provenance_json: row.get(10)?,
                    scan_session_id: row.get(11)?,
                    consumed_apply_plan_id: row.get(12)?,
                    created_at: row.get(13)?,
                    expires_at: row.get(14)?,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
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

struct PreviewSnapshotRow {
    id: i64,
    snapshot_id: String,
    source_plan_kind: String,
    source_plan_json: String,
    source_scope_json: Option<String>,
    folder_config_json: Option<String>,
    context_trail_json: String,
    preview_snapshot_hash: String,
    preview_snapshot_hash_version: String,
    preview_snapshot_hash_algorithm: String,
    preview_snapshot_provenance_json: String,
    scan_session_id: Option<i64>,
    consumed_apply_plan_id: Option<i64>,
    created_at: String,
    expires_at: Option<String>,
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
    folder_config_json: Option<String>,
    context_trail_json: String,
    plan_hash: Option<String>,
    plan_hash_version: String,
    plan_hash_algorithm: String,
    plan_hash_created_at: Option<String>,
    preview_snapshot_id: Option<i64>,
    preview_snapshot_hash: Option<String>,
    plan_provenance_json: String,
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
            BuildApplyPlanFromStagingPlanRequest, GenerateSortingPreviewPlanRequest,
            GenerateSortingPreviewPlanScope, LibrarySettings,
            SaveApplyPlanFromPreviewSnapshotRequest, StagingPlanActionKind,
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

    fn sample_builder_request() -> BuildApplyPlanFromStagingPlanRequest {
        BuildApplyPlanFromStagingPlanRequest {
            preview_request: GenerateSortingPreviewPlanRequest {
                scope: GenerateSortingPreviewPlanScope::SelectedFiles {
                    file_ids: vec![1, 2],
                },
                folder_config: None,
                context_trail: Vec::new(),
            },
            source_plan_kind: None,
            folder_config: None,
            context_trail: Vec::new(),
        }
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
    fn saved_plan_hash_verification_detects_tampered_context_and_destination() {
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

        assert!(
            verify_apply_plan_hash(&connection, result.plan_id)
                .expect("verify original")
                .is_valid
        );

        connection
            .execute(
                "UPDATE apply_plans SET context_trail_json = '[{\"sourceSystem\":\"test\",\"signalKind\":\"tamper\",\"label\":\"Tampered\",\"value\":null,\"strength\":\"evidence\"}]' WHERE id = ?1",
                [result.plan_id],
            )
            .expect("tamper context");
        let context_check =
            verify_apply_plan_hash(&connection, result.plan_id).expect("verify tampered context");
        assert!(!context_check.is_valid);
        assert_eq!(context_check.status, "mismatch");

        connection
            .execute(
                "UPDATE apply_plans SET context_trail_json = '[]' WHERE id = ?1",
                [result.plan_id],
            )
            .expect("restore context");
        connection
            .execute(
                "UPDATE apply_plan_items SET destination_path = 'C:/Sims/Mods/Tampered/a.package' WHERE apply_plan_id = ?1 AND destination_path IS NOT NULL",
                [result.plan_id],
            )
            .expect("tamper destination");
        let destination_check = verify_apply_plan_hash(&connection, result.plan_id)
            .expect("verify tampered destination");
        assert!(!destination_check.is_valid);
        assert_eq!(destination_check.status, "mismatch");
    }

    #[test]
    fn saved_plan_hash_verification_detects_same_count_source_scope_tampering() {
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
        let result = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: None,
                source_scope: Some(
                    serde_json::json!({"kind": "selected_files", "fileIds": [1, 2]}),
                ),
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
        )
        .expect("save");
        assert!(
            verify_apply_plan_hash(&connection, result.plan_id)
                .expect("verify original")
                .is_valid
        );

        connection
            .execute(
                "UPDATE apply_plans SET source_scope_json = '{\"kind\":\"selected_files\",\"fileIds\":[999,1000]}' WHERE id = ?1",
                [result.plan_id],
            )
            .expect("tamper source scope");
        let source_scope_check = verify_apply_plan_hash(&connection, result.plan_id)
            .expect("verify tampered source scope");
        assert!(!source_scope_check.is_valid);
        assert_eq!(source_scope_check.status, "mismatch");
    }

    #[test]
    fn cancelling_draft_does_not_rewrite_immutable_plan_hash() {
        let mut connection = memory_connection();
        let result = save_apply_plan_preview(
            &mut connection,
            SaveApplyPlanPreviewRequest {
                source_plan: sample_plan(),
                source_plan_kind: None,
                source_scope: None,
                folder_config: None,
                context_trail: Vec::new(),
                scan_session_id: None,
            },
        )
        .expect("save");
        let before = get_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved")
            .plan_hash;

        delete_draft_apply_plan(&mut connection, result.plan_id).expect("cancel draft");

        let after = get_apply_plan(&connection, result.plan_id)
            .expect("get after cancel")
            .expect("saved after cancel")
            .plan_hash;
        assert_eq!(before, after);
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
                folder_config: None,
                context_trail: Vec::new(),
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
                folder_config: None,
                context_trail: Vec::new(),
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
                folder_config: None,
                context_trail: Vec::new(),
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
                folder_config: None,
                context_trail: Vec::new(),
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

    #[test]
    fn builder_generates_and_saves_draft_apply_plan_from_sorting_preview() {
        let mut connection = memory_connection();
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
                ('C:/Sims/Mods/hair.package', 'hair.package', 'package', 1, '2026-01-01', 'h1', 'CAS', 'Hair', 0.95, '[]', '[]', 'mods', 1),
                ('C:/Sims/Mods/unknown.package', 'unknown.package', 'package', 1, '2026-01-01', 'h2', 'Unknown', NULL, 0.10, '[]', '[\"parser_warning\"]', 'mods', 1)",
                [],
            )
            .expect("files");

        let result = build_apply_plan_from_staging_plan(
            &mut connection,
            &settings,
            sample_builder_request(),
        )
        .expect("build saved draft");

        assert!(result.plan_id > 0);
        assert_eq!(
            result.plan.source_plan_kind,
            apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
        );
        assert!(!result.plan.would_touch_files);
        assert_eq!(result.plan.applyable_items, 0);
        assert_eq!(result.plan.total_items, 2);

        let summaries = list_saved_apply_plans(&connection, ListSavedApplyPlansRequest::default())
            .expect("list");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, result.plan_id);

        let saved = get_apply_plan(&connection, result.plan_id)
            .expect("get")
            .expect("saved");
        assert_eq!(
            saved
                .source_scope
                .as_ref()
                .and_then(|scope| scope.get("kind"))
                .and_then(|value| value.as_str()),
            Some("selected_files")
        );
        assert!(saved
            .caveats
            .iter()
            .any(|caveat| caveat.contains("No files changed")));
        assert!(saved.items.iter().all(|item| !item.signals.is_empty()));
        assert!(saved
            .items
            .iter()
            .all(|item| !item.blocked || !item.blockers.is_empty()));
        assert!(saved
            .items
            .iter()
            .any(|item| item.item_status == PersistedApplyPlanItemStatus::Blocked));
        assert!(
            saved.preview_snapshot_id.is_some(),
            "backend-generated builder saves must be bound to a preview snapshot"
        );
        assert!(saved.preview_snapshot_hash.is_some());
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
    fn same_count_rule_template_edit_invalidates_saved_hash() {
        let mut connection = memory_connection();
        connection
            .execute(
                "INSERT INTO rules (rule_name, rule_template, rule_priority, enabled)
                 VALUES ('provenance-test-rule', 'initial template', 1, 1)",
                [],
            )
            .expect("insert active rule");
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

        assert!(
            verify_apply_plan_hash(&connection, result.plan_id)
                .expect("original hash")
                .is_valid
        );
        connection
            .execute(
                "UPDATE rules
                 SET rule_template = rule_template || ' :: edited for provenance test'
                 WHERE id = (SELECT id FROM rules WHERE enabled = 1 ORDER BY rule_priority ASC, rule_name ASC LIMIT 1)",
                [],
            )
            .expect("edit one active rule");

        let check =
            verify_apply_plan_hash(&connection, result.plan_id).expect("verify after rule edit");
        assert!(!check.is_valid);
        assert_eq!(check.status, "mismatch");
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

    #[test]
    fn malformed_persisted_plan_json_fails_closed() {
        for column in [
            "caveats_json",
            "source_scope_json",
            "folder_config_json",
            "context_trail_json",
        ] {
            let mut connection = memory_connection();
            let result = save_apply_plan_preview(
                &mut connection,
                SaveApplyPlanPreviewRequest {
                    source_plan: sample_plan(),
                    source_plan_kind: None,
                    source_scope: Some(serde_json::json!({"kind": "selected_files"})),
                    folder_config: None,
                    context_trail: Vec::new(),
                    scan_session_id: None,
                },
            )
            .expect("save");
            connection
                .execute(
                    &format!("UPDATE apply_plans SET {column} = '{{' WHERE id = ?1"),
                    [result.plan_id],
                )
                .expect("corrupt json column");

            let error = get_apply_plan(&connection, result.plan_id)
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
        let error = get_apply_plan(&connection, connection.last_insert_rowid())
            .expect_err("malformed provenance should fail closed");
        assert!(error.to_string().contains("plan_provenance_json"));
    }
}
