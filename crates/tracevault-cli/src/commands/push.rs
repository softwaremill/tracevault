use crate::api_client::{resolve_credentials, ApiClient, PushTraceRequest};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;

struct GitInfo {
    repo_name: String,
    branch: Option<String>,
    commit_sha: String,
    author: String,
}

fn git_info(project_root: &Path) -> GitInfo {
    let run = |args: &[&str]| -> Option<String> {
        Command::new("git")
            .args(args)
            .current_dir(project_root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let repo_name = run(&["rev-parse", "--show-toplevel"])
        .as_deref()
        .and_then(|p| p.rsplit('/').next())
        .map(String::from)
        .unwrap_or_else(|| "unknown".into());

    let branch = run(&["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|b| b != "HEAD");

    let commit_sha = run(&["rev-parse", "HEAD"])
        .unwrap_or_else(|| "unknown".into());

    let author = run(&["config", "user.name"])
        .unwrap_or_else(|| "unknown".into());

    GitInfo { repo_name, branch, commit_sha, author }
}

struct SessionSummary {
    event_count: usize,
    files_modified: Vec<String>,
    tools_used: HashSet<String>,
    models: HashSet<String>,
    events: Vec<serde_json::Value>,
}

fn summarize_session(session_dir: &Path) -> Option<SessionSummary> {
    let events_path = session_dir.join("events.jsonl");
    if !events_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&events_path).ok()?;
    let mut files_modified = Vec::new();
    let mut files_seen = HashSet::new();
    let mut tools_used = HashSet::new();
    let mut models = HashSet::new();
    let mut events = Vec::new();

    for line in content.lines() {
        let event: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(tool) = event.get("tool_name").and_then(|v| v.as_str()) {
            tools_used.insert(tool.to_string());
        }

        if let Some(model) = event.get("model").and_then(|v| v.as_str()) {
            models.insert(model.to_string());
        }

        // Track unique file modifications
        if let Some(path) = event.get("tool_input")
            .and_then(|v| v.get("file_path"))
            .and_then(|v| v.as_str())
        {
            if files_seen.insert(path.to_string()) {
                files_modified.push(path.to_string());
            }
        }

        events.push(event);
    }

    Some(SessionSummary {
        event_count: events.len(),
        files_modified,
        tools_used,
        models,
        events,
    })
}

pub async fn push_traces(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let (server_url, token) = resolve_credentials(project_root);

    let server_url = match server_url {
        Some(url) => url,
        None => {
            eprintln!("No server URL configured. Skipping push.");
            return Ok(());
        }
    };

    if token.is_none() {
        eprintln!("Not logged in. Run 'tracevault login' to push traces.");
        return Ok(());
    }

    let client = ApiClient::new(&server_url, token.as_deref());

    let sessions_dir = project_root.join(".tracevault").join("sessions");
    if !sessions_dir.exists() {
        println!("No sessions to push.");
        return Ok(());
    }

    let git = git_info(project_root);
    let mut pushed = 0;
    let mut failed = 0;

    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let session_dir = entry.path();
        let pushed_marker = session_dir.join(".pushed");
        if pushed_marker.exists() {
            continue; // already pushed
        }

        let summary = match summarize_session(&session_dir) {
            Some(s) if s.event_count > 0 => s,
            _ => continue,
        };

        let meta_path = session_dir.join("metadata.json");
        let metadata: Option<serde_json::Value> = meta_path
            .exists()
            .then(|| fs::read_to_string(&meta_path).ok())
            .flatten()
            .and_then(|c| serde_json::from_str(&c).ok());

        let session_data = serde_json::json!({
            "session_id": entry.file_name().to_string_lossy(),
            "metadata": metadata,
            "event_count": summary.event_count,
            "files_modified": summary.files_modified,
            "tools_used": summary.tools_used.iter().collect::<Vec<_>>(),
            "events": summary.events,
        });

        let model = summary.models.iter().next().cloned();

        let req = PushTraceRequest {
            repo_name: git.repo_name.clone(),
            commit_sha: git.commit_sha.clone(),
            branch: git.branch.clone(),
            author: git.author.clone(),
            model,
            tool: Some("claude-code".into()),
            ai_percentage: None,
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            estimated_cost_usd: None,
            api_calls: Some(summary.event_count as i32),
            session_data: Some(session_data),
            attribution: None,
        };

        let session_name = entry.file_name().to_string_lossy().to_string();
        match client.push_trace(req).await {
            Ok(resp) => {
                println!(
                    "Pushed session {} ({} events, {} files) -> {}",
                    session_name,
                    summary.event_count,
                    summary.files_modified.len(),
                    resp.id,
                );
                // Mark as pushed so we don't push again
                fs::write(&pushed_marker, resp.id.to_string())?;
                pushed += 1;
            }
            Err(e) => {
                eprintln!("Failed to push {session_name}: {e}");
                failed += 1;
            }
        }
    }

    if pushed > 0 || failed > 0 {
        println!("\nPushed {pushed} session(s), {failed} failed.");
    } else {
        println!("No new sessions to push.");
    }

    Ok(())
}
