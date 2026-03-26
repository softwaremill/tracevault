CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Organizations
CREATE TABLE orgs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    display_name TEXT,
    signing_key_encrypted TEXT,
    signing_key_nonce TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Users (no org_id or role — those live in user_org_memberships)
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Multi-org membership junction table
CREATE TABLE user_org_memberships (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    org_id UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'developer',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, org_id)
);

-- Auth sessions (user-scoped, not org-scoped)
CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_auth_sessions_token ON auth_sessions(token_hash);

-- Device auth flow
CREATE TABLE device_auth_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token TEXT NOT NULL UNIQUE,
    user_id UUID REFERENCES users(id),
    status TEXT NOT NULL DEFAULT 'pending',
    session_id UUID REFERENCES auth_sessions(id),
    session_token TEXT,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_device_auth_token ON device_auth_requests(token);

-- Org compliance settings (org_id is PK)
CREATE TABLE org_compliance_settings (
    org_id UUID PRIMARY KEY REFERENCES orgs(id),
    compliance_mode TEXT,
    retention_days INTEGER DEFAULT 365,
    signing_enabled BOOLEAN DEFAULT false,
    chain_verification_interval_hours INTEGER DEFAULT 24,
    llm_provider TEXT,
    llm_api_key_encrypted TEXT,
    llm_api_key_nonce TEXT,
    llm_model TEXT,
    llm_base_url TEXT,
    created_at TIMESTAMPTZ DEFAULT now(),
    updated_at TIMESTAMPTZ DEFAULT now()
);

-- Signing key rotation history
CREATE TABLE org_signing_key_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    signing_key_encrypted TEXT NOT NULL,
    signing_key_nonce TEXT NOT NULL,
    active_from TIMESTAMPTZ NOT NULL,
    active_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Repositories
CREATE TABLE repos (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    name TEXT NOT NULL,
    github_url TEXT,
    clone_status TEXT DEFAULT 'pending',
    clone_path TEXT,
    last_fetched_at TIMESTAMPTZ,
    deploy_key_encrypted TEXT,
    deploy_key_nonce TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(org_id, name)
);

-- API keys
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    key_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Sessions: org+repo scoped, streaming architecture
CREATE TABLE sessions (
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repo_id, session_id)
);

CREATE INDEX idx_sessions_org ON sessions(org_id);
CREATE INDEX idx_sessions_repo ON sessions(repo_id);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_status ON sessions(status);
CREATE INDEX idx_sessions_created ON sessions(created_at);

-- Session seals: immutability proof for sessions
CREATE TABLE session_seals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    record_hash TEXT NOT NULL,
    signature TEXT,
    sealed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(session_id)
);

-- Events: one row per hook call
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
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
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
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
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL,
    data JSONB NOT NULL,
    token_usage JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(session_id, chunk_index)
);

CREATE INDEX idx_transcript_chunks_session ON transcript_chunks(session_id);

-- Commits: repo-scoped, independent of sessions
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    commit_sha TEXT NOT NULL,
    branch TEXT,
    author TEXT NOT NULL,
    message TEXT,
    diff_data JSONB,
    attribution JSONB,
    committed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repo_id, commit_sha)
);

CREATE INDEX idx_commits_repo ON commits(repo_id);
CREATE INDEX idx_commits_sha ON commits(commit_sha);
CREATE INDEX idx_commits_branch ON commits(branch);

-- Commit seals: immutability proof for commits
CREATE TABLE commit_seals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    record_hash TEXT NOT NULL,
    chain_hash TEXT,
    prev_chain_hash TEXT,
    signature TEXT,
    sealed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(commit_id)
);

-- Commit attributions: many-to-many link between commits and session events
CREATE TABLE commit_attributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
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
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    branch TEXT NOT NULL,
    tag TEXT,
    tracked_at TIMESTAMPTZ NOT NULL,
    tracking_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(commit_id, branch)
);

CREATE INDEX idx_branch_tracking_commit ON branch_tracking(commit_id);
CREATE INDEX idx_branch_tracking_branch ON branch_tracking(branch);

-- Policies
CREATE TABLE policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    repo_id UUID REFERENCES repos(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    condition JSONB NOT NULL,
    action TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'medium',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_policies_org_id ON policies(org_id);
CREATE INDEX idx_policies_repo_id ON policies(repo_id);

-- Amendments
CREATE TABLE amendments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_commit_id UUID NOT NULL REFERENCES commits(id),
    reason TEXT NOT NULL,
    amended_by UUID NOT NULL REFERENCES users(id),
    amendment_data JSONB NOT NULL,
    signature TEXT,
    record_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_amendments_commit ON amendments(original_commit_id);

-- Audit log
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    actor_id UUID REFERENCES users(id),
    api_key_id UUID REFERENCES api_keys(id),
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audit_log_org ON audit_log(org_id);
CREATE INDEX idx_audit_log_actor ON audit_log(actor_id);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX idx_audit_log_created ON audit_log(created_at);

-- Chain verifications
CREATE TABLE chain_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    status TEXT NOT NULL,
    total_commits INTEGER NOT NULL DEFAULT 0,
    verified_commits INTEGER NOT NULL DEFAULT 0,
    errors JSONB,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_chain_verifications_org ON chain_verifications(org_id);

-- Code stories
CREATE TABLE code_stories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    function_name TEXT NOT NULL,
    line_range_start INTEGER NOT NULL,
    line_range_end INTEGER NOT NULL,
    ref_name TEXT NOT NULL,
    head_commit_sha TEXT NOT NULL,
    story_markdown TEXT NOT NULL,
    commits_analyzed JSONB NOT NULL DEFAULT '[]',
    sessions_referenced JSONB NOT NULL DEFAULT '[]',
    references_data JSONB NOT NULL DEFAULT '[]',
    llm_provider TEXT NOT NULL,
    llm_model TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_code_stories_lookup ON code_stories (repo_id, file_path, function_name, ref_name);
CREATE INDEX idx_code_stories_repo ON code_stories (repo_id);

-- Invitation requests (public, no auth required to submit)
CREATE TABLE invitation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    name TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invitation_requests_org ON invitation_requests(org_id, status);
CREATE INDEX idx_invitation_requests_email ON invitation_requests(email);

-- Model pricing
CREATE TABLE model_pricing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    model TEXT NOT NULL,
    input_per_mtok DOUBLE PRECISION NOT NULL,
    output_per_mtok DOUBLE PRECISION NOT NULL,
    cache_read_per_mtok DOUBLE PRECISION NOT NULL,
    cache_write_per_mtok DOUBLE PRECISION NOT NULL,
    source TEXT NOT NULL DEFAULT 'manual',
    effective_from TIMESTAMPTZ NOT NULL,
    effective_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_model_pricing_lookup ON model_pricing(model, effective_from);

-- Pricing sync log
CREATE TABLE pricing_sync_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    synced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    models_updated TEXT[] NOT NULL DEFAULT '{}',
    source TEXT NOT NULL DEFAULT 'litellm',
    error TEXT
);
