use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize)]
pub struct ModelPricing {
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub cache_write_per_m: f64,
    pub cache_read_per_m: f64,
}

// Hardcoded fallbacks (Sonnet rates) — used when DB lookup fails
const FALLBACK_PRICING: ModelPricing = ModelPricing {
    input_per_m: 3.0,
    output_per_m: 15.0,
    cache_write_per_m: 3.75,
    cache_read_per_m: 0.30,
};

/// Map a model string to its canonical name for DB lookup.
pub fn canonical_model_name(model: &str) -> &'static str {
    let lower = model.to_lowercase();
    if lower.contains("opus") {
        "opus"
    } else if lower.contains("haiku") {
        "haiku"
    } else {
        "sonnet"
    }
}

/// Look up pricing from model_pricing table using exact model name match.
/// Falls back to hardcoded rates if no match found.
pub async fn fetch_pricing_for_model(
    pool: &PgPool,
    model: &str,
    at: Option<DateTime<Utc>>,
) -> ModelPricing {
    let at = at.unwrap_or_else(Utc::now);

    let row = sqlx::query_as::<_, (f64, f64, f64, f64)>(
        "SELECT input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok
         FROM model_pricing
         WHERE model = $1
           AND effective_from <= $2
           AND (effective_until IS NULL OR effective_until > $2)
         ORDER BY effective_from DESC
         LIMIT 1",
    )
    .bind(model)
    .bind(at)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some((input, output, cache_read, cache_write))) => ModelPricing {
            input_per_m: input,
            output_per_m: output,
            cache_read_per_m: cache_read,
            cache_write_per_m: cache_write,
        },
        _ => fallback_pricing_for_model(model),
    }
}

/// Synchronous fallback using hardcoded rates. Used by existing code paths
/// that don't have async context (e.g., cost_from_model_usage during ingest).
pub fn fallback_pricing_for_model(model: &str) -> ModelPricing {
    match canonical_model_name(model) {
        "opus" => ModelPricing {
            input_per_m: 15.0,
            output_per_m: 75.0,
            cache_write_per_m: 18.75,
            cache_read_per_m: 1.50,
        },
        "haiku" => ModelPricing {
            input_per_m: 0.80,
            output_per_m: 4.0,
            cache_write_per_m: 1.00,
            cache_read_per_m: 0.08,
        },
        _ => FALLBACK_PRICING,
    }
}

/// Estimate cost in USD from token counts using given pricing.
pub fn estimate_cost_with_pricing(
    pricing: &ModelPricing,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
) -> f64 {
    (input_tokens as f64 / 1_000_000.0) * pricing.input_per_m
        + (output_tokens as f64 / 1_000_000.0) * pricing.output_per_m
        + (cache_write_tokens as f64 / 1_000_000.0) * pricing.cache_write_per_m
        + (cache_read_tokens as f64 / 1_000_000.0) * pricing.cache_read_per_m
}

/// Backward-compatible wrapper: estimate cost by model name (uses hardcoded fallback).
pub fn estimate_cost(
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
) -> f64 {
    let p = fallback_pricing_for_model(model);
    estimate_cost_with_pricing(
        &p,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
    )
}

/// Estimate gross savings from cache reads vs full input price.
pub fn estimate_cache_savings(model: &str, cache_read_tokens: i64) -> f64 {
    let p = fallback_pricing_for_model(model);
    (cache_read_tokens as f64 / 1_000_000.0) * (p.input_per_m - p.cache_read_per_m)
}

/// Estimate overhead from cache writes vs base input price.
pub fn estimate_cache_write_overhead(pricing: &ModelPricing, cache_write_tokens: i64) -> f64 {
    (cache_write_tokens as f64 / 1_000_000.0) * (pricing.cache_write_per_m - pricing.input_per_m)
}

/// Compute total cost from model_usage JSONB array (backward compatible).
pub fn cost_from_model_usage(
    model_usage: Option<&serde_json::Value>,
    fallback_model: Option<&str>,
    fallback_input: i64,
    fallback_output: i64,
    fallback_cache_read: i64,
    fallback_cache_write: i64,
) -> f64 {
    if let Some(arr) = model_usage.and_then(|v| v.as_array()) {
        arr.iter()
            .map(|entry| {
                let model = entry
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let input = entry
                    .get("input_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let output = entry
                    .get("output_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let cache_read = entry
                    .get("cache_read_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let cache_write = entry
                    .get("cache_creation_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                estimate_cost(model, input, output, cache_read, cache_write)
            })
            .sum()
    } else {
        estimate_cost(
            fallback_model.unwrap_or("unknown"),
            fallback_input,
            fallback_output,
            fallback_cache_read,
            fallback_cache_write,
        )
    }
}

#[derive(Debug)]
pub struct RecalculateResult {
    pub affected_sessions: i64,
    pub total_old_cost: f64,
    pub total_new_cost: f64,
}

/// Recalculate session costs for a given model and pricing in a date range.
/// Used by both the manual recalculate endpoint and the automatic sync.
/// When `actor_id` is None, audit entries are logged as system-initiated.
pub async fn recalculate_sessions_for_pricing(
    pool: &PgPool,
    canonical_model: &str,
    pricing: &ModelPricing,
    effective_from: DateTime<Utc>,
    effective_until: Option<DateTime<Utc>>,
    actor_id: Option<uuid::Uuid>,
    org_id: Option<uuid::Uuid>,
) -> Result<RecalculateResult, sqlx::Error> {
    let sessions = sqlx::query_as::<_, (uuid::Uuid, uuid::Uuid, Option<f64>, i64, i64, i64, i64)>(
        "SELECT s.id, s.org_id, s.estimated_cost_usd,
                COALESCE(s.input_tokens, 0), COALESCE(s.output_tokens, 0),
                COALESCE(s.cache_read_tokens, 0), COALESCE(s.cache_write_tokens, 0)
         FROM sessions s
         WHERE s.created_at >= $1
           AND ($2::timestamptz IS NULL OR s.created_at < $2)
           AND s.model = $3
           AND ($4::uuid IS NULL OR s.org_id = $4)",
    )
    .bind(effective_from)
    .bind(effective_until)
    .bind(canonical_model)
    .bind(org_id)
    .fetch_all(pool)
    .await?;

    let mut total_old: f64 = 0.0;
    let mut total_new: f64 = 0.0;
    let mut affected: i64 = 0;
    let source_label = if actor_id.is_some() {
        "manual"
    } else {
        "pricing_sync"
    };

    let mut tx = pool.begin().await?;

    for (
        session_id,
        session_org_id,
        old_cost,
        input_tokens,
        output_tokens,
        cache_read,
        cache_write,
    ) in &sessions
    {
        let old = old_cost.unwrap_or(0.0);
        let new_cost = estimate_cost_with_pricing(
            pricing,
            *input_tokens,
            *output_tokens,
            *cache_read,
            *cache_write,
        );

        sqlx::query("UPDATE sessions SET estimated_cost_usd = $1 WHERE id = $2")
            .bind(new_cost)
            .bind(session_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO audit_log (org_id, actor_id, action, resource_type, resource_id, details)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session_org_id)
        .bind(actor_id)
        .bind("pricing_recalculate")
        .bind("session")
        .bind(*session_id)
        .bind(serde_json::json!({
            "old_cost": old,
            "new_cost": new_cost,
            "source": source_label,
        }))
        .execute(&mut *tx)
        .await?;

        total_old += old;
        total_new += new_cost;
        affected += 1;
    }

    tx.commit().await?;

    Ok(RecalculateResult {
        affected_sessions: affected,
        total_old_cost: total_old,
        total_new_cost: total_new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_cost_sonnet() {
        let cost = estimate_cost("claude-sonnet-4-6", 1_000_000, 1_000_000, 0, 0);
        assert!((cost - 18.0).abs() < 0.001); // 3 + 15
    }

    #[test]
    fn test_estimate_cost_opus_with_cache() {
        let cost = estimate_cost("claude-opus-4-6", 0, 0, 1_000_000, 1_000_000);
        // cache_read: 1.50 + cache_write: 18.75 = 20.25
        assert!((cost - 20.25).abs() < 0.001);
    }

    #[test]
    fn test_cache_write_overhead() {
        let p = fallback_pricing_for_model("opus");
        let overhead = estimate_cache_write_overhead(&p, 1_000_000);
        // 18.75 - 15.0 = 3.75
        assert!((overhead - 3.75).abs() < 0.001);
    }

    #[test]
    fn test_fallback_defaults_to_sonnet() {
        let p = fallback_pricing_for_model("unknown-model-xyz");
        assert!((p.input_per_m - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_canonical_model_name() {
        assert_eq!(canonical_model_name("claude-opus-4-6"), "opus");
        assert_eq!(canonical_model_name("claude-sonnet-4-6"), "sonnet");
        assert_eq!(canonical_model_name("claude-haiku-4-5"), "haiku");
        assert_eq!(canonical_model_name("unknown-model"), "sonnet");
    }

    #[test]
    fn canonical_model_name_case_insensitive() {
        assert_eq!(canonical_model_name("Claude-3.5-Sonnet"), "sonnet");
    }

    #[test]
    fn estimate_cost_with_pricing_all_types() {
        let pricing = ModelPricing {
            input_per_m: 3.0,
            output_per_m: 15.0,
            cache_write_per_m: 3.75,
            cache_read_per_m: 0.30,
        };
        let cost = estimate_cost_with_pricing(&pricing, 1_000_000, 1_000_000, 1_000_000, 1_000_000);
        let expected = 3.0 + 15.0 + 0.30 + 3.75;
        assert!((cost - expected).abs() < 0.01);
    }

    #[test]
    fn estimate_cache_savings_calculation() {
        let savings = estimate_cache_savings("sonnet", 1_000_000);
        assert!((savings - 2.70).abs() < 0.01);
    }

    #[test]
    fn cost_from_model_usage_none_uses_fallback() {
        let cost = cost_from_model_usage(None, Some("sonnet"), 1_000_000, 0, 0, 0);
        assert!((cost - 3.0).abs() < 0.01);
    }

    #[test]
    fn cost_from_model_usage_valid_json() {
        let usage = serde_json::json!([{
            "model": "sonnet",
            "input_tokens": 1000000,
            "output_tokens": 0,
            "cache_read_tokens": 0,
            "cache_creation_tokens": 0
        }]);
        let cost = cost_from_model_usage(Some(&usage), None, 0, 0, 0, 0);
        assert!((cost - 3.0).abs() < 0.01);
    }

    #[test]
    fn cost_from_model_usage_missing_fields_uses_zero() {
        let usage = serde_json::json!([{"model": "sonnet"}]);
        let cost = cost_from_model_usage(Some(&usage), None, 0, 0, 0, 0);
        assert!(cost.abs() < 0.001);
    }
}
