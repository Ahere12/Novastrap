//! Small pure helpers shared by the app layer (kept here so they are unit-tested).

use crate::error::{AppError, AppResult};

/// Hosts the UI may ask Novastrap to open in the default browser.
pub const EXTERNAL_HOSTS: &[&str] = &[
    "www.roblox.com",
    "roblox.com",
    "create.roblox.com",
    "devforum.roblox.com",
    "www.rolimons.com",
    "discord.com",
    "github.com",
];

pub fn check_external(url: &str) -> AppResult<()> {
    let ok = url
        .strip_prefix("https://")
        .map(|rest| rest.split(['/', '?', '#']).next().unwrap_or(""))
        .is_some_and(|host| EXTERNAL_HOSTS.contains(&host));
    if ok { Ok(()) } else { Err(AppError::InvalidInput("That link isn't on the list of sites Novastrap opens".into())) }
}

/// True when release tag `latest` (e.g. "v0.2.0") is newer than `current`.
pub fn newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| v.trim_start_matches('v').split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parse(latest) > parse(current)
}

/// A Windows Internet Shortcut that opens an experience through Roblox's own `roblox://` link.
/// Returns (file name, file contents).
pub fn game_shortcut(name: &str, place_id: u64) -> (String, String) {
    let mut clean: String = name
        .chars()
        .filter(|c| !c.is_control() && !"<>:\"/\\|?*".contains(*c))
        .collect::<String>()
        .trim()
        .trim_end_matches('.')
        .chars()
        .take(60)
        .collect();
    if clean.trim().is_empty() {
        clean = format!("Roblox game {place_id}");
    }
    let body = format!("[InternetShortcut]\r\nURL=roblox://experiences/start?placeId={place_id}\r\n");
    (format!("{}.url", clean.trim()), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_shortcut_files() {
        let (f, b) = game_shortcut("  Do Not Stop: <A Driving Game>?.. ", 42);
        assert_eq!(f, "Do Not Stop A Driving Game.url");
        assert!(b.contains("URL=roblox://experiences/start?placeId=42"));
        assert_eq!(game_shortcut("???", 7).0, "Roblox game 7.url");
        assert_eq!(game_shortcut(&"x".repeat(200), 1).0.len(), 64);
    }

    #[test]
    fn external_allowlist() {
        assert!(check_external("https://www.roblox.com/catalog/1").is_ok());
        assert!(check_external("https://create.roblox.com/dashboard/credentials").is_ok());
        assert!(check_external("http://www.roblox.com/").is_err());
        assert!(check_external("https://www.roblox.com.evil.io/").is_err());
        assert!(check_external("file:///C:/Windows").is_err());
    }

    #[test]
    fn version_compare() {
        assert!(newer("v0.2.0", "0.1.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(!newer("v0.1.0", "0.1.0"));
    }
}
