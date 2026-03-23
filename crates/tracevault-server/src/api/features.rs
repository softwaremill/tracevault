use crate::AppState;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct FeaturesResponse {
    #[serde(flatten)]
    pub features: crate::extensions::FeatureFlags,
    pub initialized: bool,
}

pub async fn get_features(State(state): State<AppState>) -> Json<FeaturesResponse> {
    let initialized = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0)
        > 0;

    Json(FeaturesResponse {
        features: state.extensions.features.clone(),
        initialized,
    })
}
