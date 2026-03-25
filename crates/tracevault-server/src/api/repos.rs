use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::extractors::OrgAuth;

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
    auth: OrgAuth,
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
            if let Err(e) = repo_mgr.clone_repo(&pool, repo_id, &url, None).await {
                tracing::error!("Failed to clone repo {repo_id}: {e}");
            }
        });
    }

    Ok((StatusCode::CREATED, Json(RegisterRepoResponse { repo_id })))
}

/// Decrypt the deploy key for a repo if it exists and encryption is configured.
pub async fn get_deploy_key(
    pool: &sqlx::PgPool,
    repo_id: Uuid,
    encryption: &dyn crate::extensions::EncryptionProvider,
) -> Result<Option<String>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT deploy_key_encrypted, deploy_key_nonce FROM repos WHERE id = $1",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some((Some(ct), Some(nonce))) = row {
        let plaintext = encryption.decrypt(&ct, &nonce).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to decrypt deploy key: {e}"),
            )
        })?;
        Ok(Some(plaintext))
    } else {
        Ok(None)
    }
}

pub async fn sync_repo(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, repo_id)): Path<(String, Uuid)>,
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

    let deploy_key =
        get_deploy_key(&state.pool, repo_id, state.extensions.encryption.as_ref()).await?;

    match repo.0.as_str() {
        "ready" => {
            // Already cloned — just fetch latest
            state
                .repo_manager
                .fetch_repo(repo_id, deploy_key.as_deref())
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
                if let Err(e) = repo_mgr
                    .clone_repo(&pool, repo_id, &github_url, deploy_key.as_deref())
                    .await
                {
                    tracing::error!("Failed to clone repo {repo_id}: {e}");
                }
            });

            Ok(Json(serde_json::json!({"status": "cloning"})))
        }
        "cloning" => Ok(Json(serde_json::json!({"status": "cloning"}))),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("Unknown clone status: {other}"),
        )),
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

pub async fn get_repo(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<RepoResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, github_url, clone_status, created_at FROM repos WHERE id = $1 AND org_id = $2",
    )
    .bind(id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Repo not found".into()))?;

    Ok(Json(RepoResponse {
        id: row.0,
        name: row.1,
        github_url: row.2,
        clone_status: row.3,
        created_at: row.4,
    }))
}

pub async fn list_repos(
    State(state): State<AppState>,
    auth: OrgAuth,
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
    auth: OrgAuth,
    Path((_slug, id)): Path<(String, Uuid)>,
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

// --- Settings endpoints ---

#[derive(Debug, Serialize)]
pub struct RepoSettingsResponse {
    pub github_url: Option<String>,
    pub clone_status: String,
    pub has_deploy_key: bool,
    pub last_fetched_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_settings(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<Json<RepoSettingsResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Option<String>, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT github_url, clone_status, deploy_key_encrypted, last_fetched_at FROM repos WHERE id = $1 AND org_id = $2",
    )
    .bind(id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Repo not found".into()))?;

    Ok(Json(RepoSettingsResponse {
        github_url: row.0,
        clone_status: row.1,
        has_deploy_key: row.2.is_some(),
        last_fetched_at: row.3,
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub github_url: Option<String>,
    pub deploy_key: Option<String>,
}

pub async fn update_settings(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, id)): Path<(String, Uuid)>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<RepoSettingsResponse>, (StatusCode, String)> {
    // Verify repo belongs to org
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM repos WHERE id = $1 AND org_id = $2)",
    )
    .bind(id)
    .bind(auth.org_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Repo not found".into()));
    }

    // Update github_url if provided (ignore empty strings)
    if let Some(ref url) = req.github_url.filter(|u| !u.trim().is_empty()) {
        sqlx::query("UPDATE repos SET github_url = $1 WHERE id = $2")
            .bind(url)
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Encrypt and store deploy key if provided (ignore empty strings)
    if let Some(ref key_pem) = req.deploy_key.filter(|k| !k.trim().is_empty()) {
        let (ct, nonce) = state.extensions.encryption.encrypt(key_pem).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Encryption failed: {e}"),
            )
        })?;

        sqlx::query(
            "UPDATE repos SET deploy_key_encrypted = $1, deploy_key_nonce = $2 WHERE id = $3",
        )
        .bind(&ct)
        .bind(&nonce)
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    // Read back current state to decide whether to trigger clone
    let row = sqlx::query_as::<_, (Option<String>, String, Option<String>, Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT github_url, clone_status, deploy_key_encrypted, deploy_key_nonce, last_fetched_at FROM repos WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let github_url = row.0.clone();
    let clone_status = row.1.clone();
    let has_deploy_key = row.2.is_some();
    let last_fetched_at = row.4;

    // Auto-trigger clone/sync if both github_url and deploy_key are set
    if let Some(url) = &github_url {
        match clone_status.as_str() {
            "pending" | "error" => {
                let deploy_key =
                    get_deploy_key(&state.pool, id, state.extensions.encryption.as_ref()).await?;
                let pool = state.pool.clone();
                let repo_mgr = state.repo_manager.clone();
                let url = url.clone();
                tokio::spawn(async move {
                    if let Err(e) = repo_mgr
                        .clone_repo(&pool, id, &url, deploy_key.as_deref())
                        .await
                    {
                        tracing::error!("Failed to clone repo {id}: {e}");
                    }
                });
                return Ok(Json(RepoSettingsResponse {
                    github_url,
                    clone_status: "cloning".into(),
                    has_deploy_key,
                    last_fetched_at,
                }));
            }
            "ready" => {
                // Fetch latest
                let deploy_key =
                    get_deploy_key(&state.pool, id, state.extensions.encryption.as_ref()).await?;
                state
                    .repo_manager
                    .fetch_repo(id, deploy_key.as_deref())
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                sqlx::query("UPDATE repos SET last_fetched_at = now() WHERE id = $1")
                    .bind(id)
                    .execute(&state.pool)
                    .await
                    .ok();
            }
            _ => {} // cloning — do nothing
        }
    }

    Ok(Json(RepoSettingsResponse {
        github_url,
        clone_status,
        has_deploy_key,
        last_fetched_at,
    }))
}
