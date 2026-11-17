//! Thin Tauri command handlers grouped by module. Logic lives in `novastrap-core`.

pub mod accounts;
pub mod content;
pub mod launcher;
pub mod library;
pub mod system;

use novastrap_core::error::{AppError, AppResult};
use novastrap_core::roblox::detect::{self, Installation};

pub fn now() -> u64 {
    novastrap_core::accounts::oauth::now_secs()
}

pub fn installs() -> Vec<Installation> {
    detect::official_versions_dirs().iter().flat_map(|d| detect::scan_versions_dir(d)).collect()
}

#[tauri::command]
pub fn open_external(app: tauri::AppHandle, url: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    novastrap_core::links::check_external(&url)?;
    app.opener().open_url(url, None::<&str>).map_err(|e| AppError::Launch(e.to_string()))
}
