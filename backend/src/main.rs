#![allow(unused_imports)]
use std::sync::Arc;

use anyhow::Result;
use axum::{Router, extract::Request, middleware};
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{
    net::TcpListener,
    signal::{self, unix::SignalKind},
};
use tower_http::{
    limit::RequestBodyLimitLayer,
    trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use backend::{
    CONFIG, api,
    common::{AppState, metrics},
    db::Repository,
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();

    let manager = SqliteConnectionManager::file(CONFIG.backend_db_file.clone());
    let pool = r2d2::Pool::builder().max_size(4).build(manager).unwrap();
    let repo = Repository::new(pool);
    repo.init_schema()?;
    let state = AppState {
        repo: Arc::new(repo),
    };

    let api_router = Router::new()
        .merge(api::key_routes())
        .merge(api::picture_routes())
        .with_state(state.clone())
        .route_layer(middleware::from_fn(metrics::track_http))
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
        )
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024 /* 10MiB */));
    let metrics_router = metrics::prometheus_router();

    let api_listener = TcpListener::bind(format!("0.0.0.0:{}", CONFIG.backend_port)).await?;
    let metrics_listener = TcpListener::bind(format!("0.0.0.0:{}", CONFIG.prometheus_port)).await?;
    tracing::info!("⇢ API listening on: http://{}", api_listener.local_addr()?);
    tracing::info!(
        "⇢ Metrics listening on: http://{}/metrics",
        metrics_listener.local_addr()?
    );

    // let shutdown = async {
    //     signal::unix::signal(SignalKind::terminate())
    //         .unwrap()
    //         .recv()
    //         .await;
    // };

    metrics::spawn_system_metrics(state.repo.clone());

    let (_api_server, _metrics_server) = tokio::join!(
        axum::serve(api_listener, api_router),
        axum::serve(metrics_listener, metrics_router),
    );

    Ok(())
}
