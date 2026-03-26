use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
            agent_trace: None,
            signature: None,
        }
    }
}
