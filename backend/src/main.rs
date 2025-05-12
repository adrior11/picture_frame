use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    routing::{delete, get},
};
use r2d2_sqlite::SqliteConnectionManager;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use backend::{CONFIG, core::*, db::PictureRepository};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .init();

    let manager = SqliteConnectionManager::file(CONFIG.db_file.clone());
    let pool = r2d2::Pool::new(manager).unwrap();
    let repo = PictureRepository::new(pool);
    repo.init_schema()?; // Run migrations / create table

    let state = AppState {
        pictures: Arc::new(repo),
        api_token: CONFIG.api_token.clone(),
    };

    let router = Router::new()
        .route("/api/ping", get(ping))
        .route("/api/pictures", get(list_pictures).post(add_picture))
        .route("/api/pictures/{id}", delete(delete_picture))
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let listener_addr = listener.local_addr().unwrap().to_string();
    tracing::info!("⇢ listening on http://{listener_addr}");

    axum::serve(listener, router).await.unwrap();

    Ok(())
}
