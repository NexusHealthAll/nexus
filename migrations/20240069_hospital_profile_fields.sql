-- Hospital profile fields the settings screen edits but had nowhere to store.
--
-- `PATCH /api/v1/hospitals/{id}` only ever accepted name/email/address/phone/
-- logo, so the type, staff size, opening times and department list a hospital
-- filled in were discarded on save — the department list in particular lived
-- only in the frontend's local state and vanished on reload.
--
-- `hospital_type` is TEXT + CHECK rather than a Postgres enum, for the reason
-- stated across this migration set: `ALTER TYPE … ADD VALUE` cannot run inside
-- sqlx's per-file transaction, so an enum cannot be extended later. `other` is
-- in the set deliberately — a hospital that fits none of the listed types must
-- still be able to save the rest of its profile.
--
-- `operating_hours` is JSONB rather than a `hospital_operating_hours` table
-- because it is a fixed-size value read and written whole, always with its
-- hospital, and never queried by day. The shape is validated in Rust
-- (`OperatingHours`): a list of {day, opens_at, closes_at, closed}, one entry
-- per day, times as HH:MM. A CHECK here could not express that.
--
-- Departments DO get their own table: they are added and removed individually,
-- are ordered, and are the thing a future "find a hospital with an ICU" search
-- would match on. It mirrors `clinician_qualifications` (20240029) down to the
-- unique constraint and the LOWER() index, because it is the same shape of
-- data and the same access pattern.

ALTER TABLE hospitals
    ADD COLUMN IF NOT EXISTS hospital_type    TEXT,
    ADD COLUMN IF NOT EXISTS staff_size       INTEGER,
    ADD COLUMN IF NOT EXISTS operating_hours  JSONB;

ALTER TABLE hospitals
    DROP CONSTRAINT IF EXISTS chk_hospitals_hospital_type;
ALTER TABLE hospitals
    ADD CONSTRAINT chk_hospitals_hospital_type CHECK (
        hospital_type IS NULL OR hospital_type IN (
            'general',
            'teaching',
            'specialist',
            'clinic',
            'primary_health_center',
            'maternity',
            'diagnostic_center',
            'other'
        )
    );

-- A staff size of 0 is meaningless and a negative one is a bug upstream; the
-- upper bound is a sanity rail, not a policy (the largest Nigerian teaching
-- hospitals run to a few thousand staff).
ALTER TABLE hospitals
    DROP CONSTRAINT IF EXISTS chk_hospitals_staff_size;
ALTER TABLE hospitals
    ADD CONSTRAINT chk_hospitals_staff_size CHECK (
        staff_size IS NULL OR (staff_size > 0 AND staff_size <= 100000)
    );

CREATE TABLE IF NOT EXISTS hospital_departments (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    hospital_id UUID         NOT NULL REFERENCES hospitals (id) ON DELETE CASCADE,
    department  VARCHAR(200) NOT NULL,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_hospital_department UNIQUE (hospital_id, department)
);

CREATE INDEX IF NOT EXISTS idx_hospital_departments_hospital_id
    ON hospital_departments (hospital_id);

-- Case-insensitive lookups, so "ICU" matches a stored "icu".
CREATE INDEX IF NOT EXISTS idx_hospital_departments_lower
    ON hospital_departments (LOWER(department));

-- Rollback:
--   DROP INDEX IF EXISTS idx_hospital_departments_lower;
--   DROP INDEX IF EXISTS idx_hospital_departments_hospital_id;
--   DROP TABLE IF EXISTS hospital_departments;
--   ALTER TABLE hospitals DROP CONSTRAINT IF EXISTS chk_hospitals_staff_size;
--   ALTER TABLE hospitals DROP CONSTRAINT IF EXISTS chk_hospitals_hospital_type;
--   ALTER TABLE hospitals DROP COLUMN IF EXISTS operating_hours;
--   ALTER TABLE hospitals DROP COLUMN IF EXISTS staff_size;
--   ALTER TABLE hospitals DROP COLUMN IF EXISTS hospital_type;
