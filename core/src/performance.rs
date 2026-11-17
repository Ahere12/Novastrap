//! Performance center: allowlisted FastFlags, Roblox's own graphics/FPS settings,
//! profiles and presets. No performance claims are made: each option states what it changes.
//!
//! - FastFlags go to `<version>\ClientSettings\ClientAppSettings.json`. Since 2025-09-29
//!   the client ignores flags outside Roblox's allowlist, so only allowlisted ones are written.
//! - Frame-rate cap and graphics quality are the values Roblox's in-game settings menu
//!   stores in `%LOCALAPPDATA%\Roblox\GlobalBasicSettings_13.xml`.

use crate::error::{AppError, AppResult};
use crate::store::write_atomic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum FlagKind {
    Bool,
    Int { min: i64, max: i64 },
    Choice { options: &'static [i64] },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagSpec {
    pub name: &'static str,
    pub kind: FlagKind,
    pub group: &'static str,
    pub description: &'static str,
}

use FlagKind::*;

/// Flags on Roblox's local-configuration allowlist (DevForum, Sept 2025), as compiled by
/// the Sober/Soberix projects. The list is data so it can be updated without code changes.
pub const ALLOWLIST: &[FlagSpec] = &[
    FlagSpec { name: "DFIntCSGLevelOfDetailSwitchingDistance", kind: Int { min: 0, max: 1000 }, group: "Geometry", description: "Distance (studs) where union/CSG parts switch to lower detail." },
    FlagSpec { name: "DFIntCSGLevelOfDetailSwitchingDistanceL12", kind: Int { min: 0, max: 1000 }, group: "Geometry", description: "CSG detail switch distance, level 1→2." },
    FlagSpec { name: "DFIntCSGLevelOfDetailSwitchingDistanceL23", kind: Int { min: 0, max: 1000 }, group: "Geometry", description: "CSG detail switch distance, level 2→3." },
    FlagSpec { name: "DFIntCSGLevelOfDetailSwitchingDistanceL34", kind: Int { min: 0, max: 1000 }, group: "Geometry", description: "CSG detail switch distance, level 3→4." },
    FlagSpec { name: "DFFlagTextureQualityOverrideEnabled", kind: Bool, group: "Rendering", description: "Enables the texture quality override below." },
    FlagSpec { name: "DFIntTextureQualityOverride", kind: Int { min: 0, max: 3 }, group: "Rendering", description: "Texture quality level (0 lowest – 3 highest). Needs the override enabled." },
    FlagSpec { name: "FIntDebugForceMSAASamples", kind: Choice { options: &[1, 2, 4] }, group: "Rendering", description: "Anti-aliasing samples (1 = off)." },
    FlagSpec { name: "FFlagDebugSkyGray", kind: Bool, group: "Rendering", description: "Replaces the skybox with flat gray." },
    FlagSpec { name: "DFFlagDebugPauseVoxelizer", kind: Bool, group: "Rendering", description: "Pauses voxel lighting updates (lighting stops changing)." },
    FlagSpec { name: "DFIntDebugFRMQualityLevelOverride", kind: Int { min: 0, max: 21 }, group: "Rendering", description: "Forces a rendering quality level (0 = no override)." },
    FlagSpec { name: "FIntFRMMaxGrassDistance", kind: Int { min: 0, max: 1000 }, group: "Rendering", description: "Maximum distance (studs) grass is drawn. 0 hides grass." },
    FlagSpec { name: "FIntFRMMinGrassDistance", kind: Int { min: 0, max: 1000 }, group: "Rendering", description: "Minimum grass draw distance (studs)." },
    FlagSpec { name: "FFlagDebugGraphicsPreferVulkan", kind: Bool, group: "Graphics API", description: "Prefer the Vulkan renderer." },
    FlagSpec { name: "FFlagDebugGraphicsPreferOpenGL", kind: Bool, group: "Graphics API", description: "Prefer the OpenGL renderer." },
    FlagSpec { name: "FFlagHandleAltEnterFullscreenManually", kind: Bool, group: "Window", description: "Let Alt+Enter toggle fullscreen through the client's own handler." },
    FlagSpec { name: "DFFlagDisableDPIScale", kind: Bool, group: "Window", description: "Disable Windows DPI scaling for the client window." },
    FlagSpec { name: "FIntGrassMovementReducedMotionFactor", kind: Int { min: 0, max: 100 }, group: "Accessibility", description: "Reduces grass movement animation." },
];

pub fn spec(name: &str) -> Option<&'static FlagSpec> {
    ALLOWLIST.iter().find(|s| s.name == name)
}

/// Checks one flag value against the allowlist. Accepts Bloxstrap-style string values too.
pub fn validate_flag(name: &str, value: &Value) -> AppResult<Value> {
    let s = spec(name).ok_or_else(|| AppError::InvalidInput(format!("{name} isn't on Roblox's allowlist, so the client would ignore it")))?;
    let as_int = || value.as_i64().or_else(|| value.as_str().and_then(|v| v.trim().parse().ok()));
    match s.kind {
        Bool => value
            .as_bool()
            .or_else(|| value.as_str().and_then(|v| v.trim().to_ascii_lowercase().parse().ok()))
            .map(Value::Bool)
            .ok_or_else(|| AppError::InvalidInput(format!("{name} must be true or false"))),
        Int { min, max } => as_int()
            .filter(|n| (min..=max).contains(n))
            .map(Value::from)
            .ok_or_else(|| AppError::InvalidInput(format!("{name} must be a whole number from {min} to {max}"))),
        Choice { options } => as_int()
            .filter(|n| options.contains(n))
            .map(Value::from)
            .ok_or_else(|| AppError::InvalidInput(format!("{name} must be one of {options:?}"))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct PerformanceProfile {
    pub name: String,
    pub flags: BTreeMap<String, Value>,
    /// Roblox's own "Maximum Frame Rate" setting. Roblox does not support above 240.
    pub framerate_cap: Option<u32>,
    /// Roblox's graphics quality level (1–21; the in-game slider maps onto this).
    pub graphics_quality: Option<u8>,
}

pub const FRAMERATE_CAPS: &[u32] = &[30, 60, 120, 144, 165, 240];

impl PerformanceProfile {
    /// Validates every field. Returns the cleaned profile plus names of dropped flags.
    pub fn sanitize(self) -> AppResult<(Self, Vec<String>)> {
        let name = self.name.trim().to_owned();
        if name.is_empty() || name.len() > 40 {
            return Err(AppError::InvalidInput("Profile names must be 1–40 characters".into()));
        }
        if let Some(f) = self.framerate_cap {
            if !FRAMERATE_CAPS.contains(&f) {
                return Err(AppError::InvalidInput(format!("Frame rate cap must be one of {FRAMERATE_CAPS:?}")));
            }
        }
        if let Some(q) = self.graphics_quality {
            if !(1..=21).contains(&q) {
                return Err(AppError::InvalidInput("Graphics quality must be 1–21".into()));
            }
        }
        let mut dropped = Vec::new();
        let mut flags = BTreeMap::new();
        for (k, v) in self.flags {
            match validate_flag(&k, &v) {
                Ok(v) => {
                    flags.insert(k, v);
                }
                Err(_) => dropped.push(k),
            }
        }
        Ok((PerformanceProfile { name, flags, ..self }, dropped))
    }
}

pub fn presets() -> Vec<PerformanceProfile> {
    let p = |name: &str, flags: &[(&str, Value)]| PerformanceProfile {
        name: name.into(),
        flags: flags.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        framerate_cap: None,
        graphics_quality: None,
    };
    vec![
        p("Roblox defaults", &[]),
        p(
            "Lighter visuals",
            &[
                ("FIntDebugForceMSAASamples", Value::from(1)),
                ("DFFlagTextureQualityOverrideEnabled", Value::Bool(true)),
                ("DFIntTextureQualityOverride", Value::from(2)),
                ("FIntFRMMaxGrassDistance", Value::from(400)),
            ],
        ),
        p(
            "Minimal visuals",
            &[
                ("FIntDebugForceMSAASamples", Value::from(1)),
                ("DFFlagTextureQualityOverrideEnabled", Value::Bool(true)),
                ("DFIntTextureQualityOverride", Value::from(0)),
                ("FFlagDebugSkyGray", Value::Bool(true)),
                ("FIntFRMMaxGrassDistance", Value::from(0)),
                ("DFIntCSGLevelOfDetailSwitchingDistance", Value::from(1)),
            ],
        ),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfilesFile {
    pub profiles: Vec<PerformanceProfile>,
    pub active: Option<String>,
}

pub fn client_settings_path(version_dir: &Path) -> PathBuf {
    version_dir.join("ClientSettings").join("ClientAppSettings.json")
}

/// Writes the profile's flags (re-validated) for one installed version. Empty → removes the file.
pub fn write_flags(version_dir: &Path, profile: &PerformanceProfile) -> AppResult<usize> {
    let path = client_settings_path(version_dir);
    let flags: BTreeMap<_, _> =
        profile.flags.iter().filter_map(|(k, v)| validate_flag(k, v).ok().map(|v| (k.clone(), v))).collect();
    if flags.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(0);
    }
    write_atomic(&path, &serde_json::to_vec_pretty(&flags).expect("serializable"))?;
    Ok(flags.len())
}

pub fn read_flags(version_dir: &Path) -> BTreeMap<String, Value> {
    std::fs::read_to_string(client_settings_path(version_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ---- GlobalBasicSettings (Roblox's own settings) ----------------------------------

pub fn global_settings_path() -> Option<PathBuf> {
    let dir = PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Roblox");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("GlobalBasicSettings_") && n.ends_with(".xml") && !n.contains("Studio"))
        .collect();
    files.sort();
    files.pop().map(|n| dir.join(n))
}

/// Reads `<tag name="key">value</tag>` from the XML.
pub fn read_setting(xml: &str, key: &str) -> Option<String> {
    let needle = format!("name=\"{key}\">");
    let start = xml.find(&needle)? + needle.len();
    let end = xml[start..].find('<')? + start;
    Some(xml[start..end].to_string())
}

/// Replaces the value of an existing `<tag name="key">…</tag>`; errors if the key is absent
/// (Novastrap never invents settings Roblox didn't write itself).
pub fn replace_setting(xml: &str, key: &str, value: &str) -> AppResult<String> {
    let needle = format!("name=\"{key}\">");
    let start = xml.find(&needle).ok_or_else(|| AppError::Unsupported(format!("Roblox's settings file has no {key} yet. Open Roblox's settings once, then try again.")))? + needle.len();
    let end = xml[start..].find('<').ok_or_else(|| AppError::UpstreamChanged("malformed settings XML".into()))? + start;
    Ok(format!("{}{}{}", &xml[..start], value, &xml[end..]))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxSettings {
    pub framerate_cap: Option<u32>,
    pub graphics_quality: Option<u8>,
    pub path: Option<PathBuf>,
}

pub fn read_roblox_settings() -> RobloxSettings {
    let path = global_settings_path();
    let xml = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    RobloxSettings {
        framerate_cap: read_setting(&xml, "FramerateCap").and_then(|v| v.parse().ok()),
        graphics_quality: read_setting(&xml, "GraphicsQualityLevel").and_then(|v| v.parse().ok()),
        path,
    }
}

/// Applies FPS cap / graphics quality. Refuses while Roblox runs (it rewrites the file on exit).
/// Keeps a one-time backup of the original file next to it.
pub fn write_roblox_settings(framerate_cap: Option<u32>, graphics_quality: Option<u8>, roblox_running: bool) -> AppResult<()> {
    if framerate_cap.is_none() && graphics_quality.is_none() {
        return Ok(());
    }
    if roblox_running {
        return Err(AppError::Busy("Close Roblox first: it overwrites these settings when it exits".into()));
    }
    let path = global_settings_path().ok_or_else(|| AppError::Unsupported("Roblox's settings file wasn't found. Start Roblox once first.".into()))?;
    let mut xml = std::fs::read_to_string(&path)?;
    let backup = path.with_extension("xml.novastrap-backup");
    if !backup.exists() {
        std::fs::copy(&path, &backup)?;
    }
    if let Some(f) = framerate_cap {
        if !FRAMERATE_CAPS.contains(&f) {
            return Err(AppError::InvalidInput("unsupported frame rate cap".into()));
        }
        xml = replace_setting(&xml, "FramerateCap", &f.to_string())?;
    }
    if let Some(q) = graphics_quality {
        if !(1..=21).contains(&q) {
            return Err(AppError::InvalidInput("graphics quality must be 1–21".into()));
        }
        xml = replace_setting(&xml, "GraphicsQualityLevel", &q.to_string())?;
    }
    write_atomic(&path, xml.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_flags() {
        assert_eq!(validate_flag("DFIntTextureQualityOverride", &json!(2)).unwrap(), json!(2));
        assert_eq!(validate_flag("DFIntTextureQualityOverride", &json!("3")).unwrap(), json!(3));
        assert!(validate_flag("DFIntTextureQualityOverride", &json!(9)).is_err());
        assert_eq!(validate_flag("FFlagDebugSkyGray", &json!("True")).unwrap(), json!(true));
        assert!(validate_flag("FIntDebugForceMSAASamples", &json!(3)).is_err());
        assert!(validate_flag("DFIntTaskSchedulerTargetFps", &json!(9999)).is_err());
    }

    #[test]
    fn sanitize_drops_unknown() {
        let p = PerformanceProfile {
            name: " Mine ".into(),
            flags: BTreeMap::from([
                ("FFlagDebugSkyGray".into(), json!(true)),
                ("DFIntTaskSchedulerTargetFps".into(), json!(999)),
            ]),
            framerate_cap: Some(144),
            graphics_quality: Some(5),
        };
        let (clean, dropped) = p.sanitize().unwrap();
        assert_eq!(clean.name, "Mine");
        assert_eq!(clean.flags.len(), 1);
        assert_eq!(dropped, vec!["DFIntTaskSchedulerTargetFps"]);
        let bad = PerformanceProfile { name: "x".into(), framerate_cap: Some(999), ..Default::default() };
        assert!(bad.sanitize().is_err());
    }

    #[test]
    fn presets_are_valid() {
        for p in presets() {
            let (_, dropped) = p.sanitize().unwrap();
            assert!(dropped.is_empty());
        }
    }

    #[test]
    fn writes_and_clears_flags() {
        let d = tempfile::tempdir().unwrap();
        let p = presets().remove(1);
        assert_eq!(write_flags(d.path(), &p).unwrap(), 4);
        assert_eq!(read_flags(d.path()).len(), 4);
        assert_eq!(write_flags(d.path(), &presets()[0]).unwrap(), 0);
        assert!(!client_settings_path(d.path()).exists());
    }

    #[test]
    fn xml_settings() {
        let xml = r#"<Item><Properties><int name="FramerateCap">180</int><int name="GraphicsQualityLevel">21</int></Properties></Item>"#;
        assert_eq!(read_setting(xml, "FramerateCap").as_deref(), Some("180"));
        let out = replace_setting(xml, "FramerateCap", "144").unwrap();
        assert_eq!(read_setting(&out, "FramerateCap").as_deref(), Some("144"));
        assert_eq!(read_setting(&out, "GraphicsQualityLevel").as_deref(), Some("21"));
        assert!(replace_setting(xml, "Missing", "1").is_err());
    }

    #[test]
    fn refuses_while_running() {
        assert_eq!(write_roblox_settings(Some(60), None, true).unwrap_err().kind(), "busy");
    }
}
