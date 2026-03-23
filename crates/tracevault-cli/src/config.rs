use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct TracevaultConfig {
    pub agent: String,
    pub server_url: Option<String>,
    pub api_key: Option<String>,
    pub org_slug: Option<String>,
}

impl Default for TracevaultConfig {
    fn default() -> Self {
        Self {
            agent: "claude-code".to_string(),
            server_url: None,
            api_key: None,
            org_slug: None,
        }
    }
}

impl TracevaultConfig {
    pub fn config_dir(project_root: &Path) -> PathBuf {
        project_root.join(".tracevault")
    }

    pub fn config_path(project_root: &Path) -> PathBuf {
        Self::config_dir(project_root).join("config.toml")
    }

    pub fn to_toml(&self) -> String {
        let mut out = format!("# TraceVault configuration\nagent = \"{}\"\n", self.agent);
        if let Some(url) = &self.server_url {
            out.push_str(&format!("server_url = \"{url}\"\n"));
        }
        if let Some(slug) = &self.org_slug {
            out.push_str(&format!("org_slug = \"{slug}\"\n"));
        }
        out
    }

    /// Parse config from the TOML file using simple line-based parsing
    /// (consistent with the existing resolve_credentials approach).
    pub fn load(project_root: &Path) -> Option<Self> {
        let path = Self::config_path(project_root);
        let content = std::fs::read_to_string(path).ok()?;

        let parse_field = |key: &str| -> Option<String> {
            content
                .lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split('=').nth(1))
                .map(|s| s.trim().trim_matches('"').to_string())
        };

        Some(Self {
            agent: parse_field("agent").unwrap_or_else(|| "claude-code".to_string()),
            server_url: parse_field("server_url"),
            api_key: parse_field("api_key"),
            org_slug: parse_field("org_slug"),
        })
    }
}
