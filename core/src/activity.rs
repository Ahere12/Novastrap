//! Activity tracking from the Roblox client's own log files (read-only, like Bloxstrap).
//! Feeds recently played, playtime statistics, the second-screen HUD and Discord presence.
//!
//! Sessions start at "! Joining game" and end at the next join, a disconnect line,
//! or when no Roblox client is running any more.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    Joining { job_id: String, place_id: u64, server_ip: Option<String> },
    JoinReport { place_id: u64, universe_id: Option<u64>, user_id: Option<u64> },
    Left,
}

fn field<'a>(s: &'a str, key: &str) -> Option<&'a str> {
    let start = s.find(key)? + key.len();
    Some(s[start..].split([',', ' ']).next().unwrap_or("").trim())
}

/// Parses `2026-11-17T01:40:21.669Z,...` into Unix seconds.
pub fn parse_timestamp(line: &str) -> Option<u64> {
    let t = line.get(..19)?;
    let n = |r: std::ops::Range<usize>| t.get(r)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, s) = (n(0..4)?, n(5..7)?, n(8..10)?, n(11..13)?, n(14..16)?, n(17..19)?);
    if &t[4..5] != "-" || &t[10..11] != "T" {
        return None;
    }
    // Days from civil (Howard Hinnant).
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    u64::try_from(days * 86400 + h * 3600 + mi * 60 + s).ok()
}

pub fn parse_line(line: &str) -> Option<LogEvent> {
    if let Some(i) = line.find("! Joining game '") {
        let rest = &line[i + "! Joining game '".len()..];
        let (job, rest) = rest.split_once('\'')?;
        let place_id = field(rest, "place ")?.parse().ok()?;
        let server_ip = field(rest, " at ").filter(|ip| ip.parse::<std::net::Ipv4Addr>().is_ok()).map(str::to_owned);
        if !crate::roblox::launch::is_valid_job_id(job) {
            return None;
        }
        return Some(LogEvent::Joining { job_id: job.to_owned(), place_id, server_ip });
    }
    if line.contains("Report game_join_loadtime:") {
        return Some(LogEvent::JoinReport {
            place_id: field(line, "placeid:")?.parse().ok()?,
            universe_id: field(line, "universeid:").and_then(|v| v.parse().ok()),
            user_id: field(line, "userid:").and_then(|v| v.parse().ok()),
        });
    }
    if line.contains("[FLog::Network] Time to disconnect replication data") || line.contains("leaveUGCGameInternal") {
        return Some(LogEvent::Left);
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub place_id: u64,
    pub universe_id: Option<u64>,
    pub job_id: String,
    /// Only kept in memory for the HUD; cleared before saving to history.
    #[serde(skip)]
    pub server_ip: Option<String>,
    pub user_id: Option<u64>,
    pub started_at: u64,
    pub ended_at: Option<u64>,
    #[serde(default)]
    pub game_name: Option<String>,
}

impl Session {
    pub fn duration(&self, now: u64) -> u64 {
        self.ended_at.unwrap_or(now).saturating_sub(self.started_at)
    }
}

#[derive(Debug, Default)]
pub struct Tracker {
    pub current: Option<Session>,
}

impl Tracker {
    /// Feeds one event. Returns a session that just ended, if any.
    pub fn feed(&mut self, ev: LogEvent, ts: u64) -> Option<Session> {
        match ev {
            LogEvent::Joining { job_id, place_id, server_ip } => {
                let ended = self.end(ts);
                self.current = Some(Session {
                    place_id,
                    universe_id: None,
                    job_id,
                    server_ip,
                    user_id: None,
                    started_at: ts,
                    ended_at: None,
                    game_name: None,
                });
                ended
            }
            LogEvent::JoinReport { place_id, universe_id, user_id } => {
                if let Some(c) = self.current.as_mut().filter(|c| c.place_id == place_id) {
                    c.universe_id = universe_id.or(c.universe_id);
                    c.user_id = user_id.or(c.user_id);
                }
                None
            }
            LogEvent::Left => self.end(ts),
        }
    }

    pub fn end(&mut self, ts: u64) -> Option<Session> {
        self.current.take().map(|mut s| {
            s.ended_at = Some(ts.max(s.started_at));
            s.server_ip = None;
            s
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct HistoryFile {
    pub sessions: Vec<Session>,
}

pub const MAX_SESSIONS: usize = 1000;

impl HistoryFile {
    /// Adds a finished session (deduplicated by job id), newest first.
    pub fn record(&mut self, s: Session) {
        if self.sessions.iter().any(|x| x.job_id == s.job_id && x.started_at == s.started_at) {
            return;
        }
        self.sessions.insert(0, s);
        self.sessions.truncate(MAX_SESSIONS);
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameStat {
    pub place_id: u64,
    pub universe_id: Option<u64>,
    pub game_name: Option<String>,
    pub sessions: u32,
    pub seconds: u64,
    pub last_played: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayStats {
    pub total_seconds: u64,
    pub last_7_days_seconds: u64,
    pub sessions: u32,
    /// Recently played, most recent first (one entry per game).
    pub recent: Vec<GameStat>,
    /// Most played by time.
    pub top: Vec<GameStat>,
}

pub fn stats(history: &HistoryFile, now: u64) -> PlayStats {
    let mut by_game: HashMap<u64, GameStat> = HashMap::new();
    let (mut total, mut week) = (0, 0);
    for s in &history.sessions {
        let d = s.duration(now);
        total += d;
        if s.started_at + 7 * 86400 >= now {
            week += d;
        }
        let g = by_game.entry(s.place_id).or_insert(GameStat {
            place_id: s.place_id,
            universe_id: s.universe_id,
            game_name: s.game_name.clone(),
            sessions: 0,
            seconds: 0,
            last_played: 0,
        });
        g.sessions += 1;
        g.seconds += d;
        g.last_played = g.last_played.max(s.started_at);
        g.universe_id = g.universe_id.or(s.universe_id);
        g.game_name = g.game_name.clone().or(s.game_name.clone());
    }
    let mut recent: Vec<GameStat> = by_game.values().cloned().collect();
    recent.sort_by(|a, b| b.last_played.cmp(&a.last_played));
    let mut top = recent.clone();
    top.sort_by(|a, b| b.seconds.cmp(&a.seconds));
    recent.truncate(12);
    top.truncate(5);
    PlayStats { total_seconds: total, last_7_days_seconds: week, sessions: history.sessions.len() as u32, recent, top }
}

pub fn logs_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Roblox").join("logs"))
}

/// Newest player log (crash-handler logs excluded).
pub fn newest_player_log(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.contains("_Player_") && !n.contains("CrashHandler") && n.ends_with(".log")
        })
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
}

/// Incremental reader for a growing log file.
#[derive(Debug, Default)]
pub struct LogTail {
    pub path: Option<PathBuf>,
    offset: u64,
    partial: String,
}

impl LogTail {
    /// Switches to `path` (reading from the start, or from the end when `from_end`).
    pub fn follow(&mut self, path: PathBuf, from_end: bool) {
        if self.path.as_ref() == Some(&path) {
            return;
        }
        self.offset = if from_end { std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) } else { 0 };
        self.partial.clear();
        self.path = Some(path);
    }

    /// Returns complete new lines since the last call.
    pub fn read_lines(&mut self) -> Vec<String> {
        let Some(path) = &self.path else { return vec![] };
        let Ok(mut f) = std::fs::File::open(path) else { return vec![] };
        let len = f.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            self.offset = 0; // truncated/rotated
        }
        if f.seek(SeekFrom::Start(self.offset)).is_err() {
            return vec![];
        }
        let mut buf = Vec::new();
        let _ = f.take(4 * 1024 * 1024).read_to_end(&mut buf);
        self.offset += buf.len() as u64;
        self.partial.push_str(&String::from_utf8_lossy(&buf));
        let mut lines: Vec<String> = self.partial.split('\n').map(|l| l.trim_end_matches('\r').to_owned()).collect();
        self.partial = lines.pop().unwrap_or_default();
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOIN: &str = "2026-11-17T01:40:21.669Z,1.0,abcd,6 [FLog::Output] ! Joining game '72968fb3-0a9d-42e2-a00c-626f368f5336' place 82670836075946 at 128.116.1.2";
    const REPORT: &str = "2026-11-17T01:40:22.000Z,1.0,abcd,6 [FLog::GameJoinLoadTime] Report game_join_loadtime: placeid:82670836075946, join_time:0.49, universeid:10773735871, referral_page:, sid:x, clienttime:1.0, userid:42, ";

    #[test]
    fn timestamps() {
        assert_eq!(parse_timestamp("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_timestamp("2026-11-17T01:40:21.669Z,x"), Some(1794879621));
        assert_eq!(parse_timestamp("garbage"), None);
    }

    #[test]
    fn parses_lines() {
        assert_eq!(
            parse_line(JOIN),
            Some(LogEvent::Joining {
                job_id: "72968fb3-0a9d-42e2-a00c-626f368f5336".into(),
                place_id: 82670836075946,
                server_ip: Some("128.116.1.2".into())
            })
        );
        assert_eq!(
            parse_line(REPORT),
            Some(LogEvent::JoinReport { place_id: 82670836075946, universe_id: Some(10773735871), user_id: Some(42) })
        );
        assert_eq!(parse_line("[FLog::Network] Time to disconnect replication data: 0.1"), Some(LogEvent::Left));
        assert_eq!(parse_line("[FLog::Output] something else"), None);
        assert_eq!(parse_line("! Joining game 'not-a-uuid' place 1 at 1.1.1.1"), None);
    }

    #[test]
    fn tracker_sessions() {
        let mut t = Tracker::default();
        assert!(t.feed(parse_line(JOIN).unwrap(), 100).is_none());
        t.feed(parse_line(REPORT).unwrap(), 101);
        assert_eq!(t.current.as_ref().unwrap().universe_id, Some(10773735871));
        let ended = t.feed(LogEvent::Left, 160).unwrap();
        assert_eq!(ended.duration(999), 60);
        assert!(ended.server_ip.is_none(), "IP is not persisted");
        assert!(t.current.is_none());
    }

    #[test]
    fn history_and_stats() {
        let s = |place, job: &str, start, end| Session {
            place_id: place,
            universe_id: None,
            job_id: job.into(),
            server_ip: None,
            user_id: None,
            started_at: start,
            ended_at: Some(end),
            game_name: None,
        };
        let mut h = HistoryFile::default();
        h.record(s(1, "a", 0, 100));
        h.record(s(1, "a", 0, 100)); // duplicate ignored
        h.record(s(2, "b", 1_000_000, 1_000_300));
        h.record(s(1, "c", 1_000_400, 1_000_500));
        let st = stats(&h, 1_000_600);
        assert_eq!(st.sessions, 3);
        assert_eq!(st.total_seconds, 500);
        assert_eq!(st.last_7_days_seconds, 400);
        assert_eq!(st.recent[0].place_id, 1);
        assert_eq!(st.top[0].place_id, 2);
    }

    #[test]
    fn tail_reads_incrementally() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x_Player_1_last.log");
        std::fs::write(&p, "one\ntw").unwrap();
        let mut t = LogTail::default();
        t.follow(p.clone(), false);
        assert_eq!(t.read_lines(), vec!["one"]);
        use std::io::Write;
        std::fs::OpenOptions::new().append(true).open(&p).unwrap().write_all(b"o\r\nthree\n").unwrap();
        assert_eq!(t.read_lines(), vec!["two", "three"]);
        assert_eq!(newest_player_log(d.path()), Some(p));
    }
}

#[cfg(test)]
mod live {
    use super::*;

    /// Replays the newest real client log through the tracker. `cargo test live_log -- --ignored`
    #[test]
    #[ignore]
    fn live_log_replay() {
        let log = newest_player_log(&logs_dir().unwrap()).expect("no player log");
        let mut tail = LogTail::default();
        tail.follow(log, false);
        let (mut t, mut ended, mut last_ts) = (Tracker::default(), 0, 0);
        for line in tail.read_lines() {
            let ts = parse_timestamp(&line).unwrap_or(last_ts);
            last_ts = ts;
            if let Some(ev) = parse_line(&line) {
                if t.feed(ev, ts).is_some() {
                    ended += 1;
                }
            }
        }
        let open = t.end(last_ts);
        println!("ended sessions: {ended}, open session at end: {}, universe known: {:?}", open.is_some(), open.as_ref().map(|s| s.universe_id.is_some()));
    }
}
