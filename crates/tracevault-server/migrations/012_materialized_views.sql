CREATE MATERIALIZED VIEW IF NOT EXISTS mv_daily_session_stats AS
SELECT
  s.org_id,
  r.id AS repo_id,
  s.user_id,
  s.model,
  s.tool,
  DATE(s.created_at) AS day,
  COUNT(*) AS session_count,
  SUM(s.total_tokens) AS total_tokens,
  SUM(s.input_tokens) AS input_tokens,
  SUM(s.output_tokens) AS output_tokens,
  SUM(s.cache_read_tokens) AS cache_read_tokens,
  SUM(s.cache_write_tokens) AS cache_write_tokens,
  SUM(s.estimated_cost_usd) AS total_cost,
  SUM(s.total_tool_calls) AS total_tool_calls,
  SUM(s.user_messages) AS user_messages,
  SUM(s.assistant_messages) AS assistant_messages,
  AVG(s.duration_ms) AS avg_duration_ms,
  COUNT(DISTINCT s.user_id) AS unique_users
FROM sessions s
JOIN repos r ON s.repo_id = r.id
GROUP BY s.org_id, r.id, s.user_id, s.model, s.tool, DATE(s.created_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_daily_stats_lookup
  ON mv_daily_session_stats(org_id, day, repo_id, user_id, model, tool);
