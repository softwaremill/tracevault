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
        self.tool_input
            .as_ref()?
            .get("file_path")?
            .as_str()
            .map(String::from)
    }

    pub fn is_file_modification(&self) -> bool {
        matches!(self.tool_name.as_deref(), Some("Write") | Some("Edit"))
    }
}
