//! Roblox integration. Never modifies the Roblox executable or interacts with the game process.

pub mod deployment;
pub mod detect;

use crate::error::{AppError, ErrorInfo};
use deployment::ClientVersion;
use detect::{HandlerOwner, Installation};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum UpdateState {
    NotInstalled,
    UpToDate,
    UpdateAvailable,
    /// Latest version could not be determined (offline, Roblox API error, …).
    Unknown { error: ErrorInfo },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxStatus {
    pub installations: Vec<Installation>,
    pub handler: HandlerOwner,
    pub latest: Option<ClientVersion>,
    pub update: UpdateState,
}

/// Pure decision logic, separated from I/O so it can be unit-tested.
pub fn update_state(installs: &[Installation], latest: Result<&ClientVersion, &AppError>) -> UpdateState {
    if installs.is_empty() {
        return UpdateState::NotInstalled;
    }
    match latest {
        Ok(v) if installs.iter().any(|i| i.version_guid == v.client_version_upload) => UpdateState::UpToDate,
        Ok(_) => UpdateState::UpdateAvailable,
        Err(e) => UpdateState::Unknown { error: e.into() },
    }
}

pub async fn status(client: &reqwest::Client) -> RobloxStatus {
    let installations: Vec<Installation> =
        detect::official_versions_dirs().iter().flat_map(|d| detect::scan_versions_dir(d)).collect();
    let handler = detect::classify_handler(detect::read_protocol_command().as_deref());
    let latest = deployment::latest(client, None).await;
    let update = update_state(&installations, latest.as_ref());
    log::info!("roblox status: {} install(s), handler={handler:?}, update={update:?}", installations.len());
    RobloxStatus { installations, handler, latest: latest.ok(), update }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn inst(guid: &str) -> Installation {
        Installation { version_guid: guid.into(), directory: PathBuf::new(), executable: PathBuf::new() }
    }
    fn ver(guid: &str) -> ClientVersion {
        ClientVersion { version: "0.1.0".into(), client_version_upload: guid.into() }
    }

    #[test]
    fn decides_update_state() {
        let a = "version-aaaaaaaaaaaaaaaa";
        let b = "version-bbbbbbbbbbbbbbbb";
        let (va, err) = (ver(a), AppError::Network("offline".into()));
        assert!(matches!(update_state(&[], Ok(&va)), UpdateState::NotInstalled));
        assert!(matches!(update_state(&[inst(a)], Ok(&va)), UpdateState::UpToDate));
        assert!(matches!(update_state(&[inst(b), inst(a)], Ok(&va)), UpdateState::UpToDate));
        assert!(matches!(update_state(&[inst(b)], Ok(&va)), UpdateState::UpdateAvailable));
        assert!(matches!(update_state(&[inst(a)], Err(&err)), UpdateState::Unknown { .. }));
    }
}
