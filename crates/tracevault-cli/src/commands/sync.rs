use crate::api_client::ApiClient;
use crate::commands::init::git_remote_url;
use crate::config::TracevaultConfig;
use std::fs;
use std::path::Path;

pub async fn sync_repo(project_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = TracevaultConfig::config_path(project_root);
    let config_content = fs::read_to_string(&config_path)?;

    let server_url = config_content
        .lines()
        .find(|l| l.starts_with("server_url"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .or_else(|| std::env::var("TRACEVAULT_SERVER_URL").ok());

    let server_url = match server_url {
        Some(url) => url,
        None => {
            eprintln!("No server_url configured. Skipping sync.");
            return Ok(());
        }
    };

    let remote = match git_remote_url(project_root) {
        Some(url) => url,
        None => {
            eprintln!("No git remote 'origin' configured. Skipping sync.");
            return Ok(());
        }
    };

    let api_key = config_content
        .lines()
        .find(|l| l.starts_with("api_key"))
        .and_then(|l| l.split('=').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .or_else(|| std::env::var("TRACEVAULT_API_KEY").ok());

    let client = ApiClient::new(&server_url, api_key.as_deref());

    let repo_name = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .as_deref()
        .and_then(|p| p.rsplit('/').next())
        .map(String::from)
        .unwrap_or_else(|| "unknown".into());

    let org_name = std::env::var("TRACEVAULT_ORG").unwrap_or_else(|_| "default".into());

    match client
        .register_repo(crate::api_client::RegisterRepoRequest {
            org_name,
            repo_name,
            github_url: Some(remote.clone()),
        })
        .await
    {
        Ok(resp) => {
            println!("Repo synced with server (id: {}, remote: {})", resp.repo_id, remote);
        }
        Err(e) => {
            eprintln!("Warning: could not sync repo with server: {e}");
        }
    }

    Ok(())
}
