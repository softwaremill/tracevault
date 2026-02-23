use crate::api_client::{ApiClient, PushTraceRequest};
use crate::config::TracevaultConfig;
use std::fs;
use std::path::Path;

pub async fn push_traces(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = TracevaultConfig::config_path(project_root);
    let config_content = fs::read_to_string(&config_path)?;

    // Parse minimal config to get server URL
    let server_url = config_content
        .lines()
        .find(|l| l.starts_with("server_url"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .unwrap_or_else(|| "http://localhost:3000".into());

    let client = ApiClient::new(&server_url, None);

    // Read pending sessions
    let sessions_dir = project_root.join(".tracevault").join("sessions");
    if !sessions_dir.exists() {
        println!("No pending sessions to push.");
        return Ok(());
    }

    let mut pushed = 0;
    for entry in fs::read_dir(&sessions_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let meta_path = entry.path().join("metadata.json");
            if meta_path.exists() {
                let meta: serde_json::Value =
                    serde_json::from_str(&fs::read_to_string(&meta_path)?)?;

                let req = PushTraceRequest {
                    repo_name: "unknown".into(),
                    org_name: "default".into(),
                    commit_sha: "pending".into(),
                    branch: None,
                    author: meta
                        .get("author")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .into(),
                    model: None,
                    tool: Some("claude-code".into()),
                    ai_percentage: None,
                    total_tokens: None,
                    input_tokens: None,
                    output_tokens: None,
                    estimated_cost_usd: None,
                    api_calls: None,
                    session_data: Some(meta.clone()),
                    attribution: None,
                };

                match client.push_trace(req).await {
                    Ok(resp) => {
                        println!(
                            "Pushed trace {} -> {}",
                            entry.file_name().to_string_lossy(),
                            resp.id
                        );
                        pushed += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to push {}: {e}",
                            entry.file_name().to_string_lossy()
                        );
                    }
                }
            }
        }
    }

    println!("Pushed {pushed} trace(s) to server.");
    Ok(())
}
