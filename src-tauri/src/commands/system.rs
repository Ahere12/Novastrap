//! Modules 10–12: HUD window, Discord, settings, storage, logs, backups, updates.

use super::installs;
use crate::state::{AppState, Live};
use novastrap_core::accounts::AccountsFile;
use novastrap_core::error::{AppError, AppResult};
use novastrap_core::favorites::FavoritesFile;
use novastrap_core::instances::{self, InstancesFile};
use novastrap_core::mods::ModsFile;
use novastrap_core::performance::ProfilesFile;
use novastrap_core::releases;
use novastrap_core::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

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

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.get()
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    settings.validate()?;
    let before = state.settings.get();
    // The background file is managed by its own commands.
    let next = Settings { appearance: novastrap_core::settings::Appearance { background: before.appearance.background.clone(), ..settings.appearance.clone() }, ..settings };
    state.settings.replace(next.clone())?;
    if next.shortcuts != before.shortcuts {
        crate::desktop::apply_shortcuts(&app, &next.shortcuts);
    }
    if !next.integrations.discord_enabled {
        if let Some(mut rpc) = state.discord.lock().unwrap().take() {
            rpc.clear();
        }
    }
    Ok(next)
}

// ---- HUD --------------------------------------------------------------------------

#[tauri::command]
pub fn live_activity(state: State<'_, AppState>) -> Live {
    state.live.lock().unwrap().clone()
}

/// Async on purpose: sync commands run on the main thread, and creating or closing a window
/// there deadlocks on Windows (found in QA: the HUD toggle hung the IPC call).
#[tauri::command]
pub async fn hud_toggle(app: tauri::AppHandle) -> AppResult<bool> {
    if let Some(w) = app.get_webview_window("hud") {
        w.close().map_err(|e| AppError::Launch(e.to_string()))?;
        return Ok(false);
    }
    WebviewWindowBuilder::new(&app, "hud", WebviewUrl::App("index.html".into()))
        .initialization_script("window.__NOVASTRAP_WINDOW__ = 'hud';")
        .title("Novastrap HUD")
        .inner_size(340.0, 420.0)
        .min_inner_size(280.0, 300.0)
        .always_on_top(true)
        .resizable(true)
        .background_color(tauri::window::Color(11, 12, 17, 255))
        .build()
        .map_err(|e| AppError::Launch(format!("couldn't open the HUD: {e}")))?;
    Ok(true)
}

// ---- storage ----------------------------------------------------------------------

fn dir_size(p: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(p) else { return 0 };
    entries
        .flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageItem {
    pub id: &'static str,
    pub label: String,
    pub bytes: u64,
    pub clearable: bool,
}

#[tauri::command]
pub async fn storage_usage(state: State<'_, AppState>) -> Result<Vec<StorageItem>, ()> {
    let list = installs();
    let current = list.first().map(|i| i.directory.clone());
    let old: Vec<_> = list.iter().skip(1).map(|i| i.directory.clone()).collect();
    let roblox_logs = novastrap_core::activity::logs_dir();
    let (downloads, mods, logs, data) = (state.downloads_dir(), state.mods_dir(), state.log_dir.clone(), state.data_dir.clone());
    Ok(tokio::task::spawn_blocking(move || {
        vec![
            StorageItem { id: "downloads", label: "Novastrap downloads".into(), bytes: dir_size(&downloads), clearable: true },
            StorageItem { id: "mods", label: "Mod files and originals backup".into(), bytes: dir_size(&mods), clearable: false },
            StorageItem { id: "data", label: "Settings, history and favorites".into(), bytes: dir_size(&data), clearable: false },
            StorageItem { id: "appLogs", label: "Novastrap logs".into(), bytes: dir_size(&logs), clearable: false },
            StorageItem { id: "robloxLogs", label: "Roblox client logs".into(), bytes: roblox_logs.as_deref().map(dir_size).unwrap_or(0), clearable: false },
            StorageItem { id: "current", label: "Current Roblox version".into(), bytes: current.as_deref().map(dir_size).unwrap_or(0), clearable: false },
            StorageItem {
                id: "oldVersions",
                label: format!("Older Roblox versions ({})", old.len()),
                bytes: old.iter().map(|d| dir_size(d)).sum(),
                clearable: !old.is_empty(),
            },
        ]
    })
    .await
    .unwrap_or_default())
}

#[tauri::command]
pub fn storage_clear(state: State<'_, AppState>, id: String) -> AppResult<u64> {
    match id.as_str() {
        "downloads" => {
            let d = state.downloads_dir();
            let n = dir_size(&d);
            if d.exists() {
                std::fs::remove_dir_all(&d)?;
            }
            state.api.clear_cache();
            Ok(n)
        }
        "oldVersions" => {
            if !instances::client_processes(&[]).is_empty() {
                return Err(AppError::Busy("Close Roblox before removing old versions".into()));
            }
            let list = installs();
            let mut freed = 0;
            // Keep the newest install; remove only other version-* folders that contain the player.
            for i in list.iter().skip(1) {
                if novastrap_core::roblox::detect::is_version_guid(&i.version_guid) {
                    freed += dir_size(&i.directory);
                    std::fs::remove_dir_all(&i.directory)?;
                }
            }
            Ok(freed)
        }
        _ => Err(AppError::InvalidInput("That item can't be cleared".into())),
    }
}

// ---- logs -------------------------------------------------------------------------

#[tauri::command]
pub fn app_log_tail(state: State<'_, AppState>) -> Vec<String> {
    let newest = std::fs::read_dir(&state.log_dir)
        .ok()
        .and_then(|r| r.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "log")).max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok()));
    let text = newest.and_then(|e| std::fs::read_to_string(e.path()).ok()).unwrap_or_default();
    let lines: Vec<String> = text.lines().map(str::to_owned).collect();
    lines[lines.len().saturating_sub(300)..].to_vec()
}

#[tauri::command]
pub fn open_folder(app: tauri::AppHandle, state: State<'_, AppState>, which: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let path = match which.as_str() {
        "logs" => state.log_dir.clone(),
        "data" => state.data_dir.clone(),
        "robloxLogs" => novastrap_core::activity::logs_dir().ok_or_else(|| AppError::Unsupported("no Roblox folder".into()))?,
        _ => return Err(AppError::InvalidInput("unknown folder".into())),
    };
    std::fs::create_dir_all(&path)?;
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| AppError::Launch(e.to_string()))
}

// ---- backups (no secrets: tokens stay in Credential Manager) ----------------------

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub format: String,
    pub app_version: String,
    pub settings: Settings,
    pub accounts: AccountsFile,
    pub instances: InstancesFile,
    pub favorites: FavoritesFile,
    pub performance: ProfilesFile,
    pub mods: ModsFile,
}

const BACKUP_FORMAT: &str = "novastrap-backup-1";

#[tauri::command]
pub async fn backup_export(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    let b = Backup {
        format: BACKUP_FORMAT.into(),
        app_version: env!("CARGO_PKG_VERSION").into(),
        settings: state.settings.get(),
        accounts: state.accounts.get(),
        instances: state.instances.get(),
        favorites: state.favorites.get(),
        performance: state.profiles.get(),
        mods: state.mods.get(),
    };
    let app2 = app.clone();
    let target = tokio::task::spawn_blocking(move || {
        app2.dialog().file().add_filter("Novastrap backup", &["json"]).set_file_name("novastrap-backup.json").blocking_save_file()
    })
    .await
    .map_err(|e| AppError::Launch(e.to_string()))?;
    let Some(path) = target.and_then(|p| p.into_path().ok()) else { return Ok(None) };
    novastrap_core::store::write_atomic(&path, &serde_json::to_vec_pretty(&b).expect("serializable"))?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub async fn backup_import(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<bool> {
    let app2 = app.clone();
    let picked = tokio::task::spawn_blocking(move || app2.dialog().file().add_filter("Novastrap backup", &["json"]).blocking_pick_file())
        .await
        .map_err(|e| AppError::Launch(e.to_string()))?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(false) };
    if std::fs::metadata(&path)?.len() > 5 * 1024 * 1024 {
        return Err(AppError::InvalidInput("That file is too large to be a Novastrap backup".into()));
    }
    let b: Backup = serde_json::from_slice(&std::fs::read(&path)?).map_err(|e| AppError::InvalidInput(format!("Not a Novastrap backup: {e}")))?;
    if b.format != BACKUP_FORMAT {
        return Err(AppError::InvalidInput("Unsupported backup format".into()));
    }
    b.settings.validate()?;
    // Re-validate profiles; drop anything invalid.
    let mut perf = b.performance;
    perf.profiles = perf.profiles.into_iter().filter_map(|p| p.sanitize().ok().map(|(p, _)| p)).collect();
    let mut inst = b.instances;
    inst.profiles.retain(|p| p.validate().is_ok());
    // Imported accounts are profiles only until signed in on this PC.
    let mut accounts = b.accounts;
    for a in &mut accounts.accounts {
        a.linked = novastrap_core::accounts::load_tokens(a.user_id).is_some();
    }
    // Mods reference files that aren't in the backup; keep only ones present here.
    let mut mods = b.mods;
    let dir = state.mods_dir();
    mods.enabled.retain(|_, f| !f.contains(['/', '\\']) && dir.join(f).exists());
    let mut settings = b.settings;
    settings.appearance.background = state.settings.get().appearance.background;
    state.settings.replace(settings)?;
    state.accounts.replace(accounts)?;
    state.instances.replace(inst)?;
    state.favorites.replace(b.favorites)?;
    state.profiles.replace(perf)?;
    state.mods.replace(mods)?;
    Ok(true)
}

// ---- app updates (GitHub releases of this repository) -------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: &'static str,
    pub channel: String,
    pub latest: Option<String>,
    pub url: Option<String>,
    pub update_available: bool,
}

async fn channel_releases(state: &AppState) -> AppResult<Vec<releases::Release>> {
    let all: Vec<releases::Release> = match state.api.get_json(releases::RELEASES_URL, novastrap_core::web::ttl::MEDIUM).await {
        Ok(v) => v,
        Err(AppError::HttpStatus(404)) => Vec::new(),
        Err(e) => return Err(e),
    };
    Ok(releases::for_channel(all, &state.settings.get().updates.channel))
}

#[tauri::command]
pub async fn check_app_update(state: State<'_, AppState>) -> AppResult<UpdateInfo> {
    let current = env!("CARGO_PKG_VERSION");
    let list = channel_releases(&state).await?;
    let channel = state.settings.get().updates.channel;
    let latest = list.first();
    Ok(UpdateInfo {
        current,
        channel,
        update_available: releases::update_for(&list, current).is_some(),
        latest: latest.map(|r| r.tag_name.clone()),
        url: latest.map(|r| r.html_url.clone()),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleasesView {
    /// Published releases on the chosen channel (changelog), newest first.
    pub releases: Vec<releases::Release>,
    /// Versions this PC has run (oldest first).
    pub history: Vec<releases::VersionSeen>,
}

#[tauri::command]
pub async fn app_releases(state: State<'_, AppState>) -> AppResult<ReleasesView> {
    Ok(ReleasesView { releases: channel_releases(&state).await?, history: state.update_history.get().versions })
}

// ---- notification center ----------------------------------------------------------

#[tauri::command]
pub fn notices_list(state: State<'_, AppState>) -> novastrap_core::notices::NoticesFile {
    state.notices.get()
}

#[tauri::command]
pub fn notices_mark_read(state: State<'_, AppState>) -> AppResult<()> {
    state.notices.update(|f| f.mark_all_read())
}

#[tauri::command]
pub fn notices_clear(state: State<'_, AppState>) -> AppResult<()> {
    state.notices.update(|f| f.items.clear())
}

#[tauri::command]
pub fn shortcut_problems(state: State<'_, AppState>) -> Vec<String> {
    state.shortcut_problems.lock().unwrap().clone()
}
