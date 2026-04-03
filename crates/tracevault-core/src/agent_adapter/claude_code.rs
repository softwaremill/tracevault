use sha2::{Digest, Sha256};

use crate::streaming::{ExtractedFileChange, StreamEventType};

use super::{AgentAdapter, ParsedTranscriptRecord, TokenUsage};

pub struct ClaudeCodeAdapter;

impl AgentAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn map_event_type(&self, hook_event_name: &str) -> StreamEventType {
        match hook_event_name {
            "SessionStart" => StreamEventType::SessionStart,
            "Stop" => StreamEventType::SessionEnd,
            _ => StreamEventType::ToolUse,
        }
    }

    fn is_file_modifying(&self, tool_name: &str) -> bool {
        matches!(tool_name, "Write" | "Edit" | "Bash")
    }

    fn extract_file_changes(
        &self,
        tool_name: &str,
        tool_input: &serde_json::Value,
    ) -> Vec<ExtractedFileChange> {
        match tool_name {
            "Write" => {
                let file_path = match tool_input.get("file_path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return Vec::new(),
                };
                let content = match tool_input.get("content").and_then(|v| v.as_str()) {
                    Some(c) => c,
                    None => return Vec::new(),
                };
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let hash = format!("{:x}", hasher.finalize());
                let diff_text = content
                    .lines()
                    .map(|line| format!("+{}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                vec![ExtractedFileChange {
                    file_path,
                    change_type: "create".to_string(),
                    diff_text: Some(diff_text),
                    content_hash: Some(hash),
                }]
            }
            "Edit" => {
                let file_path = match tool_input.get("file_path").and_then(|v| v.as_str()) {
                    Some(p) => p.to_string(),
                    None => return Vec::new(),
                };
                let old_string = match tool_input.get("old_string").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return Vec::new(),
                };
                let new_string = match tool_input.get("new_string").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return Vec::new(),
                };
                let diff_text = format!("--- {}\n+++ {}", old_string, new_string);
                vec![ExtractedFileChange {
                    file_path,
                    change_type: "edit".to_string(),
                    diff_text: Some(diff_text),
                    content_hash: None,
                }]
            }
            _ => Vec::new(),
        }
    }

    fn extract_token_usage(&self, chunk: &serde_json::Value) -> Option<TokenUsage> {
        let usage = chunk.get("message")?.get("usage")?;
        Some(TokenUsage {
            input_tokens: usage.get("input_tokens")?.as_i64()?,
            output_tokens: usage.get("output_tokens")?.as_i64()?,
            cache_read_tokens: usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            cache_write_tokens: usage
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        })
    }

    fn extract_model(&self, chunk: &serde_json::Value) -> Option<String> {
        chunk
            .get("message")?
            .get("model")?
            .as_str()
            .map(|s| s.to_string())
    }

    fn parse_transcript_record(&self, chunk: &serde_json::Value) -> Option<ParsedTranscriptRecord> {
        let record_type = chunk.get("type")?.as_str()?.to_string();
        let timestamp = chunk
            .get("timestamp")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match record_type.as_str() {
            "assistant" => self.parse_assistant_record(chunk, record_type, timestamp),
            "user" => self.parse_user_record(chunk, record_type, timestamp),
            "progress" => self.parse_progress_record(chunk, record_type, timestamp),
            "system" => self.parse_system_record(chunk, record_type, timestamp),
            _ => Some(ParsedTranscriptRecord {
                record_type,
                timestamp,
                content_types: Vec::new(),
                tool_name: None,
                text: None,
                raw_input_tokens: None,
                raw_output_tokens: None,
                raw_cache_read_tokens: None,
                raw_cache_write_tokens: None,
                model: None,
            }),
        }
    }
}

impl ClaudeCodeAdapter {
    fn parse_assistant_record(
        &self,
        chunk: &serde_json::Value,
        record_type: String,
        timestamp: Option<String>,
    ) -> Option<ParsedTranscriptRecord> {
        let message = chunk.get("message")?;
        let model = message
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut content_types = Vec::new();
        let mut text_parts = Vec::new();
        let mut first_tool_name: Option<String> = None;

        if let Some(content) = message.get("content").and_then(|v| v.as_array()) {
            for block in content {
                if let Some(block_type) = block.get("type").and_then(|v| v.as_str()) {
                    if !content_types.contains(&block_type.to_string()) {
                        content_types.push(block_type.to_string());
                    }
                    match block_type {
                        "text" => {
                            if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                                text_parts.push(t.to_string());
                            }
                        }
                        "thinking" => {
                            if let Some(t) = block.get("thinking").and_then(|v| v.as_str()) {
                                text_parts.push(t.to_string());
                            }
                        }
                        "tool_use" => {
                            if first_tool_name.is_none() {
                                first_tool_name = block
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        let usage = message.get("usage");
        let raw_input_tokens = usage
            .and_then(|u| u.get("input_tokens"))
            .and_then(|v| v.as_i64());
        let raw_output_tokens = usage
            .and_then(|u| u.get("output_tokens"))
            .and_then(|v| v.as_i64());
        let raw_cache_read_tokens = usage
            .and_then(|u| u.get("cache_read_input_tokens"))
            .and_then(|v| v.as_i64());
        let raw_cache_write_tokens = usage
            .and_then(|u| u.get("cache_creation_input_tokens"))
            .and_then(|v| v.as_i64());

        let text = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        };

        Some(ParsedTranscriptRecord {
            record_type,
            timestamp,
            content_types,
            tool_name: first_tool_name,
            text,
            raw_input_tokens,
            raw_output_tokens,
            raw_cache_read_tokens,
            raw_cache_write_tokens,
            model,
        })
    }

    fn parse_user_record(
        &self,
        chunk: &serde_json::Value,
        record_type: String,
        timestamp: Option<String>,
    ) -> Option<ParsedTranscriptRecord> {
        let mut content_types = Vec::new();
        let mut text_parts = Vec::new();
        let mut tool_name: Option<String> = None;

        // Check for toolUseResult (e.g. Read, Glob, Bash results)
        if let Some(tool_result) = chunk.get("toolUseResult") {
            if let Some(file_info) = tool_result.get("file") {
                let file_path = file_info
                    .get("filePath")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                tool_name = Some(format!("Read: {}", file_path));
            } else if let Some(glob_info) = tool_result.get("glob") {
                let pattern = glob_info
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                tool_name = Some(format!("Glob: {}", pattern));
            } else if let Some(bash_info) = tool_result.get("bash") {
                let command = bash_info
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                tool_name = Some(format!("Bash: {}", command));
            }
        }

        // Handle message.content as either a string or an array
        if let Some(message) = chunk.get("message") {
            if let Some(content) = message.get("content") {
                if let Some(text) = content.as_str() {
                    text_parts.push(text.to_string());
                    content_types.push("text".to_string());
                } else if let Some(arr) = content.as_array() {
                    for block in arr {
                        if let Some(block_type) = block.get("type").and_then(|v| v.as_str()) {
                            if !content_types.contains(&block_type.to_string()) {
                                content_types.push(block_type.to_string());
                            }
                            match block_type {
                                "tool_result" | "text" => {
                                    if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                                        text_parts.push(t.to_string());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        let text = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join("\n"))
        };

        Some(ParsedTranscriptRecord {
            record_type,
            timestamp,
            content_types,
            tool_name,
            text,
            raw_input_tokens: None,
            raw_output_tokens: None,
            raw_cache_read_tokens: None,
            raw_cache_write_tokens: None,
            model: None,
        })
    }

    fn parse_progress_record(
        &self,
        chunk: &serde_json::Value,
        record_type: String,
        timestamp: Option<String>,
    ) -> Option<ParsedTranscriptRecord> {
        let data = chunk.get("data");
        let hook_name = data
            .and_then(|d| d.get("hookName"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let hook_event = data
            .and_then(|d| d.get("hookEvent"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let text = format!("{}: {}", hook_event, hook_name);

        Some(ParsedTranscriptRecord {
            record_type,
            timestamp,
            content_types: Vec::new(),
            tool_name: None,
            text: Some(text),
            raw_input_tokens: None,
            raw_output_tokens: None,
            raw_cache_read_tokens: None,
            raw_cache_write_tokens: None,
            model: None,
        })
    }

    fn parse_system_record(
        &self,
        chunk: &serde_json::Value,
        record_type: String,
        timestamp: Option<String>,
    ) -> Option<ParsedTranscriptRecord> {
        let subtype = chunk.get("subtype").and_then(|v| v.as_str());

        let text = match subtype {
            Some("turn_duration") => {
                let duration_ms = chunk
                    .get("durationMs")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let seconds = duration_ms / 1000.0;
                Some(format!("Turn duration: {:.1}s", seconds))
            }
            Some("stop_hook_summary") => {
                let hook_count = chunk.get("hookCount").and_then(|v| v.as_i64()).unwrap_or(0);
                Some(format!("Stop hooks executed: {}", hook_count))
            }
            _ => None,
        };

        Some(ParsedTranscriptRecord {
            record_type,
            timestamp,
            content_types: Vec::new(),
            tool_name: None,
            text,
            raw_input_tokens: None,
            raw_output_tokens: None,
            raw_cache_read_tokens: None,
            raw_cache_write_tokens: None,
            model: None,
        })
    }
}
