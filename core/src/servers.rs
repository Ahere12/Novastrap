//! Public server browser (games.roblox.com, documented, unauthenticated).
//! `fps` and `ping` are what Roblox reports per server; they are not measured locally.

use crate::error::{AppError, AppResult};
use crate::web::{Api, ttl};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub max_players: u32,
    pub playing: u32,
    #[serde(default)]
    pub fps: Option<f64>,
    /// Roblox-reported ping in ms (server side), not the user's own latency.
    #[serde(default)]
    pub ping: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerPage {
    #[serde(default, rename(deserialize = "data"))]
    pub servers: Vec<Server>,
    #[serde(default, rename(deserialize = "nextPageCursor"))]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    /// Fullest first.
    Desc,
    /// Emptiest first.
    Asc,
}

pub fn valid_cursor(c: &str) -> bool {
    !c.is_empty() && c.len() < 4096 && c.bytes().all(|b| b.is_ascii_alphanumeric() || b"+/=_-".contains(&b))
}

fn encode_cursor(c: &str) -> String {
    c.replace('+', "%2B").replace('/', "%2F").replace('=', "%3D")
}

pub async fn list(api: &Api, place_id: u64, order: Order, cursor: Option<&str>, fresh: bool) -> AppResult<ServerPage> {
    if place_id == 0 {
        return Err(AppError::InvalidInput("place ID must be positive".into()));
    }
    let mut url = format!(
        "https://games.roblox.com/v1/games/{place_id}/servers/Public?limit=100&sortOrder={}&excludeFullGames=false",
        match order { Order::Desc => "Desc", Order::Asc => "Asc" }
    );
    if let Some(c) = cursor {
        if !valid_cursor(c) {
            return Err(AppError::InvalidInput("invalid page cursor".into()));
        }
        url.push_str("&cursor=");
        url.push_str(&encode_cursor(c));
    }
    let mut page: ServerPage = api.get_json(&url, if fresh { std::time::Duration::ZERO } else { ttl::SHORT }).await?;
    page.servers.retain(|s| crate::roblox::launch::is_valid_job_id(&s.id));
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_page() {
        let p: ServerPage = serde_json::from_str(
            r#"{"previousPageCursor":null,"nextPageCursor":"abc=","data":[{"id":"5fa5d026-13d8-4fed-b7f8-eb490936a282","maxPlayers":35,"playing":35,"playerTokens":[],"players":[],"fps":59.9,"ping":248}]}"#,
        )
        .unwrap();
        assert_eq!(p.servers[0].ping, Some(248));
        assert_eq!(p.next_cursor.as_deref(), Some("abc="));
    }

    #[test]
    fn cursors() {
        assert!(valid_cursor("eyJzdGFydEluZGV4IjoxMH0=+/"));
        assert!(!valid_cursor("a&evil=1"));
        assert_eq!(encode_cursor("a+b/c="), "a%2Bb%2Fc%3D");
    }

    #[tokio::test]
    #[ignore]
    async fn live_servers() {
        let api = Api::new(crate::roblox::deployment::http_client().unwrap());
        let p = list(&api, 920587237, Order::Desc, None, true).await.unwrap();
        println!("{} servers, first {:?}, next={}", p.servers.len(), p.servers.first(), p.next_cursor.is_some());
        assert!(!p.servers.is_empty());
        if let Some(c) = p.next_cursor.as_deref() {
            let p2 = list(&api, 920587237, Order::Desc, Some(c), true).await.unwrap();
            println!("page 2: {} servers", p2.servers.len());
        }
    }
}
