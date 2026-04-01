-- Move webhook secret from global env var to per-repo encrypted column
ALTER TABLE repos ADD COLUMN webhook_secret_encrypted TEXT;
ALTER TABLE repos ADD COLUMN webhook_secret_nonce TEXT;
