use git2::Repository;
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Clone)]
pub struct RepoManager {
    repos_dir: PathBuf,
}

/// Write a deploy key PEM to a temp file and return its path.
fn write_temp_key(deploy_key_pem: &str) -> Result<PathBuf, String> {
    use std::io::Write;
    let dir = std::env::temp_dir().join("tracevault-keys");
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create temp key dir: {e}"))?;
    let path = dir.join(format!("dk-{}", uuid::Uuid::new_v4()));
    let mut file =
        std::fs::File::create(&path).map_err(|e| format!("failed to create temp key file: {e}"))?;
    file.write_all(deploy_key_pem.as_bytes())
        .map_err(|e| format!("failed to write temp key: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("failed to set key permissions: {e}"))?;
    }
    Ok(path)
}

fn cleanup_temp_key(path: &Path) {
    std::fs::remove_file(path).ok();
}

/// Run a git command with optional deploy key SSH configuration.
fn git_cmd(deploy_key_path: Option<&Path>) -> Command {
    let mut cmd = Command::new("git");
    if let Some(key_path) = deploy_key_path {
        cmd.env(
            "GIT_SSH_COMMAND",
            format!(
                "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=accept-new",
                key_path.display()
            ),
        );
    }
    cmd
}

impl RepoManager {
    pub fn new(repos_dir: &str) -> Self {
        std::fs::create_dir_all(repos_dir).ok();
        Self {
            repos_dir: PathBuf::from(repos_dir),
        }
    }

    pub fn repo_path(&self, repo_id: Uuid) -> PathBuf {
        self.repos_dir.join(repo_id.to_string())
    }

    /// Clone a repo as bare. Updates clone_status in DB.
    pub async fn clone_repo(
        &self,
        pool: &PgPool,
        repo_id: Uuid,
        github_url: &str,
        deploy_key_pem: Option<&str>,
    ) -> Result<(), String> {
        let path = self.repo_path(repo_id);

        sqlx::query("UPDATE repos SET clone_status = 'cloning' WHERE id = $1")
            .bind(repo_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        let temp_key = deploy_key_pem.map(write_temp_key).transpose()?;

        let output = git_cmd(temp_key.as_deref())
            .args(["clone", "--bare", github_url])
            .arg(&path)
            .output()
            .map_err(|e| format!("failed to run git clone: {e}"));

        if let Some(ref kp) = temp_key {
            cleanup_temp_key(kp);
        }

        let output = output?;

        if output.status.success() {
            sqlx::query("UPDATE repos SET clone_status = 'ready', clone_path = $1, last_fetched_at = now() WHERE id = $2")
                .bind(path.to_string_lossy().to_string())
                .bind(repo_id)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(())
        } else {
            std::fs::remove_dir_all(&path).ok();
            sqlx::query("UPDATE repos SET clone_status = 'error' WHERE id = $1")
                .bind(repo_id)
                .execute(pool)
                .await
                .ok();
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("git clone failed: {stderr}"))
        }
    }

    /// Fetch latest changes for a bare repo.
    pub fn fetch_repo(&self, repo_id: Uuid, deploy_key_pem: Option<&str>) -> Result<(), String> {
        let path = self.repo_path(repo_id);
        if !path.exists() {
            return Err("bare repo directory does not exist".into());
        }

        let temp_key = deploy_key_pem.map(write_temp_key).transpose()?;

        let output = git_cmd(temp_key.as_deref())
            .args([
                "-C",
                &path.to_string_lossy(),
                "fetch",
                "origin",
                "refs/heads/*:refs/heads/*",
                "refs/tags/*:refs/tags/*",
            ])
            .output()
            .map_err(|e| format!("failed to run git fetch: {e}"));

        if let Some(ref kp) = temp_key {
            cleanup_temp_key(kp);
        }

        let output = output?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("git fetch failed: {stderr}"))
        }
    }

    /// Open a bare repo, returning git2::Repository.
    pub fn open_repo(&self, repo_id: Uuid) -> Result<Repository, String> {
        let path = self.repo_path(repo_id);
        Repository::open_bare(&path).map_err(|e| e.to_string())
    }
}
