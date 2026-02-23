use axum::{http::StatusCode, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub org_name: String,
}

pub async fn register(
    Json(_req): Json<RegisterRequest>,
) -> (StatusCode, &'static str) {
    // TODO: implement org registration with API key generation
    (StatusCode::OK, "registration endpoint - not yet implemented")
}
