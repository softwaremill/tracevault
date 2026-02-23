use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::extractors::AuthUser;

#[derive(Debug, Deserialize)]
pub struct CreateTraceRequest {
    pub repo_name: String,
    pub org_name: String,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub tool_version: Option<String>,
    pub ai_percentage: Option<f32>,
    pub total_tokens: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub api_calls: Option<i32>,
    pub session_data: Option<serde_json::Value>,
    pub attribution: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct TraceResponse {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub ai_percentage: Option<f32>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TraceQuery {
    pub repo: Option<String>,
    pub sha: Option<String>,
    pub author: Option<String>,
    pub limit: Option<i64>,
}

pub async fn create_trace(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Json(req): Json<CreateTraceRequest>,
) -> Result<(StatusCode, Json<TraceResponse>), (StatusCode, String)> {
    // Ensure org exists (create if not)
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orgs (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id"
    )
    .bind(&req.org_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Ensure repo exists (create if not)
    let repo_id: Uuid = sqlx::query_scalar(
        "INSERT INTO repos (org_id, name) VALUES ($1, $2) ON CONFLICT (org_id, name) DO UPDATE SET name = $2 RETURNING id"
    )
    .bind(org_id)
    .bind(&req.repo_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Insert trace
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>, String, Option<String>, Option<String>, Option<f32>, Option<i64>, Option<f64>, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO traces (repo_id, commit_sha, branch, author, model, tool, tool_version, ai_percentage, total_tokens, input_tokens, output_tokens, estimated_cost_usd, api_calls, session_data, attribution)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         RETURNING id, commit_sha, branch, author, model, tool, ai_percentage, total_tokens, estimated_cost_usd, created_at"
    )
    .bind(repo_id)
    .bind(&req.commit_sha)
    .bind(&req.branch)
    .bind(&req.author)
    .bind(&req.model)
    .bind(&req.tool)
    .bind(&req.tool_version)
    .bind(req.ai_percentage)
    .bind(req.total_tokens)
    .bind(req.input_tokens)
    .bind(req.output_tokens)
    .bind(req.estimated_cost_usd)
    .bind(req.api_calls)
    .bind(&req.session_data)
    .bind(&req.attribution)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(TraceResponse {
        id: row.0,
        repo_id,
        commit_sha: row.1,
        branch: row.2,
        author: row.3,
        model: row.4,
        tool: row.5,
        ai_percentage: row.6,
        total_tokens: row.7,
        estimated_cost_usd: row.8,
        created_at: row.9,
    })))
}

pub async fn get_trace(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, String, Option<String>, Option<String>, Option<f32>, Option<i64>, Option<f64>, Option<serde_json::Value>, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, repo_id, commit_sha, branch, author, model, tool, ai_percentage, total_tokens, estimated_cost_usd, session_data, attribution, created_at FROM traces WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Trace not found".into()))?;

    Ok(Json(serde_json::json!({
        "id": row.0,
        "repo_id": row.1,
        "commit_sha": row.2,
        "branch": row.3,
        "author": row.4,
        "model": row.5,
        "tool": row.6,
        "ai_percentage": row.7,
        "total_tokens": row.8,
        "estimated_cost_usd": row.9,
        "session_data": row.10,
        "attribution": row.11,
        "created_at": row.12,
    })))
}

pub async fn list_traces(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Query(query): Query<TraceQuery>,
) -> Result<Json<Vec<TraceResponse>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).min(200);

    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, String, Option<String>, Option<String>, Option<f32>, Option<i64>, Option<f64>, chrono::DateTime<chrono::Utc>)>(
        "SELECT t.id, t.repo_id, t.commit_sha, t.branch, t.author, t.model, t.tool, t.ai_percentage, t.total_tokens, t.estimated_cost_usd, t.created_at
         FROM traces t
         LEFT JOIN repos r ON t.repo_id = r.id
         WHERE ($1::TEXT IS NULL OR r.name = $1)
           AND ($2::TEXT IS NULL OR t.commit_sha = $2)
           AND ($3::TEXT IS NULL OR t.author = $3)
         ORDER BY t.created_at DESC
         LIMIT $4"
    )
    .bind(&query.repo)
    .bind(&query.sha)
    .bind(&query.author)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let traces: Vec<TraceResponse> = rows.into_iter().map(|r| TraceResponse {
        id: r.0,
        repo_id: r.1,
        commit_sha: r.2,
        branch: r.3,
        author: r.4,
        model: r.5,
        tool: r.6,
        ai_percentage: r.7,
        total_tokens: r.8,
        estimated_cost_usd: r.9,
        created_at: r.10,
    }).collect();

    Ok(Json(traces))
}
