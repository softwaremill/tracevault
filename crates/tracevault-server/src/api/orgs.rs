use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::hash_password;
use crate::extractors::AuthUser;

#[derive(Serialize)]
pub struct OrgResponse {
    pub id: Uuid,
    pub name: String,
    pub plan: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_org(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<OrgResponse>, (StatusCode, String)> {
    if auth.org_id != id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    let row = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, plan, created_at FROM orgs WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Org not found".into()))?;

    Ok(Json(OrgResponse {
        id: row.0,
        name: row.1,
        plan: row.2,
        created_at: row.3,
    }))
}

#[derive(Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
}

pub async fn update_org(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrgRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.org_id != id || auth.role != "owner" {
        return Err((StatusCode::FORBIDDEN, "Requires owner role".into()));
    }

    if let Some(name) = &req.name {
        sqlx::query("UPDATE orgs SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(StatusCode::OK)
}

#[derive(Serialize)]
pub struct MemberResponse {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MemberResponse>>, (StatusCode, String)> {
    if auth.org_id != id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if !state
        .extensions
        .permissions
        .has_permission(&auth.role, crate::permissions::Permission::UserManage)
        && !state
            .extensions
            .permissions
            .has_permission(&auth.role, crate::permissions::Permission::AuditLogView)
    {
        return Err((
            StatusCode::FORBIDDEN,
            "Requires admin or auditor role".into(),
        ));
    }

    let rows = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            Option<String>,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, email, name, role, created_at FROM users WHERE org_id = $1 ORDER BY created_at",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let members = rows
        .into_iter()
        .map(|r| MemberResponse {
            id: r.0,
            email: r.1,
            name: r.2,
            role: r.3,
            created_at: r.4,
        })
        .collect();

    Ok(Json(members))
}

#[derive(Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub password: String,
    pub name: Option<String>,
    pub role: Option<String>,
}

pub async fn invite_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<InviteMemberRequest>,
) -> Result<(StatusCode, Json<MemberResponse>), (StatusCode, String)> {
    if auth.org_id != id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    let role = req.role.unwrap_or_else(|| "developer".into());
    if !state.extensions.permissions.is_valid_role(&role) || role == "owner" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Role must be one of: admin, policy_admin, developer, auditor".into(),
        ));
    }

    let password_hash = hash_password(&req.password).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to hash password: {e}"),
        )
    })?;

    let row = sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO users (org_id, email, password_hash, name, role) VALUES ($1, $2, $3, $4, $5) RETURNING id, created_at",
    )
    .bind(id)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.name)
    .bind(&role)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            (StatusCode::CONFLICT, "Email already registered".into())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    })?;

    Ok((
        StatusCode::CREATED,
        Json(MemberResponse {
            id: row.0,
            email: req.email,
            name: req.name,
            role,
            created_at: row.1,
        }),
    ))
}

pub async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.org_id != org_id || auth.role != "owner" {
        return Err((StatusCode::FORBIDDEN, "Requires owner role".into()));
    }
    if auth.user_id == user_id {
        return Err((StatusCode::BAD_REQUEST, "Cannot remove yourself".into()));
    }

    sqlx::query("DELETE FROM users WHERE id = $1 AND org_id = $2")
        .bind(user_id)
        .bind(org_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

pub async fn change_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.org_id != org_id || auth.role != "owner" {
        return Err((StatusCode::FORBIDDEN, "Requires owner role".into()));
    }
    if !state.extensions.permissions.is_valid_role(&req.role) || req.role == "owner" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Role must be one of: admin, policy_admin, developer, auditor".into(),
        ));
    }

    sqlx::query("UPDATE users SET role = $1 WHERE id = $2 AND org_id = $3")
        .bind(&req.role)
        .bind(user_id)
        .bind(org_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    crate::audit::log(
        &state.pool,
        crate::audit::user_action(
            auth.org_id,
            auth.user_id,
            "role.change",
            "user",
            Some(user_id),
            Some(serde_json::json!({"new_role": &req.role})),
        ),
    )
    .await;

    Ok(StatusCode::OK)
}

// --- LLM Settings ---

#[derive(Serialize)]
pub struct LlmSettingsResponse {
    pub provider: Option<String>,
    pub has_api_key: bool,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

pub async fn get_llm_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
) -> Result<Json<LlmSettingsResponse>, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    let row = sqlx::query_as::<
        _,
        (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT llm_provider, llm_api_key_encrypted, llm_model, llm_base_url
         FROM org_compliance_settings WHERE org_id = $1",
    )
    .bind(org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(r) = row {
        Ok(Json(LlmSettingsResponse {
            provider: r.0,
            has_api_key: r.1.is_some(),
            model: r.2,
            base_url: r.3,
        }))
    } else {
        Ok(Json(LlmSettingsResponse {
            provider: None,
            has_api_key: false,
            model: None,
            base_url: None,
        }))
    }
}

#[derive(Deserialize)]
pub struct UpdateLlmSettingsRequest {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

pub async fn update_llm_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(org_id): Path<Uuid>,
    Json(req): Json<UpdateLlmSettingsRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.org_id != org_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".into()));
    }
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    if let Some(ref provider) = req.provider {
        if provider != "anthropic" && provider != "openai" {
            return Err((
                StatusCode::BAD_REQUEST,
                "Provider must be 'anthropic' or 'openai'".into(),
            ));
        }
    }

    // Encrypt API key if provided
    let (encrypted_key, nonce) = if let Some(ref api_key) = req.api_key {
        let (ct, n) = state.extensions.encryption.encrypt(api_key).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Encryption error: {e}"),
            )
        })?;
        (Some(ct), Some(n))
    } else {
        (None, None)
    };

    // Upsert: ensure row exists for this org
    sqlx::query(
        "INSERT INTO org_compliance_settings (org_id) VALUES ($1) ON CONFLICT (org_id) DO NOTHING",
    )
    .bind(org_id)
    .execute(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Build dynamic UPDATE
    let mut set_clauses = vec!["updated_at = NOW()".to_string()];
    let mut param_idx = 2u32; // $1 is org_id

    if req.provider.is_some() {
        set_clauses.push(format!("llm_provider = ${param_idx}"));
        param_idx += 1;
    }
    if encrypted_key.is_some() {
        set_clauses.push(format!("llm_api_key_encrypted = ${param_idx}"));
        param_idx += 1;
        set_clauses.push(format!("llm_api_key_nonce = ${param_idx}"));
        param_idx += 1;
    }
    if req.model.is_some() {
        set_clauses.push(format!("llm_model = ${param_idx}"));
        param_idx += 1;
    }
    if req.base_url.is_some() {
        set_clauses.push(format!("llm_base_url = ${param_idx}"));
    }

    let sql = format!(
        "UPDATE org_compliance_settings SET {} WHERE org_id = $1",
        set_clauses.join(", ")
    );

    let mut query = sqlx::query(&sql).bind(org_id);

    if let Some(ref provider) = req.provider {
        query = query.bind(provider);
    }
    if let Some(ref ek) = encrypted_key {
        query = query.bind(ek);
        query = query.bind(nonce.as_ref().unwrap());
    }
    if let Some(ref model) = req.model {
        query = query.bind(model);
    }
    if let Some(ref base_url) = req.base_url {
        query = query.bind(base_url);
    }

    query
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    crate::audit::log(
        &state.pool,
        crate::audit::user_action(
            auth.org_id,
            auth.user_id,
            "llm_settings.update",
            "org",
            Some(org_id),
            Some(serde_json::json!({
                "provider": req.provider,
                "model": req.model,
                "base_url": req.base_url,
                "api_key_changed": req.api_key.is_some(),
            })),
        ),
    )
    .await;

    Ok(StatusCode::OK)
}
