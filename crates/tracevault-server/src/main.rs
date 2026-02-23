mod db;

use axum::{routing::get, Router};

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("TraceVault server listening on :3000");
    axum::serve(listener, app).await.unwrap();
}
