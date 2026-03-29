use std::path::Path;
use tracevault_core::streaming::StreamEventRequest;

use super::HookAdapter;

pub struct ClaudeCodeAdapter;

impl HookAdapter for ClaudeCodeAdapter {
    fn tool_name(&self) -> &str {
        "claude-code"
    }

    fn parse_event(&self, raw: &str) -> Result<StreamEventRequest, String> {
        let mut req: StreamEventRequest =
            serde_json::from_str(raw).map_err(|e| format!("Failed to parse event JSON: {e}"))?;
        req.tool = Some("claude-code".to_string());
        if req.protocol_version == 0 {
            req.protocol_version = 2;
        }
        Ok(req)
    }

    fn parse_transcript(&self, path: &Path) -> Result<Vec<serde_json::Value>, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read transcript: {e}"))?;
        let mut lines = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(line) {
                Ok(v) => lines.push(v),
                Err(_) => continue,
            }
        }
        Ok(lines)
    }
}
