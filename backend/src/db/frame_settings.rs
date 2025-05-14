use crate::CONFIG;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf, sync::Arc};
use tokio::sync::{RwLock, watch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSettings {
    pub display_enabled: bool,
    pub rotate_enabled: bool,
    pub rotate_interval_secs: u64,
    pub shuffle: bool,
}

pub struct SettingsStore {
    inner: RwLock<FrameSettings>,
    tx: watch::Sender<FrameSettings>,
}

#[derive(Clone)]
pub struct SharedSettings(Arc<SettingsStore>);

impl SharedSettings {
    /// Load from disk.
    pub fn load() -> io::Result<Self> {
        let settings_path = PathBuf::from(&CONFIG.backend_frame_settings_file);

        let initial: FrameSettings = if settings_path.exists() {
            let toml = fs::read_to_string(&settings_path)?;
            toml::from_str(&toml).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
        } else {
            // if let Some(parent) = settings_path.parent() {
            //     fs::create_dir_all(parent)?;
            // }

            let default = FrameSettings {
                display_enabled: true,
                rotate_enabled: true,
                rotate_interval_secs: 10,
                shuffle: false,
            };
            let toml_str = toml::to_string_pretty(&default)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            fs::write(&settings_path, toml_str)?;
            default
        };

        let (tx, _) = watch::channel(initial.clone());
        Ok(SharedSettings(Arc::new(SettingsStore {
            inner: RwLock::new(initial),
            tx,
        })))
    }

    /// Get a snapshot of the current settings.
    pub async fn get(&self) -> FrameSettings {
        self.0.inner.read().await.clone()
    }

    /// Subscribe to changes in settings. (Embedded)
    pub fn subscribe(&self) -> watch::Receiver<FrameSettings> {
        self.0.tx.subscribe()
    }

    /// Mutate in memory and write back to disk atomically.
    pub async fn update<F>(&self, mutator: F) -> io::Result<FrameSettings>
    where
        F: FnOnce(&mut FrameSettings),
    {
        let mut guard = self.0.inner.write().await;
        mutator(&mut guard);
        let new = guard.clone();

        let settings_path = PathBuf::from(&CONFIG.backend_frame_settings_file);
        let tmp = settings_path.with_extension("toml.tmp");
        let s =
            toml::to_string_pretty(&new).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(&tmp, s)?;
        fs::rename(&tmp, &settings_path)?;

        let _ = self.0.tx.send(new.clone());
        Ok(new)
    }
}
