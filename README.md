# TraceVault

> **Research Preview** — This project is under heavy development. APIs, configuration, and features may change significantly between releases.

![Analytics Overview](docs/images/analytics-overview.png)

AI code governance platform for enterprises. Captures what AI coding agents do in your repos — which files they touch, how many tokens they burn, what tools they call, what percentage of code is AI-generated — then enforces policies and produces tamper-evident audit trails for regulatory compliance.

Built for financial institutions and regulated industries where AI-generated code needs the same audit rigor as human-written code.

[Learn more at VirtusLab](https://virtuslab.com/services/tracevault)

## Five Pillars of AI Governance

### 1. Capture — Full Session Tracing

Record every AI interaction with full fidelity. TraceVault hooks into AI coding agents and automatically captures session transcripts, token breakdowns (input/output/cache per model), every tool call invocation, file modifications with diffs, and cost estimates. Secrets and credentials are redacted before storage.

Nothing to configure — once initialized, capture is automatic and invisible.

### 2. Enforce — Policy Engine

Keep AI within bounds. Define rules per-repository that are evaluated on every push:

- **Model allowlists** — restrict which AI models can be used
- **Sensitive path protection** — flag AI edits to critical paths (`/payments/`, `/auth/`, `/crypto/`)
- **Required tool calls** — mandate security scanners or code review tools
- **AI percentage thresholds** — warn when AI-authored code exceeds a limit
- **Token budgets** — cap token usage or cost per session

Policies can either **block the push** (exit non-zero) or **warn**. Fail-closed by default: if the server is unreachable, the push is blocked.

### 3. Audit — Cryptographically Signed Chain of Events

Every trace pushed to the server is transformed into a tamper-proof record:

1. **Hashed** — SHA-256 digest of the canonical record
2. **Chained** — each hash links to the previous, forming a verifiable chain
3. **Signed** — Ed25519 digital signature proves authenticity
4. **Sealed** — timestamp marks when the record was finalized

Records are append-only — no updates, no deletes. Corrections create amendment records referencing the original. The entire chain can be verified at any time to prove nothing was altered or reordered.

Built-in compliance modes for **SOX** (7-year retention), **PCI-DSS** (1-year, WORM-equivalent), and **SR 11-7** (model risk management for banks). RBAC with five roles — including a dedicated **Auditor** role with read-only access to all traces and the audit log.

### 4. Analyze — Usage Analytics

Understand how AI is used across your team:

- **Token usage trends** over time, per model, per author
- **Model distribution** — which models are used most and where
- **Cost tracking** — estimated cost breakdown by model and team member
- **Cache savings** — how much prompt caching saves
- **AI attribution** — what percentage of your codebase is AI-generated
- **Author activity** — commits, tokens, and cost per developer

All available through the web dashboard with filterable time ranges and drill-down views.

### 5. Code — Stories & Documentation

See exactly what AI wrote, line by line. The code browser overlays AI attribution on your source files — highlighting which lines were AI-generated, which function or class they belong to (via tree-sitter scope detection), and linking back to the session that produced them.

**Story generation** turns raw traces into human-readable narratives: why the AI chose a particular pattern, what alternatives were considered, and what the developer's intent was. Auto-generated Architecture Decision Records give new team members full context without asking anyone.

## Architecture

Three Rust crates in a Cargo workspace:

- **tracevault-core** — domain types, policy engine (7 condition types), attribution engine (tree-sitter based), secret redactor
- **tracevault-cli** — CLI binary that hooks into Claude Code, captures traces locally, checks policies, pushes to server
- **tracevault-server** — axum HTTP server backed by PostgreSQL with Ed25519 signing, audit logging, RBAC, code browser

Plus a SvelteKit web dashboard and a GitHub Action for CI verification.

## Prerequisites

- Rust stable toolchain (install via [rustup](https://rustup.rs))
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
# Generate a signing key first (required)
export TRACEVAULT_SIGNING_KEY=$(openssl rand -base64 32)

docker compose up -d
```

This starts PostgreSQL, the TraceVault server on port 3000, and the web dashboard on port 5173. Migrations run automatically on startup.

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

For Docker Compose, the key is required via the `TRACEVAULT_SIGNING_KEY` env var. For production, store it in your secrets manager (Vault, AWS Secrets Manager, etc.) and inject it at deploy time.

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
tracevault sync       # sync repo metadata with server
tracevault check      # evaluate policies against server rules (blocks push on failure)
tracevault push       # upload traces to server
```

The command also installs the Claude Code hook configuration in `.claude/settings.json`:

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

### 7. Authenticate and push traces

```sh
# Log in to a TraceVault server (opens browser for device auth):
tracevault login --server-url https://your-server.example.com

# Push traces to the server:
tracevault push

# Check policies before pushing (also runs automatically via pre-push hook):
tracevault check

# View local session stats:
tracevault stats

# Verify commits are sealed on the server:
tracevault verify --range HEAD~5..HEAD
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `tracevault init [--server-url URL]` | Initialize TraceVault in current repo, install pre-push hook and Claude Code hooks |
| `tracevault login --server-url URL` | Authenticate via device auth flow (opens browser) |
| `tracevault logout` | Clear local credentials |
| `tracevault hook --event <type>` | Handle a Claude Code hook event (reads JSON from stdin) |
| `tracevault sync` | Sync repo metadata with the server |
| `tracevault check` | Evaluate policies against server rules, exit non-zero if blocked |
| `tracevault push` | Push collected traces to the server |
| `tracevault stats` | Show local session statistics |
| `tracevault verify` | Verify commits are registered and sealed on the server (`--commits` or `--range`) |
| `tracevault status` | Show current session status (not yet implemented) |

## Policy Enforcement

Policies are managed per-repo through the web UI or API. Seven condition types:

| Condition | What it checks |
|-----------|---------------|
| Trace Completeness | Every commit has a corresponding trace |
| AI Percentage Threshold | Warns when AI-authored code exceeds a threshold |
| Model Allowlist | Only approved model families allowed |
| Sensitive Path Pattern | Flags AI edits to sensitive paths (payments, auth, crypto) |
| Required Tool Call | Ensures specific tools were called during the session |
| Conditional Tool Call | Requires a tool when files matching glob patterns are modified |
| Token Budget | Warns when token usage or cost exceeds limits |

Each policy has a configurable action:
- **Block Push** — `tracevault check` exits non-zero, preventing `git push`
- **Warn** — prints a warning but allows the push

Fail-closed: if the server is unreachable, `tracevault check` blocks the push.

## Compliance & Audit Trail

### Compliance Modes

| Mode | Min Retention | Signing | Chain Verification | Use Case |
|------|--------------|---------|-------------------|----------|
| SOX | 7 years | Required | Daily | Public companies, financial reporting systems |
| PCI-DSS | 1 year | Required | Daily | Payment processing, cardholder data environments |
| SR 11-7 | 3 years | Required | Weekly | Banks using AI models in risk/trading/compliance |
| Custom | Configurable | Configurable | Configurable | Mix-and-match for specific requirements |

Compliance mode is set per-organization. When active, the system enforces minimum retention periods and required signing — admins can increase but not decrease below framework minimums.

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

## GitHub Action

Add to your workflow to verify that commits in a PR or push have corresponding traces sealed on the server:

```yaml
- uses: softwaremill/tracevault@main
  with:
    server-url: https://your-tracevault-server.example.com
    api-key: ${{ secrets.TRACEVAULT_API_KEY }}
```

The action installs the TraceVault CLI, detects the commit range from the PR or push event, runs `tracevault verify --range`, and writes a pass/fail summary to the GitHub Actions step summary.

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
| `TRACEVAULT_REPOS_DIR` | `./data/repos` | Directory for cloned git repos (used by code browser) |
| `TRACEVAULT_ENCRYPTION_KEY` | — | AES-256 encryption key (base64-encoded 32 bytes) for encryption at rest |
| `TRACEVAULT_LLM_PROVIDER` | — | LLM provider for story generation (e.g., `openai`, `anthropic`) |
| `TRACEVAULT_LLM_API_KEY` | — | API key for the LLM provider |
| `TRACEVAULT_LLM_MODEL` | — | LLM model name |
| `TRACEVAULT_LLM_BASE_URL` | — | Custom LLM endpoint URL |
| `RUST_LOG` | — | Log level (e.g. `info`, `debug`) |

## License

Apache-2.0
