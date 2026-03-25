use axum::{
    routing::{delete, get, post, put},
    Router,
};
use http::Method;
#[cfg(not(feature = "enterprise"))]
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod api;
mod attribution;
mod audit;
mod auth;
mod branch_tracking;
mod config;
mod db;
mod encryption;
pub mod extensions;
mod extractors;
mod llm;
mod org_signing;
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
    pub encryption_key: Option<String>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
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
            .allow_headers([http::header::CONTENT_TYPE, http::header::AUTHORIZATION])
    } else {
        CorsLayer::permissive()
    };

    let repo_manager = repo_manager::RepoManager::new(&cfg.repos_dir);
    let extensions = build_extensions(&cfg);

    // Auto-sync repos that are in 'ready' state on startup
    sync_repos_on_startup(&pool, &repo_manager, &extensions).await;

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
        .route(
            "/api/v1/auth/device/{token}/approve",
            post(api::auth::device_approve),
        )
        .route("/api/v1/auth/logout", post(api::auth::logout))
        .route("/api/v1/auth/me", get(api::auth::me))
        // Public (no auth) — for invitation request form
        .route("/api/v1/orgs/public", get(api::auth::list_public_orgs))
        .route(
            "/api/v1/invitation-requests",
            post(api::auth::request_invitation),
        )
        // User endpoints
        .route("/api/v1/me/orgs", get(api::auth::list_my_orgs))
        // Org management (create is org-agnostic)
        .route("/api/v1/orgs", post(api::orgs::create_org))
        // Org-scoped: org details & members
        .route(
            "/api/v1/orgs/{slug}",
            get(api::orgs::get_org).put(api::orgs::update_org),
        )
        .route(
            "/api/v1/orgs/{slug}/members",
            get(api::orgs::list_members).post(api::orgs::invite_member),
        )
        .route(
            "/api/v1/orgs/{slug}/members/{user_id}",
            delete(api::orgs::remove_member),
        )
        .route(
            "/api/v1/orgs/{slug}/members/{user_id}/role",
            put(api::orgs::change_role),
        )
        // Invitation requests (admin)
        .route(
            "/api/v1/orgs/{slug}/invitation-requests",
            get(api::orgs::list_invitation_requests),
        )
        .route(
            "/api/v1/orgs/{slug}/invitation-requests/{id}/approve",
            post(api::orgs::approve_invitation_request),
        )
        .route(
            "/api/v1/orgs/{slug}/invitation-requests/{id}/reject",
            post(api::orgs::reject_invitation_request),
        )
        .route(
            "/api/v1/orgs/{slug}/llm-settings",
            get(api::orgs::get_llm_settings).put(api::orgs::update_llm_settings),
        )
        // Org-scoped: repos
        .route(
            "/api/v1/orgs/{slug}/repos",
            get(api::repos::list_repos).post(api::repos::register_repo),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{id}",
            get(api::repos::get_repo).delete(api::repos::delete_repo),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{id}/settings",
            get(api::repos::get_settings).put(api::repos::update_settings),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{id}/sync",
            post(api::repos::sync_repo),
        )
        // Org-scoped: code browser
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/code/branches",
            get(api::code::list_branches),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/code/tree",
            get(api::code::get_tree),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/code/blob",
            get(api::code::get_blob),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/code/blame",
            get(api::code::get_blame),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/code/commits",
            get(api::code::list_file_commits),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/code/info",
            get(api::code::get_ref_info),
        )
        // Org-scoped: story
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/story",
            post(api::code::generate_story),
        )
        // Org-scoped: traces (v2 streaming UI)
        .route(
            "/api/v1/orgs/{slug}/traces/stats",
            get(api::traces_ui::get_stats),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/sessions",
            get(api::traces_ui::list_sessions),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/sessions/{id}",
            get(api::traces_ui::get_session),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/commits",
            get(api::traces_ui::list_commits),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/commits/{id}",
            get(api::traces_ui::get_commit),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/commits/{id}/verify",
            get(api::compliance::verify_trace),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/timeline",
            get(api::traces_ui::get_timeline),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/attribution/{commit_id}/{*file_path}",
            get(api::traces_ui::get_attribution),
        )
        .route(
            "/api/v1/orgs/{slug}/traces/branches",
            get(api::traces_ui::get_branches),
        )
        // Org-scoped: api keys
        .route(
            "/api/v1/orgs/{slug}/api-keys",
            post(api::api_keys::create_api_key).get(api::api_keys::list_api_keys),
        )
        .route(
            "/api/v1/orgs/{slug}/api-keys/{id}",
            delete(api::api_keys::delete_api_key),
        )
        // Org-scoped: policies
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/policies",
            get(api::policies::list_repo_policies).post(api::policies::create_repo_policy),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/policies/check",
            post(api::policies::check_policies),
        )
        .route(
            "/api/v1/orgs/{slug}/policies/{id}",
            put(api::policies::update_policy).delete(api::policies::delete_policy),
        )
        // Org-scoped: compliance
        .route(
            "/api/v1/orgs/{slug}/compliance",
            get(api::compliance::get_compliance_settings)
                .put(api::compliance::update_compliance_settings),
        )
        .route(
            "/api/v1/orgs/{slug}/compliance/public-key",
            get(api::compliance::get_public_key),
        )
        .route(
            "/api/v1/orgs/{slug}/compliance/verify-chain",
            post(api::compliance::verify_chain),
        )
        .route(
            "/api/v1/orgs/{slug}/compliance/chain-status",
            get(api::compliance::get_chain_status),
        )
        .route(
            "/api/v1/orgs/{slug}/audit-log",
            get(api::compliance::list_audit_log),
        )
        // Org-scoped: pricing
        .route(
            "/api/v1/orgs/{slug}/pricing",
            get(api::pricing::list_pricing).post(api::pricing::create_pricing),
        )
        .route(
            "/api/v1/orgs/{slug}/pricing/models",
            get(api::pricing::list_models),
        )
        .route(
            "/api/v1/orgs/{slug}/pricing/{id}",
            put(api::pricing::update_pricing),
        )
        .route(
            "/api/v1/orgs/{slug}/pricing/{id}/recalculate",
            post(api::pricing::recalculate),
        )
        // Org-scoped: streaming
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/stream",
            post(api::stream::handle_stream),
        )
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/commits",
            post(api::commit_push::handle_commit_push),
        )
        // Org-scoped: dashboard
        .route(
            "/api/v1/orgs/{slug}/dashboard",
            get(api::dashboard::get_dashboard),
        )
        // Org-scoped: analytics
        .route(
            "/api/v1/orgs/{slug}/analytics/filters",
            get(api::analytics::get_filters),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/overview",
            get(api::analytics::get_overview),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/tokens",
            get(api::analytics::get_tokens),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/models",
            get(api::analytics::get_models),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/authors",
            get(api::analytics::get_authors),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/attribution",
            get(api::analytics::get_attribution),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/sessions",
            get(api::analytics::get_sessions),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/sessions/{id}/detail",
            get(api::session_detail::get_session_detail),
        )
        .route(
            "/api/v1/orgs/{slug}/analytics/cost",
            get(api::analytics::get_cost),
        )
        // Org-scoped: CI
        .route(
            "/api/v1/orgs/{slug}/repos/{repo_id}/ci/verify",
            post(api::ci::verify_commits),
        )
        // GitHub webhook (org-agnostic)
        .route("/api/v1/github/webhook", post(api::github::webhook))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(AppState {
            pool,
            repo_manager,
            extensions,
            encryption_key: cfg.encryption_key.clone(),
        });

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!("TraceVault server listening on {}", bind_addr);
    axum::serve(listener, app).await.unwrap();
}

async fn sync_repos_on_startup(
    pool: &sqlx::PgPool,
    repo_manager: &repo_manager::RepoManager,
    extensions: &extensions::ExtensionRegistry,
) {
    let rows = sqlx::query_as::<_, (uuid::Uuid, Option<String>)>(
        "SELECT id, deploy_key_encrypted FROM repos WHERE clone_status = 'ready' AND github_url IS NOT NULL",
    )
    .fetch_all(pool)
    .await;

    let repos = match rows {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to query repos for auto-sync: {e}");
            return;
        }
    };

    if repos.is_empty() {
        return;
    }

    tracing::info!("Auto-syncing {} repo(s) on startup...", repos.len());

    for (repo_id, has_key) in &repos {
        let deploy_key = if has_key.is_some() {
            match api::repos::get_deploy_key(pool, *repo_id, extensions.encryption.as_ref()).await {
                Ok(k) => k,
                Err(_) => None,
            }
        } else {
            None
        };

        match repo_manager.fetch_repo(*repo_id, deploy_key.as_deref()) {
            Ok(()) => {
                sqlx::query("UPDATE repos SET last_fetched_at = now() WHERE id = $1")
                    .bind(repo_id)
                    .execute(pool)
                    .await
                    .ok();
                tracing::info!("Synced repo {repo_id}");
            }
            Err(e) => {
                tracing::warn!("Failed to sync repo {repo_id}: {e}");
            }
        }
    }
}

fn build_extensions(cfg: &config::ServerConfig) -> extensions::ExtensionRegistry {
    #[cfg(feature = "enterprise")]
    {
        use tracevault_core::extensions::EnterpriseConfig;
        let enterprise_cfg = EnterpriseConfig {
            encryption_key: cfg.encryption_key.clone(),
        };
        tracevault_enterprise::register(&enterprise_cfg)
    }

    #[cfg(not(feature = "enterprise"))]
    {
        let mut ext = extensions::community_registry();
        ext.pricing = Arc::new(extensions::FullPricingProvider);
        if let Some(ref key) = cfg.encryption_key {
            ext.encryption = Arc::new(extensions::FullEncryptionProvider::new(key.clone()));
        }
        ext
    }
}
