use std::sync::Arc;

use crate::db::PictureRepository;

#[derive(Clone)]
pub struct AppState {
    pub pictures: Arc<PictureRepository>,
    pub api_token: String,
}
