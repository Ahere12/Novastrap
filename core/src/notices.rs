//! Notification center: a persisted log of what Novastrap told the user.

use serde::{Deserialize, Serialize};

pub const MAX_NOTICES: usize = 200;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NoticeKind {
    Update,
    Game,
    Launch,
    Error,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Notice {
    pub id: u64,
    pub at: u64,
    pub kind: NoticeKind,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct NoticesFile {
    /// Newest first.
    pub items: Vec<Notice>,
}

impl NoticesFile {
    /// Adds a notice (newest first, capped). Text is trimmed and length-limited.
    pub fn push(&mut self, at: u64, kind: NoticeKind, title: &str, body: &str) -> Notice {
        let id = self.items.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        let clip = |s: &str, n: usize| s.trim().chars().take(n).collect::<String>();
        let n = Notice { id, at, kind, title: clip(title, 120), body: clip(body, 500), read: false };
        self.items.insert(0, n.clone());
        self.items.truncate(MAX_NOTICES);
        n
    }

    pub fn unread(&self) -> usize {
        self.items.iter().filter(|n| !n.read).count()
    }

    pub fn mark_all_read(&mut self) {
        self.items.iter_mut().for_each(|n| n.read = true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_orders_caps_and_counts() {
        let mut f = NoticesFile::default();
        for i in 0..(MAX_NOTICES + 5) {
            f.push(i as u64, NoticeKind::Info, &format!("n{i}"), "");
        }
        assert_eq!(f.items.len(), MAX_NOTICES);
        assert_eq!(f.items[0].title, format!("n{}", MAX_NOTICES + 4), "newest first");
        assert_eq!(f.unread(), MAX_NOTICES);
        f.mark_all_read();
        assert_eq!(f.unread(), 0);
        let n = f.push(1, NoticeKind::Error, "  x  ", &"b".repeat(900));
        assert_eq!(n.title, "x");
        assert_eq!(n.body.len(), 500);
        assert!(n.id > f.items[1].id, "ids keep increasing");
    }
}
