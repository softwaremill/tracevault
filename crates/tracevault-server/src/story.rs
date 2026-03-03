use crate::repo_manager::RepoManager;
use sqlx::PgPool;
use tracevault_core::code_nav::CodeScope;
use uuid::Uuid;

#[derive(Debug, serde::Serialize)]
pub struct StoryContext {
    pub function_source: String,
    pub changes: Vec<ChangeEntry>,
}

#[derive(Debug, serde::Serialize)]
pub struct ChangeEntry {
    pub commit_sha: String,
    pub author: String,
    pub date: String,
    pub message: String,
    pub diff_excerpt: Option<String>,
    pub session_transcript_excerpt: Option<String>,
    pub ai_percentage: Option<f64>,
    pub model: Option<String>,
}

/// Intermediate struct for git data extracted synchronously
struct GitCommitInfo {
    sha: String,
    author: String,
    date: String,
    message: String,
}

pub async fn gather_story_context(
    repo_manager: &RepoManager,
    pool: &PgPool,
    repo_id: Uuid,
    git_ref: &str,
    file_path: &str,
    scope: &CodeScope,
) -> Result<StoryContext, String> {
    // 1. Extract all git data synchronously (git2 types are !Send)
    let (function_source, git_commits) = {
        let repo = repo_manager.open_repo(repo_id)?;

        let obj = repo.revparse_single(git_ref).map_err(|e| e.to_string())?;
        let commit = obj.peel_to_commit().map_err(|e| e.to_string())?;
        let tree = commit.tree().map_err(|e| e.to_string())?;
        let entry = tree
            .get_path(std::path::Path::new(file_path))
            .map_err(|e| e.to_string())?;
        let blob = entry.to_object(&repo).map_err(|e| e.to_string())?;
        let content = blob.as_blob().ok_or("Not a blob")?;
        let source = String::from_utf8_lossy(content.content()).to_string();
        let lines: Vec<&str> = source.lines().collect();
        let func_lines =
            &lines[scope.start_line.saturating_sub(1)..scope.end_line.min(lines.len())];
        let function_source = func_lines.join("\n");

        // Walk git log for commits touching this file
        let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
        revwalk.push(commit.id()).map_err(|e| e.to_string())?;
        revwalk
            .set_sorting(git2::Sort::TIME | git2::Sort::REVERSE)
            .map_err(|e| e.to_string())?;

        let mut git_commits = Vec::new();
        for oid in revwalk.take(200) {
            let oid = oid.map_err(|e| e.to_string())?;
            let c = repo.find_commit(oid).map_err(|e| e.to_string())?;

            let touches_file = c
                .parent(0)
                .ok()
                .and_then(|parent| {
                    let old_tree = parent.tree().ok()?;
                    let new_tree = c.tree().ok()?;
                    let diff = repo
                        .diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)
                        .ok()?;
                    Some(diff.deltas().any(|d| {
                        d.new_file()
                            .path()
                            .map(|p| p.to_string_lossy() == file_path)
                            .unwrap_or(false)
                    }))
                })
                .unwrap_or(true);

            if !touches_file {
                continue;
            }

            git_commits.push(GitCommitInfo {
                sha: oid.to_string(),
                author: c.author().name().unwrap_or("unknown").to_string(),
                date: c.time().seconds().to_string(),
                message: c.message().unwrap_or("").trim().to_string(),
            });
        }

        (function_source, git_commits)
    };
    // All git2 objects are now dropped — safe to .await

    // 2. Enrich with TraceVault DB data (async)
    let mut changes = Vec::new();
    for gc in &git_commits {
        let tv_data = sqlx::query_as::<_, (Option<serde_json::Value>, Option<serde_json::Value>)>(
            "SELECT c.attribution, s.transcript FROM commits c LEFT JOIN sessions s ON s.commit_id = c.id WHERE c.repo_id = $1 AND c.commit_sha = $2 LIMIT 1",
        )
        .bind(repo_id)
        .bind(&gc.sha)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let (ai_percentage, session_excerpt, model) = if let Some((attr, transcript)) = &tv_data {
            let pct = attr
                .as_ref()
                .and_then(|a| a.get("ai_percentage"))
                .and_then(|v| v.as_f64());
            let excerpt =
                transcript
                    .as_ref()
                    .and_then(|t| extract_relevant_excerpt(t, file_path, &scope.name));
            let model = sqlx::query_scalar::<_, Option<String>>(
                "SELECT s.model FROM sessions s JOIN commits c ON c.id = s.commit_id WHERE c.repo_id = $1 AND c.commit_sha = $2 LIMIT 1",
            )
            .bind(repo_id)
            .bind(&gc.sha)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .flatten();
            (pct, excerpt, model)
        } else {
            (None, None, None)
        };

        changes.push(ChangeEntry {
            commit_sha: gc.sha.clone(),
            author: gc.author.clone(),
            date: gc.date.clone(),
            message: gc.message.clone(),
            diff_excerpt: None,
            session_transcript_excerpt: session_excerpt,
            ai_percentage,
            model,
        });
    }

    Ok(StoryContext {
        function_source,
        changes,
    })
}

fn extract_relevant_excerpt(
    transcript: &serde_json::Value,
    file_path: &str,
    func_name: &str,
) -> Option<String> {
    let text = serde_json::to_string(transcript).ok()?;
    let mut excerpts = Vec::new();
    for line in text.lines() {
        if line.contains(file_path) || line.contains(func_name) {
            excerpts.push(line.to_string());
        }
    }
    if excerpts.is_empty() {
        return None;
    }
    Some(excerpts.join("\n").chars().take(2000).collect())
}

pub fn build_story_prompt(ctx: &StoryContext) -> String {
    let mut prompt = String::from(
        "You are documenting the history of a code function for a development team.\n\n\
         Write a clear, concise narrative. Focus on WHY decisions were made, not just WHAT changed.\n\n\
         ## Current Code\n```\n",
    );
    prompt.push_str(&ctx.function_source);
    prompt.push_str("\n```\n\n## Change History (chronological)\n\n");

    for (i, change) in ctx.changes.iter().enumerate() {
        prompt.push_str(&format!(
            "### Change {}: {} by {} on {}\n",
            i + 1,
            change.commit_sha,
            change.author,
            change.date
        ));
        prompt.push_str(&format!("Commit message: {}\n", change.message));
        if let Some(model) = &change.model {
            prompt.push_str(&format!("AI Model: {}\n", model));
        }
        if let Some(pct) = change.ai_percentage {
            prompt.push_str(&format!("Attribution: {:.0}% AI-generated\n", pct));
        }
        if let Some(excerpt) = &change.session_transcript_excerpt {
            prompt.push_str(&format!("AI Session Context:\n{}\n", excerpt));
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "## Task\nWrite a clear narrative explaining:\n\
         1. Why this code exists (original intent)\n\
         2. How it evolved over time\n\
         3. Key decisions made (especially from AI session transcripts)\n\
         4. Current state and any notable patterns\n\n\
         Keep it concise but informative. Use markdown formatting.",
    );

    if prompt.len() > 100_000 {
        prompt.truncate(100_000);
        prompt.push_str("\n\n[Context truncated due to length]");
    }

    prompt
}
