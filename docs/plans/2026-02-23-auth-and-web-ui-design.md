# Auth & Web UI Design

## Summary

Add user authentication to TracevaultCLI (device authorization flow) and a SvelteKit web dashboard for managing organizations, repositories, and users.

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| CLI auth model | Device auth flow | Browser-based, like gcloud/gh CLI. No codes to type. |
| Server-side identity | Email + password (OAuth-ready schema) | Simple, no external deps. Add OAuth providers later. |
| Token format | Opaque tokens + server-side sessions | Instantly revocable, simple, no JWT complexity. |
| Password hashing | Argon2id (`argon2` crate) | Current best practice for new projects. |
| Web UI framework | Separate SvelteKit app with shadcn-svelte | Rich component library, separate deployment. |
| Web UI hosting | Standalone SvelteKit app (not embedded in Axum) | Separate process, CORS between UI and API. |

## Database Schema

### New Tables

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID REFERENCES orgs(id),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name TEXT,
    role TEXT NOT NULL DEFAULT 'member',  -- 'owner', 'admin', 'member'
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE TABLE device_auth_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT NOT NULL UNIQUE,
    user_id UUID REFERENCES users(id),  -- NULL until approved
    status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'approved', 'expired'
    session_id UUID REFERENCES auth_sessions(id),  -- set on approval
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);

CREATE INDEX idx_auth_sessions_token ON auth_sessions(token_hash);
CREATE INDEX idx_device_auth_token ON device_auth_requests(token);
```

OAuth-ready: add `provider` and `provider_id` columns to `users` later.

## Server Auth Endpoints

### Auth Flow Endpoints (no auth required)

| Method | Endpoint | Purpose |
|--------|----------|---------|
| POST | `/api/v1/auth/register` | Create org + owner user |
| POST | `/api/v1/auth/login` | Email/password login, returns session token |
| POST | `/api/v1/auth/device` | Start device flow, returns `{token, verification_url}` |
| GET | `/api/v1/auth/device/:token/status` | CLI polls this |
| POST | `/api/v1/auth/device/:token/approve` | Web UI approves device (requires session) |
| POST | `/api/v1/auth/logout` | Invalidate session (requires session) |
| GET | `/api/v1/auth/me` | Current user + org info (requires session) |

### Management Endpoints (session required)

| Method | Endpoint | Min Role | Purpose |
|--------|----------|----------|---------|
| GET | `/api/v1/orgs/:id` | admin | Get org details |
| PUT | `/api/v1/orgs/:id` | owner | Update org settings |
| GET | `/api/v1/orgs/:id/members` | admin | List members |
| POST | `/api/v1/orgs/:id/members` | admin | Invite user |
| DELETE | `/api/v1/orgs/:id/members/:uid` | owner | Remove member |
| PUT | `/api/v1/orgs/:id/members/:uid/role` | owner | Change role |
| GET | `/api/v1/repos` | member | List repos |
| DELETE | `/api/v1/repos/:id` | admin | Remove repo |
| POST | `/api/v1/api-keys` | member | Generate API key |
| DELETE | `/api/v1/api-keys/:id` | member | Revoke API key |
| GET | `/api/v1/api-keys` | member | List API keys (masked) |

### Existing Endpoints

`POST /api/v1/traces`, `GET /api/v1/traces`, `GET /api/v1/traces/:id`, `POST /api/v1/repos` all require session or API key auth after this change. `GET /health` stays open.

### Auth Middleware

Axum extractor `AuthUser` checks `Authorization: Bearer {token}` header. Tries `auth_sessions` first (SHA-256 hash lookup), then `api_keys`. Returns 401 if neither matches. Populates `AuthUser { user_id, org_id, role }`.

## CLI Auth Flow

### `tracevault login`

1. CLI calls `POST /api/v1/auth/device` with server URL
2. Opens `{server_url}/auth/device?token={token}` in browser (`open` crate)
3. Polls `GET /api/v1/auth/device/{token}/status` every 2s (5min timeout)
4. On success, stores credentials to `~/.config/tracevault/credentials.json`:

```json
{
  "server_url": "https://tv.example.com",
  "token": "tvs_...",
  "email": "user@example.com",
  "org_name": "myorg"
}
```

### `tracevault logout`

Calls `POST /api/v1/auth/logout`, deletes credentials file.

### Credential Resolution Order

1. `TRACEVAULT_API_KEY` env var (CI)
2. `~/.config/tracevault/credentials.json` (device login)
3. `.tracevault/config.toml` `api_key` field (legacy/per-project)

### Changes to `tracevault init`

If logged in: use stored credentials for registration. If not logged in with `--server-url`: print "Run `tracevault login` first", skip registration. No server URL: local-only init.

## Web UI

### Tech Stack

- SvelteKit (separate app in `web/` directory)
- shadcn-svelte (component library)
- Tailwind CSS
- TypeScript

### Pages

| Route | Purpose |
|-------|---------|
| `/auth/login` | Login form |
| `/auth/register` | Create org + owner account |
| `/auth/device?token=...` | Approve CLI device login |
| `/repos` | Repo list (data table) |
| `/repos/[id]` | Repo detail with traces |
| `/traces` | Paginated trace list |
| `/traces/[id]` | Trace detail (session data, attribution) |
| `/settings` | Org settings |
| `/settings/members` | User management |
| `/settings/api-keys` | API key management |

### API Communication

`lib/api.ts` wraps fetch with base URL from `PUBLIC_API_URL` env var, auto-attaches Bearer token from localStorage, redirects to login on 401.

### Device Auth Approval

`/auth/device?token=abc123` — if not logged in, redirects to login with return URL. If logged in, shows approval card. On approve, calls `POST /api/v1/auth/device/{token}/approve`.

## CORS & Deployment

- Development: Vite proxy or Axum CORS for `localhost:5173`
- Production: `tower-http` CORS middleware, origin from `CORS_ORIGIN` env var
- Alternative: reverse proxy (nginx/Caddy) routing `/api/*` to Axum, `/*` to SvelteKit

## Security

- Passwords: Argon2id
- Tokens: 32 random bytes, hex-encoded, prefixed `tvs_` (session) / `tvk_` (API key), stored as SHA-256 hash
- Device auth: 10-minute expiry, single-use
- Rate limiting: login endpoint, 5 attempts/min/IP (`tower-governor` or in-memory)
- CSRF: not needed (token-based API auth, no cookie auth)
