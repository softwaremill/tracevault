-- Per-organization LLM settings
ALTER TABLE org_compliance_settings
    ADD COLUMN llm_provider TEXT,
    ADD COLUMN llm_api_key_encrypted TEXT,
    ADD COLUMN llm_api_key_nonce TEXT,
    ADD COLUMN llm_model TEXT,
    ADD COLUMN llm_base_url TEXT;
