use std::sync::Arc;

use libs::frame_settings::SharedSettings;

use crate::db::Repository;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<Repository>,
    pub settings: SharedSettings,
}
