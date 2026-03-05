use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit;
use crate::extractors::AuthUser;
use crate::permissions::Permission;
use crate::AppState;

// --- Compliance Settings ---

#[derive(Debug, Serialize)]
pub struct ComplianceSettingsResponse {
    pub org_id: Uuid,
    pub retention_days: i32,
    pub signing_enabled: bool,
    pub signing_key_id: Option<String>,
    pub chain_verification_interval_hours: Option<i32>,
    pub compliance_mode: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_compliance_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ComplianceSettingsResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::ComplianceView)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            i32,
            bool,
            Option<String>,
            Option<i32>,
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT org_id, retention_days, signing_enabled, signing_key_id,
                chain_verification_interval_hours, compliance_mode, created_at, updated_at
         FROM org_compliance_settings WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(r) = row {
        Ok(Json(ComplianceSettingsResponse {
            org_id: r.0,
            retention_days: r.1,
            signing_enabled: r.2,
            signing_key_id: r.3,
            chain_verification_interval_hours: r.4,
            compliance_mode: r.5,
            created_at: r.6,
            updated_at: r.7,
        }))
    } else {
        Ok(Json(ComplianceSettingsResponse {
            org_id,
            retention_days: 365,
            signing_enabled: false,
            signing_key_id: None,
            chain_verification_interval_hours: Some(24),
            compliance_mode: "none".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }))
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateComplianceSettingsRequest {
    pub retention_days: Option<i32>,
    pub signing_enabled: Option<bool>,
    pub chain_verification_interval_hours: Option<i32>,
    pub compliance_mode: Option<String>,
}

pub async fn update_compliance_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(req): Json<UpdateComplianceSettingsRequest>,
) -> Result<Json<ComplianceSettingsResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::ComplianceManage)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let valid_modes = ["none", "sox", "pci_dss", "sr_11_7", "custom"];
    if let Some(mode) = &req.compliance_mode {
        if !valid_modes.contains(&mode.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid compliance mode. Must be one of: {}",
                    valid_modes.join(", ")
                ),
            ));
        }
    }

    let mode = req.compliance_mode.as_deref().unwrap_or("none");
    let min_retention = match mode {
        "sox" => 2555,
        "pci_dss" => 365,
        "sr_11_7" => 1095,
        _ => 0,
    };
    if let Some(days) = req.retention_days {
        if days < min_retention {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Compliance mode '{}' requires minimum {} days retention",
                    mode, min_retention
                ),
            ));
        }
    }

    let row = sqlx::query_as::<_, (Uuid, i32, bool, Option<String>, Option<i32>, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO org_compliance_settings (org_id, retention_days, signing_enabled, chain_verification_interval_hours, compliance_mode)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (org_id) DO UPDATE SET
           retention_days = COALESCE($2, org_compliance_settings.retention_days),
           signing_enabled = COALESCE($3, org_compliance_settings.signing_enabled),
           chain_verification_interval_hours = COALESCE($4, org_compliance_settings.chain_verification_interval_hours),
           compliance_mode = COALESCE($5, org_compliance_settings.compliance_mode),
           updated_at = NOW()
         RETURNING org_id, retention_days, signing_enabled, signing_key_id, chain_verification_interval_hours, compliance_mode, created_at, updated_at"
    )
    .bind(org_id)
    .bind(req.retention_days.unwrap_or(365))
    .bind(req.signing_enabled.unwrap_or(false))
    .bind(req.chain_verification_interval_hours)
    .bind(req.compliance_mode.as_deref().unwrap_or("none"))
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    audit::log(
        &state.pool,
        audit::user_action(
            auth.org_id,
            auth.user_id,
            "org.compliance.update",
            "org",
            Some(org_id),
            Some(
                serde_json::json!({"compliance_mode": mode, "retention_days": req.retention_days}),
            ),
        ),
    )
    .await;

    Ok(Json(ComplianceSettingsResponse {
        org_id: row.0,
        retention_days: row.1,
        signing_enabled: row.2,
        signing_key_id: row.3,
        chain_verification_interval_hours: row.4,
        compliance_mode: row.5,
        created_at: row.6,
        updated_at: row.7,
    }))
}

// --- Public Key ---

#[derive(Serialize)]
pub struct PublicKeyResponse {
    pub algorithm: String,
    pub public_key: String,
}

pub async fn get_public_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<PublicKeyResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    Ok(Json(PublicKeyResponse {
        algorithm: "Ed25519".into(),
        public_key: state.extensions.signing.public_key_b64(),
    }))
}

// --- Chain Verification ---

#[derive(Serialize)]
pub struct ChainStatusResponse {
    pub status: String,
    pub total_commits: i32,
    pub verified_commits: i32,
    pub errors: Option<serde_json::Value>,
    pub last_verified_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_chain_status(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ChainStatusResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::ComplianceView)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let row = sqlx::query_as::<
        _,
        (
            String,
            i32,
            i32,
            Option<serde_json::Value>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        "SELECT status, total_commits, verified_commits, errors, completed_at
         FROM chain_verifications WHERE org_id = $1
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(r) = row {
        Ok(Json(ChainStatusResponse {
            status: r.0,
            total_commits: r.1,
            verified_commits: r.2,
            errors: r.3,
            last_verified_at: r.4,
        }))
    } else {
        Ok(Json(ChainStatusResponse {
            status: "never_run".into(),
            total_commits: 0,
            verified_commits: 0,
            errors: None,
            last_verified_at: None,
        }))
    }
}

pub async fn verify_chain(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ChainStatusResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::ComplianceView)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let commits = sqlx::query_as::<
        _,
        (
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT c.id, c.record_hash, c.chain_hash, c.prev_chain_hash, c.signature
         FROM commits c JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1 AND c.sealed_at IS NOT NULL
         ORDER BY c.sealed_at ASC, c.created_at ASC, c.id ASC",
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total = commits.len() as i32;
    let mut verified = 0i32;
    let mut errors = Vec::new();
    let mut prev_hash: Option<String> = None;

    for (id, record_hash, chain_hash, prev_chain_hash, signature) in &commits {
        let (Some(rh), Some(ch), Some(sig)) = (record_hash, chain_hash, signature) else {
            errors.push(serde_json::json!({"commit_id": id, "error": "missing integrity fields"}));
            prev_hash = chain_hash.clone();
            continue;
        };

        let expected_chain = state
            .extensions
            .signing
            .chain_hash(prev_chain_hash.as_deref(), rh);
        if &expected_chain != ch {
            errors.push(serde_json::json!({"commit_id": id, "error": "chain_hash mismatch"}));
            prev_hash = Some(ch.clone());
            continue;
        }

        if prev_hash.as_deref() != prev_chain_hash.as_deref() {
            errors.push(serde_json::json!({"commit_id": id, "error": "prev_chain_hash mismatch"}));
        }

        if !state.extensions.signing.verify(rh, sig) {
            errors.push(
                serde_json::json!({"commit_id": id, "error": "signature verification failed"}),
            );
            prev_hash = Some(ch.clone());
            continue;
        }

        verified += 1;
        prev_hash = Some(ch.clone());
    }

    let status = if errors.is_empty() { "pass" } else { "fail" };
    let errors_json = if errors.is_empty() {
        None
    } else {
        Some(serde_json::json!(errors))
    };

    sqlx::query(
        "INSERT INTO chain_verifications (org_id, status, total_commits, verified_commits, errors, started_at, completed_at)
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW())"
    )
    .bind(org_id)
    .bind(status)
    .bind(total)
    .bind(verified)
    .bind(&errors_json)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    audit::log(
        &state.pool,
        audit::user_action(
            auth.org_id,
            auth.user_id,
            "chain.verify",
            "org",
            Some(org_id),
            Some(serde_json::json!({"status": status, "total": total, "verified": verified})),
        ),
    )
    .await;

    Ok(Json(ChainStatusResponse {
        status: status.into(),
        total_commits: total,
        verified_commits: verified,
        errors: errors_json,
        last_verified_at: Some(chrono::Utc::now()),
    }))
}

// --- Audit Log ---

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub action: Option<String>,
    pub actor_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Serialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct AuditLogResponse {
    pub entries: Vec<AuditLogEntry>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

pub async fn list_audit_log(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
    Query(query): Query<AuditLogQuery>,
) -> Result<Json<AuditLogResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, Permission::AuditLogView)
    {
        return Err((StatusCode::FORBIDDEN, "Insufficient permissions".into()));
    }

    let per_page = query.per_page.unwrap_or(50).min(200);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_log
         WHERE org_id = $1
           AND ($2::TEXT IS NULL OR action = $2)
           AND ($3::UUID IS NULL OR actor_id = $3)
           AND ($4::TEXT IS NULL OR resource_type = $4)
           AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
           AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)",
    )
    .bind(org_id)
    .bind(&query.action)
    .bind(query.actor_id)
    .bind(&query.resource_type)
    .bind(query.from)
    .bind(query.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            Option<Uuid>,
            String,
            String,
            Option<Uuid>,
            Option<serde_json::Value>,
            Option<String>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, actor_id, action, resource_type, resource_id, details,
                host(ip_address)::TEXT, user_agent, created_at
         FROM audit_log
         WHERE org_id = $1
           AND ($2::TEXT IS NULL OR action = $2)
           AND ($3::UUID IS NULL OR actor_id = $3)
           AND ($4::TEXT IS NULL OR resource_type = $4)
           AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
           AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)
         ORDER BY created_at DESC
         LIMIT $7 OFFSET $8",
    )
    .bind(org_id)
    .bind(&query.action)
    .bind(query.actor_id)
    .bind(&query.resource_type)
    .bind(query.from)
    .bind(query.to)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let entries = rows
        .into_iter()
        .map(|r| AuditLogEntry {
            id: r.0,
            actor_id: r.1,
            action: r.2,
            resource_type: r.3,
            resource_id: r.4,
            details: r.5,
            ip_address: r.6,
            user_agent: r.7,
            created_at: r.8,
        })
        .collect();

    Ok(Json(AuditLogResponse {
        entries,
        total,
        page,
        per_page,
    }))
}

// --- Trace Verification ---

#[derive(Serialize)]
pub struct TraceVerifyResponse {
    pub commit_id: Uuid,
    pub record_hash_valid: bool,
    pub signature_valid: bool,
    pub chain_valid: bool,
    pub sealed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn verify_trace(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TraceVerifyResponse>, (StatusCode, String)> {
    let commit = sqlx::query_as::<
        _,
        (
            Uuid,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        "SELECT c.id, c.record_hash, c.chain_hash, c.prev_chain_hash, c.signature, c.sealed_at
         FROM commits c JOIN repos r ON c.repo_id = r.id
         WHERE c.id = $1 AND r.org_id = $2",
    )
    .bind(id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Commit not found".into()))?;

    let (commit_id, record_hash, chain_hash, prev_chain_hash, signature, sealed_at) = commit;

    let signature_valid = match (&record_hash, &signature) {
        (Some(rh), Some(sig)) => state.extensions.signing.verify(rh, sig),
        _ => false,
    };

    let chain_valid = match (&record_hash, &chain_hash) {
        (Some(rh), Some(ch)) => {
            let expected = state
                .extensions
                .signing
                .chain_hash(prev_chain_hash.as_deref(), rh);
            expected == *ch
        }
        _ => false,
    };

    Ok(Json(TraceVerifyResponse {
        commit_id,
        record_hash_valid: record_hash.is_some(),
        signature_valid,
        chain_valid,
        sealed_at,
    }))
}
