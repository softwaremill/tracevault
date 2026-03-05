use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct TracevaultConfig {
    pub agent: String,
    pub server_url: Option<String>,
    pub api_key: Option<String>,
}

impl Default for TracevaultConfig {
    fn default() -> Self {
        Self {
            agent: "claude-code".to_string(),
            server_url: None,
            api_key: None,
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
        out
    }
}
