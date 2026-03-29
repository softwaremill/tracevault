use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub struct CodeStoryRepo;

impl CodeStoryRepo {
    /// Look up a cached code story by repo, file, function, ref, and head SHA.
    pub async fn find_cached(
        pool: &PgPool,
        repo_id: Uuid,
        file_path: &str,
        function_name: &str,
        ref_name: &str,
        head_sha: &str,
    ) -> Result<
        Option<(
            String,            // story_markdown
            String,            // function_name
            i32,               // line_range_start
            i32,               // line_range_end
            serde_json::Value, // commits_analyzed
            serde_json::Value, // sessions_referenced
            serde_json::Value, // references_data
            String,            // generated_at
        )>,
        AppError,
    > {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                i32,
                i32,
                serde_json::Value,
                serde_json::Value,
                serde_json::Value,
                String,
            ),
        >(
            "SELECT story_markdown, function_name, line_range_start, line_range_end, \
             commits_analyzed, sessions_referenced, references_data, generated_at::text \
             FROM code_stories WHERE repo_id = $1 AND file_path = $2 AND function_name = $3 \
             AND ref_name = $4 AND head_commit_sha = $5 LIMIT 1",
        )
        .bind(repo_id)
        .bind(file_path)
        .bind(function_name)
        .bind(ref_name)
        .bind(head_sha)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    /// Delete any existing cached story for (repo, file, function, ref), then insert a new one.
    #[allow(clippy::too_many_arguments)]
    pub async fn save(
        pool: &PgPool,
        repo_id: Uuid,
        file_path: &str,
        function_name: &str,
        start: i32,
        end: i32,
        ref_name: &str,
        head_sha: &str,
        story: &str,
        commits: &serde_json::Value,
        sessions: &serde_json::Value,
        references: &serde_json::Value,
        provider: &str,
        model: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "DELETE FROM code_stories WHERE repo_id = $1 AND file_path = $2 \
             AND function_name = $3 AND ref_name = $4",
        )
        .bind(repo_id)
        .bind(file_path)
        .bind(function_name)
        .bind(ref_name)
        .execute(pool)
        .await?;

        sqlx::query(
            "INSERT INTO code_stories (repo_id, file_path, function_name, line_range_start, \
             line_range_end, ref_name, head_commit_sha, story_markdown, commits_analyzed, \
             sessions_referenced, references_data, llm_provider, llm_model) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(repo_id)
        .bind(file_path)
        .bind(function_name)
        .bind(start)
        .bind(end)
        .bind(ref_name)
        .bind(head_sha)
        .bind(story)
        .bind(commits)
        .bind(sessions)
        .bind(references)
        .bind(provider)
        .bind(model)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete a cached story by (repo, file, function, ref).
    pub async fn delete_cached(
        pool: &PgPool,
        repo_id: Uuid,
        file_path: &str,
        function_name: &str,
        ref_name: &str,
    ) -> Result<u64, AppError> {
        let result = sqlx::query(
            "DELETE FROM code_stories WHERE repo_id = $1 AND file_path = $2 \
             AND function_name = $3 AND ref_name = $4",
        )
        .bind(repo_id)
        .bind(file_path)
        .bind(function_name)
        .bind(ref_name)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
