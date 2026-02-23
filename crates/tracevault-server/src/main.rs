use axum::{routing::{get, post}, Router};
use tower_http::trace::TraceLayer;

mod api;
mod config;
mod db;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = config::ServerConfig::from_env();
    let pool = db::create_pool(&cfg.database_url)
        .await
        .expect("Failed to connect to database");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        // Traces
        .route("/api/v1/traces", post(api::traces::create_trace))
        .route("/api/v1/traces", get(api::traces::list_traces))
        .route("/api/v1/traces/{id}", get(api::traces::get_trace))
        // Repos
        .route("/api/v1/repos", post(api::repos::register_repo))
        // Auth
        .route("/api/v1/auth/register", post(api::auth::register))
        // Policies
        .route("/api/v1/policies", get(api::policies::list_policies))
        .route("/api/v1/policies/evaluate", post(api::policies::evaluate))
        // Analytics
        .route("/api/v1/analytics/tokens", get(api::analytics::token_analytics))
        // GitHub
        .route("/api/v1/github/webhook", post(api::github::webhook))
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr())
        .await
        .unwrap();
    tracing::info!("TraceVault server listening on {}", cfg.bind_addr());
    axum::serve(listener, app).await.unwrap();
}
