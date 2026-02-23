use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegisterRepoRequest {
    pub org_name: String,
    pub repo_name: String,
    pub github_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterRepoResponse {
    pub repo_id: Uuid,
}

pub async fn register_repo(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterRepoRequest>,
) -> Result<(StatusCode, Json<RegisterRepoResponse>), (StatusCode, String)> {
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orgs (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id",
    )
    .bind(&req.org_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let repo_id: Uuid = sqlx::query_scalar(
        "INSERT INTO repos (org_id, name, github_url) VALUES ($1, $2, $3) ON CONFLICT (org_id, name) DO UPDATE SET github_url = COALESCE(EXCLUDED.github_url, repos.github_url) RETURNING id",
    )
    .bind(org_id)
    .bind(&req.repo_name)
    .bind(&req.github_url)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(RegisterRepoResponse { repo_id })))
}
