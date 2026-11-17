//! Local favorites (games, servers, catalog items). Stored only on this PC;
//! Roblox's own favorites need the session cookie, which Novastrap doesn't use.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FavGame {
    pub universe_id: u64,
    pub place_id: u64,
    pub name: String,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FavServer {
    pub place_id: u64,
    pub job_id: String,
    pub label: String,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FavItem {
    pub id: u64,
    pub item_type: String,
    pub name: String,
    pub thumbnail_url: Option<String>,
    pub added_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct FavoritesFile {
    pub games: Vec<FavGame>,
    pub servers: Vec<FavServer>,
    pub items: Vec<FavItem>,
}

/// Adds when absent, removes when present. Returns whether it is now a favorite.
fn toggle<T>(list: &mut Vec<T>, item: T, same: impl Fn(&T, &T) -> bool) -> bool {
    if let Some(i) = list.iter().position(|x| same(x, &item)) {
        list.remove(i);
        false
    } else {
        list.insert(0, item);
        true
    }
}

impl FavoritesFile {
    pub fn toggle_game(&mut self, g: FavGame) -> bool {
        toggle(&mut self.games, g, |a, b| a.universe_id == b.universe_id)
    }
    pub fn toggle_server(&mut self, s: FavServer) -> bool {
        toggle(&mut self.servers, s, |a, b| a.job_id == b.job_id)
    }
    pub fn toggle_item(&mut self, i: FavItem) -> bool {
        toggle(&mut self.items, i, |a, b| a.id == b.id && a.item_type == b.item_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles() {
        let mut f = FavoritesFile::default();
        let g = FavGame { universe_id: 1, place_id: 2, name: "a".into(), icon_url: None, genre: None, added_at: 0 };
        assert!(f.toggle_game(g.clone()));
        assert_eq!(f.games.len(), 1);
        assert!(!f.toggle_game(g));
        assert!(f.games.is_empty());
    }
}
