//! User settings. Every field has a default so older files keep loading (`#[serde(default)]`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub general: General,
    pub appearance: Appearance,
    pub accounts: Accounts,
    pub instances: Instances,
    pub integrations: Integrations,
    pub notifications: Notifications,
    pub privacy: Privacy,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            general: General::default(),
            appearance: Appearance::default(),
            accounts: Accounts::default(),
            instances: Instances::default(),
            integrations: Integrations::default(),
            notifications: Notifications::default(),
            privacy: Privacy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    /// Minimize the launcher after starting Roblox.
    pub minimize_on_launch: bool,
    /// Watch Roblox logs for game activity (history, playtime, HUD, Discord).
    pub track_activity: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub theme: String,
    pub accent: String,
    /// Background image stored in the app data folder (file name only).
    pub background: Option<String>,
    pub background_dim: u8,
    pub font: String,
    pub reduce_motion: bool,
    pub ui_sounds: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: "nova".into(),
            accent: "#8b7bff".into(),
            background: None,
            background_dim: 70,
            font: "system".into(),
            reduce_motion: false,
            ui_sounds: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Accounts {
    /// OAuth client ID from the Roblox Creator Dashboard (public client, PKCE).
    pub oauth_client_id: String,
    pub active_user_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Instances {
    /// Saved profile launched by the Home "Play" button (None = Roblox home screen).
    pub default_profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Integrations {
    pub discord_enabled: bool,
    /// Application ID from the Discord Developer Portal.
    pub discord_app_id: String,
    pub discord_show_game: bool,
    pub discord_show_join: bool,
    pub discord_show_account: bool,
    /// Optional custom line shown under the game name.
    pub discord_custom_state: String,
}

impl Default for Integrations {
    fn default() -> Self {
        Integrations {
            discord_enabled: false,
            discord_app_id: String::new(),
            discord_show_game: true,
            discord_show_join: false,
            discord_show_account: false,
            discord_custom_state: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Notifications {
    pub game_joined: bool,
    pub update_available: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Notifications { game_joined: false, update_available: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Privacy {
    /// Look up the joined server's region from its IP (sends the IP to ipinfo.io).
    pub server_location_lookup: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_load_with_defaults() {
        let s: Settings = serde_json::from_str(r#"{"general":{"minimizeOnLaunch":true}}"#).unwrap();
        assert!(s.general.minimize_on_launch);
        assert_eq!(s.appearance.accent, "#8b7bff");
        assert!(s.integrations.discord_show_game);
    }
}
