use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::generate_api_key;
use crate::error::AppError;
use crate::extractors::OrgAuth;
use crate::repo::api_keys::ApiKeyRepo;

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
    auth: OrgAuth,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), AppError> {
    let (raw_key, key_hash) = generate_api_key();

    let id = ApiKeyRepo::create(&state.pool, auth.org_id, &key_hash, &req.name).await?;

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
    auth: OrgAuth,
) -> Result<Json<Vec<ApiKeyResponse>>, AppError> {
    let rows = ApiKeyRepo::list(&state.pool, auth.org_id).await?;

    let keys = rows
        .into_iter()
        .map(|r| ApiKeyResponse {
            id: r.id,
            name: r.name,
            key_prefix: format!("tvk_...{}", r.key_prefix),
            created_at: r.created_at,
        })
        .collect();

    Ok(Json(keys))
}

pub async fn delete_api_key(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, id)): Path<(String, Uuid)>,
) -> Result<StatusCode, AppError> {
    ApiKeyRepo::delete(&state.pool, id, auth.org_id).await?;

    Ok(StatusCode::OK)
}
