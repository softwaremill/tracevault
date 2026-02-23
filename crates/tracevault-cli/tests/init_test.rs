use std::fs;
use tempfile::TempDir;

#[test]
fn init_creates_tracevault_config() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join(".tracevault").join("config.toml");

    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("claude-code"));
}

#[test]
fn init_creates_directory_structure() {
    let tmp = TempDir::new().unwrap();

    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    assert!(tmp.path().join(".tracevault").exists());
    assert!(tmp.path().join(".tracevault/sessions").exists());
    assert!(tmp.path().join(".tracevault/cache").exists());
    assert!(tmp.path().join(".tracevault/.gitignore").exists());
}

#[test]
fn init_generates_hook_config() {
    let hooks = tracevault_cli::commands::init::claude_code_hooks_json();
    let hooks_obj = hooks.get("hooks").unwrap();
    assert!(hooks_obj.get("PreToolUse").is_some());
    assert!(hooks_obj.get("PostToolUse").is_some());
}
