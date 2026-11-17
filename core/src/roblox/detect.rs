//! Detects the locally installed Roblox Player and who owns the `roblox-player:` protocol.
//! Read-only: nothing here modifies the system.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const PLAYER_EXE: &str = "RobloxPlayerBeta.exe";

/// `version-` followed by 16 lowercase hex digits (Roblox's deployment id format).
pub fn is_version_guid(s: &str) -> bool {
    s.strip_prefix("version-")
        .is_some_and(|h| h.len() == 16 && h.chars().all(|c| c.is_ascii_hexdigit()))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub version_guid: String,
    pub directory: PathBuf,
    pub executable: PathBuf,
}

/// Scans a `Versions` directory for folders that contain the player executable.
/// Returns them newest-first by executable modification time.
pub fn scan_versions_dir(versions_dir: &Path) -> Vec<Installation> {
    let Ok(entries) = std::fs::read_dir(versions_dir) else { return Vec::new() };
    let mut found: Vec<(SystemTime, Installation)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !is_version_guid(&name) {
                return None;
            }
            let exe = e.path().join(PLAYER_EXE);
            let mtime = std::fs::metadata(&exe).ok()?.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Some((mtime, Installation { version_guid: name, directory: e.path(), executable: exe }))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, i)| i).collect()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum HandlerOwner {
    /// No `roblox-player` protocol registered.
    None,
    Official { executable: PathBuf },
    Bloxstrap { executable: PathBuf },
    Fishstrap { executable: PathBuf },
    Novastrap { executable: PathBuf },
    Other { executable: PathBuf },
}

/// Extracts the executable path from a shell `open\command` value such as
/// `"C:\path\app.exe" %1` or `C:\path\app.exe -player "%1"`.
pub fn executable_from_command(cmd: &str) -> Option<PathBuf> {
    let cmd = cmd.trim();
    let path = if let Some(rest) = cmd.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        let lower = cmd.to_ascii_lowercase();
        let end = lower.find(".exe").map(|i| i + 4)?;
        &cmd[..end]
    };
    (!path.is_empty()).then(|| PathBuf::from(path))
}

pub fn classify_handler(command: Option<&str>) -> HandlerOwner {
    let Some(exe) = command.and_then(executable_from_command) else { return HandlerOwner::None };
    let name = exe
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match name.as_str() {
        "robloxplayerbeta.exe" | "robloxplayerlauncher.exe" => HandlerOwner::Official { executable: exe },
        "bloxstrap.exe" => HandlerOwner::Bloxstrap { executable: exe },
        "fishstrap.exe" => HandlerOwner::Fishstrap { executable: exe },
        "novastrap.exe" => HandlerOwner::Novastrap { executable: exe },
        _ => HandlerOwner::Other { executable: exe },
    }
}

/// Folders where the official installer places versions.
pub fn official_versions_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("Roblox").join("Versions"));
    }
    // Older machine-wide installs.
    if let Some(pf86) = std::env::var_os("ProgramFiles(x86)") {
        dirs.push(PathBuf::from(pf86).join("Roblox").join("Versions"));
    }
    dirs
}

#[cfg(windows)]
pub fn read_protocol_command() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CLASSES_ROOT, HKEY_CURRENT_USER};
    // HKCU overrides HKCR's merged view; check it first.
    let paths = [
        (HKEY_CURRENT_USER, r"Software\Classes\roblox-player\shell\open\command"),
        (HKEY_CLASSES_ROOT, r"roblox-player\shell\open\command"),
    ];
    paths.iter().find_map(|(root, path)| {
        RegKey::predef(*root).open_subkey(path).ok()?.get_value::<String, _>("").ok()
    })
}

#[cfg(not(windows))]
pub fn read_protocol_command() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_format() {
        assert!(is_version_guid("version-0123456789abcdef"));
        assert!(!is_version_guid("version-0123"));
        assert!(!is_version_guid("0123456789abcdef"));
        assert!(!is_version_guid("version-0123456789abcdeg"));
    }

    #[test]
    fn parses_commands() {
        assert_eq!(
            executable_from_command(r#""C:\R\Versions\version-0123456789abcdef\RobloxPlayerBeta.exe" %1"#),
            Some(PathBuf::from(r"C:\R\Versions\version-0123456789abcdef\RobloxPlayerBeta.exe"))
        );
        assert_eq!(
            executable_from_command(r#"C:\Apps\Bloxstrap.exe -player "%1""#),
            Some(PathBuf::from(r"C:\Apps\Bloxstrap.exe"))
        );
        assert_eq!(executable_from_command(""), None);
        assert_eq!(executable_from_command("garbage"), None);
    }

    #[test]
    fn classifies_handlers() {
        assert_eq!(classify_handler(None), HandlerOwner::None);
        assert!(matches!(classify_handler(Some(r#""C:\x\RobloxPlayerBeta.exe" %1"#)), HandlerOwner::Official { .. }));
        assert!(matches!(classify_handler(Some(r#""C:\x\Bloxstrap.exe" -player "%1""#)), HandlerOwner::Bloxstrap { .. }));
        assert!(matches!(classify_handler(Some(r#""C:\x\FISHSTRAP.EXE" %1"#)), HandlerOwner::Fishstrap { .. }));
        assert!(matches!(classify_handler(Some(r#""C:\x\other.exe" %1"#)), HandlerOwner::Other { .. }));
    }

    #[test]
    fn scans_versions_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let mk = |name: &str, with_exe: bool| {
            let p = dir.path().join(name);
            std::fs::create_dir(&p).unwrap();
            if with_exe {
                std::fs::write(p.join(PLAYER_EXE), b"").unwrap();
            }
        };
        mk("version-aaaaaaaaaaaaaaaa", true);
        std::thread::sleep(std::time::Duration::from_millis(50));
        mk("version-bbbbbbbbbbbbbbbb", true);
        mk("version-cccccccccccccccc", false); // studio-only / partial folder
        mk("not-a-version", true);

        let found = scan_versions_dir(dir.path());
        let ids: Vec<_> = found.iter().map(|i| i.version_guid.as_str()).collect();
        assert_eq!(ids, ["version-bbbbbbbbbbbbbbbb", "version-aaaaaaaaaaaaaaaa"]);
    }

    #[test]
    fn missing_dir_is_empty() {
        assert!(scan_versions_dir(Path::new(r"Z:\definitely\missing")).is_empty());
    }
}
