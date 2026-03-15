use std::{sync::mpsc, thread, time::Duration};

use chrono::{DateTime, Utc};
use tauri::AppHandle;

use crate::{
    app_state::AppState,
    commands,
    core::{content_versions, special_mod_versions},
    database,
    error::{AppError, AppResult},
    models::{WatchRefreshSummary, WorkspaceChange, WorkspaceDomain},
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
