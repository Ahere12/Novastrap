//! Modules 4, 5 and Home: discovery, servers, favorites, history and stats.

use super::now;
use crate::state::{AppState, Live};
use novastrap_core::activity::{self, PlayStats};
use novastrap_core::discover::{self, GameCard, GameDetail, Sort};
use novastrap_core::error::AppResult;
use novastrap_core::favorites::{FavGame, FavServer, FavoritesFile};
use novastrap_core::servers::{self, Order, ServerPage};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverHome {
    pub sorts: Vec<Sort>,
    pub recommended: Vec<GameCard>,
    /// Why there are no recommendations (no history/favorites yet).
    pub recommended_hint: Option<String>,
}

#[tauri::command]
pub async fn discover_home(state: State<'_, AppState>) -> AppResult<DiscoverHome> {
    let started = std::time::Instant::now();
    let sorts = discover::sorts(&state.api).await.inspect_err(|e| log::error!("discover sorts failed: {e}"))?;
    log::info!("discover: {} sorts in {} ms", sorts.len(), started.elapsed().as_millis());
    let favs = state.favorites.get();
    let history = state.history.get();
    let mut liked: HashMap<String, u32> = HashMap::new();
    let mut exclude: HashSet<u64> = HashSet::new();
    for f in &favs.games {
        exclude.insert(f.universe_id);
        if let Some(g) = &f.genre {
            *liked.entry(g.clone()).or_default() += 3;
        }
    }
    // Genres of played games are known when they appear in the sorts.
    let genre_of: HashMap<u64, String> =
        sorts.iter().flat_map(|s| s.games.iter()).filter_map(|g| Some((g.universe_id, g.genre.clone()?))).collect();
    for s in &history.sessions {
        if let Some(u) = s.universe_id {
            exclude.insert(u);
            if let Some(g) = genre_of.get(&u) {
                *liked.entry(g.clone()).or_default() += 1;
            }
        }
    }
    let recommended = discover::recommend(&sorts, &liked, &exclude, 16);
    let recommended_hint = recommended
        .is_empty()
        .then(|| "Favorite or play a few experiences and recommendations will appear here.".to_string());
    Ok(DiscoverHome { sorts, recommended, recommended_hint })
}

#[tauri::command]
pub async fn discover_search(state: State<'_, AppState>, query: String) -> AppResult<Vec<GameCard>> {
    let started = std::time::Instant::now();
    let r = discover::search(&state.api, &query).await.inspect_err(|e| log::error!("search failed: {e}"))?;
    log::info!("search: {} results in {} ms", r.len(), started.elapsed().as_millis());
    Ok(r)
}

#[tauri::command]
pub async fn game_details(state: State<'_, AppState>, universe_id: Option<u64>, place_id: Option<u64>) -> AppResult<GameDetail> {
    let u = match (universe_id, place_id) {
        (Some(u), _) => u,
        (None, Some(p)) => discover::universe_for_place(&state.api, p).await?,
        (None, None) => return Err(novastrap_core::error::AppError::InvalidInput("no game given".into())),
    };
    discover::details(&state.api, u).await
}

#[tauri::command]
pub async fn server_list(state: State<'_, AppState>, place_id: u64, order: Order, cursor: Option<String>, fresh: bool) -> AppResult<ServerPage> {
    servers::list(&state.api, place_id, order, cursor.as_deref(), fresh).await.inspect_err(|e| log::error!("server list failed: {e}"))
}

#[tauri::command]
pub fn favorites(state: State<'_, AppState>) -> FavoritesFile {
    state.favorites.get()
}

#[tauri::command]
pub fn toggle_favorite_game(state: State<'_, AppState>, mut game: FavGame) -> AppResult<bool> {
    game.added_at = now();
    state.favorites.update(|f| f.toggle_game(game))
}

#[tauri::command]
pub fn toggle_favorite_server(state: State<'_, AppState>, mut server: FavServer) -> AppResult<bool> {
    novastrap_core::roblox::launch::LaunchTarget::place(server.place_id, Some(server.job_id.clone()))?;
    server.added_at = now();
    server.label = server.label.chars().take(60).collect();
    state.favorites.update(|f| f.toggle_server(server))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeData {
    pub stats: PlayStats,
    pub icons: HashMap<u64, String>,
    pub live: Live,
    pub tracking: bool,
}

/// Recently played + playtime from local history, with icons.
#[tauri::command]
pub async fn home_data(state: State<'_, AppState>) -> Result<HomeData, ()> {
    let stats = activity::stats(&state.history.get(), now());
    let mut cards: Vec<GameCard> = stats
        .recent
        .iter()
        .chain(stats.top.iter())
        .filter_map(|g| {
            Some(GameCard {
                universe_id: g.universe_id?,
                root_place_id: g.place_id,
                name: String::new(),
                player_count: None,
                total_up_votes: None,
                total_down_votes: None,
                genre: None,
                icon_url: None,
                is_sponsored: false,
            })
        })
        .collect();
    discover::attach_icons(&state.api, &mut cards).await;
    let icons = cards.into_iter().filter_map(|c| Some((c.universe_id, c.icon_url?))).collect();
    Ok(HomeData {
        stats,
        icons,
        live: state.live.lock().unwrap().clone(),
        tracking: state.settings.get().general.track_activity,
    })
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> AppResult<()> {
    state.history.update(|h| h.sessions.clear())
}
