-- Allow multiple seals per session (commit snapshots + final seal)
ALTER TABLE session_seals DROP CONSTRAINT IF EXISTS session_seals_session_id_key;

-- Add seal metadata columns
ALTER TABLE session_seals ADD COLUMN commit_seal_id UUID REFERENCES commit_seals(id);
ALTER TABLE session_seals ADD COLUMN seal_type TEXT NOT NULL DEFAULT 'commit_snapshot';

-- Index for querying seals by session ordered by time
CREATE INDEX idx_session_seals_session_sealed ON session_seals(session_id, sealed_at);

-- Index for finding session seals linked to a commit seal
CREATE INDEX idx_session_seals_commit_seal ON session_seals(commit_seal_id);

-- Add session verification counts to chain_verifications
ALTER TABLE chain_verifications ADD COLUMN total_sessions INTEGER NOT NULL DEFAULT 0;
ALTER TABLE chain_verifications ADD COLUMN verified_sessions INTEGER NOT NULL DEFAULT 0;
