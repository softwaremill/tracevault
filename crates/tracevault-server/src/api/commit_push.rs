use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracevault_core::streaming::{CommitPushRequest, CommitPushResponse};
use uuid::Uuid;

use crate::{extractors::OrgAuth, AppState};

/// POST /api/v1/orgs/{slug}/repos/{repo_id}/commits
pub async fn handle_commit_push(
    State(state): State<AppState>,
    _auth: OrgAuth,
    Path((_slug, repo_id)): Path<(String, Uuid)>,
    Json(req): Json<CommitPushRequest>,
) -> Result<Json<CommitPushResponse>, (StatusCode, String)> {
    // 1. Upsert commit
    let commit_db_id: Uuid = sqlx::query_scalar(
        "INSERT INTO commits_v2 (repo_id, commit_sha, branch, author, diff_data, committed_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (repo_id, commit_sha)
         DO UPDATE SET
           branch = COALESCE(EXCLUDED.branch, commits_v2.branch),
           diff_data = COALESCE(EXCLUDED.diff_data, commits_v2.diff_data)
         RETURNING id",
    )
    .bind(repo_id)
    .bind(&req.commit_sha)
    .bind(&req.branch)
    .bind(&req.author)
    .bind(&req.diff_data)
    .bind(req.committed_at)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 2. File-level attribution
    let file_paths = extract_file_paths(&req.diff_data);

    // Clear previous attributions for idempotency
    sqlx::query("DELETE FROM commit_attributions WHERE commit_id = $1")
        .bind(commit_db_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let committed_at = req.committed_at.unwrap_or_else(chrono::Utc::now);
    let mut attributions_count: i64 = 0;

    for file_path in &file_paths {
        // Find matching file_changes from sessions in the same repo within 24h before committed_at
        let matches = sqlx::query_as::<_, (Uuid, Uuid)>(
            "SELECT fc.session_id, fc.event_id
             FROM file_changes fc
             JOIN sessions_v2 s ON fc.session_id = s.id
             WHERE s.repo_id = $1
               AND fc.timestamp >= $2 - INTERVAL '24 hours'
               AND fc.timestamp <= $2
               AND fc.file_path LIKE '%' || $3",
        )
        .bind(repo_id)
        .bind(committed_at)
        .bind(file_path)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for (session_id, event_id) in &matches {
            sqlx::query(
                "INSERT INTO commit_attributions (commit_id, session_id, event_id, file_path, confidence)
                 VALUES ($1, $2, $3, $4, 0.5)",
            )
            .bind(commit_db_id)
            .bind(session_id)
            .bind(event_id)
            .bind(file_path)
            .execute(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            attributions_count += 1;
        }
    }

    Ok(Json(CommitPushResponse {
        commit_db_id,
        attributions_count,
    }))
}

/// Extract file paths from diff_data JSON.
/// Expects `{ "files": [{ "path": "..." }, ...] }`.
fn extract_file_paths(diff_data: &Option<serde_json::Value>) -> Vec<String> {
    let Some(data) = diff_data else {
        return vec![];
    };
    let Some(files) = data.get("files").and_then(|f| f.as_array()) else {
        return vec![];
    };
    files
        .iter()
        .filter_map(|f| f.get("path").and_then(|p| p.as_str()).map(String::from))
        .collect()
}
