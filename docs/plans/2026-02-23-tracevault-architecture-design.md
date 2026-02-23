# TraceVault Architecture Design

**Date:** 2026-02-23
**Status:** Approved design — ready for implementation planning

## 1. Problem Statement

When AI agents write code, organizations lose visibility into the decision-making process. Git commits show *what* changed, but not *why* an agent made specific choices, which model was used, what alternatives were considered, or how many tokens it cost.

Existing tools (git-ai, entire) solve parts of this for individual developers. No platform exists that provides organizational governance: policy enforcement, compliance audit trails, cross-repo analytics, and cost tracking.

TraceVault fills this gap.

## 2. Product Positioning

### What TraceVault Is

An open-core platform for AI code governance. It collects, stores, validates, and reports on AI agent activity in software development.

### Target Audiences

- **Primary:** Financial institutions (SOX, PCI-DSS, SR 11-7 compliance)
- **Secondary:** Any engineering organization adopting AI coding tools at scale

### Open-Core Model

| | Open-source (free) | Commercial |
|---|---|---|
| **Collection** | Full trace collection from all supported agents | Same |
| **CLI** | Complete — collect, validate, basic reports | Same |
| **GitHub Action** | Single-repo, 6 built-in policies | Same |
| **Storage** | Self-hosted server (single team) | Managed SaaS or self-hosted enterprise |
| **Policies** | 6 built-in | Custom policy-as-code engine, unlimited rules |
| **Analytics** | Single-repo stats via CLI | Cross-repo dashboards, org-wide analytics |
| **GitHub App** | No | Yes — org-level enforcement, required checks |
| **Compliance reports** | Basic JSON/SARIF output | SOX/PCI-DSS/SR 11-7 formatted exports |
| **Integrations** | GitHub only | Jira, Slack, SAML/SSO, RBAC |
| **Audit trail** | Standard | Cryptographically signed, tamper-proof |
| **Support** | Community | SLA-backed |

### Relationship to Existing Tools

| Tool | Focus | TraceVault's relationship |
|---|---|---|
| **git-ai** | Per-line attribution (which AI wrote which line) | TraceVault does its own collection; may read git-ai notes as additional data source in the future |
| **entire** | Session capture and replay | TraceVault captures similar session data but adds governance layer |
| **Agent Trace spec** | Vendor-neutral attribution format (Cursor, Jan 2026) | TraceVault can produce Agent Trace-compatible records; the spec defines format, TraceVault adds storage + policies + analytics |
| **Visdom** (VirtusLab) | "Autonomous Software Factory" — makes agents faster and more correct | Complementary: Visdom = how agents work well; TraceVault = how to prove they worked well |

## 3. Full System Architecture

```
Developer Machine                     TraceVault Server (Docker)
+-------------------------+          +-----------------------------+
| AI Agent (Claude Code,  |          |                             |
|  Cursor, Copilot...)    |          |  axum HTTP server           |
|   | PreToolUse hook     |          |    |                        |
|   | PostToolUse hook    |          |    +-- /api/v1/traces       |
|   v                     |          |    +-- /api/v1/policies     |
| TraceVault CLI          |  HTTPS   |    +-- /api/v1/github      |
|   +-- Trace Collector   |--------->|    +-- /api/v1/analytics    |
|   |   (hooks -> events) |          |         |                   |
|   +-- Attribution Engine|          |    PostgreSQL                |
|   |   (diff -> line map)|          |         |                   |
|   +-- Token Tracker     |          |    Policy Engine             |
|   +-- Secret Redactor   |          |         |                   |
|   +-- Local Cache       |          |    GitHub Integration        |
|   |   (git notes+SQLite)|          |         |                   |
|   +-- API Client        |          |    Web Dashboard (future)    |
|       (push to server)  |          |                             |
+-------------------------+          +-----------------------------+
                                                |
GitHub                                          |
+-------------------------+                     |
| PR opened               |                     |
|   v                     |                     |
| GitHub Action           |  HTTPS              |
|   +-- read git notes    |---------+---------->|
|   +-- call policy eval  |
|   +-- post PR comment   |
|   +-- set status check  |
+-------------------------+
```

### Data Flow (Happy Path)

1. Developer prompts Claude Code
2. Claude Code's pre/post tool hooks fire -> TraceVault CLI captures events
3. CLI records: session metadata, transcript chunks, file diffs, token usage
4. On `git commit` -> CLI computes line-level attribution from accumulated diffs
5. CLI redacts secrets, pushes trace to server API
6. CLI stores backup in git notes (offline resilience)
7. On PR creation -> GitHub Action calls server to evaluate policies
8. Server runs policy engine, posts status check + comment on PR
9. Dashboard shows org-wide analytics (future; API-only in MVP)

### Components

**TraceVault CLI** (Rust binary, installed on developer machine)
- Installs hooks into AI coding tools
- Captures trace events in real-time during agent sessions
- Computes line-level attribution at commit time
- Redacts secrets before any data leaves the machine
- Pushes traces to server; stores git notes as offline backup

**TraceVault Server** (Single Rust binary, axum + PostgreSQL)
- Ingests and stores trace records
- Evaluates policies against traces
- Serves analytics queries
- Handles GitHub webhook events (for GitHub App, enterprise)
- Serves web dashboard (future)

**GitHub Action** (open-source, runs in CI)
- Reads traces from git notes or server
- Evaluates policies
- Posts PR comment with AI summary, token cost, policy results
- Sets GitHub status check (pass/fail)

**GitHub App** (enterprise, future)
- Org-level installation
- Webhook-driven (no CI dependency)
- Required status checks
- Org-wide policy management

## 4. Data Model

### Trace Record

```
TraceRecord
+-- id: UUID
+-- repo_id: String (org/repo)
+-- commit_sha: String (40 chars)
+-- branch: String
+-- author: String (git author)
+-- created_at: DateTime
+-- model: String (e.g., "anthropic/claude-opus-4-6")
+-- tool: String (e.g., "claude-code")
+-- tool_version: String
|
+-- session: Session
|   +-- session_id: String
|   +-- started_at: DateTime
|   +-- ended_at: DateTime
|   +-- prompts: Vec<Prompt>
|   |   +-- text, timestamp
|   +-- responses: Vec<Response> (redacted)
|   |   +-- text, timestamp, tool_calls
|   +-- token_usage: TokenUsage
|   |   +-- model: String
|   |   +-- input_tokens: u64
|   |   +-- output_tokens: u64
|   |   +-- cache_read_tokens: u64
|   |   +-- cache_write_tokens: u64
|   |   +-- total_tokens: u64
|   |   +-- estimated_cost_usd: f64
|   |   +-- api_calls: u32
|   |   +-- subagent_usage: Vec<TokenUsage>
|   +-- tools_used: Vec<ToolCall>
|       +-- name, input_summary, timestamp
|
+-- attribution: Attribution
|   +-- files: Vec<FileAttribution>
|   |   +-- path: String
|   |   +-- lines_added: u32
|   |   +-- lines_deleted: u32
|   |   +-- ai_lines: Vec<LineRange>
|   |   +-- human_lines: Vec<LineRange>
|   |   +-- mixed_lines: Vec<LineRange>
|   +-- summary: AttributionSummary
|       +-- total_lines_added: u32
|       +-- total_lines_deleted: u32
|       +-- ai_percentage: f32
|       +-- human_percentage: f32
|
+-- agent_trace: Option<AgentTraceRecord>  (Agent Trace spec v0.1.0 compat)
+-- signature: Option<String>              (cryptographic, enterprise)
```

### Policy Rule

```
PolicyRule
+-- id: UUID
+-- org_id: String
+-- name: String
+-- description: String
+-- condition: PolicyCondition
|   +-- type: Enum (file_pattern, ai_percentage, model_allowlist,
|   |              trace_completeness, required_tool_call,
|   |              token_budget, custom_expr)
|   +-- params: JSON
+-- action: Enum (block_merge, warn, require_review, notify)
+-- severity: Enum (critical, high, medium, low)
+-- enabled: bool
```

### Token Usage Aggregation Levels

| Level | What you see |
|-------|-------------|
| Per commit | Tokens used to produce this commit |
| Per PR | Sum across all commits in the PR |
| Per person | Sum across all traces by git author |
| Per repo | Sum across all traces in the repo |
| Per team | Sum across repos (needs team mapping) |
| Per model | Breakdown by model (Opus vs Sonnet vs Haiku) |

### Database Schema (PostgreSQL)

```sql
CREATE TABLE orgs (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    plan TEXT NOT NULL DEFAULT 'free'
);

CREATE TABLE repos (
    id UUID PRIMARY KEY,
    org_id UUID REFERENCES orgs(id),
    name TEXT NOT NULL,
    github_url TEXT
);

CREATE TABLE traces (
    id UUID PRIMARY KEY,
    repo_id UUID REFERENCES repos(id),
    commit_sha TEXT NOT NULL,
    branch TEXT,
    author TEXT NOT NULL,
    model TEXT,
    tool TEXT,
    tool_version TEXT,
    ai_percentage REAL,
    total_tokens BIGINT,
    input_tokens BIGINT,
    output_tokens BIGINT,
    estimated_cost_usd DOUBLE PRECISION,
    api_calls INTEGER,
    session_data JSONB,
    attribution JSONB,
    agent_trace JSONB,
    signature TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE policies (
    id UUID PRIMARY KEY,
    org_id UUID REFERENCES orgs(id),
    name TEXT NOT NULL,
    description TEXT,
    condition JSONB NOT NULL,
    action TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'medium',
    enabled BOOLEAN NOT NULL DEFAULT true
);

CREATE TABLE evaluations (
    id UUID PRIMARY KEY,
    trace_id UUID REFERENCES traces(id),
    policy_id UUID REFERENCES policies(id),
    result TEXT NOT NULL,
    details JSONB,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## 5. CLI Design

### Commands

```
tracevault init              # Initialize repo, install Claude Code hooks
tracevault status            # Show active session, pending traces
tracevault validate          # Run policies locally against pending traces
tracevault push              # Manually push traces to server
tracevault blame <file>      # Per-line AI/human attribution
tracevault stats             # Local stats: AI %, models, token usage, cost
tracevault report <commit>   # Generate compliance report for commit/range
tracevault config            # Manage server URL, API key, agent settings
tracevault login             # Authenticate with TraceVault server
```

### Hook Architecture (Claude Code)

Claude Code exposes `PreToolUse` and `PostToolUse` lifecycle hooks.

```
Claude Code session starts
  |
  +-- PreToolUse(Write/Edit) -> CLI captures "about to modify file X"
  |    +-- Records pre-edit file content hash
  |
  +-- PostToolUse(Write/Edit) -> CLI captures "file X modified"
  |    +-- Records post-edit diff, associates with session
  |
  +-- PostToolUse(Bash) -> CLI captures tool usage
  |    +-- Records command summary (redacted)
  |
  +-- On git commit (post-commit hook):
       +-- Read accumulated session data from .tracevault/sessions/
       +-- Read transcript from Claude Code session storage
       |   (~/.claude/projects/<project>/sessions/)
       +-- Read token usage from session data
       +-- Compute line-level attribution from file diffs
       +-- Redact secrets (entropy + pattern detection)
       +-- Create TraceRecord
       +-- Store in git notes (refs/notes/tracevault)
       +-- Push to TraceVault server API
```

### Local Storage

```
.tracevault/
+-- config.toml              # Repo-level config
+-- sessions/
|   +-- <session-id>/
|       +-- events.jsonl     # Hook events (pre/post tool use)
|       +-- state.json       # Pre-prompt file states
|       +-- metadata.json    # Session metadata (model, tool, tokens)
+-- cache/
    +-- policies.json        # Cached policies from server
```

### Secret Redaction

Applied before any data leaves the developer machine:
- **Entropy detection:** Shannon entropy > 4.5 on alphanumeric strings >= 10 chars
- **Pattern detection:** Known formats (AWS keys, GitHub tokens, JWTs, RSA keys, etc.)
- **Configurable allowlist:** Teams can mark patterns as safe
- **Replacement:** `[REDACTED]`

## 6. Server Design

### API Endpoints

```
# Auth
POST   /api/v1/auth/login           # API key -> JWT
POST   /api/v1/auth/register        # Register org

# Traces
POST   /api/v1/traces               # Ingest trace (from CLI)
GET    /api/v1/traces/:id           # Get single trace
GET    /api/v1/traces?repo=&sha=    # Query traces
GET    /api/v1/traces/:id/report    # Compliance report

# Repos
POST   /api/v1/repos                # Register repo
GET    /api/v1/repos                # List repos
GET    /api/v1/repos/:id/stats      # Repo stats

# Policies
POST   /api/v1/policies             # Create policy
GET    /api/v1/policies             # List policies
PUT    /api/v1/policies/:id         # Update policy
POST   /api/v1/policies/evaluate    # Evaluate trace

# GitHub
POST   /api/v1/github/webhook       # GitHub webhook receiver
GET    /api/v1/github/check/:pr     # Check result for PR

# Analytics
GET    /api/v1/analytics/overview   # Org-wide stats
GET    /api/v1/analytics/tokens     # Token usage
       ?group_by=author|repo|model
       &period=7d|30d|90d
```

### Server Crate Structure

```
tracevault-server/
+-- src/
    +-- main.rs                 # Entry point, axum router
    +-- api/                    # HTTP handlers
    |   +-- auth.rs
    |   +-- traces.rs
    |   +-- policies.rs
    |   +-- github.rs
    |   +-- analytics.rs
    +-- domain/                 # Core business logic
    |   +-- trace.rs            # TraceRecord, validation
    |   +-- policy.rs           # PolicyRule, evaluation engine
    |   +-- attribution.rs      # Line-level attribution logic
    |   +-- report.rs           # Report generation (JSON/SARIF)
    |   +-- tokens.rs           # Token usage aggregation, cost calc
    +-- infra/                  # External integrations
    |   +-- db.rs               # PostgreSQL (sqlx)
    |   +-- github.rs           # GitHub API client
    |   +-- storage.rs          # Trace blob storage (S3/local)
    +-- config.rs               # Server configuration
```

## 7. GitHub Action

### Usage

```yaml
name: TraceVault Check
on: [pull_request]

jobs:
  tracevault:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: tracevault/action@v1
        with:
          server-url: ${{ secrets.TRACEVAULT_URL }}
          api-key: ${{ secrets.TRACEVAULT_API_KEY }}
```

### PR Comment Format

```markdown
## TraceVault Report

| Metric | Value |
|--------|-------|
| AI code | 67% of changed lines |
| Model | claude-opus-4-6 |
| Tokens | 142,350 (in: 98,200 / out: 44,150) |
| Estimated cost | $4.82 |
| API calls | 12 |

### Policy Results
- [pass] Every AI commit must have a trace
- [pass] AI percentage threshold (< 90%)
- [pass] Model allowlist
- [pass] Required trace on sensitive paths
- [pass] Trace completeness
- [pass] Token budget per PR (< $50)
```

## 8. Built-in Policies (MVP)

| # | Policy | Condition | Default Action |
|---|--------|-----------|----------------|
| 1 | Every AI commit must have a trace | `trace_completeness > 0` | block_merge |
| 2 | AI percentage threshold | `ai_percentage > threshold` (default 90%) | warn |
| 3 | Model allowlist | `model NOT IN allowlist` | block_merge |
| 4 | Required trace on sensitive paths | `file_pattern(pattern) AND ai_percentage > 0` | require_review |
| 5 | Trace completeness | `trace must include session + attribution + model` | warn |
| 6 | Token budget per PR | `total_tokens > budget OR estimated_cost > cost_limit` | warn |

## 9. MVP Scope

### In Scope

- TraceVault CLI (Rust binary)
- TraceVault Server (Rust binary, axum + PostgreSQL)
- GitHub Action (reusable workflow)
- Claude Code integration (PreToolUse/PostToolUse hooks)
- 6 built-in policy rules
- Token usage tracking and cost estimation
- Per-line AI/human attribution
- Secret redaction
- Git notes as offline backup
- Docker Compose for self-hosted deployment
- API-based analytics (no web dashboard)

### Not in MVP

- Other AI tools (Cursor, Copilot) — Claude Code only
- GitHub App (org-level) — Action only
- Web dashboard — API only
- Jira integration
- Cryptographic signing of traces
- Custom policy expressions
- SAML/SSO, RBAC
- Rebase/cherry-pick attribution preservation
- Agent Trace spec output (future compatibility layer)

## 10. Tech Stack

| Component | Technology |
|-----------|-----------|
| CLI | Rust |
| Server | Rust (axum) |
| Database | PostgreSQL |
| ORM/Query | sqlx |
| Serialization | serde + serde_json |
| Git operations | git2 (libgit2 bindings) |
| GitHub Action | Composite action calling CLI binary |
| Deployment | Docker Compose (server + PostgreSQL) |
| CI | GitHub Actions |
| Secret detection | Custom entropy + regex patterns |

## 11. Agent Trace Spec Compatibility (Future)

TraceVault's internal format is richer than Agent Trace v0.1.0 (it includes sessions, token usage, policies). Future versions will:

- Accept Agent Trace records as input (from any compliant tool)
- Export traces in Agent Trace format
- Store the original Agent Trace JSON alongside the enriched TraceVault record

This positions TraceVault as the governance layer on top of the Agent Trace ecosystem.
