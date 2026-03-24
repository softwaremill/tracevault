use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{extractors::OrgAuth, AppState};

// ── Query param types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub repo_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SessionListQuery {
    pub repo_id: Option<Uuid>,
    pub status: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CommitListQuery {
    pub repo_id: Option<Uuid>,
    pub branch: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub active_sessions: i64,
    pub total_sessions: i64,
    pub total_commits: i64,
    pub total_events: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SessionListItem {
    pub id: Uuid,
    pub session_id: String,
    pub repo_id: Uuid,
    pub repo_name: String,
    pub user_id: Uuid,
    pub user_email: String,
    pub status: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub total_tool_calls: Option<i32>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub cwd: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SessionDetail {
    pub id: Uuid,
    pub session_id: String,
    pub repo_name: String,
    pub user_email: String,
    pub status: String,
    pub model: Option<String>,
    pub tool: Option<String>,
    pub total_tool_calls: Option<i32>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub cwd: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventRow {
    pub id: Uuid,
    pub event_index: i32,
    pub event_type: String,
    pub tool_name: Option<String>,
    pub tool_input: Option<serde_json::Value>,
    pub tool_response: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FileChangeRow {
    pub id: Uuid,
    pub file_path: String,
    pub change_type: String,
    pub diff_text: Option<String>,
    pub content_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TranscriptChunkRow {
    pub chunk_index: i32,
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LinkedCommitRow {
    pub commit_id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct SessionDetailResponse {
    pub session: SessionDetail,
    pub events: Vec<EventRow>,
    pub file_changes: Vec<FileChangeRow>,
    pub transcript_chunks: Vec<TranscriptChunkRow>,
    pub linked_commits: Vec<LinkedCommitRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CommitListItem {
    pub id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub files_changed: Option<i64>,
    pub ai_sessions_count: Option<i64>,
    pub committed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CommitDetail {
    pub id: Uuid,
    pub commit_sha: String,
    pub branch: Option<String>,
    pub author: String,
    pub committed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AttributionByFile {
    pub file_path: String,
    pub sessions: Vec<AttributionSession>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AttributionSession {
    pub session_id: Uuid,
    pub session_short_id: String,
    pub confidence: f32,
    pub line_start: Option<i32>,
    pub line_end: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct CommitDetailResponse {
    pub commit: CommitDetail,
    pub diff_data: Option<serde_json::Value>,
    pub attributions_by_file: Vec<AttributionByFile>,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /api/v1/orgs/{slug}/traces/stats
pub async fn get_stats(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(params): Query<StatsQuery>,
) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    let repo_filter = params.repo_id;

    let active_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         WHERE r.org_id = $1
           AND s.status = 'active'
           AND s.updated_at >= now() - interval '30 minutes'
           AND ($2::UUID IS NULL OR s.repo_id = $2)",
    )
    .bind(auth.org_id)
    .bind(repo_filter)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::UUID IS NULL OR s.repo_id = $2)",
    )
    .bind(auth.org_id)
    .bind(repo_filter)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_commits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM commits_v2 c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::UUID IS NULL OR c.repo_id = $2)",
    )
    .bind(auth.org_id)
    .bind(repo_filter)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let total_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events e
         JOIN sessions_v2 s ON e.session_id = s.id
         JOIN repos r ON s.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::UUID IS NULL OR s.repo_id = $2)",
    )
    .bind(auth.org_id)
    .bind(repo_filter)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(StatsResponse {
        active_sessions,
        total_sessions,
        total_commits,
        total_events,
    }))
}

/// GET /api/v1/orgs/{slug}/traces/sessions
pub async fn list_sessions(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(params): Query<SessionListQuery>,
) -> Result<Json<Vec<SessionListItem>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    // Map status filter: "stale" is a virtual status
    let (status_filter, use_stale) = match params.status.as_deref() {
        Some("stale") => (Some("active".to_string()), true),
        other => (other.map(String::from), false),
    };

    let rows = sqlx::query_as::<_, SessionListItem>(
        "SELECT s.id, s.session_id, s.repo_id, r.name AS repo_name,
                s.user_id, u.email AS user_email, s.status, s.model, s.tool,
                s.total_tool_calls, s.total_tokens, s.estimated_cost_usd,
                s.cwd, s.started_at, s.updated_at
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::UUID IS NULL OR s.repo_id = $2)
           AND ($3::TEXT IS NULL OR s.status = $3)
           AND ($4::BOOL = FALSE OR s.updated_at < now() - interval '30 minutes')
           AND ($5::TIMESTAMPTZ IS NULL OR s.started_at >= $5)
           AND ($6::TIMESTAMPTZ IS NULL OR s.started_at <= $6)
         ORDER BY s.updated_at DESC
         LIMIT $7 OFFSET $8",
    )
    .bind(auth.org_id)
    .bind(params.repo_id)
    .bind(&status_filter)
    .bind(use_stale)
    .bind(params.from)
    .bind(params.to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(rows))
}

/// GET /api/v1/orgs/{slug}/traces/sessions/{id}
pub async fn get_session(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, session_id)): Path<(String, Uuid)>,
) -> Result<Json<SessionDetailResponse>, (StatusCode, String)> {
    let session = sqlx::query_as::<_, SessionDetail>(
        "SELECT s.id, s.session_id, r.name AS repo_name, u.email AS user_email,
                s.status, s.model, s.tool, s.total_tool_calls, s.total_tokens,
                s.estimated_cost_usd, s.cwd, s.started_at, s.ended_at, s.updated_at
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE s.id = $1 AND r.org_id = $2",
    )
    .bind(session_id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Session not found".into()))?;

    let events = sqlx::query_as::<_, EventRow>(
        "SELECT id, event_index, event_type, tool_name, tool_input, tool_response, timestamp
         FROM events
         WHERE session_id = $1
         ORDER BY event_index ASC",
    )
    .bind(session_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let file_changes = sqlx::query_as::<_, FileChangeRow>(
        "SELECT id, file_path, change_type, diff_text, content_hash, timestamp
         FROM file_changes
         WHERE session_id = $1
         ORDER BY timestamp ASC",
    )
    .bind(session_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let transcript_chunks = sqlx::query_as::<_, TranscriptChunkRow>(
        "SELECT chunk_index, data
         FROM transcript_chunks
         WHERE session_id = $1
         ORDER BY chunk_index ASC",
    )
    .bind(session_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let linked_commits = sqlx::query_as::<_, LinkedCommitRow>(
        "SELECT ca.commit_id, c.commit_sha, c.branch, MAX(ca.confidence) AS confidence
         FROM commit_attributions ca
         JOIN commits_v2 c ON ca.commit_id = c.id
         WHERE ca.session_id = $1
         GROUP BY ca.commit_id, c.commit_sha, c.branch, c.committed_at
         ORDER BY c.committed_at DESC NULLS LAST",
    )
    .bind(session_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SessionDetailResponse {
        session,
        events,
        file_changes,
        transcript_chunks,
        linked_commits,
    }))
}

/// GET /api/v1/orgs/{slug}/traces/commits
pub async fn list_commits(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(params): Query<CommitListQuery>,
) -> Result<Json<Vec<CommitListItem>>, (StatusCode, String)> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let rows = sqlx::query_as::<_, CommitListItem>(
        "SELECT c.id, c.commit_sha, c.branch, c.author,
                COUNT(DISTINCT ca.file_path) AS files_changed,
                COUNT(DISTINCT ca.session_id) AS ai_sessions_count,
                c.committed_at
         FROM commits_v2 c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN commit_attributions ca ON ca.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::UUID IS NULL OR c.repo_id = $2)
           AND ($3::TEXT IS NULL OR c.branch = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.committed_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.committed_at <= $5)
         GROUP BY c.id
         ORDER BY c.committed_at DESC NULLS LAST
         LIMIT $6 OFFSET $7",
    )
    .bind(auth.org_id)
    .bind(params.repo_id)
    .bind(&params.branch)
    .bind(params.from)
    .bind(params.to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(rows))
}

/// GET /api/v1/orgs/{slug}/traces/commits/{id}
pub async fn get_commit(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, commit_id)): Path<(String, Uuid)>,
) -> Result<Json<CommitDetailResponse>, (StatusCode, String)> {
    let commit = sqlx::query_as::<_, CommitDetail>(
        "SELECT c.id, c.commit_sha, c.branch, c.author, c.committed_at
         FROM commits_v2 c
         JOIN repos r ON c.repo_id = r.id
         WHERE c.id = $1 AND r.org_id = $2",
    )
    .bind(commit_id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Commit not found".into()))?;

    let diff_data: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT diff_data FROM commits_v2 WHERE id = $1")
            .bind(commit_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Fetch attributions grouped by file
    let attributions = sqlx::query_as::<_, (String, Uuid, String, f32, Option<i32>, Option<i32>)>(
        "SELECT ca.file_path, ca.session_id, s.session_id AS session_short_id,
                ca.confidence, ca.line_start, ca.line_end
         FROM commit_attributions ca
         JOIN sessions_v2 s ON ca.session_id = s.id
         WHERE ca.commit_id = $1
         ORDER BY ca.file_path, ca.line_start NULLS LAST",
    )
    .bind(commit_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Group attributions by file_path
    let mut by_file: Vec<AttributionByFile> = Vec::new();
    let mut current_file: Option<String> = None;

    for (file_path, session_id, session_short_id, confidence, line_start, line_end) in attributions
    {
        if current_file.as_deref() != Some(&file_path) {
            by_file.push(AttributionByFile {
                file_path: file_path.clone(),
                sessions: Vec::new(),
            });
            current_file = Some(file_path);
        }
        if let Some(last) = by_file.last_mut() {
            last.sessions.push(AttributionSession {
                session_id,
                session_short_id,
                confidence,
                line_start,
                line_end,
            });
        }
    }

    Ok(Json(CommitDetailResponse {
        commit,
        diff_data,
        attributions_by_file: by_file,
    }))
}

/// Deprecated shim for old POST /api/v1/orgs/{slug}/traces
pub async fn traces_gone() -> (StatusCode, String) {
    (
        StatusCode::GONE,
        "This endpoint is deprecated. Please upgrade your tracevault CLI to use the streaming architecture.".to_string(),
    )
}

// ── Timeline ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    pub repo_id: Option<Uuid>,
    pub tool_name: Option<String>,
    pub session_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TimelineItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub event_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub session_short_id: Option<String>,
    pub event_type: Option<String>,
    pub tool_name: Option<String>,
    pub file_path: Option<String>,
    pub commit_sha: Option<String>,
    pub branch: Option<String>,
    pub author: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// GET /api/v1/orgs/{slug}/traces/timeline
pub async fn get_timeline(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineItem>>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);

    // Build the UNION ALL query
    let rows = sqlx::query_as::<
        _,
        (
            String,         // item_type
            Option<Uuid>,   // event_id
            Option<Uuid>,   // session_id
            Option<String>, // session_short_id
            Option<String>, // event_type
            Option<String>, // tool_name
            Option<String>, // file_path
            Option<String>, // commit_sha
            Option<String>, // branch
            Option<String>, // author
            DateTime<Utc>,  // timestamp
        ),
    >(
        "SELECT * FROM (
            SELECT 'event'::text AS item_type,
                   e.id AS event_id,
                   e.session_id,
                   LEFT(s.session_id, 8) AS session_short_id,
                   e.event_type,
                   e.tool_name,
                   e.tool_input->>'file_path' AS file_path,
                   NULL::text AS commit_sha,
                   NULL::text AS branch,
                   NULL::text AS author,
                   e.timestamp
            FROM events e
            JOIN sessions_v2 s ON e.session_id = s.id
            JOIN repos r ON s.repo_id = r.id
            WHERE r.org_id = $1
              AND ($2::uuid IS NULL OR s.repo_id = $2)
              AND ($3::text IS NULL OR e.tool_name = $3)
              AND ($4::uuid IS NULL OR e.session_id = $4)
              AND ($5::timestamptz IS NULL OR e.timestamp >= $5)
              AND ($6::timestamptz IS NULL OR e.timestamp <= $6)

            UNION ALL

            SELECT 'commit'::text AS item_type,
                   NULL::uuid AS event_id,
                   NULL::uuid AS session_id,
                   NULL::text AS session_short_id,
                   NULL::text AS event_type,
                   NULL::text AS tool_name,
                   NULL::text AS file_path,
                   c.commit_sha,
                   c.branch,
                   c.author,
                   c.committed_at AS timestamp
            FROM commits_v2 c
            JOIN repos r ON c.repo_id = r.id
            WHERE r.org_id = $1
              AND ($2::uuid IS NULL OR c.repo_id = $2)
              AND ($5::timestamptz IS NULL OR c.committed_at >= $5)
              AND ($6::timestamptz IS NULL OR c.committed_at <= $6)
              AND $3::text IS NULL
              AND $4::uuid IS NULL
        ) combined
        ORDER BY timestamp DESC
        LIMIT $7 OFFSET $8",
    )
    .bind(auth.org_id)
    .bind(q.repo_id)
    .bind(&q.tool_name)
    .bind(q.session_id)
    .bind(q.from)
    .bind(q.to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<TimelineItem> = rows
        .into_iter()
        .map(|r| TimelineItem {
            item_type: r.0,
            event_id: r.1,
            session_id: r.2,
            session_short_id: r.3,
            event_type: r.4,
            tool_name: r.5,
            file_path: r.6,
            commit_sha: r.7,
            branch: r.8,
            author: r.9,
            timestamp: r.10,
        })
        .collect();

    Ok(Json(items))
}

// ── Attribution (Code Blame) ────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AttributionResponse {
    pub file_path: String,
    pub commit_sha: String,
    pub lines: Vec<AttributionLine>,
}

#[derive(Debug, Serialize)]
pub struct AttributionLine {
    pub line_number: usize,
    pub content: String,
    pub git_author: Option<String>,
    pub session_id: Option<String>,
    pub session_short_id: Option<String>,
    pub confidence: Option<f32>,
}

/// GET /api/v1/orgs/{slug}/traces/attribution/{commit_id}/{*file_path}
pub async fn get_attribution(
    State(state): State<AppState>,
    auth: OrgAuth,
    Path((_slug, commit_id, file_path)): Path<(String, Uuid, String)>,
) -> Result<Json<AttributionResponse>, (StatusCode, String)> {
    // Get commit + repo info
    let row = sqlx::query_as::<_, (String, Uuid)>(
        "SELECT c.commit_sha, c.repo_id
         FROM commits_v2 c
         JOIN repos r ON c.repo_id = r.id
         WHERE c.id = $1 AND r.org_id = $2",
    )
    .bind(commit_id)
    .bind(auth.org_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or((StatusCode::NOT_FOUND, "Commit not found".to_string()))?;

    let (commit_sha, repo_id) = row;

    // Get clone_path
    let clone_path =
        sqlx::query_scalar::<_, Option<String>>("SELECT clone_path FROM repos WHERE id = $1")
            .bind(repo_id)
            .fetch_one(&state.pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::BAD_REQUEST, "Repo not cloned".to_string()))?;

    // git show {sha}:{path} for file content
    let file_content = std::process::Command::new("git")
        .args(["show", &format!("{commit_sha}:{file_path}")])
        .current_dir(&clone_path)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !file_content.status.success() {
        return Err((
            StatusCode::NOT_FOUND,
            "File not found at this commit".to_string(),
        ));
    }

    let content = String::from_utf8_lossy(&file_content.stdout);
    let content_lines: Vec<&str> = content.lines().collect();

    // git blame --porcelain
    let blame_output = std::process::Command::new("git")
        .args(["blame", "--porcelain", &commit_sha, "--", &file_path])
        .current_dir(&clone_path)
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let blame_text = String::from_utf8_lossy(&blame_output.stdout);
    let blame_map = parse_porcelain_blame(&blame_text);

    // Load attributions for this commit + file
    let attributions = sqlx::query_as::<_, (Option<Uuid>, i32, i32, f32)>(
        "SELECT ca.session_id, ca.line_start, ca.line_end, ca.confidence
         FROM commit_attributions ca
         JOIN sessions_v2 s ON ca.session_id = s.id
         WHERE ca.commit_id = $1 AND ca.file_path = $2",
    )
    .bind(commit_id)
    .bind(&file_path)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Load session short IDs
    let session_ids: Vec<Uuid> = attributions.iter().filter_map(|a| a.0).collect();
    let session_short_ids: std::collections::HashMap<Uuid, String> = if !session_ids.is_empty() {
        sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, LEFT(session_id, 8) FROM sessions_v2 WHERE id = ANY($1)",
        )
        .bind(&session_ids)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .into_iter()
        .collect()
    } else {
        std::collections::HashMap::new()
    };

    // Build line-by-line output
    let mut lines = Vec::new();
    for (i, line_content) in content_lines.iter().enumerate() {
        let line_num = i + 1;
        let git_author = blame_map.get(&line_num).cloned();

        // Check if this line falls in any attribution range
        let mut best_session: Option<Uuid> = None;
        let mut best_confidence: Option<f32> = None;
        for (sid, start, end, conf) in &attributions {
            if line_num as i32 >= *start
                && line_num as i32 <= *end
                && (best_confidence.is_none() || *conf > best_confidence.unwrap())
            {
                best_session = *sid;
                best_confidence = Some(*conf);
            }
        }

        lines.push(AttributionLine {
            line_number: line_num,
            content: line_content.to_string(),
            git_author,
            session_id: best_session.map(|s| s.to_string()),
            session_short_id: best_session.and_then(|s| session_short_ids.get(&s).cloned()),
            confidence: best_confidence,
        });
    }

    Ok(Json(AttributionResponse {
        file_path,
        commit_sha,
        lines,
    }))
}

fn parse_porcelain_blame(text: &str) -> std::collections::HashMap<usize, String> {
    let mut map = std::collections::HashMap::new();
    let mut current_line: usize = 0;
    let mut current_author = String::new();

    for line in text.lines() {
        // Header line: {sha} {orig_line} {final_line} [num_lines]
        if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                current_line = parts[2].parse().unwrap_or(0);
            }
        } else if let Some(author) = line.strip_prefix("author ") {
            current_author = author.to_string();
        } else if line.starts_with('\t') {
            // Content line — record the author for this line number
            if current_line > 0 {
                map.insert(current_line, current_author.clone());
            }
        }
    }

    map
}

// ── Branches ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BranchesQuery {
    pub repo_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct BranchItem {
    pub branch: String,
    pub tag: Option<String>,
    pub commits_count: i64,
    pub sessions_count: i64,
    pub total_cost: f64,
    pub status: String,
    pub last_activity: Option<DateTime<Utc>>,
}

/// GET /api/v1/orgs/{slug}/traces/branches
pub async fn get_branches(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<BranchesQuery>,
) -> Result<Json<Vec<BranchItem>>, (StatusCode, String)> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            Option<String>,
            i64,
            i64,
            Option<f64>,
            String,
            Option<DateTime<Utc>>,
        ),
    >(
        "SELECT
            bt.branch,
            MAX(bt.tag) AS tag,
            COUNT(DISTINCT bt.commit_id) AS commits_count,
            COUNT(DISTINCT ca.session_id) AS sessions_count,
            COALESCE(SUM(DISTINCT s.estimated_cost_usd), 0) AS total_cost,
            MAX(bt.tracking_type) AS status,
            MAX(bt.tracked_at) AS last_activity
         FROM branch_tracking bt
         JOIN commits_v2 c ON bt.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN commit_attributions ca ON ca.commit_id = c.id
         LEFT JOIN sessions_v2 s ON ca.session_id = s.id
         WHERE r.org_id = $1
           AND ($2::uuid IS NULL OR c.repo_id = $2)
         GROUP BY bt.branch
         ORDER BY last_activity DESC NULLS LAST",
    )
    .bind(auth.org_id)
    .bind(q.repo_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items: Vec<BranchItem> = rows
        .into_iter()
        .map(|r| {
            let status = match r.5.as_str() {
                "merge" => "merged",
                "tag" => "tagged",
                _ => "tracked",
            };
            BranchItem {
                branch: r.0,
                tag: r.1,
                commits_count: r.2,
                sessions_count: r.3,
                total_cost: r.4.unwrap_or(0.0),
                status: status.to_string(),
                last_activity: r.6,
            }
        })
        .collect();

    Ok(Json(items))
}
