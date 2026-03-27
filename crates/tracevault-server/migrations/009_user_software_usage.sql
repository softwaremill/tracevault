CREATE TABLE user_software_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    software_name TEXT NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 1,
    first_seen_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    UNIQUE(session_id, software_name)
);

CREATE INDEX idx_user_software_org ON user_software_usage(org_id);
CREATE INDEX idx_user_software_user ON user_software_usage(user_id);
CREATE INDEX idx_user_software_name ON user_software_usage(software_name);
