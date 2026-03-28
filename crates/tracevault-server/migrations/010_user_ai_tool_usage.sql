CREATE TABLE user_ai_tool_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tool_category TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    usage_count INTEGER NOT NULL DEFAULT 1,
    first_seen_at TIMESTAMPTZ NOT NULL,
    last_seen_at TIMESTAMPTZ NOT NULL,
    UNIQUE(session_id, tool_category, tool_name)
);

CREATE INDEX idx_user_ai_tool_org ON user_ai_tool_usage(org_id);
CREATE INDEX idx_user_ai_tool_user ON user_ai_tool_usage(user_id);
CREATE INDEX idx_user_ai_tool_category ON user_ai_tool_usage(tool_category);
CREATE INDEX idx_user_ai_tool_name ON user_ai_tool_usage(tool_name);
