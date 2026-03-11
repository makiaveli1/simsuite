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

#[derive(Clone)]
pub struct AppState {
    pub database_path: PathBuf,
    pub seed_pack: Arc<SeedPack>,
    pub scan_status: Arc<Mutex<ScanStatus>>,
    pub downloads_status: Arc<Mutex<DownloadsWatcherStatus>>,
    pub downloads_watcher_control: Arc<Mutex<DownloadsWatcherControl>>,
    pub downloads_processing_lock: Arc<Mutex<()>>,
    #[allow(dead_code)]
    pub app_data_dir: PathBuf,
}

impl AppState {
    pub fn initialise(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::Message(error.to_string()))?;

        fs::create_dir_all(&app_data_dir)?;

        let database_path = app_data_dir.join("simsuite.sqlite3");
        let mut connection = open_connection(&database_path)?;

        database::initialize(&mut connection)?;

        let seed_pack = seed::load_seed_pack()?;
        database::seed_database(&mut connection, &seed_pack)?;
        let library_settings = database::get_library_settings(&connection)?;
        let initial_downloads_status = build_initial_downloads_status(&library_settings);

        Ok(Self {
            database_path,
            seed_pack: Arc::new(seed_pack),
            scan_status: Arc::new(Mutex::new(ScanStatus::default())),
            downloads_status: Arc::new(Mutex::new(initial_downloads_status)),
            downloads_watcher_control: Arc::new(Mutex::new(DownloadsWatcherControl::default())),
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

    pub fn downloads_watcher_control(&self) -> Arc<Mutex<DownloadsWatcherControl>> {
        Arc::clone(&self.downloads_watcher_control)
    }

    pub fn downloads_processing_lock(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.downloads_processing_lock)
    }
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
}
