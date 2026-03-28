use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UserInfo {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserWithHash {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub password_hash: String,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct OrgMembership {
    pub org_id: Uuid,
    pub org_name: String,
    pub display_name: Option<String>,
    pub role: String,
}

pub struct UserRepo;

impl UserRepo {
    pub async fn find_by_token_hash(
        pool: &PgPool,
        token_hash: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT user_id FROM auth_sessions WHERE token_hash = $1 AND expires_at > NOW()",
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|(id,)| id))
    }

    pub async fn find_org_by_api_key_hash(
        pool: &PgPool,
        key_hash: &str,
    ) -> Result<Option<Uuid>, AppError> {
        let row = sqlx::query_as::<_, (Uuid,)>("SELECT org_id FROM api_keys WHERE key_hash = $1")
            .bind(key_hash)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(|(id,)| id))
    }

    pub async fn get_org_role(
        pool: &PgPool,
        user_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT role FROM user_org_memberships WHERE user_id = $1 AND org_id = $2",
        )
        .bind(user_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|(role,)| role))
    }

    pub async fn email_exists(pool: &PgPool, email: &str) -> Result<bool, AppError> {
        let row = sqlx::query_as::<_, (Uuid,)>("SELECT id FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await?;

        Ok(row.is_some())
    }

    pub async fn create(
        pool: &PgPool,
        email: &str,
        password_hash: &str,
        name: Option<&str>,
    ) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    pub async fn create_auth_session(
        pool: &PgPool,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Uuid, AppError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok(id)
    }

    pub async fn delete_auth_session(pool: &PgPool, token_hash: &str) -> Result<(), AppError> {
        sqlx::query("DELETE FROM auth_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserInfo>, AppError> {
        let row = sqlx::query_as::<_, UserInfo>("SELECT id, email, name FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(row)
    }

    pub async fn find_by_email(
        pool: &PgPool,
        email: &str,
    ) -> Result<Option<UserWithHash>, AppError> {
        let row = sqlx::query_as::<_, UserWithHash>(
            "SELECT id, email, name, password_hash FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn add_org_membership(
        pool: &PgPool,
        user_id: Uuid,
        org_id: Uuid,
        role: &str,
    ) -> Result<(), AppError> {
        sqlx::query("INSERT INTO user_org_memberships (user_id, org_id, role) VALUES ($1, $2, $3)")
            .bind(user_id)
            .bind(org_id)
            .bind(role)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn list_orgs(pool: &PgPool, user_id: Uuid) -> Result<Vec<OrgMembership>, AppError> {
        let rows = sqlx::query_as::<_, OrgMembership>(
            "SELECT o.id AS org_id, o.name AS org_name, o.display_name, m.role
             FROM user_org_memberships m
             JOIN orgs o ON m.org_id = o.id
             WHERE m.user_id = $1
             ORDER BY o.name",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }
}
