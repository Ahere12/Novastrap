//! Instance manager: saved instance profiles (name, target, performance profile),
//! launching them, and read-only process monitoring (status, memory, close).
//!
//! Running several clients at once is NOT implemented: on client 0.747 the second
//! client handed off to the first in testing, both with Roblox's single-instance
//! mutex held and with a mutex squatting the event name. See docs/FEATURES.md.

use crate::error::{AppError, AppResult};
use crate::roblox::launch::LaunchTarget;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub const CLIENT_EXE: &str = "RobloxPlayerBeta.exe";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceProfile {
    pub id: String,
    pub name: String,
    /// Place ID / link, or empty for the Roblox home screen.
    pub target: String,
    /// Performance profile applied before launch (module 6).
    pub performance_profile: Option<String>,
}

impl InstanceProfile {
    pub fn validate(&self) -> AppResult<LaunchTarget> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > 40 {
            return Err(AppError::InvalidInput("Instance names must be 1–40 characters".into()));
        }
        LaunchTarget::parse_user_input(&self.target)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct InstancesFile {
    pub profiles: Vec<InstanceProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunningInstance {
    pub pid: u32,
    /// Saved profile this was started from (None = started outside Novastrap or ad hoc).
    pub profile_id: Option<String>,
    pub name: String,
    pub started_at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientProcess {
    pub pid: u32,
    pub memory_bytes: u64,
    pub run_time_secs: u64,
    /// Present when Novastrap launched this process.
    pub instance: Option<RunningInstance>,
}

/// Lists running Roblox clients with memory use, matched against instances we launched.
pub fn client_processes(launched: &[RunningInstance]) -> Vec<ClientProcess> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_memory());
    let mut out: Vec<ClientProcess> = sys
        .processes()
        .values()
        .filter(|p| p.name().eq_ignore_ascii_case(CLIENT_EXE))
        .map(|p| ClientProcess {
            pid: p.pid().as_u32(),
            memory_bytes: p.memory(),
            run_time_secs: p.run_time(),
            instance: launched.iter().find(|i| i.pid == p.pid().as_u32()).cloned(),
        })
        .collect();
    out.sort_by_key(|p| p.pid);
    out
}

/// Drops launched entries whose process has exited.
pub fn prune_exited(launched: &mut Vec<RunningInstance>, running: &[ClientProcess]) {
    launched.retain(|i| running.iter().any(|p| p.pid == i.pid));
}

/// Closes a Roblox client. Refuses any PID that isn't a Roblox client process.
pub fn close_client(pid: u32) -> AppResult<()> {
    let mut sys = System::new();
    let p = Pid::from_u32(pid);
    sys.refresh_processes(ProcessesToUpdate::Some(&[p]), true);
    let proc_ = sys.process(p).ok_or_else(|| AppError::InvalidInput(format!("process {pid} has already exited")))?;
    if !proc_.name().eq_ignore_ascii_case(CLIENT_EXE) {
        return Err(AppError::InvalidInput(format!("process {pid} is not a Roblox client")));
    }
    if proc_.kill() { Ok(()) } else { Err(AppError::Launch(format!("couldn't close process {pid}"))) }
}

pub fn new_profile_id(now: u64, existing: &[InstanceProfile]) -> String {
    let mut n = now;
    loop {
        let id = format!("inst-{n:x}");
        if !existing.iter().any(|p| p.id == id) {
            return id;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str, target: &str) -> InstanceProfile {
        InstanceProfile { id: "x".into(), name: name.into(), target: target.into(), performance_profile: None }
    }

    #[test]
    fn validates_profiles() {
        assert!(profile("Main", "").validate().is_ok());
        assert!(profile("Alt", "920587237").validate().is_ok());
        assert!(profile("", "").validate().is_err());
        assert!(profile(&"x".repeat(41), "").validate().is_err());
        assert!(profile("Bad", "https://evil.example/games/1").validate().is_err());
    }

    #[test]
    fn prunes_exited() {
        let ri = |pid| RunningInstance { pid, profile_id: None, name: "n".into(), started_at: 0 };
        let mut launched = vec![ri(1), ri(2)];
        let running = vec![ClientProcess { pid: 2, memory_bytes: 0, run_time_secs: 0, instance: None }];
        prune_exited(&mut launched, &running);
        assert_eq!(launched, vec![ri(2)]);
    }

    #[test]
    fn unique_ids() {
        let a = new_profile_id(10, &[]);
        let b = new_profile_id(10, &[InstanceProfile { id: a.clone(), ..profile("a", "") }]);
        assert_ne!(a, b);
    }

    #[test]
    fn refuses_to_close_non_roblox() {
        let me = std::process::id();
        assert_eq!(close_client(me).unwrap_err().kind(), "invalid_input");
    }
}
