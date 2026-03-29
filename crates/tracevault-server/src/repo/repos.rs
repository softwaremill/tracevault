use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct RepoRow {
    pub id: Uuid,
    pub name: String,
    pub github_url: Option<String>,
    pub clone_status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ReadyRepo {
    pub id: Uuid,
    pub deploy_key_encrypted: Option<String>,
}

pub struct GitRepoRepo;

impl GitRepoRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: Uuid,
        name: &str,
        github_url: Option<&str>,
    ) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO repos (org_id, name, github_url) VALUES ($1, $2, $3) \
             ON CONFLICT (org_id, name) DO UPDATE SET github_url = COALESCE(EXCLUDED.github_url, repos.github_url) \
             RETURNING id",
        )
        .bind(org_id)
        .bind(name)
        .bind(github_url)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    pub async fn list(pool: &PgPool, org_id: Uuid) -> Result<Vec<RepoRow>, AppError> {
        let rows = sqlx::query_as::<_, RepoRow>(
            "SELECT id, name, github_url, clone_status, created_at \
             FROM repos WHERE org_id = $1 ORDER BY name",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn list_ready_for_sync(pool: &PgPool) -> Result<Vec<ReadyRepo>, AppError> {
        let rows = sqlx::query_as::<_, ReadyRepo>(
            "SELECT id, deploy_key_encrypted FROM repos WHERE clone_status = 'ready'",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn mark_fetched(pool: &PgPool, repo_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE repos SET last_fetched_at = now() WHERE id = $1")
            .bind(repo_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn set_clone_status(
        pool: &PgPool,
        repo_id: Uuid,
        status: &str,
        clone_path: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE repos SET clone_status = $2, clone_path = COALESCE($3, clone_path) WHERE id = $1",
        )
        .bind(repo_id)
        .bind(status)
        .bind(clone_path)
        .execute(pool)
        .await?;

        Ok(())
    }
}
