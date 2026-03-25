-- Streaming architecture: new tables for real-time event capture
-- These tables are independent of the existing sessions/commits tables (clean break)

-- Sessions v2: decoupled from commits, org+repo scoped
CREATE TABLE sessions_v2 (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    repo_id UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    session_id TEXT NOT NULL,
    model TEXT,
    tool TEXT DEFAULT 'claude-code',
    status TEXT NOT NULL DEFAULT 'active',
    total_tokens BIGINT DEFAULT 0,
    input_tokens BIGINT DEFAULT 0,
    output_tokens BIGINT DEFAULT 0,
    cache_read_tokens BIGINT DEFAULT 0,
    cache_write_tokens BIGINT DEFAULT 0,
    estimated_cost_usd DOUBLE PRECISION DEFAULT 0.0,
    duration_ms BIGINT,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    user_messages INTEGER DEFAULT 0,
    assistant_messages INTEGER DEFAULT 0,
    total_tool_calls INTEGER DEFAULT 0,
    cwd TEXT,
    record_hash TEXT,
    signature TEXT,
    sealed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repo_id, session_id)
);

CREATE INDEX idx_sessions_v2_org ON sessions_v2(org_id);
CREATE INDEX idx_sessions_v2_repo ON sessions_v2(repo_id);
CREATE INDEX idx_sessions_v2_user ON sessions_v2(user_id);
CREATE INDEX idx_sessions_v2_status ON sessions_v2(status);
CREATE INDEX idx_sessions_v2_created ON sessions_v2(created_at);

-- Events: one row per hook call
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions_v2(id) ON DELETE CASCADE,
    event_index INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    tool_name TEXT,
    tool_input JSONB,
    tool_response JSONB,
    tool_use_id TEXT,
    timestamp TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(session_id, event_index)
);

CREATE INDEX idx_events_session ON events(session_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_tool ON events(tool_name);

-- File changes: extracted from Write/Edit/Bash events
CREATE TABLE file_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions_v2(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    change_type TEXT NOT NULL,
    line_start INTEGER,
    line_end INTEGER,
    diff_text TEXT,
    content_hash TEXT,
    timestamp TIMESTAMPTZ NOT NULL,
    UNIQUE(event_id, file_path)
);

CREATE INDEX idx_file_changes_session ON file_changes(session_id);
CREATE INDEX idx_file_changes_file ON file_changes(file_path);
CREATE INDEX idx_file_changes_timestamp ON file_changes(timestamp);

-- Transcript chunks: conversation turns streamed incrementally
CREATE TABLE transcript_chunks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions_v2(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    data JSONB NOT NULL,
    token_usage JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(session_id, chunk_index)
);

CREATE INDEX idx_transcript_chunks_session ON transcript_chunks(session_id);

-- Commits v2: independent of sessions
CREATE TABLE commits_v2 (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    commit_sha TEXT NOT NULL,
    branch TEXT,
    author TEXT NOT NULL,
    diff_data JSONB,
    committed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repo_id, commit_sha)
);

CREATE INDEX idx_commits_v2_repo ON commits_v2(repo_id);
CREATE INDEX idx_commits_v2_sha ON commits_v2(commit_sha);
CREATE INDEX idx_commits_v2_branch ON commits_v2(branch);

-- Commit attributions: many-to-many link between commits and session events
CREATE TABLE commit_attributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits_v2(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES sessions_v2(id) ON DELETE CASCADE,
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    file_path TEXT NOT NULL,
    line_start INTEGER,
    line_end INTEGER,
    confidence REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_commit_attr_commit ON commit_attributions(commit_id);
CREATE INDEX idx_commit_attr_session ON commit_attributions(session_id);
CREATE INDEX idx_commit_attr_file ON commit_attributions(file_path);

-- Branch tracking: when commits reach target branches/tags
CREATE TABLE branch_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits_v2(id) ON DELETE CASCADE,
    branch TEXT NOT NULL,
    tag TEXT,
    tracked_at TIMESTAMPTZ NOT NULL,
    tracking_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(commit_id, branch)
);

CREATE INDEX idx_branch_tracking_commit ON branch_tracking(commit_id);
CREATE INDEX idx_branch_tracking_branch ON branch_tracking(branch);
