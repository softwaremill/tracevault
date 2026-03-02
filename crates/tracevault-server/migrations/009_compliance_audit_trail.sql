-- 1. Add immutability columns to commits
ALTER TABLE commits ADD COLUMN signature TEXT;
ALTER TABLE commits ADD COLUMN chain_hash TEXT;
ALTER TABLE commits ADD COLUMN prev_chain_hash TEXT;
ALTER TABLE commits ADD COLUMN record_hash TEXT;
ALTER TABLE commits ADD COLUMN sealed_at TIMESTAMPTZ;

-- 2. Add immutability columns to sessions
ALTER TABLE sessions ADD COLUMN signature TEXT;
ALTER TABLE sessions ADD COLUMN record_hash TEXT;
ALTER TABLE sessions ADD COLUMN sealed_at TIMESTAMPTZ;

-- 3. Create amendments table
CREATE TABLE amendments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_commit_id UUID NOT NULL REFERENCES commits(id),
    reason TEXT NOT NULL,
    amended_by UUID NOT NULL REFERENCES users(id),
    amendment_data JSONB NOT NULL,
    signature TEXT,
    record_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_amendments_commit ON amendments(original_commit_id);

-- 4. Create audit_log table
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_log_org ON audit_log(org_id);
CREATE INDEX idx_audit_log_actor ON audit_log(actor_id);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX idx_audit_log_created ON audit_log(created_at);

-- 5. Create org_compliance_settings table
CREATE TABLE org_compliance_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL UNIQUE REFERENCES orgs(id),
    retention_days INTEGER NOT NULL DEFAULT 365,
    signing_enabled BOOLEAN NOT NULL DEFAULT false,
    signing_key_id TEXT,
    chain_verification_interval_hours INTEGER DEFAULT 24,
    compliance_mode TEXT NOT NULL DEFAULT 'none',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 6. Expand user roles - update existing 'member' to 'developer'
UPDATE users SET role = 'developer' WHERE role = 'member';

-- 7. Create chain_verifications table for caching verification results
CREATE TABLE chain_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id),
    status TEXT NOT NULL,
    total_commits INTEGER NOT NULL DEFAULT 0,
    verified_commits INTEGER NOT NULL DEFAULT 0,
    errors JSONB,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chain_verifications_org ON chain_verifications(org_id);
