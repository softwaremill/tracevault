use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

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
