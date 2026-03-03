# TraceVault

AI code governance platform for enterprises. Captures what AI coding agents do in your repos — which files they touch, how many tokens they burn, what tools they call, what percentage of code is AI-generated — then enforces policies and produces tamper-evident audit trails for regulatory compliance.

Built for financial institutions and regulated industries where AI-generated code needs the same audit rigor as human-written code.

## What It Does

**Capture** — Hooks into AI coding agents (Claude Code, etc.) and records every tool call, file modification, token usage, and full session transcript.

**Enforce** — Server-managed policy rules block pushes or warn when required tools aren't called (e.g., code review, security scanners). Rules are configurable per-repo via web UI.

**Audit** — Every trace is Ed25519-signed and SHA-256 hash-chained into an immutable, append-only audit trail. Cryptographic verification proves records haven't been tampered with or reordered.

**Comply** — Built-in compliance modes for SOX (7-year retention), PCI-DSS (WORM-equivalent storage), and SR 11-7 (model risk management). Role-based access control enforces separation of duties.

## Architecture

Three Rust crates in a Cargo workspace:

- **tracevault-core** — domain types, policy engine (7 condition types), attribution engine, secret redactor
- **tracevault-cli** — CLI binary that hooks into Claude Code, captures traces locally, checks policies, pushes to server
- **tracevault-server** — axum HTTP server backed by PostgreSQL with Ed25519 signing, audit logging, RBAC

Plus a SvelteKit web dashboard and a GitHub Action for CI integration.

## Prerequisites

- Rust 1.84+ (install via [rustup](https://rustup.rs))
- PostgreSQL 16+ (or Docker)
- Node.js 20+ and pnpm (for the web dashboard)
- Docker & Docker Compose (for containerized deployment)

## Quick Start

### 1. Build from source

```sh
cargo build --release
```

Binaries land in `target/release/`:
- `tracevault` (CLI)
- `tracevault-server`

### 2. Start the server with Docker Compose

```sh
docker compose up -d
```

This starts PostgreSQL and the TraceVault server on port 3000. Migrations run automatically on startup.

To run just the database (useful during development):

```sh
docker compose up -d db
```

### 3. Generate a signing key

TraceVault uses Ed25519 signatures to create a tamper-evident audit trail. Every trace is signed and hash-chained. **You must provide a persistent signing key** — without one, the server generates an ephemeral key on each startup and all previous signatures become unverifiable.

Generate a 32-byte key and base64-encode it:

```sh
# Using openssl:
openssl rand -base64 32

# Or using Python:
python3 -c "import secrets, base64; print(base64.b64encode(secrets.token_bytes(32)).decode())"
```

Set it as an environment variable:

```sh
export TRACEVAULT_SIGNING_KEY=<output-from-above>
```

For Docker Compose, add it to the server service (see below). For production, store it in your secrets manager (Vault, AWS Secrets Manager, etc.) and inject it at deploy time.

**Backup the key.** If lost, all existing signatures become unverifiable and the chain integrity check will fail. The key cannot be recovered from the database.

To export the corresponding public key (for external verification):

```
GET /api/v1/orgs/{id}/compliance/public-key
```

### 4. Run the server locally (development)

With PostgreSQL running on localhost:

```sh
export DATABASE_URL=postgres://tracevault:tracevault@localhost:5432/tracevault
export HOST=0.0.0.0
export PORT=3000
export TRACEVAULT_SIGNING_KEY=<base64-encoded-32-byte-key>

cargo run -p tracevault-server
```

### 5. Run the web dashboard

```sh
cd web
pnpm install
pnpm dev
```

The dashboard runs on `http://localhost:5173` by default and proxies API calls to the server.

### 6. Initialize TraceVault in a repository

```sh
cd /path/to/your/repo
tracevault init
```

This creates a `.tracevault/` directory and installs a pre-push hook. The hook runs:

```sh
tracevault sync       # collect latest session data
tracevault check      # evaluate policies against server rules (blocks push on failure)
tracevault push       # upload traces to server
```

The command also prints the Claude Code hook configuration. Add it to your `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "tracevault hook --event pre-tool-use",
            "timeout": 5
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Write|Edit|Bash",
        "hooks": [
          {
            "type": "command",
            "command": "tracevault hook --event post-tool-use",
            "timeout": 5
          }
        ]
      }
    ]
  }
}
```

### 7. Push traces and check policies

```sh
# Push traces to a specific server:
TRACEVAULT_SERVER_URL=https://your-server.example.com tracevault push

# Check policies before pushing (also runs automatically via pre-push hook):
tracevault check

# View local session stats:
tracevault stats
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `tracevault init` | Initialize TraceVault in current repo, install pre-push hook |
| `tracevault status` | Show current session status |
| `tracevault hook --event <type>` | Handle a Claude Code hook event (reads JSON from stdin) |
| `tracevault push` | Push collected traces to the server |
| `tracevault check` | Evaluate policies against server rules, exit non-zero if blocked |
| `tracevault stats` | Show local session statistics |

## Policy Enforcement

Policies are managed per-repo through the web UI or API. Two condition types:

**Required Tool Call** — Require specific tools to be called during any session. Example: every session must call `mcp__codex-cli__review`.

**Conditional Tool Call** — Require a tool when specific files are modified. Example: require `mcp__security-scanner__scan` at least once when files matching `src/auth/**` are changed.

Each policy has a configurable action:
- **Block Push** — `tracevault check` exits non-zero, preventing `git push`
- **Warn** — prints a warning but allows the push

Fail-closed: if the server is unreachable, `tracevault check` blocks the push.

## Compliance & Audit Trail

### Immutable Trace Storage

Every trace pushed to the server is:
1. **Hashed** — SHA-256 digest of the canonical record content
2. **Chained** — each record's hash is linked to the previous, forming a verifiable chain
3. **Signed** — Ed25519 digital signature proves authenticity and prevents tampering
4. **Sealed** — timestamp marks when the record was finalized

Records are append-only. No UPDATE or DELETE on sealed data. Corrections create amendment records referencing the original.

### Compliance Modes

| Mode | Min Retention | Signing | Chain Verification | Use Case |
|------|--------------|---------|-------------------|----------|
| SOX | 7 years | Required | Daily | Public companies, financial reporting systems |
| PCI-DSS | 1 year | Required | Daily | Payment processing, cardholder data environments |
| SR 11-7 | 3 years | Required | Weekly | Banks using AI models in risk/trading/compliance |
| Custom | Configurable | Configurable | Configurable | Mix-and-match for specific requirements |

Compliance mode is set per-organization. When active, the system enforces minimum retention periods and required signing — admins can increase but not decrease below framework minimums.

### Chain Verification

Verify the integrity of the entire audit trail:

```
POST /api/v1/orgs/{id}/compliance/verify-chain
```

Walks every sealed commit, verifies each hash link and Ed25519 signature, and reports pass/fail with details on any broken links.

### RBAC (Role-Based Access Control)

Five roles enforce separation of duties:

| Role | Push Traces | Manage Policies | View All Traces | View Audit Log | Manage Users |
|------|------------|----------------|-----------------|---------------|-------------|
| Owner | Yes | Yes | Yes | Yes | Yes |
| Admin | Yes | Yes | Yes | Yes | Yes |
| Policy Admin | Yes | Yes | Yes | Yes | No |
| Developer | Yes | No | Own only | No | No |
| Auditor | No | No | Yes (read-only) | Yes (read-only) | No |

### Audit Log

Every state-changing operation is logged to an append-only audit trail:
- Trace creation and sealing
- Policy CRUD and check results
- User login/logout and registration
- Role changes
- Compliance settings updates
- Chain verification results

Query via API with filters (action type, actor, resource, date range) or browse in the web dashboard.

## Web Dashboard

- **Repos** — list repositories, view commits and sessions, manage per-repo policies
- **Traces** — browse commits, drill into sessions with full transcripts, token breakdowns, tool usage, diff viewer with AI attribution highlighting
- **Analytics** — token usage, model distribution, author activity, cost tracking, AI attribution percentages
- **Compliance** — chain integrity status, compliance mode, retention, role distribution, recent audit log, settings configuration, full audit log browser
- **Settings** — organization management, team members with role assignment, API keys

## Server API

### Auth

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/register` | Register a new org + owner account |
| POST | `/api/v1/auth/login` | Login, get session token |
| POST | `/api/v1/auth/device` | Start device auth flow (for CLI) |
| GET | `/api/v1/auth/device/{token}/status` | Poll device auth status |
| POST | `/api/v1/auth/device/{token}/approve` | Approve device auth request |
| POST | `/api/v1/auth/logout` | End session |
| GET | `/api/v1/auth/me` | Get current user info |

### Traces

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/traces` | Submit a trace (append-only, signed, chained) |
| GET | `/api/v1/traces` | List traces (query: `repo`, `author`, `limit`) |
| GET | `/api/v1/traces/{id}` | Get trace detail with sessions |
| GET | `/api/v1/traces/{id}/verify` | Verify signature and chain integrity |

### Repos & Policies

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/repos` | List repositories |
| POST | `/api/v1/repos` | Register a repository |
| DELETE | `/api/v1/repos/{id}` | Delete a repository |
| GET | `/api/v1/repos/{repo_id}/policies` | List policies for a repo |
| POST | `/api/v1/repos/{repo_id}/policies` | Create a policy |
| POST | `/api/v1/repos/{repo_id}/policies/check` | Evaluate policies against session data |
| PUT | `/api/v1/policies/{id}` | Update a policy |
| DELETE | `/api/v1/policies/{id}` | Delete a policy |

### Compliance

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/orgs/{id}/compliance` | Get compliance settings |
| PUT | `/api/v1/orgs/{id}/compliance` | Update compliance settings |
| GET | `/api/v1/orgs/{id}/compliance/public-key` | Get Ed25519 public key for verification |
| POST | `/api/v1/orgs/{id}/compliance/verify-chain` | Run chain integrity verification |
| GET | `/api/v1/orgs/{id}/compliance/chain-status` | Get last verification result |
| GET | `/api/v1/orgs/{id}/audit-log` | Query audit log (paginated, filterable) |

### Orgs & Members

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/orgs/{id}` | Get org details |
| PUT | `/api/v1/orgs/{id}` | Update org |
| GET | `/api/v1/orgs/{id}/members` | List members |
| POST | `/api/v1/orgs/{id}/members` | Invite member |
| DELETE | `/api/v1/orgs/{id}/members/{user_id}` | Remove member |
| PUT | `/api/v1/orgs/{id}/members/{user_id}/role` | Change member role |

### Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/analytics/overview` | Dashboard overview stats |
| GET | `/api/v1/analytics/tokens` | Token usage over time |
| GET | `/api/v1/analytics/models` | Usage by model |
| GET | `/api/v1/analytics/authors` | Usage by author |
| GET | `/api/v1/analytics/attribution` | AI vs human code attribution |
| GET | `/api/v1/analytics/sessions` | Session-level analytics |
| GET | `/api/v1/analytics/cost` | Cost tracking |

## Built-in Policy Conditions

| Condition | What it checks |
|-----------|---------------|
| Trace Completeness | Every commit has a corresponding trace |
| AI Percentage Threshold | Warns when AI-authored code exceeds a threshold |
| Model Allowlist | Only approved model families allowed |
| Sensitive Path Pattern | Flags AI edits to sensitive paths (payments, auth, crypto) |
| Required Tool Call | Ensures specific tools were called during the session |
| Conditional Tool Call | Requires a tool when files matching glob patterns are modified |
| Token Budget | Warns when token usage or cost exceeds limits |

## GitHub Action

Add to your workflow:

```yaml
- uses: softwaremill/tracevault@main
  with:
    server-url: https://your-tracevault-server.example.com
    api-key: ${{ secrets.TRACEVAULT_API_KEY }}
```

The action fetches git notes, counts traced commits in the PR, and writes a summary to the PR check.

## Configuration

### CLI config (`.tracevault/config.toml`)

```toml
agent = "claude-code"
# server_url = "https://your-server.example.com"
# api_key = "your-api-key"
```

### Server environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://tracevault:tracevault@localhost:5432/tracevault` | PostgreSQL connection string |
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | Bind port |
| `CORS_ORIGIN` | _(permissive)_ | Allowed CORS origin for web dashboard |
| `TRACEVAULT_SIGNING_KEY` | _(ephemeral)_ | **Recommended.** Base64-encoded 32-byte Ed25519 seed for trace signing. Generate with `openssl rand -base64 32`. If not set, a new ephemeral key is generated on each restart and all previous signatures become unverifiable. |
| `RUST_LOG` | — | Log level (e.g. `info`, `debug`) |

## Project Structure

```
crates/
  tracevault-core/       # Domain types, policy engine, attribution, redactor
  tracevault-cli/        # CLI binary (clap + tokio)
  tracevault-server/     # HTTP server (axum + sqlx + PostgreSQL + Ed25519)
    src/
      api/               # Route handlers (auth, traces, repos, policies, compliance, analytics)
      signing.rs         # Ed25519 signing and hash chaining
      audit.rs           # Append-only audit log
      permissions.rs     # RBAC role/permission matrix
    migrations/          # PostgreSQL migrations (run automatically on startup)
web/                     # SvelteKit 5 dashboard (shadcn-svelte + Tailwind)
action/                  # GitHub Action (composite)
docs/
  plans/                 # Design docs and implementation plans
  research/              # Compliance framework research (SOX, PCI-DSS, SR 11-7, EU AI Act, etc.)
docker-compose.yml       # PostgreSQL + server
Dockerfile               # Multi-stage server build
```

## License

Apache-2.0
