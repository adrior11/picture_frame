use std::{
    future,
    time::{Duration, Instant},
};

use axum::{
    Router,
    extract::{MatchedPath, Request},
    middleware::Next,
    response::IntoResponse,
    routing,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use tokio::net::TcpListener;

use crate::CONFIG;

pub async fn start_metrics_server() {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", CONFIG.prometheus_port))
        .await
        .unwrap();
    let listener_addr = listener.local_addr().unwrap().to_string();
    tracing::info!("⇢ listening on http://{listener_addr}");

    axum::serve(listener, metrics_routes())
        // .with_graceful_shutdown(shutdown)
        .await
        .unwrap();
}

pub async fn track_metrics(req: Request, next: Next) -> impl IntoResponse {
    let start = Instant::now();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };
    let method = req.method().clone();

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status),
    ];

    metrics::counter!("http_requests_total", &labels).increment(1);
    metrics::histogram!("http_requests_duration_seconds", &labels).record(latency);

    response
}

fn metrics_routes() -> Router {
    let recorder_handle = setup_metrics_recorder();
    Router::new().route(
        "/metrics",
        routing::get(move || future::ready(recorder_handle.render())),
    )
}

fn setup_metrics_recorder() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .unwrap()
        .install_recorder()
        .unwrap()
}
