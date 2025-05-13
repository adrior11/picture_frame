use std::sync::Arc;

use anyhow::Result;
use axum::{Router, extract::Request};
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;
use tower_http::trace::{
    DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer,
};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use backend::{CONFIG, core::*, db::Repository};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();

    let manager = SqliteConnectionManager::file(CONFIG.db_file.clone());
    let pool = r2d2::Pool::new(manager).unwrap();
    let repo = Repository::new(pool);
    repo.init_schema()?;

    let state = AppState {
        repo: Arc::new(repo),
    };

    let router = Router::new()
        .merge(key_routes())
        .merge(picture_routes())
        .with_state(state)
        .layer(
            TraceLayer::new_for_http().make_span_with(|req: &Request<_>| {
                tracing::info_span!(
                    "http_request",
                    method = %req.method(),
                    uri = %req.uri(),
                    client_ip = %req.headers().get("x-forwarded-for").and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown"),
                )
            })
            .on_request(DefaultOnRequest::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO).include_headers(true))
            .on_failure(DefaultOnFailure::new().level(Level::INFO))
            .on_eos(DefaultOnEos::new().level(Level::INFO))
        );

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let listener_addr = listener.local_addr().unwrap().to_string();
    tracing::info!("⇢ listening on http://{listener_addr}");

    axum::serve(listener, router).await.unwrap();

    Ok(())
}
