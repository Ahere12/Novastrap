//! Client mods: replacing specific files in the installed version's `content` folder
//! (the Bloxstrap mod model). Never touches executables or DLLs.
//!
//! Reversible: before a file is replaced the original is copied to
//! `<mods dir>\backup\<version>\<relative path>`, and revert restores it.
//! Each mod is checked for compatibility: the target must exist in the installed
//! version and the replacement must have the right file signature and size.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    Png,
    Ogg,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Ogg => "ogg",
        }
    }
    pub fn matches(self, bytes: &[u8]) -> bool {
        match self {
            Format::Png => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            Format::Ogg => bytes.starts_with(b"OggS"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub relative_path: &'static str,
    pub format: Format,
    pub description: &'static str,
}

pub const SLOTS: &[Slot] = &[
    Slot { id: "cursor-arrow", name: "Arrow cursor", group: "Cursors", relative_path: r"content\textures\Cursors\KeyboardMouse\ArrowCursor.png", format: Format::Png, description: "Default mouse cursor in third person." },
    Slot { id: "cursor-arrow-far", name: "Arrow cursor (far)", group: "Cursors", relative_path: r"content\textures\Cursors\KeyboardMouse\ArrowFarCursor.png", format: Format::Png, description: "Cursor shown when hovering far away." },
    Slot { id: "cursor-ibeam", name: "Text cursor", group: "Cursors", relative_path: r"content\textures\Cursors\KeyboardMouse\IBeamCursor.png", format: Format::Png, description: "Cursor over text boxes." },
    Slot { id: "sound-death", name: "Death sound", group: "Sounds", relative_path: r"content\sounds\ouch.ogg", format: Format::Ogg, description: "Plays when a character dies (classic games)." },
];

pub const MAX_BYTES: u64 = 4 * 1024 * 1024;

pub fn slot(id: &str) -> AppResult<&'static Slot> {
    SLOTS
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::InvalidInput(format!("unknown mod slot '{id}'")))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ModsFile {
    /// slot id → stored replacement file name inside the mods dir.
    pub enabled: BTreeMap<String, String>,
}

/// Validates a candidate replacement and copies it into Novastrap's mods folder.
pub fn import(mods_dir: &Path, slot_id: &str, source: &Path) -> AppResult<String> {
    let s = slot(slot_id)?;
    let meta = std::fs::metadata(source)?;
    if meta.len() == 0 || meta.len() > MAX_BYTES {
        return Err(AppError::InvalidInput(format!("The file must be between 1 byte and {} MB", MAX_BYTES / 1_048_576)));
    }
    let bytes = std::fs::read(source)?;
    if !s.format.matches(&bytes) {
        return Err(AppError::InvalidInput(format!("{} needs a real .{} file", s.name, s.format.ext())));
    }
    std::fs::create_dir_all(mods_dir)?;
    let name = format!("{slot_id}.{}", s.format.ext());
    std::fs::write(mods_dir.join(&name), bytes)?;
    Ok(name)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Compatibility {
    pub slot_id: String,
    pub ok: bool,
    pub reason: Option<String>,
}

pub fn check(version_dir: &Path, slot_id: &str) -> Compatibility {
    let res = slot(slot_id).and_then(|s| {
        let target = version_dir.join(s.relative_path);
        if !target.is_file() {
            return Err(AppError::Unsupported(format!("this Roblox version has no {}", s.relative_path)));
        }
        Ok(())
    });
    Compatibility { slot_id: slot_id.into(), ok: res.is_ok(), reason: res.err().map(|e| e.to_string()) }
}

fn backup_path(mods_dir: &Path, version_dir: &Path, rel: &str) -> PathBuf {
    let version = version_dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    mods_dir.join("backup").join(version).join(rel)
}

/// Applies all enabled mods to one installed version. Returns applied slot ids.
pub fn apply_all(mods_dir: &Path, version_dir: &Path, file: &ModsFile) -> AppResult<Vec<String>> {
    let mut applied = Vec::new();
    for (slot_id, stored) in &file.enabled {
        let s = slot(slot_id)?;
        if !check(version_dir, slot_id).ok {
            log::warn!("skipping incompatible mod {slot_id}");
            continue;
        }
        let target = version_dir.join(s.relative_path);
        let backup = backup_path(mods_dir, version_dir, s.relative_path);
        if !backup.exists() {
            std::fs::create_dir_all(backup.parent().unwrap())?;
            std::fs::copy(&target, &backup)?;
        }
        std::fs::copy(mods_dir.join(stored), &target)?;
        applied.push(slot_id.clone());
    }
    Ok(applied)
}

/// Restores one slot's original file in the given version.
pub fn revert(mods_dir: &Path, version_dir: &Path, slot_id: &str) -> AppResult<bool> {
    let s = slot(slot_id)?;
    let backup = backup_path(mods_dir, version_dir, s.relative_path);
    if !backup.exists() {
        return Ok(false);
    }
    std::fs::copy(&backup, version_dir.join(s.relative_path))?;
    std::fs::remove_file(&backup)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

    fn fake_version(root: &Path) -> PathBuf {
        let v = root.join("version-aaaaaaaaaaaaaaaa");
        let f = v.join(r"content\textures\Cursors\KeyboardMouse");
        std::fs::create_dir_all(&f).unwrap();
        std::fs::write(f.join("ArrowCursor.png"), b"\x89PNG\r\n\x1a\noriginal").unwrap();
        v
    }

    #[test]
    fn rejects_wrong_format_and_unknown_slot() {
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("x.png");
        std::fs::write(&src, b"GIF89a").unwrap();
        assert!(import(&d.path().join("mods"), "cursor-arrow", &src).is_err());
        assert!(import(&d.path().join("mods"), "nope", &src).is_err());
    }

    #[test]
    fn apply_and_revert_round_trip() {
        let d = tempfile::tempdir().unwrap();
        let mods = d.path().join("mods");
        let version = fake_version(d.path());
        let src = d.path().join("c.png");
        std::fs::write(&src, PNG).unwrap();
        let stored = import(&mods, "cursor-arrow", &src).unwrap();
        let file = ModsFile { enabled: BTreeMap::from([("cursor-arrow".into(), stored)]) };
        let target = version.join(r"content\textures\Cursors\KeyboardMouse\ArrowCursor.png");

        assert_eq!(apply_all(&mods, &version, &file).unwrap(), vec!["cursor-arrow"]);
        assert_eq!(std::fs::read(&target).unwrap(), PNG);
        // Applying twice keeps the ORIGINAL backup.
        apply_all(&mods, &version, &file).unwrap();
        assert!(revert(&mods, &version, "cursor-arrow").unwrap());
        assert_eq!(std::fs::read(&target).unwrap(), b"\x89PNG\r\n\x1a\noriginal");
        assert!(!revert(&mods, &version, "cursor-arrow").unwrap());
    }

    #[test]
    fn compatibility() {
        let d = tempfile::tempdir().unwrap();
        let v = fake_version(d.path());
        assert!(check(&v, "cursor-arrow").ok);
        assert!(!check(&v, "cursor-ibeam").ok);
    }
}
