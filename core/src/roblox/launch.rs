//! Launching the official Roblox client. The executable is run unmodified with the same
//! argument the registered `roblox:` protocol handler would receive.

use super::detect::{HandlerOwner, Installation};
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchTarget {
    /// Open the Roblox app home screen.
    App,
    /// Join an experience, optionally a specific server.
    #[serde(rename_all = "camelCase")]
    Place { place_id: u64, job_id: Option<String> },
}

/// Server job IDs are UUIDs (8-4-4-4-12 hex).
pub fn is_valid_job_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    parts.len() == 5
        && parts.iter().zip([8, 4, 4, 4, 12]).all(|(p, n)| p.len() == n && p.chars().all(|c| c.is_ascii_hexdigit()))
}

fn leading_digits(s: &str) -> Option<u64> {
    let d: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    d.parse().ok().filter(|&n| n > 0)
}

fn query_param<'a>(url: &'a str, key: &str) -> Option<&'a str> {
    let query = url.split_once('?')?.1.split('#').next()?;
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        k.eq_ignore_ascii_case(key).then_some(v)
    })
}

impl LaunchTarget {
    pub fn place(place_id: u64, job_id: Option<String>) -> AppResult<Self> {
        if place_id == 0 {
            return Err(AppError::InvalidInput("place ID must be a positive number".into()));
        }
        if let Some(j) = &job_id {
            if !is_valid_job_id(j) {
                return Err(AppError::InvalidInput(format!("'{j}' is not a valid server ID")));
            }
        }
        Ok(LaunchTarget::Place { place_id, job_id })
    }

    /// Accepts a place ID, a roblox.com game URL, or a `games/start?placeId=` link.
    pub fn parse_user_input(input: &str) -> AppResult<Self> {
        let s = input.trim();
        if s.is_empty() {
            return Ok(LaunchTarget::App);
        }
        if let Ok(id) = s.parse::<u64>() {
            return Self::place(id, None);
        }
        let lower = s.to_ascii_lowercase();
        let is_roblox_url = lower.contains("roblox.com/") || lower.starts_with("roblox://");
        if is_roblox_url {
            if let Some(id) = query_param(s, "placeId").and_then(leading_digits) {
                let job = query_param(s, "gameInstanceId").map(str::to_owned);
                return Self::place(id, job);
            }
            if let Some(idx) = lower.find("/games/") {
                if let Some(id) = leading_digits(&s[idx + "/games/".len()..]) {
                    return Self::place(id, None);
                }
            }
            if lower.contains("/share") {
                return Err(AppError::InvalidInput(
                    "Roblox share links can't be resolved without signing in. Open the game page and copy its URL instead.".into(),
                ));
            }
        }
        Err(AppError::InvalidInput("Enter a place ID or a roblox.com game link".into()))
    }

    /// The argument passed to `RobloxPlayerBeta.exe`, or `None` to open the app home.
    pub fn client_argument(&self) -> Option<String> {
        match self {
            LaunchTarget::App => None,
            LaunchTarget::Place { place_id, job_id } => Some(match job_id {
                Some(j) => format!("roblox://experiences/start?placeId={place_id}&gameInstanceId={j}"),
                None => format!("roblox://experiences/start?placeId={place_id}"),
            }),
        }
    }
}

/// Picks the client to run: the up-to-date install, else the one the official handler
/// points at, else the newest one found.
pub fn choose_executable<'a>(
    installs: &'a [Installation],
    latest_guid: Option<&str>,
    handler: &HandlerOwner,
) -> Option<&'a Path> {
    if let Some(g) = latest_guid {
        if let Some(i) = installs.iter().find(|i| i.version_guid == g) {
            return Some(&i.executable);
        }
    }
    if let HandlerOwner::Official { executable } = handler {
        if let Some(i) = installs.iter().find(|i| &i.executable == executable) {
            return Some(&i.executable);
        }
    }
    installs.first().map(|i| i.executable.as_path())
}

/// Starts the client and returns its process ID. Does not wait for it to exit.
pub fn spawn(executable: &Path, target: &LaunchTarget) -> AppResult<u32> {
    if !executable.is_file() {
        return Err(AppError::Launch(format!("{} no longer exists. Refresh and try again.", executable.display())));
    }
    let mut cmd = std::process::Command::new(executable);
    // Fully detach: the client must not hold Novastrap's stdio handles open.
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(arg) = target.client_argument() {
        cmd.arg(arg);
    }
    if let Some(dir) = executable.parent() {
        cmd.current_dir(dir);
    }
    let child = cmd.spawn().map_err(|e| AppError::Launch(format!("couldn't start Roblox: {e}")))?;
    log::info!("launched Roblox pid={} target={target:?}", child.id());
    Ok(child.id())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const JOB: &str = "0f8fad5b-d9cb-469f-a165-70867728950e";

    #[test]
    fn job_ids() {
        assert!(is_valid_job_id(JOB));
        assert!(!is_valid_job_id("0f8fad5b-d9cb-469f-a165"));
        assert!(!is_valid_job_id("0f8fad5b-d9cb-469f-a165-70867728950z"));
        assert!(!is_valid_job_id("x&placeId=1"));
    }

    #[test]
    fn parses_inputs() {
        assert_eq!(LaunchTarget::parse_user_input("  ").unwrap(), LaunchTarget::App);
        assert_eq!(
            LaunchTarget::parse_user_input("920587237").unwrap(),
            LaunchTarget::Place { place_id: 920587237, job_id: None }
        );
        assert_eq!(
            LaunchTarget::parse_user_input("https://www.roblox.com/games/920587237/Adopt-Me").unwrap(),
            LaunchTarget::Place { place_id: 920587237, job_id: None }
        );
        assert_eq!(
            LaunchTarget::parse_user_input(&format!(
                "https://www.roblox.com/games/start?placeId=1818&gameInstanceId={JOB}"
            ))
            .unwrap(),
            LaunchTarget::Place { place_id: 1818, job_id: Some(JOB.into()) }
        );
        assert!(LaunchTarget::parse_user_input("https://example.com/games/1").is_err());
        assert!(LaunchTarget::parse_user_input("0").is_err());
        assert!(LaunchTarget::parse_user_input("https://www.roblox.com/share?code=abc&type=ExperienceDetails").is_err());
        // Injection attempt through the job id is rejected.
        assert!(LaunchTarget::parse_user_input("https://www.roblox.com/games/start?placeId=1&gameInstanceId=a\" --evil").is_err());
    }

    #[test]
    fn builds_client_argument() {
        assert_eq!(LaunchTarget::App.client_argument(), None);
        assert_eq!(
            LaunchTarget::place(1818, None).unwrap().client_argument().unwrap(),
            "roblox://experiences/start?placeId=1818"
        );
        assert_eq!(
            LaunchTarget::place(1818, Some(JOB.into())).unwrap().client_argument().unwrap(),
            format!("roblox://experiences/start?placeId=1818&gameInstanceId={JOB}")
        );
    }

    #[test]
    fn chooses_executable() {
        let mk = |g: &str| Installation {
            version_guid: g.into(),
            directory: PathBuf::from(g),
            executable: PathBuf::from(g).join("RobloxPlayerBeta.exe"),
        };
        let installs = vec![mk("version-bbbbbbbbbbbbbbbb"), mk("version-aaaaaaaaaaaaaaaa")];
        let none = HandlerOwner::None;
        assert_eq!(
            choose_executable(&installs, Some("version-aaaaaaaaaaaaaaaa"), &none),
            Some(installs[1].executable.as_path())
        );
        let handler = HandlerOwner::Official { executable: installs[1].executable.clone() };
        assert_eq!(choose_executable(&installs, None, &handler), Some(installs[1].executable.as_path()));
        assert_eq!(choose_executable(&installs, None, &none), Some(installs[0].executable.as_path()));
        assert_eq!(choose_executable(&[], None, &none), None);
    }

    #[test]
    fn spawn_rejects_missing_exe() {
        let err = spawn(Path::new(r"Z:\missing\RobloxPlayerBeta.exe"), &LaunchTarget::App).unwrap_err();
        assert_eq!(err.kind(), "launch");
    }
}
