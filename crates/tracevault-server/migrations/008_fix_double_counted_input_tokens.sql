-- Fix input_tokens that were double-counted (included cache_read + cache_write tokens).
-- Subtract cache tokens to get fresh (non-cached) input only.
UPDATE sessions_v2
SET input_tokens = GREATEST(input_tokens - cache_read_tokens - cache_write_tokens, 0),
    total_tokens = GREATEST(input_tokens - cache_read_tokens - cache_write_tokens, 0)
                   + output_tokens + cache_read_tokens + cache_write_tokens
WHERE input_tokens > 0
  AND (cache_read_tokens > 0 OR cache_write_tokens > 0);

-- Reset estimated_cost_usd to 0 so it gets recalculated by the pricing sync.
-- The pricing sync runs on startup and will recalculate all affected sessions.
UPDATE sessions_v2
SET estimated_cost_usd = 0
WHERE estimated_cost_usd > 0;
