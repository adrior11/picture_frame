#[cfg(unix)]
pub async fn listen_for_shutdown(notify: std::sync::Arc<tokio::sync::Notify>) {
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = sigterm.recv() => {},
    }

    tracing::info!("shutdown signal received – starting graceful shutdown");
    notify.notify_waiters();
}
