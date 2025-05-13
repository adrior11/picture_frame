// #![allow(dead_code, unused_imports, unused_variables)]
use std::{future, sync::Arc, time::Duration};

use axum::{
    Router,
    extract::{MatchedPath, Request},
    middleware::Next,
    response::IntoResponse,
    routing,
};
use metrics::gauge;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use sysinfo::System;

use crate::{CONFIG, db::Repository};

/// Install global recorder and return an Axum `Router` with `/metrics`.
pub fn prometheus_router() -> Router {
    const BUCKETS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    let handle: PrometheusHandle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("http_requests_duration_seconds".into()),
            BUCKETS,
        )
        .unwrap()
        .install_recorder()
        .expect("global recorder");

    Router::new().route(
        "/metrics",
        routing::get(move || future::ready(handle.render())),
    )
}

/// Axum middleware: count every request + histogram latency.
pub async fn track_http(req: Request, next: Next) -> impl IntoResponse {
    use metrics::{counter, histogram};

    let start = std::time::Instant::now();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str())
        .unwrap_or(req.uri().path())
        .to_owned();
    let method = req.method().as_str().to_owned();

    let res = next.run(req).await;

    let status = res.status().as_u16().to_string();
    let labels = [("method", method), ("path", path), ("status", status)];

    counter!("http_requests_total", &labels).increment(1);
    histogram!("http_requests_duration_seconds", &labels).record(start.elapsed().as_secs_f64());

    res
}

/// Spawn a background job that refreshes host-level gauges.
pub fn spawn_system_metrics(repo: Arc<Repository>) {
    tracing::info!("Spawning system metrics");
    tokio::spawn(async move {
        let mut sys = System::new(); // Keep one instance for accuracy
        let mut tick =
            tokio::time::interval(Duration::from_secs(CONFIG.prometheus_refresh_interval));

        let pictureframe_cpu_usage_percent = gauge!("pictureframe_cpu_usage_percent");
        let pictureframe_memory_used_bytes = gauge!("pictureframe_memory_used_bytes");
        let pictureframe_memory_total_bytes = gauge!("pictureframe_memory_total_bytes");
        let pictureframe_image_count = gauge!("pictureframe_image_count");

        loop {
            tick.tick().await;

            tracing::debug!("Refreshing system metrics");

            sys.refresh_cpu_usage();
            sys.refresh_memory();

            pictureframe_cpu_usage_percent.set(sys.global_cpu_usage());
            pictureframe_memory_used_bytes.set(sys.used_memory() as f64);
            pictureframe_memory_total_bytes.set(sys.total_memory() as f64);

            if let Ok(n) = repo.count_pictures().await {
                pictureframe_image_count.set(n as f64);
            }
        }
    });
}
