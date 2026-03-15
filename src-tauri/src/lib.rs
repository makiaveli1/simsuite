mod app_state;
mod commands;
mod core;
mod database;
mod error;
mod models;
mod seed;

use app_state::AppState;
use core::{downloads_watcher, watch_polling};
use tauri::{
    menu::MenuBuilder,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tracing_subscriber::EnvFilter;

pub const MAIN_TRAY_ID: &str = "main-tray";
const TRAY_OPEN_ID: &str = "tray-open";
const TRAY_EXIT_ID: &str = "tray-exit";

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub(crate) fn ensure_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    if app.tray_by_id(MAIN_TRAY_ID).is_some() {
        return Ok(());
    }

    let menu = MenuBuilder::new(app)
        .text(TRAY_OPEN_ID, "Open SimSuite")
        .separator()
        .text(TRAY_EXIT_ID, "Exit SimSuite")
        .build()?;

    let mut tray_builder = TrayIconBuilder::with_id(MAIN_TRAY_ID)
        .menu(&menu)
        .tooltip("SimSuite")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_OPEN_ID => show_main_window(app),
            TRAY_EXIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray_builder = tray_builder.icon(icon);
    }

    let _tray = tray_builder.build(app)?;
    Ok(())
}

pub(crate) fn sync_tray_visibility(app: &tauri::AppHandle, visible: bool) -> tauri::Result<()> {
    if visible {
        ensure_tray(app)?;
    }

    if let Some(tray) = app.tray_by_id(MAIN_TRAY_ID) {
        tray.set_visible(visible)?;
    }

    Ok(())
}

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
            downloads_watcher::restart_watcher(app.handle(), &state)?;
            watch_polling::restart_poller(app.handle(), &state)?;
            app.manage(state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if !state.keep_running_in_background() {
                    return;
                }

                if sync_tray_visibility(&app, true).is_ok() {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    tracing::warn!(
                        "Background mode is enabled, but the tray icon could not be created."
                    );
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library_settings,
            commands::get_app_behavior_settings,
            commands::save_app_behavior_settings,
            commands::save_library_paths,
            commands::detect_default_library_paths,
            commands::pick_folder,
            commands::get_home_overview,
            commands::scan_library,
            commands::start_scan,
            commands::get_scan_status,
            commands::get_downloads_watcher_status,
            commands::refresh_downloads_inbox,
            commands::get_downloads_bootstrap,
            commands::get_downloads_inbox,
            commands::get_downloads_queue,
            commands::get_downloads_selection,
            commands::get_download_item_detail,
            commands::preview_download_item,
            commands::get_download_item_guided_plan,
            commands::get_download_item_review_plan,
            commands::get_library_facets,
            commands::get_duplicate_overview,
            commands::list_duplicate_pairs,
            commands::list_rule_presets,
            commands::preview_organization,
            commands::get_review_queue,
            commands::list_snapshots,
            commands::apply_preview_organization,
            commands::restore_snapshot,
            commands::apply_download_item,
            commands::apply_guided_download_item,
            commands::apply_special_review_fix,
            commands::apply_review_plan_action,
            commands::ignore_download_item,
            commands::list_library_files,
            commands::list_library_watch_items,
            commands::list_library_watch_setup_items,
            commands::list_library_watch_review_items,
            commands::get_creator_audit,
            commands::get_category_audit,
            commands::get_creator_audit_group_files,
            commands::get_category_audit_group_files,
            commands::get_file_detail,
            commands::save_watch_source_for_file,
            commands::save_watch_sources_for_files,
            commands::clear_watch_source_for_file,
            commands::refresh_watch_source_for_file,
            commands::refresh_watched_sources,
            commands::save_creator_learning,
            commands::apply_creator_audit,
            commands::apply_category_audit,
            commands::save_category_override
        ])
        .run(tauri::generate_context!())
        .expect("error while running SimSuite");
}
