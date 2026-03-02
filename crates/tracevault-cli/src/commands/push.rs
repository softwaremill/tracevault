use crate::api_client::{resolve_credentials, ApiClient, PushTraceRequest};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracevault_core::diff::parse_unified_diff;
use tracevault_core::gitai::{gitai_to_attribution, parse_gitai_note};

struct GitInfo {
    repo_name: String,
    branch: Option<String>,
    head_sha: String,
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

    let head_sha = run(&["rev-parse", "HEAD"])
        .unwrap_or_else(|| "unknown".into());

    GitInfo { repo_name, branch, head_sha }
}

fn get_commit_author(project_root: &Path, commit_sha: &str) -> String {
    Command::new("git")
        .args(["log", "-1", "--format=%aN", commit_sha])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn last_pushed_sha_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".tracevault")
        .join("cache")
        .join(".last_pushed_sha")
}

fn read_last_pushed_sha(project_root: &Path) -> Option<String> {
    fs::read_to_string(last_pushed_sha_path(project_root))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_last_pushed_sha(project_root: &Path, sha: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = last_pushed_sha_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, sha)?;
    Ok(())
}

/// Returns commit SHAs in chronological order (oldest first) that haven't been pushed yet.
fn get_unpushed_commits(project_root: &Path, last_pushed: Option<&str>, head_sha: &str) -> Vec<String> {
    let last_pushed = match last_pushed {
        Some(sha) => sha,
        None => return vec![head_sha.to_string()], // First push: just HEAD
    };

    if last_pushed == head_sha {
        return vec![]; // No new commits
    }

    // Verify last_pushed SHA still exists in history (handles rebase/force-push)
    let exists = Command::new("git")
        .args(["cat-file", "-t", last_pushed])
        .current_dir(project_root)
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !exists {
        return vec![head_sha.to_string()]; // Fallback: SHA gone after rebase
    }

    // Get all commits between last_pushed and HEAD, oldest first
    let output = Command::new("git")
        .args(["rev-list", "--reverse", &format!("{last_pushed}..HEAD")])
        .current_dir(project_root)
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => {
            let shas: Vec<String> = String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if shas.is_empty() {
                vec![]
            } else {
                shas
            }
        }
        _ => vec![head_sha.to_string()], // Fallback
    }
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

struct ModelTokens {
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
    requests: i64,
}

struct TranscriptData {
    transcript: Option<serde_json::Value>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
    model_usage: Option<serde_json::Value>,
}

fn accumulate_usage(model_tokens: &mut HashMap<String, ModelTokens>, model: &str, usage: &serde_json::Value) {
    let entry = model_tokens.entry(model.to_string()).or_insert(ModelTokens {
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        requests: 0,
    });
    entry.requests += 1;
    if let Some(n) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
        entry.input_tokens += n;
    }
    if let Some(n) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
        entry.output_tokens += n;
    }
    if let Some(n) = usage.get("cache_read_input_tokens").and_then(|v| v.as_i64()) {
        entry.cache_read_tokens += n;
    }
    if let Some(n) = usage.get("cache_creation_input_tokens").and_then(|v| v.as_i64()) {
        entry.cache_creation_tokens += n;
    }
}

fn extract_usage_from_message(model_tokens: &mut HashMap<String, ModelTokens>, message: &serde_json::Value) {
    let model = message.get("model").and_then(|v| v.as_str()).unwrap_or("unknown");
    if let Some(usage) = message.get("usage") {
        accumulate_usage(model_tokens, model, usage);
    }
}

fn extract_nested_usage(model_tokens: &mut HashMap<String, ModelTokens>, entry: &serde_json::Value) {
    // Handle subagent progress messages nested in content blocks:
    // entry.message.content[].data.message (where type == "progress" or data.type == "agent_progress")
    let content = match entry.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return,
    };
    for block in content {
        // Look for tool_result or progress blocks that contain nested assistant messages
        if let Some(data) = block.get("data") {
            let data_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if data_type == "progress" || data_type == "agent_progress" {
                if let Some(msg) = data.get("message") {
                    extract_usage_from_message(model_tokens, msg);
                }
            }
        }
    }
}

fn read_transcript(metadata: &Option<serde_json::Value>) -> TranscriptData {
    let empty = TranscriptData {
        transcript: None,
        model: None,
        input_tokens: None,
        output_tokens: None,
        total_tokens: None,
        model_usage: None,
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
    let mut model_tokens: HashMap<String, ModelTokens> = HashMap::new();

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

            // Per-model breakdown from top-level assistant message
            if let Some(message) = entry.get("message") {
                extract_usage_from_message(&mut model_tokens, message);
            }

            // Also check for nested subagent messages
            extract_nested_usage(&mut model_tokens, &entry);
        }

        lines.push(entry);
    }

    if lines.is_empty() {
        return empty;
    }

    // Primary model = most requests
    let model = model_tokens
        .iter()
        .max_by_key(|(_, t)| t.requests)
        .map(|(name, _)| name.clone());

    let total = total_input + total_output;

    // Build model_usage JSON array
    let model_usage = if model_tokens.is_empty() {
        None
    } else {
        let arr: Vec<serde_json::Value> = model_tokens
            .into_iter()
            .map(|(name, t)| {
                serde_json::json!({
                    "model": name,
                    "input_tokens": t.input_tokens,
                    "output_tokens": t.output_tokens,
                    "cache_read_tokens": t.cache_read_tokens,
                    "cache_creation_tokens": t.cache_creation_tokens,
                    "requests": t.requests,
                })
            })
            .collect();
        Some(serde_json::Value::Array(arr))
    };

    TranscriptData {
        transcript: Some(serde_json::Value::Array(lines)),
        model,
        input_tokens: if total > 0 { Some(total_input) } else { None },
        output_tokens: if total > 0 { Some(total_output) } else { None },
        total_tokens: if total > 0 { Some(total) } else { None },
        model_usage,
    }
}

fn read_git_diff(project_root: &Path, commit_sha: &str) -> Option<Vec<tracevault_core::diff::FileDiff>> {
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
    Some(parse_unified_diff(&raw))
}

fn read_gitai_attribution(
    project_root: &Path,
    commit_sha: &str,
    diff_files: &[tracevault_core::diff::FileDiff],
) -> Option<serde_json::Value> {
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
    let attribution = gitai_to_attribution(&log, diff_files);
    serde_json::to_value(&attribution).ok()
}

pub async fn push_traces(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let (server_url, token) = resolve_credentials(project_root);

    let server_url = match server_url {
        Some(url) => url,
        None => {
            return Err("No server URL configured. Run 'tracevault login' first.".into());
        }
    };

    if token.is_none() {
        return Err("Not logged in. Run 'tracevault login' to push traces.".into());
    }

    let client = ApiClient::new(&server_url, token.as_deref());

    let sessions_dir = project_root.join(".tracevault").join("sessions");

    let git = git_info(project_root);

    // Step 1: Discover and register all unpushed commits
    let last_pushed = read_last_pushed_sha(project_root);
    let unpushed = get_unpushed_commits(project_root, last_pushed.as_deref(), &git.head_sha);

    let mut commits_registered = 0;
    for sha in &unpushed {
        let author = get_commit_author(project_root, sha);
        let diff_files = read_git_diff(project_root, sha);
        let diff_data = diff_files
            .as_ref()
            .and_then(|f| serde_json::to_value(f).ok());
        let attribution = read_gitai_attribution(
            project_root,
            sha,
            diff_files.as_deref().unwrap_or(&[]),
        );

        let commit_req = PushTraceRequest {
            repo_name: git.repo_name.clone(),
            commit_sha: sha.clone(),
            branch: git.branch.clone(),
            author,
            model: None,
            tool: None,
            session_id: None,
            total_tokens: None,
            input_tokens: None,
            output_tokens: None,
            estimated_cost_usd: None,
            api_calls: None,
            session_data: None,
            attribution,
            transcript: None,
            diff_data,
            model_usage: None,
        };

        let commit_resp = client.push_trace(commit_req).await
            .map_err(|e| format!("Failed to register commit {}: {e}", &sha[..8.min(sha.len())]))?;
        println!("Registered commit {} -> {}", &sha[..8.min(sha.len())], commit_resp.commit_id);
        commits_registered += 1;
    }

    if unpushed.is_empty() {
        println!("No new commits to register.");
    }

    // Step 2: Push each unpushed session (attached to HEAD)
    let mut pushed = 0;
    let mut failed = 0;

    if sessions_dir.exists() {
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

            // Prefer model from transcript, fall back to events
            let model = transcript_data.model
                .or_else(|| summary.models.iter().next().cloned());

            let session_name = entry.file_name().to_string_lossy().to_string();
            let author = get_commit_author(project_root, &git.head_sha);

            let req = PushTraceRequest {
                repo_name: git.repo_name.clone(),
                commit_sha: git.head_sha.clone(),
                branch: git.branch.clone(),
                author,
                model,
                tool: Some("claude-code".into()),
                session_id: Some(session_name.clone()),
                total_tokens: transcript_data.total_tokens,
                input_tokens: transcript_data.input_tokens,
                output_tokens: transcript_data.output_tokens,
                estimated_cost_usd: None,
                api_calls: Some(summary.event_count as i32),
                session_data: Some(session_data),
                attribution: None, // commit-level only
                transcript: transcript_data.transcript,
                diff_data: None,   // commit-level only
                model_usage: transcript_data.model_usage,
            };

            match client.push_trace(req).await {
                Ok(resp) => {
                    println!(
                        "Pushed session {} ({} events, {} files) -> {}",
                        session_name,
                        summary.event_count,
                        summary.files_modified.len(),
                        resp.commit_id,
                    );
                    // Mark as pushed so we don't push again
                    fs::write(&pushed_marker, resp.commit_id.to_string())?;
                    pushed += 1;
                }
                Err(e) => {
                    eprintln!("Failed to push {session_name}: {e}");
                    failed += 1;
                }
            }
        }
    }

    if pushed > 0 || failed > 0 {
        println!("\nPushed {pushed} session(s), {failed} failed.");
    } else if sessions_dir.exists() {
        println!("No new sessions to push.");
    }

    if failed > 0 {
        return Err(format!("{failed} session(s) failed to push").into());
    }

    // Only update last_pushed_sha after everything succeeds
    if commits_registered > 0 || pushed > 0 {
        write_last_pushed_sha(project_root, &git.head_sha)?;
    }

    Ok(())
}
