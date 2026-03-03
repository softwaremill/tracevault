use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository};
use sqlx::PgPool;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone)]
pub struct RepoManager {
    repos_dir: PathBuf,
}

/// Build remote callbacks that handle SSH and HTTPS auth.
/// If `deploy_key_path` is provided, it takes priority for SSH auth.
/// Otherwise: SSH agent first, then ~/.ssh/id_ed25519 and ~/.ssh/id_rsa, then HTTPS credential helper.
fn make_callbacks(deploy_key_path: Option<PathBuf>) -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    let mut ssh_attempts = 0u32;
    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            ssh_attempts += 1;
            let user = username_from_url.unwrap_or("git");

            // If a deploy key is provided, try it first
            if ssh_attempts == 1 {
                if let Some(ref dk_path) = deploy_key_path {
                    if dk_path.exists() {
                        return Cred::ssh_key(user, None, dk_path, None);
                    }
                }
            }

            // Try SSH agent
            if ssh_attempts <= 2 {
                if let Ok(cred) = Cred::ssh_key_from_agent(user) {
                    return Ok(cred);
                }
            }

            // Fall back to default key files
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let key_paths = [
                format!("{home}/.ssh/id_ed25519"),
                format!("{home}/.ssh/id_rsa"),
            ];

            for key_path in &key_paths {
                let path = Path::new(key_path);
                if path.exists() {
                    return Cred::ssh_key(user, None, path, None);
                }
            }

            Err(git2::Error::from_str("no SSH key found"))
        } else if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
            Cred::credential_helper(
                &git2::Config::open_default().unwrap_or_else(|_| git2::Config::new().unwrap()),
                _url,
                username_from_url,
            )
        } else if allowed_types.contains(git2::CredentialType::DEFAULT) {
            Cred::default()
        } else {
            Err(git2::Error::from_str("unsupported credential type"))
        }
    });
    callbacks
}

fn make_fetch_options(deploy_key_path: Option<PathBuf>) -> FetchOptions<'static> {
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(make_callbacks(deploy_key_path));
    fetch_opts
}

/// Write a deploy key PEM to a temp file and return its path.
/// Caller is responsible for cleaning up via `cleanup_temp_key`.
fn write_temp_key(deploy_key_pem: &str) -> Result<PathBuf, String> {
    use std::io::Write;
    let dir = std::env::temp_dir().join("tracevault-keys");
    std::fs::create_dir_all(&dir).map_err(|e| format!("failed to create temp key dir: {e}"))?;
    let path = dir.join(format!("dk-{}", uuid::Uuid::new_v4()));
    let mut file =
        std::fs::File::create(&path).map_err(|e| format!("failed to create temp key file: {e}"))?;
    file.write_all(deploy_key_pem.as_bytes())
        .map_err(|e| format!("failed to write temp key: {e}"))?;
    // SSH requires restrictive permissions on key files
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
    /// If `deploy_key_pem` is provided, writes it to a temp file for SSH auth.
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

        let temp_key = deploy_key_pem
            .map(write_temp_key)
            .transpose()?;

        let clone_result = RepoBuilder::new()
            .bare(true)
            .fetch_options(make_fetch_options(temp_key.clone()))
            .clone(github_url, &path);

        if let Some(ref kp) = temp_key {
            cleanup_temp_key(kp);
        }

        match clone_result {
            Ok(_) => {
                sqlx::query("UPDATE repos SET clone_status = 'ready', clone_path = $1, last_fetched_at = now() WHERE id = $2")
                    .bind(path.to_string_lossy().to_string())
                    .bind(repo_id)
                    .execute(pool)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            Err(e) => {
                // Clean up partial clone on failure
                std::fs::remove_dir_all(&path).ok();
                sqlx::query("UPDATE repos SET clone_status = 'error' WHERE id = $1")
                    .bind(repo_id)
                    .execute(pool)
                    .await
                    .ok();
                Err(e.to_string())
            }
        }
    }

    /// Fetch latest changes for a bare repo.
    /// If `deploy_key_pem` is provided, writes it to a temp file for SSH auth.
    pub fn fetch_repo(&self, repo_id: Uuid, deploy_key_pem: Option<&str>) -> Result<(), String> {
        let path = self.repo_path(repo_id);
        let repo = Repository::open_bare(&path).map_err(|e| e.to_string())?;
        let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;

        let temp_key = deploy_key_pem
            .map(write_temp_key)
            .transpose()?;

        let result = remote.fetch(
            &["refs/heads/*:refs/heads/*", "refs/tags/*:refs/tags/*"],
            Some(&mut make_fetch_options(temp_key.clone())),
            None,
        );

        if let Some(ref kp) = temp_key {
            cleanup_temp_key(kp);
        }

        result.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Open a bare repo, returning git2::Repository.
    pub fn open_repo(&self, repo_id: Uuid) -> Result<Repository, String> {
        let path = self.repo_path(repo_id);
        Repository::open_bare(&path).map_err(|e| e.to_string())
    }
}
