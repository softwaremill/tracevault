use crate::extensions::FeatureFlags;
use crate::AppState;
use axum::{extract::State, Json};

pub async fn get_features(State(state): State<AppState>) -> Json<FeatureFlags> {
    Json(state.extensions.features.clone())
}
