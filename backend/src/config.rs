use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub data_dir: String,
    pub db_file: String,
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    envy::from_env::<Config>()
        .unwrap_or_else(|err| panic!("Failed to load configuration from env: {:#?}", err))
});
