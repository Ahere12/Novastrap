//! Module 1 (install/launch), 3 (instances) and 6 (performance) commands.

use super::{installs, now};
use crate::state::AppState;
use novastrap_core::error::{AppError, AppResult};
use novastrap_core::instances::{self, ClientProcess, InstanceProfile, RunningInstance};
use novastrap_core::performance::{self, FlagSpec, PerformanceProfile, RobloxSettings};
use novastrap_core::roblox::install::InstallProgress;
use novastrap_core::roblox::launch::{self, LaunchTarget};
use novastrap_core::roblox::{self, RobloxStatus, deployment, detect};
use novastrap_core::mods;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager, State};

#[tauri::command]
pub async fn roblox_status(state: State<'_, AppState>) -> Result<RobloxStatus, ()> {
    Ok(roblox::status(state.api.client()).await)
}

struct BusyGuard<'a>(&'a std::sync::atomic::AtomicBool);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

#[tauri::command]
pub async fn install_roblox(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err(AppError::Busy("an install is already running".into()));
    }
    let _guard = BusyGuard(&state.installing);
    let latest = deployment::latest(state.api.client(), None).await?;
    log::info!("installing Roblox {}", latest.client_version_upload);
    let emitter = app.clone();
    let mut last_pct = u64::MAX;
    roblox::install::install(state.api.client(), &latest.client_version_upload, &state.downloads_dir(), move |p| {
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
    /// Non-fatal problems while preparing (e.g. settings couldn't be applied).
    pub warnings: Vec<String>,
}

fn all_profiles(state: &AppState) -> Vec<PerformanceProfile> {
    let mut v = performance::presets();
    v.extend(state.profiles.get().profiles);
    v
}

/// Applies the performance profile and mods to the chosen version, then starts the client.
pub(crate) async fn prepare_and_launch(
    app: &tauri::AppHandle,
    state: &AppState,
    target: LaunchTarget,
    profile_name: Option<String>,
    instance: Option<(String, String)>,
    safe: bool,
) -> AppResult<Launched> {
    let list = installs();
    let handler = detect::classify_handler(detect::read_protocol_command().as_deref());
    let latest = deployment::latest(state.api.client(), None).await.ok();
    let exe = launch::choose_executable(&list, latest.as_ref().map(|v| v.client_version_upload.as_str()), &handler)
        .ok_or_else(|| AppError::Launch("Roblox isn't installed yet".into()))?
        .to_path_buf();
    let version_dir = exe.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let mut warnings = Vec::new();

    if safe {
        // Safe mode: no FastFlags and Roblox's original files for this launch. The next normal
        // launch writes the active profile's flags and re-applies mods as usual.
        if let Err(e) = performance::write_flags(&version_dir, &PerformanceProfile::default()) {
            warnings.push(format!("Couldn't clear FastFlags: {e}"));
        }
        for slot in state.mods.get().enabled.keys() {
            if let Err(e) = mods::revert(&state.mods_dir(), &version_dir, slot) {
                warnings.push(format!("Couldn't restore the original file for {slot}: {e}"));
            }
        }
    }
    let wanted = if safe { None } else { profile_name.or(state.profiles.get().active) };
    if let Some(name) = wanted {
        match all_profiles(state).into_iter().find(|p| p.name == name) {
            Some(p) => {
                if let Err(e) = performance::write_flags(&version_dir, &p) {
                    warnings.push(format!("FastFlags weren't applied: {e}"));
                }
                let running = !instances::client_processes(&[]).is_empty();
                if (p.framerate_cap.is_some() || p.graphics_quality.is_some()) && !running {
                    if let Err(e) = performance::write_roblox_settings(p.framerate_cap, p.graphics_quality, false) {
                        warnings.push(format!("Frame rate / graphics weren't applied: {e}"));
                    }
                } else if p.framerate_cap.is_some() || p.graphics_quality.is_some() {
                    warnings.push("Frame rate / graphics settings apply the next time no Roblox client is running".into());
                }
            }
            None => warnings.push(format!("Performance profile '{name}' no longer exists")),
        }
    }
    if !safe {
        if let Err(e) = mods::apply_all(&state.mods_dir(), &version_dir, &state.mods.get()) {
            warnings.push(format!("Mods weren't applied: {e}"));
        }
    }

    let pid = launch::spawn(&exe, &target)?;
    let (profile_id, name) = instance.map(|(i, n)| (Some(i), n)).unwrap_or((None, if safe { "Safe mode" } else { "Quick launch" }.into()));
    state.launched.lock().unwrap().push(RunningInstance { pid, profile_id, name, started_at: now() });
    if let Some(id) = state.settings.get().accounts.active_user_id {
        let _ = state.accounts.update(|a| a.touch(id, now()));
    }
    if state.settings.get().general.minimize_on_launch {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.minimize();
        }
    }
    Ok(Launched { pid, target, warnings })
}

/// Launches the app home (empty input), or joins a place ID / link / server.
#[tauri::command]
pub async fn launch_roblox(app: tauri::AppHandle, state: State<'_, AppState>, input: String, job_id: Option<String>, safe: Option<bool>) -> AppResult<Launched> {
    let target = match (LaunchTarget::parse_user_input(&input)?, job_id) {
        (LaunchTarget::Place { place_id, .. }, Some(j)) => LaunchTarget::place(place_id, Some(j))?,
        (t, _) => t,
    };
    let safe = safe.unwrap_or(false);
    let r = prepare_and_launch(&app, &state, target, None, None, safe).await;
    if safe {
        let body = match &r { Ok(_) => "Roblox started without FastFlags or mods.".to_string(), Err(e) => e.to_string() };
        crate::desktop::notice(&app, novastrap_core::notices::NoticeKind::Launch, "Safe-mode launch", &body);
    }
    r
}

/// What the tray menu and the Play shortcut launch: the Play bar's instance, or the Roblox app.
pub(crate) async fn play_default(app: &tauri::AppHandle) -> AppResult<Launched> {
    let state = app.state::<AppState>();
    let chosen = state.settings.get().instances.default_profile;
    if let Some(p) = chosen.and_then(|id| state.instances.get().profiles.into_iter().find(|p| p.id == id)) {
        let target = p.validate()?;
        return prepare_and_launch(app, &state, target, p.performance_profile.clone(), Some((p.id, p.name)), false).await;
    }
    prepare_and_launch(app, &state, LaunchTarget::App, None, None, false).await
}

// ---- instances ----------------------------------------------------------------

#[tauri::command]
pub fn instance_profiles(state: State<'_, AppState>) -> Vec<InstanceProfile> {
    state.instances.get().profiles
}

#[tauri::command]
pub fn save_instance_profile(state: State<'_, AppState>, mut profile: InstanceProfile) -> AppResult<Vec<InstanceProfile>> {
    profile.validate()?;
    profile.name = profile.name.trim().to_owned();
    if let Some(p) = &profile.performance_profile {
        if !all_profiles(&state).iter().any(|x| &x.name == p) {
            return Err(AppError::InvalidInput(format!("Performance profile '{p}' doesn't exist")));
        }
    }
    state.instances.update(|f| {
        if profile.id.is_empty() {
            profile.id = instances::new_profile_id(now(), &f.profiles);
        }
        match f.profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(p) => *p = profile,
            None => f.profiles.push(profile),
        }
        f.profiles.clone()
    })
}

#[tauri::command]
pub fn delete_instance_profile(state: State<'_, AppState>, id: String) -> AppResult<Vec<InstanceProfile>> {
    state.instances.update(|f| {
        f.profiles.retain(|p| p.id != id);
        f.profiles.clone()
    })
}

#[tauri::command]
pub async fn launch_instance(app: tauri::AppHandle, state: State<'_, AppState>, id: String) -> AppResult<Launched> {
    let p = state.instances.get().profiles.into_iter().find(|p| p.id == id).ok_or_else(|| AppError::InvalidInput("That instance profile no longer exists".into()))?;
    let target = p.validate()?;
    prepare_and_launch(&app, &state, target, p.performance_profile.clone(), Some((p.id, p.name)), false).await
}

#[tauri::command]
pub fn running_clients(state: State<'_, AppState>) -> Vec<ClientProcess> {
    let mut launched = state.launched.lock().unwrap();
    let running = instances::client_processes(&launched);
    instances::prune_exited(&mut launched, &running);
    running
}

#[tauri::command]
pub fn close_client(pid: u32) -> AppResult<()> {
    instances::close_client(pid)
}

// ---- performance --------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceState {
    pub allowlist: &'static [FlagSpec],
    pub presets: Vec<PerformanceProfile>,
    pub profiles: Vec<PerformanceProfile>,
    pub active: Option<String>,
    pub roblox: RobloxSettings,
    pub framerate_caps: &'static [u32],
}

#[tauri::command]
pub fn performance_state(state: State<'_, AppState>) -> PerformanceState {
    let f = state.profiles.get();
    PerformanceState {
        allowlist: performance::ALLOWLIST,
        presets: performance::presets(),
        profiles: f.profiles,
        active: f.active,
        roblox: performance::read_roblox_settings(),
        framerate_caps: performance::FRAMERATE_CAPS,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedProfile {
    pub profile: PerformanceProfile,
    /// Flags removed because they aren't on Roblox's allowlist.
    pub dropped: Vec<String>,
}

#[tauri::command]
pub fn save_performance_profile(state: State<'_, AppState>, profile: PerformanceProfile, original_name: Option<String>) -> AppResult<SavedProfile> {
    let (clean, dropped) = profile.sanitize()?;
    if performance::presets().iter().any(|p| p.name == clean.name) {
        return Err(AppError::InvalidInput("That name belongs to a built-in preset".into()));
    }
    let saved = clean.clone();
    state.profiles.update(|f| {
        let key = original_name.unwrap_or_else(|| clean.name.clone());
        f.profiles.retain(|p| p.name != key && p.name != clean.name);
        if f.active.as_deref() == Some(key.as_str()) {
            f.active = Some(clean.name.clone());
        }
        f.profiles.push(clean);
    })?;
    Ok(SavedProfile { profile: saved, dropped })
}

#[tauri::command]
pub fn delete_performance_profile(state: State<'_, AppState>, name: String) -> AppResult<()> {
    state.profiles.update(|f| {
        f.profiles.retain(|p| p.name != name);
        if f.active.as_deref() == Some(name.as_str()) {
            f.active = None;
        }
    })
}

/// Sets the profile applied at every launch. Also writes it to the current install now.
#[tauri::command]
pub fn set_active_performance_profile(state: State<'_, AppState>, name: Option<String>) -> AppResult<Vec<String>> {
    let mut warnings = Vec::new();
    if let Some(n) = &name {
        let p = all_profiles(&state).into_iter().find(|p| &p.name == n).ok_or_else(|| AppError::InvalidInput("Unknown profile".into()))?;
        if let Some(i) = installs().first() {
            performance::write_flags(&i.directory, &p)?;
        }
        let running = !instances::client_processes(&[]).is_empty();
        if let Err(e) = performance::write_roblox_settings(p.framerate_cap, p.graphics_quality, running) {
            warnings.push(e.to_string());
        }
    } else if let Some(i) = installs().first() {
        performance::write_flags(&i.directory, &performance::presets()[0])?;
    }
    state.profiles.update(|f| f.active = name)?;
    Ok(warnings)
}

#[tauri::command]
pub fn apply_roblox_settings(framerate_cap: Option<u32>, graphics_quality: Option<u8>) -> AppResult<RobloxSettings> {
    let running = !instances::client_processes(&[]).is_empty();
    performance::write_roblox_settings(framerate_cap, graphics_quality, running)?;
    Ok(performance::read_roblox_settings())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub status: RobloxStatus,
    pub clients: Vec<ClientProcess>,
    pub flags_file: Option<String>,
    pub flags_written: usize,
    pub roblox_settings: RobloxSettings,
    pub recent_errors: Vec<String>,
    pub mods: Vec<mods::Compatibility>,
    /// Signature check of the installed RobloxPlayerBeta.exe (Ok = signed by Roblox Corporation).
    pub executable_signature: Option<Result<(), String>>,
}

#[tauri::command]
pub async fn diagnostics(state: State<'_, AppState>) -> Result<Diagnostics, ()> {
    let status = roblox::status(state.api.client()).await;
    let dir = status.installations.first().map(|i| i.directory.clone());
    let exe = status.installations.first().map(|i| i.executable.clone());
    let executable_signature = match exe {
        Some(p) => tokio::task::spawn_blocking(move || novastrap_core::roblox::install::verify_signature(&p).map_err(|e| e.to_string())).await.ok(),
        None => None,
    };
    let recent_errors = novastrap_core::activity::logs_dir()
        .and_then(|d| novastrap_core::activity::newest_player_log(&d))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|text| {
            let mut errs: Vec<String> = text
                .lines()
                .filter(|l| l.contains("[FLog::Error]") || l.contains("[DFLog::HttpTraceError]") || l.contains("Crash"))
                .map(|l| l.chars().take(240).collect())
                .collect();
            let n = errs.len();
            errs.split_off(n.saturating_sub(15))
        })
        .unwrap_or_default();
    Ok(Diagnostics {
        clients: instances::client_processes(&state.launched.lock().unwrap()),
        flags_file: dir.as_ref().map(|d| performance::client_settings_path(d).display().to_string()),
        flags_written: dir.as_ref().map(|d| performance::read_flags(d).len()).unwrap_or(0),
        roblox_settings: performance::read_roblox_settings(),
        mods: dir
            .as_ref()
            .map(|d| state.mods.get().enabled.keys().map(|k| mods::check(d, k)).collect())
            .unwrap_or_default(),
        recent_errors,
        executable_signature,
        status,
    })
}
