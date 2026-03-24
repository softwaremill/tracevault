use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Track a commit reaching a branch (or merge target).
///
/// Looks up the commit in `commits_v2` by repo_id + commit_sha.
/// If the commit isn't in v2 yet, silently returns Ok (it may arrive later via commit-push).
/// Inserts into `branch_tracking` with ON CONFLICT DO NOTHING to avoid duplicates.
pub async fn track_commit_on_branch(
    pool: &PgPool,
    commit_sha: &str,
    repo_id: Uuid,
    branch: &str,
    tracked_at: DateTime<Utc>,
    tracking_type: &str,
) -> Result<(), String> {
    let commit_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM commits_v2 WHERE repo_id = $1 AND commit_sha = $2")
            .bind(repo_id)
            .bind(commit_sha)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Failed to look up commit: {e}"))?;

    let Some(commit_id) = commit_id else {
        return Ok(());
    };

    sqlx::query(
        "INSERT INTO branch_tracking (commit_id, branch, tracked_at, tracking_type)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (commit_id, branch) DO NOTHING",
    )
    .bind(commit_id)
    .bind(branch)
    .bind(tracked_at)
    .bind(tracking_type)
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to insert branch tracking: {e}"))?;

    Ok(())
}

/// Track a commit being tagged.
///
/// Same pattern as `track_commit_on_branch` but with tracking_type = "tag"
/// and the tag name stored in the branch column.
pub async fn track_tag(
    pool: &PgPool,
    commit_sha: &str,
    repo_id: Uuid,
    tag: &str,
    tracked_at: DateTime<Utc>,
) -> Result<(), String> {
    track_commit_on_branch(pool, commit_sha, repo_id, tag, tracked_at, "tag").await
}
