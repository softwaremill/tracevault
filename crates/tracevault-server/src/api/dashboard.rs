use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub period: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DashboardResponse {
    pub total_cost_usd: f64,
    pub cost_trend_pct: f64,
    pub cost_sparkline: Vec<f64>,
    pub active_authors: i64,
    pub authors_change: i64,
    pub authors_sparkline: Vec<f64>,
    pub total_sessions: i64,
    pub sessions_trend_pct: f64,
    pub sessions_sparkline: Vec<f64>,
    pub total_tokens: i64,
    pub tokens_trend_pct: f64,
    pub tokens_sparkline: Vec<f64>,
    pub avg_session_duration_ms: i64,
    pub avg_tool_calls_per_session: f64,
    pub avg_compactions_per_session: f64,
    pub compliance_score_pct: f64,
    pub compliance_trend_pct: f64,
    pub unsigned_sessions: i64,
    pub chain_verified: Option<bool>,
    pub cache_savings_usd: f64,
    pub cache_savings_pct: f64,
}

pub(crate) fn period_ranges(
    period: &str,
) -> (
    chrono::DateTime<Utc>,
    chrono::DateTime<Utc>,
    chrono::DateTime<Utc>,
    chrono::DateTime<Utc>,
) {
    let now = Utc::now();
    let today = now.date_naive();

    match period {
        "30d" => {
            let current_start = today - chrono::Duration::days(30);
            let prev_start = current_start - chrono::Duration::days(30);
            (
                current_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                now,
                prev_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                current_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            )
        }
        "month" => {
            let first_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
            let prev_month = if today.month() == 1 {
                NaiveDate::from_ymd_opt(today.year() - 1, 12, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1).unwrap()
            };
            (
                first_of_month.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                now,
                prev_month.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                first_of_month.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            )
        }
        _ => {
            let current_start = today - chrono::Duration::days(7);
            let prev_start = current_start - chrono::Duration::days(7);
            (
                current_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                now,
                prev_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
                current_start.and_hms_opt(0, 0, 0).unwrap().and_utc(),
            )
        }
    }
}

pub(crate) struct KpiTotals {
    pub total_cost: f64,
    pub total_sessions: i64,
    pub total_tokens: i64,
    pub active_authors: i64,
    pub avg_duration_ms: i64,
    pub avg_tool_calls: f64,
    pub avg_compactions: f64,
    pub total_cache_read_tokens: i64,
}

pub(crate) async fn query_kpi_totals(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<KpiTotals, (StatusCode, String)> {
    let row: (f64, i64, i64, i64, i64, f64, f64, i64) = sqlx::query_as(
        "SELECT
            COALESCE(SUM(s.estimated_cost_usd), 0)::float8,
            COUNT(s.id),
            COALESCE(SUM(s.total_tokens), 0),
            COUNT(DISTINCT c.author),
            COALESCE(AVG(s.duration_ms), 0)::int8,
            COALESCE(AVG(s.total_tool_calls), 0)::float8,
            COALESCE(AVG(s.compactions), 0)::float8,
            COALESCE(SUM(s.cache_read_tokens), 0)
        FROM sessions s
        JOIN commits c ON c.id = s.commit_id
        JOIN repos r ON r.id = c.repo_id
        WHERE r.org_id = $1
          AND s.started_at >= $2
          AND s.started_at < $3",
    )
    .bind(org_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(KpiTotals {
        total_cost: row.0,
        total_sessions: row.1,
        total_tokens: row.2,
        active_authors: row.3,
        avg_duration_ms: row.4,
        avg_tool_calls: row.5,
        avg_compactions: row.6,
        total_cache_read_tokens: row.7,
    })
}

pub(crate) struct SparklineDay {
    pub cost: f64,
    pub sessions: i64,
    pub tokens: i64,
    pub authors: i64,
}

pub(crate) async fn query_sparklines(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<Vec<SparklineDay>, (StatusCode, String)> {
    let rows: Vec<(String, f64, i64, i64, i64)> = sqlx::query_as(
        "SELECT
            TO_CHAR(s.started_at::date, 'YYYY-MM-DD'),
            COALESCE(SUM(s.estimated_cost_usd), 0)::float8,
            COUNT(s.id),
            COALESCE(SUM(s.total_tokens), 0),
            COUNT(DISTINCT c.author)
        FROM sessions s
        JOIN commits c ON c.id = s.commit_id
        JOIN repos r ON r.id = c.repo_id
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
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|(_d, cost, sessions, tokens, authors)| SparklineDay {
            cost,
            sessions,
            tokens,
            authors,
        })
        .collect())
}

pub(crate) struct ComplianceData {
    pub sealed_count: i64,
    pub unsigned_count: i64,
    pub chain_verified: Option<bool>,
}

pub(crate) async fn query_compliance(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<ComplianceData, (StatusCode, String)> {
    let (sealed, unsigned): (i64, i64) = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (WHERE s.sealed_at IS NOT NULL),
            COUNT(*) FILTER (WHERE s.sealed_at IS NOT NULL AND s.signature IS NULL)
        FROM sessions s
        JOIN commits c ON c.id = s.commit_id
        JOIN repos r ON r.id = c.repo_id
        WHERE r.org_id = $1
          AND s.started_at >= $2
          AND s.started_at < $3",
    )
    .bind(org_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let chain_status: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM chain_verifications
         WHERE org_id = $1
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let chain_verified = chain_status.map(|(s,)| s == "pass");

    Ok(ComplianceData {
        sealed_count: sealed,
        unsigned_count: unsigned,
        chain_verified,
    })
}
