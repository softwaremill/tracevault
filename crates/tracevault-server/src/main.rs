use axum::{
    routing::{delete, get, post, put},
    Router,
};
use http::Method;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod api;
mod audit;
mod auth;
mod config;
mod db;
mod encryption;
pub mod extensions;
mod extractors;
mod llm;
pub mod permissions;
pub mod pricing;
mod repo_manager;
mod signing;
mod story;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
    pub repo_manager: repo_manager::RepoManager,
    pub extensions: extensions::ExtensionRegistry,
}

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
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                http::header::CONTENT_TYPE,
                http::header::AUTHORIZATION,
            ])
    } else {
        CorsLayer::permissive()
    };

    let repo_manager = repo_manager::RepoManager::new(&cfg.repos_dir);
    let extensions = build_extensions(&cfg);

    let bind_addr = cfg.bind_addr();

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/features", get(api::features::get_features))
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
        .route("/api/v1/repos/{id}/sync", post(api::repos::sync_repo))
        .route(
            "/api/v1/repos/{id}/settings",
            get(api::repos::get_settings).put(api::repos::update_settings),
        )
        // Code Browser
        .route(
            "/api/v1/repos/{repo_id}/code/branches",
            get(api::code::list_branches),
        )
        .route(
            "/api/v1/repos/{repo_id}/code/tree",
            get(api::code::get_tree),
        )
        .route(
            "/api/v1/repos/{repo_id}/code/blob",
            get(api::code::get_blob),
        )
        .route(
            "/api/v1/repos/{repo_id}/code/blame",
            get(api::code::get_blame),
        )
        .route(
            "/api/v1/repos/{repo_id}/code/commits",
            get(api::code::list_file_commits),
        )
        .route(
            "/api/v1/repos/{repo_id}/code/info",
            get(api::code::get_ref_info),
        )
        // Story
        .route(
            "/api/v1/repos/{repo_id}/story",
            post(api::code::generate_story),
        )
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
        // Org LLM Settings
        .route(
            "/api/v1/orgs/{id}/llm-settings",
            get(api::orgs::get_llm_settings).put(api::orgs::update_llm_settings),
        )
        // API Keys
        .route("/api/v1/api-keys", post(api::api_keys::create_api_key))
        .route("/api/v1/api-keys", get(api::api_keys::list_api_keys))
        .route(
            "/api/v1/api-keys/{id}",
            delete(api::api_keys::delete_api_key),
        )
        // Policies
        .route(
            "/api/v1/repos/{repo_id}/policies",
            get(api::policies::list_repo_policies),
        )
        .route(
            "/api/v1/repos/{repo_id}/policies",
            post(api::policies::create_repo_policy),
        )
        .route(
            "/api/v1/repos/{repo_id}/policies/check",
            post(api::policies::check_policies),
        )
        .route("/api/v1/policies/{id}", put(api::policies::update_policy))
        .route(
            "/api/v1/policies/{id}",
            delete(api::policies::delete_policy),
        )
        // Compliance
        .route(
            "/api/v1/orgs/{id}/compliance",
            get(api::compliance::get_compliance_settings)
                .put(api::compliance::update_compliance_settings),
        )
        .route(
            "/api/v1/orgs/{id}/compliance/public-key",
            get(api::compliance::get_public_key),
        )
        .route(
            "/api/v1/orgs/{id}/compliance/verify-chain",
            post(api::compliance::verify_chain),
        )
        .route(
            "/api/v1/orgs/{id}/compliance/chain-status",
            get(api::compliance::get_chain_status),
        )
        .route(
            "/api/v1/orgs/{id}/audit-log",
            get(api::compliance::list_audit_log),
        )
        .route(
            "/api/v1/traces/{id}/verify",
            get(api::compliance::verify_trace),
        )
        // Analytics
        .route(
            "/api/v1/analytics/filters",
            get(api::analytics::get_filters),
        )
        .route(
            "/api/v1/analytics/overview",
            get(api::analytics::get_overview),
        )
        .route(
            "/api/v1/analytics/tokens",
            get(api::analytics::get_tokens),
        )
        .route(
            "/api/v1/analytics/models",
            get(api::analytics::get_models),
        )
        .route(
            "/api/v1/analytics/authors",
            get(api::analytics::get_authors),
        )
        .route(
            "/api/v1/analytics/attribution",
            get(api::analytics::get_attribution),
        )
        .route(
            "/api/v1/analytics/sessions",
            get(api::analytics::get_sessions),
        )
        .route(
            "/api/v1/analytics/cost",
            get(api::analytics::get_cost),
        )
        // CI Verification
        .route(
            "/api/v1/repos/{repo_id}/ci/verify",
            post(api::ci::verify_commits),
        )
        // GitHub
        .route("/api/v1/github/webhook", post(api::github::webhook))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(AppState {
            pool,
            repo_manager,
            extensions,
        });

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap();
    tracing::info!("TraceVault server listening on {}", bind_addr);
    axum::serve(listener, app).await.unwrap();
}

fn build_extensions(cfg: &config::ServerConfig) -> extensions::ExtensionRegistry {
    #[cfg(feature = "enterprise")]
    {
        tracevault_enterprise::register(cfg)
    }

    #[cfg(not(feature = "enterprise"))]
    {
        let signing = signing::SigningService::new(cfg.signing_key_seed.as_deref());
        let llm_instance = llm::create_llm(cfg).map(|b| Arc::from(b) as Arc<dyn llm::StoryLlm>);

        let mut ext = extensions::community_registry();
        ext.signing = Arc::new(extensions::FullSigningProvider::new(signing));
        ext.pricing = Arc::new(extensions::FullPricingProvider);
        if let Some(ref key) = cfg.encryption_key {
            ext.encryption = Arc::new(extensions::FullEncryptionProvider::new(key.clone()));
        }
        if let Some(llm) = llm_instance {
            ext.story = Arc::new(extensions::LlmStoryProvider::new(Arc::from(llm)));
        }
        ext
    }
}
