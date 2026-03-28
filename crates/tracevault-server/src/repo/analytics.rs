use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

/// Shared filter struct for analytics queries with optional parameters.
#[derive(Debug, Default)]
pub struct AnalyticsFilters {
    pub org_id: Uuid,
    pub repo: Option<String>,
    pub author: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

pub struct AnalyticsRepo;

impl AnalyticsRepo {
    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_filters
    // ═══════════════════════════════════════════════════════════════════

    pub async fn get_orgs(pool: &PgPool, org_id: Uuid) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String)>("SELECT id, name FROM orgs WHERE id = $1")
            .bind(org_id)
            .fetch_all(pool)
            .await?;
        Ok(rows)
    }

    pub async fn get_repos_for_org(
        pool: &PgPool,
        org_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, name FROM repos WHERE org_id = $1 ORDER BY name",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_distinct_authors(
        pool: &PgPool,
        org_id: Uuid,
    ) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT DISTINCT u.email FROM sessions s
             JOIN users u ON s.user_id = u.id
             WHERE s.org_id = $1
             ORDER BY u.email",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|(a,)| a).collect())
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_overview
    // ═══════════════════════════════════════════════════════════════════

    /// Main KPI aggregations: sessions, tokens, authors, cost, duration, tool_calls, cache tokens.
    pub async fn get_overview_kpi(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<
        (
            i64,         // sessions
            i64,         // total_tokens
            i64,         // input_tokens
            i64,         // output_tokens
            i64,         // active_authors
            f64,         // estimated_cost_usd
            i64,         // total_duration_ms
            Option<i64>, // avg_session_duration_ms
            i64,         // total_tool_calls
            i64,         // cache_read_tokens
            i64,         // cache_write_tokens
        ),
        AppError,
    > {
        let row = sqlx::query_as::<
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
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Total distinct commits linked via attributions.
    pub async fn get_overview_commit_count(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<i64, AppError> {
        let (count,) = sqlx::query_as::<_, (i64,)>(
            "SELECT COUNT(DISTINCT c.id)
             FROM commits c
             JOIN commit_attributions ca ON ca.commit_id = c.id
             JOIN sessions s ON ca.session_id = s.id
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Average AI percentage from commits with attribution data.
    pub async fn get_overview_ai_pct(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Option<f64>, AppError> {
        let (pct,) = sqlx::query_as::<_, (Option<f64>,)>(
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(pct)
    }

    /// Daily token time series (input/output).
    pub async fn get_tokens_over_time(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT TO_CHAR(s.created_at::date, 'YYYY-MM-DD'),
                    COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                    COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Top 5 repos by total tokens.
    pub async fn get_top_repos(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT r.name, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Model distribution (model name, count).
    pub async fn get_model_distribution(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT COALESCE(s.model, 'unknown'), COUNT(*)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Recent 10 commits with session/token info.
    pub async fn get_recent_commits(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64, i64, DateTime<Utc>)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64, i64, DateTime<Utc>)>(
            "SELECT c.commit_sha, c.author,
                    COUNT(DISTINCT ca.session_id),
                    COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                    c.created_at
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             LEFT JOIN commit_attributions ca ON ca.commit_id = c.id
             LEFT JOIN sessions s ON ca.session_id = s.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR c.author = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
             GROUP BY c.id
             ORDER BY c.created_at DESC
             LIMIT 10",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Sessions over time (daily buckets).
    pub async fn get_sessions_over_time(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT TO_CHAR(COALESCE(s.started_at, s.created_at)::date, 'YYYY-MM-DD'), COUNT(*)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Hourly activity distribution.
    pub async fn get_hourly_activity(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(i32, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (i32, i64)>(
            "SELECT EXTRACT(HOUR FROM s.started_at)::int, COUNT(*)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_tokens
    // ═══════════════════════════════════════════════════════════════════

    /// Token breakdown by repo.
    pub async fn get_tokens_by_repo(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64, i64, i64, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
            "SELECT r.name,
                    COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                    COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                    COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
                    COUNT(s.id)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR u.email = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             GROUP BY r.name
             ORDER BY 2 DESC",
        )
        .bind(f.org_id)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Token breakdown by author.
    pub async fn get_tokens_by_author(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT u.email, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             GROUP BY u.email
             ORDER BY 2 DESC",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Cache token totals (read, write).
    pub async fn get_cache_totals(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<(i64, i64), AppError> {
        let row = sqlx::query_as::<_, (i64, i64)>(
            "SELECT COALESCE(CAST(SUM(s.cache_read_tokens) AS BIGINT), 0),
                    COALESCE(CAST(SUM(s.cache_write_tokens) AS BIGINT), 0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_models
    // ═══════════════════════════════════════════════════════════════════

    /// Model distribution with token totals.
    pub async fn get_models_distribution(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64, i64)>, AppError> {
        let cte = model_cte();
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            &format!("{cte} SELECT model, COUNT(*), COALESCE(CAST(SUM(tokens) AS BIGINT), 0) FROM model_data GROUP BY model ORDER BY 2 DESC"),
        )
        .bind(f.org_id).bind(&f.repo).bind(&f.author).bind(f.from).bind(f.to)
        .fetch_all(pool).await?;
        Ok(rows)
    }

    /// Model trends over time.
    pub async fn get_models_trends(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64)>, AppError> {
        let cte = model_cte();
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            &format!("{cte} SELECT TO_CHAR(created_at::date, 'YYYY-MM-DD'), model, COUNT(*) FROM model_data GROUP BY created_at::date, model ORDER BY 1, 2"),
        )
        .bind(f.org_id).bind(&f.repo).bind(&f.author).bind(f.from).bind(f.to)
        .fetch_all(pool).await?;
        Ok(rows)
    }

    /// Author-model matrix.
    pub async fn get_author_model_matrix(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64, i64)>, AppError> {
        let cte = model_cte();
        let rows = sqlx::query_as::<_, (String, String, i64, i64)>(
            &format!("{cte} SELECT author, model, COUNT(*), COALESCE(CAST(SUM(tokens) AS BIGINT), 0) FROM model_data WHERE author IS NOT NULL GROUP BY author, model ORDER BY author, 3 DESC"),
        )
        .bind(f.org_id).bind(&f.repo).bind(&f.author).bind(f.from).bind(f.to)
        .fetch_all(pool).await?;
        Ok(rows)
    }

    /// Model comparison (avg tokens, avg cost, cache tokens, avg duration).
    pub async fn get_model_comparison(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64, f64, i64, i64, Option<i64>)>, AppError> {
        let cte = model_cte();
        let rows = sqlx::query_as::<_, (String, i64, f64, i64, i64, Option<i64>)>(
            &format!("{cte} SELECT model, COALESCE(CAST(AVG(tokens) AS BIGINT), 0), COALESCE(AVG(estimated_cost_usd), 0.0), COALESCE(CAST(SUM(cache_read_tokens) AS BIGINT), 0), COALESCE(CAST(SUM(cache_write_tokens) AS BIGINT), 0), CAST(AVG(duration_ms) AS BIGINT) FROM model_data GROUP BY model ORDER BY 2 DESC"),
        )
        .bind(f.org_id).bind(&f.repo).bind(&f.author).bind(f.from).bind(f.to)
        .fetch_all(pool).await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_authors
    // ═══════════════════════════════════════════════════════════════════

    /// Author leaderboard.
    pub async fn get_author_leaderboard(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<
        Vec<(
            Uuid,
            String,
            i64,
            i64,
            f64,
            Option<f64>,
            DateTime<Utc>,
            Option<i64>,
            i64,
        )>,
        AppError,
    > {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                i64,
                i64,
                f64,
                Option<f64>,
                DateTime<Utc>,
                Option<i64>,
                i64,
            ),
        >(
            "SELECT u.id,
                    u.email,
                    COUNT(s.id),
                    COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                    COALESCE(SUM(s.estimated_cost_usd), 0.0),
                    NULL::float8,
                    MAX(s.created_at),
                    CAST(AVG(s.duration_ms) AS BIGINT),
                    COALESCE(CAST(SUM(s.total_tool_calls) AS BIGINT), 0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             GROUP BY u.id, u.email
             ORDER BY 3 DESC",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Author timeline (commits per author per day).
    pub async fn get_author_timeline(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Author model preferences.
    pub async fn get_author_model_preferences(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT u.email, COALESCE(s.model, 'unknown'), COUNT(*)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_author_detail
    // ═══════════════════════════════════════════════════════════════════

    /// Get basic user info by ID.
    pub async fn get_user_info(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<(Uuid, String, Option<String>), AppError> {
        let row = sqlx::query_as::<_, (Uuid, String, Option<String>)>(
            "SELECT id, email, name FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Author detail stats.
    pub async fn get_author_stats(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        f: &AnalyticsFilters,
    ) -> Result<(i64, i64, f64, Option<i64>, i64), AppError> {
        let row = sqlx::query_as::<_, (i64, i64, f64, Option<i64>, i64)>(
            "SELECT COUNT(*),
                    COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                    COALESCE(SUM(s.estimated_cost_usd), 0.0),
                    CAST(AVG(s.duration_ms) AS BIGINT),
                    COALESCE(CAST(SUM(s.total_tool_calls) AS BIGINT), 0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE r.org_id = $1 AND s.user_id = $2
               AND ($3::TEXT IS NULL OR r.name = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Author model preferences for detail page.
    pub async fn get_author_detail_models(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT COALESCE(s.model, 'unknown'), COUNT(*)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE r.org_id = $1 AND s.user_id = $2
               AND ($3::TEXT IS NULL OR r.name = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
             GROUP BY s.model
             ORDER BY 2 DESC",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Top software for an author.
    pub async fn get_author_top_software(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<String>, AppError> {
        let rows = sqlx::query_as::<_, (String,)>(
            "SELECT software_name
             FROM user_software_usage
             WHERE org_id = $1 AND user_id = $2
               AND ($3::TIMESTAMPTZ IS NULL OR last_seen_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR first_seen_at <= $4)
             GROUP BY software_name
             ORDER BY SUM(usage_count) DESC
             LIMIT 5",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    /// Top AI tools for an author.
    pub async fn get_author_top_ai_tools(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT tool_category, tool_name, SUM(usage_count)::BIGINT
             FROM user_ai_tool_usage
             WHERE org_id = $1 AND user_id = $2
               AND ($3::TIMESTAMPTZ IS NULL OR last_seen_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR first_seen_at <= $4)
             GROUP BY tool_category, tool_name
             ORDER BY SUM(usage_count) DESC
             LIMIT 10",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Recent sessions for an author.
    pub async fn get_author_recent_sessions(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        f: &AnalyticsFilters,
    ) -> Result<
        Vec<(
            Uuid,
            String,
            String,
            Option<DateTime<Utc>>,
            Option<i64>,
            Option<f64>,
            Option<String>,
        )>,
        AppError,
    > {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                Option<DateTime<Utc>>,
                Option<i64>,
                Option<f64>,
                Option<String>,
            ),
        >(
            "SELECT s.id, s.session_id, r.name,
                    s.started_at, s.duration_ms, s.estimated_cost_usd, s.model
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE s.org_id = $1 AND s.user_id = $2
               AND ($3::TEXT IS NULL OR r.name = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
             ORDER BY s.created_at DESC
             LIMIT 20",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_attribution
    // ═══════════════════════════════════════════════════════════════════

    /// Attribution trend (daily AI percentage).
    pub async fn get_attribution_trend(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64)>, AppError> {
        let base_filter =
            "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";
        let rows = sqlx::query_as::<_, (String, f64)>(&format!(
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Attribution by repo.
    pub async fn get_attribution_by_repo(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64, i64, i64)>, AppError> {
        let base_filter =
            "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";
        let rows = sqlx::query_as::<_, (String, f64, i64, i64)>(
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
            ),
        )
        .bind(f.org_id)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Attribution by author.
    pub async fn get_attribution_by_author(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64)>, AppError> {
        let base_filter =
            "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";
        let rows = sqlx::query_as::<_, (String, f64)>(&format!(
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Attribution totals (ai_lines, human_lines, ai_pct).
    pub async fn get_attribution_totals(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<(i64, i64, f64), AppError> {
        let base_filter =
            "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";
        let row = sqlx::query_as::<_, (i64, i64, f64)>(
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
            ),
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_sessions (sessions analytics)
    // ═══════════════════════════════════════════════════════════════════

    /// Paginated session list for analytics.
    #[allow(clippy::type_complexity)]
    pub async fn get_sessions_list(
        pool: &PgPool,
        f: &AnalyticsFilters,
        limit: i64,
        offset: i64,
    ) -> Result<
        Vec<(
            Uuid,
            String,
            Option<String>,
            Option<i64>,
            Option<DateTime<Utc>>,
            Option<DateTime<Utc>>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i64>,
            Option<f64>,
            String,
            String,
        )>,
        AppError,
    > {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                Option<String>,
                Option<i64>,
                Option<DateTime<Utc>>,
                Option<DateTime<Utc>>,
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
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Session aggregates (count, avg duration, avg messages).
    pub async fn get_sessions_aggregates(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<(i64, Option<i64>, Option<f64>), AppError> {
        let row = sqlx::query_as::<_, (i64, Option<i64>, Option<f64>)>(
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
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Tool frequency from events table.
    pub async fn get_tool_frequency(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT e.tool_name, COUNT(*) as cnt
             FROM events e
             JOIN sessions s ON e.session_id = s.id
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_cost
    // ═══════════════════════════════════════════════════════════════════

    /// Cost totals: total cost, avg cost per session, cache read tokens.
    pub async fn get_cost_totals(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<(f64, f64, i64), AppError> {
        let row = sqlx::query_as::<_, (f64, f64, i64)>(
            "SELECT COALESCE(SUM(s.estimated_cost_usd), 0.0),
                    COALESCE(AVG(s.estimated_cost_usd), 0.0),
                    COALESCE(CAST(SUM(s.cache_read_tokens) AS BIGINT), 0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Cost over time (daily).
    pub async fn get_cost_over_time(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, f64)>(
            "SELECT TO_CHAR(s.created_at::date, 'YYYY-MM-DD'),
                    COALESCE(SUM(s.estimated_cost_usd), 0.0)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Cost by model.
    pub async fn get_cost_by_model(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64, i64, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, f64, i64, i64)>(
            "SELECT COALESCE(s.model, 'unknown'),
                    COALESCE(SUM(s.estimated_cost_usd), 0.0),
                    COALESCE(CAST(SUM(COALESCE(s.input_tokens, 0) + COALESCE(s.output_tokens, 0)) AS BIGINT), 0),
                    COUNT(*)
             FROM sessions s
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
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Cost by repo.
    pub async fn get_cost_by_repo(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, f64)>(
            "SELECT r.name, COALESCE(SUM(s.estimated_cost_usd), 0.0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR u.email = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             GROUP BY r.name
             ORDER BY 2 DESC",
        )
        .bind(f.org_id)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Cost by author.
    pub async fn get_cost_by_author(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, f64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, f64)>(
            "SELECT u.email, COALESCE(SUM(s.estimated_cost_usd), 0.0)
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             JOIN users u ON s.user_id = u.id
             WHERE r.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             GROUP BY u.email
             ORDER BY 2 DESC",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_software
    // ═══════════════════════════════════════════════════════════════════

    /// Org-level software usage.
    pub async fn get_org_top_tools(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, i64, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT usu.software_name, SUM(usu.usage_count) AS count, COUNT(DISTINCT usu.user_id) AS users
             FROM user_software_usage usu
             JOIN sessions s ON usu.session_id = s.id
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON usu.user_id = u.id
             WHERE usu.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR usu.last_seen_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR usu.first_seen_at <= $5)
             GROUP BY usu.software_name
             ORDER BY count DESC",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// AI tools summary (category, name, count).
    pub async fn get_ai_tools_summary(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64)>(
            "SELECT atu.tool_category, atu.tool_name, SUM(atu.usage_count) AS count
             FROM user_ai_tool_usage atu
             JOIN sessions s ON atu.session_id = s.id
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON atu.user_id = u.id
             WHERE atu.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR atu.last_seen_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR atu.first_seen_at <= $5)
             GROUP BY atu.tool_category, atu.tool_name
             ORDER BY count DESC",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_software_user_detail
    // ═══════════════════════════════════════════════════════════════════

    /// Software usage per user.
    pub async fn get_user_software(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, i64, DateTime<Utc>, DateTime<Utc>, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, i64, DateTime<Utc>, DateTime<Utc>, i64)>(
            "SELECT software_name,
                    SUM(usage_count) AS usage_count,
                    MIN(first_seen_at) AS first_seen,
                    MAX(last_seen_at) AS last_seen,
                    COUNT(DISTINCT session_id) AS session_count
             FROM user_software_usage
             WHERE org_id = $1 AND user_id = $2
               AND ($3::TIMESTAMPTZ IS NULL OR last_seen_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR first_seen_at <= $4)
             GROUP BY software_name
             ORDER BY usage_count DESC",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Recent sessions for a user.
    pub async fn get_user_recent_sessions(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(Uuid, String, String, Option<DateTime<Utc>>, Option<i64>)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, Option<DateTime<Utc>>, Option<i64>)>(
            "SELECT s.id, s.session_id, r.name,
                    s.started_at, s.duration_ms
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE s.org_id = $1 AND s.user_id = $2
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             ORDER BY s.created_at DESC
             LIMIT 20",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Software tools for a batch of sessions.
    pub async fn get_session_software_tools(
        pool: &PgPool,
        session_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT session_id, software_name
             FROM user_software_usage
             WHERE session_id = ANY($1)
             ORDER BY session_id, usage_count DESC",
        )
        .bind(session_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_ai_tools
    // ═══════════════════════════════════════════════════════════════════

    /// AI tools with category, name, count, users.
    pub async fn get_ai_tools(
        pool: &PgPool,
        f: &AnalyticsFilters,
    ) -> Result<Vec<(String, String, i64, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64, i64)>(
            "SELECT atu.tool_category, atu.tool_name, SUM(atu.usage_count) AS count, COUNT(DISTINCT atu.user_id) AS users
             FROM user_ai_tool_usage atu
             JOIN sessions s ON atu.session_id = s.id
             JOIN repos r ON s.repo_id = r.id
             LEFT JOIN users u ON atu.user_id = u.id
             WHERE atu.org_id = $1
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR u.email = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR atu.last_seen_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR atu.first_seen_at <= $5)
             GROUP BY atu.tool_category, atu.tool_name
             ORDER BY count DESC",
        )
        .bind(f.org_id)
        .bind(&f.repo)
        .bind(&f.author)
        .bind(f.from)
        .bind(f.to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/analytics.rs — get_ai_tools_user_detail
    // ═══════════════════════════════════════════════════════════════════

    /// AI tool usage per user (category, name, count, first_seen, last_seen, session_count).
    pub async fn get_user_ai_tools(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(String, String, i64, DateTime<Utc>, DateTime<Utc>, i64)>, AppError> {
        let rows = sqlx::query_as::<_, (String, String, i64, DateTime<Utc>, DateTime<Utc>, i64)>(
            "SELECT tool_category, tool_name,
                    SUM(usage_count) AS usage_count,
                    MIN(first_seen_at) AS first_seen,
                    MAX(last_seen_at) AS last_seen,
                    COUNT(DISTINCT session_id) AS session_count
             FROM user_ai_tool_usage
             WHERE org_id = $1 AND user_id = $2
               AND ($3::TIMESTAMPTZ IS NULL OR last_seen_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR first_seen_at <= $4)
             GROUP BY tool_category, tool_name
             ORDER BY usage_count DESC",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Recent sessions for a user that have AI tool usage.
    pub async fn get_user_ai_tool_sessions(
        pool: &PgPool,
        org_id: Uuid,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<Vec<(Uuid, String, String, Option<DateTime<Utc>>, Option<i64>)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, Option<DateTime<Utc>>, Option<i64>)>(
            "SELECT s.id, s.session_id, r.name,
                    s.started_at, s.duration_ms
             FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE s.org_id = $1 AND s.user_id = $2
               AND EXISTS (SELECT 1 FROM user_ai_tool_usage WHERE session_id = s.id)
               AND ($3::TIMESTAMPTZ IS NULL OR s.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR s.created_at <= $4)
             ORDER BY s.created_at DESC
             LIMIT 20",
        )
        .bind(org_id)
        .bind(user_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// AI tools for a batch of sessions.
    pub async fn get_session_ai_tools(
        pool: &PgPool,
        session_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT session_id, tool_name
             FROM user_ai_tool_usage
             WHERE session_id = ANY($1)
             ORDER BY session_id, usage_count DESC",
        )
        .bind(session_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/dashboard.rs
    // ═══════════════════════════════════════════════════════════════════

    /// Dashboard KPI totals for a period.
    pub async fn query_dashboard_kpi(
        pool: &PgPool,
        org_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<(f64, i64, i64, i64, i64, f64, f64, i64), AppError> {
        let row: (f64, i64, i64, i64, i64, f64, f64, i64) = sqlx::query_as(
            "SELECT
                COALESCE(SUM(s.estimated_cost_usd), 0)::float8,
                COUNT(s.id),
                COALESCE(SUM(s.total_tokens), 0)::int8,
                COUNT(DISTINCT u.email),
                COALESCE(AVG(s.duration_ms), 0)::int8,
                COALESCE(AVG(s.total_tool_calls), 0)::float8,
                0::float8,
                COALESCE(SUM(s.cache_read_tokens), 0)::int8
            FROM sessions s
            JOIN repos r ON r.id = s.repo_id
            JOIN users u ON u.id = s.user_id
            WHERE r.org_id = $1
              AND s.started_at >= $2
              AND s.started_at < $3",
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await?;
        Ok(row)
    }

    /// Dashboard sparkline data (daily cost/sessions/tokens/authors).
    pub async fn query_dashboard_sparklines(
        pool: &PgPool,
        org_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<(String, f64, i64, i64, i64)>, AppError> {
        let rows: Vec<(String, f64, i64, i64, i64)> = sqlx::query_as(
            "SELECT
                TO_CHAR(s.started_at::date, 'YYYY-MM-DD'),
                COALESCE(SUM(s.estimated_cost_usd), 0)::float8,
                COUNT(s.id),
                COALESCE(SUM(s.total_tokens), 0)::int8,
                COUNT(DISTINCT u.email)
            FROM sessions s
            JOIN repos r ON r.id = s.repo_id
            JOIN users u ON u.id = s.user_id
            WHERE r.org_id = $1
              AND s.started_at >= $2
              AND s.started_at < $3
            GROUP BY s.started_at::date
            ORDER BY s.started_at::date",
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Dashboard compliance data (sealed/unsigned counts, chain status).
    pub async fn query_dashboard_compliance(
        pool: &PgPool,
        org_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<(i64, i64, Option<bool>), AppError> {
        let (sealed, unsigned): (i64, i64) = sqlx::query_as(
            "SELECT
                COUNT(*) FILTER (WHERE ss.sealed_at IS NOT NULL),
                COUNT(*) FILTER (WHERE ss.sealed_at IS NOT NULL AND ss.signature IS NULL)
            FROM sessions s
            LEFT JOIN session_seals ss ON ss.session_id = s.id
            JOIN repos r ON r.id = s.repo_id
            WHERE r.org_id = $1
              AND s.started_at >= $2
              AND s.started_at < $3",
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool)
        .await?;

        let chain_status: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM chain_verifications
             WHERE org_id = $1
             ORDER BY created_at DESC
             LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(pool)
        .await?;

        let chain_verified = chain_status.map(|(s,)| s == "pass");

        Ok((sealed, unsigned, chain_verified))
    }

    /// Dashboard top authors for a period.
    pub async fn query_dashboard_top_authors(
        pool: &PgPool,
        org_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<Vec<(String, i64, i64, f64)>, AppError> {
        let rows: Vec<(String, i64, i64, f64)> = sqlx::query_as(
            "SELECT
                u.email,
                COUNT(s.id)::int8,
                COALESCE(SUM(s.total_tokens), 0)::int8,
                COALESCE(SUM(s.estimated_cost_usd), 0)::float8
            FROM sessions s
            JOIN repos r ON r.id = s.repo_id
            JOIN users u ON u.id = s.user_id
            WHERE r.org_id = $1
              AND s.started_at >= $2
              AND s.started_at < $3
            GROUP BY u.email
            ORDER BY SUM(s.estimated_cost_usd) DESC NULLS LAST
            LIMIT 5",
        )
        .bind(org_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════════════
    // From api/traces_ui.rs
    // ═══════════════════════════════════════════════════════════════════

    /// Active sessions count.
    pub async fn get_active_sessions_count(
        pool: &PgPool,
        org_id: Uuid,
        repo_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE r.org_id = $1
               AND s.status = 'active'
               AND s.updated_at >= now() - interval '30 minutes'
               AND ($2::UUID IS NULL OR s.repo_id = $2)",
        )
        .bind(org_id)
        .bind(repo_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Total sessions count.
    pub async fn get_total_sessions_count(
        pool: &PgPool,
        org_id: Uuid,
        repo_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE r.org_id = $1
               AND ($2::UUID IS NULL OR s.repo_id = $2)",
        )
        .bind(org_id)
        .bind(repo_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Total commits count.
    pub async fn get_total_commits_count(
        pool: &PgPool,
        org_id: Uuid,
        repo_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1
               AND ($2::UUID IS NULL OR c.repo_id = $2)",
        )
        .bind(org_id)
        .bind(repo_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Total events count.
    pub async fn get_total_events_count(
        pool: &PgPool,
        org_id: Uuid,
        repo_id: Option<Uuid>,
    ) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM events e
             JOIN sessions s ON e.session_id = s.id
             JOIN repos r ON s.repo_id = r.id
             WHERE r.org_id = $1
               AND ($2::UUID IS NULL OR s.repo_id = $2)",
        )
        .bind(org_id)
        .bind(repo_id)
        .fetch_one(pool)
        .await?;
        Ok(count)
    }

    /// Timeline query (events + commits UNION ALL).
    #[allow(clippy::type_complexity)]
    pub async fn get_timeline(
        pool: &PgPool,
        org_id: Uuid,
        repo_id: Option<Uuid>,
        tool_name: &Option<String>,
        session_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> Result<
        Vec<(
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
        )>,
        AppError,
    > {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                Option<Uuid>,
                Option<Uuid>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                DateTime<Utc>,
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
                JOIN sessions s ON e.session_id = s.id
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
                FROM commits c
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
        .bind(org_id)
        .bind(repo_id)
        .bind(tool_name)
        .bind(session_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Traces UI attribution: get commit SHA and repo_id.
    pub async fn get_commit_for_attribution(
        pool: &PgPool,
        commit_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<(String, Uuid)>, AppError> {
        let row = sqlx::query_as::<_, (String, Uuid)>(
            "SELECT c.commit_sha, c.repo_id
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE c.id = $1 AND r.org_id = $2",
        )
        .bind(commit_id)
        .bind(org_id)
        .fetch_optional(pool)
        .await?;
        Ok(row)
    }

    /// Get clone_path for a repo.
    pub async fn get_repo_clone_path(
        pool: &PgPool,
        repo_id: Uuid,
    ) -> Result<Option<String>, AppError> {
        let path =
            sqlx::query_scalar::<_, Option<String>>("SELECT clone_path FROM repos WHERE id = $1")
                .bind(repo_id)
                .fetch_one(pool)
                .await?;
        Ok(path)
    }

    /// Resolve commit SHAs to commit IDs.
    pub async fn resolve_shas_to_commit_ids(
        pool: &PgPool,
        repo_id: Uuid,
        shas: &[String],
    ) -> Result<Vec<(String, Uuid)>, AppError> {
        let rows = sqlx::query_as::<_, (String, Uuid)>(
            "SELECT commit_sha, id FROM commits WHERE repo_id = $1 AND commit_sha = ANY($2)",
        )
        .bind(repo_id)
        .bind(shas)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Get attributions for commits + file path.
    pub async fn get_file_attributions(
        pool: &PgPool,
        commit_ids: &[Uuid],
        file_path: &str,
    ) -> Result<Vec<(Uuid, Option<Uuid>, i32, i32, f32)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, Option<Uuid>, i32, i32, f32)>(
            "SELECT ca.commit_id, ca.session_id, ca.line_start, ca.line_end, ca.confidence
             FROM commit_attributions ca
             JOIN sessions s ON ca.session_id = s.id
             WHERE ca.commit_id = ANY($1) AND ca.file_path = $2",
        )
        .bind(commit_ids)
        .bind(file_path)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Get session short IDs for a batch.
    pub async fn get_session_short_ids(
        pool: &PgPool,
        session_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, String)>, AppError> {
        let rows = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, LEFT(session_id, 8) FROM sessions WHERE id = ANY($1)",
        )
        .bind(session_ids)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Branch tracking data.
    pub async fn get_branches(
        pool: &PgPool,
        org_id: Uuid,
        repo_id: Option<Uuid>,
    ) -> Result<
        Vec<(
            String,
            Option<String>,
            i64,
            i64,
            Option<f64>,
            String,
            Option<DateTime<Utc>>,
        )>,
        AppError,
    > {
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
             JOIN commits c ON bt.commit_id = c.id
             JOIN repos r ON c.repo_id = r.id
             LEFT JOIN commit_attributions ca ON ca.commit_id = c.id
             LEFT JOIN sessions s ON ca.session_id = s.id
             WHERE r.org_id = $1
               AND ($2::uuid IS NULL OR c.repo_id = $2)
             GROUP BY bt.branch
             ORDER BY last_activity DESC NULLS LAST",
        )
        .bind(org_id)
        .bind(repo_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }

    /// Check if a repo exists, belongs to org, and has clone_status = 'ready'.
    pub async fn repo_ready(pool: &PgPool, org_id: Uuid, repo_id: Uuid) -> Result<bool, AppError> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM repos WHERE id = $1 AND org_id = $2 AND clone_status = 'ready')",
        )
        .bind(repo_id)
        .bind(org_id)
        .fetch_one(pool)
        .await?;
        Ok(exists)
    }
}

/// Shared CTE used by model analytics queries.
fn model_cte() -> &'static str {
    "WITH model_data AS (
        SELECT u.email as author, s.created_at, r.name as repo_name,
               COALESCE(s.model, 'unknown') as model,
               COALESCE(s.input_tokens, 0) + COALESCE(s.output_tokens, 0) as tokens,
               s.input_tokens,
               s.output_tokens,
               s.estimated_cost_usd,
               COALESCE(s.cache_read_tokens, 0) as cache_read_tokens,
               COALESCE(s.cache_write_tokens, 0) as cache_write_tokens,
               s.duration_ms
        FROM sessions s
        JOIN repos r ON s.repo_id = r.id
        LEFT JOIN users u ON s.user_id = u.id
        WHERE r.org_id = $1
          AND ($2::TEXT IS NULL OR r.name = $2)
          AND ($3::TEXT IS NULL OR u.email = $3)
          AND ($4::TIMESTAMPTZ IS NULL OR s.created_at >= $4)
          AND ($5::TIMESTAMPTZ IS NULL OR s.created_at <= $5)
    )"
}
