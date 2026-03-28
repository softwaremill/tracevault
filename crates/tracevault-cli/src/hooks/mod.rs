use std::path::Path;
use tracevault_core::streaming::StreamEventRequest;

pub mod claude_code;
pub mod cursor;

pub trait HookAdapter: Send + Sync {
    fn tool_name(&self) -> &str;
    fn parse_event(&self, raw: &str) -> Result<StreamEventRequest, String>;
    fn parse_transcript(&self, path: &Path) -> Result<Vec<serde_json::Value>, String>;
}

#[derive(Debug, Clone, Copy)]
pub enum DetectedTool {
    ClaudeCode,
    Cursor,
}

impl DetectedTool {
    pub fn name(&self) -> &str {
        match self {
            DetectedTool::ClaudeCode => "claude-code",
            DetectedTool::Cursor => "cursor",
        }
    }

    pub fn adapter(&self) -> Box<dyn HookAdapter> {
        match self {
            DetectedTool::ClaudeCode => Box::new(claude_code::ClaudeCodeAdapter),
            DetectedTool::Cursor => Box::new(cursor::CursorAdapter),
        }
    }
}

pub fn detect_tools(cwd: &Path) -> Vec<DetectedTool> {
    let mut tools = vec![];
    if cwd.join(".claude").exists() {
        tools.push(DetectedTool::ClaudeCode);
    }
    if cwd.join(".cursor").exists() {
        tools.push(DetectedTool::Cursor);
    }
    tools
}
