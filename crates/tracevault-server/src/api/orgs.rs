use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use crate::AppState;
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
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>, String, chrono::DateTime<chrono::Utc>)>(
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

    let role = req.role.unwrap_or_else(|| "member".into());
    if role != "member" && role != "admin" {
        return Err((
            StatusCode::BAD_REQUEST,
            "Role must be 'member' or 'admin'".into(),
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
    if !["member", "admin"].contains(&req.role.as_str()) {
        return Err((
            StatusCode::BAD_REQUEST,
            "Role must be 'member' or 'admin'".into(),
        ));
    }

    sqlx::query("UPDATE users SET role = $1 WHERE id = $2 AND org_id = $3")
        .bind(&req.role)
        .bind(user_id)
        .bind(org_id)
        .execute(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
