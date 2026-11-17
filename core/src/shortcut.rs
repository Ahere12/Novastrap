//! Global keyboard shortcut strings ("Ctrl+Alt+N"): parsing and normalization.
//!
//! Only combinations with at least one modifier and a single letter, digit or F-key are accepted,
//! so a shortcut can never swallow plain typing system-wide.

use crate::error::{AppError, AppResult};

const MODS: &[(&str, &str)] = &[("ctrl", "Ctrl"), ("control", "Ctrl"), ("alt", "Alt"), ("shift", "Shift"), ("super", "Super"), ("win", "Super")];
const ORDER: &[&str] = &["Ctrl", "Alt", "Shift", "Super"];

/// Returns the canonical form, e.g. " alt + ctrl+n" → "Ctrl+Alt+N".
pub fn normalize(input: &str) -> AppResult<String> {
    let bad = |m: &str| AppError::InvalidInput(format!("'{}': {m}", input.trim()));
    let mut mods: Vec<&str> = Vec::new();
    let mut key: Option<String> = None;
    for part in input.split('+').map(str::trim) {
        if part.is_empty() {
            return Err(bad("empty part"));
        }
        let lower = part.to_ascii_lowercase();
        if let Some((_, m)) = MODS.iter().find(|(k, _)| *k == lower) {
            if mods.contains(m) {
                return Err(bad("modifier repeated"));
            }
            mods.push(m);
        } else if key.is_some() {
            return Err(bad("only one key is allowed"));
        } else {
            key = Some(key_name(part).ok_or_else(|| bad("use a letter, digit or F1–F24"))?);
        }
    }
    let key = key.ok_or_else(|| bad("add a key after the modifiers"))?;
    if mods.is_empty() {
        return Err(bad("needs at least one of Ctrl, Alt, Shift or Win"));
    }
    if mods == ["Shift"] {
        return Err(bad("Shift alone would block typing capitals; add Ctrl or Alt"));
    }
    mods.sort_by_key(|m| ORDER.iter().position(|o| o == m));
    Ok(format!("{}+{key}", mods.join("+")))
}

fn key_name(k: &str) -> Option<String> {
    let up = k.to_ascii_uppercase();
    if up.len() == 1 && up.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(up);
    }
    let n: u8 = up.strip_prefix('F')?.parse().ok()?;
    (1..=24).contains(&n).then(|| format!("F{n}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes() {
        assert_eq!(normalize(" alt + ctrl+n").unwrap(), "Ctrl+Alt+N");
        assert_eq!(normalize("Win+Shift+f12").unwrap(), "Shift+Super+F12");
        assert_eq!(normalize("Control+1").unwrap(), "Ctrl+1");
    }

    #[test]
    fn rejects() {
        for s in ["N", "Ctrl", "Ctrl+", "Ctrl+Ctrl+N", "Ctrl+N+M", "Shift+N", "Ctrl+F25", "Ctrl+Space", ""] {
            assert!(normalize(s).is_err(), "{s} should be rejected");
        }
    }
}
