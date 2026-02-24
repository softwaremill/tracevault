-- commits: one per (repo_id, commit_sha)
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    commit_sha TEXT NOT NULL,
    branch TEXT,
    author TEXT NOT NULL,
    diff_data JSONB,
    attribution JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(repo_id, commit_sha)
);
CREATE INDEX idx_commits_repo_id ON commits(repo_id);
CREATE INDEX idx_commits_commit_sha ON commits(commit_sha);
CREATE INDEX idx_commits_created_at ON commits(created_at);

-- sessions: one per (commit_id, session_id)
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    model TEXT,
    tool TEXT,
    total_tokens BIGINT,
    input_tokens BIGINT,
    output_tokens BIGINT,
    estimated_cost_usd DOUBLE PRECISION,
    api_calls INTEGER,
    session_data JSONB,
    transcript JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(commit_id, session_id)
);
CREATE INDEX idx_sessions_commit_id ON sessions(commit_id);

-- Migrate: one commit per distinct (repo_id, commit_sha), using latest trace
INSERT INTO commits (repo_id, commit_sha, branch, author, diff_data, attribution, created_at)
SELECT DISTINCT ON (t.repo_id, t.commit_sha)
    t.repo_id, t.commit_sha, t.branch, t.author, t.diff_data, t.attribution, t.created_at
FROM traces t
ORDER BY t.repo_id, t.commit_sha, t.created_at DESC;

-- Migrate sessions from traces
INSERT INTO sessions (commit_id, session_id, model, tool, total_tokens, input_tokens, output_tokens,
    estimated_cost_usd, api_calls, session_data, transcript, created_at)
SELECT c.id, COALESCE(t.session_data->>'session_id', t.id::TEXT),
    t.model, t.tool, t.total_tokens, t.input_tokens, t.output_tokens,
    t.estimated_cost_usd, t.api_calls, t.session_data, t.transcript, t.created_at
FROM traces t
JOIN commits c ON c.repo_id = t.repo_id AND c.commit_sha = t.commit_sha
ON CONFLICT (commit_id, session_id) DO NOTHING;

-- Drop old tables (evaluations has FK to traces)
DROP TABLE IF EXISTS evaluations;
DROP TABLE traces;
