use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::generate_api_key;
use crate::extractors::AuthUser;

#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String,
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

pub async fn create_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), (StatusCode, String)> {
    let (raw_key, key_hash) = generate_api_key();

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO api_keys (org_id, key_hash, name) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(auth.org_id)
    .bind(&key_hash)
    .bind(&req.name)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(CreateApiKeyResponse {
            id,
            key: raw_key,
            name: req.name,
        }),
    ))
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ApiKeyResponse>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, LEFT(key_hash, 8), created_at FROM api_keys WHERE org_id = $1 ORDER BY created_at",
    )
    .bind(auth.org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let keys = rows
        .into_iter()
        .map(|r| ApiKeyResponse {
            id: r.0,
            name: r.1,
            key_prefix: format!("tvk_...{}", r.2),
            created_at: r.3,
        })
        .collect();

    Ok(Json(keys))
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("DELETE FROM api_keys WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(auth.org_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
