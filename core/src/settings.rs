//! User settings. Every field has a default so older files keep loading (`#[serde(default)]`).

use crate::error::{AppError, AppResult};
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
    pub shortcuts: Shortcuts,
    pub updates: Updates,
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
            shortcuts: Shortcuts::default(),
            updates: Updates::default(),
        }
    }
}

impl Settings {
    pub fn validate(&self) -> AppResult<()> {
        let id = self.accounts.oauth_client_id.trim();
        if !id.is_empty() && (id.len() > 40 || !id.bytes().all(|b| b.is_ascii_digit())) {
            return Err(AppError::InvalidInput("Roblox OAuth client IDs are numeric".into()));
        }
        let d = self.integrations.discord_app_id.trim();
        if !d.is_empty() && !crate::discord::valid_app_id(d) {
            return Err(AppError::InvalidInput("Discord application IDs are 17–20 digits".into()));
        }
        if self.integrations.discord_custom_state.chars().count() > 120 {
            return Err(AppError::InvalidInput("Custom Discord text is limited to 120 characters".into()));
        }
        let a = &self.appearance.accent;
        if !(a.len() == 7 && a.starts_with('#') && a[1..].bytes().all(|b| b.is_ascii_hexdigit())) {
            return Err(AppError::InvalidInput("Accent colors are #RRGGBB".into()));
        }
        if self.appearance.background_dim > 95 {
            return Err(AppError::InvalidInput("Background dimming is 0–95%".into()));
        }
        let ap = &self.appearance;
        if !(TEXT_SCALE_MIN..=TEXT_SCALE_MAX).contains(&ap.text_scale) {
            return Err(AppError::InvalidInput(format!("Text size is {TEXT_SCALE_MIN}–{TEXT_SCALE_MAX}%")));
        }
        for id in ap.sidebar_order.iter().chain(&ap.sidebar_hidden) {
            if !SIDEBAR_SECTIONS.contains(&id.as_str()) {
                return Err(AppError::InvalidInput(format!("Unknown sidebar section '{id}'")));
            }
        }
        if ap.sidebar_hidden.iter().any(|h| h == "play") {
            return Err(AppError::InvalidInput("The Roblox Player section can't be hidden".into()));
        }
        if !UPDATE_CHANNELS.contains(&self.updates.channel.as_str()) {
            return Err(AppError::InvalidInput("Update channel is stable or beta".into()));
        }
        if self.shortcuts.enabled {
            let mut seen = Vec::new();
            for s in self.shortcuts.all().into_iter().filter(|s| !s.trim().is_empty()) {
                let n = crate::shortcut::normalize(s)?;
                if seen.contains(&n) {
                    return Err(AppError::InvalidInput(format!("{n} is used twice")));
                }
                seen.push(n);
            }
        }
        if !THEMES.contains(&self.appearance.theme.as_str()) {
            return Err(AppError::InvalidInput("Unknown theme".into()));
        }
        if !FONTS.contains(&self.appearance.font.as_str()) {
            return Err(AppError::InvalidInput("Unknown font".into()));
        }
        Ok(())
    }
}

/// Launcher fonts (system-installed families only; nothing is downloaded).
/// "system" means the theme's own font (Source Sans Pro/Segoe UI for Legacy 2019, Verdana for Classic, Segoe UI for Launcher).
pub const FONTS: &[&str] = &["system", "segoe", "verdana", "bahnschrift", "consolas", "georgia"];

/// Launcher themes. "nova" is the name the dark theme had before themes were selectable; it is
/// still accepted in saved files and shown as the default (Legacy 2019), since nobody could have picked it.
pub const THEMES: &[&str] = &["legacy", "classic", "launcher", "nova"];

/// Sidebar sections that can be reordered (Settings is always last; Play can't be hidden).
pub const SIDEBAR_SECTIONS: &[&str] = &["play", "explore", "market", "tune"];
pub const TEXT_SCALE_MIN: u8 = 85;
pub const TEXT_SCALE_MAX: u8 = 130;
pub const UPDATE_CHANNELS: &[&str] = &["stable", "beta"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Shortcuts {
    /// System-wide shortcuts are off until the user turns them on.
    pub enabled: bool,
    pub show_window: String,
    pub toggle_hud: String,
    pub play: String,
}

impl Default for Shortcuts {
    fn default() -> Self {
        Shortcuts { enabled: false, show_window: "Ctrl+Alt+N".into(), toggle_hud: "Ctrl+Alt+H".into(), play: "Ctrl+Alt+P".into() }
    }
}

impl Shortcuts {
    pub fn all(&self) -> [&str; 3] {
        [&self.show_window, &self.toggle_hud, &self.play]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Updates {
    /// "stable" = full releases only; "beta" also offers GitHub pre-releases.
    pub channel: String,
}

impl Default for Updates {
    fn default() -> Self {
        Updates { channel: "stable".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    /// Minimize the launcher after starting Roblox.
    pub minimize_on_launch: bool,
    /// Watch Roblox's local logs for game activity (history, playtime, HUD, Discord).
    pub track_activity: bool,
    /// Closing the window hides Novastrap to the system tray instead of quitting.
    pub close_to_tray: bool,
}

impl Default for General {
    fn default() -> Self {
        General { minimize_on_launch: false, track_activity: true, close_to_tray: false }
    }
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
    /// Denser spacing everywhere.
    pub compact: bool,
    /// Text size in percent (accessibility).
    pub text_scale: u8,
    /// Section order in the sidebar (ids from SIDEBAR_SECTIONS; missing ones keep their default place).
    pub sidebar_order: Vec<String>,
    pub sidebar_hidden: Vec<String>,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: "legacy".into(),
            accent: "#8b7bff".into(),
            background: None,
            background_dim: 70,
            font: "system".into(),
            reduce_motion: false,
            ui_sounds: false,
            compact: false,
            text_scale: 100,
            sidebar_order: Vec::new(),
            sidebar_hidden: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_load_with_defaults() {
        let s: Settings = serde_json::from_str(r#"{"general":{"minimizeOnLaunch":true}}"#).unwrap();
        assert!(s.general.minimize_on_launch);
        assert!(s.general.track_activity, "missing fields take the struct default");
        assert_eq!(s.appearance.accent, "#8b7bff");
        assert!(s.integrations.discord_show_game);
    }

    #[test]
    fn validation() {
        let mut s = Settings::default();
        assert!(s.validate().is_ok());
        s.accounts.oauth_client_id = "abc".into();
        assert!(s.validate().is_err());
        s.accounts.oauth_client_id = "1234567".into();
        s.appearance.accent = "red".into();
        assert!(s.validate().is_err());
        s.appearance.accent = "#00ffaa".into();
        s.appearance.font = "Comic".into();
        assert!(s.validate().is_err());
        s.appearance.font = "verdana".into();
        s.appearance.theme = "neon".into();
        assert!(s.validate().is_err());
        s.appearance.theme = "launcher".into();
        assert!(s.validate().is_ok());
        s.appearance.text_scale = 200;
        assert!(s.validate().is_err());
        s.appearance.text_scale = 110;
        s.appearance.sidebar_hidden = vec!["play".into()];
        assert!(s.validate().is_err());
        s.appearance.sidebar_hidden = vec!["market".into()];
        s.appearance.sidebar_order = vec!["tune".into(), "play".into()];
        assert!(s.validate().is_ok());
        s.appearance.sidebar_order = vec!["bogus".into()];
        assert!(s.validate().is_err());
        s.appearance.sidebar_order.clear();
        s.updates.channel = "nightly".into();
        assert!(s.validate().is_err());
        s.updates.channel = "beta".into();
        s.shortcuts.enabled = true;
        assert!(s.validate().is_ok(), "defaults are valid");
        s.shortcuts.play = "alt+ctrl+n".into();
        assert!(s.validate().is_err(), "duplicate of show_window after normalizing");
        s.shortcuts.play = "N".into();
        assert!(s.validate().is_err());
        s.shortcuts.play = String::new();
        assert!(s.validate().is_ok(), "empty = unassigned");
    }
}
