use std::fs;
use tempfile::TempDir;

fn tmp_git_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();
    tmp
}

#[tokio::test]
async fn full_flow_init_hook_and_local_stats() {
    let tmp = tmp_git_repo();

    // 1. Init
    tracevault_cli::commands::init::init_in_directory(tmp.path(), None)
        .await
        .unwrap();
    assert!(tmp.path().join(".tracevault/config.toml").exists());
    assert!(tmp.path().join(".tracevault/sessions").exists());
    assert!(tmp.path().join(".tracevault/cache").exists());

    // 2. Simulate multiple hook events (a coding session)
    let events = vec![
        serde_json::json!({
            "session_id": "e2e-session-001",
            "transcript_path": "/tmp/transcript.jsonl",
            "cwd": tmp.path().to_str().unwrap(),
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "tool_input": {
                "file_path": tmp.path().join("src/main.rs").to_str().unwrap(),
                "content": "fn main() { println!(\"hello\"); }"
            }
        }),
        serde_json::json!({
            "session_id": "e2e-session-001",
            "transcript_path": "/tmp/transcript.jsonl",
            "cwd": tmp.path().to_str().unwrap(),
            "hook_event_name": "PostToolUse",
            "tool_name": "Write",
            "tool_input": {
                "file_path": tmp.path().join("src/main.rs").to_str().unwrap(),
                "content": "fn main() { println!(\"hello\"); }"
            },
            "tool_response": { "success": true }
        }),
        serde_json::json!({
            "session_id": "e2e-session-001",
            "transcript_path": "/tmp/transcript.jsonl",
            "cwd": tmp.path().to_str().unwrap(),
            "hook_event_name": "PostToolUse",
            "tool_name": "Edit",
            "tool_input": {
                "file_path": tmp.path().join("src/lib.rs").to_str().unwrap(),
                "old_string": "old code",
                "new_string": "new code"
            },
            "tool_response": { "success": true }
        }),
    ];

    for event in &events {
        tracevault_cli::commands::hook::handle_hook_event(
            &event.to_string(),
            tmp.path(),
        )
        .unwrap();
    }

    // 3. Verify session data was captured
    let session_dir = tmp.path().join(".tracevault/sessions/e2e-session-001");
    assert!(session_dir.exists());
    assert!(session_dir.join("events.jsonl").exists());
    assert!(session_dir.join("metadata.json").exists());

    // 4. Verify events.jsonl has 3 entries
    let events_content = fs::read_to_string(session_dir.join("events.jsonl")).unwrap();
    let event_count = events_content.lines().count();
    assert_eq!(event_count, 3, "Expected 3 events, got {}", event_count);

    // 5. Verify events contain correct data
    assert!(events_content.contains("PreToolUse"));
    assert!(events_content.contains("PostToolUse"));
    assert!(events_content.contains("Write"));
    assert!(events_content.contains("Edit"));
    assert!(events_content.contains("src/main.rs"));

    // 6. Verify metadata
    let metadata: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(session_dir.join("metadata.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(metadata["session_id"], "e2e-session-001");
}

#[tokio::test]
async fn multiple_sessions_tracked_independently() {
    let tmp = tmp_git_repo();
    tracevault_cli::commands::init::init_in_directory(tmp.path(), None)
        .await
        .unwrap();

    // Session 1
    let event1 = serde_json::json!({
        "session_id": "session-aaa",
        "transcript_path": "/tmp/t1.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": { "file_path": "a.rs", "content": "a" }
    });

    // Session 2
    let event2 = serde_json::json!({
        "session_id": "session-bbb",
        "transcript_path": "/tmp/t2.jsonl",
        "cwd": tmp.path().to_str().unwrap(),
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": { "file_path": "b.rs", "content": "b" }
    });

    tracevault_cli::commands::hook::handle_hook_event(&event1.to_string(), tmp.path()).unwrap();
    tracevault_cli::commands::hook::handle_hook_event(&event2.to_string(), tmp.path()).unwrap();

    // Both sessions should have their own directories
    assert!(
        tmp.path()
            .join(".tracevault/sessions/session-aaa/events.jsonl")
            .exists()
    );
    assert!(
        tmp.path()
            .join(".tracevault/sessions/session-bbb/events.jsonl")
            .exists()
    );
}
