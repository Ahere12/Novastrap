//! Modules 7 (mods/customization), 8 (catalog) and 9 (trading).

use super::{installs, now};
use crate::state::AppState;
use base64::Engine;
use novastrap_core::catalog::{self, CatalogPage, Category, SortBy};
use novastrap_core::error::{AppError, AppResult};
use novastrap_core::favorites::FavItem;
use novastrap_core::mods::{self, Slot};
use novastrap_core::trading::{self, ValueTable};
use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

fn pick_file(app: &tauri::AppHandle, name: &str, exts: &[&str]) -> Option<std::path::PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

// ---- client mods ----------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModSlotView {
    pub slot: &'static Slot,
    pub enabled: bool,
    pub compatible: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn mod_slots(state: State<'_, AppState>) -> Vec<ModSlotView> {
    let file = state.mods.get();
    let dir = installs().first().map(|i| i.directory.clone());
    mods::SLOTS
        .iter()
        .map(|s| {
            let c = dir.as_ref().map(|d| mods::check(d, s.id));
            ModSlotView {
                slot: s,
                enabled: file.enabled.contains_key(s.id),
                compatible: c.as_ref().is_some_and(|c| c.ok),
                reason: match c {
                    Some(c) => c.reason,
                    None => Some("Roblox isn't installed".into()),
                },
            }
        })
        .collect()
}

/// Picks a replacement file, validates it, enables the slot and applies it to the current install.
#[tauri::command]
pub async fn mod_choose_file(app: tauri::AppHandle, state: State<'_, AppState>, slot_id: String) -> AppResult<bool> {
    let slot = mods::slot(&slot_id)?;
    let ext = slot.format.ext();
    let app2 = app.clone();
    let picked = tokio::task::spawn_blocking(move || pick_file(&app2, "Replacement file", &[ext]))
        .await
        .map_err(|e| AppError::Launch(e.to_string()))?;
    let Some(path) = picked else { return Ok(false) };
    let stored = mods::import(&state.mods_dir(), &slot_id, &path)?;
    state.mods.update(|f| {
        f.enabled.insert(slot_id.clone(), stored);
    })?;
    if let Some(i) = installs().first() {
        mods::apply_all(&state.mods_dir(), &i.directory, &state.mods.get())?;
    }
    Ok(true)
}

/// Disables a mod and restores Roblox's original file in every installed version.
#[tauri::command]
pub fn mod_revert(state: State<'_, AppState>, slot_id: String) -> AppResult<()> {
    mods::slot(&slot_id)?;
    for i in installs() {
        mods::revert(&state.mods_dir(), &i.directory, &slot_id)?;
    }
    state.mods.update(|f| {
        f.enabled.remove(&slot_id);
    })
}

// ---- launcher appearance: background image ------------------------------------------

const MAX_BG: u64 = 8 * 1024 * 1024;

#[tauri::command]
pub async fn background_choose(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<Option<String>> {
    let app2 = app.clone();
    let picked = tokio::task::spawn_blocking(move || pick_file(&app2, "Image", &["png", "jpg", "jpeg", "webp"]))
        .await
        .map_err(|e| AppError::Launch(e.to_string()))?;
    let Some(src) = picked else { return Ok(None) };
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if std::fs::metadata(&src)?.len() > MAX_BG {
        return Err(AppError::InvalidInput("Pick an image under 8 MB".into()));
    }
    let bytes = std::fs::read(&src)?;
    let valid = bytes.starts_with(b"\x89PNG") || bytes.starts_with(&[0xFF, 0xD8]) || (bytes.len() > 12 && &bytes[8..12] == b"WEBP");
    if !valid {
        return Err(AppError::InvalidInput("That file isn't a PNG, JPEG or WebP image".into()));
    }
    let dir = state.backgrounds_dir();
    std::fs::create_dir_all(&dir)?;
    let name = format!("background.{ext}");
    for old in std::fs::read_dir(&dir)?.flatten() {
        let _ = std::fs::remove_file(old.path());
    }
    std::fs::write(dir.join(&name), bytes)?;
    state.settings.update(|s| s.appearance.background = Some(name.clone()))?;
    Ok(Some(name))
}

#[tauri::command]
pub fn background_clear(state: State<'_, AppState>) -> AppResult<()> {
    if let Ok(entries) = std::fs::read_dir(state.backgrounds_dir()) {
        for e in entries.flatten() {
            let _ = std::fs::remove_file(e.path());
        }
    }
    state.settings.update(|s| s.appearance.background = None)
}

/// The background as a data URL (the webview can't read local files directly).
#[tauri::command]
pub fn background_data(state: State<'_, AppState>) -> Option<String> {
    let name = state.settings.get().appearance.background?;
    if name.contains(['/', '\\']) || name.contains("..") {
        return None;
    }
    let bytes = std::fs::read(state.backgrounds_dir().join(&name)).ok()?;
    let mime = if name.ends_with(".png") { "image/png" } else if name.ends_with(".webp") { "image/webp" } else { "image/jpeg" };
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

// ---- catalog --------------------------------------------------------------------------

#[tauri::command]
pub async fn catalog_search(state: State<'_, AppState>, keyword: String, category: Category, sort: SortBy, cursor: Option<String>) -> AppResult<CatalogPage> {
    catalog::search(&state.api, &keyword, category, sort, cursor.as_deref()).await
}

#[tauri::command]
pub fn toggle_favorite_item(state: State<'_, AppState>, mut item: FavItem) -> AppResult<bool> {
    item.added_at = now();
    state.favorites.update(|f| f.toggle_item(item))
}

// ---- trading --------------------------------------------------------------------------

#[tauri::command]
pub async fn trade_values(state: State<'_, AppState>) -> AppResult<ValueTable> {
    trading::values(&state.api).await
}
