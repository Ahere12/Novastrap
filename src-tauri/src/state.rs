//! Shared backend state. All persistent documents live in the app data folders.

use novastrap_core::accounts::AccountsFile;
use novastrap_core::activity::{HistoryFile, Session};
use novastrap_core::discord::Rpc;
use novastrap_core::favorites::FavoritesFile;
use novastrap_core::instances::{InstancesFile, RunningInstance};
use novastrap_core::mods::ModsFile;
use novastrap_core::performance::ProfilesFile;
use novastrap_core::settings::Settings;
use novastrap_core::store::JsonStore;
use novastrap_core::web::Api;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    pub session: Option<Session>,
    pub icon_url: Option<String>,
    pub discord_status: Option<String>,
}

pub struct AppState {
    pub api: Api,
    pub installing: AtomicBool,
    /// Roaming data (settings, profiles, favorites…).
    pub data_dir: PathBuf,
    /// Local data (downloads, mods, backgrounds).
    pub local_dir: PathBuf,
    pub log_dir: PathBuf,
    pub settings: JsonStore<Settings>,
    pub accounts: JsonStore<AccountsFile>,
    pub instances: JsonStore<InstancesFile>,
    pub favorites: JsonStore<FavoritesFile>,
    pub profiles: JsonStore<ProfilesFile>,
    pub mods: JsonStore<ModsFile>,
    pub history: JsonStore<HistoryFile>,
    pub launched: Mutex<Vec<RunningInstance>>,
    pub live: Mutex<Live>,
    pub discord: Mutex<Option<Rpc>>,
}

impl AppState {
    pub fn new(api: Api, data_dir: PathBuf, local_dir: PathBuf, log_dir: PathBuf) -> Self {
        let d = |n: &str| data_dir.join(n);
        AppState {
            api,
            installing: AtomicBool::new(false),
            settings: JsonStore::open(d("settings.json")),
            accounts: JsonStore::open(d("accounts.json")),
            instances: JsonStore::open(d("instances.json")),
            favorites: JsonStore::open(d("favorites.json")),
            profiles: JsonStore::open(d("performance.json")),
            mods: JsonStore::open(d("mods.json")),
            history: JsonStore::open(d("history.json")),
            launched: Mutex::new(Vec::new()),
            live: Mutex::new(Live::default()),
            discord: Mutex::new(None),
            data_dir,
            local_dir,
            log_dir,
        }
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.local_dir.join("mods")
    }
    pub fn downloads_dir(&self) -> PathBuf {
        self.local_dir.join("downloads")
    }
    pub fn backgrounds_dir(&self) -> PathBuf {
        self.local_dir.join("backgrounds")
    }
}
