mod commands;

use novastrap_core::{reqwest, roblox};

/// Shared backend state, managed by Tauri.
pub struct AppState {
    pub http: reqwest::Client,
    /// Set while an install/update runs, so only one can run at a time.
    pub installing: std::sync::atomic::AtomicBool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let http = roblox::deployment::http_client().expect("failed to build HTTP client");

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                // Writes to %LOCALAPPDATA%\com.novastrap.app\logs, rotating at 2 MB.
                .max_file_size(2 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { http, installing: Default::default() })
        .invoke_handler(tauri::generate_handler![
            commands::roblox_status,
            commands::install_roblox,
            commands::launch_roblox,
            commands::app_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running Novastrap");
}
