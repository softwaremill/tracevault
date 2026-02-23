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
