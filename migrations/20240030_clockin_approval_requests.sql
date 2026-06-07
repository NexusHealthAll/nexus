-- Tier 3.5 — GPS-fallback clock-in approval workflow (FRS §3.6.6).
--
-- Workers whose GPS is inaccurate can submit a photo of the hospital entrance
-- plus the device-reported coordinates. A hospital admin then approves or
-- denies. On approval, the clock-in endpoint accepts the worker's `manual`
-- clock-in method for this specific shift.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'clockin_approval_status') THEN
        CREATE TYPE clockin_approval_status AS ENUM ('pending', 'approved', 'denied');
    END IF;
END$$;

CREATE TABLE IF NOT EXISTS clockin_approval_requests (
    id              UUID                    PRIMARY KEY DEFAULT gen_random_uuid(),
    shift_id        UUID                    NOT NULL REFERENCES shifts (id) ON DELETE CASCADE,
    clinician_id    UUID                    NOT NULL REFERENCES clinicians (id) ON DELETE CASCADE,
    -- Device-reported GPS coords at the time of the photo (for audit).
    latitude        DOUBLE PRECISION,
    longitude       DOUBLE PRECISION,
    -- Raw photo bytes. Production would store an S3 URL here instead.
    photo_bytes     BYTEA                   NOT NULL,
    -- e.g. "image/jpeg" — set by the client.
    photo_mime_type VARCHAR(50),
    status          clockin_approval_status NOT NULL DEFAULT 'pending',
    submitted_at    TIMESTAMPTZ             NOT NULL DEFAULT NOW(),
    decided_at      TIMESTAMPTZ,
    decided_by      UUID                    REFERENCES users (id),
    decision_notes  TEXT,
    created_at      TIMESTAMPTZ             NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ             NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_clockin_approval_pending UNIQUE (shift_id, clinician_id)
);

CREATE INDEX IF NOT EXISTS idx_clockin_approval_shift_id
    ON clockin_approval_requests (shift_id);
CREATE INDEX IF NOT EXISTS idx_clockin_approval_pending
    ON clockin_approval_requests (shift_id) WHERE status = 'pending';

CREATE TRIGGER trg_clockin_approval_updated_at
    BEFORE UPDATE ON clockin_approval_requests
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
