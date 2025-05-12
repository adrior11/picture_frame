use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::db::PictureDto;

use super::state::AppState;

pub async fn ping() -> &'static str {
    "pong"
}

pub async fn list_pictures(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PictureDto>>, StatusCode> {
    auth(&headers, &state.api_token).await?;
    let pics = state
        .pictures
        .list()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(pics))
}

pub async fn add_picture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(dto): Json<PictureDto>, // NOTE: just the filename as of now
) -> Result<impl IntoResponse, StatusCode> {
    auth(&headers, &state.api_token).await?;
    let saved = state
        .pictures
        .add(&dto.filename)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(saved)))
}

pub async fn delete_picture(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    auth(&headers, &state.api_token).await?;
    let deleted = state
        .pictures
        .delete(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn auth(headers: &HeaderMap, token: &str) -> Result<(), StatusCode> {
    match headers.get("X-Auth-Token").and_then(|v| v.to_str().ok()) {
        Some(t) if t == token => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
