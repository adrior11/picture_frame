use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrameSettings {
    pub display_enabled: bool,
    pub rotate_interval_secs: u64,
    pub shuffle: bool,
    pub pinned_image: Option<String>,
}

#[derive(Clone)]
pub struct SharedSettings {
    pub settings_store: Arc<RwLock<FrameSettings>>,
    pub file_path: String,
}

impl SharedSettings {
    /// Load from disk.
    pub fn load(file_path: &str) -> io::Result<Self> {
        let settings_path = PathBuf::from(file_path);

        let initial: FrameSettings = if settings_path.exists() {
            let toml = fs::read_to_string(&settings_path)?;
            toml::from_str(&toml).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        } else {
            let default = FrameSettings {
                display_enabled: true,
                rotate_interval_secs: 10,
                shuffle: false,
                pinned_image: None,
            };
            let toml_str = toml::to_string_pretty(&default).map_err(io::Error::other)?;
            fs::write(&settings_path, toml_str)?;
            default
        };

        Ok(SharedSettings {
            settings_store: Arc::new(RwLock::new(initial)),
            file_path: file_path.to_string(),
        })
    }

    /// Get a snapshot of the current settings.
    pub async fn get(&self) -> FrameSettings {
        self.settings_store.read().await.clone()
    }

    /// Mutate in memory and write back to disk atomically.
    pub async fn update<F>(&self, mutator: F) -> io::Result<FrameSettings>
    where
        F: FnOnce(&mut FrameSettings),
    {
        let mut guard = self.settings_store.write().await;
        mutator(&mut guard);
        let new = guard.clone();

        let settings_path = PathBuf::from(&self.file_path);
        let tmp = settings_path.with_extension("toml.tmp");
        let s = toml::to_string_pretty(&new).map_err(io::Error::other)?;
        fs::write(&tmp, s)?;
        fs::rename(&tmp, &settings_path)?;

        Ok(new)
    }
}
