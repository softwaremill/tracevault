# Auth & Web UI Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add user authentication (device auth flow for CLI, email/password for web) and a SvelteKit dashboard for managing orgs, repos, and users.

**Architecture:** Server-side auth with opaque session tokens stored in PostgreSQL. CLI uses a device authorization flow (open browser, poll for approval). Web UI is a separate SvelteKit app with shadcn-svelte, communicating with the Axum API via REST. CORS handled by tower-http.

**Tech Stack:** Rust/Axum (server), argon2 + sha2 + rand (crypto), SvelteKit + shadcn-svelte + Tailwind (web), open crate (CLI browser launch)

**Design doc:** `docs/plans/2026-02-23-auth-and-web-ui-design.md`

---

## Phase 1: Database Migration & Server Auth Core

### Task 1: Add auth database migration

**Files:**
- Create: `crates/tracevault-server/migrations/002_auth.sql`

**Step 1: Write the migration SQL**

```sql
-- 002_auth.sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name TEXT,
    role TEXT NOT NULL DEFAULT 'member',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE device_auth_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT NOT NULL UNIQUE,
    user_id UUID REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    session_id UUID REFERENCES auth_sessions(id),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_auth_sessions_token ON auth_sessions(token_hash);
CREATE INDEX idx_device_auth_token ON device_auth_requests(token);
CREATE INDEX idx_users_org ON users(org_id);
```

**Step 2: Verify migration compiles**

Run: `cargo build -p tracevault-server`
Expected: success (sqlx checks migrations at compile time only with `sqlx-cli`, otherwise runtime)

**Step 3: Commit**

```bash
git add crates/tracevault-server/migrations/002_auth.sql
git commit -m "feat: add auth database migration (users, sessions, device_auth)"
```

---

### Task 2: Add server dependencies for auth

**Files:**
- Modify: `crates/tracevault-server/Cargo.toml`

**Step 1: Add argon2, sha2, rand, and hex dependencies**

Add to `[dependencies]` section of `crates/tracevault-server/Cargo.toml`:

```toml
argon2 = "0.5"
sha2 = "0.10"
rand = "0.9"
hex = "0.4"
```

**Step 2: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 3: Commit**

```bash
git add crates/tracevault-server/Cargo.toml
git commit -m "feat: add auth crypto dependencies (argon2, sha2, rand, hex)"
```

---

### Task 3: Implement token generation and password hashing utilities

**Files:**
- Create: `crates/tracevault-server/src/auth.rs`

**Step 1: Write the auth utilities module**

```rust
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use rand::rngs::OsRng;
use sha2::{Sha256, Digest};

/// Hash a password with Argon2id. Returns PHC-format string.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2.hash_password(password.as_bytes(), &salt)?.to_string())
}

/// Verify a password against a PHC-format hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

/// Generate a random session token. Returns (raw_token, sha256_hash).
/// Raw token is prefixed with `tvs_` and hex-encoded.
pub fn generate_session_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let raw = format!("tvs_{}", hex::encode(bytes));
    let hash = sha256_hex(&raw);
    (raw, hash)
}

/// Generate a random API key. Returns (raw_key, sha256_hash).
/// Raw key is prefixed with `tvk_` and hex-encoded.
pub fn generate_api_key() -> (String, String) {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    let raw = format!("tvk_{}", hex::encode(bytes));
    let hash = sha256_hex(&raw);
    (raw, hash)
}

/// Generate a random device auth token (hex-encoded, no prefix).
pub fn generate_device_token() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    hex::encode(bytes)
}

/// SHA-256 hash a string, return hex-encoded.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
```

**Step 2: Register the module in main.rs**

Add `mod auth;` to `crates/tracevault-server/src/main.rs` (after `mod db;`).

**Step 3: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/auth.rs crates/tracevault-server/src/main.rs
git commit -m "feat: add password hashing and token generation utilities"
```

---

### Task 4: Implement AuthUser extractor middleware

**Files:**
- Create: `crates/tracevault-server/src/extractors.rs`
- Modify: `crates/tracevault-server/src/main.rs`

This extractor checks the `Authorization: Bearer {token}` header, looks up the token in `auth_sessions` (by SHA-256 hash), falls back to `api_keys`, and returns 401 if neither matches.

**Step 1: Write the extractor**

```rust
use axum::{
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::sha256_hex;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub role: String,
}

impl FromRequestParts<PgPool> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        pool: &PgPool,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing or invalid Authorization header"))?;

        let token_hash = sha256_hex(header);

        // Try auth_sessions first
        let session_row = sqlx::query_as::<_, (Uuid, Uuid, String)>(
            "SELECT u.id, u.org_id, u.role FROM auth_sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.token_hash = $1 AND s.expires_at > NOW()"
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        if let Some((user_id, org_id, role)) = session_row {
            return Ok(AuthUser { user_id, org_id, role });
        }

        // Fall back to api_keys
        let api_key_row = sqlx::query_as::<_, (Uuid,)>(
            "SELECT org_id FROM api_keys WHERE key_hash = $1"
        )
        .bind(&token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?;

        if let Some((org_id,)) = api_key_row {
            // API keys act as org-level "member" — no specific user
            return Ok(AuthUser {
                user_id: Uuid::nil(),
                org_id,
                role: "member".to_string(),
            });
        }

        Err((StatusCode::UNAUTHORIZED, "Invalid or expired token"))
    }
}
```

**Step 2: Register the module in main.rs**

Add `mod extractors;` to `crates/tracevault-server/src/main.rs`.

**Step 3: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/extractors.rs crates/tracevault-server/src/main.rs
git commit -m "feat: add AuthUser axum extractor for session/api-key auth"
```

---

### Task 5: Implement register and login endpoints

**Files:**
- Modify: `crates/tracevault-server/src/api/auth.rs`

Replace the existing stubbed `auth.rs` with full register + login.

**Step 1: Rewrite auth.rs**

```rust
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{hash_password, verify_password, generate_session_token};

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
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 8 characters".into()));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to hash password: {e}")))?;

    // Create org
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orgs (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id"
    )
    .bind(&req.org_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Check if email already taken
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM users WHERE email = $1"
    )
    .bind(&req.email)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if existing.is_some() {
        return Err((StatusCode::CONFLICT, "Email already registered".into()));
    }

    // Create user with owner role
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (org_id, email, password_hash, name, role) VALUES ($1, $2, $3, $4, 'owner') RETURNING id"
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
        "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)"
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(RegisterResponse {
        user_id,
        org_id,
        token: raw_token,
    })))
}

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
        "SELECT u.id, u.org_id, u.password_hash, u.role FROM users u WHERE u.email = $1"
    )
    .bind(&req.email)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::UNAUTHORIZED, "Invalid email or password".into()))?;

    let (user_id, org_id, password_hash, role) = row;

    if !verify_password(&req.password, &password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".into()));
    }

    let org_name: String = sqlx::query_scalar("SELECT name FROM orgs WHERE id = $1")
        .bind(org_id)
        .fetch_one(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (raw_token, token_hash) = generate_session_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)"
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
```

**Step 2: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 3: Commit**

```bash
git add crates/tracevault-server/src/api/auth.rs
git commit -m "feat: implement register and login endpoints with argon2 password hashing"
```

---

### Task 6: Implement device auth, logout, and me endpoints

**Files:**
- Modify: `crates/tracevault-server/src/api/auth.rs` (append)

**Step 1: Add device auth, logout, and me handlers to auth.rs**

Append these to `crates/tracevault-server/src/api/auth.rs`:

```rust
use crate::auth::generate_device_token;
use crate::extractors::AuthUser;

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
        "INSERT INTO device_auth_requests (token, status, expires_at) VALUES ($1, 'pending', $2)"
    )
    .bind(&token)
    .bind(expires_at)
    .execute(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // The verification_url will be constructed by the CLI using server_url + this path.
    // We return a relative path here; CLI prepends the server URL.
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
        "SELECT status, session_id FROM device_auth_requests WHERE token = $1 AND expires_at > NOW()"
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Device auth request not found or expired".into()))?;

    let (status, session_id) = row;

    if status == "approved" {
        if let Some(sid) = session_id {
            // Fetch the raw token — we stored it in a column? No, we only have the hash.
            // We need to return the raw token. The approach: store the raw token temporarily
            // in the device_auth_requests row when approving. But we don't have that column.
            // Alternative: we'll fetch the session and return info. The raw token was already
            // associated during approval. We need to adjust the schema slightly.
            //
            // Better approach: during approval, we generate the session token and store
            // the RAW token in a dedicated column on device_auth_requests (it's short-lived
            // and single-read). Let's add `session_token TEXT` to the migration.
            //
            // For now, fetch user info via session_id.
            let info = sqlx::query_as::<_, (String, String)>(
                "SELECT u.email, o.name FROM auth_sessions s
                 JOIN users u ON s.user_id = u.id
                 JOIN orgs o ON u.org_id = o.id
                 WHERE s.id = $1"
            )
            .bind(sid)
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            if let Some((email, org_name)) = info {
                // We need the raw token. It was stored in device_auth_requests.session_token.
                let raw_token: Option<String> = sqlx::query_scalar(
                    "SELECT session_token FROM device_auth_requests WHERE token = $1"
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
    // Verify the device request is pending
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, status FROM device_auth_requests WHERE token = $1 AND expires_at > NOW()"
    )
    .bind(&token)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Device auth request not found or expired".into()))?;

    let (request_id, status) = row;
    if status != "pending" {
        return Err((StatusCode::CONFLICT, "Device auth request already processed".into()));
    }

    // Create a new session for the CLI
    let (raw_token, token_hash) = generate_session_token();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    let session_id: Uuid = sqlx::query_scalar(
        "INSERT INTO auth_sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(auth.user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update device request with approval + session reference + raw token
    sqlx::query(
        "UPDATE device_auth_requests SET status = 'approved', user_id = $1, session_id = $2, session_token = $3 WHERE id = $4"
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
    // Get the raw token from the header to hash and delete
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
        "SELECT u.email, u.name, u.role, o.name FROM users u JOIN orgs o ON u.org_id = o.id WHERE u.id = $1"
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
```

**Step 2: Update migration 002 to add `session_token` column**

Add this to `crates/tracevault-server/migrations/002_auth.sql` inside the `device_auth_requests` table:

```sql
    session_token TEXT,  -- raw token stored temporarily for CLI to retrieve
```

The full `device_auth_requests` table should be:

```sql
CREATE TABLE device_auth_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT NOT NULL UNIQUE,
    user_id UUID REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    session_id UUID REFERENCES auth_sessions(id),
    session_token TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Step 3: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/auth.rs crates/tracevault-server/migrations/002_auth.sql
git commit -m "feat: implement device auth flow, logout, and me endpoints"
```

---

### Task 7: Wire new auth routes in main.rs and add CORS

**Files:**
- Modify: `crates/tracevault-server/src/main.rs`
- Modify: `crates/tracevault-server/src/config.rs`

**Step 1: Add `cors_origin` to ServerConfig**

In `crates/tracevault-server/src/config.rs`, add field and read from env:

```rust
pub struct ServerConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cors_origin: Option<String>,
}

// In from_env():
cors_origin: env::var("CORS_ORIGIN").ok(),
```

**Step 2: Update main.rs routes and CORS**

```rust
use axum::{routing::{get, post, delete, put}, Router};
use tower_http::trace::TraceLayer;
use tower_http::cors::{CorsLayer, Any};

// Add new routes:
// Auth (public)
.route("/api/v1/auth/register", post(api::auth::register))
.route("/api/v1/auth/login", post(api::auth::login))
.route("/api/v1/auth/device", post(api::auth::device_start))
.route("/api/v1/auth/device/{token}/status", get(api::auth::device_status))
// Auth (requires session)
.route("/api/v1/auth/device/{token}/approve", post(api::auth::device_approve))
.route("/api/v1/auth/logout", post(api::auth::logout))
.route("/api/v1/auth/me", get(api::auth::me))

// CORS layer
let cors = if let Some(origin) = &cfg.cors_origin {
    CorsLayer::new()
        .allow_origin(origin.parse::<http::HeaderValue>().unwrap())
        .allow_methods(Any)
        .allow_headers(Any)
} else {
    CorsLayer::permissive()  // dev mode: allow all
};

// Add .layer(cors) before .with_state(pool)
```

Remove the old stubbed `.route("/api/v1/auth/register", ...)` line since auth.rs now has the real implementation.

**Step 3: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/main.rs crates/tracevault-server/src/config.rs
git commit -m "feat: wire auth routes and add CORS support"
```

---

### Task 8: Add auth to existing trace/repo endpoints

**Files:**
- Modify: `crates/tracevault-server/src/api/traces.rs`
- Modify: `crates/tracevault-server/src/api/repos.rs`

**Step 1: Add AuthUser parameter to trace handlers**

In `create_trace`, `list_traces`, and `get_trace`, add `_auth: AuthUser` parameter (Axum will run the extractor). Import `use crate::extractors::AuthUser;`.

For example:
```rust
pub async fn create_trace(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Json(req): Json<CreateTraceRequest>,
) -> ...
```

Do the same for `list_traces` and `get_trace`.

**Step 2: Add AuthUser to register_repo**

```rust
pub async fn register_repo(
    State(pool): State<PgPool>,
    _auth: AuthUser,
    Json(req): Json<RegisterRepoRequest>,
) -> ...
```

**Step 3: Verify it compiles**

Run: `cargo build -p tracevault-server`
Expected: success

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/traces.rs crates/tracevault-server/src/api/repos.rs
git commit -m "feat: require auth on trace and repo endpoints"
```

---

## Phase 2: Management Endpoints

### Task 9: Implement org management endpoints

**Files:**
- Create: `crates/tracevault-server/src/api/orgs.rs`
- Modify: `crates/tracevault-server/src/api/mod.rs`
- Modify: `crates/tracevault-server/src/main.rs`

**Step 1: Create orgs.rs with get, update, list members, invite, remove, change role**

```rust
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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
    State(pool): State<PgPool>,
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
        "SELECT id, name, plan, created_at FROM orgs WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Org not found".into()))?;

    Ok(Json(OrgResponse { id: row.0, name: row.1, plan: row.2, created_at: row.3 }))
}

#[derive(Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
}

pub async fn update_org(
    State(pool): State<PgPool>,
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
            .execute(&pool)
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
    State(pool): State<PgPool>,
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
        "SELECT id, email, name, role, created_at FROM users WHERE org_id = $1 ORDER BY created_at"
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let members = rows.into_iter().map(|r| MemberResponse {
        id: r.0, email: r.1, name: r.2, role: r.3, created_at: r.4,
    }).collect();

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
    State(pool): State<PgPool>,
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
        return Err((StatusCode::BAD_REQUEST, "Role must be 'member' or 'admin'".into()));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to hash password: {e}")))?;

    let row = sqlx::query_as::<_, (Uuid, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO users (org_id, email, password_hash, name, role) VALUES ($1, $2, $3, $4, $5) RETURNING id, created_at"
    )
    .bind(id)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.name)
    .bind(&role)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            (StatusCode::CONFLICT, "Email already registered".into())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    })?;

    Ok((StatusCode::CREATED, Json(MemberResponse {
        id: row.0, email: req.email, name: req.name, role, created_at: row.1,
    })))
}

pub async fn remove_member(
    State(pool): State<PgPool>,
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
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ChangeRoleRequest {
    pub role: String,
}

pub async fn change_role(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path((org_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ChangeRoleRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.org_id != org_id || auth.role != "owner" {
        return Err((StatusCode::FORBIDDEN, "Requires owner role".into()));
    }
    if !["member", "admin"].contains(&req.role.as_str()) {
        return Err((StatusCode::BAD_REQUEST, "Role must be 'member' or 'admin'".into()));
    }

    sqlx::query("UPDATE users SET role = $1 WHERE id = $2 AND org_id = $3")
        .bind(&req.role)
        .bind(user_id)
        .bind(org_id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
```

**Step 2: Add `pub mod orgs;` to `crates/tracevault-server/src/api/mod.rs`**

**Step 3: Wire routes in main.rs**

```rust
.route("/api/v1/orgs/{id}", get(api::orgs::get_org))
.route("/api/v1/orgs/{id}", put(api::orgs::update_org))
.route("/api/v1/orgs/{id}/members", get(api::orgs::list_members))
.route("/api/v1/orgs/{id}/members", post(api::orgs::invite_member))
.route("/api/v1/orgs/{id}/members/{user_id}", delete(api::orgs::remove_member))
.route("/api/v1/orgs/{id}/members/{user_id}/role", put(api::orgs::change_role))
```

**Step 4: Verify it compiles**

Run: `cargo build -p tracevault-server`

**Step 5: Commit**

```bash
git add crates/tracevault-server/src/api/orgs.rs crates/tracevault-server/src/api/mod.rs crates/tracevault-server/src/main.rs
git commit -m "feat: implement org management endpoints (get, update, members)"
```

---

### Task 10: Implement API key management endpoints and repo list/delete

**Files:**
- Create: `crates/tracevault-server/src/api/api_keys.rs`
- Modify: `crates/tracevault-server/src/api/repos.rs` (add list_repos, delete_repo)
- Modify: `crates/tracevault-server/src/api/mod.rs`
- Modify: `crates/tracevault-server/src/main.rs`

**Step 1: Create api_keys.rs**

```rust
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::generate_api_key;
use crate::extractors::AuthUser;

#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String,
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
}

pub async fn create_api_key(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<CreateApiKeyResponse>), (StatusCode, String)> {
    let (raw_key, key_hash) = generate_api_key();

    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO api_keys (org_id, key_hash, name) VALUES ($1, $2, $3) RETURNING id"
    )
    .bind(auth.org_id)
    .bind(&key_hash)
    .bind(&req.name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(CreateApiKeyResponse { id, key: raw_key, name: req.name })))
}

pub async fn list_api_keys(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<Vec<ApiKeyResponse>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (Uuid, String, String, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, LEFT(key_hash, 8), created_at FROM api_keys WHERE org_id = $1 ORDER BY created_at"
    )
    .bind(auth.org_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let keys = rows.into_iter().map(|r| ApiKeyResponse {
        id: r.0, name: r.1, key_prefix: format!("tvk_...{}", r.2), created_at: r.3,
    }).collect();

    Ok(Json(keys))
}

pub async fn delete_api_key(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("DELETE FROM api_keys WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(auth.org_id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
```

**Step 2: Add list_repos and delete_repo to repos.rs**

```rust
pub async fn list_repos(
    State(pool): State<PgPool>,
    auth: AuthUser,
) -> Result<Json<Vec<RepoResponse>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<String>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, name, github_url, created_at FROM repos WHERE org_id = $1 ORDER BY name"
    )
    .bind(auth.org_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let repos = rows.into_iter().map(|r| RepoResponse {
        id: r.0, name: r.1, github_url: r.2, created_at: r.3,
    }).collect();

    Ok(Json(repos))
}

pub async fn delete_repo(
    State(pool): State<PgPool>,
    auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    if auth.role != "owner" && auth.role != "admin" {
        return Err((StatusCode::FORBIDDEN, "Requires admin role".into()));
    }

    sqlx::query("DELETE FROM repos WHERE id = $1 AND org_id = $2")
        .bind(id)
        .bind(auth.org_id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::OK)
}
```

Add `RepoResponse` struct and `use crate::extractors::AuthUser;` import.

```rust
#[derive(Debug, Serialize)]
pub struct RepoResponse {
    pub id: Uuid,
    pub name: String,
    pub github_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

**Step 3: Add `pub mod api_keys;` to api/mod.rs**

**Step 4: Wire routes in main.rs**

```rust
// Repos
.route("/api/v1/repos", get(api::repos::list_repos))
.route("/api/v1/repos", post(api::repos::register_repo))
.route("/api/v1/repos/{id}", delete(api::repos::delete_repo))
// API Keys
.route("/api/v1/api-keys", post(api::api_keys::create_api_key))
.route("/api/v1/api-keys", get(api::api_keys::list_api_keys))
.route("/api/v1/api-keys/{id}", delete(api::api_keys::delete_api_key))
```

**Step 5: Verify it compiles**

Run: `cargo build -p tracevault-server`

**Step 6: Commit**

```bash
git add crates/tracevault-server/src/api/api_keys.rs crates/tracevault-server/src/api/repos.rs crates/tracevault-server/src/api/mod.rs crates/tracevault-server/src/main.rs
git commit -m "feat: implement API key management and repo list/delete endpoints"
```

---

## Phase 3: CLI Auth Commands

### Task 11: Add CLI dependencies (open, dirs)

**Files:**
- Modify: `crates/tracevault-cli/Cargo.toml`

**Step 1: Add open and dirs crates**

```toml
open = "5"
dirs = "6"
```

`open` launches the default browser. `dirs` provides `config_dir()` for `~/.config/tracevault/`.

**Step 2: Verify it compiles**

Run: `cargo build -p tracevault-cli`

**Step 3: Commit**

```bash
git add crates/tracevault-cli/Cargo.toml
git commit -m "feat: add open and dirs dependencies for CLI auth"
```

---

### Task 12: Add credentials file handling

**Files:**
- Create: `crates/tracevault-cli/src/credentials.rs`
- Modify: `crates/tracevault-cli/src/lib.rs`

**Step 1: Write credentials module**

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub server_url: String,
    pub token: String,
    pub email: String,
    pub org_name: String,
}

impl Credentials {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("tracevault")
            .join("credentials.json")
    }

    pub fn load() -> Option<Self> {
        let path = Self::path();
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&path, json)
    }

    pub fn delete() -> Result<(), std::io::Error> {
        let path = Self::path();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }
}
```

**Step 2: Add `pub mod credentials;` to `crates/tracevault-cli/src/lib.rs`**

**Step 3: Verify it compiles**

Run: `cargo build -p tracevault-cli`

**Step 4: Commit**

```bash
git add crates/tracevault-cli/src/credentials.rs crates/tracevault-cli/src/lib.rs
git commit -m "feat: add credentials file handling for CLI auth"
```

---

### Task 13: Add device auth methods to ApiClient

**Files:**
- Modify: `crates/tracevault-cli/src/api_client.rs`

**Step 1: Add device auth request/response types and methods**

Add these types:

```rust
#[derive(Deserialize)]
pub struct DeviceAuthResponse {
    pub token: String,
    pub verification_url: String,
    pub expires_in: i64,
}

#[derive(Deserialize)]
pub struct DeviceStatusResponse {
    pub status: String,
    pub token: Option<String>,
    pub email: Option<String>,
    pub org_name: Option<String>,
}
```

Add these methods to `impl ApiClient`:

```rust
pub async fn device_start(&self) -> Result<DeviceAuthResponse, Box<dyn Error>> {
    let resp = self.client
        .post(format!("{}/api/v1/auth/device", self.base_url))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server returned {status}: {body}").into());
    }

    Ok(resp.json().await?)
}

pub async fn device_status(&self, token: &str) -> Result<DeviceStatusResponse, Box<dyn Error>> {
    let resp = self.client
        .get(format!("{}/api/v1/auth/device/{token}/status", self.base_url))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server returned {status}: {body}").into());
    }

    Ok(resp.json().await?)
}

pub async fn logout(&self) -> Result<(), Box<dyn Error>> {
    let mut builder = self.client.post(format!("{}/api/v1/auth/logout", self.base_url));
    if let Some(key) = &self.api_key {
        builder = builder.header("Authorization", format!("Bearer {key}"));
    }
    let resp = builder.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Server returned {status}: {body}").into());
    }
    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build -p tracevault-cli`

**Step 3: Commit**

```bash
git add crates/tracevault-cli/src/api_client.rs
git commit -m "feat: add device auth and logout methods to API client"
```

---

### Task 14: Implement `tracevault login` command

**Files:**
- Create: `crates/tracevault-cli/src/commands/login.rs`
- Modify: `crates/tracevault-cli/src/commands/mod.rs`
- Modify: `crates/tracevault-cli/src/main.rs`

**Step 1: Write login command**

```rust
use crate::api_client::ApiClient;
use crate::credentials::Credentials;

pub async fn login(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = ApiClient::new(server_url, None);

    // Start device auth flow
    let device = client.device_start().await?;
    let full_url = format!("{}{}", server_url.trim_end_matches('/'), device.verification_url);

    println!("Opening browser for authentication...");
    println!("If the browser doesn't open, visit: {full_url}");

    // Open browser
    if let Err(e) = open::that(&full_url) {
        eprintln!("Could not open browser: {e}");
    }

    // Poll for approval
    print!("Waiting for authentication...");
    let poll_interval = std::time::Duration::from_secs(2);
    let max_attempts = 150; // 5 minutes at 2s intervals

    for _ in 0..max_attempts {
        std::thread::sleep(poll_interval);

        match client.device_status(&device.token).await {
            Ok(status) => {
                if status.status == "approved" {
                    if let (Some(token), Some(email), Some(org_name)) =
                        (status.token, status.email, status.org_name)
                    {
                        println!(" done!");
                        println!();
                        println!("Logged in as {} (org: {})", email, org_name);

                        let creds = Credentials {
                            server_url: server_url.to_string(),
                            token,
                            email,
                            org_name,
                        };
                        creds.save()?;
                        println!("Credentials saved to {}", Credentials::path().display());
                        return Ok(());
                    }
                }
                // Still pending, continue polling
            }
            Err(e) => {
                eprintln!("\nError polling status: {e}");
                return Err(e);
            }
        }
    }

    Err("Authentication timed out after 5 minutes".into())
}
```

**Step 2: Add `pub mod login;` to commands/mod.rs**

**Step 3: Add Login variant to CLI enum in main.rs**

```rust
/// Log in to a TraceVault server
Login {
    /// TraceVault server URL
    #[arg(long)]
    server_url: String,
},
```

Wire in the match:

```rust
Cli::Login { server_url } => {
    if let Err(e) = commands::login::login(&server_url).await {
        eprintln!("Login error: {e}");
    }
}
```

**Step 4: Verify it compiles**

Run: `cargo build -p tracevault-cli`

**Step 5: Commit**

```bash
git add crates/tracevault-cli/src/commands/login.rs crates/tracevault-cli/src/commands/mod.rs crates/tracevault-cli/src/main.rs
git commit -m "feat: implement tracevault login command with device auth flow"
```

---

### Task 15: Implement `tracevault logout` command

**Files:**
- Create: `crates/tracevault-cli/src/commands/logout.rs`
- Modify: `crates/tracevault-cli/src/commands/mod.rs`
- Modify: `crates/tracevault-cli/src/main.rs`

**Step 1: Write logout command**

```rust
use crate::api_client::ApiClient;
use crate::credentials::Credentials;

pub async fn logout() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials::load()
        .ok_or("Not logged in. No credentials file found.")?;

    let client = ApiClient::new(&creds.server_url, Some(&creds.token));
    match client.logout().await {
        Ok(()) => {}
        Err(e) => eprintln!("Warning: could not invalidate server session: {e}"),
    }

    Credentials::delete()?;
    println!("Logged out. Credentials removed.");
    Ok(())
}
```

**Step 2: Add `pub mod logout;` to commands/mod.rs**

**Step 3: Add Logout variant to CLI enum and wire handler**

```rust
/// Log out from the TraceVault server
Logout,
```

```rust
Cli::Logout => {
    if let Err(e) = commands::logout::logout().await {
        eprintln!("Logout error: {e}");
    }
}
```

**Step 4: Verify it compiles**

Run: `cargo build -p tracevault-cli`

**Step 5: Commit**

```bash
git add crates/tracevault-cli/src/commands/logout.rs crates/tracevault-cli/src/commands/mod.rs crates/tracevault-cli/src/main.rs
git commit -m "feat: implement tracevault logout command"
```

---

### Task 16: Update credential resolution in ApiClient and existing commands

**Files:**
- Modify: `crates/tracevault-cli/src/api_client.rs`
- Modify: `crates/tracevault-cli/src/commands/push.rs`
- Modify: `crates/tracevault-cli/src/commands/sync.rs`
- Modify: `crates/tracevault-cli/src/commands/init.rs`

The goal is to implement the 3-tier credential resolution:
1. `TRACEVAULT_API_KEY` env var
2. `~/.config/tracevault/credentials.json`
3. `.tracevault/config.toml` api_key

**Step 1: Add `resolve_credentials` function to api_client.rs**

```rust
use crate::credentials::Credentials;

/// Resolve server URL and auth token from multiple sources.
/// Returns (server_url, auth_token) if available.
pub fn resolve_credentials(project_root: &std::path::Path) -> (Option<String>, Option<String>) {
    // 1. Env var API key
    let env_key = std::env::var("TRACEVAULT_API_KEY").ok();

    // 2. Credentials file
    let creds = Credentials::load();

    // 3. Project config
    let config_path = crate::config::TracevaultConfig::config_path(project_root);
    let config_content = std::fs::read_to_string(&config_path).unwrap_or_default();

    let config_server_url = config_content
        .lines()
        .find(|l| l.starts_with("server_url"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string());

    let config_api_key = config_content
        .lines()
        .find(|l| l.starts_with("api_key"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string());

    // Resolve server URL: env > creds > config
    let server_url = std::env::var("TRACEVAULT_SERVER_URL").ok()
        .or_else(|| creds.as_ref().map(|c| c.server_url.clone()))
        .or(config_server_url);

    // Resolve token: env api key > creds token > config api key
    let token = env_key
        .or_else(|| creds.map(|c| c.token))
        .or(config_api_key);

    (server_url, token)
}
```

**Step 2: Update push.rs to use resolve_credentials**

Replace the manual config parsing for server_url and api_key at the top of `push_traces` with:

```rust
let (server_url, token) = crate::api_client::resolve_credentials(project_root);
let server_url = server_url.unwrap_or_else(|| "http://localhost:3000".into());
let client = ApiClient::new(&server_url, token.as_deref());
```

Remove the old `config_content` parsing for `server_url` and `api_key` (keep the rest).

**Step 3: Update sync.rs similarly**

Replace server_url/api_key resolution with `resolve_credentials`.

**Step 4: Update init.rs**

When checking if user is logged in before server registration, use `Credentials::load()` to check auth status. If logged in, use stored credentials. If not, and `--server-url` is provided, suggest `tracevault login` first.

**Step 5: Verify it compiles**

Run: `cargo build -p tracevault-cli`

**Step 6: Commit**

```bash
git add crates/tracevault-cli/src/api_client.rs crates/tracevault-cli/src/commands/push.rs crates/tracevault-cli/src/commands/sync.rs crates/tracevault-cli/src/commands/init.rs
git commit -m "feat: implement 3-tier credential resolution across all CLI commands"
```

---

## Phase 4: SvelteKit Web UI

### Task 17: Scaffold SvelteKit project with shadcn-svelte

**Files:**
- Create: `web/` directory (SvelteKit project)

**Step 1: Create SvelteKit project**

```bash
cd /path/to/tracevault
pnpm dlx sv create web
# Select: SvelteKit minimal, TypeScript, Tailwind CSS
cd web
pnpm install
```

**Step 2: Add shadcn-svelte**

```bash
pnpm i shadcn-svelte@latest
pnpm dlx shadcn-svelte@latest init
```

Follow prompts to configure with Tailwind.

**Step 3: Add commonly needed components**

```bash
pnpm dlx shadcn-svelte@latest add button card input label form table data-table badge dialog select sidebar alert
```

**Step 4: Configure API base URL**

In `web/.env`:
```
PUBLIC_API_URL=http://localhost:3000
```

In `web/.env.example`:
```
PUBLIC_API_URL=http://localhost:3000
```

**Step 5: Verify dev server starts**

Run: `cd web && pnpm dev`
Expected: SvelteKit dev server on http://localhost:5173

**Step 6: Commit**

```bash
git add web/
git commit -m "feat: scaffold SvelteKit project with shadcn-svelte and Tailwind"
```

---

### Task 18: Create API client and auth store

**Files:**
- Create: `web/src/lib/api.ts`
- Create: `web/src/lib/stores/auth.ts`

**Step 1: Write API client wrapper**

```typescript
// web/src/lib/api.ts
import { browser } from '$app/environment';
import { goto } from '$app/navigation';

const BASE_URL = import.meta.env.PUBLIC_API_URL || 'http://localhost:3000';

async function request<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const token = browser ? localStorage.getItem('tracevault_token') : null;

  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }

  const resp = await fetch(`${BASE_URL}${path}`, {
    ...options,
    headers,
  });

  if (resp.status === 401 && browser) {
    localStorage.removeItem('tracevault_token');
    goto('/auth/login');
    throw new Error('Unauthorized');
  }

  if (!resp.ok) {
    const body = await resp.text();
    throw new Error(body || `HTTP ${resp.status}`);
  }

  if (resp.status === 204 || resp.headers.get('content-length') === '0') {
    return undefined as T;
  }

  return resp.json();
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, body?: unknown) =>
    request<T>(path, { method: 'POST', body: body ? JSON.stringify(body) : undefined }),
  put: <T>(path: string, body?: unknown) =>
    request<T>(path, { method: 'PUT', body: body ? JSON.stringify(body) : undefined }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};
```

**Step 2: Write auth store**

```typescript
// web/src/lib/stores/auth.ts
import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { api } from '$lib/api';

interface User {
  user_id: string;
  org_id: string;
  org_name: string;
  email: string;
  name: string | null;
  role: string;
}

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  loading: boolean;
}

function createAuthStore() {
  const { subscribe, set, update } = writable<AuthState>({
    user: null,
    isAuthenticated: false,
    loading: true,
  });

  return {
    subscribe,
    async init() {
      if (!browser) return;
      const token = localStorage.getItem('tracevault_token');
      if (!token) {
        set({ user: null, isAuthenticated: false, loading: false });
        return;
      }
      try {
        const user = await api.get<User>('/api/v1/auth/me');
        set({ user, isAuthenticated: true, loading: false });
      } catch {
        localStorage.removeItem('tracevault_token');
        set({ user: null, isAuthenticated: false, loading: false });
      }
    },
    setToken(token: string) {
      if (browser) localStorage.setItem('tracevault_token', token);
    },
    logout() {
      if (browser) localStorage.removeItem('tracevault_token');
      set({ user: null, isAuthenticated: false, loading: false });
    },
  };
}

export const auth = createAuthStore();
```

**Step 3: Verify it compiles**

Run: `cd web && pnpm check`
Expected: no errors

**Step 4: Commit**

```bash
git add web/src/lib/api.ts web/src/lib/stores/auth.ts
git commit -m "feat: add API client and auth store for web UI"
```

---

### Task 19: Create auth pages (login, register, device approval)

**Files:**
- Create: `web/src/routes/auth/login/+page.svelte`
- Create: `web/src/routes/auth/register/+page.svelte`
- Create: `web/src/routes/auth/device/+page.svelte`
- Modify: `web/src/routes/+layout.svelte`
- Modify: `web/src/routes/+page.svelte`

These pages use shadcn-svelte Card, Input, Button, Label components. Exact Svelte 5 syntax may vary — use the shadcn-svelte docs (Context7) during implementation for current component APIs.

**Step 1: Create login page**

`web/src/routes/auth/login/+page.svelte` — form with email + password fields, calls `POST /api/v1/auth/login`, stores token, redirects to `/repos`.

**Step 2: Create register page**

`web/src/routes/auth/register/+page.svelte` — form with org name, email, password, name. Calls `POST /api/v1/auth/register`, stores token, redirects to `/repos`.

**Step 3: Create device approval page**

`web/src/routes/auth/device/+page.svelte` — reads `token` from URL query params. If not logged in, redirects to `/auth/login?redirect=/auth/device?token={token}`. If logged in, shows "Approve CLI login?" card. On approve, calls `POST /api/v1/auth/device/{token}/approve`.

**Step 4: Create root layout with auth guard**

`web/src/routes/+layout.svelte` — initializes auth store. For non-`/auth/*` routes, redirects to login if not authenticated.

**Step 5: Create root page**

`web/src/routes/+page.svelte` — redirects to `/repos` if authenticated, `/auth/login` if not.

**Step 6: Verify it works**

Run: `cd web && pnpm dev`
Navigate to `http://localhost:5173/auth/login`

**Step 7: Commit**

```bash
git add web/src/routes/
git commit -m "feat: add login, register, and device approval pages"
```

---

### Task 20: Create dashboard pages (repos, traces)

**Files:**
- Create: `web/src/routes/repos/+page.svelte`
- Create: `web/src/routes/repos/[id]/+page.svelte`
- Create: `web/src/routes/traces/+page.svelte`
- Create: `web/src/routes/traces/[id]/+page.svelte`
- Create: `web/src/routes/(app)/+layout.svelte` (sidebar layout for authenticated pages)

**Step 1: Create app layout with sidebar**

Sidebar with navigation: Repos, Traces, Settings. Uses shadcn-svelte Sidebar component.

**Step 2: Create repos list page**

Calls `GET /api/v1/repos`, displays in a data-table with columns: name, github_url, created_at. Links to `/repos/[id]`.

**Step 3: Create repo detail page**

Calls `GET /api/v1/traces?repo={name}`, shows traces for that repo in a table.

**Step 4: Create traces list page**

Calls `GET /api/v1/traces`, shows paginated data-table. Columns: commit_sha, author, model, ai_percentage, created_at.

**Step 5: Create trace detail page**

Calls `GET /api/v1/traces/{id}`, shows full trace data including session_data and attribution in cards/accordions.

**Step 6: Verify it works**

Run: `cd web && pnpm dev`

**Step 7: Commit**

```bash
git add web/src/routes/
git commit -m "feat: add repos and traces dashboard pages"
```

---

### Task 21: Create settings pages (org, members, API keys)

**Files:**
- Create: `web/src/routes/settings/+page.svelte`
- Create: `web/src/routes/settings/members/+page.svelte`
- Create: `web/src/routes/settings/api-keys/+page.svelte`

**Step 1: Create org settings page**

Shows org name + plan. Edit button for owner. Calls `GET /api/v1/orgs/{id}` and `PUT /api/v1/orgs/{id}`.

**Step 2: Create members page**

Data table of members with email, name, role, created_at. Invite dialog for admin+. Remove button for owner. Calls `GET/POST/DELETE /api/v1/orgs/{id}/members`.

**Step 3: Create API keys page**

Data table of API keys (name, prefix, created_at). Create dialog. Delete button. Shows the raw key ONCE on creation. Calls `GET/POST/DELETE /api/v1/api-keys`.

**Step 4: Verify it works**

Run: `cd web && pnpm dev`

**Step 5: Commit**

```bash
git add web/src/routes/settings/
git commit -m "feat: add settings pages for org, members, and API keys"
```

---

## Phase 5: Integration & Testing

### Task 22: Add server integration tests for auth flow

**Files:**
- Create: `crates/tracevault-server/tests/auth_test.rs`

Write integration tests that spin up the server with a test database and verify:
1. Register creates org + user + returns token
2. Login with correct password returns token
3. Login with wrong password returns 401
4. Token from register/login works for authenticated endpoints
5. Logout invalidates the token
6. Device flow: start -> pending -> approve -> approved with token
7. Duplicate email registration returns 409

These tests require a running PostgreSQL. Use `sqlx::PgPool` with a test database or use `#[sqlx::test]` macro if available.

**Step 1: Write the test file**

**Step 2: Run tests**

Run: `cargo test -p tracevault-server`

**Step 3: Commit**

```bash
git add crates/tracevault-server/tests/
git commit -m "test: add server integration tests for auth flow"
```

---

### Task 23: Update docker-compose for development

**Files:**
- Modify: `docker-compose.yml`

**Step 1: Add web service and CORS_ORIGIN env var**

```yaml
services:
  web:
    build:
      context: ./web
      dockerfile: Dockerfile
    ports:
      - "5173:5173"
    environment:
      - PUBLIC_API_URL=http://localhost:3000
    depends_on:
      - server

  server:
    environment:
      - CORS_ORIGIN=http://localhost:5173
```

**Step 2: Create web/Dockerfile**

```dockerfile
FROM node:22-slim
WORKDIR /app
RUN npm install -g pnpm
COPY package.json pnpm-lock.yaml ./
RUN pnpm install
COPY . .
CMD ["pnpm", "dev", "--host", "0.0.0.0"]
```

**Step 3: Verify docker-compose up works**

Run: `docker-compose up --build`

**Step 4: Commit**

```bash
git add docker-compose.yml web/Dockerfile
git commit -m "feat: add web UI to docker-compose and configure CORS"
```

---

### Task 24: End-to-end smoke test

Manually verify the full flow works:

1. `docker-compose up` (starts DB, server, web)
2. Open `http://localhost:5173/auth/register` — create an account
3. See the repos page (empty)
4. In a test git repo: `tracevault login --server-url http://localhost:3000`
5. Browser opens device approval page → approve
6. CLI prints "Logged in as ..."
7. `tracevault init` → repo registered on server
8. Verify repo appears in web UI
9. Create an API key in web UI
10. `tracevault logout` → credentials removed

**Step 1: Run through the flow above**

**Step 2: Fix any issues found**

**Step 3: Final commit if any fixes needed**
