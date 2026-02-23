use crate::config::TracevaultConfig;
use std::fs;
use std::io;
use std::path::Path;

pub fn init_in_directory(project_root: &Path) -> Result<(), io::Error> {
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

    // Write default config
    let config = TracevaultConfig::default();
    fs::write(TracevaultConfig::config_path(project_root), config.to_toml())?;

    // Create .tracevault/.gitignore
    fs::write(
        config_dir.join(".gitignore"),
        "sessions/\ncache/\n*.local.toml\n",
    )?;

    // Install Claude Code hooks into .claude/settings.json
    install_claude_hooks(project_root)?;

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
