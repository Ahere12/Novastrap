//! Thin Tauri command handlers. Logic lives in the service modules.

use crate::AppState;
use novastrap_core::roblox::{self, RobloxStatus};
use serde::Serialize;

#[tauri::command]
pub async fn roblox_status(state: tauri::State<'_, AppState>) -> Result<RobloxStatus, ()> {
    Ok(roblox::status(&state.http).await)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo { name: "Novastrap", version: env!("CARGO_PKG_VERSION") }
}
