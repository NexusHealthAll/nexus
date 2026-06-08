-- Tier 3.6 — Two related caches:
--   1. clinician_qualifications stores free-text qualification tags so we
--      can compute the FRS §3.4.3 "qualifications match" component for real.
--   2. clinicians.acceptance_rate_pct caches the rolling accept/(accept +
--      decline + expire) percentage, recomputed by the service after each
--      offer-lifecycle write. NULL means "no offer history yet".

CREATE TABLE IF NOT EXISTS clinician_qualifications (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    clinician_id  UUID         NOT NULL REFERENCES clinicians (id) ON DELETE CASCADE,
    qualification VARCHAR(200) NOT NULL,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_clinician_qualification UNIQUE (clinician_id, qualification)
);

CREATE INDEX IF NOT EXISTS idx_clinician_quals_clinician_id
    ON clinician_qualifications (clinician_id);

-- Case-insensitive lookups (e.g. "ACLS Certified" should match "acls certified").
CREATE INDEX IF NOT EXISTS idx_clinician_quals_lower
    ON clinician_qualifications (LOWER(qualification));

ALTER TABLE clinicians
    ADD COLUMN IF NOT EXISTS acceptance_rate_pct REAL
        CHECK (acceptance_rate_pct IS NULL OR acceptance_rate_pct BETWEEN 0 AND 100);
