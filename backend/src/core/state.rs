use std::sync::Arc;

use crate::db::Repository;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<Repository>,
}
