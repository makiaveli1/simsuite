use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::Digest;

use crate::{
    error::{AppError, AppResult},
    models::{ApplyPlanContextSignal, ApplyPlanFolderConfig, PersistedApplyPlan, StagingPlan},
};

pub const PLAN_HASH_VERSION: &str = "apply_plan_hash_v1";
pub const PREVIEW_SNAPSHOT_HASH_VERSION: &str = "apply_plan_preview_snapshot_v3";
pub const PLAN_HASH_ALGORITHM: &str = "sha256";
pub const SORTING_PREVIEW_PLAN_VERSION: &str = "sorting_preview_plan_v1";
pub const VALIDATION_PREVIEW_VERSION: &str = "apply_plan_validation_preview_v1";
pub const CLIENT_SUPPLIED_PREVIEW_SOURCE_KIND: &str = "client_supplied_preview";
pub const BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND: &str = "backend_generated_sorting_preview";

#[derive(Debug, Clone)]
pub struct ApplyPlanHashStamp {
    pub hash: String,
    pub version: String,
    pub algorithm: String,
    pub provenance: Value,
}

#[derive(Debug, Clone)]
pub struct ApplyPlanHashVerification {
    pub is_valid: bool,
    pub status: String,
    pub message: String,
}

pub fn build_hash_stamp(
    connection: &Connection,
    plan: &PersistedApplyPlan,
) -> AppResult<ApplyPlanHashStamp> {
    let provenance = build_provenance(connection, plan)?;
    let canonical = serde_json::to_vec(&provenance)?;
    let hash = hex::encode(sha2::Sha256::digest(canonical));

    Ok(ApplyPlanHashStamp {
        hash,
        version: PLAN_HASH_VERSION.to_owned(),
        algorithm: PLAN_HASH_ALGORITHM.to_owned(),
        provenance,
    })
}

pub fn build_preview_snapshot_hash_stamp(
    connection: &Connection,
    source_plan: &StagingPlan,
    source_plan_kind: &str,
    source_scope: &Option<Value>,
    folder_config: &Option<ApplyPlanFolderConfig>,
    context_trail: &[ApplyPlanContextSignal],
    scan_session_id: Option<i64>,
) -> AppResult<ApplyPlanHashStamp> {
    let seed_version = optional_scalar(
        connection,
        "SELECT value FROM seed_meta WHERE key = 'seed_version' LIMIT 1",
    )?;
    let creator_learning_version = optional_scalar(
        connection,
        "SELECT value FROM app_settings WHERE key = 'creator_learning_version' LIMIT 1",
    )?;
    let category_override_version = optional_scalar(
        connection,
        "SELECT value FROM app_settings WHERE key = 'category_override_version' LIMIT 1",
    )?;
    let active_rules = load_active_rule_snapshot(connection)?;
    let active_rules_hash = hash_json(&active_rules)?;
    let mut items = source_plan
        .items
        .iter()
        .map(|item| {
            Ok(json!({
                "sourceItemId": item.id,
                "fileName": item.file_name,
                "currentPath": item.current_path,
                "currentRoot": item.current_root,
                "destinationPath": item.suggested_destination_path,
                "actionKind": item.action_kind,
                "evidenceLevel": item.evidence_level,
                "reason": item.reason,
                "bucket": item.bucket,
                "confidenceLabel": item.confidence_label,
                "blockedReasons": item.blocked_reasons,
                "sourceSignals": item.source_signals,
                "caveats": item.caveats,
                "wouldTouchFiles": item.would_touch_files,
                "sourceFileSnapshot": match item.file_id {
                    Some(file_id) => load_source_file_snapshot(connection, file_id)?,
                    None => Value::Null,
                },
            }))
        })
        .collect::<AppResult<Vec<_>>>()?;
    items = canonical_sorted(items)?;
    let items = items
        .into_iter()
        .enumerate()
        .map(|(ordinal, mut item)| {
            if let Some(object) = item.as_object_mut() {
                object.insert("ordinal".to_owned(), json!(ordinal));
            }
            item
        })
        .collect::<Vec<_>>();

    let provenance = json!({
        "previewSnapshotHashVersion": PREVIEW_SNAPSHOT_HASH_VERSION,
        "previewSnapshotHashAlgorithm": PLAN_HASH_ALGORITHM,
        "sourcePlanKind": source_plan_kind,
        "sourceKind": source_plan_kind,
        "sourceStagingPlanId": source_plan.id,
        "source": source_plan.source,
        "status": source_plan.status,
        "title": source_plan.title,
        "summary": source_plan.summary,
        "itemCount": source_plan.item_count,
        "wouldTouchFiles": source_plan.would_touch_files,
        "caveats": source_plan.caveats,
        "sourceScope": sanitized_source_scope(connection, source_scope)?,
        "folderConfig": folder_config,
        "contextTrail": context_trail,
        "scanSessionAttached": scan_session_id.is_some(),
        "items": items,
        "versionEvidence": {
            "sortingPreviewPlanVersion": SORTING_PREVIEW_PLAN_VERSION,
            "validationPreviewVersion": VALIDATION_PREVIEW_VERSION,
            "seedVersion": seed_version,
            "creatorLearningVersion": creator_learning_version,
            "categoryOverrideVersion": category_override_version,
            "activeRuleCount": active_rules.as_array().map_or(0, Vec::len),
            "activeRulesHash": active_rules_hash,
            "activeRules": active_rules
        },
        "immutabilityNotes": [
            "Preview snapshot hash binds the backend-generated preview's stable reviewed content returned to the frontend.",
            "Saving from this snapshot must not regenerate preview rows.",
            "This hash is identity/provenance only; it does not authorize Apply, Restore, backup execution, result logs, or file mutation."
        ]
    });
    let canonical = serde_json::to_vec(&provenance)?;
    let hash = hex::encode(sha2::Sha256::digest(canonical));

    Ok(ApplyPlanHashStamp {
        hash,
        version: PREVIEW_SNAPSHOT_HASH_VERSION.to_owned(),
        algorithm: PLAN_HASH_ALGORITHM.to_owned(),
        provenance,
    })
}

pub fn verify_hash(
    connection: &Connection,
    plan: &PersistedApplyPlan,
) -> AppResult<ApplyPlanHashVerification> {
    let Some(stored_hash) = plan
        .plan_hash
        .clone()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(ApplyPlanHashVerification {
            is_valid: false,
            status: "missing".to_owned(),
            message: "Saved ApplyPlan hash/provenance is missing.".to_owned(),
        });
    };

    if !plan.plan_provenance.is_object()
        || plan
            .plan_provenance
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
    {
        return Ok(ApplyPlanHashVerification {
            is_valid: false,
            status: "missing_provenance".to_owned(),
            message: "Saved ApplyPlan hash/provenance is missing.".to_owned(),
        });
    }

    if plan.plan_hash_version != PLAN_HASH_VERSION
        || plan.plan_hash_algorithm != PLAN_HASH_ALGORITHM
    {
        return Ok(ApplyPlanHashVerification {
            is_valid: false,
            status: "unsupported_version".to_owned(),
            message: "Saved ApplyPlan hash/provenance uses an unsupported version or algorithm."
                .to_owned(),
        });
    }

    let stamp = build_hash_stamp(connection, plan)?;
    let is_valid = stored_hash == stamp.hash && plan.plan_provenance == stamp.provenance;
    Ok(ApplyPlanHashVerification {
        is_valid,
        status: if is_valid { "valid" } else { "mismatch" }.to_owned(),
        message: if is_valid {
            "Saved ApplyPlan hash/provenance matches the persisted preview snapshot.".to_owned()
        } else {
            "Saved ApplyPlan hash/provenance no longer matches the persisted source/config/context/items. Regenerate the preview before any future confirmation."
                .to_owned()
        },
    })
}

fn build_provenance(connection: &Connection, plan: &PersistedApplyPlan) -> AppResult<Value> {
    let seed_version = optional_scalar(
        connection,
        "SELECT value FROM seed_meta WHERE key = 'seed_version' LIMIT 1",
    )?;
    let creator_learning_version = optional_scalar(
        connection,
        "SELECT value FROM app_settings WHERE key = 'creator_learning_version' LIMIT 1",
    )?;
    let category_override_version = optional_scalar(
        connection,
        "SELECT value FROM app_settings WHERE key = 'category_override_version' LIMIT 1",
    )?;
    let active_rules = load_active_rule_snapshot(connection)?;
    let active_rules_hash = hash_json(&active_rules)?;

    let mut item_values = plan
        .items
        .iter()
        .map(|item| {
            let signals = canonical_sorted(
                item.signals
                    .iter()
                    .map(|signal| {
                        json!({
                            "signalKind": signal.signal_kind,
                            "signalLabel": signal.signal_label,
                            "signalValue": signal.signal_value,
                            "evidenceLevel": signal.evidence_level,
                            "sourceSystem": signal.source_system,
                        })
                    })
                    .collect(),
            )?;
            let blockers = canonical_sorted(
                item.blockers
                    .iter()
                    .map(|blocker| {
                        json!({
                            "blockerKind": blocker.blocker_kind,
                            "reasonCode": blocker.reason_code,
                            "message": blocker.message,
                            "sourceSystem": blocker.source_system,
                        })
                    })
                    .collect(),
            )?;

            Ok(json!({
                "fileName": item.file_name,
                "currentPath": item.current_path,
                "currentRoot": item.current_root,
                "destinationPath": item.destination_path,
                "destinationRoot": item.destination_root,
                "actionKind": item.action_kind,
                "evidenceLevel": item.evidence_level,
                "bucket": item.bucket,
                "confidenceLabel": item.confidence_label,
                "itemStatus": item.item_status,
                "blocked": item.blocked,
                "reviewOnly": item.review_only,
                "validationStatus": item.validation_status,
                "conflictStatus": item.conflict_status,
                "pathPrivacyLevel": item.path_privacy_level,
                "signals": signals,
                "blockers": blockers,
                "sourceFileSnapshot": match item.file_id {
                    Some(file_id) => load_source_file_snapshot(connection, file_id)?,
                    None => Value::Null,
                },
            }))
        })
        .collect::<AppResult<Vec<_>>>()?;
    item_values = canonical_sorted(item_values)?;
    let items = item_values
        .into_iter()
        .enumerate()
        .map(|(ordinal, mut item)| {
            if let Some(object) = item.as_object_mut() {
                object.insert("ordinal".to_owned(), json!(ordinal));
            }
            item
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "planHashVersion": PLAN_HASH_VERSION,
        "planHashAlgorithm": PLAN_HASH_ALGORITHM,
        "previewSnapshotBound": plan.preview_snapshot_hash.is_some(),
        "previewSnapshotHash": plan.preview_snapshot_hash,
        "sourcePlanKind": plan.source_plan_kind,
        "sourceKind": plan.source_plan_kind,
        "sourceStagingPlanId": plan.source_staging_plan_id,
        "title": plan.title,
        "summary": plan.summary,
        "wouldTouchFiles": plan.would_touch_files,
        "confirmationRequired": plan.confirmation_required,
        "backupRequired": plan.backup_required,
        "restoreAvailable": plan.restore_available,
        "totalItems": plan.total_items,
        "applyableItems": plan.applyable_items,
        "blockedItems": plan.blocked_items,
        "reviewOnlyItems": plan.review_only_items,
        "caveats": plan.caveats,
        "sourceScope": sanitized_source_scope(connection, &plan.source_scope)?,
        "folderConfig": plan.folder_config,
        "contextTrail": plan.context_trail,
        "scanSessionAttached": plan.scan_session_id.is_some(),
        "items": items,
        "versionEvidence": {
            "sortingPreviewPlanVersion": SORTING_PREVIEW_PLAN_VERSION,
            "validationPreviewVersion": VALIDATION_PREVIEW_VERSION,
            "seedVersion": seed_version,
            "creatorLearningVersion": creator_learning_version,
            "categoryOverrideVersion": category_override_version,
            "activeRuleCount": active_rules.as_array().map_or(0, Vec::len),
            "activeRulesHash": active_rules_hash,
            "activeRules": active_rules
        },
        "immutabilityNotes": [
            "Plan hash binds preview content, item destinations, source scope, folder configuration, context trail, source file snapshots, and rule/seed/settings evidence.",
            "Lifecycle status and updated_at are excluded so cancelling a draft does not rewrite the immutable preview identity.",
            "This hash is identity/provenance only; it does not authorize Apply, Restore, backup execution, result logs, or file mutation."
        ]
    }))
}

fn sanitized_source_scope(
    connection: &Connection,
    source_scope: &Option<Value>,
) -> AppResult<Option<Value>> {
    let Some(mut scope) = source_scope.clone() else {
        return Ok(None);
    };
    if let Some(object) = scope.as_object_mut() {
        if object
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind == "selected_files")
        {
            let mut selected_file_ids = object
                .remove("fileIds")
                .and_then(|value| value.as_array().cloned())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| value.as_i64())
                .collect::<Vec<_>>();
            selected_file_ids.sort_unstable();

            let mut selected_file_snapshots = Vec::with_capacity(selected_file_ids.len());
            for file_id in selected_file_ids.iter().copied() {
                selected_file_snapshots.push(load_source_file_snapshot(connection, file_id)?);
            }
            let selected_file_snapshots = canonical_sorted(selected_file_snapshots)?;
            object.insert(
                "selectedFileCount".to_owned(),
                json!(selected_file_ids.len()),
            );
            object.insert(
                "selectedFileSnapshotsHash".to_owned(),
                json!(hash_json(&Value::Array(selected_file_snapshots))?),
            );
        }
    }
    Ok(Some(scope))
}

fn canonical_sorted(values: Vec<Value>) -> AppResult<Vec<Value>> {
    let mut keyed = values
        .into_iter()
        .map(|value| serde_json::to_string(&value).map(|key| (key, value)))
        .collect::<Result<Vec<_>, _>>()?;
    keyed.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(keyed.into_iter().map(|(_, value)| value).collect())
}

fn load_active_rule_snapshot(connection: &Connection) -> AppResult<Value> {
    let mut statement = connection.prepare(
        "SELECT rule_name, rule_template, rule_priority
         FROM rules
         WHERE enabled = 1
         ORDER BY rule_priority ASC, rule_name ASC",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok(json!({
                "ruleName": row.get::<_, String>(0)?,
                "ruleTemplate": row.get::<_, String>(1)?,
                "rulePriority": row.get::<_, i64>(2)?,
            }))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Value::Array(rows))
}

fn hash_json(value: &Value) -> AppResult<String> {
    Ok(hex::encode(sha2::Sha256::digest(serde_json::to_vec(
        value,
    )?)))
}

fn load_source_file_snapshot(connection: &Connection, file_id: i64) -> AppResult<Value> {
    let row = connection
        .query_row(
            "SELECT path, filename, extension, size, modified_at, hash, kind, subtype,
                confidence, source_location, relative_depth
             FROM files
             WHERE id = ?1",
            params![file_id],
            |row| {
                Ok(json!({
                    "path": row.get::<_, String>(0)?,
                    "filename": row.get::<_, String>(1)?,
                    "extension": row.get::<_, String>(2)?,
                    "size": row.get::<_, i64>(3)?,
                    "modifiedAt": row.get::<_, String>(4)?,
                    "hash": row.get::<_, Option<String>>(5)?,
                    "kind": row.get::<_, Option<String>>(6)?,
                    "subtype": row.get::<_, Option<String>>(7)?,
                    "confidence": row.get::<_, Option<f64>>(8)?,
                    "sourceLocation": row.get::<_, String>(9)?,
                    "relativeDepth": row.get::<_, Option<i64>>(10)?,
                }))
            },
        )
        .optional()?;

    Ok(row.unwrap_or_else(|| json!({ "missingSourceFileSnapshot": true })))
}

fn optional_scalar(connection: &Connection, sql: &str) -> AppResult<Option<String>> {
    connection
        .query_row(sql, [], |row| row.get::<_, String>(0))
        .optional()
        .map_err(AppError::from)
}
