use axum::{extract::State, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn list_policies(
    State(_pool): State<PgPool>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "policies endpoint - not yet implemented")
}

pub async fn evaluate(
    State(_pool): State<PgPool>,
    Json(_body): Json<serde_json::Value>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "evaluate endpoint - not yet implemented")
}
