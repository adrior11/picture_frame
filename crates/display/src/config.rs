use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub backend_data_dir: String,
    pub backend_frame_settings_file: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    envy::from_env::<Config>()
        .unwrap_or_else(|err| panic!("Failed to load configuration from env: {:#?}", err))
});
