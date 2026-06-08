-- =============================================================================
-- Shift Marketplace v2 schema additions (FRS v2.0 §3.4–3.7, §3.9)
--   * shift_assignments — offer / accept / decline / expire lifecycle
--   * shift_handovers   — end-of-shift documentation (F1-H01..H05)
--   * shift_ratings     — mutual rating system (hospital ↔ worker)
--   * shifts.virtual_link — extract auto-generated URL out of `notes`
--   * shift_attendance late-clockin tracking
--   * shift_status: add 'assigned' value (offered → accepted lifecycle)
-- =============================================================================

-- ---------------------------------------------------------------------------
-- shift_status: add 'assigned' state.
-- Sequence: open → assigned (offer accepted) → upcoming (start time near) →
--           in_progress (clocked in) → completed.
-- ---------------------------------------------------------------------------
ALTER TYPE shift_status ADD VALUE IF NOT EXISTS 'assigned' AFTER 'open';

-- ---------------------------------------------------------------------------
-- F1-F15: store the auto-generated virtual consultation link in its own column
-- instead of stringly-embedded in `notes`.
-- ---------------------------------------------------------------------------
ALTER TABLE shifts
    ADD COLUMN IF NOT EXISTS virtual_link TEXT;

-- ---------------------------------------------------------------------------
-- shift_assignment_status — offer lifecycle for §3.4 / §3.5
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'shift_assignment_status') THEN
        CREATE TYPE shift_assignment_status AS ENUM (
            'offered',   -- hospital sent offer, awaiting worker response
            'accepted',  -- worker accepted (NDPR consent recorded)
            'declined',  -- worker explicitly declined
            'expired'    -- 30-min acceptance window passed without response
        );
    END IF;
END$$;

-- ---------------------------------------------------------------------------
-- shift_assignments — one row per (shift, clinician) offer.
-- BR-F1-21 30-min acceptance window via expires_at.
-- BR-F1-24 uniqueness prevents re-offering the same clinician for the same shift.
-- ndpr_consent stores the 5 NDPR booleans from §3.5.3 at accept time.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS shift_assignments (
    id                UUID                    PRIMARY KEY DEFAULT gen_random_uuid(),
    shift_id          UUID                    NOT NULL REFERENCES shifts (id) ON DELETE CASCADE,
    clinician_id      UUID                    NOT NULL REFERENCES clinicians (id) ON DELETE CASCADE,
    status            shift_assignment_status NOT NULL DEFAULT 'offered',
    offered_at        TIMESTAMPTZ             NOT NULL DEFAULT NOW(),
    responded_at      TIMESTAMPTZ,
    expires_at        TIMESTAMPTZ             NOT NULL,
    decline_reason    TEXT,
    ndpr_consent      JSONB,
    created_at        TIMESTAMPTZ             NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ             NOT NULL DEFAULT NOW(),

    CONSTRAINT uniq_shift_clinician_assignment UNIQUE (shift_id, clinician_id)
);

CREATE INDEX IF NOT EXISTS idx_shift_assignments_shift_id     ON shift_assignments (shift_id);
CREATE INDEX IF NOT EXISTS idx_shift_assignments_clinician_id ON shift_assignments (clinician_id);
CREATE INDEX IF NOT EXISTS idx_shift_assignments_expiry       ON shift_assignments (expires_at)
    WHERE status = 'offered';

CREATE TRIGGER trg_shift_assignments_updated_at
    BEFORE UPDATE ON shift_assignments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ---------------------------------------------------------------------------
-- shift_handovers — F1-H01..H05 (§3.7.3)
-- BR-F1-36 editable_until = submitted_at + 1h
-- BR-F1-39 auto_approve_after = submitted_at + 48h
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS shift_handovers (
    id                     UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    shift_id               UUID         NOT NULL UNIQUE REFERENCES shifts (id) ON DELETE CASCADE,

    -- F1-H01..H05
    patients_seen          INTEGER      NOT NULL CHECK (patients_seen >= 0),
    critical_patients      JSONB        NOT NULL DEFAULT '[]'::jsonb,
    pending_tasks          JSONB        NOT NULL DEFAULT '[]'::jsonb,
    instructions           TEXT         NOT NULL,
    equipment_status       TEXT,

    -- Timestamps governing edit / approval windows.
    submitted_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    editable_until         TIMESTAMPTZ  NOT NULL,
    auto_approve_after     TIMESTAMPTZ  NOT NULL,
    hospital_approved_at   TIMESTAMPTZ,
    revision_requested_at  TIMESTAMPTZ,
    revision_notes         TEXT,

    created_at             TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_handovers_auto_approve ON shift_handovers (auto_approve_after)
    WHERE hospital_approved_at IS NULL AND revision_requested_at IS NULL;

CREATE TRIGGER trg_shift_handovers_updated_at
    BEFORE UPDATE ON shift_handovers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ---------------------------------------------------------------------------
-- shift_attendance: late-clockin tracking + handover link.
-- Spec §3.6.7: 0-15min OK, 15-30min -25% pay for first hour,
-- 30-60min hospital approval required, >60min missed.
-- ---------------------------------------------------------------------------
ALTER TABLE shift_attendance
    ADD COLUMN IF NOT EXISTS handover_id          UUID REFERENCES shift_handovers (id),
    ADD COLUMN IF NOT EXISTS late_minutes         INTEGER     CHECK (late_minutes IS NULL OR late_minutes >= 0),
    ADD COLUMN IF NOT EXISTS late_penalty_applied BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS missed               BOOLEAN     NOT NULL DEFAULT FALSE;

-- ---------------------------------------------------------------------------
-- rating_ratee_kind: enum for who is being rated.
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'rating_ratee_kind') THEN
        CREATE TYPE rating_ratee_kind AS ENUM ('clinician', 'hospital');
    END IF;
END$$;

-- ---------------------------------------------------------------------------
-- shift_ratings — mutual ratings (§3.9).
-- BR-F1-46 7-day submission window after shift completion → window_closes_at.
-- BR-F1-50 editable for 48h after submission → editable_until.
-- BR-F1-48 anonymous flag (default true).
-- The `dimensions` jsonb holds the 4 hospital sub-scores when ratee_kind='hospital'.
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS shift_ratings (
    id                UUID               PRIMARY KEY DEFAULT gen_random_uuid(),
    shift_id          UUID               NOT NULL REFERENCES shifts (id) ON DELETE CASCADE,
    rater_user_id     UUID               NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    ratee_id          UUID               NOT NULL,
    ratee_kind        rating_ratee_kind  NOT NULL,
    score             SMALLINT           NOT NULL CHECK (score BETWEEN 1 AND 5),
    dimensions        JSONB,
    comment           TEXT,
    is_anonymous      BOOLEAN            NOT NULL DEFAULT TRUE,
    editable_until    TIMESTAMPTZ        NOT NULL,
    window_closes_at  TIMESTAMPTZ        NOT NULL,
    created_at        TIMESTAMPTZ        NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ        NOT NULL DEFAULT NOW(),

    CONSTRAINT uniq_rating_per_shift UNIQUE (shift_id, rater_user_id, ratee_kind)
);

CREATE INDEX IF NOT EXISTS idx_shift_ratings_ratee ON shift_ratings (ratee_kind, ratee_id);

CREATE TRIGGER trg_shift_ratings_updated_at
    BEFORE UPDATE ON shift_ratings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
