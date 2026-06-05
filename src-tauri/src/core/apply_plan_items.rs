use rusqlite::{params, Connection, Row, Transaction};

use crate::{
    core::apply_plan_persistence_rows,
    error::{AppError, AppResult},
    models::{
        PersistedApplyPlanBlocker, PersistedApplyPlanItem, PersistedApplyPlanItemStatus,
        PersistedApplyPlanSignal, StagingPlan, StagingPlanActionKind, StagingPlanBucket,
        StagingPlanConfidenceLabel, StagingPlanEvidenceLevel, StagingPlanStatus,
    },
};

const PATH_PRIVACY_LEVEL: &str = "local_full_path_required";
const SOURCE_SYSTEM: &str = "staging_plan";

pub(crate) struct PreparedApplyPlanItem {
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
    pub(crate) item_status: PersistedApplyPlanItemStatus,
    blocked: bool,
    pub(crate) review_only: bool,
    signals: Vec<String>,
    blockers: Vec<PreparedApplyPlanBlocker>,
}

struct PreparedApplyPlanBlocker {
    kind: String,
    code: String,
    message: String,
}

pub(crate) struct ApplyPlanItemRow {
    pub(crate) id: i64,
    pub(crate) apply_plan_id: i64,
    pub(crate) source_item_id: Option<String>,
    pub(crate) file_id: Option<i64>,
    pub(crate) file_name: String,
    pub(crate) current_path: String,
    pub(crate) current_root: String,
    pub(crate) destination_path: Option<String>,
    pub(crate) destination_root: Option<String>,
    pub(crate) action_kind: String,
    pub(crate) evidence_level: String,
    pub(crate) bucket: Option<String>,
    pub(crate) confidence_label: Option<String>,
    pub(crate) item_status: String,
    pub(crate) blocked: i64,
    pub(crate) review_only: i64,
    pub(crate) validation_status: Option<String>,
    pub(crate) conflict_status: Option<String>,
    pub(crate) path_privacy_level: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

pub(crate) fn prepare_apply_plan_items(
    source_plan: &StagingPlan,
) -> AppResult<Vec<PreparedApplyPlanItem>> {
    source_plan
        .items
        .iter()
        .map(|item| {
            let mut blockers = item
                .blocked_reasons
                .iter()
                .map(|reason| PreparedApplyPlanBlocker {
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
                blockers.push(PreparedApplyPlanBlocker {
                    kind: "blocked".to_owned(),
                    code: "missing_current_path".to_owned(),
                    message: "Current path was not present in the preview source.".to_owned(),
                });
            }
            if source_plan.status == StagingPlanStatus::Blocked {
                blockers.push(PreparedApplyPlanBlocker {
                    kind: "blocked".to_owned(),
                    code: "source_plan_blocked".to_owned(),
                    message: "Source preview plan was blocked.".to_owned(),
                });
            }

            let review_only = is_review_only_item(item);
            if review_only {
                blockers.push(PreparedApplyPlanBlocker {
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

            Ok(PreparedApplyPlanItem {
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

pub(crate) fn insert_apply_plan_items(
    transaction: &Transaction<'_>,
    plan_id: i64,
    prepared_items: Vec<PreparedApplyPlanItem>,
    now: &str,
) -> AppResult<()> {
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

    Ok(())
}

pub(crate) fn load_apply_plan_items(
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
        .query_map(params![plan_id], apply_plan_item_row_from_row)?
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
            item_status: apply_plan_persistence_rows::parse_item_status(&row.item_status)?,
            blocked: apply_plan_persistence_rows::int_to_bool(row.blocked),
            review_only: apply_plan_persistence_rows::int_to_bool(row.review_only),
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

pub(crate) fn apply_plan_item_row_from_row(row: &Row<'_>) -> rusqlite::Result<ApplyPlanItemRow> {
    Ok(ApplyPlanItemRow {
        id: row.get(0)?,
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

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::apply_plan_item_row_from_row;

    #[test]
    fn row_mapper_preserves_status_and_validation_columns() {
        let connection = Connection::open_in_memory().expect("memory db");

        let item_row = connection
            .query_row(
                "SELECT 44, 43, 'item-source', 99, 'file.package',
                    'C:/Sims/Mods/file.package', 'mods', 'C:/Sims/Mods/Sorted/file.package', 'mods',
                    'suggest_move', 'strong', 'cas', 'high', 'draft_candidate',
                    0, 1, 'valid', 'none', 'local_full_path_required',
                    '2026-01-04T03:04:00Z', '2026-01-04T03:04:06Z'",
                [],
                apply_plan_item_row_from_row,
            )
            .expect("map ApplyPlan item row");

        assert_eq!(item_row.id, 44);
        assert_eq!(item_row.apply_plan_id, 43);
        assert_eq!(item_row.source_item_id.as_deref(), Some("item-source"));
        assert_eq!(item_row.file_id, Some(99));
        assert_eq!(item_row.file_name, "file.package");
        assert_eq!(item_row.current_root, "mods");
        assert_eq!(item_row.destination_root.as_deref(), Some("mods"));
        assert_eq!(item_row.action_kind, "suggest_move");
        assert_eq!(item_row.evidence_level, "strong");
        assert_eq!(item_row.bucket.as_deref(), Some("cas"));
        assert_eq!(item_row.confidence_label.as_deref(), Some("high"));
        assert_eq!(item_row.item_status, "draft_candidate");
        assert_eq!(item_row.blocked, 0);
        assert_eq!(item_row.review_only, 1);
        assert_eq!(item_row.validation_status.as_deref(), Some("valid"));
        assert_eq!(item_row.conflict_status.as_deref(), Some("none"));
        assert_eq!(item_row.path_privacy_level, "local_full_path_required");
    }
}
