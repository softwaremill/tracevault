use tracevault_core::attribution::*;
use tracevault_core::policy::*;
use tracevault_core::policy_engine::PolicyEngine;
use tracevault_core::token_usage::TokenUsage;
use tracevault_core::trace::*;

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
            prompts: vec![Prompt {
                text: "do something".into(),
                timestamp: Utc::now(),
            }],
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
            files: files
                .iter()
                .map(|p| FileAttribution {
                    path: p.to_string(),
                    lines_added: 10,
                    lines_deleted: 0,
                    ai_lines: vec![LineRange { start: 1, end: 10 }],
                    human_lines: vec![],
                    mixed_lines: vec![],
                })
                .collect(),
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
    let completeness = results
        .iter()
        .find(|r| r.policy.name == "Trace completeness")
        .unwrap();
    assert_eq!(completeness.result, EvalResult::Pass);
}

#[test]
fn ai_percentage_warns_above_threshold() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(95.0, "anthropic/claude-opus-4-6", 1000, 0.5, vec!["src/main.rs"]);
    let results = engine.evaluate(&trace);
    let pct = results
        .iter()
        .find(|r| r.policy.name == "AI percentage threshold")
        .unwrap();
    assert_eq!(pct.result, EvalResult::Warn);
}

#[test]
fn model_allowlist_fails_for_unknown_model() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(50.0, "unknown/bad-model", 1000, 0.5, vec!["src/main.rs"]);
    let results = engine.evaluate(&trace);
    let model = results
        .iter()
        .find(|r| r.policy.name == "Model allowlist")
        .unwrap();
    assert_eq!(model.result, EvalResult::Fail);
}

#[test]
fn sensitive_path_flags_payments() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(
        50.0,
        "anthropic/claude-opus-4-6",
        1000,
        0.5,
        vec!["src/payments/charge.rs"],
    );
    let results = engine.evaluate(&trace);
    let sensitive = results
        .iter()
        .find(|r| r.policy.name == "Sensitive path review")
        .unwrap();
    assert_eq!(sensitive.result, EvalResult::Warn);
}

#[test]
fn token_budget_warns_over_limit() {
    let engine = PolicyEngine::with_defaults();
    let trace = make_trace(
        50.0,
        "anthropic/claude-opus-4-6",
        600_000,
        55.0,
        vec!["src/main.rs"],
    );
    let results = engine.evaluate(&trace);
    let budget = results
        .iter()
        .find(|r| r.policy.name == "Token budget")
        .unwrap();
    assert_eq!(budget.result, EvalResult::Warn);
}
