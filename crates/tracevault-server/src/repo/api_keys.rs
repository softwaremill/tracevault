use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
}

pub struct ApiKeyRepo;

impl ApiKeyRepo {
    pub async fn create(
        pool: &PgPool,
        org_id: Uuid,
        key_hash: &str,
        name: &str,
    ) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO api_keys (org_id, key_hash, name) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(org_id)
        .bind(key_hash)
        .bind(name)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    pub async fn list(pool: &PgPool, org_id: Uuid) -> Result<Vec<ApiKeyRow>, AppError> {
        let rows = sqlx::query_as::<_, ApiKeyRow>(
            "SELECT id, name, LEFT(key_hash, 8) AS key_prefix, created_at \
             FROM api_keys WHERE org_id = $1 ORDER BY created_at",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    pub async fn delete(pool: &PgPool, id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND org_id = $2")
            .bind(id)
            .bind(org_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
