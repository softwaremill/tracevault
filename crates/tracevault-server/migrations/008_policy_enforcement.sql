-- Add repo_id and updated_at to existing policies table
ALTER TABLE policies ADD COLUMN repo_id UUID REFERENCES repos(id) ON DELETE CASCADE;
ALTER TABLE policies ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE policies ALTER COLUMN description SET DEFAULT '';
ALTER TABLE policies ALTER COLUMN description SET NOT NULL;

CREATE INDEX idx_policies_org_id ON policies(org_id);
CREATE INDEX idx_policies_repo_id ON policies(repo_id);
