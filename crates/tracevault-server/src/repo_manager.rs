use git2::{build::RepoBuilder, Repository};
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone)]
pub struct RepoManager {
    repos_dir: PathBuf,
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

        let clone_result = RepoBuilder::new().bare(true).clone(github_url, &path);

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
                None,
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
