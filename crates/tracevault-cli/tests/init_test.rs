use std::fs;
use tempfile::TempDir;

fn tmp_git_repo() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::create_dir(tmp.path().join(".git")).unwrap();
    tmp
}

#[test]
fn init_fails_without_git() {
    let tmp = TempDir::new().unwrap();
    let result = tracevault_cli::commands::init::init_in_directory(tmp.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Not a git repository"));
}

#[test]
fn init_creates_tracevault_config() {
    let tmp = tmp_git_repo();
    let config_path = tmp.path().join(".tracevault").join("config.toml");

    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("claude-code"));
}

#[test]
fn init_creates_directory_structure() {
    let tmp = tmp_git_repo();

    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    assert!(tmp.path().join(".tracevault").exists());
    assert!(tmp.path().join(".tracevault/sessions").exists());
    assert!(tmp.path().join(".tracevault/cache").exists());
    assert!(tmp.path().join(".tracevault/.gitignore").exists());
}

#[test]
fn init_installs_claude_hooks() {
    let tmp = tmp_git_repo();

    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    let settings_path = tmp.path().join(".claude/settings.json");
    assert!(settings_path.exists());

    let content = fs::read_to_string(&settings_path).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&content).unwrap();
    let hooks = settings.get("hooks").unwrap();
    assert!(hooks.get("PreToolUse").is_some());
    assert!(hooks.get("PostToolUse").is_some());
}

#[test]
fn init_merges_into_existing_settings() {
    let tmp = tmp_git_repo();

    // Pre-existing settings.json with other config
    let claude_dir = tmp.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"model": "opus"}"#,
    )
    .unwrap();

    tracevault_cli::commands::init::init_in_directory(tmp.path()).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let settings: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Hooks were added
    assert!(settings.get("hooks").is_some());
    // Existing config preserved
    assert_eq!(settings.get("model").unwrap(), "opus");
}

#[test]
fn tracevault_hooks_has_pre_and_post() {
    let hooks = tracevault_cli::commands::init::tracevault_hooks();
    assert!(hooks.get("PreToolUse").is_some());
    assert!(hooks.get("PostToolUse").is_some());
}
