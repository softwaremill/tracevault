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

-- Commits (from 005 + 009 immutability columns)
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id UUID NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    commit_sha TEXT NOT NULL,
    branch TEXT,
    author TEXT NOT NULL,
    diff_data JSONB,
    attribution JSONB,
    signature TEXT,
    chain_hash TEXT,
    prev_chain_hash TEXT,
    record_hash TEXT,
    sealed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(repo_id, commit_sha)
);

CREATE INDEX idx_commits_repo_id ON commits(repo_id);
CREATE INDEX idx_commits_commit_sha ON commits(commit_sha);
CREATE INDEX idx_commits_created_at ON commits(created_at);

-- Sessions (from 005 + 006 + 007 + 009 immutability columns)
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
    model_usage JSONB,
    duration_ms BIGINT,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    user_messages INTEGER,
    assistant_messages INTEGER,
    tool_calls JSONB,
    total_tool_calls INTEGER,
    cache_read_tokens BIGINT,
    cache_write_tokens BIGINT,
    compactions INTEGER,
    compaction_tokens_saved BIGINT,
    signature TEXT,
    record_hash TEXT,
    sealed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(commit_id, session_id)
);

CREATE INDEX idx_sessions_commit_id ON sessions(commit_id);

-- Policies (from 001 + 008)
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

-- Amendments (from 009)
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

-- Audit log (from 009)
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

-- Chain verifications (from 009)
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

-- Code stories (from 010 + 013)
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
