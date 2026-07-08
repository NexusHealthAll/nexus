-- Deterministic salted hash of the BVN/NIN so we can enforce
-- cross-owner uniqueness (the encrypted number varies per encryption).
-- Partial unique index applies only to verified rows so a failed
-- verification doesn't lock the same number out permanently.
ALTER TABLE identity_verifications
    ADD COLUMN IF NOT EXISTS number_hash TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS uq_identity_verified_number
    ON identity_verifications (identity_type, number_hash)
    WHERE status = 'verified' AND number_hash IS NOT NULL;
