//! Installing/updating Roblox with the official installer from Roblox's CDN.
//!
//! Safety properties:
//! - The URL is built from a validated deployment GUID on a fixed Roblox host.
//! - Downloads land in `*.partial` and are only renamed after the full body arrived.
//! - The file's Authenticode signature must be valid AND signed by Roblox Corporation
//!   before it is executed. Anything else is deleted.

use super::detect::is_version_guid;
use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::path::{Path, PathBuf};

const SETUP_HOST: &str = "https://setup.rbxcdn.com";
const EXPECTED_ORG: &str = "Roblox Corporation";

pub fn installer_url(version_guid: &str) -> AppResult<String> {
    if !is_version_guid(version_guid) {
        return Err(AppError::InvalidInput(format!("invalid version '{version_guid}'")));
    }
    Ok(format!("{SETUP_HOST}/{version_guid}-RobloxPlayerInstaller.exe"))
}

pub fn installer_file_name(version_guid: &str) -> String {
    format!("{version_guid}-RobloxPlayerInstaller.exe")
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum InstallProgress {
    Downloading { downloaded: u64, total: Option<u64> },
    Verifying,
    Installing,
    Done,
}

/// Removes leftover `.partial` downloads from interrupted runs.
pub fn cleanup_partials(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "partial"))
        .filter(|e| std::fs::remove_file(e.path()).is_ok())
        .count()
}

/// Streams `url` into `dir/file_name`, reporting progress. Atomic: the final file only
/// appears once the download completed and matched Content-Length.
pub async fn download(
    client: &reqwest::Client,
    url: &str,
    dir: &Path,
    file_name: &str,
    mut progress: impl FnMut(u64, Option<u64>),
) -> AppResult<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let final_path = dir.join(file_name);
    let partial = dir.join(format!("{file_name}.partial"));

    let mut res = client.get(url).send().await?;
    if !res.status().is_success() {
        return Err(AppError::HttpStatus(res.status().as_u16()));
    }
    let total = res.content_length();
    let result: AppResult<u64> = async {
        use std::io::Write;
        let mut file = std::fs::File::create(&partial)?;
        let mut downloaded = 0u64;
        while let Some(chunk) = res.chunk().await? {
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            progress(downloaded, total);
        }
        file.sync_all()?;
        Ok(downloaded)
    }
    .await;

    match result {
        Ok(n) if total.is_none_or(|t| t == n) && n > 0 => {
            std::fs::rename(&partial, &final_path)?;
            Ok(final_path)
        }
        Ok(n) => {
            let _ = std::fs::remove_file(&partial);
            Err(AppError::Network(format!("download incomplete ({n} of {} bytes)", total.unwrap_or(0))))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&partial);
            Err(e)
        }
    }
}

/// Checks the output of `Get-AuthenticodeSignature`: status must be `Valid` and the
/// certificate subject's Organization must be exactly Roblox Corporation.
pub fn signature_acceptable(status: &str, subject: &str) -> bool {
    status.trim() == "Valid"
        && subject.split(',').any(|part| {
            part.trim().strip_prefix("O=").is_some_and(|o| o.trim_matches('"') == EXPECTED_ORG)
        })
}

#[cfg(windows)]
pub fn verify_signature(path: &Path) -> AppResult<()> {
    // The path goes through an environment variable, never interpolated into the script.
    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$s = Get-AuthenticodeSignature -LiteralPath $env:NOVASTRAP_VERIFY_PATH; \
             Write-Output $s.Status; Write-Output $s.SignerCertificate.Subject",
        ])
        .env("NOVASTRAP_VERIFY_PATH", path)
        .output()
        .map_err(|e| AppError::Verification(format!("couldn't run signature check: {e}")))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let (status, subject) = (lines.next().unwrap_or(""), lines.next().unwrap_or(""));
    log::info!("signature check: status={status} subject={subject}");
    if signature_acceptable(status, subject) {
        Ok(())
    } else {
        Err(AppError::Verification(format!(
            "installer signature rejected (status '{status}', signer '{subject}')"
        )))
    }
}

#[cfg(not(windows))]
pub fn verify_signature(_path: &Path) -> AppResult<()> {
    Err(AppError::Unsupported("installing the Windows client is only supported on Windows".into()))
}

/// Runs the verified installer and waits for it. Blocking: call from a worker thread.
pub fn run_installer(path: &Path) -> AppResult<()> {
    use std::process::Stdio;
    let status = std::process::Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| AppError::Launch(format!("couldn't start the Roblox installer: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::Launch(format!("the Roblox installer exited with code {}", status.code().unwrap_or(-1))))
    }
}

/// Full flow: download → verify → run. Cleans the installer file up afterwards.
pub async fn install(
    client: &reqwest::Client,
    version_guid: &str,
    downloads_dir: &Path,
    mut on_progress: impl FnMut(InstallProgress) + Send,
) -> AppResult<()> {
    let url = installer_url(version_guid)?;
    let removed = cleanup_partials(downloads_dir);
    if removed > 0 {
        log::info!("removed {removed} interrupted download(s)");
    }
    let file = download(client, &url, downloads_dir, &installer_file_name(version_guid), |d, t| {
        on_progress(InstallProgress::Downloading { downloaded: d, total: t })
    })
    .await?;

    on_progress(InstallProgress::Verifying);
    let verify_path = file.clone();
    let verified = blocking(move || verify_signature(&verify_path)).await;
    if let Err(e) = verified {
        let _ = std::fs::remove_file(&file);
        return Err(e);
    }

    on_progress(InstallProgress::Installing);
    let run_path = file.clone();
    let ran = blocking(move || run_installer(&run_path)).await;
    let _ = std::fs::remove_file(&file);
    ran?;
    on_progress(InstallProgress::Done);
    Ok(())
}

/// Runs blocking work (signature check, installer process) on tokio's blocking pool.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Launch(format!("worker task failed: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_url() {
        assert_eq!(
            installer_url("version-4ea8e85481e34f46").unwrap(),
            "https://setup.rbxcdn.com/version-4ea8e85481e34f46-RobloxPlayerInstaller.exe"
        );
        assert!(installer_url("version-../../x").is_err());
    }

    #[test]
    fn signature_rules() {
        let subj = "CN=Roblox Corporation, O=Roblox Corporation, L=San Mateo, S=California, C=US";
        assert!(signature_acceptable("Valid", subj));
        assert!(!signature_acceptable("HashMismatch", subj));
        assert!(!signature_acceptable("NotSigned", ""));
        assert!(!signature_acceptable("Valid", "CN=Roblox Corporation, O=Evil Roblox Corporation Ltd"));
        assert!(!signature_acceptable("Valid", "CN=Someone, O=Someone Else"));
    }

    #[test]
    fn cleans_partials() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.exe.partial"), b"x").unwrap();
        std::fs::write(d.path().join("keep.exe"), b"x").unwrap();
        assert_eq!(cleanup_partials(d.path()), 1);
        assert!(d.path().join("keep.exe").exists());
    }

    #[cfg(windows)]
    #[test]
    fn rejects_unsigned_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("fake.exe");
        std::fs::write(&p, b"MZ not really an exe").unwrap();
        assert_eq!(verify_signature(&p).unwrap_err().kind(), "verification");
    }

    /// Downloads and verifies the real current installer (does NOT run it).
    /// `cargo test -- --ignored live_download_and_verify`
    #[tokio::test]
    #[ignore]
    async fn live_download_and_verify() {
        let client = super::super::deployment::http_client().unwrap();
        let v = super::super::deployment::latest(&client, None).await.unwrap();
        let d = tempfile::tempdir().unwrap();
        let mut last = 0;
        let f = download(&client, &installer_url(&v.client_version_upload).unwrap(), d.path(), "i.exe", |n, _| last = n)
            .await
            .unwrap();
        println!("downloaded {last} bytes");
        verify_signature(&f).unwrap();
        println!("signature OK");
    }
}
