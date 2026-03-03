-- Store structured commit/session references in code stories
ALTER TABLE code_stories ADD COLUMN references_data JSONB NOT NULL DEFAULT '[]';
