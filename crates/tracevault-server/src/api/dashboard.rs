use std::collections::HashMap;

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::OrgAuth;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub period: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TopAuthor {
    pub author: String,
    pub sessions: i64,
    pub tokens: i64,
    pub cost: f64,
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
    pub top_authors: Vec<TopAuthor>,
}

fn period_ranges(
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

struct KpiTotals {
    total_cost: f64,
    total_sessions: i64,
    total_tokens: i64,
    active_authors: i64,
    avg_duration_ms: i64,
    avg_tool_calls: f64,
    avg_compactions: f64,
    total_cache_read_tokens: i64,
}

async fn query_kpi_totals(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<KpiTotals, AppError> {
    let row: (f64, i64, i64, i64, i64, f64, f64, i64) = sqlx::query_as(
        "SELECT
            COALESCE(SUM(s.estimated_cost_usd), 0)::float8,
            COUNT(s.id),
            COALESCE(SUM(s.total_tokens), 0)::int8,
            COUNT(DISTINCT u.email),
            COALESCE(AVG(COALESCE(NULLIF(s.duration_ms, 0), CASE WHEN s.ended_at IS NOT NULL AND s.started_at IS NOT NULL THEN EXTRACT(EPOCH FROM (s.ended_at - s.started_at))::BIGINT * 1000 ELSE NULL END)), 0)::int8,
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

struct SparklineDay {
    date: String,
    cost: f64,
    sessions: i64,
    tokens: i64,
    authors: i64,
}

async fn query_sparklines(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<Vec<SparklineDay>, AppError> {
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

    Ok(rows
        .into_iter()
        .map(|(date, cost, sessions, tokens, authors)| SparklineDay {
            date,
            cost,
            sessions,
            tokens,
            authors,
        })
        .collect())
}

struct ComplianceData {
    sealed_count: i64,
    unsigned_count: i64,
    chain_verified: Option<bool>,
}

async fn query_compliance(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<ComplianceData, AppError> {
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

    Ok(ComplianceData {
        sealed_count: sealed,
        unsigned_count: unsigned,
        chain_verified,
    })
}

async fn query_top_authors(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    from: chrono::DateTime<Utc>,
    to: chrono::DateTime<Utc>,
) -> Result<Vec<TopAuthor>, AppError> {
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

    Ok(rows
        .into_iter()
        .map(|(author, sessions, tokens, cost)| TopAuthor {
            author,
            sessions,
            tokens,
            cost,
        })
        .collect())
}

pub async fn get_dashboard(
    State(state): State<AppState>,
    auth: OrgAuth,
    Query(q): Query<DashboardQuery>,
) -> Result<Json<DashboardResponse>, AppError> {
    let period = q.period.as_deref().unwrap_or("7d");
    let (cur_start, cur_end, prev_start, prev_end) = period_ranges(period);

    let (current, previous, sparkline_raw, compliance_cur, compliance_prev, top_authors) = tokio::try_join!(
        query_kpi_totals(&state.pool, auth.org_id, cur_start, cur_end),
        query_kpi_totals(&state.pool, auth.org_id, prev_start, prev_end),
        query_sparklines(&state.pool, auth.org_id, cur_start, cur_end),
        query_compliance(&state.pool, auth.org_id, cur_start, cur_end),
        query_compliance(&state.pool, auth.org_id, prev_start, prev_end),
        query_top_authors(&state.pool, auth.org_id, cur_start, cur_end),
    )?;

    // Fill sparkline gaps: generate all dates in [cur_start, cur_end) and zero-fill missing days
    let sparkline_map: HashMap<String, &SparklineDay> =
        sparkline_raw.iter().map(|d| (d.date.clone(), d)).collect();

    let mut sparkline_data = Vec::new();
    let mut date = cur_start.date_naive();
    let end_date = cur_end.date_naive();
    while date < end_date {
        let key = date.format("%Y-%m-%d").to_string();
        if let Some(day) = sparkline_map.get(&key) {
            sparkline_data.push(SparklineDay {
                date: key,
                cost: day.cost,
                sessions: day.sessions,
                tokens: day.tokens,
                authors: day.authors,
            });
        } else {
            sparkline_data.push(SparklineDay {
                date: key,
                cost: 0.0,
                sessions: 0,
                tokens: 0,
                authors: 0,
            });
        }
        date += chrono::Duration::days(1);
    }

    let cost_trend = trend_pct(current.total_cost, previous.total_cost);
    let sessions_trend = trend_pct(
        current.total_sessions as f64,
        previous.total_sessions as f64,
    );
    let tokens_trend = trend_pct(current.total_tokens as f64, previous.total_tokens as f64);
    let authors_change = current.active_authors - previous.active_authors;

    let compliance_score = if compliance_cur.sealed_count > 0 {
        ((compliance_cur.sealed_count - compliance_cur.unsigned_count) as f64
            / compliance_cur.sealed_count as f64)
            * 100.0
    } else {
        100.0
    };
    let prev_compliance_score = if compliance_prev.sealed_count > 0 {
        ((compliance_prev.sealed_count - compliance_prev.unsigned_count) as f64
            / compliance_prev.sealed_count as f64)
            * 100.0
    } else {
        100.0
    };
    let compliance_trend = compliance_score - prev_compliance_score;

    let cache_savings = state
        .extensions
        .pricing
        .estimate_cache_savings("sonnet", current.total_cache_read_tokens);
    let cache_savings_pct = if current.total_cost > 0.0 {
        (cache_savings / current.total_cost) * 100.0
    } else {
        0.0
    };

    Ok(Json(DashboardResponse {
        total_cost_usd: current.total_cost,
        cost_trend_pct: cost_trend,
        cost_sparkline: sparkline_data.iter().map(|d| d.cost).collect(),
        active_authors: current.active_authors,
        authors_change,
        authors_sparkline: sparkline_data.iter().map(|d| d.authors as f64).collect(),
        total_sessions: current.total_sessions,
        sessions_trend_pct: sessions_trend,
        sessions_sparkline: sparkline_data.iter().map(|d| d.sessions as f64).collect(),
        total_tokens: current.total_tokens,
        tokens_trend_pct: tokens_trend,
        tokens_sparkline: sparkline_data.iter().map(|d| d.tokens as f64).collect(),
        avg_session_duration_ms: current.avg_duration_ms,
        avg_tool_calls_per_session: current.avg_tool_calls,
        avg_compactions_per_session: current.avg_compactions,
        compliance_score_pct: compliance_score,
        compliance_trend_pct: compliance_trend,
        unsigned_sessions: compliance_cur.unsigned_count,
        chain_verified: compliance_cur.chain_verified,
        cache_savings_usd: cache_savings,
        cache_savings_pct,
        top_authors,
    }))
}

fn trend_pct(current: f64, previous: f64) -> f64 {
    if previous == 0.0 {
        if current > 0.0 {
            100.0
        } else {
            0.0
        }
    } else {
        ((current - previous) / previous) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_pct_zero_previous_current_positive() {
        // previous=0, current=100 → 100% (special case)
        assert_eq!(trend_pct(100.0, 0.0), 100.0);
    }

    #[test]
    fn trend_pct_both_zero() {
        assert_eq!(trend_pct(0.0, 0.0), 0.0);
    }

    #[test]
    fn trend_pct_no_change() {
        assert!((trend_pct(100.0, 100.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn trend_pct_decrease() {
        // current=50, previous=100 → -50%
        assert!((trend_pct(50.0, 100.0) - (-50.0)).abs() < 0.01);
    }

    #[test]
    fn trend_pct_increase() {
        // current=200, previous=100 → +100%
        assert!((trend_pct(200.0, 100.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn period_ranges_7d() {
        let (from, to, prev_from, _prev_to) = period_ranges("7d");
        let diff = to.signed_duration_since(from);
        assert!(diff.num_days() >= 6 && diff.num_days() <= 7);
        assert!(prev_from < from);
    }

    #[test]
    fn period_ranges_30d() {
        let (from, to, _, _) = period_ranges("30d");
        let diff = to.signed_duration_since(from);
        assert!(diff.num_days() >= 29 && diff.num_days() <= 30);
    }
}
