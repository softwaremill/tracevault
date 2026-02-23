use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{
    generate_device_token, generate_session_token, hash_password, verify_password,
};
use crate::extractors::AuthUser;

// --- Register ---

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub org_name: String,
    pub email: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub token: String,
}

pub async fn register(
    State(pool): State<PgPool>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), (StatusCode, String)> {
    if req.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 8 characters".into(),
        ));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to hash password: {e}")))?;

    // Create org
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orgs (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id",
    )
    .bind(&req.org_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check if email already taken
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = $1")
            .bind(&req.email)
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "Email already registered".into()));
    }

    // Create user with owner role
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (org_id, email, password_hash, name, role) VALUES ($1, $2, $3, $4, 'owner') RETURNING id",
    )
    .bind(org_id)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Create session
    let (raw_token, token_hash) = generate_session_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id,
            org_id,
            token: raw_token,
        }),
    ))
}

// --- Login ---

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub org_name: String,
    pub email: String,
    pub role: String,
}

pub async fn login(
    State(pool): State<PgPool>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String)>(
        "SELECT u.id, u.org_id, u.password_hash, u.role FROM users u WHERE u.email = $1",
    )
    .bind(&req.email)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::UNAUTHORIZED,
        "Invalid email or password".into(),
    ))?;

    let (user_id, org_id, password_hash, role) = row;

    if !verify_password(&req.password, &password_hash) {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".into(),
        ));
    }

    let org_name: String = sqlx::query_scalar("SELECT name FROM orgs WHERE id = $1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (raw_token, token_hash) = generate_session_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse {
        token: raw_token,
        user_id,
        org_id,
        org_name,
        email: req.email,
        role,
    }))
}

// --- Device Auth ---

#[derive(Serialize)]
pub struct DeviceAuthResponse {
    pub token: String,
    pub verification_url: String,
    pub expires_in: i64,
}

pub async fn device_start(
    State(pool): State<PgPool>,
) -> Result<Json<DeviceAuthResponse>, (StatusCode, String)> {
    let token = generate_device_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);

    sqlx::query(
        "INSERT INTO device_auth_requests (token, status, expires_at) VALUES ($1, 'pending', $2)",
    )
    .bind(&token)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DeviceAuthResponse {
        verification_url: format!("/auth/device?token={token}"),
        token,
        expires_in: 600,
    }))
}

#[derive(Serialize)]
pub struct DeviceStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_name: Option<String>,
}

pub async fn device_status(
    State(pool): State<PgPool>,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> Result<Json<DeviceStatusResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (String, Option<Uuid>)>(
        "SELECT status, session_id FROM device_auth_requests WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Device auth request not found or expired".into(),
    ))?;

    let (status, session_id) = row;

    if status == "approved" {
        if let Some(sid) = session_id {
            let info = sqlx::query_as::<_, (String, String)>(
                "SELECT u.email, o.name FROM auth_sessions s
                 JOIN users u ON s.user_id = u.id
                 JOIN orgs o ON u.org_id = o.id
                 WHERE s.id = $1",
            )
            .bind(sid)
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some((email, org_name)) = info {
                let raw_token: Option<String> = sqlx::query_scalar(
                    "SELECT session_token FROM device_auth_requests WHERE token = $1",
                )
                .bind(&token)
                .fetch_optional(&pool)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .flatten();

                return Ok(Json(DeviceStatusResponse {
                    status: "approved".into(),
                    token: raw_token,
                    email: Some(email),
                    org_name: Some(org_name),
                }));
            }
        }
    }

    Ok(Json(DeviceStatusResponse {
        status,
        token: None,
        email: None,
        org_name: None,
    }))
}

pub async fn device_approve(
    State(pool): State<PgPool>,
    auth: AuthUser,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, status FROM device_auth_requests WHERE token = $1 AND expires_at > NOW()",
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((
        StatusCode::NOT_FOUND,
        "Device auth request not found or expired".into(),
    ))?;

    let (request_id, status) = row;
    if status != "pending" {
        return Err((
            StatusCode::CONFLICT,
            "Device auth request already processed".into(),
        ));
    }

    // Create a new session for the CLI
    let (raw_token, token_hash) = generate_session_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    let session_id: Uuid = sqlx::query_scalar(
        "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update device request with approval + session reference + raw token
    sqlx::query(
        "UPDATE device_auth_requests SET status = 'approved', user_id = $1, session_id = $2, session_token = $3 WHERE id = $4",
    )
    .bind(auth.user_id)
    .bind(session_id)
    .bind(&raw_token)
    .bind(request_id)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

// --- Logout ---

pub async fn logout(
    State(pool): State<PgPool>,
    auth: AuthUser,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, (StatusCode, String)> {
    let raw_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((StatusCode::BAD_REQUEST, "Missing token".into()))?;

    let token_hash = crate::auth::sha256_hex(raw_token);

    sqlx::query("DELETE FROM auth_sessions WHERE token_hash = $1 AND user_id = $2")
        .bind(&token_hash)
        .bind(auth.user_id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

// --- Me ---

#[derive(Serialize)]
pub struct MeResponse {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub org_name: String,
    pub email: String,
    pub name: Option<String>,
    pub role: String,
}

pub async fn me(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (String, Option<String>, String, String)>(
        "SELECT u.email, u.name, u.role, o.name FROM users u JOIN orgs o ON u.org_id = o.id WHERE u.id = $1",
    )
    .bind(auth.user_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(MeResponse {
        user_id: auth.user_id,
        org_id: auth.org_id,
        email: row.0,
        name: row.1,
        role: row.2,
        org_name: row.3,
    }))
}
