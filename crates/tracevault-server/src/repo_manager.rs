use git2::{build::RepoBuilder, Cred, FetchOptions, RemoteCallbacks, Repository};
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone)]
pub struct RepoManager {
    repos_dir: PathBuf,
}

/// Build remote callbacks that handle SSH and HTTPS auth.
/// - SSH: tries the SSH agent first, then falls back to ~/.ssh/id_ed25519 and ~/.ssh/id_rsa
/// - HTTPS: tries git credential helper
fn make_callbacks() -> RemoteCallbacks<'static> {
    let mut callbacks = RemoteCallbacks::new();
    let mut ssh_attempts = 0u32;
    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        if allowed_types.contains(git2::CredentialType::SSH_KEY) {
            ssh_attempts += 1;
            let user = username_from_url.unwrap_or("git");

            // First attempt: try SSH agent
            if ssh_attempts == 1 {
                if let Ok(cred) = Cred::ssh_key_from_agent(user) {
                    return Ok(cred);
                }
            }

            // Second attempt: try default key files
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
            let key_paths = [
                format!("{home}/.ssh/id_ed25519"),
                format!("{home}/.ssh/id_rsa"),
            ];

            for key_path in &key_paths {
                let path = std::path::Path::new(key_path);
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

fn make_fetch_options() -> FetchOptions<'static> {
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(make_callbacks());
    fetch_opts
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
    ) -> Result<(), String> {
        let path = self.repo_path(repo_id);

        sqlx::query("UPDATE repos SET clone_status = 'cloning' WHERE id = $1")
            .bind(repo_id)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        let clone_result = RepoBuilder::new()
            .bare(true)
            .fetch_options(make_fetch_options())
            .clone(github_url, &path);

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
    pub fn fetch_repo(&self, repo_id: Uuid) -> Result<(), String> {
        let path = self.repo_path(repo_id);
        let repo = Repository::open_bare(&path).map_err(|e| e.to_string())?;
        let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
        remote
            .fetch(
                &["refs/heads/*:refs/heads/*", "refs/tags/*:refs/tags/*"],
                Some(&mut make_fetch_options()),
                None,
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Open a bare repo, returning git2::Repository.
    pub fn open_repo(&self, repo_id: Uuid) -> Result<Repository, String> {
        let path = self.repo_path(repo_id);
        Repository::open_bare(&path).map_err(|e| e.to_string())
    }
}
