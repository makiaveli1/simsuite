use std::{
    fs,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::{
    database,
    error::{AppError, AppResult},
    models::{DownloadsWatcherState, DownloadsWatcherStatus, LibrarySettings, ScanStatus},
    seed::{self, SeedPack},
};

#[derive(Default)]
pub struct DownloadsWatcherControl {
    pub stop_sender: Option<mpsc::Sender<()>>,
}

#[derive(Default)]
pub struct WatchPollingControl {
    pub stop_sender: Option<mpsc::Sender<()>>,
}

#[derive(Clone)]
pub struct AppState {
    pub database_path: PathBuf,
    pub seed_pack: Arc<SeedPack>,
    pub scan_status: Arc<Mutex<ScanStatus>>,
    pub downloads_status: Arc<Mutex<DownloadsWatcherStatus>>,
    pub keep_running_in_background: Arc<Mutex<bool>>,
    pub automatic_watch_checks: Arc<Mutex<bool>>,
    pub watch_check_interval_hours: Arc<Mutex<i64>>,
    pub downloads_watcher_control: Arc<Mutex<DownloadsWatcherControl>>,
    pub watch_polling_control: Arc<Mutex<WatchPollingControl>>,
    pub downloads_processing_lock: Arc<Mutex<()>>,
    #[allow(dead_code)]
    pub app_data_dir: PathBuf,
}

impl AppState {
    pub fn initialise(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_data_dir = resolve_app_data_dir(
            app.path()
                .app_data_dir()
                .map_err(|error| AppError::Message(error.to_string()))?,
        );

        fs::create_dir_all(&app_data_dir)?;

        let database_path = app_data_dir.join("simsuite.sqlite3");
        let mut connection = open_connection(&database_path)?;

        database::initialize(&mut connection)?;

        let seed_pack = seed::load_seed_pack()?;
        database::seed_database(&mut connection, &seed_pack)?;
        let library_settings = database::get_library_settings(&connection)?;
        let keep_running_in_background = parse_bool_setting(database::get_app_setting(
            &connection,
            "keep_running_in_background",
        )?);
        let automatic_watch_checks = parse_watch_checks_setting(database::get_app_setting(
            &connection,
            "automatic_watch_checks",
        )?);
        let watch_check_interval_hours = parse_watch_check_interval_hours(
            database::get_app_setting(&connection, "watch_check_interval_hours")?,
        );
        let initial_downloads_status = build_initial_downloads_status(&library_settings);

        Ok(Self {
            database_path,
            seed_pack: Arc::new(seed_pack),
            scan_status: Arc::new(Mutex::new(ScanStatus::default())),
            downloads_status: Arc::new(Mutex::new(initial_downloads_status)),
            keep_running_in_background: Arc::new(Mutex::new(keep_running_in_background)),
            automatic_watch_checks: Arc::new(Mutex::new(automatic_watch_checks)),
            watch_check_interval_hours: Arc::new(Mutex::new(watch_check_interval_hours)),
            downloads_watcher_control: Arc::new(Mutex::new(DownloadsWatcherControl::default())),
            watch_polling_control: Arc::new(Mutex::new(WatchPollingControl::default())),
            downloads_processing_lock: Arc::new(Mutex::new(())),
            app_data_dir,
        })
    }

    pub fn connection(&self) -> AppResult<Connection> {
        open_connection(&self.database_path)
    }

    pub fn seed_pack(&self) -> Arc<SeedPack> {
        Arc::clone(&self.seed_pack)
    }

    pub fn scan_status(&self) -> Arc<Mutex<ScanStatus>> {
        Arc::clone(&self.scan_status)
    }

    pub fn downloads_status(&self) -> Arc<Mutex<DownloadsWatcherStatus>> {
        Arc::clone(&self.downloads_status)
    }

    pub fn keep_running_in_background(&self) -> bool {
        self.keep_running_in_background
            .lock()
            .map(|value| *value)
            .unwrap_or(false)
    }

    pub fn set_keep_running_in_background(&self, enabled: bool) -> AppResult<()> {
        let keep_running_in_background = Arc::clone(&self.keep_running_in_background);
        let mut guard = keep_running_in_background
            .lock()
            .map_err(|_| AppError::Message("Background mode lock poisoned".to_owned()))?;
        *guard = enabled;
        Ok(())
    }

    pub fn automatic_watch_checks(&self) -> bool {
        self.automatic_watch_checks
            .lock()
            .map(|value| *value)
            .unwrap_or(false)
    }

    pub fn set_automatic_watch_checks(&self, enabled: bool) -> AppResult<()> {
        let automatic_watch_checks = Arc::clone(&self.automatic_watch_checks);
        let mut guard = automatic_watch_checks
            .lock()
            .map_err(|_| AppError::Message("Automatic watch-check lock poisoned".to_owned()))?;
        *guard = enabled;
        Ok(())
    }

    pub fn watch_check_interval_hours(&self) -> i64 {
        self.watch_check_interval_hours
            .lock()
            .map(|value| *value)
            .unwrap_or_else(|_| default_watch_check_interval_hours())
    }

    pub fn set_watch_check_interval_hours(&self, hours: i64) -> AppResult<()> {
        let watch_check_interval_hours = Arc::clone(&self.watch_check_interval_hours);
        let mut guard = watch_check_interval_hours
            .lock()
            .map_err(|_| AppError::Message("Watch-check interval lock poisoned".to_owned()))?;
        *guard = clamp_watch_check_interval_hours(hours);
        Ok(())
    }

    pub fn downloads_watcher_control(&self) -> Arc<Mutex<DownloadsWatcherControl>> {
        Arc::clone(&self.downloads_watcher_control)
    }

    pub fn watch_polling_control(&self) -> Arc<Mutex<WatchPollingControl>> {
        Arc::clone(&self.watch_polling_control)
    }

    pub fn downloads_processing_lock(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.downloads_processing_lock)
    }
}

fn resolve_app_data_dir(default_dir: PathBuf) -> PathBuf {
    std::env::var("SIMSUITE_APP_DATA_DIR")
        .ok()
        .and_then(clean_override_path)
        .map(PathBuf::from)
        .unwrap_or(default_dir)
}

fn clean_override_path(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn parse_bool_setting(value: Option<String>) -> bool {
    value
        .as_deref()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn default_watch_check_interval_hours() -> i64 {
    12
}

fn clamp_watch_check_interval_hours(value: i64) -> i64 {
    value.clamp(1, 168)
}

fn parse_watch_checks_setting(value: Option<String>) -> bool {
    value
        .map(|value| value.trim().to_ascii_lowercase())
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn parse_watch_check_interval_hours(value: Option<String>) -> i64 {
    value
        .and_then(|value| value.trim().parse::<i64>().ok())
        .map(clamp_watch_check_interval_hours)
        .unwrap_or_else(default_watch_check_interval_hours)
}

fn build_initial_downloads_status(settings: &LibrarySettings) -> DownloadsWatcherStatus {
    let Some(downloads_path) = settings
        .downloads_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return DownloadsWatcherStatus::default();
    };

    let watched_root = PathBuf::from(downloads_path);
    if !watched_root.exists() {
        return DownloadsWatcherStatus {
            state: DownloadsWatcherState::Error,
            watched_path: Some(watched_root.to_string_lossy().to_string()),
            configured: true,
            current_item: None,
            last_run_at: None,
            last_change_at: None,
            last_error: Some("Downloads folder does not exist.".to_owned()),
            ready_items: 0,
            needs_review_items: 0,
            active_items: 0,
            ..Default::default()
        };
    }

    DownloadsWatcherStatus {
        state: DownloadsWatcherState::Processing,
        watched_path: Some(watched_root.to_string_lossy().to_string()),
        configured: true,
        current_item: Some("Initial inbox refresh".to_owned()),
        last_run_at: None,
        last_change_at: None,
        last_error: None,
        ready_items: 0,
        needs_review_items: 0,
        active_items: 0,
        ..Default::default()
    }
}

fn open_connection(database_path: &PathBuf) -> AppResult<Connection> {
    let connection = Connection::open(database_path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_downloads_status_is_unconfigured_without_a_path() {
        let settings = LibrarySettings {
            mods_path: None,
            tray_path: None,
            downloads_path: None,
            download_reject_folder: None,
        };

        let status = build_initial_downloads_status(&settings);
        assert!(!status.configured);
        assert_eq!(status.state, DownloadsWatcherState::Idle);
    }

    #[test]
    fn initial_downloads_status_starts_as_processing_for_real_downloads_path() {
        let downloads_root = std::env::temp_dir().join("simsuite-downloads-test");
        let settings = LibrarySettings {
            mods_path: None,
            tray_path: None,
            downloads_path: Some(downloads_root.to_string_lossy().to_string()),
            download_reject_folder: None,
        };

        fs::create_dir_all(settings.downloads_path.as_ref().expect("downloads path"))
            .expect("temp downloads path");
        let status = build_initial_downloads_status(&settings);

        assert!(status.configured);
        assert_eq!(status.state, DownloadsWatcherState::Processing);
        assert_eq!(
            status.current_item.as_deref(),
            Some("Initial inbox refresh")
        );
        let _ = fs::remove_dir_all(downloads_root);
    }

    #[test]
    fn resolve_app_data_dir_keeps_default_without_override() {
        let default_dir = PathBuf::from("C:/Users/Test/AppData/Default");

        assert_eq!(resolve_app_data_dir(default_dir.clone()), default_dir);
    }

    #[test]
    fn clean_override_path_trims_non_empty_values() {
        assert_eq!(
            clean_override_path(" C:/Temp/SimSuiteSmoke ".to_owned()),
            Some("C:/Temp/SimSuiteSmoke".to_owned())
        );
        assert_eq!(clean_override_path("   ".to_owned()), None);
    }

    #[test]
    fn watch_check_settings_default_safely() {
        assert!(!parse_watch_checks_setting(None));
        assert_eq!(
            parse_watch_check_interval_hours(None),
            default_watch_check_interval_hours()
        );
    }

    #[test]
    fn watch_check_interval_is_clamped() {
        assert_eq!(parse_watch_check_interval_hours(Some("0".to_owned())), 1);
        assert_eq!(
            parse_watch_check_interval_hours(Some("999".to_owned())),
            168
        );
        assert_eq!(parse_watch_check_interval_hours(Some("24".to_owned())), 24);
    }
}
