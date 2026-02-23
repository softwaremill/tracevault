use axum::{extract::State, http::StatusCode, Json};
use crate::AppState;

pub async fn list_policies(
    State(_state): State<AppState>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "policies endpoint - not yet implemented")
}

pub async fn evaluate(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "evaluate endpoint - not yet implemented")
}
