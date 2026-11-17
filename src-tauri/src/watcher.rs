//! Background activity watcher: tails the Roblox client log, tracks sessions, records
//! history, updates Discord presence and emits `activity` events (Home, HUD).

use crate::state::AppState;
use novastrap_core::activity::{self, LogTail, Tracker};
use novastrap_core::{discord, discover, instances};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub fn spawn(app: AppHandle) {
    let startup = app.clone();
    tauri::async_runtime::spawn(async move {
        // One check at startup for the "update available" notification.
        let state = startup.state::<AppState>();
        if state.settings.get().notifications.update_available {
            let st = novastrap_core::roblox::status(state.api.client()).await;
            if matches!(st.update, novastrap_core::roblox::UpdateState::UpdateAvailable) {
                let _ = startup.notification().builder().title("Roblox update available").body("Open Novastrap's Home page to update.").show();
                crate::desktop::notice(&startup, novastrap_core::notices::NoticeKind::Update, "Roblox update available", "Use Update now on the Play screen.");
            }
        }
    });
    tauri::async_runtime::spawn(async move {
        let mut tail = LogTail::default();
        let mut tracker = Tracker::default();
        let mut first = true;
        loop {
            tokio_sleep(1500).await;
            let state = app.state::<AppState>();
            let settings = state.settings.get();
            if !settings.general.track_activity {
                if tracker.current.is_some() {
                    finish(&app, &mut tracker);
                }
                first = true;
                continue;
            }
            let running = !instances::client_processes(&[]).is_empty();
            if let Some(log) = activity::logs_dir().and_then(|d| activity::newest_player_log(&d)) {
                // On first start, only replay the current log if Roblox is running now.
                tail.follow(log, first && !running);
            }
            first = false;
            let mut last_ts = activity_now();
            for line in tail.read_lines() {
                let ts = activity::parse_timestamp(&line).unwrap_or(last_ts);
                last_ts = ts;
                if let Some(ev) = activity::parse_line(&line) {
                    let joining = matches!(ev, activity::LogEvent::Joining { .. });
                    if let Some(ended) = tracker.feed(ev, ts) {
                        record(&state, ended);
                    }
                    if joining {
                        on_join(&app, &mut tracker).await;
                    } else {
                        publish(&app, &tracker);
                    }
                }
            }
            if !running && tracker.current.is_some() {
                finish(&app, &mut tracker);
            }
            // Fill in the universe/name once the join report arrives.
            if let Some(s) = tracker.current.as_ref() {
                if s.game_name.is_none() && s.universe_id.is_some() {
                    on_join(&app, &mut tracker).await;
                }
            }
        }
    });
}

async fn tokio_sleep(ms: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
}

fn activity_now() -> u64 {
    novastrap_core::accounts::oauth::now_secs()
}

fn record(state: &AppState, s: activity::Session) {
    if let Err(e) = state.history.update(|h| h.record(s)) {
        log::error!("couldn't save play history: {e}");
    }
}

fn finish(app: &AppHandle, tracker: &mut Tracker) {
    let state = app.state::<AppState>();
    if let Some(s) = tracker.end(activity_now()) {
        record(&state, s);
    }
    if let Some(rpc) = state.discord.lock().unwrap().as_mut() {
        rpc.clear();
    }
    *state.live.lock().unwrap() = Default::default();
    let _ = app.emit("activity", &*state.live.lock().unwrap());
}

/// Resolves the game name/icon for the current session, notifies, and publishes.
async fn on_join(app: &AppHandle, tracker: &mut Tracker) {
    let state = app.state::<AppState>();
    let mut icon = None;
    if let Some(s) = tracker.current.as_mut() {
        let universe = match s.universe_id {
            Some(u) => Some(u),
            None => discover::universe_for_place(&state.api, s.place_id).await.ok(),
        };
        if let Some(u) = universe {
            s.universe_id = Some(u);
            if let Ok(d) = discover::details(&state.api, u).await {
                s.game_name = Some(d.name);
                icon = d.icon_url;
            }
        }
        // on_join runs again while the name is still unknown, and the log is replayed after a
        // restart; notify only the first time this server is seen.
        let game = s.game_name.clone().unwrap_or_else(|| format!("Place {}", s.place_id));
        let first = crate::desktop::notice_once(app, &format!("game:{}", s.job_id), novastrap_core::notices::NoticeKind::Game, "Joined a game", &game);
        if first && state.settings.get().notifications.game_joined {
            let _ = app
                .notification()
                .builder()
                .title("Joined a game")
                .body(s.game_name.clone().unwrap_or_else(|| format!("Place {}", s.place_id)))
                .show();
        }
    }
    state.live.lock().unwrap().icon_url = icon;
    publish(app, tracker);
}

fn publish(app: &AppHandle, tracker: &Tracker) {
    let state = app.state::<AppState>();
    let settings = state.settings.get();
    let mut discord_status = None;
    if let Some(s) = &tracker.current {
        if settings.integrations.discord_enabled {
            let icon = state.live.lock().unwrap().icon_url.clone();
            let account = settings
                .accounts
                .active_user_id
                .and_then(|id| state.accounts.get().accounts.into_iter().find(|a| a.user_id == id))
                .map(|a| a.display_name);
            let presence = discord::build(s, icon.as_deref(), account.as_deref(), &settings.integrations);
            let mut guard = state.discord.lock().unwrap();
            if guard.as_ref().map(|r| r.app_id()) != Some(settings.integrations.discord_app_id.as_str()) {
                *guard = discord::Rpc::new(&settings.integrations.discord_app_id).ok();
            }
            discord_status = Some(match guard.as_mut() {
                None => "Add a valid Discord application ID in Settings → Integrations".into(),
                Some(rpc) => match rpc.set(&presence) {
                    Ok(()) => "Showing on Discord".into(),
                    Err(e) => e.to_string(),
                },
            });
        }
    }
    let mut live = state.live.lock().unwrap();
    live.session = tracker.current.clone();
    live.discord_status = discord_status;
    let _ = app.emit("activity", &*live);
}
