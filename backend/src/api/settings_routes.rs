use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use serde::Deserialize;

use crate::{common::AppState, db::FrameSettings};

#[derive(Deserialize)]
pub struct PartialSettings {
    pub display_enabled: Option<bool>,
    pub rotate_enabled: Option<bool>,
    pub rotate_interval_secs: Option<u64>,
    pub shuffle: Option<bool>,
}

pub fn settings_routes() -> Router<AppState> {
    Router::new().route("/api/settings", get(get_settings).patch(patch_settings))
}

async fn get_settings(State(state): State<AppState>) -> Json<FrameSettings> {
    Json(state.settings.get().await)
}

async fn patch_settings(
    State(state): State<AppState>,
    Json(chg): Json<PartialSettings>,
) -> Result<Json<FrameSettings>, StatusCode> {
    state
        .settings
        .update(|s| {
            if let Some(v) = chg.display_enabled {
                s.display_enabled = v;
            }
            if let Some(v) = chg.rotate_enabled {
                s.rotate_enabled = v;
            }
            if let Some(v) = chg.rotate_interval_secs {
                s.rotate_interval_secs = v;
            }
            if let Some(v) = chg.shuffle {
                s.shuffle = v;
            }
        })
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
