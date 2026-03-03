use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use crate::AppState;
use uuid::Uuid;

use crate::extractors::AuthUser;

#[derive(Debug, Deserialize)]
pub struct RegisterRepoRequest {
    pub repo_name: String,
    pub github_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterRepoResponse {
    pub repo_id: Uuid,
}

pub async fn register_repo(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterRepoRequest>,
) -> Result<(StatusCode, Json<RegisterRepoResponse>), (StatusCode, String)> {
    let repo_id: Uuid = sqlx::query_scalar(
        "INSERT INTO repos (org_id, name, github_url) VALUES ($1, $2, $3) ON CONFLICT (org_id, name) DO UPDATE SET github_url = COALESCE(EXCLUDED.github_url, repos.github_url) RETURNING id",
    )
    .bind(auth.org_id)
    .bind(&req.repo_name)
    .bind(&req.github_url)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Trigger background clone if github_url is provided
    if let Some(url) = &req.github_url {
        let pool = state.pool.clone();
        let repo_mgr = state.repo_manager.clone();
        let url = url.clone();
        tokio::spawn(async move {
            if let Err(e) = repo_mgr.clone_repo(&pool, repo_id, &url).await {
                tracing::error!("Failed to clone repo {repo_id}: {e}");
            }
        });
    }

    Ok((StatusCode::CREATED, Json(RegisterRepoResponse { repo_id })))
}

pub async fn sync_repo(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(repo_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let repo = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT clone_status, github_url FROM repos WHERE id = $1 AND org_id = $2",
    )
    .bind(repo_id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Repo not found".into()))?;

    match repo.0.as_str() {
        "ready" => {
            // Already cloned — just fetch latest
            state
                .repo_manager
                .fetch_repo(repo_id)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            sqlx::query("UPDATE repos SET last_fetched_at = now() WHERE id = $1")
                .bind(repo_id)
                .execute(&state.pool)
                .await
                .ok();

            Ok(Json(serde_json::json!({"status": "synced"})))
        }
        "pending" | "error" => {
            // Not yet cloned or previous clone failed — trigger clone
            let github_url = repo.1.ok_or((
                StatusCode::BAD_REQUEST,
                "Repo has no github_url set. Update the repo with a github_url first.".into(),
            ))?;

            let pool = state.pool.clone();
            let repo_mgr = state.repo_manager.clone();
            tokio::spawn(async move {
                if let Err(e) = repo_mgr.clone_repo(&pool, repo_id, &github_url).await {
                    tracing::error!("Failed to clone repo {repo_id}: {e}");
                }
            });

            Ok(Json(serde_json::json!({"status": "cloning"})))
        }
        "cloning" => {
            Ok(Json(serde_json::json!({"status": "cloning"})))
        }
        other => {
            Err((StatusCode::BAD_REQUEST, format!("Unknown clone status: {other}")))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RepoResponse {
    pub id: Uuid,
    pub name: String,
    pub github_url: Option<String>,
    pub clone_status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_repos(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<RepoResponse>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, github_url, clone_status, created_at FROM repos WHERE org_id = $1 ORDER BY name",
    )
    .bind(auth.org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let repos = rows
        .into_iter()
        .map(|r| RepoResponse {
            id: r.0,
            name: r.1,
            github_url: r.2,
            clone_status: r.3,
            created_at: r.4,
        })
        .collect();

    Ok(Json(repos))
}

pub async fn delete_repo(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    sqlx::query("DELETE FROM repos WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(auth.org_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
