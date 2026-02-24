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

// --- Tokens endpoint ---

#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub time_series: Vec<TokenTimePoint>,
    pub by_repo: Vec<RepoTokenDetail>,
    pub by_author: Vec<AuthorTokens>,
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
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<TokensResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let time_series = sqlx::query_as::<_, (String, i64, i64)>(
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

    let by_repo = sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
        "SELECT r.name,
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
                COUNT(s.id)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR c.author = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC"
    )
    .bind(org_id)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_author = sqlx::query_as::<_, (String, i64)>(
        "SELECT c.author, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY c.author
         ORDER BY 2 DESC"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TokensResponse {
        time_series: time_series.into_iter().map(|(d, i, o)| TokenTimePoint { date: d, input: i, output: o }).collect(),
        by_repo: by_repo.into_iter().map(|(r, t, i, o, s)| RepoTokenDetail { repo: r, total: t, input: i, output: o, sessions: s }).collect(),
        by_author: by_author.into_iter().map(|(a, t)| AuthorTokens { author: a, total: t }).collect(),
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
}

pub async fn get_models(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<ModelsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let distribution = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT COALESCE(s.model, 'unknown'), COUNT(*), COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
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
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let trends = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'), COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date, s.model
         ORDER BY 1, 2"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let author_model_matrix = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT c.author, COALESCE(s.model, 'unknown'), COUNT(*), COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.author, s.model
         ORDER BY c.author, 3 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comparison = sqlx::query_as::<_, (String, i64, f64)>(
        "SELECT COALESCE(s.model, 'unknown'),
                COALESCE(CAST(AVG(s.total_tokens) AS BIGINT), 0),
                COALESCE(AVG(s.estimated_cost_usd), 0.0)
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
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ModelsResponse {
        distribution: distribution.into_iter().map(|(m, c, t)| ModelDistribution { model: m, session_count: c, total_tokens: t }).collect(),
        trends: trends.into_iter().map(|(d, m, c)| ModelTrend { date: d, model: m, count: c }).collect(),
        author_model_matrix: author_model_matrix.into_iter().map(|(a, m, s, t)| AuthorModel { author: a, model: m, sessions: s, tokens: t }).collect(),
        comparison: comparison.into_iter().map(|(m, t, c)| ModelComparison { model: m, avg_tokens: t, avg_cost: c }).collect(),
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
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AuthorsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let leaderboard = sqlx::query_as::<_, (String, i64, i64, i64, f64, Option<f64>, chrono::DateTime<chrono::Utc>)>(
        "SELECT c.author,
                COUNT(DISTINCT c.id),
                COUNT(s.id),
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                COALESCE(SUM(s.estimated_cost_usd), 0.0),
                AVG(CASE WHEN c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL
                    THEN (c.attribution->'summary'->>'ai_percentage')::float END),
                MAX(c.created_at)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY c.author
         ORDER BY 4 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
         ORDER BY 1, 2"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let model_preferences = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT c.author, COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.author, s.model
         ORDER BY c.author, 3 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthorsResponse {
        leaderboard: leaderboard.into_iter().map(|(a, c, s, t, cost, ai, la)| AuthorLeaderboard {
            author: a, commits: c, sessions: s, tokens: t, cost, ai_pct: ai, last_active: la
        }).collect(),
        timeline: timeline.into_iter().map(|(d, a, c)| AuthorTimeline { date: d, author: a, commits: c }).collect(),
        model_preferences: model_preferences.into_iter().map(|(a, m, s)| AuthorModelPreference { author: a, model: m, sessions: s }).collect(),
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

pub async fn get_attribution(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AttributionResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // Only include commits that have attribution data
    let base_filter = "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";

    let trend = sqlx::query_as::<_, (String, f64)>(
        &format!(
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
        )
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
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
             GROUP BY r.name
             ORDER BY 2 DESC"
        )
    )
    .bind(org_id).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_author = sqlx::query_as::<_, (String, f64)>(
        &format!(
            "SELECT c.author, AVG((c.attribution->'summary'->>'ai_percentage')::float)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
             GROUP BY c.author
             ORDER BY 2 DESC"
        )
    )
    .bind(org_id).bind(&q.repo).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
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
        trend: trend.into_iter().map(|(d, ai)| AttributionTrend { date: d, ai_pct: ai, human_pct: 100.0 - ai }).collect(),
        by_repo: by_repo.into_iter().map(|(r, ai, al, hl)| RepoAttribution { repo: r, ai_pct: ai, ai_lines: al, human_lines: hl }).collect(),
        by_author: by_author.into_iter().map(|(a, ai)| AuthorAttribution { author: a, ai_pct: ai }).collect(),
        totals: AttributionTotals { ai_lines: totals.0, human_lines: totals.1, ai_pct: totals.2 },
    }))
}
