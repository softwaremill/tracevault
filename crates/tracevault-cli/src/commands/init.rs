use crate::api_client::ApiClient;
use crate::config::TracevaultConfig;
use std::fs;
use std::io;
use std::path::Path;

pub fn git_remote_url(project_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

pub async fn init_in_directory(
    project_root: &Path,
    server_url: Option<&str>,
) -> Result<(), io::Error> {
    // Check for git repository
    if !project_root.join(".git").exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Not a git repository. Run 'git init' first.",
        ));
    }

    // Create .tracevault/ directory
    let config_dir = TracevaultConfig::config_dir(project_root);
    fs::create_dir_all(&config_dir)?;
    fs::create_dir_all(config_dir.join("sessions"))?;
    fs::create_dir_all(config_dir.join("cache"))?;

    // Write config (include server_url if provided)
    let mut config = TracevaultConfig::default();
    if let Some(url) = server_url {
        config.server_url = Some(url.to_string());
    }
    fs::write(TracevaultConfig::config_path(project_root), config.to_toml())?;

    // Create .tracevault/.gitignore
    fs::write(
        config_dir.join(".gitignore"),
        "sessions/\ncache/\n*.local.toml\n",
    )?;

    // Install Claude Code hooks into .claude/settings.json
    install_claude_hooks(project_root)?;

    // Install git pre-push hook
    install_git_hook(project_root)?;

    // Register repo on server if authenticated, server URL known, and git remote available
    let remote_url = git_remote_url(project_root);
    if remote_url.is_none() {
        eprintln!("Warning: no git remote 'origin' configured. Skipping server registration.");
        eprintln!("Run 'git remote add origin <url>' then 'tracevault sync' to register.");
    }

    let (resolved_url, resolved_token) =
        crate::api_client::resolve_credentials(project_root);
    let effective_url = server_url
        .map(String::from)
        .or(resolved_url);

    if resolved_token.is_none() {
        eprintln!("Not logged in. Run 'tracevault login' to register this repo with the server.");
    } else if let (Some(url), Some(remote)) = (effective_url, remote_url) {
        let client = ApiClient::new(&url, resolved_token.as_deref());
        let repo_name = git_repo_name(project_root);

        match client
            .register_repo(crate::api_client::RegisterRepoRequest {
                repo_name,
                github_url: Some(remote),
            })
            .await
        {
            Ok(resp) => {
                println!("Repo registered on server (id: {})", resp.repo_id);
            }
            Err(e) => {
                eprintln!("Warning: could not register repo on server: {e}");
            }
        }
    }

    Ok(())
}

fn git_repo_name(project_root: &Path) -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .as_deref()
        .and_then(|p| p.rsplit('/').next())
        .map(String::from)
        .unwrap_or_else(|| "unknown".into())
}

const HOOK_MARKER: &str = "# tracevault:enforce";
const OLD_HOOK_MARKER: &str = "# tracevault:auto-push";

fn install_git_hook(project_root: &Path) -> Result<(), io::Error> {
    let hooks_dir = project_root.join(".git/hooks");
    fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("pre-push");
    let tracevault_block = format!(
        "{HOOK_MARKER}\ntracevault sync 2>/dev/null || true\ntracevault push || {{ echo \"tracevault: push failed, git push blocked.\"; exit 1; }}\n"
    );

    if hook_path.exists() {
        let existing = fs::read_to_string(&hook_path)?;

        // Already has new-style hook
        if existing.contains(HOOK_MARKER) {
            return Ok(());
        }

        // Replace old-style hook block if present
        if existing.contains(OLD_HOOK_MARKER) {
            let mut new_content = String::new();
            let mut skip = false;
            for line in existing.lines() {
                if line.contains(OLD_HOOK_MARKER) {
                    skip = true;
                    continue;
                }
                if skip {
                    // Skip old tracevault lines (they start with "tracevault " or are empty continuations)
                    if line.starts_with("tracevault ") {
                        continue;
                    }
                    skip = false;
                }
                new_content.push_str(line);
                new_content.push('\n');
            }
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push_str(&tracevault_block);
            fs::write(&hook_path, new_content)?;
        } else {
            // Append to existing hook
            let mut content = existing;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push_str(&tracevault_block);
            fs::write(&hook_path, content)?;
        }
    } else {
        let content = format!("#!/bin/sh\n{tracevault_block}");
        fs::write(&hook_path, content)?;
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

fn install_claude_hooks(project_root: &Path) -> Result<(), io::Error> {
    let claude_dir = project_root.join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let settings_path = claude_dir.join("settings.json");
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to parse .claude/settings.json: {e}"),
            )
        })?
    } else {
        serde_json::json!({})
    };

    let hooks = tracevault_hooks();

    // Merge hooks into existing settings
    let settings_obj = settings.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            ".claude/settings.json is not a JSON object",
        )
    })?;

    settings_obj.insert("hooks".to_string(), hooks);

    let formatted = serde_json::to_string_pretty(&settings).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Failed to serialize settings: {e}"))
    })?;
    fs::write(&settings_path, formatted)?;

    Ok(())
}

pub fn tracevault_hooks() -> serde_json::Value {
    serde_json::json!({
        "PreToolUse": [{
            "matcher": "Write|Edit",
            "hooks": [{
                "type": "command",
                "command": "tracevault hook --event pre-tool-use",
                "timeout": 5,
                "statusMessage": "TraceVault: capturing pre-edit state"
            }]
        }],
        "PostToolUse": [{
            "matcher": "Write|Edit|Bash",
            "hooks": [{
                "type": "command",
                "command": "tracevault hook --event post-tool-use",
                "timeout": 5,
                "statusMessage": "TraceVault: recording change"
            }]
        }]
    })
}
