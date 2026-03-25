use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::permissions::Permission;
use crate::{audit, extractors::OrgAuth, pricing_sync, AppState};

// ── Types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PricingEntry {
    pub id: Uuid,
    pub model: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_write_per_mtok: f64,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePricingRequest {
    pub model: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_write_per_mtok: f64,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricingRequest {
    pub model: Option<String>,
    pub input_per_mtok: Option<f64>,
    pub output_per_mtok: Option<f64>,
    pub cache_read_per_mtok: Option<f64>,
    pub cache_write_per_mtok: Option<f64>,
    pub effective_from: Option<DateTime<Utc>>,
    pub effective_until: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Serialize)]
pub struct RecalculateResponse {
    pub affected_sessions: i64,
    pub total_old_cost: f64,
    pub total_new_cost: f64,
}

// ── Handlers ───────────────────────────────────────────────────────

/// GET /api/v1/orgs/{slug}/pricing
/// Pricing is global (no org_id filter) — org slug is for RBAC only.
pub async fn list_pricing(
    State(state): State<AppState>,
    _auth: OrgAuth,
) -> Result<Json<Vec<PricingEntry>>, (StatusCode, String)> {
    let entries = sqlx::query_as::<_, PricingEntry>(
        "SELECT id, model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                effective_from, effective_until, created_at, source
         FROM model_pricing
         ORDER BY model, effective_from DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(entries))
}

/// GET /api/v1/orgs/{slug}/pricing/models
/// Distinct model names from sessions (org-scoped).
pub async fn list_models(
    State(state): State<AppState>,
    auth: OrgAuth,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    let models = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT model FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         WHERE r.org_id = $1 AND s.model IS NOT NULL
         ORDER BY model",
    )
    .bind(auth.org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(models))
}

/// POST /api/v1/orgs/{slug}/pricing
pub async fn create_pricing(
    State(state): State<AppState>,
    auth: OrgAuth,
    Json(req): Json<CreatePricingRequest>,
) -> Result<(StatusCode, Json<PricingEntry>), (StatusCode, String)> {
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::OrgSettingsManage)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let row = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
        "INSERT INTO model_pricing (model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, effective_from, effective_until)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, created_at",
    )
    .bind(&req.model)
    .bind(req.input_per_mtok)
    .bind(req.output_per_mtok)
    .bind(req.cache_read_per_mtok)
    .bind(req.cache_write_per_mtok)
    .bind(req.effective_from)
    .bind(req.effective_until)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (id, created_at) = row;

    audit::log(
        &state.pool,
        audit::user_action(
            auth.org_id,
            auth.user_id,
            "create",
            "model_pricing",
            Some(id),
            Some(serde_json::json!({
                "model": req.model,
                "input_per_mtok": req.input_per_mtok,
                "output_per_mtok": req.output_per_mtok,
                "effective_from": req.effective_from,
            })),
        ),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(PricingEntry {
            id,
            model: req.model,
            input_per_mtok: req.input_per_mtok,
            output_per_mtok: req.output_per_mtok,
            cache_read_per_mtok: req.cache_read_per_mtok,
            cache_write_per_mtok: req.cache_write_per_mtok,
            effective_from: req.effective_from,
            effective_until: req.effective_until,
            created_at,
            source: "manual".to_string(),
        }),
    ))
}

/// PUT /api/v1/orgs/{slug}/pricing/{id}
pub async fn update_pricing(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, pricing_id)): Path<(String, Uuid)>,
    Json(req): Json<UpdatePricingRequest>,
) -> Result<Json<PricingEntry>, (StatusCode, String)> {
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::OrgSettingsManage)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let existing = sqlx::query_as::<_, PricingEntry>(
        "SELECT id, model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                effective_from, effective_until, created_at, source
         FROM model_pricing WHERE id = $1",
    )
    .bind(pricing_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Pricing entry not found".into()))?;

    let new_model = req.model.unwrap_or(existing.model);
    let new_input = req.input_per_mtok.unwrap_or(existing.input_per_mtok);
    let new_output = req.output_per_mtok.unwrap_or(existing.output_per_mtok);
    let new_cache_read = req
        .cache_read_per_mtok
        .unwrap_or(existing.cache_read_per_mtok);
    let new_cache_write = req
        .cache_write_per_mtok
        .unwrap_or(existing.cache_write_per_mtok);
    let new_from = req.effective_from.unwrap_or(existing.effective_from);
    let new_until = req.effective_until.unwrap_or(existing.effective_until);

    sqlx::query(
        "UPDATE model_pricing
         SET model = $1, input_per_mtok = $2, output_per_mtok = $3,
             cache_read_per_mtok = $4, cache_write_per_mtok = $5,
             effective_from = $6, effective_until = $7
         WHERE id = $8",
    )
    .bind(&new_model)
    .bind(new_input)
    .bind(new_output)
    .bind(new_cache_read)
    .bind(new_cache_write)
    .bind(new_from)
    .bind(new_until)
    .bind(pricing_id)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    audit::log(
        &state.pool,
        audit::user_action(
            auth.org_id,
            auth.user_id,
            "update",
            "model_pricing",
            Some(pricing_id),
            Some(serde_json::json!({ "model": new_model })),
        ),
    )
    .await;

    Ok(Json(PricingEntry {
        id: pricing_id,
        model: new_model,
        input_per_mtok: new_input,
        output_per_mtok: new_output,
        cache_read_per_mtok: new_cache_read,
        cache_write_per_mtok: new_cache_write,
        effective_from: new_from,
        effective_until: new_until,
        created_at: existing.created_at,
        source: existing.source,
    }))
}

/// POST /api/v1/orgs/{slug}/pricing/{id}/recalculate
/// Recalculate session costs for this pricing entry's date range.
pub async fn recalculate(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, pricing_id)): Path<(String, Uuid)>,
) -> Result<Json<RecalculateResponse>, (StatusCode, String)> {
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::OrgSettingsManage)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let pricing = sqlx::query_as::<
        _,
        (
            String,
            f64,
            f64,
            f64,
            f64,
            DateTime<Utc>,
            Option<DateTime<Utc>>,
        ),
    >(
        "SELECT model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                effective_from, effective_until
         FROM model_pricing WHERE id = $1",
    )
    .bind(pricing_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Pricing entry not found".into()))?;

    let (
        canonical_model,
        input_per_m,
        output_per_m,
        cache_read_per_m,
        cache_write_per_m,
        effective_from,
        effective_until,
    ) = pricing;

    let pricing_data = crate::pricing::ModelPricing {
        input_per_m,
        output_per_m,
        cache_write_per_m,
        cache_read_per_m,
    };

    let result = crate::pricing::recalculate_sessions_for_pricing(
        &state.pool,
        &canonical_model,
        &pricing_data,
        effective_from,
        effective_until,
        Some(auth.user_id),
        Some(auth.org_id),
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RecalculateResponse {
        affected_sessions: result.affected_sessions,
        total_old_cost: result.total_old_cost,
        total_new_cost: result.total_new_cost,
    }))
}

// ── Sync endpoints ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub models_updated: Vec<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SyncStatusResponse {
    pub last_synced_at: Option<DateTime<Utc>>,
}

/// POST /api/v1/orgs/{slug}/pricing/sync
pub async fn trigger_sync(
    State(state): State<AppState>,
    auth: OrgAuth,
) -> Result<Json<SyncResponse>, (StatusCode, String)> {
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::OrgSettingsManage)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    // Rate limit: skip if last sync was <5 minutes ago
    if let Some(last) = pricing_sync::last_sync_time(&state.pool).await {
        let elapsed = Utc::now() - last;
        if elapsed.num_seconds() < 300 {
            return Ok(Json(SyncResponse {
                models_updated: vec![],
                last_synced_at: Some(last),
            }));
        }
    }

    let result = pricing_sync::sync_pricing(&state.pool, &state.http_client)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    audit::log(
        &state.pool,
        audit::user_action(
            auth.org_id,
            auth.user_id,
            "pricing_sync",
            "model_pricing",
            None,
            Some(serde_json::json!({
                "models_updated": result.models_updated,
            })),
        ),
    )
    .await;

    Ok(Json(SyncResponse {
        models_updated: result.models_updated,
        last_synced_at: Some(result.last_synced_at),
    }))
}

/// GET /api/v1/orgs/{slug}/pricing/sync/status
pub async fn sync_status(
    State(state): State<AppState>,
    _auth: OrgAuth,
) -> Result<Json<SyncStatusResponse>, (StatusCode, String)> {
    let last = pricing_sync::last_sync_time(&state.pool).await;
    Ok(Json(SyncStatusResponse {
        last_synced_at: last,
    }))
}
