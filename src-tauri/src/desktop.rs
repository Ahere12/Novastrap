//! Desktop integration: system tray, global shortcuts, and the notification center log.

use crate::commands::{launcher, now, system};
use crate::state::AppState;
use novastrap_core::notices::NoticeKind;
use novastrap_core::settings::Shortcuts;
use novastrap_core::shortcut;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Records a notice in the notification center and tells open windows.
pub fn notice(app: &AppHandle, kind: NoticeKind, title: &str, body: &str) {
    let state = app.state::<AppState>();
    let mut added = None;
    if state.notices.update(|f| added = Some(f.push(now(), kind, title, body))).is_ok() {
        let _ = app.emit("notice", added);
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn play(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match launcher::play_default(&app).await {
            Ok(r) => {
                for w in &r.warnings {
                    notice(&app, NoticeKind::Launch, "Launch warning", w);
                }
            }
            Err(e) => notice(&app, NoticeKind::Error, "Couldn't start Roblox", &e.to_string()),
        }
    });
}

/// Shortcut and tray handlers run on the event loop thread; creating a window there can
/// deadlock on Windows, so the HUD is opened from an async task instead.
fn toggle_hud(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = system::hud_toggle(app.clone()) {
            notice(&app, NoticeKind::Error, "Couldn't open the HUD", &e.to_string());
        }
    });
}

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open Novastrap", true, None::<&str>)?;
    let play = MenuItem::with_id(app, "play", "Play Roblox", true, None::<&str>)?;
    let hud = MenuItem::with_id(app, "hud", "Second-screen HUD", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Novastrap", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &play, &hud, &sep, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main").tooltip("Novastrap").menu(&menu).show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.on_menu_event(|app, e| match e.id.as_ref() {
        "show" => show_main(app),
        "play" => self::play(app),
        "hud" => toggle_hud(app),
        "quit" => app.exit(0),
        _ => {}
    })
    .on_tray_icon_event(|tray, e| {
        if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
            show_main(tray.app_handle());
        }
    })
    .build(app)?;
    Ok(())
}

/// Shortcut actions in the same order as `Shortcuts::all()`.
const ACTIONS: [&str; 3] = ["show", "hud", "play"];

/// (Re)registers global shortcuts from settings. Returns problems to show in Settings.
pub fn apply_shortcuts(app: &AppHandle, cfg: &Shortcuts) -> Vec<String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let mut problems = Vec::new();
    if cfg.enabled {
        for (raw, action) in cfg.all().into_iter().zip(ACTIONS) {
            if raw.trim().is_empty() {
                continue;
            }
            let parsed = shortcut::normalize(raw).and_then(|n| {
                n.parse::<Shortcut>().map_err(|e| novastrap_core::error::AppError::InvalidInput(format!("{n}: {e}")))
            });
            match parsed {
                Ok(sc) => {
                    let result = gs.on_shortcut(sc, move |app, _, ev| {
                        if ev.state == ShortcutState::Pressed {
                            log::info!("global shortcut: {action}");
                            match action {
                                "show" => show_main(app),
                                "hud" => toggle_hud(app),
                                _ => play(app),
                            }
                        }
                    });
                    if let Err(e) = result {
                        problems.push(format!("{raw}: couldn't register (another app may already use it): {e}"));
                    }
                }
                Err(e) => problems.push(e.to_string()),
            }
        }
    }
    *app.state::<AppState>().shortcut_problems.lock().unwrap() = problems.clone();
    problems
}
