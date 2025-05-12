use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_api_token")]
    pub api_token: String,
    pub data_dir: String,
    pub db_file: String,
}

fn default_api_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    envy::from_env::<Config>()
        .unwrap_or_else(|err| panic!("Failed to load configuration from env: {:#?}", err))
});
