use axum::{extract::{Query, State}, http::StatusCode};
use serde::Deserialize;
use crate::AppState;

#[derive(Deserialize)]
pub struct TokenQuery {
    #[allow(dead_code)]
    group_by: Option<String>,
    #[allow(dead_code)]
    period: Option<String>,
}

pub async fn token_analytics(
    State(_state): State<AppState>,
    Query(_query): Query<TokenQuery>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "analytics endpoint - not yet implemented")
}
