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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_hook_event(
        hook_event_name: &str,
        tool_name: Option<&str>,
        tool_input: Option<serde_json::Value>,
    ) -> HookEvent {
        HookEvent {
            session_id: "sess-123".to_string(),
            transcript_path: "/tmp/transcript.json".to_string(),
            cwd: "/home/user/project".to_string(),
            permission_mode: None,
            hook_event_name: hook_event_name.to_string(),
            tool_name: tool_name.map(String::from),
            tool_input,
            tool_response: None,
            tool_use_id: None,
        }
    }

    #[test]
    fn new_initializes_correctly() {
        let event = make_hook_event("Init", None, None);
        let state = SessionState::new(&event);

        assert_eq!(state.session_id, "sess-123");
        assert_eq!(state.transcript_path, "/tmp/transcript.json");
        assert_eq!(state.cwd, "/home/user/project");
        assert!(state.events.is_empty());
        assert!(state.pre_edit_hashes.is_empty());
        assert!(state.ai_modified_files.is_empty());
    }

    #[test]
    fn record_event_appends() {
        let init = make_hook_event("Init", None, None);
        let mut state = SessionState::new(&init);

        let evt = make_hook_event("PreToolUse", Some("Read"), None);
        state.record_event(&evt);

        assert_eq!(state.events.len(), 1);
        assert_eq!(state.events[0].event_type, "PreToolUse");
        assert_eq!(state.events[0].tool_name.as_deref(), Some("Read"));
    }

    #[test]
    fn record_event_post_tool_use_file_mod_adds_to_ai_modified() {
        let init = make_hook_event("Init", None, None);
        let mut state = SessionState::new(&init);

        let evt = make_hook_event(
            "PostToolUse",
            Some("Write"),
            Some(json!({"file_path": "/tmp/test.rs"})),
        );
        state.record_event(&evt);

        assert_eq!(state.ai_modified_files, vec!["/tmp/test.rs"]);
    }

    #[test]
    fn record_event_pre_tool_use_file_mod_does_not_add() {
        let init = make_hook_event("Init", None, None);
        let mut state = SessionState::new(&init);

        let evt = make_hook_event(
            "PreToolUse",
            Some("Write"),
            Some(json!({"file_path": "/tmp/test.rs"})),
        );
        state.record_event(&evt);

        assert!(state.ai_modified_files.is_empty());
    }

    #[test]
    fn record_event_post_tool_use_non_file_does_not_add() {
        let init = make_hook_event("Init", None, None);
        let mut state = SessionState::new(&init);

        let evt = make_hook_event(
            "PostToolUse",
            Some("Read"),
            Some(json!({"file_path": "/tmp/test.rs"})),
        );
        state.record_event(&evt);

        assert!(state.ai_modified_files.is_empty());
    }

    #[test]
    fn record_pre_edit_hash_inserts() {
        let init = make_hook_event("Init", None, None);
        let mut state = SessionState::new(&init);

        state.record_pre_edit_hash("/tmp/test.rs", "abc123");

        assert_eq!(
            state.pre_edit_hashes.get("/tmp/test.rs"),
            Some(&"abc123".to_string())
        );
    }

    #[test]
    fn record_pre_edit_hash_does_not_overwrite() {
        let init = make_hook_event("Init", None, None);
        let mut state = SessionState::new(&init);

        state.record_pre_edit_hash("/tmp/test.rs", "first");
        state.record_pre_edit_hash("/tmp/test.rs", "second");

        assert_eq!(
            state.pre_edit_hashes.get("/tmp/test.rs"),
            Some(&"first".to_string())
        );
    }

    #[test]
    fn session_dir_returns_correct_path() {
        let init = make_hook_event("Init", None, None);
        let state = SessionState::new(&init);

        let dir = state.session_dir();
        assert!(dir.ends_with("sessions/sess-123"));
        assert_eq!(
            dir,
            PathBuf::from("/home/user/project/.tracevault/sessions/sess-123")
        );
    }
}
