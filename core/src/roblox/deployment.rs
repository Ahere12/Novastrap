//! Latest client version lookup via Roblox's client settings service.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const PRIMARY: &str = "https://clientsettingscdn.roblox.com";
const FALLBACK: &str = "https://clientsettings.roblox.com";
pub const BINARY_TYPE: &str = "WindowsPlayer";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersion {
    /// Human-readable version, e.g. "0.700.0.7000123".
    pub version: String,
    /// Deployment identifier, e.g. "version-0123456789abcdef". Matches the install folder name.
    pub client_version_upload: String,
}

impl ClientVersion {
    /// Rejects responses that parse as JSON but don't look like a real deployment.
    pub fn validate(self) -> AppResult<Self> {
        if !super::detect::is_version_guid(&self.client_version_upload) {
            return Err(AppError::UpstreamChanged(format!(
                "unexpected clientVersionUpload '{}'",
                self.client_version_upload
            )));
        }
        if self.version.is_empty() || !self.version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(AppError::UpstreamChanged(format!("unexpected version '{}'", self.version)));
        }
        Ok(self)
    }
}

pub fn client_version_path(channel: Option<&str>) -> AppResult<String> {
    match channel {
        None => Ok(format!("/v2/client-version/{BINARY_TYPE}")),
        Some(c) if !c.is_empty() && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') => {
            Ok(format!("/v2/client-version/{BINARY_TYPE}/channel/{c}"))
        }
        Some(c) => Err(AppError::Unsupported(format!("invalid channel name '{c}'"))),
    }
}

pub fn http_client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("Novastrap/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

async fn fetch_from(client: &reqwest::Client, base: &str, path: &str) -> AppResult<ClientVersion> {
    let res = client.get(format!("{base}{path}")).send().await?;
    let status = res.status();
    if !status.is_success() {
        return Err(AppError::HttpStatus(status.as_u16()));
    }
    res.json::<ClientVersion>().await?.validate()
}

/// Fetches the latest deployment, falling back to the non-CDN host on failure.
pub async fn latest(client: &reqwest::Client, channel: Option<&str>) -> AppResult<ClientVersion> {
    let path = client_version_path(channel)?;
    match fetch_from(client, PRIMARY, &path).await {
        Ok(v) => Ok(v),
        Err(e) => {
            log::warn!("clientsettingscdn failed ({e}); falling back");
            fetch_from(client, FALLBACK, &path).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_building() {
        assert_eq!(client_version_path(None).unwrap(), "/v2/client-version/WindowsPlayer");
        assert_eq!(
            client_version_path(Some("zbeta")).unwrap(),
            "/v2/client-version/WindowsPlayer/channel/zbeta"
        );
        assert!(client_version_path(Some("../evil")).is_err());
        assert!(client_version_path(Some("")).is_err());
    }

    #[test]
    fn validates_shape() {
        let ok: ClientVersion = serde_json::from_str(
            r#"{"version":"0.700.0.7000123","clientVersionUpload":"version-0123456789abcdef","bootstrapperVersion":"1"}"#,
        )
        .unwrap();
        assert!(ok.validate().is_ok());

        let bad = ClientVersion { version: "0.1".into(), client_version_upload: "nope".into() };
        assert!(matches!(bad.validate(), Err(AppError::UpstreamChanged(_))));
    }

    /// Live check against Roblox. Run with `cargo test -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn live_latest_version() {
        let v = latest(&http_client().unwrap(), None).await.unwrap();
        println!("{v:?}");
    }
}
