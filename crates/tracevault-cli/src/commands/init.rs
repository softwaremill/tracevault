use crate::config::TracevaultConfig;
use std::fs;
use std::io;
use std::path::Path;

pub fn init_in_directory(project_root: &Path) -> Result<(), io::Error> {
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

    Ok(())
}

/// Generate Claude Code hook configuration JSON
pub fn claude_code_hooks_json() -> serde_json::Value {
    serde_json::json!({
        "hooks": {
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
        }
    })
}
