-- Enable trigram extension for LIKE optimization
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Attribution: fix full table scan on trailing wildcard LIKE
CREATE INDEX IF NOT EXISTS idx_file_changes_path_trgm
  ON file_changes USING gin(file_path gin_trgm_ops);

-- Background repo sync job
CREATE INDEX IF NOT EXISTS idx_repos_clone_status
  ON repos(clone_status) WHERE clone_status = 'ready';

-- Pricing sync DISTINCT model lookup
CREATE INDEX IF NOT EXISTS idx_sessions_model
  ON sessions(model) WHERE model IS NOT NULL;

-- Analytics: common filter patterns
CREATE INDEX IF NOT EXISTS idx_sessions_org_created
  ON sessions(org_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_sessions_org_repo_created
  ON sessions(org_id, repo_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_commits_repo_committed
  ON commits(repo_id, committed_at DESC);

-- Tool field: backfill and enforce NOT NULL
UPDATE sessions SET tool = 'claude-code' WHERE tool IS NULL;
ALTER TABLE sessions ALTER COLUMN tool SET NOT NULL;
ALTER TABLE sessions ALTER COLUMN tool DROP DEFAULT;
