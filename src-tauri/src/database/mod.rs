pub mod schema;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    error::{AppError, AppResult},
    models::{
        DownloadsTimelineEntry, GameInstallationConfirmationState,
        GameInstallationDetectionMethod, GameInstallationProfile,
        GameInstallationProfileStatus, GameInstallationRoot,
        GameInstallationRootValidationState, GameOperatingEnvironment, LibrarySettings,
    },
    seed::{SeedCreator, SeedPack},
};

const APPLY_PLAN_PERSISTENCE_SCHEMA_SQL: &str =
    include_str!("../../../database/migrations/0003_applyplan_persistence_foundation.sql");
const APPLY_PLAN_RESULT_RESTORE_SCHEMA_SQL: &str =
    include_str!("../../../database/migrations/0004_applyplan_result_restore_foundation.sql");
const APPLY_PLAN_CONTEXT_SNAPSHOTS_SCHEMA_SQL: &str =
    include_str!("../../../database/migrations/0005_applyplan_context_snapshots.sql");
const APPLY_PLAN_HASH_PROVENANCE_SCHEMA_SQL: &str =
    include_str!("../../../database/migrations/0006_applyplan_hash_provenance_v1.sql");
const APPLY_PLAN_PREVIEW_SNAPSHOTS_SCHEMA_SQL: &str =
    include_str!("../../../database/migrations/0007_applyplan_preview_snapshots_v2.sql");
const GAME_INSTALLATION_PROFILES_SCHEMA_SQL: &str =
    include_str!("../../../database/migrations/0008_game_installation_profiles_v1.sql");

const LEGACY_GAME_INSTALLATION_PROFILE_ID: &str = "legacy-sims4-default";
const ACTIVE_GAME_INSTALLATION_PROFILE_SETTING: &str =
    "active_game_installation_profile_id";

#[derive(Debug, Clone)]
pub struct UserCategoryOverride {
    pub match_path: String,
    pub kind: String,
    pub subtype: Option<String>,
}

pub fn initialize(connection: &mut Connection) -> AppResult<()> {
    let has_files_table = table_exists(connection, "files")?;

    if !has_files_table {
        connection.execute_batch(schema::INITIAL_SCHEMA_SQL)?;
    } else {
        ensure_migration_table(connection)?;
    }

    ensure_schema(connection)?;

    let version_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    if version_exists.is_none() {
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![1_i64, "initial_schema"],
        )?;
    }

    // Migration v2: add rejected_at column to download_items
    let v2_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 2",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v2_exists.is_none() {
        let column_exists: bool = connection
            .query_row(
                "SELECT 1 FROM pragma_table_info('download_items') WHERE name = 'rejected_at'",
                [],
                |_row| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !column_exists {
            connection.execute("ALTER TABLE download_items ADD COLUMN rejected_at TEXT", [])?;
        }
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![2_i64, "add_rejected_at"],
        )?;
    }

    let v3_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 3",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v3_exists.is_none() {
        ensure_apply_plan_schema(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![3_i64, "applyplan_persistence_foundation"],
        )?;
    }

    let v4_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 4",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v4_exists.is_none() {
        ensure_apply_plan_result_restore_schema(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![4_i64, "applyplan_result_restore_foundation"],
        )?;
    }

    let v5_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 5",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v5_exists.is_none() {
        ensure_apply_plan_context_snapshot_schema(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![5_i64, "applyplan_context_snapshots"],
        )?;
    }

    let v6_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 6",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v6_exists.is_none() {
        ensure_apply_plan_hash_provenance_schema(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![6_i64, "applyplan_hash_provenance_v1"],
        )?;
    }

    let v7_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 7",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v7_exists.is_none() {
        ensure_apply_plan_preview_snapshot_schema(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![7_i64, "applyplan_preview_snapshots_v2"],
        )?;
    }

    let v8_exists: Option<i64> = connection
        .query_row(
            "SELECT version FROM schema_migrations WHERE version = 8",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if v8_exists.is_none() {
        ensure_game_installation_profile_schema(connection)?;
        backfill_legacy_game_installation_profile(connection)?;
        connection.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![8_i64, "game_installation_profiles_v1"],
        )?;
    }

    Ok(())
}

fn ensure_game_installation_profile_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(GAME_INSTALLATION_PROFILES_SCHEMA_SQL)?;
    Ok(())
}

fn current_native_operating_environment_storage() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "native_windows"
    }
    #[cfg(target_os = "macos")]
    {
        "native_macos"
    }
    #[cfg(target_os = "linux")]
    {
        "native_linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "unknown"
    }
}

fn backfill_legacy_game_installation_profile(connection: &mut Connection) -> AppResult<()> {
    let existing_profiles: i64 = connection.query_row(
        "SELECT COUNT(*) FROM game_installation_profiles",
        [],
        |row| row.get(0),
    )?;
    if existing_profiles > 0 {
        return Ok(());
    }

    let mods_path = get_app_setting(connection, "mods_path")?;
    let tray_path = get_app_setting(connection, "tray_path")?;
    let downloads_path = get_app_setting(connection, "downloads_path")?;
    if mods_path.is_none() && tray_path.is_none() && downloads_path.is_none() {
        return Ok(());
    }

    let now = Utc::now().to_rfc3339();
    let transaction = connection.transaction()?;
    transaction.execute(
        "INSERT INTO game_installation_profiles (
            profile_id,
            profile_name,
            game_id,
            operating_environment,
            status,
            detection_method,
            detection_evidence_json,
            confirmation_state,
            confirmed_at,
            last_validated_at,
            created_at,
            updated_at
         ) VALUES (?1, ?2, 'sims4', ?3, 'needs_review', 'legacy_settings_migration', ?4, 'confirmed', ?5, NULL, ?5, ?5)",
        params![
            LEGACY_GAME_INSTALLATION_PROFILE_ID,
            "Current Sims 4 setup",
            current_native_operating_environment_storage(),
            r#"{"source":"legacy_app_settings"}"#,
            now,
        ],
    )?;

    upsert_legacy_game_installation_root(
        &transaction,
        "mods",
        "installed_mods",
        mods_path.as_deref(),
        true,
        &now,
    )?;
    upsert_legacy_game_installation_root(
        &transaction,
        "tray",
        "installed_tray",
        tray_path.as_deref(),
        true,
        &now,
    )?;
    upsert_legacy_game_installation_root(
        &transaction,
        "downloads",
        "intake_downloads",
        downloads_path.as_deref(),
        false,
        &now,
    )?;

    transaction.execute(
        "INSERT INTO app_settings (key, value, source, updated_at)
         VALUES (?1, ?2, 'user', ?3)
         ON CONFLICT(key) DO UPDATE SET
            value = excluded.value,
            source = excluded.source,
            updated_at = excluded.updated_at",
        params![
            ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
            LEGACY_GAME_INSTALLATION_PROFILE_ID,
            now,
        ],
    )?;
    transaction.commit()?;
    Ok(())
}

fn upsert_legacy_game_installation_root(
    transaction: &rusqlite::Transaction<'_>,
    root_id: &str,
    root_role: &str,
    configured_path: Option<&str>,
    required: bool,
    now: &str,
) -> AppResult<()> {
    let Some(configured_path) = configured_path.map(str::trim).filter(|path| !path.is_empty())
    else {
        transaction.execute(
            "DELETE FROM game_installation_roots WHERE profile_id = ?1 AND root_id = ?2",
            params![LEGACY_GAME_INSTALLATION_PROFILE_ID, root_id],
        )?;
        return Ok(());
    };

    transaction.execute(
        "INSERT INTO game_installation_roots (
            profile_id,
            root_id,
            root_role,
            configured_path,
            required,
            validation_state,
            filesystem_capabilities_json,
            last_validated_at,
            created_at,
            updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, 'unvalidated', '{}', NULL, ?6, ?6)
         ON CONFLICT(profile_id, root_id) DO UPDATE SET
            root_role = excluded.root_role,
            configured_path = excluded.configured_path,
            required = excluded.required,
            validation_state = 'unvalidated',
            filesystem_capabilities_json = '{}',
            last_validated_at = NULL,
            updated_at = excluded.updated_at",
        params![
            LEGACY_GAME_INSTALLATION_PROFILE_ID,
            root_id,
            root_role,
            configured_path,
            if required { 1_i64 } else { 0_i64 },
            now,
        ],
    )?;
    Ok(())
}

fn table_exists(connection: &Connection, table_name: &str) -> AppResult<bool> {
    let exists = connection
        .query_row(
            "SELECT 1
             FROM sqlite_master
             WHERE type = 'table' AND name = ?1
             LIMIT 1",
            params![table_name],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();

    Ok(exists)
}

fn ensure_apply_plan_context_snapshot_schema(connection: &Connection) -> AppResult<()> {
    let _migration_sql = APPLY_PLAN_CONTEXT_SNAPSHOTS_SCHEMA_SQL;
    if table_exists(connection, "apply_plans")? {
        ensure_column(connection, "apply_plans", "folder_config_json", "TEXT")?;
        ensure_column(
            connection,
            "apply_plans",
            "context_trail_json",
            "TEXT NOT NULL DEFAULT '[]'",
        )?;
    }

    Ok(())
}

fn ensure_apply_plan_hash_provenance_schema(connection: &Connection) -> AppResult<()> {
    let _migration_sql = APPLY_PLAN_HASH_PROVENANCE_SCHEMA_SQL;
    if table_exists(connection, "apply_plans")? {
        ensure_column(connection, "apply_plans", "plan_hash", "TEXT")?;
        ensure_column(
            connection,
            "apply_plans",
            "plan_hash_version",
            "TEXT NOT NULL DEFAULT 'apply_plan_hash_v1'",
        )?;
        ensure_column(
            connection,
            "apply_plans",
            "plan_hash_algorithm",
            "TEXT NOT NULL DEFAULT 'sha256'",
        )?;
        ensure_column(connection, "apply_plans", "plan_hash_created_at", "TEXT")?;
        ensure_column(
            connection,
            "apply_plans",
            "plan_provenance_json",
            "TEXT NOT NULL DEFAULT '{}'",
        )?;
        connection.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_apply_plans_plan_hash ON apply_plans (plan_hash);

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_immutable
            BEFORE UPDATE OF plan_hash ON apply_plans
            WHEN OLD.plan_hash IS NOT NULL AND OLD.plan_hash IS NOT NEW.plan_hash
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.plan_hash is immutable once set');
            END;

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_version_immutable
            BEFORE UPDATE OF plan_hash_version ON apply_plans
            WHEN OLD.plan_hash_version IS NOT NEW.plan_hash_version
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.plan_hash_version is immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_algorithm_immutable
            BEFORE UPDATE OF plan_hash_algorithm ON apply_plans
            WHEN OLD.plan_hash_algorithm IS NOT NEW.plan_hash_algorithm
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.plan_hash_algorithm is immutable');
            END;

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_hash_created_at_immutable
            BEFORE UPDATE OF plan_hash_created_at ON apply_plans
            WHEN OLD.plan_hash_created_at IS NOT NULL AND OLD.plan_hash_created_at IS NOT NEW.plan_hash_created_at
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.plan_hash_created_at is immutable once set');
            END;

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_plan_provenance_immutable
            BEFORE UPDATE OF plan_provenance_json ON apply_plans
            WHEN OLD.plan_provenance_json != '{}' AND OLD.plan_provenance_json IS NOT NEW.plan_provenance_json
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.plan_provenance_json is immutable once set');
            END;",
        )?;
    }

    Ok(())
}

fn ensure_apply_plan_preview_snapshot_schema(connection: &Connection) -> AppResult<()> {
    let _migration_sql = APPLY_PLAN_PREVIEW_SNAPSHOTS_SCHEMA_SQL;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS apply_plan_preview_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id TEXT NOT NULL UNIQUE,
            source_plan_kind TEXT NOT NULL DEFAULT 'backend_generated_sorting_preview',
            source_plan_json TEXT NOT NULL,
            source_scope_json TEXT,
            folder_config_json TEXT,
            context_trail_json TEXT NOT NULL DEFAULT '[]',
            preview_snapshot_hash TEXT NOT NULL,
            preview_snapshot_hash_version TEXT NOT NULL DEFAULT 'apply_plan_preview_snapshot_v2',
            preview_snapshot_hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
            preview_snapshot_provenance_json TEXT NOT NULL,
            scan_session_id INTEGER REFERENCES scan_sessions(id) ON DELETE SET NULL,
            consumed_apply_plan_id INTEGER REFERENCES apply_plans(id) ON DELETE SET NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at TEXT
        );",
    )?;

    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "source_plan_kind",
        "TEXT NOT NULL DEFAULT 'backend_generated_sorting_preview'",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "source_plan_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "source_scope_json",
        "TEXT",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "folder_config_json",
        "TEXT",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "context_trail_json",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "preview_snapshot_hash",
        "TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "preview_snapshot_hash_version",
        "TEXT NOT NULL DEFAULT 'apply_plan_preview_snapshot_v2'",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "preview_snapshot_hash_algorithm",
        "TEXT NOT NULL DEFAULT 'sha256'",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "preview_snapshot_provenance_json",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "scan_session_id",
        "INTEGER REFERENCES scan_sessions(id) ON DELETE SET NULL",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "consumed_apply_plan_id",
        "INTEGER REFERENCES apply_plans(id) ON DELETE SET NULL",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "created_at",
        "TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP",
    )?;
    ensure_column(
        connection,
        "apply_plan_preview_snapshots",
        "expires_at",
        "TEXT",
    )?;

    connection.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_apply_plan_preview_snapshots_hash
            ON apply_plan_preview_snapshots(preview_snapshot_hash);
        CREATE INDEX IF NOT EXISTS idx_apply_plan_preview_snapshots_created_at
            ON apply_plan_preview_snapshots(created_at);

        CREATE TRIGGER IF NOT EXISTS trg_apply_plan_preview_snapshots_identity_immutable
        BEFORE UPDATE OF snapshot_id, source_plan_kind, source_plan_json, source_scope_json, folder_config_json, context_trail_json, preview_snapshot_hash, preview_snapshot_hash_version, preview_snapshot_hash_algorithm, preview_snapshot_provenance_json, scan_session_id, created_at, expires_at
        ON apply_plan_preview_snapshots
        BEGIN
            SELECT RAISE(ABORT, 'apply_plan_preview_snapshots identity/provenance is immutable');
        END;",
    )?;

    if table_exists(connection, "apply_plans")? {
        ensure_column(
            connection,
            "apply_plans",
            "preview_snapshot_id",
            "INTEGER REFERENCES apply_plan_preview_snapshots(id) ON DELETE SET NULL",
        )?;
        ensure_column(connection, "apply_plans", "preview_snapshot_hash", "TEXT")?;
        connection.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_apply_plans_preview_snapshot_id_unique
                ON apply_plans(preview_snapshot_id)
                WHERE preview_snapshot_id IS NOT NULL;

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_preview_snapshot_id_immutable
            BEFORE UPDATE OF preview_snapshot_id ON apply_plans
            WHEN OLD.preview_snapshot_id IS NOT NULL AND OLD.preview_snapshot_id IS NOT NEW.preview_snapshot_id
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.preview_snapshot_id is immutable once set');
            END;

            CREATE TRIGGER IF NOT EXISTS trg_apply_plans_preview_snapshot_hash_immutable
            BEFORE UPDATE OF preview_snapshot_hash ON apply_plans
            WHEN OLD.preview_snapshot_hash IS NOT NULL AND OLD.preview_snapshot_hash IS NOT NEW.preview_snapshot_hash
            BEGIN
                SELECT RAISE(ABORT, 'apply_plans.preview_snapshot_hash is immutable once set');
            END;",
        )?;
    }

    Ok(())
}

fn ensure_migration_table(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );",
    )?;

    Ok(())
}

fn create_index_if_table_exists(
    connection: &Connection,
    table_name: &str,
    sql: &str,
) -> AppResult<()> {
    if table_exists(connection, table_name)? {
        connection.execute_batch(sql)?;
    }

    Ok(())
}

pub fn seed_database(connection: &mut Connection, seed_pack: &SeedPack) -> AppResult<()> {
    let transaction = connection.transaction()?;

    {
        let mut creator_insert = transaction
            .prepare("INSERT OR IGNORE INTO creators (canonical_name, notes) VALUES (?1, ?2)")?;
        let mut creator_lookup =
            transaction.prepare("SELECT id FROM creators WHERE canonical_name = ?1")?;
        let mut alias_insert = transaction.prepare(
            "INSERT OR IGNORE INTO creator_aliases (creator_id, alias_name) VALUES (?1, ?2)",
        )?;

        for creator in &seed_pack.creators {
            creator_insert.execute(params![creator.canonical_name, creator.notes])?;
            let creator_id: i64 =
                creator_lookup.query_row(params![creator.canonical_name], |row| row.get(0))?;

            for alias in &creator.aliases {
                alias_insert.execute(params![creator_id, normalize_key(alias)])?;
            }
        }
    }

    {
        let mut preset_insert = transaction.prepare(
            "INSERT OR IGNORE INTO rules (rule_name, rule_template, rule_priority) VALUES (?1, ?2, ?3)",
        )?;
        for preset in &seed_pack.presets {
            preset_insert.execute(params![preset.name, preset.template, preset.priority])?;
        }
    }

    {
        let mut settings_insert = transaction.prepare(
            "INSERT INTO app_settings (key, value, source, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO NOTHING",
        )?;
        for (key, value) in &seed_pack.defaults.settings {
            settings_insert.execute(params![key, value, "seed", Utc::now().to_rfc3339()])?;
        }
    }

    transaction.execute(
        "INSERT INTO seed_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["seed_version", seed_pack.seed_version],
    )?;

    transaction.commit()?;
    Ok(())
}

pub fn load_runtime_seed_pack(
    connection: &Connection,
    base_seed: &SeedPack,
) -> AppResult<SeedPack> {
    let mut runtime = base_seed.clone();

    {
        let mut statement = connection.prepare(
            "SELECT canonical_name, COALESCE(notes, ''), locked_by_user, created_by_user, preferred_path
             FROM creators
             WHERE created_by_user = 1 OR locked_by_user = 1 OR preferred_path IS NOT NULL",
        )?;

        let rows = statement
            .query_map([], |row| {
                Ok(SeedCreator {
                    canonical_name: row.get(0)?,
                    aliases: Vec::new(),
                    likely_kinds: Vec::new(),
                    likely_subtypes: Vec::new(),
                    notes: row.get(1)?,
                    locked_by_user: row.get::<_, i64>(2)? != 0,
                    created_by_user: row.get::<_, i64>(3)? != 0,
                    preferred_path: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for creator in rows {
            merge_runtime_creator(&mut runtime, creator);
        }
    }

    {
        let mut statement = connection.prepare(
            "SELECT c.canonical_name, u.alias_name
             FROM user_creator_aliases u
             JOIN creators c ON c.id = u.creator_id
             ORDER BY u.updated_at DESC, u.alias_name COLLATE NOCASE",
        )?;

        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        for (canonical_name, alias_name) in rows {
            merge_runtime_alias(&mut runtime, &canonical_name, &alias_name);
        }
    }

    Ok(runtime)
}

pub fn get_creator_learning_version(connection: &Connection) -> AppResult<Option<String>> {
    get_app_setting(connection, "creator_learning_version")
}

pub fn get_category_override_version(connection: &Connection) -> AppResult<Option<String>> {
    get_app_setting(connection, "category_override_version")
}

pub fn record_download_item_event(
    connection: &Connection,
    item_id: i64,
    event_kind: &str,
    label: &str,
    detail: Option<&str>,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO download_item_events (
            download_item_id,
            event_kind,
            label,
            detail,
            created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![item_id, event_kind, label, detail, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub fn load_download_item_events(
    connection: &Connection,
    item_id: i64,
    limit: i64,
) -> AppResult<Vec<DownloadsTimelineEntry>> {
    let mut statement = connection.prepare(
        "SELECT label, detail, created_at
         FROM download_item_events
         WHERE download_item_id = ?1
         ORDER BY created_at DESC, id DESC
         LIMIT ?2",
    )?;
    let entries = statement
        .query_map(params![item_id, limit.max(1)], |row| {
            Ok(DownloadsTimelineEntry {
                label: row.get(0)?,
                detail: row.get(1)?,
                at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

pub fn list_category_overrides(connection: &Connection) -> AppResult<Vec<UserCategoryOverride>> {
    let mut statement = connection.prepare(
        "SELECT match_path, kind, subtype
         FROM user_category_overrides
         ORDER BY updated_at DESC, match_path COLLATE NOCASE",
    )?;

    let overrides = statement
        .query_map([], |row| {
            Ok(UserCategoryOverride {
                match_path: row.get(0)?,
                kind: row.get(1)?,
                subtype: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(crate::error::AppError::from)?;

    Ok(overrides)
}

pub fn save_creator_learning(
    connection: &mut Connection,
    settings: &LibrarySettings,
    file_id: i64,
    creator_name: &str,
    alias_name: Option<&str>,
    lock_preference: bool,
    preferred_path: Option<&str>,
) -> AppResult<()> {
    save_creator_learning_batch(
        connection,
        settings,
        &[file_id],
        creator_name,
        alias_name,
        lock_preference,
        preferred_path,
    )
    .map(|_| ())
}

pub fn save_creator_learning_batch(
    connection: &mut Connection,
    settings: &LibrarySettings,
    file_ids: &[i64],
    creator_name: &str,
    alias_name: Option<&str>,
    lock_preference: bool,
    preferred_path: Option<&str>,
) -> AppResult<(i64, i64)> {
    let canonical_name = creator_name.trim();
    if canonical_name.is_empty() {
        return Err(AppError::Message("Creator name is required.".to_owned()));
    }

    let unique_file_ids = file_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if unique_file_ids.is_empty() {
        return Ok((0, 0));
    }

    let transaction = connection.transaction()?;
    let creator_id = ensure_creator(&transaction, canonical_name)?;
    let now = Utc::now().to_rfc3339();

    if let Some(alias_name) = alias_name.and_then(normalize_optional_alias) {
        transaction.execute(
            "INSERT INTO user_creator_aliases (creator_id, alias_name, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(alias_name) DO UPDATE
             SET creator_id = excluded.creator_id,
                 updated_at = excluded.updated_at",
            params![creator_id, alias_name, now],
        )?;
    }

    if lock_preference {
        let (file_path, source_location, kind, subtype): (String, String, String, Option<String>) =
            transaction.query_row(
                "SELECT path, source_location, kind, subtype
             FROM files
             WHERE id = ?1",
                params![unique_file_ids[0]],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;

        let resolved_path = resolve_preferred_path(
            settings,
            &file_path,
            &source_location,
            &kind,
            subtype.as_deref(),
            canonical_name,
            preferred_path,
        );

        transaction.execute(
            "UPDATE creators
             SET locked_by_user = 1,
                 preferred_path = ?1
             WHERE id = ?2",
            params![resolved_path, creator_id],
        )?;
    }

    let mut cleared_review_count = 0_i64;
    {
        let mut file_lookup = transaction.prepare(
            "SELECT parser_warnings, confidence
             FROM files
             WHERE id = ?1",
        )?;
        let mut file_update = transaction.prepare(
            "UPDATE files
             SET creator_id = ?1,
                 confidence = ?2,
                 parser_warnings = ?3
             WHERE id = ?4",
        )?;

        for file_id in &unique_file_ids {
            let (parser_warnings_json, confidence): (String, f64) =
                file_lookup.query_row(params![file_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
            let parser_warnings = filter_creator_warnings(&parser_warnings_json);
            file_update.execute(params![
                creator_id,
                confidence.max(0.92_f64),
                serde_json::to_string(&parser_warnings)?,
                file_id
            ])?;

            cleared_review_count += transaction.execute(
                "DELETE FROM review_queue
                 WHERE file_id = ?1
                   AND reason IN ('low_confidence_parse', 'conflicting_creator_signals')",
                params![file_id],
            )? as i64;
        }
    }

    transaction.execute(
        "INSERT INTO app_settings (key, value, source, updated_at)
         VALUES (?1, ?2, 'user', ?2)
         ON CONFLICT(key) DO UPDATE
         SET value = excluded.value,
             source = 'user',
             updated_at = excluded.updated_at",
        params!["creator_learning_version", now],
    )?;

    transaction.commit()?;
    Ok((unique_file_ids.len() as i64, cleared_review_count))
}

pub fn save_category_override(
    connection: &mut Connection,
    file_id: i64,
    kind: &str,
    subtype: Option<&str>,
) -> AppResult<()> {
    save_category_override_batch(connection, &[file_id], kind, subtype).map(|_| ())
}

pub fn save_category_override_batch(
    connection: &mut Connection,
    file_ids: &[i64],
    kind: &str,
    subtype: Option<&str>,
) -> AppResult<(i64, i64)> {
    let normalized_kind = kind.trim();
    if normalized_kind.is_empty() {
        return Err(AppError::Message("Kind is required.".to_owned()));
    }

    let unique_file_ids = file_ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if unique_file_ids.is_empty() {
        return Ok((0, 0));
    }

    let transaction = connection.transaction()?;
    let normalized_subtype = subtype.and_then(|value| clean_optional_string(value.to_owned()));
    let now = Utc::now().to_rfc3339();
    let mut cleared_review_count = 0_i64;
    {
        let mut file_lookup = transaction.prepare(
            "SELECT path, parser_warnings, confidence
             FROM files
             WHERE id = ?1",
        )?;
        let mut override_upsert = transaction.prepare(
            "INSERT INTO user_category_overrides (match_path, kind, subtype, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(match_path) DO UPDATE
             SET kind = excluded.kind,
                 subtype = excluded.subtype,
                 updated_at = excluded.updated_at",
        )?;
        let mut file_update = transaction.prepare(
            "UPDATE files
             SET kind = ?1,
                 subtype = ?2,
                 confidence = ?3,
                 parser_warnings = ?4
             WHERE id = ?5",
        )?;

        for file_id in &unique_file_ids {
            let (file_path, parser_warnings_json, confidence): (String, String, f64) = file_lookup
                .query_row(params![file_id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;

            override_upsert.execute(params![
                file_path,
                normalized_kind,
                normalized_subtype,
                now,
            ])?;

            let parser_warnings = filter_category_warnings(&parser_warnings_json);
            file_update.execute(params![
                normalized_kind,
                normalized_subtype,
                confidence.max(0.82_f64),
                serde_json::to_string(&parser_warnings)?,
                file_id
            ])?;

            cleared_review_count += transaction.execute(
                "DELETE FROM review_queue
                 WHERE file_id = ?1
                   AND reason IN ('low_confidence_parse', 'no_category_detected', 'conflicting_category_signals')",
                params![file_id],
            )? as i64;
        }
    }

    transaction.execute(
        "INSERT INTO app_settings (key, value, source, updated_at)
         VALUES (?1, ?2, 'user', ?2)
         ON CONFLICT(key) DO UPDATE
         SET value = excluded.value,
             source = 'user',
             updated_at = excluded.updated_at",
        params!["category_override_version", now],
    )?;

    transaction.commit()?;
    Ok((unique_file_ids.len() as i64, cleared_review_count))
}

pub fn sync_category_override_path(
    connection: &Connection,
    old_path: &str,
    new_path: &str,
) -> AppResult<()> {
    if old_path == new_path {
        return Ok(());
    }

    let existing: Option<(String, Option<String>)> = connection
        .query_row(
            "SELECT kind, subtype
             FROM user_category_overrides
             WHERE match_path = ?1",
            params![old_path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let Some((kind, subtype)) = existing else {
        return Ok(());
    };

    let now = Utc::now().to_rfc3339();
    connection.execute(
        "INSERT INTO user_category_overrides (match_path, kind, subtype, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(match_path) DO UPDATE
         SET kind = excluded.kind,
             subtype = excluded.subtype,
             updated_at = excluded.updated_at",
        params![new_path, kind, subtype, now],
    )?;
    connection.execute(
        "DELETE FROM user_category_overrides WHERE match_path = ?1",
        params![old_path],
    )?;
    Ok(())
}

pub fn get_game_installation_profile(
    connection: &Connection,
    profile_id: &str,
) -> AppResult<GameInstallationProfile> {
    let stored_profile = connection
        .query_row(
            "SELECT
                profile_id,
                profile_name,
                game_id,
                operating_environment,
                status,
                detection_method,
                detection_evidence_json,
                confirmation_state,
                confirmed_at,
                last_validated_at,
                created_at,
                updated_at
             FROM game_installation_profiles
             WHERE profile_id = ?1",
            params![profile_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                ))
            },
        )
        .optional()?;

    let Some((
        profile_id,
        profile_name,
        game_id,
        operating_environment,
        status,
        detection_method,
        detection_evidence_json,
        confirmation_state,
        confirmed_at,
        last_validated_at,
        created_at,
        updated_at,
    )) = stored_profile
    else {
        return Err(AppError::Message(format!(
            "game installation profile `{profile_id}` does not exist"
        )));
    };

    validate_profile_json_object(
        &detection_evidence_json,
        "game installation profile detection evidence",
    )?;

    let mut statement = connection.prepare(
        "SELECT
            profile_id,
            root_id,
            root_role,
            configured_path,
            required,
            validation_state,
            filesystem_capabilities_json,
            last_validated_at,
            created_at,
            updated_at
         FROM game_installation_roots
         WHERE profile_id = ?1
         ORDER BY root_id",
    )?;
    let stored_roots = statement
        .query_map(params![profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let roots = stored_roots
        .into_iter()
        .map(
            |(
                profile_id,
                root_id,
                root_role,
                configured_path,
                required,
                validation_state,
                filesystem_capabilities_json,
                last_validated_at,
                created_at,
                updated_at,
            )| {
                validate_profile_json_object(
                    &filesystem_capabilities_json,
                    "game installation root filesystem capabilities",
                )?;
                Ok(GameInstallationRoot {
                    profile_id,
                    root_id,
                    root_role,
                    configured_path,
                    required: required != 0,
                    validation_state: parse_game_installation_root_validation_state(
                        &validation_state,
                    )?,
                    filesystem_capabilities_json,
                    last_validated_at,
                    created_at,
                    updated_at,
                })
            },
        )
        .collect::<AppResult<Vec<_>>>()?;

    Ok(GameInstallationProfile {
        profile_id,
        profile_name,
        game_id,
        operating_environment: parse_game_operating_environment(&operating_environment)?,
        status: parse_game_installation_profile_status(&status)?,
        detection_method: parse_game_installation_detection_method(&detection_method)?,
        detection_evidence_json,
        confirmation_state: parse_game_installation_confirmation_state(&confirmation_state)?,
        confirmed_at,
        last_validated_at,
        created_at,
        updated_at,
        roots,
    })
}

pub fn list_game_installation_profiles(
    connection: &Connection,
) -> AppResult<Vec<GameInstallationProfile>> {
    let mut statement = connection.prepare(
        "SELECT profile_id
         FROM game_installation_profiles
         ORDER BY created_at ASC, profile_name COLLATE NOCASE ASC, profile_id ASC",
    )?;
    let profile_ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    profile_ids
        .into_iter()
        .map(|profile_id| get_game_installation_profile(connection, &profile_id))
        .collect()
}

pub fn get_active_game_installation_profile(
    connection: &Connection,
) -> AppResult<Option<GameInstallationProfile>> {
    let Some(profile_id) = get_app_setting(connection, ACTIVE_GAME_INSTALLATION_PROFILE_SETTING)?
    else {
        return Ok(None);
    };

    get_game_installation_profile(connection, &profile_id).map(Some)
}

pub fn set_active_game_installation_profile(
    connection: &mut Connection,
    profile_id: &str,
) -> AppResult<GameInstallationProfile> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return Err(AppError::Message(
            "game installation profile ID cannot be empty".to_owned(),
        ));
    }

    let transaction = connection.transaction()?;
    let profile = get_game_installation_profile(&transaction, profile_id)?;
    if profile.confirmation_state != GameInstallationConfirmationState::Confirmed {
        return Err(AppError::Message(format!(
            "Game installation profile '{}' must be confirmed before it can become active.",
            profile.profile_name
        )));
    }
    upsert_setting(
        &transaction,
        ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
        Some(&profile.profile_id),
    )?;
    transaction.commit()?;

    Ok(profile)
}

fn validate_profile_json_object(value: &str, label: &str) -> AppResult<()> {
    let parsed: serde_json::Value = serde_json::from_str(value).map_err(|error| {
        AppError::Message(format!("{label} contains invalid JSON: {error}"))
    })?;
    if !parsed.is_object() {
        return Err(AppError::Message(format!(
            "{label} must be stored as a JSON object"
        )));
    }
    Ok(())
}

fn parse_game_operating_environment(value: &str) -> AppResult<GameOperatingEnvironment> {
    match value {
        "native_windows" => Ok(GameOperatingEnvironment::NativeWindows),
        "native_macos" => Ok(GameOperatingEnvironment::NativeMacos),
        "native_linux" => Ok(GameOperatingEnvironment::NativeLinux),
        "wine" => Ok(GameOperatingEnvironment::Wine),
        "proton" => Ok(GameOperatingEnvironment::Proton),
        "lutris" => Ok(GameOperatingEnvironment::Lutris),
        "unknown" => Ok(GameOperatingEnvironment::Unknown),
        _ => Err(AppError::Message(format!(
            "unsupported game installation operating environment `{value}`"
        ))),
    }
}

fn parse_game_installation_profile_status(
    value: &str,
) -> AppResult<GameInstallationProfileStatus> {
    match value {
        "draft" => Ok(GameInstallationProfileStatus::Draft),
        "valid" => Ok(GameInstallationProfileStatus::Valid),
        "needs_review" => Ok(GameInstallationProfileStatus::NeedsReview),
        "unavailable" => Ok(GameInstallationProfileStatus::Unavailable),
        _ => Err(AppError::Message(format!(
            "unsupported game installation profile status `{value}`"
        ))),
    }
}

fn parse_game_installation_detection_method(
    value: &str,
) -> AppResult<GameInstallationDetectionMethod> {
    match value {
        "manual" => Ok(GameInstallationDetectionMethod::Manual),
        "legacy_settings_migration" => {
            Ok(GameInstallationDetectionMethod::LegacySettingsMigration)
        }
        "platform_candidate" => Ok(GameInstallationDetectionMethod::PlatformCandidate),
        _ => Err(AppError::Message(format!(
            "unsupported game installation detection method `{value}`"
        ))),
    }
}

fn parse_game_installation_confirmation_state(
    value: &str,
) -> AppResult<GameInstallationConfirmationState> {
    match value {
        "unconfirmed" => Ok(GameInstallationConfirmationState::Unconfirmed),
        "confirmed" => Ok(GameInstallationConfirmationState::Confirmed),
        "confirmation_stale" => Ok(GameInstallationConfirmationState::ConfirmationStale),
        _ => Err(AppError::Message(format!(
            "unsupported game installation confirmation state `{value}`"
        ))),
    }
}

fn parse_game_installation_root_validation_state(
    value: &str,
) -> AppResult<GameInstallationRootValidationState> {
    match value {
        "unvalidated" => Ok(GameInstallationRootValidationState::Unvalidated),
        "valid" => Ok(GameInstallationRootValidationState::Valid),
        "needs_review" => Ok(GameInstallationRootValidationState::NeedsReview),
        "unavailable" => Ok(GameInstallationRootValidationState::Unavailable),
        _ => Err(AppError::Message(format!(
            "unsupported game installation root validation state `{value}`"
        ))),
    }
}

fn library_settings_from_game_installation_profile(
    profile: &GameInstallationProfile,
) -> LibrarySettings {
    let mut settings = LibrarySettings::default();
    for root in &profile.roots {
        match root.root_id.as_str() {
            "mods" => settings.mods_path = Some(root.configured_path.clone()),
            "tray" => settings.tray_path = Some(root.configured_path.clone()),
            "downloads" => settings.downloads_path = Some(root.configured_path.clone()),
            "reject" => settings.download_reject_folder = Some(root.configured_path.clone()),
            _ => {}
        }
    }
    settings
}

pub fn get_library_settings(connection: &Connection) -> AppResult<LibrarySettings> {
    let legacy_settings = LibrarySettings {
        mods_path: get_app_setting(connection, "mods_path")?,
        tray_path: get_app_setting(connection, "tray_path")?,
        downloads_path: get_app_setting(connection, "downloads_path")?,
        ..Default::default()
    };
    let stored_settings = get_active_game_installation_profile(connection)?
        .as_ref()
        .map(library_settings_from_game_installation_profile)
        .unwrap_or(legacy_settings);
    Ok(apply_library_settings_overrides(stored_settings))
}

pub fn save_library_paths(
    connection: &mut Connection,
    settings: &LibrarySettings,
) -> AppResult<()> {
    let transaction = connection.transaction()?;
    upsert_setting(&transaction, "mods_path", settings.mods_path.as_deref())?;
    upsert_setting(&transaction, "tray_path", settings.tray_path.as_deref())?;
    upsert_setting(
        &transaction,
        "downloads_path",
        settings.downloads_path.as_deref(),
    )?;

    let active_profile_id = transaction
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![ACTIVE_GAME_INSTALLATION_PROFILE_SETTING],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(clean_optional_string);
    if active_profile_id.as_deref() == Some(LEGACY_GAME_INSTALLATION_PROFILE_ID) {
        let detection_method = transaction
            .query_row(
                "SELECT detection_method
                 FROM game_installation_profiles
                 WHERE profile_id = ?1",
                params![LEGACY_GAME_INSTALLATION_PROFILE_ID],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if detection_method.as_deref() == Some("legacy_settings_migration") {
            let now = Utc::now().to_rfc3339();
            upsert_legacy_game_installation_root(
                &transaction,
                "mods",
                "installed_mods",
                settings.mods_path.as_deref(),
                true,
                &now,
            )?;
            upsert_legacy_game_installation_root(
                &transaction,
                "tray",
                "installed_tray",
                settings.tray_path.as_deref(),
                true,
                &now,
            )?;
            upsert_legacy_game_installation_root(
                &transaction,
                "downloads",
                "intake_downloads",
                settings.downloads_path.as_deref(),
                false,
                &now,
            )?;
            transaction.execute(
                "UPDATE game_installation_profiles
                 SET status = 'needs_review',
                     last_validated_at = NULL,
                     updated_at = ?2
                 WHERE profile_id = ?1",
                params![LEGACY_GAME_INSTALLATION_PROFILE_ID, now],
            )?;
        }
    }

    transaction.commit()?;
    Ok(())
}

pub fn get_app_setting(connection: &Connection, key: &str) -> AppResult<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(AppError::from)
        .map(|value| value.and_then(clean_optional_string))
}

pub fn save_app_setting(
    connection: &mut Connection,
    key: &str,
    value: Option<&str>,
    source: &str,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO app_settings (key, value, source, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, source = excluded.source, updated_at = excluded.updated_at",
        params![key, value.unwrap_or_default(), source, Utc::now().to_rfc3339()],
    )?;

    Ok(())
}

fn upsert_setting(
    transaction: &rusqlite::Transaction<'_>,
    key: &str,
    value: Option<&str>,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO app_settings (key, value, source, updated_at)
         VALUES (?1, ?2, 'user', ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, source = 'user', updated_at = excluded.updated_at",
        params![
            key,
            value.unwrap_or_default(),
            Utc::now().to_rfc3339()
        ],
    )?;

    Ok(())
}

fn clean_optional_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn env_override_path(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(clean_optional_string)
}

fn apply_library_settings_overrides(settings: LibrarySettings) -> LibrarySettings {
    apply_library_settings_override_values(
        settings,
        env_override_path("SIMSUITE_MODS_PATH"),
        env_override_path("SIMSUITE_TRAY_PATH"),
        env_override_path("SIMSUITE_DOWNLOADS_PATH"),
    )
}

fn apply_library_settings_override_values(
    settings: LibrarySettings,
    mods_override: Option<String>,
    tray_override: Option<String>,
    downloads_override: Option<String>,
) -> LibrarySettings {
    LibrarySettings {
        mods_path: mods_override.or(settings.mods_path),
        tray_path: tray_override.or(settings.tray_path),
        downloads_path: downloads_override.or(settings.downloads_path),
        download_reject_folder: settings.download_reject_folder,
    }
}

fn normalize_key(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn ensure_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS user_creator_aliases (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            creator_id INTEGER NOT NULL REFERENCES creators (id) ON DELETE CASCADE,
            alias_name TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_user_creator_aliases_creator_id ON user_creator_aliases (creator_id);
        CREATE TABLE IF NOT EXISTS user_category_overrides (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            match_path TEXT NOT NULL UNIQUE,
            kind TEXT NOT NULL,
            subtype TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_user_category_overrides_kind ON user_category_overrides (kind);
        CREATE TABLE IF NOT EXISTS download_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_path TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            source_kind TEXT NOT NULL CHECK (source_kind IN ('file', 'archive')),
            archive_format TEXT,
            staging_path TEXT,
            source_size INTEGER NOT NULL DEFAULT 0,
            source_modified_at TEXT,
            detected_file_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'ready', 'needs_review', 'partial', 'applied', 'ignored', 'rejected', 'error')),
            intake_mode TEXT NOT NULL DEFAULT 'standard' CHECK (intake_mode IN ('standard', 'guided', 'needs_review', 'blocked')),
            risk_level TEXT NOT NULL DEFAULT 'low' CHECK (risk_level IN ('low', 'medium', 'high')),
            matched_profile_key TEXT,
            matched_profile_name TEXT,
            special_family TEXT,
            assessment_reasons TEXT NOT NULL DEFAULT '[]',
            dependency_summary TEXT NOT NULL DEFAULT '[]',
            missing_dependencies TEXT NOT NULL DEFAULT '[]',
            inbox_dependencies TEXT NOT NULL DEFAULT '[]',
            incompatibility_warnings TEXT NOT NULL DEFAULT '[]',
            post_install_notes TEXT NOT NULL DEFAULT '[]',
            evidence_summary TEXT NOT NULL DEFAULT '[]',
            catalog_source_url TEXT,
            catalog_download_url TEXT,
            latest_check_url TEXT,
            latest_check_strategy TEXT,
            catalog_reference_source TEXT NOT NULL DEFAULT '[]',
            catalog_reviewed_at TEXT,
            existing_install_detected INTEGER NOT NULL DEFAULT 0 CHECK (existing_install_detected IN (0, 1)),
            guided_install_available INTEGER NOT NULL DEFAULT 0 CHECK (guided_install_available IN (0, 1)),
            error_message TEXT,
            notes TEXT NOT NULL DEFAULT '[]',
            first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_download_items_status ON download_items (status);
        CREATE INDEX IF NOT EXISTS idx_download_items_intake_mode ON download_items (intake_mode);
        CREATE TABLE IF NOT EXISTS special_mod_family_state (
            profile_key TEXT PRIMARY KEY,
            profile_name TEXT NOT NULL,
            install_state TEXT NOT NULL DEFAULT 'not_installed',
            install_path TEXT,
            installed_version TEXT,
            installed_signature TEXT,
            source_item_id INTEGER REFERENCES download_items (id) ON DELETE SET NULL,
            checked_at TEXT,
            latest_source_url TEXT,
            latest_download_url TEXT,
            latest_version TEXT,
            latest_checked_at TEXT,
            latest_confidence REAL NOT NULL DEFAULT 0,
            latest_status TEXT NOT NULL DEFAULT 'unknown',
            latest_note TEXT,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS download_item_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            download_item_id INTEGER NOT NULL REFERENCES download_items (id) ON DELETE CASCADE,
            event_kind TEXT NOT NULL,
            label TEXT NOT NULL,
            detail TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_download_item_events_item_id ON download_item_events (download_item_id, created_at DESC);
        CREATE TABLE IF NOT EXISTS content_watch_sources (
            subject_key TEXT PRIMARY KEY,
            anchor_file_id INTEGER REFERENCES files (id) ON DELETE CASCADE,
            source_kind TEXT NOT NULL DEFAULT 'exact_page',
            source_label TEXT,
            source_url TEXT NOT NULL,
            approved_by_user INTEGER NOT NULL DEFAULT 0 CHECK (approved_by_user IN (0, 1)),
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS content_watch_results (
            subject_key TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'unknown',
            latest_version TEXT,
            checked_at TEXT,
            confidence TEXT NOT NULL DEFAULT 'unknown',
            note TEXT,
            evidence TEXT NOT NULL DEFAULT '[]',
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(subject_key) REFERENCES content_watch_sources(subject_key) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_content_watch_sources_kind ON content_watch_sources (source_kind);
        CREATE INDEX IF NOT EXISTS idx_content_watch_results_status ON content_watch_results (status);
        CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_name TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS snapshot_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL REFERENCES snapshots (id) ON DELETE CASCADE,
            file_id INTEGER REFERENCES files (id) ON DELETE SET NULL,
            original_path TEXT NOT NULL,
            original_hash TEXT,
            backup_path TEXT
        );
        CREATE TABLE IF NOT EXISTS library_folders (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_location TEXT NOT NULL CHECK (source_location IN ('mods', 'tray')),
            relative_path TEXT NOT NULL DEFAULT '',
            normalized_relative_path TEXT NOT NULL DEFAULT '',
            parent_normalized_relative_path TEXT,
            name TEXT NOT NULL,
            depth INTEGER NOT NULL DEFAULT 0,
            full_path TEXT NOT NULL,
            scan_session_id INTEGER REFERENCES scan_sessions (id) ON DELETE SET NULL,
            indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (source_location, normalized_relative_path)
        );",
    )?;

    ensure_column(
        connection,
        "files",
        "insights",
        "TEXT NOT NULL DEFAULT '{}'",
    )?;
    ensure_column(connection, "files", "content_fingerprint", "TEXT")?;
    ensure_column(connection, "files", "content_fingerprint_kind", "TEXT")?;
    ensure_column(connection, "files", "content_fingerprint_version", "TEXT")?;
    ensure_column(
        connection,
        "files",
        "content_fingerprint_status",
        "TEXT NOT NULL DEFAULT 'not_attempted'",
    )?;
    ensure_column(connection, "files", "content_fingerprint_error", "TEXT")?;
    ensure_column(connection, "files", "download_item_id", "INTEGER")?;
    ensure_column(connection, "files", "source_origin_path", "TEXT")?;
    ensure_column(connection, "files", "archive_member_path", "TEXT")?;
    ensure_column(connection, "snapshot_items", "backup_path", "TEXT")?;
    ensure_column(
        connection,
        "download_items",
        "intake_mode",
        "TEXT NOT NULL DEFAULT 'standard'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "risk_level",
        "TEXT NOT NULL DEFAULT 'low'",
    )?;
    ensure_column(connection, "download_items", "matched_profile_key", "TEXT")?;
    ensure_column(connection, "download_items", "matched_profile_name", "TEXT")?;
    ensure_column(connection, "download_items", "special_family", "TEXT")?;
    ensure_column(
        connection,
        "download_items",
        "assessment_reasons",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "dependency_summary",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "missing_dependencies",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "inbox_dependencies",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "incompatibility_warnings",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "post_install_notes",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(
        connection,
        "download_items",
        "evidence_summary",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(connection, "download_items", "catalog_source_url", "TEXT")?;
    ensure_column(connection, "download_items", "catalog_download_url", "TEXT")?;
    ensure_column(connection, "download_items", "latest_check_url", "TEXT")?;
    ensure_column(
        connection,
        "download_items",
        "latest_check_strategy",
        "TEXT",
    )?;
    ensure_column(
        connection,
        "download_items",
        "catalog_reference_source",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    ensure_column(connection, "download_items", "catalog_reviewed_at", "TEXT")?;
    ensure_column(connection, "download_items", "snoozed_until", "INTEGER")?;
    ensure_column(
        connection,
        "download_items",
        "existing_install_detected",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        connection,
        "download_items",
        "guided_install_available",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(connection, "download_items", "last_applied_at", "INTEGER")?;
    ensure_column(
        connection,
        "content_watch_sources",
        "anchor_file_id",
        "INTEGER",
    )?;
    connection.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_files_download_item_id ON files (download_item_id);
         CREATE INDEX IF NOT EXISTS idx_download_items_updated_at ON download_items (updated_at);
         CREATE INDEX IF NOT EXISTS idx_download_items_status_updated_at ON download_items (status, updated_at);
         CREATE INDEX IF NOT EXISTS idx_download_items_special_family ON download_items (special_family);
         CREATE INDEX IF NOT EXISTS idx_download_items_matched_profile_key ON download_items (matched_profile_key);
         CREATE INDEX IF NOT EXISTS idx_files_download_item_id_source_location ON files (download_item_id, source_location);
         CREATE INDEX IF NOT EXISTS idx_files_source_location_kind ON files (source_location, kind);
         CREATE INDEX IF NOT EXISTS idx_files_source_location_filename ON files (source_location, filename);
         CREATE INDEX IF NOT EXISTS idx_files_source_location_depth ON files (source_location, relative_depth);
         CREATE INDEX IF NOT EXISTS idx_files_relative_depth ON files (relative_depth);
         CREATE INDEX IF NOT EXISTS idx_files_content_fingerprint ON files (content_fingerprint_kind, content_fingerprint);
         CREATE INDEX IF NOT EXISTS idx_library_folders_source_location ON library_folders (source_location);
         CREATE INDEX IF NOT EXISTS idx_library_folders_source_path ON library_folders (source_location, normalized_relative_path);
         CREATE INDEX IF NOT EXISTS idx_library_folders_source_parent ON library_folders (source_location, parent_normalized_relative_path);
         CREATE INDEX IF NOT EXISTS idx_library_folders_source_depth ON library_folders (source_location, depth);
         CREATE INDEX IF NOT EXISTS idx_snapshot_items_snapshot_id ON snapshot_items (snapshot_id);
         CREATE TABLE IF NOT EXISTS special_mod_family_state (
            profile_key TEXT PRIMARY KEY,
            profile_name TEXT NOT NULL,
            install_state TEXT NOT NULL DEFAULT 'not_installed',
            install_path TEXT,
            installed_version TEXT,
            installed_signature TEXT,
            source_item_id INTEGER REFERENCES download_items (id) ON DELETE SET NULL,
            checked_at TEXT,
            latest_source_url TEXT,
            latest_download_url TEXT,
            latest_version TEXT,
            latest_checked_at TEXT,
            latest_confidence REAL NOT NULL DEFAULT 0,
            latest_status TEXT NOT NULL DEFAULT 'unknown',
            latest_note TEXT,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS download_item_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            download_item_id INTEGER NOT NULL REFERENCES download_items (id) ON DELETE CASCADE,
            event_kind TEXT NOT NULL,
            label TEXT NOT NULL,
            detail TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE INDEX IF NOT EXISTS idx_download_item_events_item_id ON download_item_events (download_item_id, created_at DESC);
         CREATE TABLE IF NOT EXISTS content_watch_sources (
            subject_key TEXT PRIMARY KEY,
            anchor_file_id INTEGER REFERENCES files (id) ON DELETE CASCADE,
            source_kind TEXT NOT NULL DEFAULT 'exact_page',
            source_label TEXT,
            source_url TEXT NOT NULL,
            approved_by_user INTEGER NOT NULL DEFAULT 0 CHECK (approved_by_user IN (0, 1)),
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS content_watch_results (
            subject_key TEXT PRIMARY KEY,
            status TEXT NOT NULL DEFAULT 'unknown',
            latest_version TEXT,
            checked_at TEXT,
            confidence TEXT NOT NULL DEFAULT 'unknown',
            note TEXT,
            evidence TEXT NOT NULL DEFAULT '[]',
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(subject_key) REFERENCES content_watch_sources(subject_key) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_content_watch_sources_kind ON content_watch_sources (source_kind);
         CREATE INDEX IF NOT EXISTS idx_content_watch_sources_anchor_file_id ON content_watch_sources (anchor_file_id);
         CREATE INDEX IF NOT EXISTS idx_content_watch_results_status ON content_watch_results (status);",
    )?;

    create_index_if_table_exists(
        connection,
        "review_queue",
        "CREATE INDEX IF NOT EXISTS idx_review_queue_created_at ON review_queue (created_at);",
    )?;
    create_index_if_table_exists(
        connection,
        "review_queue",
        "CREATE INDEX IF NOT EXISTS idx_review_queue_file_id ON review_queue (file_id);",
    )?;
    create_index_if_table_exists(
        connection,
        "duplicates",
        "CREATE INDEX IF NOT EXISTS idx_duplicates_duplicate_type ON duplicates (duplicate_type);",
    )?;
    create_index_if_table_exists(
        connection,
        "duplicates",
        "CREATE INDEX IF NOT EXISTS idx_duplicates_file_id_a ON duplicates (file_id_a);",
    )?;
    create_index_if_table_exists(
        connection,
        "duplicates",
        "CREATE INDEX IF NOT EXISTS idx_duplicates_file_id_b ON duplicates (file_id_b);",
    )?;
    ensure_apply_plan_schema(connection)?;
    ensure_apply_plan_result_restore_schema(connection)?;
    ensure_apply_plan_context_snapshot_schema(connection)?;
    ensure_apply_plan_hash_provenance_schema(connection)?;
    ensure_apply_plan_preview_snapshot_schema(connection)?;
    ensure_game_installation_profile_schema(connection)?;

    Ok(())
}

fn ensure_apply_plan_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(APPLY_PLAN_PERSISTENCE_SCHEMA_SQL)?;
    Ok(())
}

fn ensure_apply_plan_result_restore_schema(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(APPLY_PLAN_RESULT_RESTORE_SCHEMA_SQL)?;
    Ok(())
}

fn merge_runtime_creator(runtime: &mut SeedPack, creator: SeedCreator) {
    let canonical_name = creator.canonical_name.clone();
    let normalized = normalize_key(&canonical_name);
    if normalized.is_empty() {
        return;
    }

    runtime
        .creator_lookup
        .insert(normalized.clone(), canonical_name.clone());
    runtime.parser_lexicon.insert(normalized);

    if let Some(existing) = runtime.creator_profiles.get_mut(&canonical_name) {
        existing.locked_by_user |= creator.locked_by_user;
        existing.created_by_user |= creator.created_by_user;
        if creator.preferred_path.is_some() {
            existing.preferred_path = creator.preferred_path.clone();
        }
        if existing.notes.trim().is_empty() && !creator.notes.trim().is_empty() {
            existing.notes = creator.notes;
        }
        return;
    }

    runtime.creators.push(creator.clone());
    runtime.creator_profiles.insert(canonical_name, creator);
}

fn merge_runtime_alias(runtime: &mut SeedPack, canonical_name: &str, alias_name: &str) {
    let normalized_alias = normalize_key(alias_name);
    if normalized_alias.is_empty() {
        return;
    }

    if !runtime.creator_profiles.contains_key(canonical_name) {
        merge_runtime_creator(
            runtime,
            SeedCreator {
                canonical_name: canonical_name.to_owned(),
                aliases: Vec::new(),
                likely_kinds: Vec::new(),
                likely_subtypes: Vec::new(),
                notes: "User-learned creator".to_owned(),
                locked_by_user: false,
                created_by_user: true,
                preferred_path: None,
            },
        );
    }

    runtime
        .creator_lookup
        .insert(normalized_alias.clone(), canonical_name.to_owned());
    runtime.parser_lexicon.insert(normalized_alias);

    if let Some(profile) = runtime.creator_profiles.get_mut(canonical_name) {
        if !profile
            .aliases
            .iter()
            .any(|alias| normalize_key(alias) == normalize_key(alias_name))
        {
            profile.aliases.push(alias_name.to_owned());
        }
    }
}

fn ensure_creator(transaction: &rusqlite::Transaction<'_>, creator_name: &str) -> AppResult<i64> {
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT id
             FROM creators
             WHERE canonical_name = ?1 COLLATE NOCASE",
            params![creator_name],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        return Ok(id);
    }

    transaction.execute(
        "INSERT INTO creators (canonical_name, notes, created_by_user)
         VALUES (?1, ?2, 1)",
        params![creator_name, "User-learned creator"],
    )?;

    Ok(transaction.last_insert_rowid())
}

fn normalize_optional_alias(value: &str) -> Option<String> {
    let normalized = normalize_key(value.trim());
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn filter_creator_warnings(parser_warnings_json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(parser_warnings_json)
        .unwrap_or_default()
        .into_iter()
        .filter(|warning| warning != "conflicting_creator_signals")
        .collect()
}

fn filter_category_warnings(parser_warnings_json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(parser_warnings_json)
        .unwrap_or_default()
        .into_iter()
        .filter(|warning| {
            warning != "no_category_detected" && warning != "conflicting_category_signals"
        })
        .collect()
}

fn resolve_preferred_path(
    settings: &LibrarySettings,
    file_path: &str,
    source_location: &str,
    kind: &str,
    subtype: Option<&str>,
    creator_name: &str,
    preferred_path: Option<&str>,
) -> String {
    if let Some(preferred_path) =
        preferred_path.and_then(|value| clean_optional_string(value.to_owned()))
    {
        return normalize_relative_path(&preferred_path);
    }

    if let Some(current) = derive_current_relative_parent(settings, file_path, source_location) {
        return current;
    }

    let creator = sanitize_component(creator_name, "Unknown");
    let subtype = sanitize_component(subtype.unwrap_or("Misc"), "Misc");
    match kind {
        "CAS" => normalize_relative_path(&format!("CAS/{subtype}/{creator}")),
        "BuildBuy" => normalize_relative_path(&format!("BuildBuy/{creator}")),
        "Gameplay" => normalize_relative_path(&format!("Gameplay/{creator}")),
        "ScriptMods" => normalize_relative_path(&format!("ScriptMods/{creator}")),
        "OverridesAndDefaults" => normalize_relative_path(&format!("Overrides/{creator}")),
        "PosesAndAnimation" => normalize_relative_path(&format!("Poses/{creator}")),
        "PresetsAndSliders" => normalize_relative_path(&format!("Presets/{subtype}/{creator}")),
        _ => normalize_relative_path(&format!("Creators/{creator}")),
    }
}

fn derive_current_relative_parent(
    settings: &LibrarySettings,
    file_path: &str,
    source_location: &str,
) -> Option<String> {
    let root = match source_location {
        "tray" => settings.tray_path.as_deref(),
        _ => settings.mods_path.as_deref(),
    }?;

    let relative = std::path::Path::new(file_path)
        .strip_prefix(root)
        .ok()?
        .parent()?;
    let normalized = normalize_relative_path(&relative.to_string_lossy());
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn normalize_relative_path(value: &str) -> String {
    value
        .replace('\\', "/")
        .split('/')
        .map(|segment| sanitize_component(segment, ""))
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_component(value: &str, fallback: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            ) {
                '_'
            } else {
                character
            }
        })
        .collect::<String>()
        .trim()
        .replace('.', "_");

    if cleaned.is_empty() {
        fallback.to_owned()
    } else {
        cleaned
    }
}

fn ensure_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    definition: &str,
) -> AppResult<()> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma)?;
    let existing = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if existing.iter().any(|name| name == column_name) {
        return Ok(());
    }

    let alter = format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {definition}");
    connection.execute(&alter, [])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{models::LibrarySettings, seed::load_seed_pack};

    use super::*;

    #[test]
    fn library_settings_overrides_replace_only_present_values() {
        let settings = apply_library_settings_override_values(
            LibrarySettings {
                mods_path: Some("C:/Mods/Real".to_owned()),
                tray_path: Some("C:/Tray/Real".to_owned()),
                downloads_path: Some("C:/Downloads/Real".to_owned()),
                download_reject_folder: Some("C:/Downloads/Rejected".to_owned()),
            },
            Some("C:/Mods/Test".to_owned()),
            None,
            Some("C:/Downloads/Test".to_owned()),
        );

        assert_eq!(settings.mods_path, Some("C:/Mods/Test".to_owned()));
        assert_eq!(settings.tray_path, Some("C:/Tray/Real".to_owned()));
        assert_eq!(
            settings.downloads_path,
            Some("C:/Downloads/Test".to_owned())
        );
        assert_eq!(
            settings.download_reject_folder,
            Some("C:/Downloads/Rejected".to_owned())
        );
    }

    fn legacy_settings_connection(
        mods_path: Option<&str>,
        tray_path: Option<&str>,
        downloads_path: Option<&str>,
    ) -> Connection {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        connection
            .execute_batch(schema::INITIAL_SCHEMA_SQL)
            .expect("initial schema");
        for (key, value) in [
            ("mods_path", mods_path),
            ("tray_path", tray_path),
            ("downloads_path", downloads_path),
        ] {
            if let Some(value) = value {
                save_app_setting(&mut connection, key, Some(value), "user")
                    .expect("legacy setting");
            }
        }
        connection
    }

    #[test]
    fn initialize_creates_game_installation_profile_schema_without_fake_profile() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        for table_name in ["game_installation_profiles", "game_installation_roots"] {
            assert!(
                table_exists(&connection, table_name).expect("table lookup"),
                "missing table {table_name}"
            );
        }
        let migration_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 8 AND name = 'game_installation_profiles_v1'",
                [],
                |row| row.get(0),
            )
            .expect("migration row");
        assert_eq!(migration_exists, 1);
        let profile_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM game_installation_profiles", [], |row| {
                row.get(0)
            })
            .expect("profile count");
        assert_eq!(profile_count, 0);
        assert!(get_active_game_installation_profile(&connection)
            .expect("active profile")
            .is_none());
    }

    #[test]
    fn legacy_library_paths_backfill_one_active_profile_idempotently() {
        let mut connection = legacy_settings_connection(
            Some("/Users/player/Documents/Electronic Arts/The Sims 4/Mods"),
            Some("/Users/player/Documents/Electronic Arts/The Sims 4/Tray"),
            Some("/Users/player/Downloads"),
        );
        initialize(&mut connection).expect("first initialization");
        initialize(&mut connection).expect("second initialization");

        let profile_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM game_installation_profiles", [], |row| {
                row.get(0)
            })
            .expect("profile count");
        assert_eq!(profile_count, 1);
        let root_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM game_installation_roots", [], |row| {
                row.get(0)
            })
            .expect("root count");
        assert_eq!(root_count, 3);
        assert_eq!(
            get_app_setting(&connection, ACTIVE_GAME_INSTALLATION_PROFILE_SETTING)
                .expect("active profile setting"),
            Some(LEGACY_GAME_INSTALLATION_PROFILE_ID.to_owned())
        );

        let profile = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("migrated profile");
        assert_eq!(profile.profile_id, LEGACY_GAME_INSTALLATION_PROFILE_ID);
        assert_eq!(profile.game_id, "sims4");
        assert_eq!(
            profile.detection_method,
            GameInstallationDetectionMethod::LegacySettingsMigration
        );
        assert_eq!(
            profile.confirmation_state,
            GameInstallationConfirmationState::Confirmed
        );
        assert_eq!(profile.roots.len(), 3);

        let settings = get_library_settings(&connection).expect("derived settings");
        assert_eq!(
            settings.mods_path.as_deref(),
            Some("/Users/player/Documents/Electronic Arts/The Sims 4/Mods")
        );
        assert_eq!(
            settings.tray_path.as_deref(),
            Some("/Users/player/Documents/Electronic Arts/The Sims 4/Tray")
        );
        assert_eq!(
            settings.downloads_path.as_deref(),
            Some("/Users/player/Downloads")
        );
    }

    #[test]
    fn profile_list_and_active_lookup_share_ordered_complete_records() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute_batch(
                "INSERT INTO game_installation_profiles (
                    profile_id, profile_name, game_id, operating_environment, status,
                    detection_method, detection_evidence_json, confirmation_state,
                    confirmed_at, last_validated_at, created_at, updated_at
                 ) VALUES
                    ('profile-later', 'Later profile', 'sims4', 'native_macos', 'needs_review',
                     'manual', '{}', 'confirmed', '2026-02-01T00:00:00Z', NULL,
                     '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z'),
                    ('profile-earlier', 'Earlier profile', 'sims4', 'native_macos', 'valid',
                     'manual', '{}', 'confirmed', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z');
                 INSERT INTO game_installation_roots (
                    profile_id, root_id, root_role, configured_path, required,
                    validation_state, filesystem_capabilities_json,
                    last_validated_at, created_at, updated_at
                 ) VALUES
                    ('profile-later', 'mods', 'installed_mods', '/later/Mods', 1,
                     'unvalidated', '{}', NULL, '2026-02-01T00:00:00Z',
                     '2026-02-01T00:00:00Z'),
                    ('profile-later', 'downloads', 'intake_downloads', '/later/Downloads', 0,
                     'unvalidated', '{}', NULL, '2026-02-01T00:00:00Z',
                     '2026-02-01T00:00:00Z');",
            )
            .expect("profile fixtures");
        save_app_setting(
            &mut connection,
            ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
            Some("profile-later"),
            "user",
        )
        .expect("active profile setting");

        let profiles = list_game_installation_profiles(&connection).expect("profile list");
        assert_eq!(
            profiles
                .iter()
                .map(|profile| profile.profile_id.as_str())
                .collect::<Vec<_>>(),
            vec!["profile-earlier", "profile-later"]
        );
        assert!(profiles[0].roots.is_empty());
        assert_eq!(
            profiles[1]
                .roots
                .iter()
                .map(|root| root.root_id.as_str())
                .collect::<Vec<_>>(),
            vec!["downloads", "mods"]
        );

        let active = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("selected profile");
        assert_eq!(active.profile_id, "profile-later");
        assert_eq!(active.roots, profiles[1].roots);
    }

    #[test]
    fn active_profile_selection_is_atomic_and_updates_library_compatibility() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute_batch(
                "INSERT INTO game_installation_profiles (
                    profile_id, profile_name, game_id, operating_environment, status,
                    detection_method, detection_evidence_json, confirmation_state,
                    confirmed_at, last_validated_at, created_at, updated_at
                 ) VALUES
                    ('profile-a', 'Profile A', 'sims4', 'native_macos', 'valid',
                     'manual', '{}', 'confirmed', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z'),
                    ('profile-b', 'Profile B', 'sims4', 'native_macos', 'needs_review',
                     'manual', '{}', 'confirmed', '2026-02-01T00:00:00Z', NULL,
                     '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z');
                 INSERT INTO game_installation_roots (
                    profile_id, root_id, root_role, configured_path, required,
                    validation_state, filesystem_capabilities_json,
                    last_validated_at, created_at, updated_at
                 ) VALUES
                    ('profile-a', 'mods', 'installed_mods', '/profile-a/Mods', 1,
                     'valid', '{}', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
                    ('profile-b', 'mods', 'installed_mods', '/profile-b/Mods', 1,
                     'unvalidated', '{}', NULL,
                     '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z'),
                    ('profile-b', 'tray', 'installed_tray', '/profile-b/Tray', 1,
                     'unvalidated', '{}', NULL,
                     '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z');",
            )
            .expect("profile fixtures");
        save_app_setting(
            &mut connection,
            ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
            Some("profile-a"),
            "user",
        )
        .expect("initial active profile");

        let selected = set_active_game_installation_profile(&mut connection, " profile-b ")
            .expect("select profile");
        assert_eq!(selected.profile_id, "profile-b");

        let active = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("selected profile");
        assert_eq!(active.profile_id, "profile-b");

        let settings = get_library_settings(&connection).expect("derived settings");
        assert_eq!(settings.mods_path.as_deref(), Some("/profile-b/Mods"));
        assert_eq!(settings.tray_path.as_deref(), Some("/profile-b/Tray"));
    }

    #[test]
    fn invalid_active_profile_selection_preserves_previous_profile() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute_batch(
                "INSERT INTO game_installation_profiles (
                    profile_id, profile_name, game_id, operating_environment, status,
                    detection_method, detection_evidence_json, confirmation_state,
                    confirmed_at, last_validated_at, created_at, updated_at
                 ) VALUES ('profile-a', 'Profile A', 'sims4', 'native_macos', 'valid',
                    'manual', '{}', 'confirmed', '2026-01-01T00:00:00Z',
                    '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
                    '2026-01-01T00:00:00Z');",
            )
            .expect("profile fixture");
        save_app_setting(
            &mut connection,
            ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
            Some("profile-a"),
            "user",
        )
        .expect("initial active profile");

        let error = set_active_game_installation_profile(&mut connection, "missing-profile")
            .expect_err("unknown profile should fail");
        assert!(error.to_string().contains("does not exist"));

        let active = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("preserved profile");
        assert_eq!(active.profile_id, "profile-a");
    }

    #[test]
    fn unconfirmed_active_profile_selection_preserves_previous_profile() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute_batch(
                "INSERT INTO game_installation_profiles (
                    profile_id, profile_name, game_id, operating_environment, status,
                    detection_method, detection_evidence_json, confirmation_state,
                    confirmed_at, last_validated_at, created_at, updated_at
                 ) VALUES
                    ('profile-a', 'Profile A', 'sims4', 'native_macos', 'valid',
                     'manual', '{}', 'confirmed', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
                     '2026-01-01T00:00:00Z'),
                    ('profile-draft', 'Draft profile', 'sims4', 'native_macos', 'draft',
                     'manual', '{}', 'unconfirmed', NULL, NULL,
                     '2026-02-01T00:00:00Z', '2026-02-01T00:00:00Z');",
            )
            .expect("profile fixtures");
        save_app_setting(
            &mut connection,
            ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
            Some("profile-a"),
            "user",
        )
        .expect("initial active profile");

        let error = set_active_game_installation_profile(&mut connection, "profile-draft")
            .expect_err("unconfirmed profile should fail");
        assert!(error.to_string().contains("must be confirmed"));

        let active = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("preserved profile");
        assert_eq!(active.profile_id, "profile-a");
    }

    #[test]
    fn legacy_path_save_updates_only_the_migrated_profile() {
        let mut connection =
            legacy_settings_connection(Some("/old/Mods"), Some("/old/Tray"), None);
        initialize(&mut connection).expect("schema");

        save_library_paths(
            &mut connection,
            &LibrarySettings {
                mods_path: Some("/new/Mods".to_owned()),
                tray_path: Some("/new/Tray".to_owned()),
                downloads_path: Some("/new/Downloads".to_owned()),
                ..Default::default()
            },
        )
        .expect("save paths");

        let profile = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("migrated profile");
        let settings = library_settings_from_game_installation_profile(&profile);
        assert_eq!(settings.mods_path.as_deref(), Some("/new/Mods"));
        assert_eq!(settings.tray_path.as_deref(), Some("/new/Tray"));
        assert_eq!(settings.downloads_path.as_deref(), Some("/new/Downloads"));
        assert_eq!(profile.status, GameInstallationProfileStatus::NeedsReview);
        assert!(profile.last_validated_at.is_none());
    }

    #[test]
    fn legacy_path_save_does_not_rewrite_manual_profile() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        let now = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO game_installation_profiles (
                    profile_id, profile_name, game_id, operating_environment, status,
                    detection_method, detection_evidence_json, confirmation_state,
                    confirmed_at, last_validated_at, created_at, updated_at
                 ) VALUES ('manual-profile', 'Manual profile', 'sims4', 'native_macos',
                    'valid', 'manual', '{}', 'confirmed', ?1, ?1, ?1, ?1)",
                params![now],
            )
            .expect("manual profile");
        connection
            .execute(
                "INSERT INTO game_installation_roots (
                    profile_id, root_id, root_role, configured_path, required,
                    validation_state, filesystem_capabilities_json,
                    last_validated_at, created_at, updated_at
                 ) VALUES ('manual-profile', 'mods', 'installed_mods', '/manual/Mods', 1,
                    'valid', '{}', ?1, ?1, ?1)",
                params![now],
            )
            .expect("manual root");
        save_app_setting(
            &mut connection,
            ACTIVE_GAME_INSTALLATION_PROFILE_SETTING,
            Some("manual-profile"),
            "user",
        )
        .expect("active profile");

        save_library_paths(
            &mut connection,
            &LibrarySettings {
                mods_path: Some("/legacy/Mods".to_owned()),
                tray_path: Some("/legacy/Tray".to_owned()),
                downloads_path: None,
                ..Default::default()
            },
        )
        .expect("legacy settings save");

        let profile = get_active_game_installation_profile(&connection)
            .expect("active profile")
            .expect("manual profile");
        assert_eq!(
            library_settings_from_game_installation_profile(&profile)
                .mods_path
                .as_deref(),
            Some("/manual/Mods")
        );
        assert_eq!(
            get_app_setting(&connection, "mods_path").expect("legacy mods setting"),
            Some("/legacy/Mods".to_owned())
        );
    }

    #[test]
    fn profile_enum_and_json_parsing_fail_closed_for_unknown_values() {
        assert!(parse_game_operating_environment("future_runtime").is_err());
        assert!(parse_game_installation_profile_status("maybe").is_err());
        assert!(parse_game_installation_detection_method("automatic").is_err());
        assert!(parse_game_installation_confirmation_state("assumed").is_err());
        assert!(parse_game_installation_root_validation_state("unchecked").is_err());
        assert!(validate_profile_json_object("{}", "evidence").is_ok());
        assert!(validate_profile_json_object("[]", "evidence").is_err());
        assert!(validate_profile_json_object("not-json", "evidence").is_err());
    }

    #[test]
    fn initialize_repairs_missing_game_installation_profile_table() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute("DROP TABLE game_installation_roots", [])
            .expect("drop roots table");

        initialize(&mut connection).expect("schema repair");
        assert!(table_exists(&connection, "game_installation_roots").expect("table lookup"));
    }

    #[test]
    fn initialize_creates_apply_plan_persistence_foundation_tables() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        for table_name in [
            "apply_plans",
            "apply_plan_items",
            "apply_plan_item_signals",
            "apply_plan_item_blockers",
        ] {
            assert!(
                table_exists(&connection, table_name).expect("table lookup"),
                "missing table {table_name}"
            );
        }

        let migration_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 3 AND name = 'applyplan_persistence_foundation'",
                [],
                |row| row.get(0),
            )
            .expect("migration row");
        assert_eq!(migration_exists, 1);

        for index_name in [
            "idx_apply_plans_status_created_at",
            "idx_apply_plans_source_staging_plan_id",
            "idx_apply_plan_items_plan_id",
            "idx_apply_plan_items_file_id",
            "idx_apply_plan_items_status",
            "idx_apply_plan_item_signals_item_id",
            "idx_apply_plan_item_signals_kind",
            "idx_apply_plan_item_blockers_item_id",
            "idx_apply_plan_item_blockers_reason_code",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    params![index_name],
                    |row| row.get(0),
                )
                .expect("index lookup");
            assert_eq!(count, 1, "missing index {index_name}");
        }
    }

    #[test]
    fn initialize_creates_apply_plan_result_restore_foundation_tables() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        for table_name in [
            "apply_plan_runs",
            "apply_plan_results",
            "apply_plan_restore_entries",
        ] {
            assert!(
                table_exists(&connection, table_name).expect("table lookup"),
                "missing table {table_name}"
            );
        }

        let migration_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 4 AND name = 'applyplan_result_restore_foundation'",
                [],
                |row| row.get(0),
            )
            .expect("migration row");
        assert_eq!(migration_exists, 1);

        for index_name in [
            "idx_apply_plan_runs_plan_id",
            "idx_apply_plan_runs_status_created_at",
            "idx_apply_plan_results_run_id",
            "idx_apply_plan_results_plan_item_id",
            "idx_apply_plan_results_status",
            "idx_apply_plan_restore_entries_run_id",
            "idx_apply_plan_restore_entries_result_id",
            "idx_apply_plan_restore_entries_item_id",
            "idx_apply_plan_restore_entries_status",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    params![index_name],
                    |row| row.get(0),
                )
                .expect("index lookup");
            assert_eq!(count, 1, "missing index {index_name}");
        }
    }

    #[test]
    fn initialize_creates_apply_plan_context_snapshot_columns() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        for column_name in ["folder_config_json", "context_trail_json"] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('apply_plans') WHERE name = ?1",
                    params![column_name],
                    |row| row.get(0),
                )
                .expect("column lookup");
            assert_eq!(count, 1, "missing ApplyPlan context column {column_name}");
        }

        let migration_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 5 AND name = 'applyplan_context_snapshots'",
                [],
                |row| row.get(0),
            )
            .expect("migration row");
        assert_eq!(migration_exists, 1);
    }

    #[test]
    fn initialize_creates_apply_plan_hash_provenance_columns_and_guards() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        for column_name in [
            "plan_hash",
            "plan_hash_version",
            "plan_hash_algorithm",
            "plan_hash_created_at",
            "plan_provenance_json",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('apply_plans') WHERE name = ?1",
                    params![column_name],
                    |row| row.get(0),
                )
                .expect("column lookup");
            assert_eq!(count, 1, "missing ApplyPlan hash column {column_name}");
        }

        let migration_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 6 AND name = 'applyplan_hash_provenance_v1'",
                [],
                |row| row.get(0),
            )
            .expect("migration row");
        assert_eq!(migration_exists, 1);

        for trigger_name in [
            "trg_apply_plans_plan_hash_immutable",
            "trg_apply_plans_plan_provenance_immutable",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger' AND name = ?1",
                    params![trigger_name],
                    |row| row.get(0),
                )
                .expect("trigger lookup");
            assert_eq!(count, 1, "missing trigger {trigger_name}");
        }
    }

    #[test]
    fn initialize_creates_apply_plan_preview_snapshot_columns_and_guards() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        assert!(
            table_exists(&connection, "apply_plan_preview_snapshots").expect("table lookup"),
            "missing preview snapshot table"
        );

        for column_name in [
            "snapshot_id",
            "source_plan_kind",
            "source_plan_json",
            "source_scope_json",
            "folder_config_json",
            "context_trail_json",
            "preview_snapshot_hash",
            "preview_snapshot_hash_version",
            "preview_snapshot_hash_algorithm",
            "preview_snapshot_provenance_json",
            "consumed_apply_plan_id",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('apply_plan_preview_snapshots') WHERE name = ?1",
                    params![column_name],
                    |row| row.get(0),
                )
                .expect("column lookup");
            assert_eq!(count, 1, "missing preview snapshot column {column_name}");
        }

        for column_name in ["preview_snapshot_id", "preview_snapshot_hash"] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('apply_plans') WHERE name = ?1",
                    params![column_name],
                    |row| row.get(0),
                )
                .expect("apply_plans column lookup");
            assert_eq!(count, 1, "missing ApplyPlan snapshot column {column_name}");
        }

        let migration_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = 7 AND name = 'applyplan_preview_snapshots_v2'",
                [],
                |row| row.get(0),
            )
            .expect("migration row");
        assert_eq!(migration_exists, 1);

        for object_name in [
            "idx_apply_plan_preview_snapshots_hash",
            "idx_apply_plan_preview_snapshots_created_at",
            "idx_apply_plans_preview_snapshot_id_unique",
            "trg_apply_plan_preview_snapshots_identity_immutable",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                    params![object_name],
                    |row| row.get(0),
                )
                .expect("sqlite object lookup");
            assert_eq!(count, 1, "missing sqlite object {object_name}");
        }
    }

    #[test]
    fn ensure_schema_repairs_partial_apply_plan_preview_snapshot_schema() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute_batch(
                "DROP TABLE apply_plan_preview_snapshots;
                 CREATE TABLE apply_plan_preview_snapshots (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    snapshot_id TEXT NOT NULL UNIQUE
                 );",
            )
            .expect("create partial preview snapshot table");

        initialize(&mut connection).expect("repair schema");

        for column_name in [
            "source_plan_kind",
            "source_plan_json",
            "source_scope_json",
            "folder_config_json",
            "context_trail_json",
            "preview_snapshot_hash",
            "preview_snapshot_hash_version",
            "preview_snapshot_hash_algorithm",
            "preview_snapshot_provenance_json",
            "scan_session_id",
            "consumed_apply_plan_id",
            "created_at",
            "expires_at",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('apply_plan_preview_snapshots') WHERE name = ?1",
                    params![column_name],
                    |row| row.get(0),
                )
                .expect("column lookup");
            assert_eq!(
                count, 1,
                "missing repaired preview snapshot column {column_name}"
            );
        }

        for object_name in [
            "idx_apply_plan_preview_snapshots_hash",
            "idx_apply_plan_preview_snapshots_created_at",
            "idx_apply_plans_preview_snapshot_id_unique",
            "trg_apply_plan_preview_snapshots_identity_immutable",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                    params![object_name],
                    |row| row.get(0),
                )
                .expect("sqlite object lookup");
            assert_eq!(count, 1, "missing repaired sqlite object {object_name}");
        }
    }

    #[test]
    fn ensure_schema_repairs_apply_plan_result_restore_foundation_tables() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        connection
            .execute_batch(
                "DROP TABLE IF EXISTS apply_plan_restore_entries;
                 DROP TABLE IF EXISTS apply_plan_results;
                 DROP TABLE IF EXISTS apply_plan_runs;",
            )
            .expect("drop result restore tables");

        ensure_schema(&connection).expect("repair schema");

        for table_name in [
            "apply_plan_runs",
            "apply_plan_results",
            "apply_plan_restore_entries",
        ] {
            assert!(
                table_exists(&connection, table_name).expect("table lookup"),
                "missing repaired table {table_name}"
            );
        }

        for index_name in [
            "idx_apply_plan_runs_plan_id",
            "idx_apply_plan_runs_status_created_at",
            "idx_apply_plan_results_run_id",
            "idx_apply_plan_results_plan_item_id",
            "idx_apply_plan_results_status",
            "idx_apply_plan_restore_entries_run_id",
            "idx_apply_plan_restore_entries_result_id",
            "idx_apply_plan_restore_entries_item_id",
            "idx_apply_plan_restore_entries_status",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                    params![index_name],
                    |row| row.get(0),
                )
                .expect("index lookup");
            assert_eq!(count, 1, "missing repaired index {index_name}");
        }
    }

    #[test]
    fn runtime_seed_pack_includes_user_learned_aliases() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        seed_database(&mut connection, &seed_pack).expect("seed db");

        connection
            .execute(
                "INSERT INTO creators (canonical_name, notes, created_by_user, locked_by_user, preferred_path)
                 VALUES (?1, ?2, 1, 1, ?3)",
                params!["CustomMaker", "User-learned creator", "Creators/CustomMaker"],
            )
            .expect("creator");
        let creator_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO user_creator_aliases (creator_id, alias_name) VALUES (?1, ?2)",
                params![creator_id, "cmaker"],
            )
            .expect("alias");

        let runtime = load_runtime_seed_pack(&connection, &seed_pack).expect("runtime");
        assert_eq!(
            runtime.creator_lookup.get("cmaker"),
            Some(&"CustomMaker".to_owned())
        );
        let profile = runtime
            .creator_profiles
            .get("CustomMaker")
            .expect("custom profile");
        assert!(profile.locked_by_user);
        assert_eq!(
            profile.preferred_path.as_deref(),
            Some("Creators/CustomMaker")
        );
    }

    #[test]
    fn save_creator_learning_updates_file_review_and_version() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        seed_database(&mut connection, &seed_pack).expect("seed db");

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, subtype, confidence, source_location, parser_warnings
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "C:/Mods/Downloads/mystery.package",
                    "mystery.package",
                    ".package",
                    "CAS",
                    "Skinblend",
                    0.42_f64,
                    "mods",
                    serde_json::to_string(&vec![
                        "conflicting_creator_signals",
                        "low_confidence_parse",
                    ])
                    .expect("warnings json"),
                ],
            )
            .expect("file");
        let file_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO review_queue (file_id, reason, confidence) VALUES (?1, ?2, ?3)",
                params![file_id, "conflicting_creator_signals", 0.42_f64],
            )
            .expect("review 1");
        connection
            .execute(
                "INSERT INTO review_queue (file_id, reason, confidence) VALUES (?1, ?2, ?3)",
                params![file_id, "low_confidence_parse", 0.42_f64],
            )
            .expect("review 2");

        save_creator_learning(
            &mut connection,
            &LibrarySettings {
                mods_path: Some("C:/Mods".to_owned()),
                tray_path: None,
                downloads_path: None,
                ..Default::default()
            },
            file_id,
            "CustomMaker",
            Some("[CustomMaker]"),
            true,
            None,
        )
        .expect("save creator learning");

        let file_state: (String, f64, String) = connection
            .query_row(
                "SELECT c.canonical_name, f.confidence, f.parser_warnings
                 FROM files f
                 JOIN creators c ON f.creator_id = c.id
                 WHERE f.id = ?1",
                params![file_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("file state");
        assert_eq!(file_state.0, "CustomMaker");
        assert!(file_state.1 >= 0.92_f64);
        let parser_warnings: Vec<String> =
            serde_json::from_str(&file_state.2).expect("parser warnings");
        assert!(!parser_warnings.contains(&"conflicting_creator_signals".to_owned()));

        let remaining_reviews: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM review_queue WHERE file_id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .expect("remaining reviews");
        assert_eq!(remaining_reviews, 0);

        let alias_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM user_creator_aliases WHERE alias_name = ?1",
                params!["custommaker"],
                |row| row.get(0),
            )
            .expect("alias count");
        assert_eq!(alias_count, 1);

        let creator_meta: (i64, Option<String>) = connection
            .query_row(
                "SELECT locked_by_user, preferred_path FROM creators WHERE canonical_name = ?1",
                params!["CustomMaker"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("creator meta");
        assert_eq!(creator_meta.0, 1);
        assert_eq!(creator_meta.1.as_deref(), Some("Downloads"));

        assert!(get_creator_learning_version(&connection)
            .expect("learning version")
            .is_some());
    }

    #[test]
    fn save_category_override_updates_file_and_version() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");
        let seed_pack = load_seed_pack().expect("seed pack");
        seed_database(&mut connection, &seed_pack).expect("seed db");

        connection
            .execute(
                "INSERT INTO files (
                    path, filename, extension, kind, confidence, source_location, parser_warnings
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    "C:/Mods/Downloads/mystery.package",
                    "mystery.package",
                    ".package",
                    "Unknown",
                    0.33_f64,
                    "mods",
                    serde_json::to_string(&vec!["no_category_detected"]).expect("warnings"),
                ],
            )
            .expect("file");
        let file_id = connection.last_insert_rowid();
        connection
            .execute(
                "INSERT INTO review_queue (file_id, reason, confidence) VALUES (?1, ?2, ?3)",
                params![file_id, "low_confidence_parse", 0.33_f64],
            )
            .expect("review");

        save_category_override(&mut connection, file_id, "Gameplay", Some("Utility"))
            .expect("save override");

        let file_state: (String, Option<String>, f64, String) = connection
            .query_row(
                "SELECT kind, subtype, confidence, parser_warnings FROM files WHERE id = ?1",
                params![file_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("file state");
        assert_eq!(file_state.0, "Gameplay");
        assert_eq!(file_state.1.as_deref(), Some("Utility"));
        assert!(file_state.2 >= 0.82_f64);
        let parser_warnings: Vec<String> =
            serde_json::from_str(&file_state.3).expect("parser warnings");
        assert!(!parser_warnings.contains(&"no_category_detected".to_owned()));

        let review_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM review_queue WHERE file_id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .expect("review count");
        assert_eq!(review_count, 0);

        let override_row: (String, Option<String>) = connection
            .query_row(
                "SELECT kind, subtype FROM user_category_overrides WHERE match_path = ?1",
                params!["C:/Mods/Downloads/mystery.package"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("override row");
        assert_eq!(override_row.0, "Gameplay");
        assert_eq!(override_row.1.as_deref(), Some("Utility"));

        assert!(get_category_override_version(&connection)
            .expect("category override version")
            .is_some());
    }

    #[test]
    fn sync_category_override_path_moves_override_record() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        initialize(&mut connection).expect("schema");

        connection
            .execute(
                "INSERT INTO user_category_overrides (match_path, kind, subtype)
                 VALUES (?1, ?2, ?3)",
                params!["C:/Mods/Old/item.package", "CAS", "Hair"],
            )
            .expect("override");

        sync_category_override_path(
            &connection,
            "C:/Mods/Old/item.package",
            "C:/Mods/New/item.package",
        )
        .expect("sync override");

        let old_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM user_category_overrides WHERE match_path = ?1",
                params!["C:/Mods/Old/item.package"],
                |row| row.get(0),
            )
            .expect("old count");
        let new_row: (String, Option<String>) = connection
            .query_row(
                "SELECT kind, subtype FROM user_category_overrides WHERE match_path = ?1",
                params!["C:/Mods/New/item.package"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("new row");

        assert_eq!(old_count, 0);
        assert_eq!(new_row.0, "CAS");
        assert_eq!(new_row.1.as_deref(), Some("Hair"));
    }

    #[test]
    fn initialize_upgrades_existing_files_table_before_creating_download_index() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        connection
            .execute_batch(
                "CREATE TABLE files (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    path TEXT NOT NULL UNIQUE,
                    filename TEXT NOT NULL,
                    extension TEXT NOT NULL,
                    hash TEXT,
                    size INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT,
                    modified_at TEXT,
                    bundle_id INTEGER,
                    creator_id INTEGER,
                    kind TEXT NOT NULL DEFAULT 'Unknown',
                    subtype TEXT,
                    confidence REAL NOT NULL DEFAULT 0,
                    source_location TEXT NOT NULL,
                    scan_session_id INTEGER,
                    relative_depth INTEGER NOT NULL DEFAULT 0,
                    safety_notes TEXT NOT NULL DEFAULT '[]',
                    parser_warnings TEXT NOT NULL DEFAULT '[]',
                    indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE app_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    source TEXT NOT NULL DEFAULT 'seed',
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );",
            )
            .expect("legacy schema");

        initialize(&mut connection).expect("upgrade schema");

        let columns = connection
            .prepare("PRAGMA table_info(files)")
            .expect("pragma")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("column rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("columns");

        assert!(columns.contains(&"download_item_id".to_owned()));
        assert!(columns.contains(&"source_origin_path".to_owned()));
        assert!(columns.contains(&"archive_member_path".to_owned()));
        assert!(columns.contains(&"insights".to_owned()));
        assert!(columns.contains(&"content_fingerprint".to_owned()));
        assert!(columns.contains(&"content_fingerprint_kind".to_owned()));
        assert!(columns.contains(&"content_fingerprint_version".to_owned()));
        assert!(columns.contains(&"content_fingerprint_status".to_owned()));
        assert!(columns.contains(&"content_fingerprint_error".to_owned()));

        let index_exists: Option<String> = connection
            .query_row(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_files_download_item_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("index lookup");
        assert_eq!(index_exists.as_deref(), Some("idx_files_download_item_id"));

        let fingerprint_index_exists: Option<String> = connection
            .query_row(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_files_content_fingerprint'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("fingerprint index lookup");
        assert_eq!(
            fingerprint_index_exists.as_deref(),
            Some("idx_files_content_fingerprint")
        );
    }

    #[test]
    fn initialize_upgrades_watch_sources_before_creating_anchor_index() {
        let mut connection = Connection::open_in_memory().expect("in-memory db");
        connection
            .execute_batch(
                "CREATE TABLE files (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    path TEXT NOT NULL UNIQUE,
                    filename TEXT NOT NULL,
                    extension TEXT NOT NULL,
                    hash TEXT,
                    size INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT,
                    modified_at TEXT,
                    bundle_id INTEGER,
                    creator_id INTEGER,
                    kind TEXT NOT NULL DEFAULT 'Unknown',
                    subtype TEXT,
                    confidence REAL NOT NULL DEFAULT 0,
                    source_location TEXT NOT NULL,
                    scan_session_id INTEGER,
                    relative_depth INTEGER NOT NULL DEFAULT 0,
                    safety_notes TEXT NOT NULL DEFAULT '[]',
                    parser_warnings TEXT NOT NULL DEFAULT '[]',
                    indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE app_settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    source TEXT NOT NULL DEFAULT 'seed',
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE content_watch_sources (
                    subject_key TEXT PRIMARY KEY,
                    source_kind TEXT NOT NULL DEFAULT 'exact_page',
                    source_label TEXT,
                    source_url TEXT NOT NULL,
                    approved_by_user INTEGER NOT NULL DEFAULT 0 CHECK (approved_by_user IN (0, 1)),
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE content_watch_results (
                    subject_key TEXT PRIMARY KEY,
                    status TEXT NOT NULL DEFAULT 'unknown',
                    latest_version TEXT,
                    checked_at TEXT,
                    confidence TEXT NOT NULL DEFAULT 'unknown',
                    note TEXT,
                    evidence TEXT NOT NULL DEFAULT '[]',
                    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );",
            )
            .expect("legacy watch schema");

        initialize(&mut connection).expect("upgrade schema");

        let columns = connection
            .prepare("PRAGMA table_info(content_watch_sources)")
            .expect("pragma")
            .query_map([], |row| row.get::<_, String>(1))
            .expect("column rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("columns");
        assert!(columns.contains(&"anchor_file_id".to_owned()));

        let index_exists: Option<String> = connection
            .query_row(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'index' AND name = 'idx_content_watch_sources_anchor_file_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("index lookup");
        assert_eq!(
            index_exists.as_deref(),
            Some("idx_content_watch_sources_anchor_file_id")
        );
    }
}
