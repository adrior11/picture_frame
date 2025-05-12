use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};

use crate::db::dto::Picture;

use super::{auth::ApiKey, state::AppState};

pub fn picture_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/pictures",
            routing::get(list_pictures).post(add_picture),
        )
        .route("/api/pictures/{id}", routing::delete(delete_picture))
}

async fn list_pictures(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
) -> Result<Json<Vec<Picture>>, StatusCode> {
    if scope != "ro" && scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }

    let pics = state
        .repo
        .list_pictures()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(pics))
}

async fn add_picture(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    Json(dto): Json<Picture>, // NOTE: just the filename as of now
) -> Result<impl IntoResponse, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }

    let saved = state
        .repo
        .add_picture(&dto.filename)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(saved)))
}

async fn delete_picture(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }

    let deleted = state
        .repo
        .delete_picture(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
