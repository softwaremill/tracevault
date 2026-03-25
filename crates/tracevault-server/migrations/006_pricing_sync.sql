-- Add source column to model_pricing
ALTER TABLE model_pricing ADD COLUMN source TEXT NOT NULL DEFAULT 'manual';

-- Create pricing sync log table
CREATE TABLE IF NOT EXISTS pricing_sync_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    synced_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    models_updated TEXT[] NOT NULL DEFAULT '{}',
    source TEXT NOT NULL DEFAULT 'litellm',
    error TEXT
);
