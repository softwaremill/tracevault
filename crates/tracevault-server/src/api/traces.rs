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
    pub model_usage: Option<serde_json::Value>,
    pub duration_ms: Option<i64>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub user_messages: Option<i32>,
    pub assistant_messages: Option<i32>,
    pub tool_calls: Option<serde_json::Value>,
    pub total_tool_calls: Option<i32>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    pub compactions: Option<i32>,
    pub compaction_tokens_saved: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateTraceResponse {
    pub commit_id: Uuid,
    pub session_id: Option<Uuid>,
    pub chain_hash: Option<String>,
    pub signature: Option<String>,
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

    // Insert commit (append-only — no UPDATE on conflict)
    let commit_row = sqlx::query_as::<_, (Uuid, bool)>(
        "WITH ins AS (
            INSERT INTO commits (repo_id, commit_sha, branch, author, diff_data, attribution)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (repo_id, commit_sha) DO NOTHING
            RETURNING id, true AS is_new
        )
        SELECT id, is_new FROM ins
        UNION ALL
        SELECT id, false AS is_new FROM commits WHERE repo_id = $1 AND commit_sha = $2
        LIMIT 1"
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

    let (commit_id, commit_is_new) = commit_row;

    // Seal the commit if it's new
    let mut resp_chain_hash = None;
    let mut resp_signature = None;

    if commit_is_new {
        let canonical = serde_json::json!({
            "commit_sha": &req.commit_sha,
            "branch": &req.branch,
            "author": &req.author,
            "repo_id": repo_id,
        });
        let canonical_bytes = serde_json::to_vec(&canonical).unwrap();
        let record_hash = state.signing.record_hash(&canonical_bytes);

        // Get previous chain hash
        let prev: Option<String> = sqlx::query_scalar(
            "SELECT chain_hash FROM commits
             WHERE repo_id = $1 AND sealed_at IS NOT NULL
             ORDER BY sealed_at DESC LIMIT 1"
        )
        .bind(repo_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let chain_hash = state.signing.chain_hash(prev.as_deref(), &record_hash);
        let signature = state.signing.sign(&record_hash);

        sqlx::query(
            "UPDATE commits SET record_hash = $1, chain_hash = $2, prev_chain_hash = $3,
             signature = $4, sealed_at = NOW() WHERE id = $5"
        )
        .bind(&record_hash)
        .bind(&chain_hash)
        .bind(&prev)
        .bind(&signature)
        .bind(commit_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        resp_chain_hash = Some(chain_hash);
        resp_signature = Some(signature);
    }

    // Optionally insert session (append-only)
    let session_db_id = if let Some(sid) = &req.session_id {
        // Compute cost server-side from model_usage or fallback fields
        let estimated_cost = crate::pricing::cost_from_model_usage(
            req.model_usage.as_ref(),
            req.model.as_deref(),
            req.input_tokens.unwrap_or(0),
            req.output_tokens.unwrap_or(0),
            req.cache_read_tokens.unwrap_or(0),
            req.cache_write_tokens.unwrap_or(0),
        );
        let cost = if estimated_cost > 0.0 {
            Some(estimated_cost)
        } else {
            req.estimated_cost_usd
        };

        let session_row = sqlx::query_as::<_, (Uuid, bool)>(
            "WITH ins AS (
                INSERT INTO sessions (commit_id, session_id, model, tool, total_tokens, input_tokens, output_tokens,
                    estimated_cost_usd, api_calls, session_data, transcript, model_usage,
                    duration_ms, started_at, ended_at, user_messages, assistant_messages,
                    tool_calls, total_tool_calls, cache_read_tokens, cache_write_tokens,
                    compactions, compaction_tokens_saved)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                        $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
                ON CONFLICT (commit_id, session_id) DO NOTHING
                RETURNING id, true AS is_new
            )
            SELECT id, is_new FROM ins
            UNION ALL
            SELECT id, false AS is_new FROM sessions WHERE commit_id = $1 AND session_id = $2
            LIMIT 1"
        )
        .bind(commit_id)
        .bind(sid)
        .bind(&req.model)
        .bind(&req.tool)
        .bind(req.total_tokens)
        .bind(req.input_tokens)
        .bind(req.output_tokens)
        .bind(cost)
        .bind(req.api_calls)
        .bind(&req.session_data)
        .bind(&req.transcript)
        .bind(&req.model_usage)
        .bind(req.duration_ms)
        .bind(req.started_at)
        .bind(req.ended_at)
        .bind(req.user_messages)
        .bind(req.assistant_messages)
        .bind(&req.tool_calls)
        .bind(req.total_tool_calls)
        .bind(req.cache_read_tokens)
        .bind(req.cache_write_tokens)
        .bind(req.compactions)
        .bind(req.compaction_tokens_saved)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let (session_db_id, session_is_new) = session_row;

        // Seal the session if new
        if session_is_new {
            let canonical = serde_json::json!({
                "session_id": sid,
                "commit_id": commit_id,
                "model": &req.model,
                "tool": &req.tool,
            });
            let canonical_bytes = serde_json::to_vec(&canonical).unwrap();
            let record_hash = state.signing.record_hash(&canonical_bytes);
            let signature = state.signing.sign(&record_hash);

            sqlx::query(
                "UPDATE sessions SET record_hash = $1, signature = $2, sealed_at = NOW() WHERE id = $3"
            )
            .bind(&record_hash)
            .bind(&signature)
            .bind(session_db_id)
            .execute(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        }

        Some(session_db_id)
    } else {
        None
    };

    // Audit log
    crate::audit::log(&state.pool, crate::audit::user_action(
        auth.org_id, auth.user_id,
        "trace.create", "commit", Some(commit_id),
        Some(serde_json::json!({"commit_sha": &req.commit_sha, "session_count": if req.session_id.is_some() { 1 } else { 0 }})),
    )).await;

    Ok((StatusCode::CREATED, Json(CreateTraceResponse {
        commit_id,
        session_id: session_db_id,
        chain_hash: resp_chain_hash,
        signature: resp_signature,
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
