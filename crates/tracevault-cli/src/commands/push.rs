use crate::api_client::{resolve_credentials, ApiClient, PushTraceRequest};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use tracevault_core::diff::parse_unified_diff;
use tracevault_core::gitai::{gitai_to_attribution, parse_gitai_note};

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

struct TranscriptData {
    transcript: Option<serde_json::Value>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

fn read_transcript(metadata: &Option<serde_json::Value>) -> TranscriptData {
    let empty = TranscriptData {
        transcript: None,
        model: None,
        input_tokens: None,
        output_tokens: None,
        total_tokens: None,
    };

    let transcript_path = metadata
        .as_ref()
        .and_then(|m| m.get("transcript_path"))
        .and_then(|v| v.as_str());

    let path = match transcript_path {
        Some(p) => std::path::PathBuf::from(p),
        None => return empty,
    };

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return empty,
    };

    let mut lines: Vec<serde_json::Value> = Vec::new();
    let mut total_input: i64 = 0;
    let mut total_output: i64 = 0;
    let mut model_counts: HashMap<String, usize> = HashMap::new();

    for line in content.lines() {
        let entry: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Extract token usage from assistant messages
        if entry.get("type").and_then(|v| v.as_str()) == Some("assistant") {
            if let Some(usage) = entry.get("message").and_then(|m| m.get("usage")) {
                if let Some(n) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                    total_input += n;
                }
                if let Some(n) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                    total_output += n;
                }
                if let Some(n) = usage.get("cache_creation_input_tokens").and_then(|v| v.as_i64()) {
                    total_input += n;
                }
                if let Some(n) = usage.get("cache_read_input_tokens").and_then(|v| v.as_i64()) {
                    total_input += n;
                }
            }
        }

        // Collect model IDs
        if let Some(model) = entry.get("message").and_then(|m| m.get("model")).and_then(|v| v.as_str()) {
            *model_counts.entry(model.to_string()).or_insert(0) += 1;
        }

        lines.push(entry);
    }

    if lines.is_empty() {
        return empty;
    }

    let model = model_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(name, _)| name);

    let total = total_input + total_output;

    TranscriptData {
        transcript: Some(serde_json::Value::Array(lines)),
        model,
        input_tokens: if total > 0 { Some(total_input) } else { None },
        output_tokens: if total > 0 { Some(total_output) } else { None },
        total_tokens: if total > 0 { Some(total) } else { None },
    }
}

fn read_git_diff(project_root: &Path, commit_sha: &str) -> Option<serde_json::Value> {
    let output = Command::new("git")
        .args(["diff", &format!("{commit_sha}~1..{commit_sha}")])
        .current_dir(project_root)
        .output()
        .ok()?;

    let raw = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        // May fail for initial commit — try diffing against empty tree
        let output = Command::new("git")
            .args([
                "diff",
                "4b825dc642cb6eb9a060e54bf899d69f245df2c1",
                commit_sha,
            ])
            .current_dir(project_root)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    if raw.is_empty() {
        return None;
    }
    let files = parse_unified_diff(&raw);
    serde_json::to_value(&files).ok()
}

fn read_gitai_attribution(project_root: &Path, commit_sha: &str) -> Option<serde_json::Value> {
    let output = Command::new("git")
        .args(["notes", "--ref", "refs/notes/ai", "show", commit_sha])
        .current_dir(project_root)
        .output()
        .ok()?;

    if !output.status.success() {
        return None; // git-ai not installed or no note for this commit
    }

    let note = String::from_utf8_lossy(&output.stdout);
    let log = parse_gitai_note(&note)?;
    let attribution = gitai_to_attribution(&log);
    serde_json::to_value(&attribution).ok()
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

        let transcript_data = read_transcript(&metadata);
        let diff_data = read_git_diff(project_root, &git.commit_sha);
        let attribution = read_gitai_attribution(project_root, &git.commit_sha);

        // Prefer model from transcript, fall back to events
        let model = transcript_data.model
            .or_else(|| summary.models.iter().next().cloned());

        let req = PushTraceRequest {
            repo_name: git.repo_name.clone(),
            commit_sha: git.commit_sha.clone(),
            branch: git.branch.clone(),
            author: git.author.clone(),
            model,
            tool: Some("claude-code".into()),
            ai_percentage: None,
            total_tokens: transcript_data.total_tokens,
            input_tokens: transcript_data.input_tokens,
            output_tokens: transcript_data.output_tokens,
            estimated_cost_usd: None,
            api_calls: Some(summary.event_count as i32),
            session_data: Some(session_data),
            attribution,
            transcript: transcript_data.transcript,
            diff_data,
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
