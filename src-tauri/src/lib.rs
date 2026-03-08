mod app_state;
mod commands;
mod core;
mod database;
mod error;
mod models;
mod seed;

use app_state::AppState;
use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,simsuite=debug")),
        )
        .try_init();

    tauri::Builder::default()
        .setup(|app| {
            let state = AppState::initialise(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library_settings,
            commands::save_library_paths,
            commands::detect_default_library_paths,
            commands::pick_folder,
            commands::get_home_overview,
            commands::scan_library,
            commands::start_scan,
            commands::get_scan_status,
            commands::get_library_facets,
            commands::get_duplicate_overview,
            commands::list_duplicate_pairs,
            commands::list_rule_presets,
            commands::preview_organization,
            commands::get_review_queue,
            commands::list_snapshots,
            commands::apply_preview_organization,
            commands::restore_snapshot,
            commands::list_library_files,
            commands::get_creator_audit,
            commands::get_category_audit,
            commands::get_creator_audit_group_files,
            commands::get_category_audit_group_files,
            commands::get_file_detail,
            commands::save_creator_learning,
            commands::apply_creator_audit,
            commands::apply_category_audit,
            commands::save_category_override
        ])
        .run(tauri::generate_context!())
        .expect("error while running SimSuite");
}
