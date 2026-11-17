//! Game discovery: Roblox's explore sorts (trending, popular…), search, game details,
//! thumbnails and local recommendations.
//!
//! explore-api and search-api are the endpoints roblox.com itself uses; they are not
//! formally documented, so every response is decoded into strict types and failures
//! surface as `upstream_changed` instead of bad data.

use crate::accounts::ThumbList;
use crate::error::{AppError, AppResult};
use crate::web::{Api, ttl};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const SESSION: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameCard {
    pub universe_id: u64,
    pub root_place_id: u64,
    pub name: String,
    #[serde(default)]
    pub player_count: Option<u64>,
    #[serde(default)]
    pub total_up_votes: Option<u64>,
    #[serde(default)]
    pub total_down_votes: Option<u64>,
    #[serde(default, rename(deserialize = "genreL1"))]
    pub genre: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub is_sponsored: bool,
}

impl GameCard {
    /// Like ratio in percent, when votes are known.
    pub fn rating(&self) -> Option<u8> {
        let (u, d) = (self.total_up_votes?, self.total_down_votes?);
        (u + d > 0).then(|| ((u * 100) / (u + d)) as u8)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sort {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub games: Vec<GameCard>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSorts {
    sorts: Vec<RawSort>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSort {
    content_type: String,
    #[serde(default)]
    sort_id: String,
    #[serde(default)]
    sort_display_name: String,
    #[serde(default)]
    topic_layout_data: Option<serde_json::Value>,
    #[serde(default)]
    games: Vec<GameCard>,
}

/// Fills `icon_url` for each card (batched, 100 per request).
pub async fn attach_icons(api: &Api, cards: &mut [GameCard]) {
    let ids: Vec<u64> = cards.iter().filter(|c| c.icon_url.is_none()).map(|c| c.universe_id).collect::<HashSet<_>>().into_iter().collect();
    let mut found: HashMap<u64, String> = HashMap::new();
    for chunk in ids.chunks(100) {
        let list = chunk.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
        let url = format!("https://thumbnails.roblox.com/v1/games/icons?universeIds={list}&size=150x150&format=Png&isCircular=false");
        if let Ok(t) = api.get_json::<ThumbList>(&url, ttl::LONG).await {
            found.extend(t.data.into_iter().filter_map(|t| Some((t.target_id, t.image_url?))));
        }
    }
    for c in cards.iter_mut() {
        if c.icon_url.is_none() {
            c.icon_url = found.get(&c.universe_id).cloned();
        }
    }
}

/// Roblox home sorts (Top Trending, Up-and-Coming, Top Playing Now, …). Sponsored entries removed.
pub async fn sorts(api: &Api) -> AppResult<Vec<Sort>> {
    let url = format!("https://apis.roblox.com/explore-api/v1/get-sorts?sessionId={SESSION}");
    let raw: RawSorts = api.get_json(&url, ttl::MEDIUM).await?;
    let mut out: Vec<Sort> = raw
        .sorts
        .into_iter()
        .filter(|s| s.content_type == "Games" && !s.games.is_empty())
        .map(|s| Sort {
            id: s.sort_id,
            name: s.sort_display_name,
            description: s
                .topic_layout_data
                .and_then(|v| v.get("infoText").and_then(|t| t.as_str()).map(str::to_owned)),
            games: s.games.into_iter().filter(|g| !g.is_sponsored).take(40).collect(),
        })
        .collect();
    let mut all: Vec<GameCard> = out.iter().flat_map(|s| s.games.clone()).collect();
    attach_icons(api, &mut all).await;
    let icons: HashMap<u64, Option<String>> = all.into_iter().map(|g| (g.universe_id, g.icon_url)).collect();
    for s in &mut out {
        for g in &mut s.games {
            g.icon_url = icons.get(&g.universe_id).cloned().flatten();
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSearch {
    #[serde(default)]
    search_results: Vec<RawGroup>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawGroup {
    content_group_type: String,
    #[serde(default)]
    contents: Vec<GameCard>,
}

pub fn encode_query(q: &str) -> String {
    q.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

pub async fn search(api: &Api, query: &str) -> AppResult<Vec<GameCard>> {
    let q = query.trim();
    if q.is_empty() || q.len() > 80 {
        return Err(AppError::InvalidInput("Search for 1–80 characters".into()));
    }
    let url = format!(
        "https://apis.roblox.com/search-api/omni-search?searchQuery={}&sessionId={SESSION}&pageType=all",
        encode_query(q)
    );
    let raw: RawSearch = api.get_json(&url, ttl::MEDIUM).await?;
    let mut games: Vec<GameCard> = raw
        .search_results
        .into_iter()
        .filter(|g| g.content_group_type == "Game")
        .flat_map(|g| g.contents)
        .filter(|g| !g.is_sponsored)
        .collect();
    let mut seen = HashSet::new();
    games.retain(|g| seen.insert(g.universe_id));
    attach_icons(api, &mut games).await;
    Ok(games)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Creator {
    pub id: u64,
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub kind: String,
    #[serde(default)]
    pub has_verified_badge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameDetail {
    pub id: u64,
    pub root_place_id: u64,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub creator: Creator,
    #[serde(default)]
    pub playing: u64,
    #[serde(default)]
    pub visits: u64,
    #[serde(default)]
    pub max_players: u32,
    #[serde(default)]
    pub favorited_count: u64,
    pub created: String,
    pub updated: String,
    #[serde(default, rename(deserialize = "genre_l1"))]
    pub genre: Option<String>,
    #[serde(default, rename(deserialize = "genre_l2"))]
    pub subgenre: Option<String>,
    #[serde(default)]
    pub thumbnails: Vec<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub up_votes: Option<u64>,
    #[serde(default)]
    pub down_votes: Option<u64>,
}

#[derive(Deserialize)]
struct DataList<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct UniverseOfPlace {
    #[serde(rename = "universeId")]
    universe_id: Option<u64>,
}

pub async fn universe_for_place(api: &Api, place_id: u64) -> AppResult<u64> {
    let r: UniverseOfPlace =
        api.get_json(&format!("https://apis.roblox.com/universes/v1/places/{place_id}/universe"), ttl::LONG).await?;
    r.universe_id.ok_or_else(|| AppError::InvalidInput(format!("place {place_id} doesn't exist")))
}

#[derive(Deserialize)]
struct MultiThumbs {
    #[serde(default)]
    thumbnails: Vec<crate::accounts::Thumb>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Votes {
    up_votes: u64,
    down_votes: u64,
}

pub async fn details(api: &Api, universe_id: u64) -> AppResult<GameDetail> {
    let list: DataList<GameDetail> =
        api.get_json(&format!("https://games.roblox.com/v1/games?universeIds={universe_id}"), ttl::MEDIUM).await?;
    let mut g = list.data.into_iter().next().ok_or_else(|| AppError::InvalidInput("experience not found".into()))?;
    let thumbs_url = format!(
        "https://thumbnails.roblox.com/v1/games/multiget/thumbnails?universeIds={universe_id}&countPerUniverse=6&size=768x432&format=Png&isCircular=false"
    );
    if let Ok(t) = api.get_json::<DataList<MultiThumbs>>(&thumbs_url, ttl::LONG).await {
        g.thumbnails = t.data.into_iter().flat_map(|m| m.thumbnails).filter_map(|t| t.image_url).collect();
    }
    let mut card = [GameCard {
        universe_id,
        root_place_id: g.root_place_id,
        name: String::new(),
        player_count: None,
        total_up_votes: None,
        total_down_votes: None,
        genre: None,
        icon_url: None,
        is_sponsored: false,
    }];
    attach_icons(api, &mut card).await;
    g.icon_url = card[0].icon_url.take();
    if let Ok(v) =
        api.get_json::<DataList<Votes>>(&format!("https://games.roblox.com/v1/games/votes?universeIds={universe_id}"), ttl::MEDIUM).await
    {
        if let Some(v) = v.data.into_iter().next() {
            g.up_votes = Some(v.up_votes);
            g.down_votes = Some(v.down_votes);
        }
    }
    Ok(g)
}

/// Local recommendations: games from Roblox's sorts whose genre matches what the user
/// plays or favorites, excluding ones they already have. Ranked by genre affinity, then players.
pub fn recommend(sorts: &[Sort], liked_genres: &HashMap<String, u32>, exclude: &HashSet<u64>, limit: usize) -> Vec<GameCard> {
    let mut seen = HashSet::new();
    let mut pool: Vec<(u32, GameCard)> = sorts
        .iter()
        .flat_map(|s| s.games.iter())
        .filter(|g| !exclude.contains(&g.universe_id) && seen.insert(g.universe_id))
        .map(|g| (g.genre.as_ref().and_then(|x| liked_genres.get(x)).copied().unwrap_or(0), g.clone()))
        .filter(|(score, _)| *score > 0)
        .collect();
    pool.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.player_count.cmp(&a.1.player_count)));
    pool.into_iter().take(limit).map(|(_, g)| g).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: u64, genre: &str, players: u64) -> GameCard {
        GameCard {
            universe_id: id,
            root_place_id: id * 10,
            name: format!("g{id}"),
            player_count: Some(players),
            total_up_votes: Some(90),
            total_down_votes: Some(10),
            genre: Some(genre.into()),
            icon_url: None,
            is_sponsored: false,
        }
    }

    #[test]
    fn parses_sort_game() {
        let g: GameCard = serde_json::from_str(
            r#"{"universeId":1,"rootPlaceId":2,"name":"X","playerCount":5,"totalUpVotes":3,"totalDownVotes":1,"isSponsored":false,"genreL1":"Survival"}"#,
        )
        .unwrap();
        assert_eq!(g.genre.as_deref(), Some("Survival"));
        assert_eq!(g.rating(), Some(75));
    }

    #[test]
    fn query_encoding() {
        assert_eq!(encode_query("tower defense&x=1"), "tower%20defense%26x%3D1");
    }

    #[test]
    fn recommendations() {
        let sorts = vec![Sort {
            id: "s".into(),
            name: "S".into(),
            description: None,
            games: vec![card(1, "RPG", 10), card(2, "Obby", 50), card(3, "RPG", 99), card(4, "Horror", 1)],
        }];
        let liked = HashMap::from([("RPG".to_string(), 3), ("Obby".to_string(), 1)]);
        let exclude = HashSet::from([3]);
        let r: Vec<u64> = recommend(&sorts, &liked, &exclude, 10).iter().map(|g| g.universe_id).collect();
        assert_eq!(r, vec![1, 2]);
    }

    #[tokio::test]
    #[ignore]
    async fn live_discovery() {
        let api = Api::new(crate::roblox::deployment::http_client().unwrap());
        let s = sorts(&api).await.unwrap();
        println!("sorts: {:?}", s.iter().map(|s| (&s.name, s.games.len())).collect::<Vec<_>>());
        assert!(s.iter().any(|s| s.games.iter().any(|g| g.icon_url.is_some())));
        let r = search(&api, "tower defense").await.unwrap();
        println!("search hits: {}", r.len());
        let d = details(&api, 383310974).await.unwrap();
        println!("{} by {} — {} playing, {} thumbs, votes {:?}/{:?}", d.name, d.creator.name, d.playing, d.thumbnails.len(), d.up_votes, d.down_votes);
        assert_eq!(universe_for_place(&api, 920587237).await.unwrap(), 383310974);
    }
}
