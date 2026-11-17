//! Discord Rich Presence over Discord's local IPC pipe. Opt-in; every field the user
//! might not want shared is controlled by a privacy toggle.

use crate::activity::Session;
use crate::error::{AppError, AppResult};
use crate::settings::Integrations;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};

#[derive(Debug, Clone, PartialEq)]
pub struct Presence {
    pub details: String,
    pub state: Option<String>,
    pub start: u64,
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub join_url: Option<String>,
}

/// Builds the presence for the current session, honouring privacy settings.
pub fn build(s: &Session, icon_url: Option<&str>, account: Option<&str>, cfg: &Integrations) -> Presence {
    let name = s.game_name.clone().unwrap_or_else(|| "a Roblox experience".into());
    let details = if cfg.discord_show_game { format!("Playing {name}") } else { "Playing Roblox".into() };
    let custom = cfg.discord_custom_state.trim();
    let state = if !custom.is_empty() {
        Some(custom.chars().take(120).collect())
    } else if cfg.discord_show_account {
        account.map(|a| format!("as {a}"))
    } else {
        None
    };
    Presence {
        details,
        state,
        start: s.started_at,
        large_image: if cfg.discord_show_game { icon_url.map(str::to_owned) } else { None },
        large_text: if cfg.discord_show_game { Some(name) } else { None },
        join_url: (cfg.discord_show_game && cfg.discord_show_join).then(|| {
            format!("https://www.roblox.com/games/start?placeId={}&gameInstanceId={}", s.place_id, s.job_id)
        }),
    }
}

pub fn valid_app_id(id: &str) -> bool {
    (17..=20).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit())
}

pub struct Rpc {
    client: DiscordIpcClient,
    app_id: String,
    connected: bool,
}

impl Rpc {
    pub fn new(app_id: &str) -> AppResult<Self> {
        if !valid_app_id(app_id) {
            return Err(AppError::InvalidInput("Discord application IDs are 17–20 digits".into()));
        }
        Ok(Rpc { client: DiscordIpcClient::new(app_id), app_id: app_id.into(), connected: false })
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    fn ensure(&mut self) -> AppResult<()> {
        if !self.connected {
            self.client.connect().map_err(|e| AppError::Unsupported(format!("Discord isn't running or refused the connection ({e})")))?;
            self.connected = true;
        }
        Ok(())
    }

    pub fn set(&mut self, p: &Presence) -> AppResult<()> {
        self.ensure()?;
        let mut a = activity::Activity::new().details(&p.details).timestamps(activity::Timestamps::new().start(p.start as i64));
        if let Some(s) = &p.state {
            a = a.state(s);
        }
        let mut assets = activity::Assets::new();
        if let Some(img) = &p.large_image {
            assets = assets.large_image(img);
        }
        if let Some(t) = &p.large_text {
            assets = assets.large_text(t);
        }
        a = a.assets(assets);
        let buttons;
        if let Some(url) = &p.join_url {
            buttons = vec![activity::Button::new("Join server", url)];
            a = a.buttons(buttons);
        }
        if let Err(e) = self.client.set_activity(a) {
            self.connected = false;
            return Err(AppError::Unsupported(format!("Discord presence update failed ({e})")));
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        if self.connected && self.client.clear_activity().is_err() {
            self.connected = false;
        }
    }
}

impl Drop for Rpc {
    fn drop(&mut self) {
        if self.connected {
            let _ = self.client.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session {
            place_id: 1818,
            universe_id: Some(13058),
            job_id: "0f8fad5b-d9cb-469f-a165-70867728950e".into(),
            server_ip: None,
            user_id: None,
            started_at: 100,
            ended_at: None,
            game_name: Some("Crossroads".into()),
        }
    }

    #[test]
    fn privacy_controls() {
        let mut cfg = Integrations::default();
        let p = build(&session(), Some("https://tr.rbxcdn.com/i"), Some("Builder"), &cfg);
        assert_eq!(p.details, "Playing Crossroads");
        assert!(p.join_url.is_none(), "join button is off by default");
        assert!(p.state.is_none(), "account is hidden by default");

        cfg.discord_show_game = false;
        cfg.discord_show_join = true;
        let p = build(&session(), Some("x"), None, &cfg);
        assert_eq!(p.details, "Playing Roblox");
        assert!(p.large_image.is_none() && p.join_url.is_none());

        cfg.discord_show_game = true;
        cfg.discord_show_account = true;
        let p = build(&session(), None, Some("Builder"), &cfg);
        assert_eq!(p.state.as_deref(), Some("as Builder"));
        assert!(p.join_url.unwrap().contains("gameInstanceId=0f8fad5b"));

        cfg.discord_custom_state = "Grinding levels".into();
        assert_eq!(build(&session(), None, Some("B"), &cfg).state.as_deref(), Some("Grinding levels"));
    }

    #[test]
    fn app_ids() {
        assert!(valid_app_id("1234567890123456789"));
        assert!(!valid_app_id("abc"));
        assert!(Rpc::new("12").is_err());
    }
}
