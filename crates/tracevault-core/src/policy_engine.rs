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
        if !has_session {
            missing.push("session");
        }
        if !has_model {
            missing.push("model");
        }
        if !has_attribution {
            missing.push("attribution");
        }
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
        (
            EvalResult::Pass,
            format!("AI percentage {pct:.1}% within threshold"),
        )
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
            format!(
                "Model {model} is not in allowlist: {}",
                allowed.join(", ")
            ),
        ),
        None => (EvalResult::Fail, "No model specified in trace".into()),
    }
}

fn eval_sensitive_paths(trace: &TraceRecord, patterns: &[String]) -> (EvalResult, String) {
    let matched: Vec<_> = trace
        .attribution
        .files
        .iter()
        .filter(|f| patterns.iter().any(|p| f.path.contains(p)) && !f.ai_lines.is_empty())
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
    let missing: Vec<_> = required
        .iter()
        .filter(|r| !used.iter().any(|u| u.contains(r.as_str())))
        .collect();

    if missing.is_empty() {
        (EvalResult::Pass, "All required tools were used".into())
    } else {
        (
            EvalResult::Warn,
            format!(
                "Missing required tools: {}",
                missing
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
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

    let token_over = max_tokens.is_some_and(|max| tokens > max);
    let cost_over = max_cost.is_some_and(|max| cost > max);

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
