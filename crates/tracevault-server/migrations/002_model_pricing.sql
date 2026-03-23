CREATE TABLE model_pricing (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    model TEXT NOT NULL,
    input_per_mtok DOUBLE PRECISION NOT NULL,
    output_per_mtok DOUBLE PRECISION NOT NULL,
    cache_read_per_mtok DOUBLE PRECISION NOT NULL,
    cache_write_per_mtok DOUBLE PRECISION NOT NULL,
    effective_from TIMESTAMPTZ NOT NULL,
    effective_until TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_model_pricing_lookup
  ON model_pricing(model, effective_from);

-- Seed with current Anthropic rates
INSERT INTO model_pricing (model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, effective_from)
VALUES
    ('opus',   15.00, 75.00, 1.50,  18.75, '2025-01-01T00:00:00Z'),
    ('sonnet',  3.00, 15.00, 0.30,   3.75, '2025-01-01T00:00:00Z'),
    ('haiku',   0.80,  4.00, 0.08,   1.00, '2025-01-01T00:00:00Z');
