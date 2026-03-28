use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub struct UpsertCommit {
    pub repo_id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub message: Option<String>,
    pub diff_data: Option<serde_json::Value>,
    pub committed_at: Option<DateTime<Utc>>,
}

pub struct InsertAttribution {
    pub commit_id: Uuid,
    pub session_id: Uuid,
    pub event_id: Uuid,
    pub file_path: String,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub confidence: f32,
}

#[derive(sqlx::FromRow)]
pub struct FileChangeMatch {
    pub session_id: Uuid,
    pub event_id: Uuid,
    pub change_type: String,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
    pub diff_text: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct AttributionRow {
    pub file_path: String,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
}

pub struct CommitRepo;

impl CommitRepo {
    /// INSERT INTO commits ... ON CONFLICT DO UPDATE RETURNING id.
    pub async fn upsert(pool: &PgPool, req: &UpsertCommit) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO commits (repo_id, commit_sha, branch, author, message, diff_data, committed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (repo_id, commit_sha)
             DO UPDATE SET
               branch = COALESCE(EXCLUDED.branch, commits.branch),
               message = COALESCE(EXCLUDED.message, commits.message),
               diff_data = COALESCE(EXCLUDED.diff_data, commits.diff_data)
             RETURNING id",
        )
        .bind(req.repo_id)
        .bind(&req.commit_sha)
        .bind(&req.branch)
        .bind(&req.author)
        .bind(&req.message)
        .bind(&req.diff_data)
        .bind(req.committed_at)
        .fetch_one(pool)
        .await?;
        Ok(id)
    }

    /// DELETE FROM commit_attributions WHERE commit_id = $1.
    pub async fn clear_attributions(pool: &PgPool, commit_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM commit_attributions WHERE commit_id = $1")
            .bind(commit_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// INSERT INTO commit_attributions.
    pub async fn insert_attribution(
        pool: &PgPool,
        req: &InsertAttribution,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO commit_attributions
                (commit_id, session_id, event_id, file_path, line_start, line_end, confidence)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(req.commit_id)
        .bind(req.session_id)
        .bind(req.event_id)
        .bind(&req.file_path)
        .bind(req.line_start)
        .bind(req.line_end)
        .bind(req.confidence)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// UPDATE commits SET attribution = $2 WHERE id = $1.
    pub async fn update_attribution_summary(
        pool: &PgPool,
        commit_id: Uuid,
        summary: &serde_json::Value,
    ) -> Result<(), AppError> {
        sqlx::query("UPDATE commits SET attribution = $1 WHERE id = $2")
            .bind(summary)
            .bind(commit_id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Find file changes matching a file path within 48h before committed_at.
    pub async fn find_matching_file_changes(
        pool: &PgPool,
        repo_id: Uuid,
        committed_at: DateTime<Utc>,
        file_path: &str,
    ) -> Result<Vec<FileChangeMatch>, AppError> {
        let rows = sqlx::query_as::<_, FileChangeMatch>(
            "SELECT fc.session_id, fc.event_id, fc.change_type,
                    fc.line_start, fc.line_end, fc.diff_text
             FROM file_changes fc
             JOIN sessions s ON fc.session_id = s.id
             WHERE s.repo_id = $1
               AND fc.timestamp >= $2 - INTERVAL '48 hours'
               AND fc.timestamp <= $2
               AND fc.file_path LIKE '%' || $3",
        )
        .bind(repo_id)
        .bind(committed_at)
        .bind(file_path)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// SELECT file_path, line_start, line_end FROM commit_attributions WHERE commit_id = $1.
    pub async fn get_attributions(
        pool: &PgPool,
        commit_id: Uuid,
    ) -> Result<Vec<AttributionRow>, AppError> {
        let rows = sqlx::query_as::<_, AttributionRow>(
            "SELECT file_path, line_start, line_end FROM commit_attributions WHERE commit_id = $1",
        )
        .bind(commit_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
