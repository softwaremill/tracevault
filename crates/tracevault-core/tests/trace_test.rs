use tracevault_core::trace::*;
use tracevault_core::attribution::*;
use tracevault_core::token_usage::*;
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
