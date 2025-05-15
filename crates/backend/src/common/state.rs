use std::sync::Arc;

use crate::db::{Repository, SharedSettings};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<Repository>,
    pub settings: SharedSettings,
}
