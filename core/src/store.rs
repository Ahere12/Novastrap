//! Persistent JSON documents with atomic writes (write temp → rename) and
//! corrupt-file recovery (the bad file is kept as `*.corrupt` and defaults load).

use crate::error::AppResult;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct JsonStore<T> {
    path: PathBuf,
    value: Mutex<T>,
}

impl<T: Serialize + DeserializeOwned + Default + Clone> JsonStore<T> {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let value = Self::read(&path);
        JsonStore { path, value: Mutex::new(value) }
    }

    fn read(path: &Path) -> T {
        let Ok(text) = std::fs::read_to_string(path) else { return T::default() };
        match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{} is corrupt ({e}); keeping a copy and using defaults", path.display());
                let _ = std::fs::rename(path, path.with_extension("json.corrupt"));
                T::default()
            }
        }
    }

    pub fn get(&self) -> T {
        self.value.lock().unwrap().clone()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Applies `f` and persists atomically. The in-memory value only changes if the write succeeds.
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> AppResult<R> {
        let mut guard = self.value.lock().unwrap();
        let mut next = guard.clone();
        let r = f(&mut next);
        write_atomic(&self.path, &serde_json::to_vec_pretty(&next).expect("serializable"))?;
        *guard = next;
        Ok(r)
    }

    pub fn replace(&self, v: T) -> AppResult<()> {
        self.update(|cur| *cur = v)
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Default, Clone, PartialEq, Debug)]
    struct Doc {
        n: u32,
        #[serde(default)]
        added_later: Vec<String>,
    }

    #[test]
    fn persists_and_reloads() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("s.json");
        let s = JsonStore::<Doc>::open(&p);
        s.update(|v| v.n = 7).unwrap();
        assert_eq!(JsonStore::<Doc>::open(&p).get().n, 7);
    }

    #[test]
    fn missing_fields_default() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("s.json");
        std::fs::write(&p, r#"{"n":3}"#).unwrap();
        assert_eq!(JsonStore::<Doc>::open(&p).get(), Doc { n: 3, added_later: vec![] });
    }

    #[test]
    fn corrupt_file_recovers() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("s.json");
        std::fs::write(&p, "{not json").unwrap();
        assert_eq!(JsonStore::<Doc>::open(&p).get(), Doc::default());
        assert!(d.path().join("s.json.corrupt").exists());
    }
}
