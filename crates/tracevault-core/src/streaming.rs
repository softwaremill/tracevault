use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StreamEventType {
    ToolUse,
    Transcript,
    SessionStart,
    SessionEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEventRequest {
    pub protocol_version: u32,
    #[serde(default)]
    pub tool: Option<String>,
    pub event_type: StreamEventType,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub hook_event_name: Option<String>,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_response: Option<serde_json::Value>,
    pub event_index: Option<i32>,
    pub transcript_lines: Option<Vec<serde_json::Value>>,
    pub transcript_offset: Option<i64>,
    pub model: Option<String>,
    pub cwd: Option<String>,
    pub final_stats: Option<SessionFinalStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFinalStats {
    pub duration_ms: Option<i64>,
    pub total_tokens: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    pub user_messages: Option<i32>,
    pub assistant_messages: Option<i32>,
    pub total_tool_calls: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEventResponse {
    pub session_db_id: uuid::Uuid,
    pub event_db_id: Option<uuid::Uuid>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitPushRequest {
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub message: Option<String>,
    pub diff_data: Option<serde_json::Value>,
    pub committed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitPushResponse {
    pub commit_db_id: uuid::Uuid,
    pub attributions_count: i64,
}

#[derive(Debug, Clone)]
pub struct ExtractedFileChange {
    pub file_path: String,
    pub change_type: String,
    pub diff_text: Option<String>,
    pub content_hash: Option<String>,
}
