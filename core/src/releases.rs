//! Novastrap's own releases (GitHub): update channels, changelog, and local update history.

use crate::links::newer;
use serde::{Deserialize, Serialize};

pub const RELEASES_URL: &str = "https://api.github.com/repos/Ahere12/Novastrap/releases?per_page=30";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    #[serde(alias = "tag_name")]
    pub tag_name: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Release notes (Markdown as written on GitHub).
    #[serde(default)]
    pub body: Option<String>,
    #[serde(alias = "html_url")]
    pub html_url: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
    #[serde(default, alias = "published_at")]
    pub published_at: Option<String>,
}

/// Releases offered on a channel, newest version first. Drafts are never shown;
/// pre-releases only on "beta".
pub fn for_channel(mut all: Vec<Release>, channel: &str) -> Vec<Release> {
    all.retain(|r| !r.draft && (channel == "beta" || !r.prerelease) && r.html_url.starts_with("https://github.com/"));
    all.sort_by(|a, b| {
        if newer(&a.tag_name, &b.tag_name) {
            std::cmp::Ordering::Less
        } else if newer(&b.tag_name, &a.tag_name) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    all
}

/// The release to offer as an update, if any is newer than `current`.
pub fn update_for<'a>(channel_releases: &'a [Release], current: &str) -> Option<&'a Release> {
    channel_releases.first().filter(|r| newer(&r.tag_name, current))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VersionSeen {
    pub version: String,
    pub first_run: u64,
}

/// Versions of Novastrap this PC has run, oldest first.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateHistory {
    pub versions: Vec<VersionSeen>,
}

impl UpdateHistory {
    /// Records `version` if it differs from the last one run. Returns true when recorded.
    pub fn record(&mut self, version: &str, at: u64) -> bool {
        if self.versions.last().is_some_and(|v| v.version == version) {
            return false;
        }
        self.versions.push(VersionSeen { version: version.to_owned(), first_run: at });
        let n = self.versions.len();
        if n > 100 {
            self.versions.drain(..n - 100);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(tag: &str, pre: bool, draft: bool) -> Release {
        Release {
            tag_name: tag.into(),
            name: None,
            body: None,
            html_url: format!("https://github.com/Ahere12/Novastrap/releases/tag/{tag}"),
            prerelease: pre,
            draft,
            published_at: None,
        }
    }

    #[test]
    fn channels() {
        let all = vec![rel("v0.1.0", false, false), rel("v0.3.0-beta", true, false), rel("v0.2.0", false, false), rel("v9.0.0", false, true)];
        let stable = for_channel(all.clone(), "stable");
        assert_eq!(stable.iter().map(|r| r.tag_name.as_str()).collect::<Vec<_>>(), ["v0.2.0", "v0.1.0"]);
        assert_eq!(update_for(&stable, "0.1.0").map(|r| r.tag_name.as_str()), Some("v0.2.0"));
        assert!(update_for(&stable, "0.2.0").is_none());
        let beta = for_channel(all, "beta");
        assert_eq!(beta[0].tag_name, "v0.3.0-beta");
        assert!(for_channel(vec![], "stable").is_empty());
    }

    #[test]
    fn rejects_foreign_links() {
        let mut r = rel("v1.0.0", false, false);
        r.html_url = "https://evil.example/x".into();
        assert!(for_channel(vec![r], "stable").is_empty());
    }

    #[test]
    fn parses_github_json() {
        let j = r#"[{"tag_name":"v0.2.0","name":"0.2","body":"notes","html_url":"https://github.com/a/b","prerelease":false,"draft":false,"published_at":"2026-11-01T00:00:00Z"}]"#;
        let v: Vec<Release> = serde_json::from_str(j).unwrap();
        assert_eq!(v[0].body.as_deref(), Some("notes"));
    }

    #[test]
    fn history_records_changes_only() {
        let mut h = UpdateHistory::default();
        assert!(h.record("0.1.0", 1));
        assert!(!h.record("0.1.0", 2));
        assert!(h.record("0.2.0", 3));
        assert_eq!(h.versions.len(), 2);
    }
}
