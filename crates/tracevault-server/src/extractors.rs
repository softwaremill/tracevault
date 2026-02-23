use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::sha256_hex;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub role: String,
}

impl FromRequestParts<PgPool> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        pool: &PgPool,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header",
            ))?;

        let token_hash = sha256_hex(header);

        // Try auth_sessions first
        let session_row = sqlx::query_as::<_, (Uuid, Uuid, String)>(
            "SELECT u.id, u.org_id, u.role FROM auth_sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token_hash = $1 AND s.expires_at > NOW()",
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        if let Some((user_id, org_id, role)) = session_row {
            return Ok(AuthUser {
                user_id,
                org_id,
                role,
            });
        }

        // Fall back to api_keys
        let api_key_row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT org_id FROM api_keys WHERE key_hash = $1",
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        if let Some((org_id,)) = api_key_row {
            return Ok(AuthUser {
                user_id: Uuid::nil(),
                org_id,
                role: "member".to_string(),
            });
        }

        Err((StatusCode::UNAUTHORIZED, "Invalid or expired token"))
    }
}
