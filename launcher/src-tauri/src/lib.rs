//! Stabled launcher backend.

mod auth;
mod backups;
mod commands;
mod compat;
mod crash;
mod config;
mod error;
mod gallery;
mod images;
mod logs;
mod minecraft;
mod mods;
mod news;
mod optimiser;
mod paths;
mod saves;
mod settings;
mod state;
mod store;
mod system;
mod util;
#[cfg(feature = "dev-tools")]
pub mod devtools;

use std::sync::Arc;
use tauri::Manager;

#[cfg(windows)]
pub use optimiser::run_elevated_helper;

fn locate_gallery(app: &tauri::App) -> Option<std::path::PathBuf> {
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join("backgrounds");
        if p.is_dir() {
            return Some(p);
        }
    }
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("backgrounds");
    dev.is_dir().then_some(dev)
}

fn locate_bundled_clients(app: &tauri::App) -> Option<std::path::PathBuf> {
    if let Ok(dir) = app.path().resource_dir() {
        let p = dir.join("clients");
        if p.is_dir() {
            return Some(p);
        }
    }
    // Development: the jars copied by `npm run bundle-client`.
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources").join("clients");
    dev.is_dir().then_some(dev)
}

pub fn run() {
    let _ = paths::ensure_base_dirs();
    util::log::init();
    log_info!("Stabled {} starting on {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS);
    let state = Arc::new(state::AppState::new());

    tauri::Builder::default()
        // One launcher at a time, so two windows can never launch the same Save or compete for Discord presence.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            match locate_bundled_clients(app) {
                Some(p) => saves::managed::set_bundled_clients_dir(p),
                None => log_warn!("No Stabled client builds are bundled; launching Saves will be unavailable"),
            }
            log_info!("Bundled Stabled client builds: {:?}", saves::managed::bundled_versions());
            match locate_gallery(app) {
                Some(p) => gallery::set_dir(p),
                None => log_warn!("The background gallery isn't bundled"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::system_static,
            commands::system_dynamic,
            commands::settings_get,
            commands::settings_set,
            commands::appearance_get,
            commands::appearance_set,
            commands::appearance_set_background,
            commands::appearance_set_gallery_background,
            commands::appearance_reset_background,
            commands::image_inspect,
            commands::versions_list,
            commands::saves_list,
            commands::save_get,
            commands::save_validate_name,
            commands::save_create,
            commands::save_rename,
            commands::save_set_icon,
            commands::save_duplicate,
            commands::save_delete,
            commands::save_summary,
            commands::save_open_folder,
            commands::save_settings_get,
            commands::save_settings_set,
            commands::save_reset_intro,
            commands::mods_list,
            commands::mods_add,
            commands::mods_set_enabled,
            commands::mods_remove,
            commands::mods_restore_component,
            commands::save_install,
            commands::save_launch,
            commands::save_stop,
            commands::save_clear_error,
            commands::auth_status,
            commands::auth_restore,
            commands::auth_sign_in,
            commands::auth_cancel,
            commands::auth_sign_out,
            commands::news_get,
            commands::news_detail,
            commands::open_external,
            commands::open_discord_invite,
            commands::open_data_folder,
            commands::shared_cache_clear,
            commands::optimiser_view,
            commands::optimiser_apply,
            commands::optimiser_restore,
            commands::optimiser_apply_all,
            commands::optimiser_restore_all,
            commands::mods_check,
            commands::save_logs,
            commands::save_log_read,
            commands::crash_explain,
            commands::backups_view,
            commands::backups_create,
            commands::backups_restore,
            commands::backups_delete,
            commands::backups_open_folder,
            commands::bisect_start,
            commands::bisect_apply,
            commands::bisect_end,
            commands::store_search,
            commands::store_project,
            commands::store_installed,
            commands::store_install,
            commands::game_modes,
            commands::save_set_game_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Stabled");
}
