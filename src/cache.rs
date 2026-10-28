use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::model::Provider;

#[derive(Default, Serialize, Deserialize)]
pub struct Cache(HashMap<String, Provider>);

impl Cache {
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn get(&self, key: &str) -> Option<&Provider> {
        self.0.get(key)
    }

    pub fn put(&mut self, key: &str, provider: Provider) {
        self.0.insert(key.to_string(), provider);
    }

    pub fn save(&self) {
        let Some(path) = path() else { return };
        let Some(dir) = path.parent() else { return };
        let Ok(bytes) = serde_json::to_vec(self) else {
            return;
        };
        let tmp = path.with_extension("tmp");
        if std::fs::create_dir_all(dir).is_ok() && std::fs::write(&tmp, bytes).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

fn path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Caches/tokenleft/cache.json"))
}
