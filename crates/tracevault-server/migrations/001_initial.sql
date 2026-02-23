CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE orgs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    plan TEXT NOT NULL DEFAULT 'free',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    key_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE repos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    name TEXT NOT NULL,
    github_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(org_id, name)
);

CREATE TABLE traces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id),
    commit_sha TEXT NOT NULL,
    branch TEXT,
    author TEXT NOT NULL,
    model TEXT,
    tool TEXT,
    tool_version TEXT,
    ai_percentage REAL,
    total_tokens BIGINT,
    input_tokens BIGINT,
    output_tokens BIGINT,
    estimated_cost_usd DOUBLE PRECISION,
    api_calls INTEGER,
    session_data JSONB,
    attribution JSONB,
    agent_trace JSONB,
    signature TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_traces_repo_id ON traces(repo_id);
CREATE INDEX idx_traces_commit_sha ON traces(commit_sha);
CREATE INDEX idx_traces_author ON traces(author);
CREATE INDEX idx_traces_created_at ON traces(created_at);

CREATE TABLE policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    name TEXT NOT NULL,
    description TEXT,
    condition JSONB NOT NULL,
    action TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'medium',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    trace_id UUID NOT NULL REFERENCES traces(id),
    policy_id UUID NOT NULL REFERENCES policies(id),
    result TEXT NOT NULL,
    details JSONB,
    evaluated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
