mod commands;
mod state;
mod watcher;

use commands::{accounts, content, launcher, library, system};
use novastrap_core::{roblox, web::Api};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                // %LOCALAPPDATA%\com.novastrap.app\logs, rotating at 2 MB.
                .max_file_size(2 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let paths = app.path();
            let http = roblox::deployment::http_client()?;
            let state = AppState::new(Api::new(http), paths.app_data_dir()?, paths.app_local_data_dir()?, paths.app_log_dir()?);
            app.manage(state);
            watcher::spawn(app.handle().clone());
            Ok(())
        })
        // Closing the main window quits Novastrap (including the HUD window).
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_external,
            launcher::roblox_status,
            launcher::install_roblox,
            launcher::launch_roblox,
            launcher::instance_profiles,
            launcher::save_instance_profile,
            launcher::delete_instance_profile,
            launcher::launch_instance,
            launcher::running_clients,
            launcher::close_client,
            launcher::performance_state,
            launcher::save_performance_profile,
            launcher::delete_performance_profile,
            launcher::set_active_performance_profile,
            launcher::apply_roblox_settings,
            launcher::diagnostics,
            accounts::accounts_list,
            accounts::account_add_profile,
            accounts::account_sign_in,
            accounts::account_switch,
            accounts::account_remove,
            accounts::avatar_render,
            library::discover_home,
            library::discover_search,
            library::game_details,
            library::server_list,
            library::favorites,
            library::toggle_favorite_game,
            library::toggle_favorite_server,
            library::home_data,
            library::clear_history,
            content::mod_slots,
            content::mod_choose_file,
            content::mod_revert,
            content::background_choose,
            content::background_clear,
            content::background_data,
            content::catalog_search,
            content::toggle_favorite_item,
            content::trade_values,
            system::app_info,
            system::get_settings,
            system::save_settings,
            system::live_activity,
            system::hud_toggle,
            system::storage_usage,
            system::storage_clear,
            system::app_log_tail,
            system::open_folder,
            system::backup_export,
            system::backup_import,
            system::check_app_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Novastrap");
}
