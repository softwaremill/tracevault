-- Add clone management columns to repos
ALTER TABLE repos ADD COLUMN clone_status TEXT NOT NULL DEFAULT 'pending';
ALTER TABLE repos ADD COLUMN clone_path TEXT;
ALTER TABLE repos ADD COLUMN last_fetched_at TIMESTAMPTZ;

-- Story cache table
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
    llm_provider TEXT NOT NULL,
    llm_model TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_code_stories_lookup ON code_stories (repo_id, file_path, function_name, ref_name);
CREATE INDEX idx_code_stories_repo ON code_stories (repo_id);
