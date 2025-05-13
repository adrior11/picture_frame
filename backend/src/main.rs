use std::sync::Arc;

use anyhow::Result;
use axum::{Router, extract::Request, middleware};
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;
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

    let router = Router::new()
        .merge(api::key_routes())
        .merge(api::picture_routes())
        .with_state(state.clone())
        .route_layer(middleware::from_fn(metrics::track_metrics))
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
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024 /* 10MB */));

    // let shutdown = async {
    //     signal::unix::signal(SignalKind::terminate())
    //         .unwrap()
    //         .recv()
    //         .await;
    // };

    tokio::join!(start_main_server(router), metrics::start_metrics_server());

    Ok(())
}

async fn start_main_server(router: Router) {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", CONFIG.backend_port))
        .await
        .unwrap();
    let listener_addr = listener.local_addr().unwrap().to_string();
    tracing::info!("⇢ listening on http://{listener_addr}");
    axum::serve(listener, router)
        // .with_graceful_shutdown(shutdown)
        .await
        .unwrap();
}
