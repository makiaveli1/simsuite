use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

use crate::{
    database,
    error::{AppError, AppResult},
    models::ScanStatus,
    seed::{self, SeedPack},
};

#[derive(Clone)]
pub struct AppState {
    pub database_path: PathBuf,
    pub seed_pack: Arc<SeedPack>,
    pub scan_status: Arc<Mutex<ScanStatus>>,
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

        Ok(Self {
            database_path,
            seed_pack: Arc::new(seed_pack),
            scan_status: Arc::new(Mutex::new(ScanStatus::default())),
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
}

fn open_connection(database_path: &PathBuf) -> AppResult<Connection> {
    let connection = Connection::open(database_path)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.busy_timeout(Duration::from_secs(5))?;
    Ok(connection)
}
