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

    // 2. Line-level attribution
    let attributions_count = if let Some(diff_data) = &req.diff_data {
        let committed_at = req.committed_at.unwrap_or_else(chrono::Utc::now);
        crate::attribution::attribute_commit(
            &state.pool,
            commit_db_id,
            repo_id,
            diff_data,
            committed_at,
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    } else {
        0
    };

    // 3. Track commit on its branch
    if let Some(branch) = &req.branch {
        let tracked_at = req.committed_at.unwrap_or_else(chrono::Utc::now);
        sqlx::query(
            "INSERT INTO branch_tracking (commit_id, branch, tracked_at, tracking_type)
             VALUES ($1, $2, $3, 'push')
             ON CONFLICT (commit_id, branch) DO NOTHING",
        )
        .bind(commit_db_id)
        .bind(branch)
        .bind(tracked_at)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Json(CommitPushResponse {
        commit_db_id,
        attributions_count,
    }))
}
