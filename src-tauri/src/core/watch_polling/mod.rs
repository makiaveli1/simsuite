use std::{sync::mpsc, thread, time::Duration};

use chrono::{DateTime, Utc};
use tauri::AppHandle;

use crate::{
    adapters::{ea_broken_mods::EABrokenModsAdapter, SourceAdapter, UpdateDecision},
    app_state::AppState,
    commands,
    core::{content_versions, special_mod_versions},
    database,
    error::{AppError, AppResult},
    models::{
        AccessTier, SourceBinding, SourceKind, UpdateStatus, WatchRefreshSummary, WorkspaceChange,
        WorkspaceDomain,
    },
    services::{
        candidate_discovery::CandidateDiscovery, scheduler::UpdateScheduler,
        update_events::UpdateEvents, LocalInventory, SharedRateLimiter,
    },
    MAIN_TRAY_ID,
};

const WATCH_POLLER_TICK_SECONDS: u64 = 60;
const WATCH_INITIAL_DELAY_SECONDS: i64 = 90;

pub fn restart_poller(app: &AppHandle, state: &AppState) -> AppResult<()> {
    stop_poller(state)?;
    refresh_tray_tooltip(app, state)?;

    if !state.automatic_watch_checks() {
        return Ok(());
    }

    let (stop_sender, stop_receiver) = mpsc::channel();
    {
        let control = state.watch_polling_control();
        let mut guard = control
            .lock()
            .map_err(|_| AppError::Message("Watch-polling control lock poisoned".to_owned()))?;
        guard.stop_sender = Some(stop_sender);
    }

    let app = app.clone();
    let state = state.clone();
    thread::spawn(move || watch_poller_loop(app, state, stop_receiver));
    Ok(())
}

pub fn refresh_watched_sources_now(
    app: &AppHandle,
    state: &AppState,
) -> AppResult<WatchRefreshSummary> {
    run_refresh_cycle(app, state)
}

fn stop_poller(state: &AppState) -> AppResult<()> {
    let control = state.watch_polling_control();
    let mut guard = control
        .lock()
        .map_err(|_| AppError::Message("Watch-polling control lock poisoned".to_owned()))?;
    if let Some(stop_sender) = guard.stop_sender.take() {
        let _ = stop_sender.send(());
    }
    Ok(())
}

fn watch_poller_loop(app: AppHandle, state: AppState, stop_receiver: mpsc::Receiver<()>) {
    let mut next_due_at = initial_due_at(&state);

    loop {
        if stop_receiver
            .recv_timeout(Duration::from_secs(WATCH_POLLER_TICK_SECONDS))
            .is_ok()
        {
            break;
        }

        if !state.automatic_watch_checks() {
            continue;
        }

        if Utc::now() < next_due_at {
            continue;
        }

        if let Err(error) = run_refresh_cycle(&app, &state) {
            let _ = save_watch_refresh_error(&state, Some(error.to_string()));
        }

        next_due_at = Utc::now() + chrono::Duration::hours(state.watch_check_interval_hours());
    }
}

fn initial_due_at(state: &AppState) -> DateTime<Utc> {
    let last_run = state
        .connection()
        .ok()
        .and_then(|connection| {
            database::get_app_setting(&connection, "watch_auto_last_run_at")
                .ok()
                .flatten()
        })
        .and_then(|value| DateTime::parse_from_rfc3339(&value).ok())
        .map(|value| value.with_timezone(&Utc));

    match last_run {
        Some(last_run) => last_run + chrono::Duration::hours(state.watch_check_interval_hours()),
        None => Utc::now() + chrono::Duration::seconds(WATCH_INITIAL_DELAY_SECONDS),
    }
}

fn run_refresh_cycle(app: &AppHandle, state: &AppState) -> AppResult<WatchRefreshSummary> {
    let mut connection = state.connection()?;
    let settings = database::get_library_settings(&connection)?;
    let seed_pack = state.seed_pack();

    let watch_file_ids =
        content_versions::list_auto_refreshable_watch_file_ids(&connection, &seed_pack)?;
    let mut checked_subjects = 0_i64;

    for file_id in watch_file_ids {
        let refreshed = content_versions::refresh_watch_source_for_library_file(
            &connection,
            &settings,
            &seed_pack,
            file_id,
        )?;
        if refreshed.is_some() {
            checked_subjects += 1;
        }
    }

    for profile_key in
        content_versions::list_auto_refreshable_special_profile_keys(&connection, &seed_pack)?
    {
        let Some(profile) = seed_pack
            .install_catalog
            .guided_profiles
            .iter()
            .find(|profile| profile.key == profile_key)
        else {
            continue;
        };

        let _ = special_mod_versions::load_or_refresh_latest_info(&connection, profile, true)?;
        checked_subjects += 1;
    }

    if let Err(error) = run_tracking_refresh(&connection, &settings) {
        tracing::warn!("Tracking refresh failed: {}", error);
    }

    let (exact_update_items, possible_update_items, unknown_watch_items) =
        content_versions::load_watch_counts(&connection)?;
    let checked_at = Utc::now().to_rfc3339();

    database::save_app_setting(
        &mut connection,
        "watch_auto_last_run_at",
        Some(&checked_at),
        "system",
    )?;
    database::save_app_setting(&mut connection, "watch_auto_last_error", None, "system")?;

    let summary = WatchRefreshSummary {
        checked_subjects,
        exact_update_items,
        possible_update_items,
        unknown_watch_items,
        checked_at,
    };

    refresh_tray_tooltip(app, state)?;
    let change = WorkspaceChange {
        domains: vec![WorkspaceDomain::Home, WorkspaceDomain::Library],
        reason: "watch-refresh-finished".to_owned(),
        item_ids: Vec::new(),
        family_keys: Vec::new(),
    };
    commands::emit_workspace_change(app, &change).map_err(AppError::Message)?;

    Ok(summary)
}

fn check_ea_broken_mods(
    connection: &rusqlite::Connection,
    tracked_mods: &[crate::models::LocalMod],
) -> AppResult<()> {
    let adapter = EABrokenModsAdapter::default();

    let binding = SourceBinding {
        id: "ea-broken-mods-check".to_string(),
        local_mod_id: String::new(),
        source_kind: SourceKind::EaBrokenMods,
        source_url: EABROKENMODS_INDEX.to_string(),
        provider_mod_id: None,
        provider_file_id: None,
        provider_repo: None,
        bind_method: "compatibility_check".to_string(),
        is_primary: false,
        custom_headers_json: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    let snapshot = match adapter.refresh_snapshot(&binding) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("EA broken mods check failed: {}", e);
            return Ok(());
        }
    };

    let broken_mods: Vec<String> = snapshot.release_asset_names;

    if broken_mods.is_empty() {
        return Ok(());
    }

    for local_mod in tracked_mods {
        let normalized = normalize_mod_name_for_ea(&local_mod.display_name);

        for broken in &broken_mods {
            let broken_normalized = normalize_mod_name_for_ea(broken);

            if normalized.contains(&broken_normalized)
                || broken_normalized.contains(&normalized)
                || strings_similarity(&normalized, &broken_normalized) > 0.8
            {
                let decision = UpdateDecision {
                    status: UpdateStatus::NeedsGameUpdate,
                    confidence: 0.95,
                    summary: Some(
                        "This mod may be broken by a recent game update. Check EA forums for details.".to_string()
                    ),
                };

                let _ = UpdateEvents::create_event(
                    connection,
                    &local_mod.id,
                    None,
                    &decision,
                    None,
                    None,
                );

                tracing::info!(
                    "Mod {} appears in EA broken mods list",
                    local_mod.display_name
                );
                break;
            }
        }
    }

    Ok(())
}

const EABROKENMODS_INDEX: &str =
    "https://forums.thesims.com/en_US/discussions/the-sims-4-mods-and-custom-content-en/broken-and-updated-sims-4-mods-and-cc";

fn normalize_mod_name_for_ea(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "")
        .replace('-', "")
        .replace('_', "")
}

fn strings_similarity(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;
    let a_chars: HashSet<char> = a.chars().collect();
    let b_chars: HashSet<char> = b.chars().collect();
    let intersection = a_chars.intersection(&b_chars).count() as f64;
    let union = a_chars.union(&b_chars).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn run_tracking_refresh(
    connection: &rusqlite::Connection,
    settings: &crate::models::LibrarySettings,
) -> AppResult<()> {
    let mods_path = match &settings.mods_path {
        Some(path) if !path.is_empty() => Path::new(path),
        _ => return Ok(()),
    };

    if !mods_path.exists() {
        tracing::debug!("Mods path does not exist, skipping tracking refresh");
        return Ok(());
    }

    tracing::info!("Running tracking refresh for local mods");

    let scan_result = LocalInventory::scan_and_update_local_mods(connection, mods_path)?;
    tracing::info!(
        "Local mod scan complete: {} mods found ({} new, {} updated), {} files processed",
        scan_result.mods_found,
        scan_result.new_mods,
        scan_result.updated_mods,
        scan_result.files_processed
    );

    let app_settings = crate::models::AppBehaviorSettings {
        keep_running_in_background: false,
        automatic_watch_checks: false,
        watch_check_interval_hours: 12,
        last_watch_check_at: None,
        last_watch_check_error: None,
        curseforge_api_key: database::get_app_setting(connection, "curseforge_api_key")
            .ok()
            .flatten(),
        github_api_token: database::get_app_setting(connection, "github_api_token")
            .ok()
            .flatten(),
    };

    let rate_limiter = SharedRateLimiter::default();
    let discovery = CandidateDiscovery::new(&app_settings, rate_limiter);

    let untracked_mods = get_untracked_local_mods(connection)?;
    tracing::debug!(
        "Found {} untracked mods for candidate discovery",
        untracked_mods.len()
    );

    for local_mod in untracked_mods {
        let files = LocalInventory::get_local_files(connection, &local_mod.id)?;
        if files.is_empty() {
            continue;
        }

        match discovery.discover_for_mod(&local_mod, &files) {
            Ok(candidates) => {
                if !candidates.is_empty() {
                    tracing::debug!(
                        "Discovered {} candidates for mod {}",
                        candidates.len(),
                        local_mod.display_name
                    );
                    if let Err(e) =
                        CandidateDiscovery::store_candidates(connection, &local_mod.id, &candidates)
                    {
                        tracing::warn!("Failed to store candidates: {}", e);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Candidate discovery failed for mod {}: {}",
                    local_mod.display_name,
                    e
                );
            }
        }
    }

    let tracked_mods = get_tracked_local_mods_with_bindings(connection)?;
    tracing::debug!(
        "Found {} tracked mods with bindings to check",
        tracked_mods.len()
    );

    for local_mod in &tracked_mods {
        let Some(binding_id) = local_mod.confirmed_source_id.as_ref() else {
            continue;
        };

        let binding = get_source_binding(connection, binding_id)?;
        let Some(binding) = binding else {
            continue;
        };

        let scheduler = UpdateScheduler::new();
        let last_checked = local_mod.last_checked_at.as_deref();
        if !scheduler.is_due(last_checked, binding.source_kind) {
            tracing::debug!(
                "Skipping {:?} for {} - not due for check yet (last checked: {:?})",
                binding.source_kind,
                local_mod.display_name,
                last_checked
            );
            continue;
        }

        let app_settings = crate::models::AppBehaviorSettings {
            keep_running_in_background: false,
            automatic_watch_checks: false,
            watch_check_interval_hours: 12,
            last_watch_check_at: None,
            last_watch_check_error: None,
            curseforge_api_key: database::get_app_setting(connection, "curseforge_api_key")
                .ok()
                .flatten(),
            github_api_token: database::get_app_setting(connection, "github_api_token")
                .ok()
                .flatten(),
        };
        let registry =
            crate::adapters::AdapterRegistry::new(&app_settings, SharedRateLimiter::default());
        let adapter = registry.for_kind(binding.source_kind);

        let Some(adapter) = adapter else {
            continue;
        };

        match adapter.refresh_snapshot(&binding) {
            Ok(snapshot) => {
                if matches!(
                    snapshot.access_tier,
                    AccessTier::PatronOnly | AccessTier::EarlyAccess
                ) && snapshot.version_text.is_none()
                {
                    if let Err(e) = UpdateEvents::create_patron_update_event(
                        connection,
                        &local_mod.id,
                        &binding.id,
                        snapshot.access_tier.clone(),
                    ) {
                        tracing::warn!("Failed to create patron update event: {}", e);
                    }
                } else {
                    let decision = crate::adapters::UpdateDecision {
                        status: determine_update_status(&snapshot, &local_mod),
                        confidence: snapshot.confidence,
                        summary: Some(format!(
                            "Version {} published {}",
                            snapshot.version_text.clone().unwrap_or_default(),
                            snapshot.published_at.clone().unwrap_or_default()
                        )),
                    };

                    if let Err(e) = UpdateEvents::create_event(
                        connection,
                        &local_mod.id,
                        Some(&binding.id),
                        &decision,
                        snapshot.version_text.as_deref(),
                        snapshot.published_at.as_deref(),
                    ) {
                        tracing::warn!("Failed to create update event: {}", e);
                    }

                    let new_confidence = snapshot.confidence;
                    LocalInventory::update_mod_status(
                        connection,
                        &local_mod.id,
                        decision.status,
                        new_confidence,
                        Some(&binding.id),
                    )?;
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to refresh snapshot for mod {}: {}",
                    local_mod.display_name,
                    e
                );
                let decision = crate::adapters::UpdateDecision {
                    status: crate::models::UpdateStatus::SourceUnreachable,
                    confidence: 0.0,
                    summary: Some(format!("Source unreachable: {}", e)),
                };
                let _ = UpdateEvents::create_event(
                    connection,
                    &local_mod.id,
                    Some(&binding.id),
                    &decision,
                    None,
                    None,
                );
                let _ = LocalInventory::update_mod_status(
                    connection,
                    &local_mod.id,
                    crate::models::UpdateStatus::SourceUnreachable,
                    0.0,
                    Some(&binding.id),
                );
            }
        }
    }

    tracing::info!("Tracking refresh complete");

    if let Err(e) = check_ea_broken_mods(connection, &tracked_mods) {
        tracing::warn!("EA broken mods check failed: {}", e);
    }

    Ok(())
}

fn get_untracked_local_mods(
    connection: &rusqlite::Connection,
) -> AppResult<Vec<crate::models::LocalMod>> {
    let mut stmt = connection.prepare(
        "SELECT id, display_name, normalized_name, creator_name, category,
                local_root_path, tracking_mode, source_confidence, confirmed_source_id,
                current_status, last_checked_at, created_at, updated_at
         FROM local_mods
         WHERE tracking_mode = 'auto' AND confirmed_source_id IS NULL
         ORDER BY display_name
         LIMIT 50",
    )?;

    let mods = stmt
        .query_map([], |row| {
            Ok(crate::models::LocalMod {
                id: row.get(0)?,
                display_name: row.get(1)?,
                normalized_name: row.get(2)?,
                creator_name: row.get(3)?,
                category: row.get(4)?,
                local_root_path: row.get(5)?,
                tracking_mode: parse_tracking_mode(&row.get::<_, String>(6)?),
                source_confidence: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                confirmed_source_id: row.get(8)?,
                current_status: parse_update_status(&row.get::<_, String>(9)?),
                last_checked_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(mods)
}

fn get_tracked_local_mods_with_bindings(
    connection: &rusqlite::Connection,
) -> AppResult<Vec<crate::models::LocalMod>> {
    let mut stmt = connection.prepare(
        "SELECT lm.id, lm.display_name, lm.normalized_name, lm.creator_name, lm.category,
                lm.local_root_path, lm.tracking_mode, lm.source_confidence, lm.confirmed_source_id,
                lm.current_status, lm.last_checked_at, lm.created_at, lm.updated_at
         FROM local_mods lm
         INNER JOIN source_bindings sb ON lm.confirmed_source_id = sb.id
         WHERE lm.tracking_mode IN ('auto', 'manual')
         ORDER BY lm.display_name
         LIMIT 100",
    )?;

    let mods = stmt
        .query_map([], |row| {
            Ok(crate::models::LocalMod {
                id: row.get(0)?,
                display_name: row.get(1)?,
                normalized_name: row.get(2)?,
                creator_name: row.get(3)?,
                category: row.get(4)?,
                local_root_path: row.get(5)?,
                tracking_mode: parse_tracking_mode(&row.get::<_, String>(6)?),
                source_confidence: row.get::<_, Option<f64>>(7)?.unwrap_or(0.0),
                confirmed_source_id: row.get(8)?,
                current_status: parse_update_status(&row.get::<_, String>(9)?),
                last_checked_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(mods)
}

fn get_source_binding(
    connection: &rusqlite::Connection,
    binding_id: &str,
) -> AppResult<Option<crate::models::SourceBinding>> {
    use rusqlite::OptionalExtension;

    let result = connection
        .query_row(
            "SELECT id, local_mod_id, source_kind, source_url, provider_mod_id,
                    provider_file_id, provider_repo, bind_method, is_primary,
                    custom_headers_json, created_at, updated_at
             FROM source_bindings
             WHERE id = ?1",
            rusqlite::params![binding_id],
            |row| {
                Ok(crate::models::SourceBinding {
                    id: row.get(0)?,
                    local_mod_id: row.get(1)?,
                    source_kind: parse_source_kind(&row.get::<_, String>(2)?),
                    source_url: row.get(3)?,
                    provider_mod_id: row.get(4)?,
                    provider_file_id: row.get(5)?,
                    provider_repo: row.get(6)?,
                    bind_method: row.get(7)?,
                    is_primary: row.get::<_, i64>(8)? != 0,
                    custom_headers_json: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .optional()?;

    Ok(result)
}

fn parse_tracking_mode(value: &str) -> crate::models::TrackingMode {
    match value {
        "auto" => crate::models::TrackingMode::Auto,
        "manual" => crate::models::TrackingMode::Manual,
        "ignored" => crate::models::TrackingMode::Ignored,
        _ => crate::models::TrackingMode::DetectedOnly,
    }
}

fn parse_update_status(value: &str) -> crate::models::UpdateStatus {
    match value {
        "up_to_date" => crate::models::UpdateStatus::UpToDate,
        "confirmed_update" => crate::models::UpdateStatus::ConfirmedUpdate,
        "probable_update" => crate::models::UpdateStatus::ProbableUpdate,
        "source_activity" => crate::models::UpdateStatus::SourceActivity,
        "source_unreachable" => crate::models::UpdateStatus::SourceUnreachable,
        _ => crate::models::UpdateStatus::Untracked,
    }
}

fn parse_source_kind(value: &str) -> crate::models::SourceKind {
    match value {
        "curseforge" => crate::models::SourceKind::CurseForge,
        "github" => crate::models::SourceKind::GitHub,
        "nexus" => crate::models::SourceKind::Nexus,
        "feed" => crate::models::SourceKind::Feed,
        "structured_page" => crate::models::SourceKind::StructuredPage,
        _ => crate::models::SourceKind::GenericPage,
    }
}

fn determine_update_status(
    snapshot: &crate::adapters::RemoteSnapshot,
    _local_mod: &crate::models::LocalMod,
) -> crate::models::UpdateStatus {
    if snapshot.version_text.is_none() && snapshot.published_at.is_none() {
        return crate::models::UpdateStatus::SourceActivity;
    }

    crate::models::UpdateStatus::ProbableUpdate
}

use std::path::Path;

fn refresh_tray_tooltip(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let connection = state.connection()?;
    let (exact_update_items, possible_update_items, unknown_watch_items) =
        content_versions::load_watch_counts(&connection)?;

    if let Some(tray) = app.tray_by_id(MAIN_TRAY_ID) {
        tray.set_tooltip(Some(build_tray_tooltip(
            exact_update_items,
            possible_update_items,
            unknown_watch_items,
        )))
        .map_err(|error| AppError::Message(error.to_string()))?;
    }

    Ok(())
}

fn build_tray_tooltip(exact_updates: i64, possible_updates: i64, unknown_updates: i64) -> String {
    if exact_updates > 0 {
        return format!(
            "SimSuite - {exact_updates} confirmed mod update{} waiting",
            if exact_updates == 1 { "" } else { "s" }
        );
    }

    if possible_updates > 0 {
        return format!(
            "SimSuite - {possible_updates} possible mod update{} to check",
            if possible_updates == 1 { "" } else { "s" }
        );
    }

    if unknown_updates > 0 {
        return format!(
            "SimSuite - {unknown_updates} watched item{} still unclear",
            if unknown_updates == 1 { "" } else { "s" }
        );
    }

    "SimSuite - watched mods look current".to_owned()
}

fn save_watch_refresh_error(state: &AppState, error: Option<String>) -> AppResult<()> {
    let mut connection = state.connection()?;
    database::save_app_setting(
        &mut connection,
        "watch_auto_last_error",
        error.as_deref(),
        "system",
    )
}

#[cfg(test)]
mod tests {
    use super::{build_tray_tooltip, initial_due_at};
    use crate::{app_state::AppState, database, seed};
    use chrono::Utc;
    use rusqlite::Connection;
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    fn fake_state() -> AppState {
        let database_path = std::env::temp_dir().join(format!(
            "simsuite-watch-test-{}-{}.sqlite3",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let mut connection = Connection::open(&database_path).expect("db");
        database::initialize(&mut connection).expect("schema");
        let seed_pack = seed::load_seed_pack().expect("seed");
        database::seed_database(&mut connection, &seed_pack).expect("seed db");

        AppState {
            database_path,
            seed_pack: Arc::new(seed_pack),
            scan_status: Arc::new(Mutex::new(Default::default())),
            downloads_status: Arc::new(Mutex::new(Default::default())),
            keep_running_in_background: Arc::new(Mutex::new(false)),
            automatic_watch_checks: Arc::new(Mutex::new(true)),
            watch_check_interval_hours: Arc::new(Mutex::new(12)),
            downloads_watcher_control: Arc::new(Mutex::new(Default::default())),
            watch_polling_control: Arc::new(Mutex::new(Default::default())),
            downloads_processing_lock: Arc::new(Mutex::new(())),
            app_data_dir: PathBuf::new(),
        }
    }

    #[test]
    fn tray_tooltip_prefers_exact_updates() {
        assert_eq!(
            build_tray_tooltip(2, 1, 1),
            "SimSuite - 2 confirmed mod updates waiting"
        );
        assert_eq!(
            build_tray_tooltip(0, 1, 0),
            "SimSuite - 1 possible mod update to check"
        );
    }

    #[test]
    fn first_due_time_waits_briefly_when_never_checked() {
        let state = fake_state();
        let due_at = initial_due_at(&state);
        assert!(due_at > Utc::now());
    }
}
