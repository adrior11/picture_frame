use once_cell::sync::Lazy;
use serde::Deserialize;

use libs::util;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub backend_data_dir: String,
    pub backend_frame_settings_file: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let mut config = envy::from_env::<Config>()
        .unwrap_or_else(|err| panic!("Failed to load configuration from env: {:#?}", err));

    let config_dir = util::get_config_dir();

    let backend_data_dir = config_dir
        .join(&config.backend_data_dir)
        .to_string_lossy()
        .into_owned();
    let backend_frame_settings_file = config_dir
        .join(&config.backend_frame_settings_file)
        .to_string_lossy()
        .into_owned();

    std::fs::create_dir_all(&backend_data_dir).expect("Failed to create data directory");

    // update the config with the full paths
    config.backend_data_dir = backend_data_dir;
    config.backend_frame_settings_file = backend_frame_settings_file;

    config
});
