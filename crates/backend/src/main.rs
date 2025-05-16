use std::sync::Arc;

use anyhow::Result;
use axum::{Router, extract::Request, middleware};
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{net::TcpListener, sync::Notify};
use tower_http::{
    limit::RequestBodyLimitLayer,
    trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use libs::{frame_settings::SharedSettings, util};

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

    let shared_settings = SharedSettings::load(&CONFIG.backend_frame_settings_file).unwrap();
    let manager = SqliteConnectionManager::file(CONFIG.backend_db_file.clone());
    let pool = r2d2::Pool::builder().max_size(4).build(manager).unwrap();
    let repo = Repository::new(pool);
    repo.init_schema()?;
    let state = AppState {
        repo: Arc::new(repo),
        settings: shared_settings.clone(),
    };

    let api_router = Router::new()
        .merge(api::picture_routes())
        .merge(api::settings_routes())
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

    metrics::spawn_system_metrics(state.repo.clone());

    let shutdown_notify = Arc::new(Notify::new());
    tokio::spawn(util::listen_for_shutdown(shutdown_notify.clone()));

    // let cert_path = &CONFIG.backend_tls_cert_file;
    // let key_path = &CONFIG.backend_tls_key_file;
    // let tls = RustlsConfig::from_pem_file(cert_path, key_path)
    //     .await
    //     .unwrap();

    // let api_addr = SocketAddr::from(([0, 0, 0, 0], CONFIG.backend_port));
    // let api_server = axum_server::bind_rustls(api_addr, tls).serve(api_router.into_make_service());

    let api_listener = TcpListener::bind(format!("0.0.0.0:{}", CONFIG.backend_port)).await?;
    let metrics_listener = TcpListener::bind(format!("0.0.0.0:{}", CONFIG.prometheus_port)).await?;

    tracing::info!("⇢ API listening on: http://{}", api_listener.local_addr()?);
    tracing::info!(
        "⇢ Metrics listening on: http://{}/metrics",
        metrics_listener.local_addr()?
    );

    let api_server = axum::serve(api_listener, api_router).with_graceful_shutdown({
        let n = shutdown_notify.clone();
        async move { n.notified().await }
    });
    let metrics_server = axum::serve(metrics_listener, metrics_router).with_graceful_shutdown({
        let n = shutdown_notify.clone();
        async move { n.notified().await }
    });

    tokio::try_join!(api_server, metrics_server)?;

    Ok(())
}
