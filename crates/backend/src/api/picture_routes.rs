use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing,
};
use futures::StreamExt;
use mime::{IMAGE_JPEG, IMAGE_PNG, Mime};
use tokio::io::AsyncWriteExt;

use crate::{
    CONFIG,
    common::{ApiKey, AppState},
    db::Picture,
};

pub fn picture_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/pictures",
            routing::get(list_pictures).post(upload_picture),
        )
        .route(
            "/api/pictures/{id}/pin",
            routing::put(pin_picture).delete(unpin_picture),
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

async fn upload_picture(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut field = multipart
        .next_field()
        .await
        .map_err(|e| {
            tracing::warn!("bad multipart: {e}");
            StatusCode::BAD_REQUEST
        })?
        .ok_or(StatusCode::BAD_REQUEST)?;

    if field.name() != Some("file") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let ct: Mime = field
        .content_type()
        .ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?
        .parse()
        .map_err(|_| StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

    let ext = if ct == IMAGE_JPEG {
        "jpg"
    } else if ct == IMAGE_PNG {
        "png"
    } else {
        return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    };

    let file_id = uuid::Uuid::new_v4().to_string();
    let filename = format!("{file_id}.{ext}");

    let data_dir = std::path::Path::new(&CONFIG.backend_data_dir);
    if !data_dir.exists() {
        tokio::fs::create_dir_all(data_dir).await.map_err(|e| {
            tracing::error!("cannot create data_dir {data_dir:?}: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }
    let path = data_dir.join(&filename);
    tracing::debug!("saving {}", path.display());

    let mut dest = tokio::fs::File::create(&path).await.map_err(|e| {
        tracing::error!("cannot open {path:?}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    while let Some(chunk) = field.next().await {
        let bytes = chunk.map_err(|e| {
            tracing::warn!("multipart read error: {e}");
            StatusCode::BAD_REQUEST
        })?;
        if let Err(e) = dest.write_all(&bytes).await {
            tracing::error!("write error on {path:?}: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
    dest.flush().await.ok(); // ignore flush error; already logged
    dest.sync_all().await.ok();

    let saved = state.repo.add_picture(&filename).await.map_err(|e| {
        tracing::error!("db error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

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

    let fname_opt = state
        .repo
        .delete_picture_and_return_filename(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(fname) = fname_opt else {
        return Err(StatusCode::NOT_FOUND);
    };

    let settings = state.settings.get().await;
    if settings.pinned_image.as_ref() == Some(&fname) {
        state
            .settings
            .update(|s| {
                s.pinned_image = None;
            })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let path = std::path::Path::new(&CONFIG.backend_data_dir).join(&fname);
    match tokio::fs::remove_file(&path).await {
        Ok(_) => tracing::debug!("removed file {}", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::warn!("file already gone: {}", path.display())
        }
        Err(e) => {
            tracing::error!("failed to delete {path:?}: {e}");
            // DB row is already gone; still return 204
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn pin_picture(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }

    let picture = state
        .repo
        .get_picture(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    state
        .settings
        .update(|s| {
            s.pinned_image = Some(picture.filename);
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn unpin_picture(
    ApiKey { scope, .. }: ApiKey,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if scope != "rw" {
        return Err(StatusCode::FORBIDDEN);
    }

    let picture = state
        .repo
        .get_picture(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let settings = state.settings.get().await;
    if settings.pinned_image.as_ref().is_none() {
        return Ok(StatusCode::NO_CONTENT);
    } else if settings.pinned_image.as_ref() != Some(&picture.filename) {
        return Err(StatusCode::BAD_REQUEST);
    }

    state
        .settings
        .update(|s| {
            s.pinned_image = None;
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
