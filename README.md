# TraceVault

AI code governance platform. Captures what AI coding agents do in your repo — which files they touch, how many tokens they burn, what percentage of code is AI-generated — and enforces policies before code gets merged.

## Architecture

Three Rust crates in a Cargo workspace:

- **tracevault-core** — domain types, policy engine (6 built-in policies), secret redactor, attribution engine
- **tracevault-cli** — CLI binary that hooks into Claude Code, captures trace data locally, pushes to server
- **tracevault-server** — axum HTTP server backed by PostgreSQL, stores traces, runs policy evaluations

Plus a GitHub Action (`action/`) for PR-level policy checks.

## Prerequisites

- Rust 1.84+ (install via [rustup](https://rustup.rs))
- PostgreSQL 16+ (or Docker)
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

### 3. Run the server locally (development)

With PostgreSQL running on localhost:

```sh
# Optional — these are the defaults:
export DATABASE_URL=postgres://tracevault:tracevault@localhost:5432/tracevault
export HOST=0.0.0.0
export PORT=3000

cargo run -p tracevault-server
```

### 4. Initialize TraceVault in a repository

```sh
cd /path/to/your/repo
tracevault init
```

This creates a `.tracevault/` directory with config, sessions, and cache subdirectories.

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

### 5. Push traces to the server

After a coding session:

```sh
tracevault push
```

To push to a specific server:

```sh
TRACEVAULT_SERVER_URL=https://your-server.example.com tracevault push
```

### 6. View local stats

```sh
tracevault stats
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `tracevault init` | Initialize TraceVault in current repo |
| `tracevault status` | Show current session status |
| `tracevault hook --event <type>` | Handle a Claude Code hook event (reads JSON from stdin) |
| `tracevault push` | Push collected traces to the server |
| `tracevault stats` | Show local session statistics |

## Server API

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| POST | `/api/v1/traces` | Submit a trace |
| GET | `/api/v1/traces` | List traces (query params: `repo`, `author`, `limit`) |
| GET | `/api/v1/traces/{id}` | Get a single trace |
| POST | `/api/v1/auth/register` | Register an org |
| GET | `/api/v1/policies` | List policies |
| POST | `/api/v1/policies/evaluate` | Evaluate policies against a trace |
| GET | `/api/v1/analytics/tokens` | Token usage analytics |
| POST | `/api/v1/github/webhook` | GitHub webhook receiver |

## Built-in Policies

| Policy | What it checks |
|--------|---------------|
| Trace Completeness | Every commit has a corresponding trace |
| AI Percentage Threshold | Warns when AI-authored code exceeds 90% |
| Model Allowlist | Only approved model families (Claude, GPT, Gemini) |
| Sensitive Path Pattern | Flags AI edits to sensitive paths (payments, auth, crypto, secrets) |
| Required Tool Call | Ensures test commands were run during the session |
| Token Budget | Warns when token usage exceeds 500K tokens or $50 estimated cost |

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
| `RUST_LOG` | — | Log level (e.g. `info`, `debug`) |

## Running Tests

```sh
cargo test
```

All tests run without a database. Server integration tests that need PostgreSQL are planned for a future iteration.

## Project Structure

```
crates/
  tracevault-core/       # Domain types, policy engine, redactor, attribution
  tracevault-cli/        # CLI binary (clap + tokio)
  tracevault-server/     # HTTP server (axum + sqlx + PostgreSQL)
action/                  # GitHub Action (composite)
docker-compose.yml       # PostgreSQL + server
Dockerfile               # Multi-stage server build
```

## License

Apache-2.0
