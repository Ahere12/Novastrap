//! Account manager: saved Roblox profiles, OAuth-linked sessions, switching.
//! Tokens live only in the OS credential store (Windows Credential Manager).

pub mod oauth;

use crate::error::{AppError, AppResult};
use crate::web::{Api, ttl};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SavedAccount {
    pub user_id: u64,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    /// True when signed in through Roblox OAuth (tokens in the credential store).
    pub linked: bool,
    pub added_at: u64,
    pub last_used_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct AccountsFile {
    pub accounts: Vec<SavedAccount>,
}

impl AccountsFile {
    pub fn upsert(&mut self, acc: SavedAccount) {
        match self.accounts.iter_mut().find(|a| a.user_id == acc.user_id) {
            Some(existing) => {
                let added = existing.added_at;
                *existing = SavedAccount { added_at: added, ..acc };
            }
            None => self.accounts.push(acc),
        }
    }

    pub fn remove(&mut self, user_id: u64) -> bool {
        let before = self.accounts.len();
        self.accounts.retain(|a| a.user_id != user_id);
        before != self.accounts.len()
    }

    pub fn touch(&mut self, user_id: u64, now: u64) {
        if let Some(a) = self.accounts.iter_mut().find(|a| a.user_id == user_id) {
            a.last_used_at = Some(now);
        }
    }

    /// Most recently used first.
    pub fn recent(&self) -> Vec<SavedAccount> {
        let mut v = self.accounts.clone();
        v.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        v
    }
}

#[derive(Deserialize)]
struct UsernamesResponse {
    data: Vec<UsernameHit>,
}
#[derive(Deserialize)]
struct UsernameHit {
    id: u64,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Deserialize)]
struct UserResponse {
    id: u64,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thumb {
    pub target_id: u64,
    pub state: String,
    pub image_url: Option<String>,
}

#[derive(Deserialize)]
pub struct ThumbList {
    pub data: Vec<Thumb>,
}

pub fn valid_username(name: &str) -> bool {
    (3..=20).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub async fn headshot(api: &Api, user_id: u64) -> Option<String> {
    let url = format!(
        "https://thumbnails.roblox.com/v1/users/avatar-headshot?userIds={user_id}&size=150x150&format=Png&isCircular=false"
    );
    let list: ThumbList = api.get_json(&url, ttl::LONG).await.ok()?;
    list.data.into_iter().find(|t| t.state == "Completed").and_then(|t| t.image_url)
}

/// Public profile (no authentication involved).
pub async fn profile_by_id(api: &Api, user_id: u64, now: u64) -> AppResult<SavedAccount> {
    let u: UserResponse = api.get_json(&format!("https://users.roblox.com/v1/users/{user_id}"), ttl::MEDIUM).await?;
    Ok(SavedAccount {
        user_id: u.id,
        username: u.name,
        display_name: u.display_name,
        avatar_url: headshot(api, u.id).await,
        linked: false,
        added_at: now,
        last_used_at: None,
    })
}

pub async fn profile_by_username(api: &Api, username: &str, now: u64) -> AppResult<SavedAccount> {
    let username = username.trim();
    if !valid_username(username) {
        return Err(AppError::InvalidInput("Roblox usernames are 3–20 letters, digits or underscores".into()));
    }
    let body = serde_json::json!({ "usernames": [username], "excludeBannedUsers": true });
    let r: UsernamesResponse = api.post_json("https://users.roblox.com/v1/usernames/users", &body, ttl::MEDIUM).await?;
    let hit = r.data.into_iter().next().ok_or_else(|| AppError::InvalidInput(format!("No Roblox user named '{username}'")))?;
    Ok(SavedAccount {
        user_id: hit.id,
        username: hit.name,
        display_name: hit.display_name,
        avatar_url: headshot(api, hit.id).await,
        linked: false,
        added_at: now,
        last_used_at: None,
    })
}

// ---- credential store -------------------------------------------------------

const SERVICE: &str = "Novastrap";

fn entry(user_id: u64) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, &format!("roblox-oauth-{user_id}"))
        .map_err(|e| AppError::Unsupported(format!("credential store unavailable: {e}")))
}

pub fn save_tokens(user_id: u64, t: &oauth::Tokens) -> AppResult<()> {
    let json = serde_json::to_string(t).expect("serializable");
    entry(user_id)?
        .set_password(&json)
        .map_err(|e| AppError::Unsupported(format!("couldn't save to Credential Manager: {e}")))
}

pub fn load_tokens(user_id: u64) -> Option<oauth::Tokens> {
    let s = entry(user_id).ok()?.get_password().ok()?;
    serde_json::from_str(&s).ok()
}

pub fn delete_tokens(user_id: u64) {
    if let Ok(e) = entry(user_id) {
        let _ = e.delete_credential();
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SessionStatus {
    /// Saved profile only; never signed in.
    ProfileOnly,
    SignedIn,
    /// Tokens missing or refresh rejected; sign in again.
    Expired,
}

/// Validates the stored session, refreshing tokens when needed.
pub async fn session_status(client: &reqwest::Client, client_id: &str, acc: &SavedAccount) -> SessionStatus {
    if !acc.linked {
        return SessionStatus::ProfileOnly;
    }
    let Some(tokens) = load_tokens(acc.user_id) else { return SessionStatus::Expired };
    if !tokens.expired(oauth::now_secs()) {
        return SessionStatus::SignedIn;
    }
    match oauth::refresh(client, client_id, &tokens.refresh_token).await {
        Ok(t) => {
            let _ = save_tokens(acc.user_id, &t);
            SessionStatus::SignedIn
        }
        Err(e) => {
            log::warn!("token refresh failed for {}: {e}", acc.user_id);
            SessionStatus::Expired
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(id: u64, used: Option<u64>) -> SavedAccount {
        SavedAccount {
            user_id: id,
            username: format!("u{id}"),
            display_name: format!("U{id}"),
            avatar_url: None,
            linked: false,
            added_at: 1,
            last_used_at: used,
        }
    }

    #[test]
    fn upsert_remove_recent() {
        let mut f = AccountsFile::default();
        f.upsert(acc(1, Some(5)));
        f.upsert(acc(2, Some(9)));
        f.upsert(SavedAccount { display_name: "New".into(), added_at: 99, ..acc(1, Some(5)) });
        assert_eq!(f.accounts.len(), 2);
        assert_eq!(f.accounts[0].display_name, "New");
        assert_eq!(f.accounts[0].added_at, 1, "added_at is preserved");
        assert_eq!(f.recent()[0].user_id, 2);
        f.touch(1, 50);
        assert_eq!(f.recent()[0].user_id, 1);
        assert!(f.remove(2));
        assert!(!f.remove(2));
    }

    #[test]
    fn usernames() {
        assert!(valid_username("Builder_Man1"));
        assert!(!valid_username("ab"));
        assert!(!valid_username("has space"));
        assert!(!valid_username("x\"; DROP"));
    }

    #[tokio::test]
    #[ignore]
    async fn live_lookup() {
        let api = Api::new(crate::roblox::deployment::http_client().unwrap());
        let a = profile_by_username(&api, "Roblox", 0).await.unwrap();
        assert_eq!(a.user_id, 1);
        assert!(a.avatar_url.is_some());
        println!("{a:?}");
    }

    /// Writes, reads and deletes a throwaway entry in Windows Credential Manager.
    #[test]
    #[ignore]
    fn live_credential_store_round_trip() {
        let id = 9_999_999_999;
        let t = oauth::Tokens { access_token: "a".into(), refresh_token: "r".into(), expires_at: 42 };
        save_tokens(id, &t).unwrap();
        assert_eq!(load_tokens(id).unwrap().expires_at, 42);
        delete_tokens(id);
        assert!(load_tokens(id).is_none());
    }
}
