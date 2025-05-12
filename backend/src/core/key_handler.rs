use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing,
};

use crate::db::dto::{KeyCreateRequest, KeyInfo};

use super::{auth::ApiKey, state::AppState};

pub fn key_routes() -> Router<AppState> {
    Router::new()
        .route("/api/keys", routing::get(list_keys).post(create_key))
        .route("/api/keys/{id}", routing::delete(revoke_key))
}

async fn list_keys(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
) -> Result<Json<Vec<KeyInfo>>, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }
    let keys = state
        .repo
        .list_api_keys()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(keys))
}

async fn create_key(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    Json(payload): Json<KeyCreateRequest>,
) -> Result<Json<String>, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }
    if payload.scope != "ro" && payload.scope != "rw" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // returns the new plaintext secret once
    let secret = state
        .repo
        .create_api_key_and_return_secret(&payload.id, &payload.scope)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(secret))
}

async fn revoke_key(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }
    let deleted = state
        .repo
        .delete_api_key(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(if deleted {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    })
}
