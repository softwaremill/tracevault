use crate::api_client::{resolve_credentials, ApiClient};
use crate::config::TracevaultConfig;
use serde_json::json;
use std::path::Path;
use std::process::Command;
use tracevault_core::streaming::CommitPushRequest;

pub async fn run_commit_push(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = TracevaultConfig::load(project_root).ok_or("config not found")?;
    let org_slug = config.org_slug.ok_or("org_slug not configured")?;
    let repo_id = config.repo_id.ok_or("repo_id not configured")?;

    let (server_url, token) = resolve_credentials(project_root);
    let server_url = server_url.ok_or("server_url not configured")?;
    let client = ApiClient::new(&server_url, token.as_deref());

    // Gather git info
    let run_git = |args: &[&str]| -> Option<String> {
        Command::new("git")
            .args(args)
            .current_dir(project_root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    };

    let commit_sha = run_git(&["rev-parse", "HEAD"]).ok_or("failed to get HEAD sha")?;
    let branch = run_git(&["rev-parse", "--abbrev-ref", "HEAD"]);
    let author = run_git(&["log", "-1", "--format=%ae"]).unwrap_or_default();

    // Get diff (ok if fails, e.g. initial commit)
    let diff_output = run_git(&["diff", "HEAD~1..HEAD", "--unified=3"]);

    let diff_data = diff_output.map(|diff| parse_diff_to_json(&diff));

    let req = CommitPushRequest {
        commit_sha,
        branch,
        author,
        diff_data,
        committed_at: Some(chrono::Utc::now()),
    };

    match client.push_commit(&org_slug, &repo_id, &req).await {
        Ok(resp) => {
            println!(
                "Commit pushed: {} ({} attributions)",
                resp.commit_db_id, resp.attributions_count
            );
        }
        Err(e) => {
            eprintln!("Warning: commit push failed: {e}");
            // Don't fail — post-commit hook should not block
        }
    }

    Ok(())
}

fn parse_diff_to_json(diff: &str) -> serde_json::Value {
    let mut files: Vec<serde_json::Value> = Vec::new();
    let mut current_file: Option<String> = None;
    let mut current_hunks: Vec<serde_json::Value> = Vec::new();
    let mut current_hunk_lines: Vec<String> = Vec::new();
    let mut current_new_start: i64 = 0;
    let mut current_new_count: i64 = 0;

    let flush_hunk =
        |hunks: &mut Vec<serde_json::Value>, lines: &mut Vec<String>, start: i64, count: i64| {
            if !lines.is_empty() {
                hunks.push(json!({
                    "new_start": start,
                    "new_count": count,
                    "added_lines": lines.clone(),
                }));
                lines.clear();
            }
        };

    let flush_file = |files: &mut Vec<serde_json::Value>,
                      file: &Option<String>,
                      hunks: &mut Vec<serde_json::Value>| {
        if let Some(path) = file {
            if !hunks.is_empty() {
                files.push(json!({
                    "path": path,
                    "hunks": hunks.clone(),
                }));
                hunks.clear();
            }
        }
    };

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ b/") {
            // Flush previous hunk and file
            flush_hunk(
                &mut current_hunks,
                &mut current_hunk_lines,
                current_new_start,
                current_new_count,
            );
            flush_file(&mut files, &current_file, &mut current_hunks);
            current_file = Some(path.to_string());
        } else if line.starts_with("@@ ") {
            // Flush previous hunk
            flush_hunk(
                &mut current_hunks,
                &mut current_hunk_lines,
                current_new_start,
                current_new_count,
            );
            // Parse @@ -old_start,old_count +new_start,new_count @@
            if let Some(plus_part) = line.split('+').nth(1) {
                let nums: Vec<&str> = plus_part
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .split(',')
                    .collect();
                current_new_start = nums.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                current_new_count = nums.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            }
        } else if let Some(added) = line.strip_prefix('+') {
            current_hunk_lines.push(added.to_string());
        }
    }

    // Flush remaining
    flush_hunk(
        &mut current_hunks,
        &mut current_hunk_lines,
        current_new_start,
        current_new_count,
    );
    flush_file(&mut files, &current_file, &mut current_hunks);

    json!({ "files": files })
}
