#![cfg(test)]

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    core::{apply_plan_persistence, apply_plan_provenance, apply_plan_results},
    database,
    error::{AppError, AppResult},
    models::{
        GameInstallationConfirmationState, GameInstallationProfileStatus,
        GameInstallationRootValidationState, PersistedApplyPlan, PersistedApplyPlanStatus,
    },
};

const CONFIRMATION_BINDING_VERSION: &str = "sorting_confirmation_binding_v1";
const TOKEN_DIGEST_ALGORITHM: &str = "sha256";
const MIN_CONFIRMATION_TOKEN_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureConfirmationState {
    Issued,
    Consumed,
    Revoked,
    Expired,
}

impl FixtureConfirmationState {
    fn parse(value: &str) -> AppResult<Self> {
        match value {
            "issued" => Ok(Self::Issued),
            "consumed" => Ok(Self::Consumed),
            "revoked" => Ok(Self::Revoked),
            "expired" => Ok(Self::Expired),
            _ => Err(AppError::Message(format!(
                "Unknown fixture confirmation state: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
struct FixtureConfirmationRecord {
    id: i64,
    apply_plan_id: i64,
    apply_plan_run_id: i64,
    plan_hash: String,
    token_digest: String,
    binding_hash: String,
    context_json: String,
    issued_at_epoch: i64,
    expires_at_epoch: i64,
    state: FixtureConfirmationState,
    consumed_at_epoch: Option<i64>,
    revoked_at_epoch: Option<i64>,
}

fn ensure_confirmation_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS fixture_apply_plan_confirmations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            apply_plan_id INTEGER NOT NULL,
            apply_plan_run_id INTEGER NOT NULL UNIQUE,
            plan_hash TEXT NOT NULL,
            token_digest TEXT NOT NULL UNIQUE,
            binding_hash TEXT NOT NULL,
            context_json TEXT NOT NULL,
            issued_at_epoch INTEGER NOT NULL,
            expires_at_epoch INTEGER NOT NULL,
            state TEXT NOT NULL CHECK (state IN ('issued', 'consumed', 'revoked', 'expired')),
            consumed_at_epoch INTEGER,
            revoked_at_epoch INTEGER,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_fixture_apply_plan_confirmations_plan
            ON fixture_apply_plan_confirmations(apply_plan_id);
        CREATE INDEX IF NOT EXISTS idx_fixture_apply_plan_confirmations_state
            ON fixture_apply_plan_confirmations(state);",
    )?;
    Ok(())
}

fn token_digest(token: &str) -> AppResult<String> {
    if token.as_bytes().len() < MIN_CONFIRMATION_TOKEN_BYTES || token.trim() != token {
        return Err(AppError::Message(
            "Confirmation token is too short or contains surrounding whitespace.".to_owned(),
        ));
    }
    Ok(hex::encode(Sha256::digest(token.as_bytes())))
}

fn load_run_plan_id(connection: &Connection, run_id: i64) -> AppResult<i64> {
    connection
        .query_row(
            "SELECT apply_plan_id FROM apply_plan_runs WHERE id = ?1 AND status = 'draft_log'",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            AppError::Message(
                "Confirmation requires an existing non-blocked draft execution run.".to_owned(),
            )
        })
}

fn load_live_confirmable_plan(
    connection: &Connection,
    plan_id: i64,
) -> AppResult<PersistedApplyPlan> {
    let plan = apply_plan_persistence::get_apply_plan(connection, plan_id)?
        .ok_or_else(|| AppError::Message(format!("Saved ApplyPlan {plan_id} was not found")))?;
    if matches!(plan.status, PersistedApplyPlanStatus::Blocked | PersistedApplyPlanStatus::Cancelled)
        || plan.items.is_empty()
        || !plan.confirmation_required
        || plan.blocked_items != 0
        || plan.review_only_items != 0
        || plan.source_plan_kind != apply_plan_provenance::BACKEND_GENERATED_SORTING_PREVIEW_SOURCE_KIND
    {
        return Err(AppError::Message(
            "Saved sorting plan is not eligible for confirmation.".to_owned(),
        ));
    }
    if plan.items.iter().any(|item| {
        item.action_kind != "suggest_move"
            || item.blocked
            || item.review_only
            || item.destination_path.as_deref().is_none_or(str::is_empty)
    }) {
        return Err(AppError::Message(
            "Confirmation only accepts exact non-blocked suggest_move items with destinations."
                .to_owned(),
        ));
    }

    let verification = apply_plan_persistence::verify_apply_plan_hash(connection, plan_id)?;
    if !verification.is_valid {
        return Err(AppError::Message(format!(
            "Saved sorting plan is stale and cannot be confirmed: {}",
            verification.status
        )));
    }
    Ok(plan)
}

fn active_context_json(connection: &Connection, plan: &PersistedApplyPlan) -> AppResult<Value> {
    let active_profile = database::get_active_game_installation_profile(connection)?
        .ok_or_else(|| {
            AppError::Message(
                "Confirmation requires one active validated game installation profile.".to_owned(),
            )
        })?;
    if active_profile.status != GameInstallationProfileStatus::Valid
        || active_profile.confirmation_state != GameInstallationConfirmationState::Confirmed
    {
        return Err(AppError::Message(
            "Confirmation requires an active profile that is valid and already confirmed."
                .to_owned(),
        ));
    }

    let mut plan_roots = plan
        .items
        .iter()
        .flat_map(|item| {
            [
                Some(item.current_root.clone()),
                item.destination_root.clone(),
            ]
        })
        .flatten()
        .collect::<Vec<_>>();
    plan_roots.sort();
    plan_roots.dedup();
    if plan_roots.is_empty() {
        return Err(AppError::Message(
            "Confirmation requires explicit saved plan roots.".to_owned(),
        ));
    }
    for plan_root in &plan_roots {
        let matching_root = active_profile
            .roots
            .iter()
            .find(|root| root.root_id == *plan_root)
            .ok_or_else(|| {
                AppError::Message(format!(
                    "Active profile does not contain the saved plan root {plan_root}."
                ))
            })?;
        if matching_root.validation_state != GameInstallationRootValidationState::Valid {
            return Err(AppError::Message(format!(
                "Active profile root {plan_root} is not currently validated."
            )));
        }
    }

    let mut profile_roots = active_profile
        .roots
        .iter()
        .map(|root| {
            json!({
                "rootId": root.root_id,
                "rootRole": root.root_role,
                "configuredPath": root.configured_path,
                "validationState": root.validation_state,
            })
        })
        .collect::<Vec<_>>();
    profile_roots.sort_by_key(|value| serde_json::to_string(value).unwrap_or_default());

    Ok(json!({
        "activeProfileId": active_profile.profile_id,
        "activeProfileStatus": active_profile.status,
        "activeProfileConfirmationState": active_profile.confirmation_state,
        "activeProfileRoots": profile_roots,
        "planRoots": plan_roots,
    }))
}

fn confirmation_binding_hash(
    plan: &PersistedApplyPlan,
    run_id: i64,
    token_digest: &str,
    context: &Value,
) -> AppResult<String> {
    let plan_hash = plan.plan_hash.as_deref().ok_or_else(|| {
        AppError::Message("Confirmation requires a saved immutable plan hash.".to_owned())
    })?;
    let mut items = plan
        .items
        .iter()
        .map(|item| {
            json!({
                "itemId": item.id,
                "fileId": item.file_id,
                "currentPath": item.current_path,
                "currentRoot": item.current_root,
                "destinationPath": item.destination_path,
                "destinationRoot": item.destination_root,
                "actionKind": item.action_kind,
            })
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|value| serde_json::to_string(value).unwrap_or_default());
    let payload = json!({
        "bindingVersion": CONFIRMATION_BINDING_VERSION,
        "tokenDigestAlgorithm": TOKEN_DIGEST_ALGORITHM,
        "tokenDigest": token_digest,
        "applyPlanId": plan.id,
        "applyPlanRunId": run_id,
        "planHash": plan_hash,
        "planHashVersion": plan.plan_hash_version,
        "planHashAlgorithm": plan.plan_hash_algorithm,
        "previewSnapshotHash": plan.preview_snapshot_hash,
        "sourcePlanKind": plan.source_plan_kind,
        "context": context,
        "items": items,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

fn load_confirmation(
    connection: &Connection,
    run_id: i64,
) -> AppResult<Option<FixtureConfirmationRecord>> {
    ensure_confirmation_schema(connection)?;
    connection
        .query_row(
            "SELECT id, apply_plan_id, apply_plan_run_id, plan_hash, token_digest,
                binding_hash, context_json, issued_at_epoch, expires_at_epoch, state,
                consumed_at_epoch, revoked_at_epoch
             FROM fixture_apply_plan_confirmations
             WHERE apply_plan_run_id = ?1",
            params![run_id],
            |row| {
                let state: String = row.get(9)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    state,
                    row.get::<_, Option<i64>>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                ))
            },
        )
        .optional()?
        .map(|row| {
            Ok(FixtureConfirmationRecord {
                id: row.0,
                apply_plan_id: row.1,
                apply_plan_run_id: row.2,
                plan_hash: row.3,
                token_digest: row.4,
                binding_hash: row.5,
                context_json: row.6,
                issued_at_epoch: row.7,
                expires_at_epoch: row.8,
                state: FixtureConfirmationState::parse(&row.9)?,
                consumed_at_epoch: row.10,
                revoked_at_epoch: row.11,
            })
        })
        .transpose()
}

fn issue_confirmation(
    connection: &Connection,
    plan_id: i64,
    run_id: i64,
    raw_token: &str,
    now_epoch: i64,
    ttl_seconds: i64,
) -> AppResult<FixtureConfirmationRecord> {
    if ttl_seconds <= 0 {
        return Err(AppError::Message(
            "Confirmation lifetime must be positive.".to_owned(),
        ));
    }
    ensure_confirmation_schema(connection)?;
    if load_run_plan_id(connection, run_id)? != plan_id {
        return Err(AppError::Message(
            "Confirmation run does not belong to the requested plan.".to_owned(),
        ));
    }
    if load_confirmation(connection, run_id)?.is_some() {
        return Err(AppError::Message(
            "This run already has a confirmation record.".to_owned(),
        ));
    }
    let plan = load_live_confirmable_plan(connection, plan_id)?;
    let digest = token_digest(raw_token)?;
    let context = active_context_json(connection, &plan)?;
    let context_json = serde_json::to_string(&context)?;
    let binding_hash = confirmation_binding_hash(&plan, run_id, &digest, &context)?;
    let plan_hash = plan.plan_hash.clone().ok_or_else(|| {
        AppError::Message("Confirmation requires a saved immutable plan hash.".to_owned())
    })?;
    let expires_at_epoch = now_epoch
        .checked_add(ttl_seconds)
        .ok_or_else(|| AppError::Message("Confirmation expiry overflowed.".to_owned()))?;
    connection.execute(
        "INSERT INTO fixture_apply_plan_confirmations (
            apply_plan_id, apply_plan_run_id, plan_hash, token_digest, binding_hash,
            context_json, issued_at_epoch, expires_at_epoch, state
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'issued')",
        params![
            plan_id,
            run_id,
            plan_hash,
            digest,
            binding_hash,
            context_json,
            now_epoch,
            expires_at_epoch,
        ],
    )?;
    load_confirmation(connection, run_id)?.ok_or_else(|| {
        AppError::Message("Issued confirmation could not be reloaded.".to_owned())
    })
}

fn consume_confirmation(
    connection: &Connection,
    run_id: i64,
    raw_token: &str,
    now_epoch: i64,
) -> AppResult<FixtureConfirmationRecord> {
    let record = load_confirmation(connection, run_id)?.ok_or_else(|| {
        AppError::Message("No confirmation exists for this execution run.".to_owned())
    })?;
    if record.state != FixtureConfirmationState::Issued {
        return Err(AppError::Message(
            "Confirmation is no longer available for first use.".to_owned(),
        ));
    }
    if now_epoch > record.expires_at_epoch {
        connection.execute(
            "UPDATE fixture_apply_plan_confirmations
             SET state = 'expired', updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND state = 'issued'",
            params![record.id],
        )?;
        return Err(AppError::Message("Confirmation has expired.".to_owned()));
    }
    let presented_digest = token_digest(raw_token)?;
    if presented_digest != record.token_digest {
        return Err(AppError::Message(
            "Confirmation token does not match this run.".to_owned(),
        ));
    }
    let plan_id = load_run_plan_id(connection, run_id)?;
    if plan_id != record.apply_plan_id {
        return Err(AppError::Message(
            "Confirmation run/plan binding changed.".to_owned(),
        ));
    }
    let plan = load_live_confirmable_plan(connection, plan_id)?;
    if plan.plan_hash.as_deref() != Some(record.plan_hash.as_str()) {
        return Err(AppError::Message(
            "Confirmation plan hash no longer matches the saved plan.".to_owned(),
        ));
    }
    let context = active_context_json(connection, &plan)?;
    let expected_binding = confirmation_binding_hash(&plan, run_id, &presented_digest, &context)?;
    if expected_binding != record.binding_hash || serde_json::to_string(&context)? != record.context_json {
        return Err(AppError::Message(
            "Confirmation context or exact item binding changed after approval.".to_owned(),
        ));
    }
    let changed = connection.execute(
        "UPDATE fixture_apply_plan_confirmations
         SET state = 'consumed', consumed_at_epoch = ?1, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?2 AND state = 'issued'",
        params![now_epoch, record.id],
    )?;
    if changed != 1 {
        return Err(AppError::Message(
            "Confirmation was already consumed by another execution attempt.".to_owned(),
        ));
    }
    load_confirmation(connection, run_id)?.ok_or_else(|| {
        AppError::Message("Consumed confirmation could not be reloaded.".to_owned())
    })
}

fn revoke_confirmation(
    connection: &Connection,
    run_id: i64,
    now_epoch: i64,
) -> AppResult<FixtureConfirmationRecord> {
    let record = load_confirmation(connection, run_id)?.ok_or_else(|| {
        AppError::Message("No confirmation exists for this execution run.".to_owned())
    })?;
    let changed = connection.execute(
        "UPDATE fixture_apply_plan_confirmations
         SET state = 'revoked', revoked_at_epoch = ?1, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?2 AND state = 'issued'",
        params![now_epoch, record.id],
    )?;
    if changed != 1 {
        return Err(AppError::Message(
            "Only an unused issued confirmation can be revoked.".to_owned(),
        ));
    }
    load_confirmation(connection, run_id)?.ok_or_else(|| {
        AppError::Message("Revoked confirmation could not be reloaded.".to_owned())
    })
}

fn verify_consumed_confirmation_for_recovery(
    connection: &Connection,
    run_id: i64,
) -> AppResult<FixtureConfirmationRecord> {
    let record = load_confirmation(connection, run_id)?.ok_or_else(|| {
        AppError::Message("No historical confirmation exists for this run.".to_owned())
    })?;
    if record.state != FixtureConfirmationState::Consumed || record.consumed_at_epoch.is_none() {
        return Err(AppError::Message(
            "Recovery requires a previously consumed confirmation.".to_owned(),
        ));
    }
    let run_plan_id: i64 = connection
        .query_row(
            "SELECT apply_plan_id FROM apply_plan_runs WHERE id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Message("Historical execution run is missing.".to_owned()))?;
    if run_plan_id != record.apply_plan_id {
        return Err(AppError::Message(
            "Historical confirmation run/plan binding changed.".to_owned(),
        ));
    }
    let plan = apply_plan_persistence::get_apply_plan(connection, record.apply_plan_id)?
        .ok_or_else(|| AppError::Message("Historical confirmed plan is missing.".to_owned()))?;
    if plan.plan_hash.as_deref() != Some(record.plan_hash.as_str()) {
        return Err(AppError::Message(
            "Historical confirmation plan hash changed.".to_owned(),
        ));
    }
    let stored_context: Value = serde_json::from_str(&record.context_json)?;
    let expected_binding =
        confirmation_binding_hash(&plan, run_id, &record.token_digest, &stored_context)?;
    if expected_binding != record.binding_hash {
        return Err(AppError::Message(
            "Historical confirmation binding no longer matches the saved plan.".to_owned(),
        ));
    }
    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database,
        models::{
            BuildApplyPlanFromStagingPlanRequest, CreateApplyPlanRunLogRequest,
            GenerateSortingPreviewPlanRequest, GenerateSortingPreviewPlanScope, LibrarySettings,
        },
    };

    const PROFILE_A: &str = "fixture-profile-a";
    const PROFILE_B: &str = "fixture-profile-b";
    const TOKEN_A: &str = "confirmation-token-a-0123456789";
    const TOKEN_B: &str = "confirmation-token-b-0123456789";

    fn memory_connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("memory db");
        database::initialize(&mut connection).expect("schema");
        connection
    }

    fn insert_profile(connection: &Connection, profile_id: &str, mods_path: &str) {
        connection
            .execute(
                "INSERT INTO game_installation_profiles (
                    profile_id, profile_name, game_id, operating_environment, status,
                    detection_method, detection_evidence_json, confirmation_state,
                    confirmed_at, last_validated_at, created_at, updated_at
                 ) VALUES (?1, ?2, 'sims4', 'native_windows', 'valid', 'manual', '{}',
                    'confirmed', '2026-08-09T00:00:00Z', '2026-08-09T00:00:00Z',
                    '2026-08-09T00:00:00Z', '2026-08-09T00:00:00Z')",
                params![profile_id, profile_id],
            )
            .expect("insert fixture profile");
        for (root_id, role, path) in [
            ("mods", "installed_mods", mods_path),
            ("tray", "installed_tray", "C:/Sims/Tray"),
        ] {
            connection
                .execute(
                    "INSERT INTO game_installation_roots (
                        profile_id, root_id, root_role, configured_path, required,
                        validation_state, filesystem_capabilities_json, last_validated_at,
                        created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, 1, 'valid', '{}',
                        '2026-08-09T00:00:00Z', '2026-08-09T00:00:00Z', '2026-08-09T00:00:00Z')",
                    params![profile_id, root_id, role, path],
                )
                .expect("insert fixture profile root");
        }
    }

    fn set_active_profile(connection: &Connection, profile_id: &str) {
        connection
            .execute(
                "INSERT INTO app_settings(key, value, source, updated_at)
                 VALUES ('active_game_installation_profile_id', ?1, 'user', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, source = 'user',
                     updated_at = CURRENT_TIMESTAMP",
                params![profile_id],
            )
            .expect("set active fixture profile");
    }

    fn setup_confirmable_plan(connection: &mut Connection) -> (i64, i64) {
        insert_profile(connection, PROFILE_A, "C:/Sims/Mods");
        insert_profile(connection, PROFILE_B, "D:/OtherSims/Mods");
        set_active_profile(connection, PROFILE_A);
        connection
            .execute(
                "INSERT INTO files (
                    id, path, filename, extension, size, modified_at, hash, kind, subtype,
                    confidence, safety_notes, parser_warnings, source_location, relative_depth
                 ) VALUES
                    (1, 'C:/Sims/Mods/Unsorted/a.package', 'a.package', 'package', 11,
                        '2026-08-09T00:00:00Z', 'hash-a', 'Gameplay', NULL, 0.95,
                        '[]', '[]', 'mods', 1),
                    (2, 'C:/Sims/Mods/Unsorted/b.package', 'b.package', 'package', 12,
                        '2026-08-09T00:00:00Z', 'hash-b', 'CAS', 'Hair', 0.95,
                        '[]', '[]', 'mods', 1)",
                [],
            )
            .expect("insert confirmable fixture files");
        let settings = LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        };
        let saved = apply_plan_persistence::build_apply_plan_from_staging_plan(
            connection,
            &settings,
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
            },
        )
        .expect("build confirmable backend sorting plan");
        let plan_id = saved.plan_id;
        let plan = apply_plan_persistence::get_apply_plan(connection, plan_id)
            .expect("load confirmable plan")
            .expect("confirmable plan exists");
        assert!(plan.items.iter().all(|item| item.action_kind == "suggest_move"));
        assert!(
            apply_plan_persistence::verify_apply_plan_hash(connection, plan_id)
                .expect("verify confirmable plan")
                .is_valid
        );
        let run_id = apply_plan_results::create_apply_plan_run_log(
            connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: Some(plan.total_items),
                skipped_items: None,
                failed_items: None,
                summary: Some("DB-only confirmation fixture run".to_owned()),
            },
        )
        .expect("create confirmation fixture run")
        .id;
        (plan_id, run_id)
    }

    fn create_second_run(connection: &Connection, plan_id: i64) -> i64 {
        apply_plan_results::create_apply_plan_run_log(
            connection,
            CreateApplyPlanRunLogRequest {
                apply_plan_id: plan_id,
                status: None,
                backup_strategy: None,
                confirmation_token: None,
                total_items: None,
                skipped_items: None,
                failed_items: None,
                summary: Some("Second DB-only confirmation fixture run".to_owned()),
            },
        )
        .expect("create second confirmation fixture run")
        .id
    }

    fn create_second_plan(connection: &mut Connection) -> i64 {
        let settings = LibrarySettings {
            mods_path: Some("C:/Sims/Mods".to_owned()),
            tray_path: Some("C:/Sims/Tray".to_owned()),
            ..Default::default()
        };
        apply_plan_persistence::build_apply_plan_from_staging_plan(
            connection,
            &settings,
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
            },
        )
        .expect("build second confirmation fixture plan")
        .plan_id
    }

    #[test]
    fn exact_confirmation_is_one_use_and_recovery_can_verify_historical_authority() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        let issued = issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 1_000, 60)
            .expect("issue exact confirmation");
        assert_eq!(issued.state, FixtureConfirmationState::Issued);
        assert_eq!(issued.apply_plan_id, plan_id);
        assert_eq!(issued.apply_plan_run_id, run_id);
        assert_eq!(issued.issued_at_epoch, 1_000);
        assert_eq!(issued.token_digest, token_digest(TOKEN_A).unwrap());
        assert_ne!(issued.token_digest, TOKEN_A);
        let plaintext_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM fixture_apply_plan_confirmations WHERE token_digest = ?1 OR context_json LIKE ?2",
                params![TOKEN_A, format!("%{TOKEN_A}%")],
                |row| row.get(0),
            )
            .expect("check plaintext token persistence");
        assert_eq!(plaintext_count, 0);
        let plan = apply_plan_persistence::get_apply_plan(&connection, plan_id)
            .expect("reload confirmed plan")
            .expect("confirmed plan exists");
        assert!(plan.items.iter().all(|item| item.action_kind == "suggest_move"));

        let consumed = consume_confirmation(&connection, run_id, TOKEN_A, 1_010)
            .expect("consume exact confirmation once");
        assert_eq!(consumed.state, FixtureConfirmationState::Consumed);
        assert_eq!(consumed.consumed_at_epoch, Some(1_010));
        consume_confirmation(&connection, run_id, TOKEN_A, 1_011)
            .expect_err("confirmation replay must fail");

        connection
            .execute(
                "UPDATE files SET path = 'C:/Sims/Mods/Gameplay/a.package' WHERE id = 1",
                [],
            )
            .expect("simulate truthful post-start Library movement");
        assert!(
            !apply_plan_persistence::verify_apply_plan_hash(&connection, plan_id)
                .expect("live plan hash after simulated execution")
                .is_valid
        );
        let historical = verify_consumed_confirmation_for_recovery(&connection, run_id)
            .expect("historical recovery authority remains verifiable");
        assert_eq!(historical.state, FixtureConfirmationState::Consumed);
        assert_eq!(historical.id, consumed.id);
    }

    #[test]
    fn confirmation_rejects_cross_run_wrong_token_and_profile_context_replay() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 2_000, 60)
            .expect("issue confirmation for first run");
        let run_two = create_second_run(&connection, plan_id);
        consume_confirmation(&connection, run_two, TOKEN_A, 2_010)
            .expect_err("confirmation cannot move to another run");
        consume_confirmation(&connection, run_id, TOKEN_B, 2_010)
            .expect_err("wrong token cannot consume confirmation");
        set_active_profile(&connection, PROFILE_B);
        consume_confirmation(&connection, run_id, TOKEN_A, 2_010)
            .expect_err("changed active profile/root context must invalidate unused approval");
        let record = load_confirmation(&connection, run_id)
            .expect("load context-rejected confirmation")
            .expect("context-rejected confirmation exists");
        assert_eq!(record.state, FixtureConfirmationState::Issued);
    }

    #[test]
    fn confirmation_requires_an_active_confirmed_profile_with_valid_plan_roots() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        connection
            .execute(
                "DELETE FROM app_settings WHERE key = 'active_game_installation_profile_id'",
                [],
            )
            .expect("remove active profile setting");
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 2_050, 60)
            .expect_err("confirmation must require an active profile");

        set_active_profile(&connection, PROFILE_A);
        connection
            .execute(
                "UPDATE game_installation_profiles
                 SET confirmation_state = 'unconfirmed'
                 WHERE profile_id = ?1",
                params![PROFILE_A],
            )
            .expect("make active profile unconfirmed");
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 2_051, 60)
            .expect_err("confirmation must require a confirmed active profile");

        connection
            .execute(
                "UPDATE game_installation_profiles
                 SET confirmation_state = 'confirmed'
                 WHERE profile_id = ?1",
                params![PROFILE_A],
            )
            .expect("restore active profile confirmation");
        connection
            .execute(
                "UPDATE game_installation_roots
                 SET validation_state = 'needs_review'
                 WHERE profile_id = ?1 AND root_id = 'mods'",
                params![PROFILE_A],
            )
            .expect("make active Mods root need review");
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 2_052, 60)
            .expect_err("confirmation must require every used plan root to be valid");
        assert!(
            load_confirmation(&connection, run_id)
                .expect("load profile-rejected confirmation state")
                .is_none(),
            "failed profile/root checks must not issue any approval record"
        );
    }

    #[test]
    fn confirmation_rejects_run_rebound_to_a_different_saved_plan() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 2_100, 60)
            .expect("issue confirmation before run rebind");
        let second_plan_id = create_second_plan(&mut connection);
        assert_ne!(second_plan_id, plan_id);
        connection
            .execute(
                "UPDATE apply_plan_runs SET apply_plan_id = ?1 WHERE id = ?2",
                params![second_plan_id, run_id],
            )
            .expect("rebind fixture run to another saved plan");
        consume_confirmation(&connection, run_id, TOKEN_A, 2_110)
            .expect_err("confirmation must not follow a run rebound to another plan");
        assert_eq!(
            load_confirmation(&connection, run_id)
                .expect("load rebound confirmation")
                .expect("rebound confirmation exists")
                .state,
            FixtureConfirmationState::Issued,
            "failed cross-plan use must not consume the original approval"
        );
    }

    #[test]
    fn approved_destination_tamper_blocks_consumption_without_consuming_authority() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 2_200, 60)
            .expect("issue confirmation before destination tamper");
        connection
            .execute(
                "UPDATE apply_plan_items
                 SET destination_path = 'C:/Sims/Mods/Gameplay/tampered.package'
                 WHERE apply_plan_id = ?1 AND id = (
                    SELECT MIN(id) FROM apply_plan_items WHERE apply_plan_id = ?1
                 )",
                params![plan_id],
            )
            .expect("tamper approved destination");
        assert!(
            !apply_plan_persistence::verify_apply_plan_hash(&connection, plan_id)
                .expect("verify tampered destination plan hash")
                .is_valid
        );
        consume_confirmation(&connection, run_id, TOKEN_A, 2_210)
            .expect_err("approved destination tamper must block consumption");
        assert_eq!(
            load_confirmation(&connection, run_id)
                .expect("load destination-tampered confirmation")
                .expect("destination-tampered confirmation exists")
                .state,
            FixtureConfirmationState::Issued,
            "stale/tampered approval must remain unconsumed"
        );
    }

    #[test]
    fn stale_plan_or_library_evidence_cannot_be_issued_or_consumed() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 3_000, 60)
            .expect("issue before staleness");
        connection
            .execute("UPDATE files SET hash = 'changed-hash' WHERE id = 2", [])
            .expect("change Library evidence after approval");
        consume_confirmation(&connection, run_id, TOKEN_A, 3_010)
            .expect_err("stale Library evidence must block consumption");

        let run_two = create_second_run(&connection, plan_id);
        issue_confirmation(&connection, plan_id, run_two, TOKEN_B, 3_020, 60)
            .expect_err("stale plan must not receive a fresh confirmation");
    }

    #[test]
    fn expired_and_revoked_confirmations_fail_closed() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 4_000, 10)
            .expect("issue expiring confirmation");
        consume_confirmation(&connection, run_id, TOKEN_A, 4_011)
            .expect_err("expired confirmation must fail");
        assert_eq!(
            load_confirmation(&connection, run_id)
                .unwrap()
                .unwrap()
                .state,
            FixtureConfirmationState::Expired
        );

        let run_two = create_second_run(&connection, plan_id);
        issue_confirmation(&connection, plan_id, run_two, TOKEN_B, 4_020, 60)
            .expect("issue revocable confirmation");
        let revoked = revoke_confirmation(&connection, run_two, 4_021)
            .expect("revoke unused confirmation");
        assert_eq!(revoked.state, FixtureConfirmationState::Revoked);
        assert_eq!(revoked.revoked_at_epoch, Some(4_021));
        consume_confirmation(&connection, run_two, TOKEN_B, 4_022)
            .expect_err("revoked confirmation must not be consumable");
        verify_consumed_confirmation_for_recovery(&connection, run_two)
            .expect_err("revoked confirmation is not historical execution authority");
    }

    #[test]
    fn confirmation_prototype_remains_database_only_and_test_only() {
        let source = include_str!("apply_plan_confirmation_prototype.rs");
        for forbidden in [
            ["std", "::", "fs"].concat(),
            ["fs", "::"].concat(),
            ["File", "::", "create"].concat(),
            ["File", "::", "open"].concat(),
            ["tauri", "::", "command"].concat(),
        ] {
            assert!(
                !source.contains(&forbidden),
                "confirmation authority must stay database-only/test-only; found forbidden source fragment {forbidden}"
            );
        }
        assert!(source.starts_with("#![cfg(test)]"));
        let core_registration = include_str!("mod.rs");
        assert!(core_registration.contains(
            "#[cfg(test)]\npub mod apply_plan_confirmation_prototype;"
        ));
    }

    #[test]
    fn confirmation_binding_tamper_fails_without_mutating_the_saved_plan() {
        let mut connection = memory_connection();
        let (plan_id, run_id) = setup_confirmable_plan(&mut connection);
        issue_confirmation(&connection, plan_id, run_id, TOKEN_A, 5_000, 60)
            .expect("issue confirmation before tamper");
        connection
            .execute(
                "UPDATE fixture_apply_plan_confirmations SET binding_hash = 'tampered' WHERE apply_plan_run_id = ?1",
                params![run_id],
            )
            .expect("tamper confirmation binding");
        consume_confirmation(&connection, run_id, TOKEN_A, 5_010)
            .expect_err("tampered confirmation binding must fail");
        let plan = apply_plan_persistence::get_apply_plan(&connection, plan_id)
            .expect("reload plan after confirmation tamper")
            .expect("plan survives confirmation tamper");
        assert!(plan.items.iter().all(|item| item.action_kind == "suggest_move"));
        assert!(plan.items.iter().all(|item| !item.blocked && !item.review_only));
    }
}
