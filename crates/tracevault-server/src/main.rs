use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

mod api;
mod auth;
mod config;
mod db;
mod extractors;

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

    let cors = if let Some(origin) = &cfg.cors_origin {
        CorsLayer::new()
            .allow_origin(origin.parse::<http::HeaderValue>().unwrap())
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::permissive()
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        // Auth (public)
        .route("/api/v1/auth/register", post(api::auth::register))
        .route("/api/v1/auth/login", post(api::auth::login))
        .route("/api/v1/auth/device", post(api::auth::device_start))
        .route(
            "/api/v1/auth/device/{token}/status",
            get(api::auth::device_status),
        )
        // Auth (requires session)
        .route(
            "/api/v1/auth/device/{token}/approve",
            post(api::auth::device_approve),
        )
        .route("/api/v1/auth/logout", post(api::auth::logout))
        .route("/api/v1/auth/me", get(api::auth::me))
        // Traces
        .route("/api/v1/traces", post(api::traces::create_trace))
        .route("/api/v1/traces", get(api::traces::list_traces))
        .route("/api/v1/traces/{id}", get(api::traces::get_trace))
        // Repos
        .route("/api/v1/repos", get(api::repos::list_repos))
        .route("/api/v1/repos", post(api::repos::register_repo))
        .route("/api/v1/repos/{id}", delete(api::repos::delete_repo))
        // Orgs
        .route("/api/v1/orgs/{id}", get(api::orgs::get_org))
        .route("/api/v1/orgs/{id}", put(api::orgs::update_org))
        .route(
            "/api/v1/orgs/{id}/members",
            get(api::orgs::list_members),
        )
        .route(
            "/api/v1/orgs/{id}/members",
            post(api::orgs::invite_member),
        )
        .route(
            "/api/v1/orgs/{id}/members/{user_id}",
            delete(api::orgs::remove_member),
        )
        .route(
            "/api/v1/orgs/{id}/members/{user_id}/role",
            put(api::orgs::change_role),
        )
        // API Keys
        .route("/api/v1/api-keys", post(api::api_keys::create_api_key))
        .route("/api/v1/api-keys", get(api::api_keys::list_api_keys))
        .route(
            "/api/v1/api-keys/{id}",
            delete(api::api_keys::delete_api_key),
        )
        // Policies
        .route("/api/v1/policies", get(api::policies::list_policies))
        .route("/api/v1/policies/evaluate", post(api::policies::evaluate))
        // Analytics
        .route(
            "/api/v1/analytics/tokens",
            get(api::analytics::token_analytics),
        )
        // GitHub
        .route("/api/v1/github/webhook", post(api::github::webhook))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr())
        .await
        .unwrap();
    tracing::info!("TraceVault server listening on {}", cfg.bind_addr());
    axum::serve(listener, app).await.unwrap();
}
