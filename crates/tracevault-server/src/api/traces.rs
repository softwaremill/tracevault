use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use crate::AppState;
use uuid::Uuid;

use crate::extractors::AuthUser;

#[derive(Debug, Deserialize)]
pub struct CreateTraceRequest {
    pub repo_name: String,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub session_id: Option<String>,
    pub total_tokens: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub api_calls: Option<i32>,
    pub session_data: Option<serde_json::Value>,
    pub attribution: Option<serde_json::Value>,
    pub transcript: Option<serde_json::Value>,
    pub diff_data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CreateTraceResponse {
    pub commit_id: Uuid,
    pub session_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CommitListItem {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub session_count: i64,
    pub total_tokens: Option<i64>,
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
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTraceRequest>,
) -> Result<(StatusCode, Json<CreateTraceResponse>), (StatusCode, String)> {
    // Ensure repo exists under the authenticated user's org
    let repo_id: Uuid = sqlx::query_scalar(
        "INSERT INTO repos (org_id, name) VALUES ($1, $2) ON CONFLICT (org_id, name) DO UPDATE SET name = $2 RETURNING id"
    )
    .bind(auth.org_id)
    .bind(&req.repo_name)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Upsert commit
    let commit_id: Uuid = sqlx::query_scalar(
        "INSERT INTO commits (repo_id, commit_sha, branch, author, diff_data, attribution)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (repo_id, commit_sha) DO UPDATE SET
           branch = COALESCE(EXCLUDED.branch, commits.branch),
           diff_data = COALESCE(EXCLUDED.diff_data, commits.diff_data),
           attribution = COALESCE(EXCLUDED.attribution, commits.attribution)
         RETURNING id"
    )
    .bind(repo_id)
    .bind(&req.commit_sha)
    .bind(&req.branch)
    .bind(&req.author)
    .bind(&req.diff_data)
    .bind(&req.attribution)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Optionally upsert session
    let session_db_id = if let Some(sid) = &req.session_id {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO sessions (commit_id, session_id, model, tool, total_tokens, input_tokens, output_tokens,
                estimated_cost_usd, api_calls, session_data, transcript)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (commit_id, session_id) DO UPDATE SET
               model = COALESCE(EXCLUDED.model, sessions.model),
               tool = COALESCE(EXCLUDED.tool, sessions.tool),
               total_tokens = COALESCE(EXCLUDED.total_tokens, sessions.total_tokens),
               input_tokens = COALESCE(EXCLUDED.input_tokens, sessions.input_tokens),
               output_tokens = COALESCE(EXCLUDED.output_tokens, sessions.output_tokens),
               estimated_cost_usd = COALESCE(EXCLUDED.estimated_cost_usd, sessions.estimated_cost_usd),
               api_calls = COALESCE(EXCLUDED.api_calls, sessions.api_calls),
               session_data = COALESCE(EXCLUDED.session_data, sessions.session_data),
               transcript = COALESCE(EXCLUDED.transcript, sessions.transcript)
             RETURNING id"
        )
        .bind(commit_id)
        .bind(sid)
        .bind(&req.model)
        .bind(&req.tool)
        .bind(req.total_tokens)
        .bind(req.input_tokens)
        .bind(req.output_tokens)
        .bind(req.estimated_cost_usd)
        .bind(req.api_calls)
        .bind(&req.session_data)
        .bind(&req.transcript)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        Some(id)
    } else {
        None
    };

    Ok((StatusCode::CREATED, Json(CreateTraceResponse {
        commit_id,
        session_id: session_db_id,
    })))
}

pub async fn get_trace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Fetch commit
    let commit = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, String, Option<serde_json::Value>, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>)>(
        "SELECT c.id, c.repo_id, c.commit_sha, c.branch, c.author, c.diff_data, c.attribution, c.created_at
         FROM commits c JOIN repos r ON c.repo_id = r.id
         WHERE c.id = $1 AND r.org_id = $2"
    )
    .bind(id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Commit not found".into()))?;

    // Fetch sessions for this commit
    let sessions = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>, Option<i64>, Option<i64>, Option<i64>, Option<f64>, Option<i32>, Option<serde_json::Value>, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, session_id, model, tool, total_tokens, input_tokens, output_tokens,
                estimated_cost_usd, api_calls, session_data, transcript, created_at
         FROM sessions WHERE commit_id = $1 ORDER BY created_at ASC"
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let sessions_json: Vec<serde_json::Value> = sessions.into_iter().map(|s| {
        serde_json::json!({
            "id": s.0,
            "session_id": s.1,
            "model": s.2,
            "tool": s.3,
            "total_tokens": s.4,
            "input_tokens": s.5,
            "output_tokens": s.6,
            "estimated_cost_usd": s.7,
            "api_calls": s.8,
            "session_data": s.9,
            "transcript": s.10,
            "created_at": s.11,
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "id": commit.0,
        "repo_id": commit.1,
        "commit_sha": commit.2,
        "branch": commit.3,
        "author": commit.4,
        "diff_data": commit.5,
        "attribution": commit.6,
        "created_at": commit.7,
        "sessions": sessions_json,
    })))
}

pub async fn list_traces(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<TraceQuery>,
) -> Result<Json<Vec<CommitListItem>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).min(200);

    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, String, i64, Option<i64>, chrono::DateTime<chrono::Utc>)>(
        "SELECT c.id, c.repo_id, c.commit_sha, c.branch, c.author,
                COUNT(s.id) as session_count,
                CAST(SUM(s.total_tokens) AS BIGINT) as total_tokens,
                c.created_at
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.commit_sha = $3)
           AND ($4::TEXT IS NULL OR c.author = $4)
         GROUP BY c.id
         ORDER BY c.created_at DESC
         LIMIT $5"
    )
    .bind(auth.org_id)
    .bind(&query.repo)
    .bind(&query.sha)
    .bind(&query.author)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<CommitListItem> = rows.into_iter().map(|r| CommitListItem {
        id: r.0,
        repo_id: r.1,
        commit_sha: r.2,
        branch: r.3,
        author: r.4,
        session_count: r.5,
        total_tokens: r.6,
        created_at: r.7,
    }).collect();

    Ok(Json(items))
}
