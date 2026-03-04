use axum::{extract::State, Json};
use crate::AppState;
use crate::extensions::FeatureFlags;

pub async fn get_features(State(state): State<AppState>) -> Json<FeatureFlags> {
    Json(state.extensions.features.clone())
}
