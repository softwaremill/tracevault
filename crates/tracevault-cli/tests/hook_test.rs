use std::fs;
use tempfile::TempDir;

#[test]
fn hook_handler_records_event_to_session_dir() {
    let tmp = TempDir::new().unwrap();

    // Set up .tracevault/sessions/
    fs::create_dir_all(tmp.path().join(".tracevault/sessions")).unwrap();

    let hook_json = serde_json::json!({
        "session_id": "test-session-123",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": {
            "file_path": tmp.path().join("src/main.rs").to_str().unwrap(),
            "content": "fn main() {}"
        }
    });

    let result = tracevault_cli::commands::hook::handle_hook_event(
        &hook_json.to_string(),
        tmp.path(),
    );
    assert!(result.is_ok());

    // Check that event was recorded
    let session_dir = tmp.path().join(".tracevault/sessions/test-session-123");
    assert!(session_dir.join("events.jsonl").exists());
    assert!(session_dir.join("metadata.json").exists());

    // Verify events.jsonl content
    let events_content = fs::read_to_string(session_dir.join("events.jsonl")).unwrap();
    assert!(events_content.contains("test-session-123"));
    assert!(events_content.contains("PostToolUse"));
    assert!(events_content.contains("Write"));

    // Verify metadata.json content
    let metadata_content = fs::read_to_string(session_dir.join("metadata.json")).unwrap();
    let metadata: serde_json::Value = serde_json::from_str(&metadata_content).unwrap();
    assert_eq!(metadata["session_id"], "test-session-123");
    assert!(metadata["started_at"].as_str().is_some());
}

#[test]
fn hook_handler_appends_multiple_events() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join(".tracevault/sessions")).unwrap();

    let event1 = serde_json::json!({
        "session_id": "multi-event-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": { "file_path": "/tmp/a.rs", "content": "a" }
    });

    let event2 = serde_json::json!({
        "session_id": "multi-event-session",
        "transcript_path": "/tmp/transcript.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "tool_input": { "file_path": "/tmp/b.rs", "old_string": "x", "new_string": "y" }
    });

    tracevault_cli::commands::hook::handle_hook_event(&event1.to_string(), tmp.path()).unwrap();
    tracevault_cli::commands::hook::handle_hook_event(&event2.to_string(), tmp.path()).unwrap();

    let events_content = fs::read_to_string(
        tmp.path().join(".tracevault/sessions/multi-event-session/events.jsonl"),
    )
    .unwrap();

    let lines: Vec<&str> = events_content.lines().collect();
    assert_eq!(lines.len(), 2);
}
