use axum::{http::StatusCode, Json};

pub async fn webhook(Json(_body): Json<serde_json::Value>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "github webhook - not yet implemented")
}
