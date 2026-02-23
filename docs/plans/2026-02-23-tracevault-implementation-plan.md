# TraceVault MVP Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build TraceVault MVP — a Rust-based AI code governance platform with CLI, server, and GitHub Action.

**Architecture:** Rust workspace with 3 crates: `tracevault-core` (shared domain types, attribution engine, redaction, policy engine), `tracevault-cli` (CLI binary with hook handlers and git integration), `tracevault-server` (axum HTTP server with PostgreSQL). GitHub Action is a composite action wrapping the CLI binary.

**Tech Stack:** Rust, axum, sqlx, PostgreSQL, serde, git2, clap, regex, tokio

**Design doc:** `docs/plans/2026-02-23-tracevault-architecture-design.md`

---

## Phase 1: Project Scaffolding

### Task 1: Initialize Rust workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/tracevault-core/Cargo.toml`
- Create: `crates/tracevault-core/src/lib.rs`
- Create: `crates/tracevault-cli/Cargo.toml`
- Create: `crates/tracevault-cli/src/main.rs`
- Create: `crates/tracevault-server/Cargo.toml`
- Create: `crates/tracevault-server/src/main.rs`
- Create: `.gitignore`

**Step 1: Create workspace Cargo.toml**

```toml
[workspace]
members = ["crates/tracevault-core", "crates/tracevault-cli", "crates/tracevault-server"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Apache-2.0"
repository = "https://github.com/softwaremill/tracevault"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

**Step 2: Create tracevault-core crate**

`crates/tracevault-core/Cargo.toml`:
```toml
[package]
name = "tracevault-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
uuid.workspace = true
chrono.workspace = true
thiserror.workspace = true
```

`crates/tracevault-core/src/lib.rs`:
```rust
pub mod trace;
```

Create `crates/tracevault-core/src/trace.rs`:
```rust
// Domain types - will be populated in Phase 2
```

**Step 3: Create tracevault-cli crate**

`crates/tracevault-cli/Cargo.toml`:
```toml
[package]
name = "tracevault-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "tracevault"
path = "src/main.rs"

[dependencies]
tracevault-core = { path = "../tracevault-core" }
clap = { version = "4", features = ["derive"] }
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

`crates/tracevault-cli/src/main.rs`:
```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "tracevault", about = "AI code governance platform")]
enum Cli {
    /// Initialize TraceVault in current repository
    Init,
    /// Show current session status
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Init => println!("tracevault init - not yet implemented"),
        Cli::Status => println!("tracevault status - not yet implemented"),
    }
}
```

**Step 4: Create tracevault-server crate**

`crates/tracevault-server/Cargo.toml`:
```toml
[package]
name = "tracevault-server"
version.workspace = true
edition.workspace = true

[dependencies]
tracevault-core = { path = "../tracevault-core" }
axum = "0.8"
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

`crates/tracevault-server/src/main.rs`:
```rust
use axum::{routing::get, Router};

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new().route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("TraceVault server listening on :3000");
    axum::serve(listener, app).await.unwrap();
}
```

**Step 5: Create .gitignore**

```
/target
.env
*.swp
.DS_Store
```

**Step 6: Build and verify**

Run: `cargo build`
Expected: Compiles all 3 crates without errors.

Run: `cargo run -p tracevault-cli -- --help`
Expected: Shows help text with `init` and `status` subcommands.

**Step 7: Initialize git and commit**

```bash
git init
git add Cargo.toml Cargo.lock crates/ .gitignore
git commit -m "feat: initialize rust workspace with core, cli, and server crates"
```

---

## Phase 2: Core Domain Types

### Task 2: Define TraceRecord and related types

**Files:**
- Create: `crates/tracevault-core/src/trace.rs`
- Create: `crates/tracevault-core/src/attribution.rs`
- Create: `crates/tracevault-core/src/token_usage.rs`
- Create: `crates/tracevault-core/src/policy.rs`
- Modify: `crates/tracevault-core/src/lib.rs`
- Test: `crates/tracevault-core/tests/trace_test.rs`

**Step 1: Write test for TraceRecord serialization**

`crates/tracevault-core/tests/trace_test.rs`:
```rust
use tracevault_core::trace::*;
use chrono::Utc;
use uuid::Uuid;

#[test]
fn trace_record_serializes_to_json() {
    let record = TraceRecord {
        id: Uuid::nil(),
        repo_id: "softwaremill/tracevault".to_string(),
        commit_sha: "a".repeat(40),
        branch: Some("main".to_string()),
        author: "dev@example.com".to_string(),
        created_at: Utc::now(),
        model: Some("anthropic/claude-opus-4-6".to_string()),
        tool: "claude-code".to_string(),
        tool_version: Some("1.0.0".to_string()),
        session: Session {
            session_id: "test-session".to_string(),
            started_at: Utc::now(),
            ended_at: None,
            prompts: vec![],
            responses: vec![],
            token_usage: TokenUsage::default(),
            tools_used: vec![],
        },
        attribution: Attribution {
            files: vec![],
            summary: AttributionSummary {
                total_lines_added: 0,
                total_lines_deleted: 0,
                ai_percentage: 0.0,
                human_percentage: 100.0,
            },
        },
        agent_trace: None,
        signature: None,
    };

    let json = serde_json::to_string(&record).unwrap();
    let deserialized: TraceRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.repo_id, "softwaremill/tracevault");
    assert_eq!(deserialized.tool, "claude-code");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p tracevault-core`
Expected: FAIL — modules don't exist yet.

**Step 3: Implement core types**

`crates/tracevault-core/src/lib.rs`:
```rust
pub mod attribution;
pub mod policy;
pub mod token_usage;
pub mod trace;
```

`crates/tracevault-core/src/token_usage.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub api_calls: u32,
    #[serde(default)]
    pub subagent_usage: Vec<TokenUsage>,
}
```

`crates/tracevault-core/src/attribution.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    pub files: Vec<FileAttribution>,
    pub summary: AttributionSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttribution {
    pub path: String,
    pub lines_added: u32,
    pub lines_deleted: u32,
    pub ai_lines: Vec<LineRange>,
    pub human_lines: Vec<LineRange>,
    pub mixed_lines: Vec<LineRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionSummary {
    pub total_lines_added: u32,
    pub total_lines_deleted: u32,
    pub ai_percentage: f32,
    pub human_percentage: f32,
}
```

`crates/tracevault-core/src/trace.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::attribution::{Attribution, AttributionSummary};
use crate::token_usage::TokenUsage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRecord {
    pub id: Uuid,
    pub repo_id: String,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub model: Option<String>,
    pub tool: String,
    pub tool_version: Option<String>,
    pub session: Session,
    pub attribution: Attribution,
    pub agent_trace: Option<serde_json::Value>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub prompts: Vec<Prompt>,
    pub responses: Vec<Response>,
    pub token_usage: TokenUsage,
    pub tools_used: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub text: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub input_summary: String,
    pub timestamp: DateTime<Utc>,
}

impl TraceRecord {
    pub fn new(
        repo_id: String,
        commit_sha: String,
        author: String,
        tool: String,
        session: Session,
        attribution: Attribution,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            repo_id,
            commit_sha,
            branch: None,
            author,
            created_at: Utc::now(),
            model: None,
            tool,
            tool_version: None,
            session,
            attribution,
            agent_trace: None,
            signature: None,
        }
    }
}

impl Default for Attribution {
    fn default() -> Self {
        Self {
            files: vec![],
            summary: AttributionSummary {
                total_lines_added: 0,
                total_lines_deleted: 0,
                ai_percentage: 0.0,
                human_percentage: 100.0,
            },
        }
    }
}
```

`crates/tracevault-core/src/policy.rs`:
```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: Uuid,
    pub org_id: Option<String>,
    pub name: String,
    pub description: String,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
    pub severity: PolicySeverity,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PolicyCondition {
    TraceCompleteness,
    AiPercentageThreshold { threshold: f32 },
    ModelAllowlist { allowed_models: Vec<String> },
    SensitivePathPattern { patterns: Vec<String> },
    RequiredToolCall { tool_names: Vec<String> },
    TokenBudget { max_tokens: Option<u64>, max_cost_usd: Option<f64> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyAction {
    BlockMerge,
    Warn,
    RequireReview,
    Notify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PolicySeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    pub policy: PolicyRule,
    pub result: EvalResult,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EvalResult {
    Pass,
    Fail,
    Warn,
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p tracevault-core`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-core/
git commit -m "feat: define core domain types - TraceRecord, Attribution, TokenUsage, Policy"
```

---

## Phase 3: Secret Redaction Engine

### Task 3: Build entropy-based and pattern-based secret redactor

**Files:**
- Create: `crates/tracevault-core/src/redact.rs`
- Test: `crates/tracevault-core/tests/redact_test.rs`
- Modify: `crates/tracevault-core/src/lib.rs` (add `pub mod redact;`)
- Modify: `crates/tracevault-core/Cargo.toml` (add `regex` dependency)

**Step 1: Write failing tests**

`crates/tracevault-core/tests/redact_test.rs`:
```rust
use tracevault_core::redact::Redactor;

#[test]
fn redacts_high_entropy_strings() {
    let r = Redactor::new();
    let input = "token = \"aK3bF9xZ2mQ7nR4pL8wS5vY1cD6eH0j\"";
    let output = r.redact_string(input);
    assert!(output.contains("[REDACTED]"));
    assert!(!output.contains("aK3bF9xZ2mQ7nR4pL8wS5vY1cD6eH0j"));
}

#[test]
fn redacts_aws_access_key() {
    let r = Redactor::new();
    let input = "aws_key = \"AKIAIOSFODNN7EXAMPLE\"";
    let output = r.redact_string(input);
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn redacts_github_token() {
    let r = Redactor::new();
    let input = "token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdef12";
    let output = r.redact_string(input);
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn preserves_normal_text() {
    let r = Redactor::new();
    let input = "This is a normal code comment with variable_name and some_function()";
    let output = r.redact_string(input);
    assert_eq!(input, output);
}

#[test]
fn preserves_short_alphanumeric() {
    let r = Redactor::new();
    let input = "id = \"abc123\"";
    let output = r.redact_string(input);
    assert_eq!(input, output);
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p tracevault-core`
Expected: FAIL — `redact` module not found.

**Step 3: Implement redactor**

Add `regex = "1"` to `crates/tracevault-core/Cargo.toml` dependencies.

Add `pub mod redact;` to `crates/tracevault-core/src/lib.rs`.

`crates/tracevault-core/src/redact.rs`:
```rust
use regex::Regex;

pub struct Redactor {
    patterns: Vec<Regex>,
    high_entropy_pattern: Regex,
}

const REDACTED: &str = "[REDACTED]";

impl Redactor {
    pub fn new() -> Self {
        let patterns = vec![
            // AWS Access Key
            r"AKIA[0-9A-Z]{16}",
            // GitHub token
            r"gh[ps]_[A-Za-z0-9]{36,}",
            // Generic API key patterns
            r"(?i)(api[_-]?key|apikey|secret[_-]?key)\s*[:=]\s*[\"']?[A-Za-z0-9/+=]{20,}",
            // JWT
            r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+",
            // RSA private key header
            r"-----BEGIN (?:RSA )?PRIVATE KEY-----",
            // Slack token
            r"xox[bpras]-[0-9A-Za-z\-]+",
            // Generic bearer token
            r"(?i)bearer\s+[A-Za-z0-9\-._~+/]+=*",
        ];

        Self {
            patterns: patterns
                .iter()
                .map(|p| Regex::new(p).unwrap())
                .collect(),
            high_entropy_pattern: Regex::new(r"[A-Za-z0-9/+_=\-]{16,}").unwrap(),
        }
    }

    pub fn redact_string(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Pattern-based redaction first
        for pattern in &self.patterns {
            result = pattern.replace_all(&result, REDACTED).to_string();
        }

        // Entropy-based redaction
        let entropy_re = &self.high_entropy_pattern;
        result = entropy_re
            .replace_all(&result, |caps: &regex::Captures| {
                let matched = caps.get(0).unwrap().as_str();
                if shannon_entropy(matched) > 4.5 {
                    REDACTED.to_string()
                } else {
                    matched.to_string()
                }
            })
            .to_string();

        result
    }
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut freq = [0u32; 256];
    for b in s.bytes() {
        freq[b as usize] += 1;
    }
    let len = s.len() as f64;
    freq.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}
```

**Step 4: Run tests**

Run: `cargo test -p tracevault-core`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-core/
git commit -m "feat: add secret redaction engine with entropy and pattern detection"
```

---

## Phase 4: Policy Evaluation Engine

### Task 4: Implement the 6 built-in policy evaluators

**Files:**
- Create: `crates/tracevault-core/src/policy_engine.rs`
- Test: `crates/tracevault-core/tests/policy_engine_test.rs`
- Modify: `crates/tracevault-core/src/lib.rs` (add `pub mod policy_engine;`)

**Step 1: Write failing tests**

`crates/tracevault-core/tests/policy_engine_test.rs`:
```rust
use tracevault_core::policy::*;
use tracevault_core::policy_engine::PolicyEngine;
use tracevault_core::trace::*;
use tracevault_core::attribution::*;
use tracevault_core::token_usage::TokenUsage;
use chrono::Utc;
use uuid::Uuid;

fn make_trace(ai_pct: f32, model: &str, tokens: u64, cost: f64, files: Vec<&str>) -> TraceRecord {
    TraceRecord {
        id: Uuid::new_v4(),
        repo_id: "org/repo".into(),
        commit_sha: "a".repeat(40),
        branch: Some("main".into()),
        author: "dev@test.com".into(),
        created_at: Utc::now(),
        model: Some(model.into()),
        tool: "claude-code".into(),
        tool_version: Some("1.0".into()),
        session: Session {
            session_id: "s1".into(),
            started_at: Utc::now(),
            ended_at: None,
            prompts: vec![Prompt { text: "do something".into(), timestamp: Utc::now() }],
            responses: vec![],
            token_usage: TokenUsage {
                model: Some(model.into()),
                total_tokens: tokens,
                estimated_cost_usd: cost,
                ..Default::default()
            },
            tools_used: vec![],
        },
        attribution: Attribution {
            files: files.iter().map(|p| FileAttribution {
                path: p.to_string(),
                lines_added: 10,
                lines_deleted: 0,
                ai_lines: vec![LineRange { start: 1, end: 10 }],
                human_lines: vec![],
                mixed_lines: vec![],
            }).collect(),
            summary: AttributionSummary {
                total_lines_added: 10,
                total_lines_deleted: 0,
                ai_percentage: ai_pct,
                human_percentage: 100.0 - ai_pct,
            },
        },
        agent_trace: None,
        signature: None,
    }
}

#[test]
fn trace_completeness_passes_for_complete_trace() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(50.0, "anthropic/claude-opus-4-6", 1000, 0.5, vec!["src/main.rs"]);
    let results = engine.evaluate(&trace);
    let completeness = results.iter().find(|r| r.policy.name == "Trace completeness").unwrap();
    assert_eq!(completeness.result, EvalResult::Pass);
}

#[test]
fn ai_percentage_warns_above_threshold() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(95.0, "anthropic/claude-opus-4-6", 1000, 0.5, vec!["src/main.rs"]);
    let results = engine.evaluate(&trace);
    let pct = results.iter().find(|r| r.policy.name == "AI percentage threshold").unwrap();
    assert_eq!(pct.result, EvalResult::Warn);
}

#[test]
fn model_allowlist_fails_for_unknown_model() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(50.0, "unknown/bad-model", 1000, 0.5, vec!["src/main.rs"]);
    let results = engine.evaluate(&trace);
    let model = results.iter().find(|r| r.policy.name == "Model allowlist").unwrap();
    assert_eq!(model.result, EvalResult::Fail);
}

#[test]
fn sensitive_path_flags_payments() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(50.0, "anthropic/claude-opus-4-6", 1000, 0.5, vec!["src/payments/charge.rs"]);
    let results = engine.evaluate(&trace);
    let sensitive = results.iter().find(|r| r.policy.name == "Sensitive path review").unwrap();
    assert_eq!(sensitive.result, EvalResult::Warn);
}

#[test]
fn token_budget_warns_over_limit() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(50.0, "anthropic/claude-opus-4-6", 600_000, 55.0, vec!["src/main.rs"]);
    let results = engine.evaluate(&trace);
    let budget = results.iter().find(|r| r.policy.name == "Token budget").unwrap();
    assert_eq!(budget.result, EvalResult::Warn);
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p tracevault-core`
Expected: FAIL

**Step 3: Implement policy engine**

Add `pub mod policy_engine;` to `crates/tracevault-core/src/lib.rs`.

`crates/tracevault-core/src/policy_engine.rs`:
```rust
use crate::policy::*;
use crate::trace::TraceRecord;
use uuid::Uuid;

pub struct PolicyEngine {
    policies: Vec<PolicyRule>,
}

impl PolicyEngine {
    pub fn new(policies: Vec<PolicyRule>) -> Self {
        Self { policies }
    }

    pub fn with_defaults() -> Self {
        Self::new(default_policies())
    }

    pub fn evaluate(&self, trace: &TraceRecord) -> Vec<PolicyEvaluation> {
        self.policies
            .iter()
            .filter(|p| p.enabled)
            .map(|p| evaluate_policy(p, trace))
            .collect()
    }
}

fn evaluate_policy(policy: &PolicyRule, trace: &TraceRecord) -> PolicyEvaluation {
    let (result, details) = match &policy.condition {
        PolicyCondition::TraceCompleteness => eval_trace_completeness(trace),
        PolicyCondition::AiPercentageThreshold { threshold } => {
            eval_ai_percentage(trace, *threshold)
        }
        PolicyCondition::ModelAllowlist { allowed_models } => {
            eval_model_allowlist(trace, allowed_models)
        }
        PolicyCondition::SensitivePathPattern { patterns } => {
            eval_sensitive_paths(trace, patterns)
        }
        PolicyCondition::RequiredToolCall { tool_names } => {
            eval_required_tool_call(trace, tool_names)
        }
        PolicyCondition::TokenBudget {
            max_tokens,
            max_cost_usd,
        } => eval_token_budget(trace, *max_tokens, *max_cost_usd),
    };

    PolicyEvaluation {
        policy: policy.clone(),
        result,
        details,
    }
}

fn eval_trace_completeness(trace: &TraceRecord) -> (EvalResult, String) {
    let has_session = !trace.session.session_id.is_empty();
    let has_model = trace.model.is_some();
    let has_attribution = !trace.attribution.files.is_empty()
        || trace.attribution.summary.total_lines_added == 0;

    if has_session && has_model && has_attribution {
        (EvalResult::Pass, "Trace is complete".into())
    } else {
        let mut missing = vec![];
        if !has_session { missing.push("session"); }
        if !has_model { missing.push("model"); }
        if !has_attribution { missing.push("attribution"); }
        (EvalResult::Warn, format!("Missing: {}", missing.join(", ")))
    }
}

fn eval_ai_percentage(trace: &TraceRecord, threshold: f32) -> (EvalResult, String) {
    let pct = trace.attribution.summary.ai_percentage;
    if pct > threshold {
        (
            EvalResult::Warn,
            format!("AI percentage {pct:.1}% exceeds threshold {threshold:.1}%"),
        )
    } else {
        (EvalResult::Pass, format!("AI percentage {pct:.1}% within threshold"))
    }
}

fn eval_model_allowlist(trace: &TraceRecord, allowed: &[String]) -> (EvalResult, String) {
    if allowed.is_empty() {
        return (EvalResult::Pass, "No allowlist configured".into());
    }
    match &trace.model {
        Some(model) if allowed.iter().any(|a| model.contains(a)) => {
            (EvalResult::Pass, format!("Model {model} is allowed"))
        }
        Some(model) => (
            EvalResult::Fail,
            format!("Model {model} is not in allowlist: {}", allowed.join(", ")),
        ),
        None => (EvalResult::Fail, "No model specified in trace".into()),
    }
}

fn eval_sensitive_paths(trace: &TraceRecord, patterns: &[String]) -> (EvalResult, String) {
    let matched: Vec<_> = trace
        .attribution
        .files
        .iter()
        .filter(|f| {
            patterns.iter().any(|p| f.path.contains(p))
                && !f.ai_lines.is_empty()
        })
        .map(|f| f.path.clone())
        .collect();

    if matched.is_empty() {
        (EvalResult::Pass, "No sensitive paths with AI code".into())
    } else {
        (
            EvalResult::Warn,
            format!("AI code in sensitive paths: {}", matched.join(", ")),
        )
    }
}

fn eval_required_tool_call(trace: &TraceRecord, required: &[String]) -> (EvalResult, String) {
    let used: Vec<_> = trace.session.tools_used.iter().map(|t| &t.name).collect();
    let missing: Vec<_> = required.iter().filter(|r| !used.iter().any(|u| u.contains(r.as_str()))).collect();

    if missing.is_empty() {
        (EvalResult::Pass, "All required tools were used".into())
    } else {
        (
            EvalResult::Warn,
            format!("Missing required tools: {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")),
        )
    }
}

fn eval_token_budget(
    trace: &TraceRecord,
    max_tokens: Option<u64>,
    max_cost: Option<f64>,
) -> (EvalResult, String) {
    let tokens = trace.session.token_usage.total_tokens;
    let cost = trace.session.token_usage.estimated_cost_usd;

    let token_over = max_tokens.map_or(false, |max| tokens > max);
    let cost_over = max_cost.map_or(false, |max| cost > max);

    if token_over || cost_over {
        (
            EvalResult::Warn,
            format!("Budget exceeded: {tokens} tokens (${cost:.2})"),
        )
    } else {
        (
            EvalResult::Pass,
            format!("Within budget: {tokens} tokens (${cost:.2})"),
        )
    }
}

fn default_policies() -> Vec<PolicyRule> {
    vec![
        PolicyRule {
            id: Uuid::nil(),
            org_id: None,
            name: "Trace completeness".into(),
            description: "Every AI commit must have complete trace data".into(),
            condition: PolicyCondition::TraceCompleteness,
            action: PolicyAction::Warn,
            severity: PolicySeverity::Medium,
            enabled: true,
        },
        PolicyRule {
            id: Uuid::nil(),
            org_id: None,
            name: "AI percentage threshold".into(),
            description: "Warn when AI-generated code exceeds 90%".into(),
            condition: PolicyCondition::AiPercentageThreshold { threshold: 90.0 },
            action: PolicyAction::Warn,
            severity: PolicySeverity::Medium,
            enabled: true,
        },
        PolicyRule {
            id: Uuid::nil(),
            org_id: None,
            name: "Model allowlist".into(),
            description: "Only approved models may generate code".into(),
            condition: PolicyCondition::ModelAllowlist {
                allowed_models: vec![
                    "anthropic/claude".into(),
                    "openai/gpt".into(),
                    "google/gemini".into(),
                ],
            },
            action: PolicyAction::BlockMerge,
            severity: PolicySeverity::High,
            enabled: true,
        },
        PolicyRule {
            id: Uuid::nil(),
            org_id: None,
            name: "Sensitive path review".into(),
            description: "AI code in sensitive paths requires review".into(),
            condition: PolicyCondition::SensitivePathPattern {
                patterns: vec![
                    "payments".into(),
                    "auth".into(),
                    "security".into(),
                    "crypto".into(),
                ],
            },
            action: PolicyAction::RequireReview,
            severity: PolicySeverity::High,
            enabled: true,
        },
        PolicyRule {
            id: Uuid::nil(),
            org_id: None,
            name: "Required tool call".into(),
            description: "Trace must show that tests were run".into(),
            condition: PolicyCondition::RequiredToolCall {
                tool_names: vec![],
            },
            action: PolicyAction::Warn,
            severity: PolicySeverity::Low,
            enabled: true,
        },
        PolicyRule {
            id: Uuid::nil(),
            org_id: None,
            name: "Token budget".into(),
            description: "Warn when token usage exceeds budget".into(),
            condition: PolicyCondition::TokenBudget {
                max_tokens: Some(500_000),
                max_cost_usd: Some(50.0),
            },
            action: PolicyAction::Warn,
            severity: PolicySeverity::Medium,
            enabled: true,
        },
    ]
}
```

**Step 4: Run tests**

Run: `cargo test -p tracevault-core`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-core/
git commit -m "feat: implement policy evaluation engine with 6 built-in policies"
```

---

## Phase 5: Claude Code Hook Handler

### Task 5: Parse Claude Code hook events and capture session data

**Files:**
- Create: `crates/tracevault-core/src/hooks.rs`
- Create: `crates/tracevault-core/src/session.rs`
- Test: `crates/tracevault-core/tests/hooks_test.rs`
- Modify: `crates/tracevault-core/src/lib.rs`

**Step 1: Write failing tests**

`crates/tracevault-core/tests/hooks_test.rs`:
```rust
use tracevault_core::hooks::{HookEvent, parse_hook_event};

#[test]
fn parses_pre_tool_use_write() {
    let json = r#"{
        "session_id": "abc123",
        "transcript_path": "/home/user/.claude/projects/test/abc123.jsonl",
        "cwd": "/home/user/project",
        "hook_event_name": "PreToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": "/home/user/project/src/main.rs",
            "content": "fn main() {}"
        }
    }"#;

    let event = parse_hook_event(json).unwrap();
    assert_eq!(event.session_id, "abc123");
    assert_eq!(event.hook_event_name, "PreToolUse");
    assert_eq!(event.tool_name.as_deref(), Some("Write"));
}

#[test]
fn parses_post_tool_use_edit() {
    let json = r#"{
        "session_id": "abc123",
        "transcript_path": "/home/user/.claude/projects/test/abc123.jsonl",
        "cwd": "/home/user/project",
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "tool_input": {
            "file_path": "/home/user/project/src/lib.rs",
            "old_string": "old",
            "new_string": "new"
        },
        "tool_response": {
            "success": true
        }
    }"#;

    let event = parse_hook_event(json).unwrap();
    assert_eq!(event.hook_event_name, "PostToolUse");
    assert_eq!(event.tool_name.as_deref(), Some("Edit"));
    assert!(event.tool_input.is_some());
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p tracevault-core`
Expected: FAIL

**Step 3: Implement hook parser**

Add to `crates/tracevault-core/src/lib.rs`:
```rust
pub mod hooks;
pub mod session;
```

`crates/tracevault-core/src/hooks.rs`:
```rust
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    #[serde(default)]
    pub permission_mode: Option<String>,
    pub hook_event_name: String,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_input: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_response: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#continue: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
}

impl HookResponse {
    pub fn allow() -> Self {
        Self {
            r#continue: None,
            suppress_output: Some(true),
        }
    }
}

#[derive(Debug, Error)]
pub enum HookError {
    #[error("Failed to parse hook event: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub fn parse_hook_event(json: &str) -> Result<HookEvent, HookError> {
    Ok(serde_json::from_str(json)?)
}

impl HookEvent {
    /// Extract file path from tool_input if this is a Write or Edit event
    pub fn file_path(&self) -> Option<String> {
        self.tool_input.as_ref()?.get("file_path")?.as_str().map(String::from)
    }

    pub fn is_file_modification(&self) -> bool {
        matches!(
            self.tool_name.as_deref(),
            Some("Write") | Some("Edit")
        )
    }
}
```

`crates/tracevault-core/src/session.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::hooks::HookEvent;

/// Tracks file states during an active AI session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    pub started_at: DateTime<Utc>,
    pub events: Vec<SessionEvent>,
    /// file path -> content hash before AI modification
    pub pre_edit_hashes: HashMap<String, String>,
    /// files modified by AI during this session
    pub ai_modified_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub file_path: Option<String>,
    pub details: Option<serde_json::Value>,
}

impl SessionState {
    pub fn new(event: &HookEvent) -> Self {
        Self {
            session_id: event.session_id.clone(),
            transcript_path: event.transcript_path.clone(),
            cwd: event.cwd.clone(),
            started_at: Utc::now(),
            events: vec![],
            pre_edit_hashes: HashMap::new(),
            ai_modified_files: vec![],
        }
    }

    pub fn record_event(&mut self, event: &HookEvent) {
        self.events.push(SessionEvent {
            timestamp: Utc::now(),
            event_type: event.hook_event_name.clone(),
            tool_name: event.tool_name.clone(),
            file_path: event.file_path(),
            details: event.tool_input.clone(),
        });

        // Track AI-modified files
        if event.hook_event_name == "PostToolUse" && event.is_file_modification() {
            if let Some(path) = event.file_path() {
                if !self.ai_modified_files.contains(&path) {
                    self.ai_modified_files.push(path);
                }
            }
        }
    }

    pub fn record_pre_edit_hash(&mut self, file_path: &str, hash: &str) {
        self.pre_edit_hashes
            .entry(file_path.to_string())
            .or_insert_with(|| hash.to_string());
    }

    /// Path to session data directory: .tracevault/sessions/<session_id>/
    pub fn session_dir(&self) -> PathBuf {
        PathBuf::from(&self.cwd)
            .join(".tracevault")
            .join("sessions")
            .join(&self.session_id)
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p tracevault-core`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-core/
git commit -m "feat: add Claude Code hook event parser and session state tracking"
```

---

## Phase 6: Attribution Engine

### Task 6: Compute per-line AI/human attribution from diffs

**Files:**
- Create: `crates/tracevault-core/src/attribution_engine.rs`
- Test: `crates/tracevault-core/tests/attribution_engine_test.rs`
- Modify: `crates/tracevault-core/src/lib.rs`

**Step 1: Write failing tests**

`crates/tracevault-core/tests/attribution_engine_test.rs`:
```rust
use tracevault_core::attribution_engine::*;

#[test]
fn all_lines_ai_when_file_created_by_agent() {
    let result = compute_file_attribution(
        "src/new.rs",
        None,                                // no previous content
        "fn main() {\n    println!(\"hi\");\n}\n",  // new content
        true,                                // written by AI
    );

    assert_eq!(result.path, "src/new.rs");
    assert_eq!(result.lines_added, 3);
    assert_eq!(result.ai_lines.len(), 1);
    assert_eq!(result.ai_lines[0].start, 1);
    assert_eq!(result.ai_lines[0].end, 3);
    assert!(result.human_lines.is_empty());
}

#[test]
fn all_lines_human_when_file_not_touched_by_agent() {
    let result = compute_file_attribution(
        "src/human.rs",
        None,
        "fn main() {}\n",
        false,
    );

    assert_eq!(result.lines_added, 1);
    assert!(result.ai_lines.is_empty());
    assert_eq!(result.human_lines.len(), 1);
}

#[test]
fn summary_computes_percentages() {
    let files = vec![
        compute_file_attribution("a.rs", None, "line1\nline2\n", true),
        compute_file_attribution("b.rs", None, "line1\nline2\nline3\n", false),
    ];
    let summary = compute_attribution_summary(&files);
    assert_eq!(summary.total_lines_added, 5);
    // 2 AI lines out of 5 total = 40%
    assert!((summary.ai_percentage - 40.0).abs() < 0.1);
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p tracevault-core`
Expected: FAIL

**Step 3: Implement attribution engine**

Add `pub mod attribution_engine;` to `crates/tracevault-core/src/lib.rs`.

`crates/tracevault-core/src/attribution_engine.rs`:
```rust
use crate::attribution::{AttributionSummary, FileAttribution, LineRange};

/// Compute attribution for a single file.
///
/// `old_content`: previous content (None if new file)
/// `new_content`: current content
/// `is_ai_authored`: whether the AI agent wrote/edited this file
pub fn compute_file_attribution(
    path: &str,
    old_content: Option<&str>,
    new_content: &str,
    is_ai_authored: bool,
) -> FileAttribution {
    let new_lines: Vec<&str> = new_content.lines().collect();
    let line_count = new_lines.len() as u32;

    if line_count == 0 {
        return FileAttribution {
            path: path.to_string(),
            lines_added: 0,
            lines_deleted: 0,
            ai_lines: vec![],
            human_lines: vec![],
            mixed_lines: vec![],
        };
    }

    match old_content {
        None => {
            // New file: all lines attributed to whoever created it
            let range = vec![LineRange {
                start: 1,
                end: line_count,
            }];
            FileAttribution {
                path: path.to_string(),
                lines_added: line_count,
                lines_deleted: 0,
                ai_lines: if is_ai_authored { range.clone() } else { vec![] },
                human_lines: if is_ai_authored { vec![] } else { range },
                mixed_lines: vec![],
            }
        }
        Some(old) => {
            let old_lines: Vec<&str> = old.lines().collect();
            let old_count = old_lines.len() as u32;

            // Find changed line ranges using simple diff
            let changed = find_changed_lines(&old_lines, &new_lines);
            let added_count = if line_count > old_count {
                line_count - old_count
            } else {
                0
            };
            let deleted_count = if old_count > line_count {
                old_count - line_count
            } else {
                0
            };

            let (ai_lines, human_lines) = if is_ai_authored {
                (changed.clone(), vec![])
            } else {
                (vec![], changed)
            };

            // Unchanged lines retain their original attribution (assumed human for MVP)
            FileAttribution {
                path: path.to_string(),
                lines_added: added_count + changed.iter().map(|r| r.end - r.start + 1).sum::<u32>(),
                lines_deleted: deleted_count,
                ai_lines,
                human_lines,
                mixed_lines: vec![],
            }
        }
    }
}

/// Compute summary across all file attributions
pub fn compute_attribution_summary(files: &[FileAttribution]) -> AttributionSummary {
    let total_ai: u32 = files
        .iter()
        .flat_map(|f| &f.ai_lines)
        .map(|r| r.end - r.start + 1)
        .sum();

    let total_human: u32 = files
        .iter()
        .flat_map(|f| &f.human_lines)
        .map(|r| r.end - r.start + 1)
        .sum();

    let total_mixed: u32 = files
        .iter()
        .flat_map(|f| &f.mixed_lines)
        .map(|r| r.end - r.start + 1)
        .sum();

    let total = total_ai + total_human + total_mixed;
    let total_added: u32 = files.iter().map(|f| f.lines_added).sum();
    let total_deleted: u32 = files.iter().map(|f| f.lines_deleted).sum();

    let ai_pct = if total > 0 {
        (total_ai as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    AttributionSummary {
        total_lines_added: total_added,
        total_lines_deleted: total_deleted,
        ai_percentage: ai_pct,
        human_percentage: 100.0 - ai_pct,
    }
}

/// Simple line-based diff: returns ranges of lines in new_lines that differ from old_lines
fn find_changed_lines(old: &[&str], new: &[&str]) -> Vec<LineRange> {
    let mut ranges = vec![];
    let mut range_start: Option<u32> = None;

    for (i, new_line) in new.iter().enumerate() {
        let is_changed = old.get(i).map_or(true, |old_line| old_line != new_line);

        if is_changed {
            if range_start.is_none() {
                range_start = Some(i as u32 + 1); // 1-indexed
            }
        } else if let Some(start) = range_start.take() {
            ranges.push(LineRange {
                start,
                end: i as u32, // previous line was the end
            });
        }
    }

    // Close any open range
    if let Some(start) = range_start {
        ranges.push(LineRange {
            start,
            end: new.len() as u32,
        });
    }

    ranges
}
```

**Step 4: Run tests**

Run: `cargo test -p tracevault-core`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-core/
git commit -m "feat: add attribution engine for per-line AI/human code mapping"
```

---

## Phase 7: CLI — Init and Hook Installation

### Task 7: Implement `tracevault init` command

**Files:**
- Create: `crates/tracevault-cli/src/commands/mod.rs`
- Create: `crates/tracevault-cli/src/commands/init.rs`
- Create: `crates/tracevault-cli/src/config.rs`
- Modify: `crates/tracevault-cli/src/main.rs`
- Test: `crates/tracevault-cli/tests/init_test.rs`

**Step 1: Write failing test**

`crates/tracevault-cli/tests/init_test.rs`:
```rust
use std::fs;
use tempfile::TempDir;

#[test]
fn init_creates_tracevault_config() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join(".tracevault").join("config.toml");

    // Simulate init
    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("claude-code"));
}
```

Add `tempfile = "3"` to `crates/tracevault-cli/Cargo.toml` under `[dev-dependencies]`.

**Step 2: Run to verify failure**

Run: `cargo test -p tracevault-cli`
Expected: FAIL

**Step 3: Implement init command**

`crates/tracevault-cli/src/config.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct TracevaultConfig {
    pub agent: String,
    pub server_url: Option<String>,
    pub api_key: Option<String>,
}

impl Default for TracevaultConfig {
    fn default() -> Self {
        Self {
            agent: "claude-code".to_string(),
            server_url: None,
            api_key: None,
        }
    }
}

impl TracevaultConfig {
    pub fn config_dir(project_root: &Path) -> PathBuf {
        project_root.join(".tracevault")
    }

    pub fn config_path(project_root: &Path) -> PathBuf {
        Self::config_dir(project_root).join("config.toml")
    }

    pub fn to_toml(&self) -> String {
        format!(
            "# TraceVault configuration\nagent = \"{}\"\n",
            self.agent
        )
    }
}
```

`crates/tracevault-cli/src/commands/mod.rs`:
```rust
pub mod init;
```

`crates/tracevault-cli/src/commands/init.rs`:
```rust
use crate::config::TracevaultConfig;
use std::fs;
use std::io;
use std::path::Path;

pub fn init_in_directory(project_root: &Path) -> Result<(), io::Error> {
    // Create .tracevault/ directory
    let config_dir = TracevaultConfig::config_dir(project_root);
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(config_dir.join("sessions"))?;
    fs::create_dir_all(config_dir.join("cache"))?;

    // Write default config
    let config = TracevaultConfig::default();
    fs::write(TracevaultConfig::config_path(project_root), config.to_toml())?;

    // Create .tracevault/.gitignore
    fs::write(
        config_dir.join(".gitignore"),
        "sessions/\ncache/\n*.local.toml\n",
    )?;

    Ok(())
}

/// Generate Claude Code hook configuration JSON
pub fn claude_code_hooks_json() -> serde_json::Value {
    serde_json::json!({
        "hooks": {
            "PreToolUse": [{
                "matcher": "Write|Edit",
                "hooks": [{
                    "type": "command",
                    "command": "tracevault hook --event pre-tool-use",
                    "timeout": 5,
                    "statusMessage": "TraceVault: capturing pre-edit state"
                }]
            }],
            "PostToolUse": [{
                "matcher": "Write|Edit|Bash",
                "hooks": [{
                    "type": "command",
                    "command": "tracevault hook --event post-tool-use",
                    "timeout": 5,
                    "statusMessage": "TraceVault: recording change"
                }]
            }]
        }
    })
}
```

Update `crates/tracevault-cli/src/main.rs`:
```rust
use clap::Parser;
use std::env;

pub mod commands;
pub mod config;

#[derive(Parser)]
#[command(name = "tracevault", about = "AI code governance platform")]
enum Cli {
    /// Initialize TraceVault in current repository
    Init,
    /// Show current session status
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Init => {
            let cwd = env::current_dir().expect("Cannot determine current directory");
            match commands::init::init_in_directory(&cwd) {
                Ok(()) => {
                    println!("TraceVault initialized in {}", cwd.display());
                    println!("\nTo enable Claude Code hooks, add this to .claude/settings.json:");
                    let hooks = commands::init::claude_code_hooks_json();
                    println!("{}", serde_json::to_string_pretty(&hooks).unwrap());
                }
                Err(e) => eprintln!("Error: {e}"),
            }
        }
        Cli::Status => println!("tracevault status - not yet implemented"),
    }
}
```

Also add to `crates/tracevault-cli/Cargo.toml` under `[lib]`:
```toml
[lib]
name = "tracevault_cli"
path = "src/lib.rs"
```

Create `crates/tracevault-cli/src/lib.rs`:
```rust
pub mod commands;
pub mod config;
```

**Step 4: Run tests**

Run: `cargo test -p tracevault-cli`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-cli/
git commit -m "feat: implement tracevault init with config creation and hook config generation"
```

---

## Phase 8: CLI — Hook Command (Event Capture)

### Task 8: Implement `tracevault hook` for real-time event capture

**Files:**
- Create: `crates/tracevault-cli/src/commands/hook.rs`
- Modify: `crates/tracevault-cli/src/commands/mod.rs`
- Modify: `crates/tracevault-cli/src/main.rs`
- Test: `crates/tracevault-cli/tests/hook_test.rs`

**Step 1: Write failing test**

`crates/tracevault-cli/tests/hook_test.rs`:
```rust
use std::fs;
use tempfile::TempDir;

#[test]
fn hook_handler_records_event_to_session_dir() {
    let tmp = TempDir::new().unwrap();

    // Set up .tracevault/sessions/
    fs::create_dir_all(tmp.path().join(".tracevault/sessions")).unwrap();

    let hook_json = serde_json::json!({
        "session_id": "test-session-123",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": tmp.path().join("src/main.rs").to_str().unwrap(),
            "content": "fn main() {}"
        }
    });

    let result = tracevault_cli::commands::hook::handle_hook_event(
        &hook_json.to_string(),
        tmp.path(),
    );
    assert!(result.is_ok());

    // Check that event was recorded
    let session_dir = tmp.path().join(".tracevault/sessions/test-session-123");
    assert!(session_dir.join("events.jsonl").exists());
}
```

**Step 2: Run to verify failure**

Run: `cargo test -p tracevault-cli`
Expected: FAIL

**Step 3: Implement hook handler**

`crates/tracevault-cli/src/commands/hook.rs`:
```rust
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use tracevault_core::hooks::{parse_hook_event, HookResponse};

pub fn handle_hook_from_stdin(project_root: &Path) -> Result<(), io::Error> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    handle_hook_event(&input, project_root)?;

    // Output JSON response to stdout
    let response = HookResponse::allow();
    println!("{}", serde_json::to_string(&response).unwrap());
    Ok(())
}

pub fn handle_hook_event(json_input: &str, project_root: &Path) -> Result<(), io::Error> {
    let event = parse_hook_event(json_input)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Create session directory
    let session_dir = project_root
        .join(".tracevault")
        .join("sessions")
        .join(&event.session_id);
    fs::create_dir_all(&session_dir)?;

    // Append event to events.jsonl
    let events_path = session_dir.join("events.jsonl");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&events_path)?;

    let event_json = serde_json::to_string(&event)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writeln!(file, "{event_json}")?;

    // Write session metadata if it doesn't exist
    let meta_path = session_dir.join("metadata.json");
    if !meta_path.exists() {
        let metadata = serde_json::json!({
            "session_id": event.session_id,
            "transcript_path": event.transcript_path,
            "cwd": event.cwd,
            "started_at": chrono::Utc::now().to_rfc3339(),
        });
        fs::write(&meta_path, serde_json::to_string_pretty(&metadata).unwrap())?;
    }

    Ok(())
}
```

Add `chrono.workspace = true` to `crates/tracevault-cli/Cargo.toml`.

Update `crates/tracevault-cli/src/commands/mod.rs`:
```rust
pub mod hook;
pub mod init;
```

Update `crates/tracevault-cli/src/lib.rs`:
```rust
pub mod commands;
pub mod config;
```

Update CLI in `main.rs` to add `Hook` subcommand:
```rust
#[derive(Parser)]
#[command(name = "tracevault", about = "AI code governance platform")]
enum Cli {
    Init,
    Status,
    /// Handle Claude Code hook event (reads JSON from stdin)
    Hook {
        #[arg(long)]
        event: String,
    },
}
```

And the match arm:
```rust
Cli::Hook { event: _ } => {
    let cwd = env::current_dir().expect("Cannot determine current directory");
    if let Err(e) = commands::hook::handle_hook_from_stdin(&cwd) {
        eprintln!("Hook error: {e}");
    }
}
```

**Step 4: Run tests**

Run: `cargo test -p tracevault-cli`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/tracevault-cli/
git commit -m "feat: implement hook command for real-time Claude Code event capture"
```

---

## Phase 9: Server — Database and Migrations

### Task 9: Set up PostgreSQL with sqlx migrations

**Files:**
- Create: `crates/tracevault-server/migrations/001_initial.sql`
- Modify: `crates/tracevault-server/Cargo.toml` (add sqlx)
- Create: `crates/tracevault-server/src/db.rs`
- Create: `docker-compose.yml`

**Step 1: Create docker-compose.yml for dev**

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_DB: tracevault
      POSTGRES_USER: tracevault
      POSTGRES_PASSWORD: tracevault
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

**Step 2: Create migration**

`crates/tracevault-server/migrations/001_initial.sql`:
```sql
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE orgs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    plan TEXT NOT NULL DEFAULT 'free',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    key_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE repos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    name TEXT NOT NULL,
    github_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(org_id, name)
);

CREATE TABLE traces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id),
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

CREATE INDEX idx_traces_repo_id ON traces(repo_id);
CREATE INDEX idx_traces_commit_sha ON traces(commit_sha);
CREATE INDEX idx_traces_author ON traces(author);
CREATE INDEX idx_traces_created_at ON traces(created_at);

CREATE TABLE policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    name TEXT NOT NULL,
    description TEXT,
    condition JSONB NOT NULL,
    action TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'medium',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trace_id UUID NOT NULL REFERENCES traces(id),
    policy_id UUID NOT NULL REFERENCES policies(id),
    result TEXT NOT NULL,
    details JSONB,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Step 3: Add sqlx to server**

Update `crates/tracevault-server/Cargo.toml`:
```toml
[dependencies]
tracevault-core = { path = "../tracevault-core" }
axum = "0.8"
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
uuid.workspace = true
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "json"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
```

**Step 4: Implement db module**

`crates/tracevault-server/src/db.rs`:
```rust
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
```

**Step 5: Start database and verify migration**

```bash
docker compose up -d db
DATABASE_URL=postgres://tracevault:tracevault@localhost:5432/tracevault sqlx migrate run --source crates/tracevault-server/migrations/
```

**Step 6: Commit**

```bash
git add docker-compose.yml crates/tracevault-server/
git commit -m "feat: add PostgreSQL schema, migrations, and docker-compose for dev"
```

---

## Phase 10: Server — API Endpoints

### Task 10: Implement trace ingestion and query APIs

**Files:**
- Create: `crates/tracevault-server/src/api/mod.rs`
- Create: `crates/tracevault-server/src/api/traces.rs`
- Create: `crates/tracevault-server/src/api/policies.rs`
- Create: `crates/tracevault-server/src/api/auth.rs`
- Create: `crates/tracevault-server/src/api/analytics.rs`
- Create: `crates/tracevault-server/src/api/github.rs`
- Create: `crates/tracevault-server/src/config.rs`
- Modify: `crates/tracevault-server/src/main.rs`

This is a larger task. Implement endpoint by endpoint with tests.

**Step 1: Create server config**

`crates/tracevault-server/src/config.rs`:
```rust
use std::env;

pub struct ServerConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://tracevault:tracevault@localhost:5432/tracevault".into()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
```

**Step 2: Implement traces API**

`crates/tracevault-server/src/api/mod.rs`:
```rust
pub mod analytics;
pub mod auth;
pub mod github;
pub mod policies;
pub mod traces;
```

`crates/tracevault-server/src/api/traces.rs`:
```rust
use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateTraceRequest {
    pub repo_name: String,
    pub org_name: String,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub tool_version: Option<String>,
    pub ai_percentage: Option<f32>,
    pub total_tokens: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub api_calls: Option<i32>,
    pub session_data: Option<serde_json::Value>,
    pub attribution: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct TraceResponse {
    pub id: Uuid,
    pub repo_id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub ai_percentage: Option<f32>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TraceQuery {
    pub repo: Option<String>,
    pub sha: Option<String>,
    pub author: Option<String>,
    pub limit: Option<i64>,
}

pub async fn create_trace(
    State(pool): State<PgPool>,
    Json(req): Json<CreateTraceRequest>,
) -> Result<(StatusCode, Json<TraceResponse>), (StatusCode, String)> {
    // Ensure org exists (create if not)
    let org_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orgs (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = $1 RETURNING id"
    )
    .bind(&req.org_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Ensure repo exists (create if not)
    let repo_id: Uuid = sqlx::query_scalar(
        "INSERT INTO repos (org_id, name) VALUES ($1, $2) ON CONFLICT (org_id, name) DO UPDATE SET name = $2 RETURNING id"
    )
    .bind(org_id)
    .bind(&req.repo_name)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Insert trace
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>, String, Option<String>, Option<String>, Option<f32>, Option<i64>, Option<f64>, chrono::DateTime<chrono::Utc>)>(
        "INSERT INTO traces (repo_id, commit_sha, branch, author, model, tool, tool_version, ai_percentage, total_tokens, input_tokens, output_tokens, estimated_cost_usd, api_calls, session_data, attribution)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
         RETURNING id, commit_sha, branch, author, model, tool, ai_percentage, total_tokens, estimated_cost_usd, created_at"
    )
    .bind(repo_id)
    .bind(&req.commit_sha)
    .bind(&req.branch)
    .bind(&req.author)
    .bind(&req.model)
    .bind(&req.tool)
    .bind(&req.tool_version)
    .bind(req.ai_percentage)
    .bind(req.total_tokens)
    .bind(req.input_tokens)
    .bind(req.output_tokens)
    .bind(req.estimated_cost_usd)
    .bind(req.api_calls)
    .bind(&req.session_data)
    .bind(&req.attribution)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(TraceResponse {
        id: row.0,
        repo_id,
        commit_sha: row.1,
        branch: row.2,
        author: row.3,
        model: row.4,
        tool: row.5,
        ai_percentage: row.6,
        total_tokens: row.7,
        estimated_cost_usd: row.8,
        created_at: row.9,
    })))
}

pub async fn get_trace(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, String, Option<String>, Option<String>, Option<f32>, Option<i64>, Option<f64>, Option<serde_json::Value>, Option<serde_json::Value>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, repo_id, commit_sha, branch, author, model, tool, ai_percentage, total_tokens, estimated_cost_usd, session_data, attribution, created_at FROM traces WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Trace not found".into()))?;

    Ok(Json(serde_json::json!({
        "id": row.0,
        "repo_id": row.1,
        "commit_sha": row.2,
        "branch": row.3,
        "author": row.4,
        "model": row.5,
        "tool": row.6,
        "ai_percentage": row.7,
        "total_tokens": row.8,
        "estimated_cost_usd": row.9,
        "session_data": row.10,
        "attribution": row.11,
        "created_at": row.12,
    })))
}

pub async fn list_traces(
    State(pool): State<PgPool>,
    Query(query): Query<TraceQuery>,
) -> Result<Json<Vec<TraceResponse>>, (StatusCode, String)> {
    let limit = query.limit.unwrap_or(50).min(200);

    // Build dynamic query
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, Option<String>, String, Option<String>, Option<String>, Option<f32>, Option<i64>, Option<f64>, chrono::DateTime<chrono::Utc>)>(
        "SELECT t.id, t.repo_id, t.commit_sha, t.branch, t.author, t.model, t.tool, t.ai_percentage, t.total_tokens, t.estimated_cost_usd, t.created_at
         FROM traces t
         LEFT JOIN repos r ON t.repo_id = r.id
         WHERE ($1::TEXT IS NULL OR r.name = $1)
           AND ($2::TEXT IS NULL OR t.commit_sha = $2)
           AND ($3::TEXT IS NULL OR t.author = $3)
         ORDER BY t.created_at DESC
         LIMIT $4"
    )
    .bind(&query.repo)
    .bind(&query.sha)
    .bind(&query.author)
    .bind(limit)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let traces: Vec<TraceResponse> = rows.into_iter().map(|r| TraceResponse {
        id: r.0,
        repo_id: r.1,
        commit_sha: r.2,
        branch: r.3,
        author: r.4,
        model: r.5,
        tool: r.6,
        ai_percentage: r.7,
        total_tokens: r.8,
        estimated_cost_usd: r.9,
        created_at: r.10,
    }).collect();

    Ok(Json(traces))
}
```

**Step 3: Create stub API files for remaining endpoints**

`crates/tracevault-server/src/api/auth.rs`:
```rust
use axum::{http::StatusCode, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub org_name: String,
}

pub async fn register(
    Json(_req): Json<RegisterRequest>,
) -> (StatusCode, &'static str) {
    // TODO: implement org registration with API key generation
    (StatusCode::OK, "registration endpoint - not yet implemented")
}
```

`crates/tracevault-server/src/api/policies.rs`:
```rust
use axum::{extract::State, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn list_policies(
    State(_pool): State<PgPool>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "policies endpoint - not yet implemented")
}

pub async fn evaluate(
    State(_pool): State<PgPool>,
    Json(_body): Json<serde_json::Value>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "evaluate endpoint - not yet implemented")
}
```

`crates/tracevault-server/src/api/analytics.rs`:
```rust
use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct TokenQuery {
    pub group_by: Option<String>,
    pub period: Option<String>,
}

pub async fn token_analytics(
    State(_pool): State<PgPool>,
    Query(_query): Query<TokenQuery>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "analytics endpoint - not yet implemented")
}
```

`crates/tracevault-server/src/api/github.rs`:
```rust
use axum::{http::StatusCode, Json};

pub async fn webhook(
    Json(_body): Json<serde_json::Value>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "github webhook - not yet implemented")
}
```

**Step 4: Wire up router in main.rs**

`crates/tracevault-server/src/main.rs`:
```rust
use axum::{routing::{get, post}, Router};
use tower_http::trace::TraceLayer;

mod api;
mod config;
mod db;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = config::ServerConfig::from_env();
    let pool = db::create_pool(&cfg.database_url)
        .await
        .expect("Failed to connect to database");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        // Traces
        .route("/api/v1/traces", post(api::traces::create_trace))
        .route("/api/v1/traces", get(api::traces::list_traces))
        .route("/api/v1/traces/{id}", get(api::traces::get_trace))
        // Auth
        .route("/api/v1/auth/register", post(api::auth::register))
        // Policies
        .route("/api/v1/policies", get(api::policies::list_policies))
        .route("/api/v1/policies/evaluate", post(api::policies::evaluate))
        // Analytics
        .route("/api/v1/analytics/tokens", get(api::analytics::token_analytics))
        // GitHub
        .route("/api/v1/github/webhook", post(api::github::webhook))
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr())
        .await
        .unwrap();
    tracing::info!("TraceVault server listening on {}", cfg.bind_addr());
    axum::serve(listener, app).await.unwrap();
}
```

**Step 5: Build and verify**

Run: `cargo build -p tracevault-server`
Expected: Compiles without errors.

**Step 6: Commit**

```bash
git add crates/tracevault-server/ docker-compose.yml
git commit -m "feat: implement server with trace ingestion/query API, PostgreSQL, and stub endpoints"
```

---

## Phase 11: CLI — Push and Stats Commands

### Task 11: Implement `tracevault push` and `tracevault stats`

**Files:**
- Create: `crates/tracevault-cli/src/commands/push.rs`
- Create: `crates/tracevault-cli/src/commands/stats.rs`
- Create: `crates/tracevault-cli/src/api_client.rs`
- Modify: `crates/tracevault-cli/src/commands/mod.rs`
- Modify: `crates/tracevault-cli/src/main.rs`

**Step 1: Implement API client**

Add `reqwest = { version = "0.12", features = ["json"] }` to `crates/tracevault-cli/Cargo.toml`.

`crates/tracevault-cli/src/api_client.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::error::Error;

pub struct ApiClient {
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Serialize)]
pub struct PushTraceRequest {
    pub repo_name: String,
    pub org_name: String,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub ai_percentage: Option<f32>,
    pub total_tokens: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub api_calls: Option<i32>,
    pub session_data: Option<serde_json::Value>,
    pub attribution: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct PushTraceResponse {
    pub id: uuid::Uuid,
}

impl ApiClient {
    pub fn new(base_url: &str, api_key: Option<&str>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.map(String::from),
            client: reqwest::Client::new(),
        }
    }

    pub async fn push_trace(
        &self,
        req: PushTraceRequest,
    ) -> Result<PushTraceResponse, Box<dyn Error>> {
        let mut builder = self.client.post(format!("{}/api/v1/traces", self.base_url));

        if let Some(key) = &self.api_key {
            builder = builder.header("Authorization", format!("Bearer {key}"));
        }

        let resp = builder.json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Server returned {status}: {body}").into());
        }

        Ok(resp.json().await?)
    }
}
```

**Step 2: Implement push command**

`crates/tracevault-cli/src/commands/push.rs`:
```rust
use crate::api_client::{ApiClient, PushTraceRequest};
use crate::config::TracevaultConfig;
use std::fs;
use std::path::Path;

pub async fn push_traces(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = TracevaultConfig::config_path(project_root);
    let config_content = fs::read_to_string(&config_path)?;

    // Parse minimal config to get server URL
    let server_url = config_content
        .lines()
        .find(|l| l.starts_with("server_url"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| "http://localhost:3000".into());

    let client = ApiClient::new(&server_url, None);

    // Read pending sessions
    let sessions_dir = project_root.join(".tracevault").join("sessions");
    if !sessions_dir.exists() {
        println!("No pending sessions to push.");
        return Ok(());
    }

    let mut pushed = 0;
    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let meta_path = entry.path().join("metadata.json");
            if meta_path.exists() {
                let meta: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&meta_path)?)?;

                let req = PushTraceRequest {
                    repo_name: "unknown".into(), // TODO: detect from git remote
                    org_name: "default".into(),
                    commit_sha: "pending".into(),
                    branch: None,
                    author: meta.get("author").and_then(|v| v.as_str()).unwrap_or("unknown").into(),
                    model: None,
                    tool: Some("claude-code".into()),
                    ai_percentage: None,
                    total_tokens: None,
                    input_tokens: None,
                    output_tokens: None,
                    estimated_cost_usd: None,
                    api_calls: None,
                    session_data: Some(meta.clone()),
                    attribution: None,
                };

                match client.push_trace(req).await {
                    Ok(resp) => {
                        println!("Pushed trace {} -> {}", entry.file_name().to_string_lossy(), resp.id);
                        pushed += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to push {}: {e}", entry.file_name().to_string_lossy());
                    }
                }
            }
        }
    }

    println!("Pushed {pushed} trace(s) to server.");
    Ok(())
}
```

**Step 3: Implement stats command**

`crates/tracevault-cli/src/commands/stats.rs`:
```rust
use std::fs;
use std::path::Path;

pub fn show_stats(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let sessions_dir = project_root.join(".tracevault").join("sessions");
    if !sessions_dir.exists() {
        println!("No sessions found. Run `tracevault init` first.");
        return Ok(());
    }

    let mut total_sessions = 0;
    let mut total_events = 0;

    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            total_sessions += 1;
            let events_path = entry.path().join("events.jsonl");
            if events_path.exists() {
                let content = fs::read_to_string(&events_path)?;
                total_events += content.lines().count();
            }
        }
    }

    println!("TraceVault Stats");
    println!("================");
    println!("Sessions:     {total_sessions}");
    println!("Total events: {total_events}");

    Ok(())
}
```

**Step 4: Update mod.rs and main.rs**

`crates/tracevault-cli/src/commands/mod.rs`:
```rust
pub mod hook;
pub mod init;
pub mod push;
pub mod stats;
```

`crates/tracevault-cli/src/lib.rs`:
```rust
pub mod api_client;
pub mod commands;
pub mod config;
```

Add all new subcommands to `main.rs` Cli enum and match arms.

**Step 5: Build and verify**

Run: `cargo build -p tracevault-cli`
Expected: Compiles.

**Step 6: Commit**

```bash
git add crates/tracevault-cli/
git commit -m "feat: implement push and stats CLI commands with API client"
```

---

## Phase 12: GitHub Action

### Task 12: Create GitHub Action composite action

**Files:**
- Create: `action/action.yml`
- Create: `action/entrypoint.sh`

**Step 1: Create action.yml**

`action/action.yml`:
```yaml
name: "TraceVault Check"
description: "Evaluate AI code governance policies on pull requests"
branding:
  icon: shield
  color: blue

inputs:
  server-url:
    description: "TraceVault server URL"
    required: false
    default: ""
  api-key:
    description: "TraceVault API key"
    required: false
    default: ""
  policies:
    description: "Comma-separated list of policies to enforce (default: all)"
    required: false
    default: "all"

runs:
  using: "composite"
  steps:
    - name: Install TraceVault CLI
      shell: bash
      run: |
        # Download latest tracevault binary
        # For MVP: build from source or download release binary
        echo "Installing TraceVault CLI..."
        if ! command -v tracevault &> /dev/null; then
          echo "::warning::TraceVault CLI not found. Install it or add it to the release."
        fi

    - name: Check traces and evaluate policies
      shell: bash
      env:
        TRACEVAULT_SERVER_URL: ${{ inputs.server-url }}
        TRACEVAULT_API_KEY: ${{ inputs.api-key }}
      run: |
        echo "## TraceVault Report" >> $GITHUB_STEP_SUMMARY
        echo "" >> $GITHUB_STEP_SUMMARY

        # Read git notes for tracevault data
        git fetch origin refs/notes/tracevault:refs/notes/tracevault 2>/dev/null || true

        # Get commits in this PR
        BASE_SHA="${{ github.event.pull_request.base.sha }}"
        HEAD_SHA="${{ github.event.pull_request.head.sha }}"

        COMMITS=$(git log --format=%H ${BASE_SHA}..${HEAD_SHA} 2>/dev/null || echo "")

        if [ -z "$COMMITS" ]; then
          echo "| Metric | Value |" >> $GITHUB_STEP_SUMMARY
          echo "|--------|-------|" >> $GITHUB_STEP_SUMMARY
          echo "| Status | No commits found in PR |" >> $GITHUB_STEP_SUMMARY
          exit 0
        fi

        TRACE_COUNT=0
        TOTAL_AI_PCT=0
        TOTAL_TOKENS=0

        for SHA in $COMMITS; do
          NOTE=$(git notes --ref=tracevault show $SHA 2>/dev/null || echo "")
          if [ -n "$NOTE" ]; then
            TRACE_COUNT=$((TRACE_COUNT + 1))
          fi
        done

        COMMIT_COUNT=$(echo "$COMMITS" | wc -l | tr -d ' ')

        echo "| Metric | Value |" >> $GITHUB_STEP_SUMMARY
        echo "|--------|-------|" >> $GITHUB_STEP_SUMMARY
        echo "| Commits | $COMMIT_COUNT |" >> $GITHUB_STEP_SUMMARY
        echo "| Traces found | $TRACE_COUNT |" >> $GITHUB_STEP_SUMMARY
        echo "" >> $GITHUB_STEP_SUMMARY

        # Policy evaluation
        echo "### Policy Results" >> $GITHUB_STEP_SUMMARY

        if [ "$TRACE_COUNT" -eq 0 ] && [ "$COMMIT_COUNT" -gt 0 ]; then
          echo "- ⚠️ No traces found for $COMMIT_COUNT commit(s)" >> $GITHUB_STEP_SUMMARY
        else
          echo "- ✅ Traces present for $TRACE_COUNT/$COMMIT_COUNT commit(s)" >> $GITHUB_STEP_SUMMARY
        fi
```

**Step 2: Commit**

```bash
git add action/
git commit -m "feat: add GitHub Action for PR policy evaluation"
```

---

## Phase 13: Docker Compose Production Deployment

### Task 13: Complete docker-compose with server

**Files:**
- Modify: `docker-compose.yml`
- Create: `Dockerfile`

**Step 1: Create Dockerfile**

```dockerfile
FROM rust:1.84-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release -p tracevault-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/tracevault-server /usr/local/bin/
COPY crates/tracevault-server/migrations/ /app/migrations/
EXPOSE 3000
CMD ["tracevault-server"]
```

**Step 2: Update docker-compose.yml**

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_DB: tracevault
      POSTGRES_USER: tracevault
      POSTGRES_PASSWORD: tracevault
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U tracevault"]
      interval: 5s
      timeout: 5s
      retries: 5

  server:
    build: .
    depends_on:
      db:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://tracevault:tracevault@db:5432/tracevault
      HOST: 0.0.0.0
      PORT: 3000
      RUST_LOG: info
    ports:
      - "3000:3000"

volumes:
  pgdata:
```

**Step 3: Build and test**

```bash
docker compose build
docker compose up -d
curl http://localhost:3000/health
```
Expected: Returns "ok"

**Step 4: Commit**

```bash
git add Dockerfile docker-compose.yml
git commit -m "feat: add Dockerfile and production docker-compose configuration"
```

---

## Phase 14: Integration Test

### Task 14: End-to-end test: hook event → push → query

**Files:**
- Create: `tests/integration/e2e_test.rs`

**Step 1: Write integration test**

This test requires a running PostgreSQL (use docker compose).

`tests/integration/e2e_test.rs`:
```rust
//! Integration test: simulates the full flow
//! Run with: cargo test -p tracevault-cli --test e2e_test
//! Requires: docker compose up -d db

use std::fs;
use tempfile::TempDir;

#[test]
fn full_flow_init_hook_and_local_stats() {
    let tmp = TempDir::new().unwrap();

    // 1. Init
    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();
    assert!(tmp.path().join(".tracevault/config.toml").exists());

    // 2. Simulate hook events
    let hook_json = serde_json::json!({
        "session_id": "e2e-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": tmp.path().join("src/main.rs").to_str().unwrap(),
            "content": "fn main() { println!(\"hello\"); }"
        }
    });

    tracevault_cli::commands::hook::handle_hook_event(
        &hook_json.to_string(),
        tmp.path(),
    ).unwrap();

    // 3. Verify session data was captured
    let session_dir = tmp.path().join(".tracevault/sessions/e2e-session");
    assert!(session_dir.join("events.jsonl").exists());
    assert!(session_dir.join("metadata.json").exists());

    let events = fs::read_to_string(session_dir.join("events.jsonl")).unwrap();
    assert!(events.contains("PostToolUse"));
    assert!(events.contains("Write"));
}
```

**Step 2: Run**

Run: `cargo test --test e2e_test -p tracevault-cli`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add end-to-end integration test for init → hook → stats flow"
```

---

## Summary: Task Dependency Graph

```
Phase 1: Scaffolding (Task 1)
    ↓
Phase 2: Core Types (Task 2)
    ↓
    ├── Phase 3: Redaction (Task 3)
    ├── Phase 4: Policy Engine (Task 4)
    ├── Phase 5: Hook Parser (Task 5)
    └── Phase 6: Attribution Engine (Task 6)
         ↓
Phase 7: CLI Init (Task 7)
    ↓
Phase 8: CLI Hook Command (Task 8)
    ↓
Phase 9: Server DB (Task 9)
    ↓
Phase 10: Server API (Task 10)
    ↓
Phase 11: CLI Push/Stats (Task 11)
    ↓
Phase 12: GitHub Action (Task 12)
    ↓
Phase 13: Docker Deployment (Task 13)
    ↓
Phase 14: Integration Test (Task 14)
```

**Parallelizable:** Tasks 3, 4, 5, 6 can all be done in parallel after Task 2.

**Total tasks:** 14
**Critical path:** Tasks 1 → 2 → 5 → 7 → 8 → 9 → 10 → 11 → 13 → 14
