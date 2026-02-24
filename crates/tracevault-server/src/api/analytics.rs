use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::AppState;
use crate::extractors::AuthUser;

// Shared query params for all analytics endpoints
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub org_id: Option<Uuid>,
    pub repo: Option<String>,
    pub author: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

impl AnalyticsQuery {
    /// Returns the effective org_id, falling back to the auth user's org
    fn effective_org_id(&self, auth: &AuthUser) -> Uuid {
        self.org_id.unwrap_or(auth.org_id)
    }
}

// --- Filters endpoint ---

#[derive(Debug, Serialize)]
pub struct FiltersResponse {
    pub orgs: Vec<OrgOption>,
    pub repos: Vec<RepoOption>,
    pub authors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OrgOption {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RepoOption {
    pub id: Uuid,
    pub name: String,
}

pub async fn get_filters(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<FiltersResponse>, (StatusCode, String)> {
    // Get orgs the user belongs to
    let orgs = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM orgs WHERE id = $1"
    )
    .bind(auth.org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let org_id = auth.org_id;

    // Get repos for this org
    let repos = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM repos WHERE org_id = $1 ORDER BY name"
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get distinct authors from commits in this org's repos
    let authors = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT c.author FROM commits c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
         ORDER BY c.author"
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(FiltersResponse {
        orgs: orgs.into_iter().map(|(id, name)| OrgOption { id, name }).collect(),
        repos: repos.into_iter().map(|(id, name)| RepoOption { id, name }).collect(),
        authors: authors.into_iter().map(|(a,)| a).collect(),
    }))
}

// --- Overview endpoint ---

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    pub total_commits: i64,
    pub total_sessions: i64,
    pub total_tokens: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub active_authors: i64,
    pub estimated_cost_usd: f64,
    pub ai_percentage: Option<f64>,
    pub tokens_over_time: Vec<TimeSeriesPoint>,
    pub top_repos: Vec<RepoTokens>,
    pub model_distribution: Vec<ModelCount>,
    pub recent_commits: Vec<RecentCommit>,
}

#[derive(Debug, Serialize)]
pub struct TimeSeriesPoint {
    pub date: String,
    pub input: i64,
    pub output: i64,
}

#[derive(Debug, Serialize)]
pub struct RepoTokens {
    pub repo: String,
    pub tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ModelCount {
    pub model: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct RecentCommit {
    pub commit_sha: String,
    pub author: String,
    pub session_count: i64,
    pub total_tokens: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_overview(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<OverviewResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // KPI: total commits, sessions, tokens, authors, cost
    let kpi = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, f64)>(
        "SELECT
            COUNT(DISTINCT c.id),
            COUNT(s.id),
            COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
            COUNT(DISTINCT c.author),
            COALESCE(SUM(s.estimated_cost_usd), 0.0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // AI percentage (avg across commits that have attribution)
    let ai_pct = sqlx::query_as::<_, (Option<f64>,)>(
        "SELECT AVG((c.attribution->'summary'->>'ai_percentage')::float)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND c.attribution IS NOT NULL
           AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Tokens over time (daily buckets)
    let tokens_time = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date
         ORDER BY c.created_at::date"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Top 5 repos by tokens
    let top_repos = sqlx::query_as::<_, (String, i64)>(
        "SELECT r.name, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR c.author = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC
         LIMIT 5"
    )
    .bind(org_id)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Model distribution
    let models = sqlx::query_as::<_, (String, i64)>(
        "SELECT COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY s.model
         ORDER BY 2 DESC"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Recent 10 commits
    let recent = sqlx::query_as::<_, (String, String, i64, i64, chrono::DateTime<chrono::Utc>)>(
        "SELECT c.commit_sha, c.author, COUNT(s.id), COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0), c.created_at
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.id
         ORDER BY c.created_at DESC
         LIMIT 10"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(OverviewResponse {
        total_commits: kpi.0,
        total_sessions: kpi.1,
        total_tokens: kpi.2,
        total_input_tokens: kpi.3,
        total_output_tokens: kpi.4,
        active_authors: kpi.5,
        estimated_cost_usd: kpi.6,
        ai_percentage: ai_pct.0,
        tokens_over_time: tokens_time.into_iter().map(|(d, i, o)| TimeSeriesPoint { date: d, input: i, output: o }).collect(),
        top_repos: top_repos.into_iter().map(|(r, t)| RepoTokens { repo: r, tokens: t }).collect(),
        model_distribution: models.into_iter().map(|(m, c)| ModelCount { model: m, count: c }).collect(),
        recent_commits: recent.into_iter().map(|(sha, author, sc, tt, ca)| RecentCommit {
            commit_sha: sha, author, session_count: sc, total_tokens: tt, created_at: ca
        }).collect(),
    }))
}
