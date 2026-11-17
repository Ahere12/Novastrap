//! Thin Tauri command handlers. Logic lives in `novastrap-core`.

use crate::AppState;
use novastrap_core::error::{AppError, AppResult};
use novastrap_core::roblox::install::InstallProgress;
use novastrap_core::roblox::launch::{self, LaunchTarget};
use novastrap_core::roblox::{self, RobloxStatus, deployment, detect};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn roblox_status(state: tauri::State<'_, AppState>) -> Result<RobloxStatus, ()> {
    Ok(roblox::status(&state.http).await)
}

/// Releases the install lock even if the install future errors or panics.
struct BusyGuard<'a>(&'a std::sync::atomic::AtomicBool);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Installs or updates Roblox with the official, signature-verified installer.
/// Progress is emitted as `install-progress` events.
#[tauri::command]
pub async fn install_roblox(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> AppResult<()> {
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err(AppError::Busy("an install is already running".into()));
    }
    let _guard = BusyGuard(&state.installing);

    let latest = deployment::latest(&state.http, None).await?;
    let downloads = app
        .path()
        .app_local_data_dir()
        .map_err(|e| AppError::Unsupported(format!("no app data folder: {e}")))?
        .join("downloads");

    log::info!("installing Roblox {}", latest.client_version_upload);
    let emitter = app.clone();
    let mut last_pct = u64::MAX;
    roblox::install::install(&state.http, &latest.client_version_upload, &downloads, move |p| {
        // Throttle download events to whole-percent changes.
        if let InstallProgress::Downloading { downloaded, total: Some(t) } = &p {
            let pct = downloaded * 100 / (*t).max(1);
            if pct == last_pct {
                return;
            }
            last_pct = pct;
        }
        let _ = emitter.emit("install-progress", &p);
    })
    .await
    .inspect_err(|e| log::error!("install failed: {e}"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Launched {
    pub pid: u32,
    pub target: LaunchTarget,
}

/// Launches the Roblox app, or joins the experience described by `input`
/// (a place ID or roblox.com link). Empty input opens the app.
#[tauri::command]
pub async fn launch_roblox(state: tauri::State<'_, AppState>, input: String) -> AppResult<Launched> {
    let target = LaunchTarget::parse_user_input(&input)?;
    let installs: Vec<_> =
        detect::official_versions_dirs().iter().flat_map(|d| detect::scan_versions_dir(d)).collect();
    let handler = detect::classify_handler(detect::read_protocol_command().as_deref());
    // The version lookup is only a preference; launching still works offline.
    let latest = deployment::latest(&state.http, None).await.ok();
    let exe = launch::choose_executable(&installs, latest.as_ref().map(|v| v.client_version_upload.as_str()), &handler)
        .ok_or_else(|| AppError::Launch("Roblox isn't installed yet".into()))?
        .to_path_buf();
    let pid = launch::spawn(&exe, &target)?;
    Ok(Launched { pid, target })
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
