struct ModelPricing {
    input_per_m: f64,
    output_per_m: f64,
    cache_write_per_m: f64,
    cache_read_per_m: f64,
}

const OPUS: ModelPricing = ModelPricing {
    input_per_m: 15.0,
    output_per_m: 75.0,
    cache_write_per_m: 18.75,
    cache_read_per_m: 1.50,
};

const SONNET: ModelPricing = ModelPricing {
    input_per_m: 3.0,
    output_per_m: 15.0,
    cache_write_per_m: 3.75,
    cache_read_per_m: 0.30,
};

const HAIKU: ModelPricing = ModelPricing {
    input_per_m: 0.80,
    output_per_m: 4.0,
    cache_write_per_m: 1.00,
    cache_read_per_m: 0.08,
};

fn pricing_for_model(model: &str) -> &'static ModelPricing {
    let lower = model.to_lowercase();
    if lower.contains("opus") {
        &OPUS
    } else if lower.contains("haiku") {
        &HAIKU
    } else {
        &SONNET // default
    }
}

/// Estimate cost in USD from token counts for a single model.
pub fn estimate_cost(
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
) -> f64 {
    let p = pricing_for_model(model);
    (input_tokens as f64 / 1_000_000.0) * p.input_per_m
        + (output_tokens as f64 / 1_000_000.0) * p.output_per_m
        + (cache_write_tokens as f64 / 1_000_000.0) * p.cache_write_per_m
        + (cache_read_tokens as f64 / 1_000_000.0) * p.cache_read_per_m
}

/// Estimate how much was saved by cache reads vs full input price.
pub fn estimate_cache_savings(model: &str, cache_read_tokens: i64) -> f64 {
    let p = pricing_for_model(model);
    (cache_read_tokens as f64 / 1_000_000.0) * (p.input_per_m - p.cache_read_per_m)
}

/// Compute total cost from model_usage JSONB array.
/// Each element: {"model": "...", "input_tokens": N, "output_tokens": N, "cache_read_tokens": N, "cache_creation_tokens": N}
/// Falls back to session-level totals if model_usage is absent.
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
