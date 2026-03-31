use axum::{
    extract::{Path, State},
    Json,
};
use tracevault_core::streaming::{CommitPushRequest, CommitPushResponse};
use uuid::Uuid;

use crate::error::AppError;
use crate::repo::commits::{CommitRepo, UpsertCommit};
use crate::service::attribution::AttributionService;
use crate::{extractors::OrgAuth, AppState};

/// POST /api/v1/orgs/{slug}/repos/{repo_id}/commits
pub async fn handle_commit_push(
    State(state): State<AppState>,
    _auth: OrgAuth,
    Path((_slug, repo_id)): Path<(String, Uuid)>,
    Json(req): Json<CommitPushRequest>,
) -> Result<Json<CommitPushResponse>, AppError> {
    // 1. Upsert commit
    let commit_db_id = CommitRepo::upsert(
        &state.pool,
        &UpsertCommit {
            repo_id,
            commit_sha: req.commit_sha.clone(),
            branch: req.branch.clone(),
            author: req.author.clone(),
            message: req.message.clone(),
            diff_data: req.diff_data.clone(),
            committed_at: req.committed_at,
        },
    )
    .await?;

    // 2. Line-level attribution + summary
    let attributions_count = if let Some(diff_data) = &req.diff_data {
        let committed_at = req.committed_at.unwrap_or_else(chrono::Utc::now);
        let count = AttributionService::attribute_commit(
            &state.pool,
            commit_db_id,
            repo_id,
            diff_data,
            committed_at,
        )
        .await?;

        // Compute and store attribution summary from commit_attributions
        AttributionService::compute_summary(&state.pool, commit_db_id, diff_data).await?;

        count
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
        .await?;
    }

    // 4. Seal commit (if signing enabled for this org)
    if let Err(e) = crate::service::sealing::SealingService::seal_commit(
        &state.pool,
        commit_db_id,
        state.encryption_key.as_deref(),
    ).await {
        tracing::warn!("Failed to seal commit {}: {e}", req.commit_sha);
    }

    Ok(Json(CommitPushResponse {
        commit_db_id,
        attributions_count,
    }))
}
