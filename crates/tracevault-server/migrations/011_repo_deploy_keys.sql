-- Add encrypted deploy key column to repos
ALTER TABLE repos ADD COLUMN deploy_key_encrypted TEXT;
ALTER TABLE repos ADD COLUMN deploy_key_nonce TEXT;
