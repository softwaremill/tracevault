use axum::{extract::{Query, State}, http::StatusCode};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct TokenQuery {
    pub group_by: Option<String>,
    pub period: Option<String>,
}

pub async fn token_analytics(
    State(_pool): State<PgPool>,
    Query(_query): Query<TokenQuery>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "analytics endpoint - not yet implemented")
}
