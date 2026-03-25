use crate::extractors::OrgAuth;
use crate::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    fn effective_org_id(&self, auth: &OrgAuth) -> Uuid {
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
    auth: OrgAuth,
) -> Result<Json<FiltersResponse>, (StatusCode, String)> {
    // Get orgs the user belongs to
    let orgs = sqlx::query_as::<_, (Uuid, String)>("SELECT id, name FROM orgs WHERE id = $1")
        .bind(auth.org_id)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let org_id = auth.org_id;

    // Get repos for this org
    let repos = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM repos WHERE org_id = $1 ORDER BY name",
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get distinct authors from sessions_v2 via users
    let authors = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT u.email FROM sessions_v2 s
         JOIN users u ON s.user_id = u.id
         WHERE s.org_id = $1
         ORDER BY u.email",
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(FiltersResponse {
        orgs: orgs
            .into_iter()
            .map(|(id, name)| OrgOption { id, name })
            .collect(),
        repos: repos
            .into_iter()
            .map(|(id, name)| RepoOption { id, name })
            .collect(),
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
    pub total_duration_ms: i64,
    pub avg_session_duration_ms: Option<i64>,
    pub total_tool_calls: i64,
    pub total_compactions: i64,
    pub total_compaction_tokens_saved: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_write_tokens: i64,
    pub cache_savings_usd: f64,
    pub tokens_over_time: Vec<TimeSeriesPoint>,
    pub sessions_over_time: Vec<SessionTimePoint>,
    pub hourly_activity: Vec<HourlyActivity>,
    pub top_repos: Vec<RepoTokens>,
    pub model_distribution: Vec<ModelCount>,
    pub recent_commits: Vec<RecentCommit>,
}

#[derive(Debug, Serialize)]
pub struct SessionTimePoint {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct HourlyActivity {
    pub hour: i32,
    pub count: i64,
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
    auth: OrgAuth,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<OverviewResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // KPI: total sessions, tokens, authors, cost, duration, tool_calls, cache tokens
    let kpi = sqlx::query_as::<
        _,
        (
            i64,
            i64,
            i64,
            i64,
            i64,
            f64,
            i64,
            Option<i64>,
            i64,
            i64,
            i64,
        ),
    >(
        "SELECT
            COUNT(s.id),
            COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
            COUNT(DISTINCT s.user_id),
            COALESCE(SUM(s.estimated_cost_usd), 0.0),
            COALESCE(CAST(SUM(s.duration_ms) AS BIGINT), 0),
            CAST(AVG(s.duration_ms) AS BIGINT),
            COALESCE(CAST(SUM(s.total_tool_calls) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.cache_read_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.cache_write_tokens) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Total commits count (separate query since commits are decoupled from sessions in v2)
    let commit_count = sqlx::query_as::<_, (i64,)>(
        "SELECT COUNT(DISTINCT c.id)
         FROM commits_v2 c
         JOIN commit_attributions ca ON ca.commit_id = c.id
         JOIN sessions_v2 s ON ca.session_id = s.id
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // AI percentage (avg across commits that have attribution) - stays on commits table
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
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)",
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
        "SELECT TO_CHAR(s.created_at::date, 'YYYY-MM-DD'),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY s.created_at::date
         ORDER BY s.created_at::date",
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
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR u.email = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC
         LIMIT 5",
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
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY s.model
         ORDER BY 2 DESC",
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
        "SELECT c.commit_sha, c.author,
                COUNT(DISTINCT ca.session_id),
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                c.created_at
         FROM commits_v2 c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN commit_attributions ca ON ca.commit_id = c.id
         LEFT JOIN sessions_v2 s ON ca.session_id = s.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.id
         ORDER BY c.created_at DESC
         LIMIT 10",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Sessions over time (daily buckets)
    let sessions_time = sqlx::query_as::<_, (String, i64)>(
        "SELECT TO_CHAR(COALESCE(s.started_at, s.created_at)::date, 'YYYY-MM-DD'), COUNT(*)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY COALESCE(s.started_at, s.created_at)::date
         ORDER BY 1",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Hourly activity
    let hourly = sqlx::query_as::<_, (i32, i64)>(
        "SELECT EXTRACT(HOUR FROM s.started_at)::int, COUNT(*)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND s.started_at IS NOT NULL
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY 1
         ORDER BY 1",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // kpi tuple indices:
    // 0=sessions, 1=tokens, 2=input, 3=output, 4=authors, 5=cost,
    // 6=duration, 7=avg_dur, 8=tool_calls, 9=cache_read, 10=cache_write
    let cache_savings = state
        .extensions
        .pricing
        .estimate_cache_savings("sonnet", kpi.9);

    Ok(Json(OverviewResponse {
        total_commits: commit_count.0,
        total_sessions: kpi.0,
        total_tokens: kpi.1,
        total_input_tokens: kpi.2,
        total_output_tokens: kpi.3,
        active_authors: kpi.4,
        estimated_cost_usd: kpi.5,
        ai_percentage: ai_pct.0,
        total_duration_ms: kpi.6,
        avg_session_duration_ms: kpi.7,
        total_tool_calls: kpi.8,
        total_compactions: 0,
        total_compaction_tokens_saved: 0,
        total_cache_read_tokens: kpi.9,
        total_cache_write_tokens: kpi.10,
        cache_savings_usd: cache_savings,
        tokens_over_time: tokens_time
            .into_iter()
            .map(|(d, i, o)| TimeSeriesPoint {
                date: d,
                input: i,
                output: o,
            })
            .collect(),
        sessions_over_time: sessions_time
            .into_iter()
            .map(|(d, c)| SessionTimePoint { date: d, count: c })
            .collect(),
        hourly_activity: hourly
            .into_iter()
            .map(|(h, c)| HourlyActivity { hour: h, count: c })
            .collect(),
        top_repos: top_repos
            .into_iter()
            .map(|(r, t)| RepoTokens { repo: r, tokens: t })
            .collect(),
        model_distribution: models
            .into_iter()
            .map(|(m, c)| ModelCount { model: m, count: c })
            .collect(),
        recent_commits: recent
            .into_iter()
            .map(|(sha, author, sc, tt, ca)| RecentCommit {
                commit_sha: sha,
                author,
                session_count: sc,
                total_tokens: tt,
                created_at: ca,
            })
            .collect(),
    }))
}

// --- Tokens endpoint ---

#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub time_series: Vec<TokenTimePoint>,
    pub by_repo: Vec<RepoTokenDetail>,
    pub by_author: Vec<AuthorTokens>,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub cache_savings_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct TokenTimePoint {
    pub date: String,
    pub input: i64,
    pub output: i64,
}

#[derive(Debug, Serialize)]
pub struct RepoTokenDetail {
    pub repo: String,
    pub total: i64,
    pub input: i64,
    pub output: i64,
    pub sessions: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorTokens {
    pub author: String,
    pub total: i64,
}

pub async fn get_tokens(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<TokensResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let time_series = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT TO_CHAR(s.created_at::date, 'YYYY-MM-DD'),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY s.created_at::date
         ORDER BY s.created_at::date",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_repo = sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
        "SELECT r.name,
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
                COUNT(s.id)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR u.email = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC",
    )
    .bind(org_id)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_author = sqlx::query_as::<_, (String, i64)>(
        "SELECT u.email, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
         GROUP BY u.email
         ORDER BY 2 DESC",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Cache token totals
    let cache_totals = sqlx::query_as::<_, (i64, i64)>(
        "SELECT COALESCE(CAST(SUM(s.cache_read_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.cache_write_tokens) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let cache_savings = state
        .extensions
        .pricing
        .estimate_cache_savings("sonnet", cache_totals.0);

    Ok(Json(TokensResponse {
        time_series: time_series
            .into_iter()
            .map(|(d, i, o)| TokenTimePoint {
                date: d,
                input: i,
                output: o,
            })
            .collect(),
        by_repo: by_repo
            .into_iter()
            .map(|(r, t, i, o, s)| RepoTokenDetail {
                repo: r,
                total: t,
                input: i,
                output: o,
                sessions: s,
            })
            .collect(),
        by_author: by_author
            .into_iter()
            .map(|(a, t)| AuthorTokens {
                author: a,
                total: t,
            })
            .collect(),
        cache_read_tokens: cache_totals.0,
        cache_write_tokens: cache_totals.1,
        cache_savings_usd: cache_savings,
    }))
}

// --- Models endpoint ---

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub distribution: Vec<ModelDistribution>,
    pub trends: Vec<ModelTrend>,
    pub author_model_matrix: Vec<AuthorModel>,
    pub comparison: Vec<ModelComparison>,
}

#[derive(Debug, Serialize)]
pub struct ModelDistribution {
    pub model: String,
    pub session_count: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ModelTrend {
    pub date: String,
    pub model: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorModel {
    pub author: String,
    pub model: String,
    pub sessions: i64,
    pub tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ModelComparison {
    pub model: String,
    pub avg_tokens: i64,
    pub avg_cost: f64,
    pub cache_read_tokens: i64,
    pub cache_write_tokens: i64,
    pub avg_duration_ms: Option<i64>,
}

pub async fn get_models(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<ModelsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // Common CTE using s.model directly (model_usage JSONB not available in v2)
    let model_cte = "WITH model_data AS (
           SELECT u.email as author, s.created_at, r.name as repo_name,
                  COALESCE(s.model, 'unknown') as model,
                  COALESCE(s.input_tokens, 0) + COALESCE(s.output_tokens, 0) as tokens,
                  s.input_tokens,
                  s.output_tokens,
                  s.estimated_cost_usd,
                  COALESCE(s.cache_read_tokens, 0) as cache_read_tokens,
                  COALESCE(s.cache_write_tokens, 0) as cache_write_tokens,
                  s.duration_ms
           FROM sessions_v2 s
           JOIN repos r ON s.repo_id = r.id
           LEFT JOIN users u ON s.user_id = u.id
           WHERE r.org_id = $1
             AND ($2::TEXT IS NULL OR r.name = $2)
             AND ($3::TEXT IS NULL OR u.email = $3)
             AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
             AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         )";

    let distribution = sqlx::query_as::<_, (String, i64, i64)>(
        &format!("{model_cte} SELECT model, COUNT(*), COALESCE(CAST(SUM(tokens) AS BIGINT), 0) FROM model_data GROUP BY model ORDER BY 2 DESC")
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let trends = sqlx::query_as::<_, (String, String, i64)>(
        &format!("{model_cte} SELECT TO_CHAR(created_at::date, 'YYYY-MM-DD'), model, COUNT(*) FROM model_data GROUP BY created_at::date, model ORDER BY 1, 2")
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let author_model_matrix = sqlx::query_as::<_, (String, String, i64, i64)>(
        &format!("{model_cte} SELECT author, model, COUNT(*), COALESCE(CAST(SUM(tokens) AS BIGINT), 0) FROM model_data WHERE author IS NOT NULL GROUP BY author, model ORDER BY author, 3 DESC")
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comparison = sqlx::query_as::<_, (String, i64, f64, i64, i64, Option<i64>)>(
        &format!("{model_cte} SELECT model, COALESCE(CAST(AVG(tokens) AS BIGINT), 0), COALESCE(AVG(estimated_cost_usd), 0.0), COALESCE(CAST(SUM(cache_read_tokens) AS BIGINT), 0), COALESCE(CAST(SUM(cache_write_tokens) AS BIGINT), 0), CAST(AVG(duration_ms) AS BIGINT) FROM model_data GROUP BY model ORDER BY 2 DESC")
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ModelsResponse {
        distribution: distribution
            .into_iter()
            .map(|(m, c, t)| ModelDistribution {
                model: m,
                session_count: c,
                total_tokens: t,
            })
            .collect(),
        trends: trends
            .into_iter()
            .map(|(d, m, c)| ModelTrend {
                date: d,
                model: m,
                count: c,
            })
            .collect(),
        author_model_matrix: author_model_matrix
            .into_iter()
            .map(|(a, m, s, t)| AuthorModel {
                author: a,
                model: m,
                sessions: s,
                tokens: t,
            })
            .collect(),
        comparison: comparison
            .into_iter()
            .map(|(m, t, c, cr, cw, d)| ModelComparison {
                model: m,
                avg_tokens: t,
                avg_cost: c,
                cache_read_tokens: cr,
                cache_write_tokens: cw,
                avg_duration_ms: d,
            })
            .collect(),
    }))
}

// --- Authors endpoint ---

#[derive(Debug, Serialize)]
pub struct AuthorsResponse {
    pub leaderboard: Vec<AuthorLeaderboard>,
    pub timeline: Vec<AuthorTimeline>,
    pub model_preferences: Vec<AuthorModelPreference>,
}

#[derive(Debug, Serialize)]
pub struct AuthorLeaderboard {
    pub author: String,
    pub commits: i64,
    pub sessions: i64,
    pub tokens: i64,
    pub cost: f64,
    pub ai_pct: Option<f64>,
    pub last_active: chrono::DateTime<chrono::Utc>,
    pub avg_duration_ms: Option<i64>,
    pub total_tool_calls: i64,
    pub total_compactions: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorTimeline {
    pub date: String,
    pub author: String,
    pub commits: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorModelPreference {
    pub author: String,
    pub model: String,
    pub sessions: i64,
}

pub async fn get_authors(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AuthorsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // Author leaderboard from sessions_v2 via users
    let leaderboard = sqlx::query_as::<
        _,
        (
            String,
            i64,
            i64,
            f64,
            Option<f64>,
            chrono::DateTime<chrono::Utc>,
            Option<i64>,
            i64,
        ),
    >(
        "SELECT u.email,
                COUNT(s.id),
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                COALESCE(SUM(s.estimated_cost_usd), 0.0),
                NULL::float8,
                MAX(s.created_at),
                CAST(AVG(s.duration_ms) AS BIGINT),
                COALESCE(CAST(SUM(s.total_tool_calls) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
         GROUP BY u.email
         ORDER BY 3 DESC",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Author timeline stays on commits table (counting commits per author per day)
    let timeline = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'), c.author, COUNT(DISTINCT c.id)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date, c.author
         ORDER BY 1, 2",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Model preferences from sessions_v2 (no model_usage JSONB in v2)
    let model_preferences = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT u.email, COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY u.email, s.model
         ORDER BY u.email, 3 DESC",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthorsResponse {
        leaderboard: leaderboard
            .into_iter()
            .map(|(a, s, t, cost, ai, la, dur, tc)| AuthorLeaderboard {
                author: a,
                commits: 0, // commits are decoupled from sessions in v2
                sessions: s,
                tokens: t,
                cost,
                ai_pct: ai,
                last_active: la,
                avg_duration_ms: dur,
                total_tool_calls: tc,
                total_compactions: 0, // not tracked in v2
            })
            .collect(),
        timeline: timeline
            .into_iter()
            .map(|(d, a, c)| AuthorTimeline {
                date: d,
                author: a,
                commits: c,
            })
            .collect(),
        model_preferences: model_preferences
            .into_iter()
            .map(|(a, m, s)| AuthorModelPreference {
                author: a,
                model: m,
                sessions: s,
            })
            .collect(),
    }))
}

// --- Attribution endpoint ---

#[derive(Debug, Serialize)]
pub struct AttributionResponse {
    pub trend: Vec<AttributionTrend>,
    pub by_repo: Vec<RepoAttribution>,
    pub by_author: Vec<AuthorAttribution>,
    pub totals: AttributionTotals,
}

#[derive(Debug, Serialize)]
pub struct AttributionTrend {
    pub date: String,
    pub ai_pct: f64,
    pub human_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct RepoAttribution {
    pub repo: String,
    pub ai_pct: f64,
    pub ai_lines: i64,
    pub human_lines: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorAttribution {
    pub author: String,
    pub ai_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct AttributionTotals {
    pub ai_lines: i64,
    pub human_lines: i64,
    pub ai_pct: f64,
}

// Attribution stays on the commits table (attribution JSONB is only on old commits)
pub async fn get_attribution(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AttributionResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // Only include commits that have attribution data
    let base_filter =
        "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";

    let trend = sqlx::query_as::<_, (String, f64)>(&format!(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'),
                    AVG((c.attribution->'summary'->>'ai_percentage')::float)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR c.author = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
             GROUP BY c.created_at::date
             ORDER BY 1"
    ))
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_repo = sqlx::query_as::<_, (String, f64, i64, i64)>(
        &format!(
            "SELECT r.name,
                    AVG((c.attribution->'summary'->>'ai_percentage')::float),
                    COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_added')::bigint) AS BIGINT), 0),
                    COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_deleted')::bigint) AS BIGINT), 0)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR c.author = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
             Group BY r.name
             ORDER BY 2 DESC"
        )
    )
    .bind(org_id).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_author = sqlx::query_as::<_, (String, f64)>(&format!(
        "SELECT c.author, AVG((c.attribution->'summary'->>'ai_percentage')::float)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
             GROUP BY c.author
             ORDER BY 2 DESC"
    ))
    .bind(org_id)
    .bind(&q.repo)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let totals = sqlx::query_as::<_, (i64, i64, f64)>(
        &format!(
            "SELECT
                COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_added')::bigint) AS BIGINT), 0),
                COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_deleted')::bigint) AS BIGINT), 0),
                COALESCE(AVG((c.attribution->'summary'->>'ai_percentage')::float), 0.0)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR c.author = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)"
        )
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_one(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AttributionResponse {
        trend: trend
            .into_iter()
            .map(|(d, ai)| AttributionTrend {
                date: d,
                ai_pct: ai,
                human_pct: 100.0 - ai,
            })
            .collect(),
        by_repo: by_repo
            .into_iter()
            .map(|(r, ai, al, hl)| RepoAttribution {
                repo: r,
                ai_pct: ai,
                ai_lines: al,
                human_lines: hl,
            })
            .collect(),
        by_author: by_author
            .into_iter()
            .map(|(a, ai)| AuthorAttribution {
                author: a,
                ai_pct: ai,
            })
            .collect(),
        totals: AttributionTotals {
            ai_lines: totals.0,
            human_lines: totals.1,
            ai_pct: totals.2,
        },
    }))
}

// --- Sessions endpoint ---

#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    pub org_id: Option<Uuid>,
    pub repo: Option<String>,
    pub author: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl SessionsQuery {
    fn effective_org_id(&self, auth: &OrgAuth) -> Uuid {
        self.org_id.unwrap_or(auth.org_id)
    }
}

#[derive(Debug, Serialize)]
pub struct SessionsResponse {
    pub sessions: Vec<SessionItem>,
    pub tool_frequency: serde_json::Value,
    pub avg_duration_ms: Option<i64>,
    pub avg_messages_per_session: Option<f64>,
    pub total_sessions: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionItem {
    pub id: Uuid,
    pub session_id: String,
    pub model: Option<String>,
    pub duration_ms: Option<i64>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub user_messages: Option<i32>,
    pub assistant_messages: Option<i32>,
    pub total_tool_calls: Option<i32>,
    pub total_tokens: Option<i64>,
    pub estimated_cost_usd: Option<f64>,
    pub author: String,
    pub repo_name: String,
}

pub async fn get_sessions(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<SessionsQuery>,
) -> Result<Json<SessionsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = q.offset.unwrap_or(0);

    // Paginated session list
    let sessions = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            Option<String>,
            Option<i64>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i64>,
            Option<f64>,
            String,
            String,
        ),
    >(
        "SELECT s.id, s.session_id, s.model,
                COALESCE(NULLIF(s.duration_ms, 0), EXTRACT(EPOCH FROM (COALESCE(s.ended_at, NOW()) - s.started_at))::BIGINT * 1000),
                s.started_at, s.ended_at,
                CASE WHEN COALESCE(s.user_messages, 0) = 0
                     THEN (SELECT COUNT(*) FROM transcript_chunks tc WHERE tc.session_id = s.id AND tc.data->>'type' = 'human')::INT
                     ELSE s.user_messages END,
                CASE WHEN COALESCE(s.assistant_messages, 0) = 0
                     THEN (SELECT COUNT(*) FROM transcript_chunks tc WHERE tc.session_id = s.id AND tc.data->>'type' = 'assistant')::INT
                     ELSE s.assistant_messages END,
                s.total_tool_calls,
                s.total_tokens, s.estimated_cost_usd,
                u.email, r.name
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         ORDER BY s.created_at DESC
         LIMIT $6 OFFSET $7",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Aggregates
    let agg = sqlx::query_as::<_, (i64, Option<i64>, Option<f64>)>(
        "SELECT COUNT(s.id),
                CAST(AVG(COALESCE(NULLIF(s.duration_ms, 0), EXTRACT(EPOCH FROM (COALESCE(s.ended_at, NOW()) - s.started_at))::BIGINT * 1000)) AS BIGINT),
                AVG(
                    CASE WHEN COALESCE(s.user_messages, 0) = 0
                         THEN (SELECT COUNT(*) FROM transcript_chunks tc WHERE tc.session_id = s.id AND tc.data->>'type' = 'human')::INT
                         ELSE COALESCE(s.user_messages, 0) END
                    + CASE WHEN COALESCE(s.assistant_messages, 0) = 0
                           THEN (SELECT COUNT(*) FROM transcript_chunks tc WHERE tc.session_id = s.id AND tc.data->>'type' = 'assistant')::INT
                           ELSE COALESCE(s.assistant_messages, 0) END
                )::float8
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Tool frequency from events table
    let tool_freq_rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT e.tool_name, COUNT(*) as cnt
         FROM events e
         JOIN sessions_v2 s ON e.session_id = s.id
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND e.tool_name IS NOT NULL
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY e.tool_name
         ORDER BY cnt DESC",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let tool_frequency: serde_json::Value = tool_freq_rows
        .into_iter()
        .map(|(name, count)| (name, serde_json::Value::from(count)))
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();

    Ok(Json(SessionsResponse {
        sessions: sessions
            .into_iter()
            .map(|s| SessionItem {
                id: s.0,
                session_id: s.1,
                model: s.2,
                duration_ms: s.3,
                started_at: s.4,
                ended_at: s.5,
                user_messages: s.6,
                assistant_messages: s.7,
                total_tool_calls: s.8,
                total_tokens: s.9,
                estimated_cost_usd: s.10,
                author: s.11,
                repo_name: s.12,
            })
            .collect(),
        tool_frequency,
        avg_duration_ms: agg.1,
        avg_messages_per_session: agg.2,
        total_sessions: agg.0,
    }))
}

// --- Cost endpoint ---

#[derive(Debug, Serialize)]
pub struct CostResponse {
    pub total_cost: f64,
    pub avg_cost_per_session: f64,
    pub cache_savings_usd: f64,
    pub cost_over_time: Vec<CostTimePoint>,
    pub cost_by_model: Vec<ModelCost>,
    pub cost_by_repo: Vec<RepoCost>,
    pub cost_by_author: Vec<AuthorCost>,
}

#[derive(Debug, Serialize)]
pub struct CostTimePoint {
    pub date: String,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct ModelCost {
    pub model: String,
    pub cost: f64,
    pub tokens: i64,
    pub sessions: i64,
}

#[derive(Debug, Serialize)]
pub struct RepoCost {
    pub repo: String,
    pub cost: f64,
}

#[derive(Debug, Serialize)]
pub struct AuthorCost {
    pub author: String,
    pub cost: f64,
}

pub async fn get_cost(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<CostResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // Total cost and avg cost per session
    let totals = sqlx::query_as::<_, (f64, f64, i64)>(
        "SELECT COALESCE(SUM(s.estimated_cost_usd), 0.0),
                COALESCE(AVG(s.estimated_cost_usd), 0.0),
                COALESCE(CAST(SUM(s.cache_read_tokens) AS BIGINT), 0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Approximate cache savings using Sonnet rates for aggregate
    let cache_savings = state
        .extensions
        .pricing
        .estimate_cache_savings("sonnet", totals.2);

    // Cost over time (daily)
    let cost_time = sqlx::query_as::<_, (String, f64)>(
        "SELECT TO_CHAR(s.created_at::date, 'YYYY-MM-DD'),
                COALESCE(SUM(s.estimated_cost_usd), 0.0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY s.created_at::date
         ORDER BY 1",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Cost by model (using s.model directly, no model_usage JSONB in v2)
    let cost_model = sqlx::query_as::<_, (String, f64, i64, i64)>(
        "SELECT COALESCE(s.model, 'unknown'),
                COALESCE(SUM(s.estimated_cost_usd), 0.0),
                COALESCE(CAST(SUM(COALESCE(s.input_tokens, 0) + COALESCE(s.output_tokens, 0)) AS BIGINT), 0),
                COUNT(*)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR u.email = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
         GROUP BY s.model
         ORDER BY 2 DESC",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Cost by repo
    let cost_repo = sqlx::query_as::<_, (String, f64)>(
        "SELECT r.name, COALESCE(SUM(s.estimated_cost_usd), 0.0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         LEFT JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR u.email = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC",
    )
    .bind(org_id)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Cost by author
    let cost_author = sqlx::query_as::<_, (String, f64)>(
        "SELECT u.email, COALESCE(SUM(s.estimated_cost_usd), 0.0)
         FROM sessions_v2 s
         JOIN repos r ON s.repo_id = r.id
         JOIN users u ON s.user_id = u.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
         GROUP BY u.email
         ORDER BY 2 DESC",
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(CostResponse {
        total_cost: totals.0,
        avg_cost_per_session: totals.1,
        cache_savings_usd: cache_savings,
        cost_over_time: cost_time
            .into_iter()
            .map(|(d, c)| CostTimePoint { date: d, cost: c })
            .collect(),
        cost_by_model: cost_model
            .into_iter()
            .map(|(m, c, t, s)| ModelCost {
                model: m,
                cost: c,
                tokens: t,
                sessions: s,
            })
            .collect(),
        cost_by_repo: cost_repo
            .into_iter()
            .map(|(r, c)| RepoCost { repo: r, cost: c })
            .collect(),
        cost_by_author: cost_author
            .into_iter()
            .map(|(a, c)| AuthorCost { author: a, cost: c })
            .collect(),
    }))
}
