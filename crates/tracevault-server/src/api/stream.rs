use axum::{
    extract::{Path, State},
    Json,
};
use tracevault_core::streaming::{StreamEventRequest, StreamEventResponse};
use uuid::Uuid;

use crate::error::AppError;
use crate::service::stream::StreamService;
use crate::{extractors::OrgAuth, AppState};

/// POST /api/v1/orgs/{slug}/repos/{repo_id}/stream
pub async fn handle_stream(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, repo_id)): Path<(String, Uuid)>,
    Json(req): Json<StreamEventRequest>,
) -> Result<Json<StreamEventResponse>, AppError> {
    let response = StreamService::process(&state, auth.org_id, repo_id, auth.user_id, req).await?;
    Ok(Json(response))
}
