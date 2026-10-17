-- Years of experience and spoken languages on the worker's own profile.
--
-- Both already existed in the product but neither was persisted against the
-- clinician. `shift_applications.years_experience` (20240021) is per
-- application — a worker retyped it every time they applied, and nothing could
-- show it on their profile. Languages were kept in the frontend's
-- localStorage and never reached the backend at all, so they were lost on a
-- new device and invisible to the hospital deciding who to assign.
--
-- `clinician_languages` mirrors `clinician_qualifications` (20240029) exactly,
-- including the LOWER() index: same shape (a replaceable set of short tags),
-- same access pattern, and the same future use — qualifications already feed
-- shift matching, and language is the obvious next filter when a patient and a
-- clinician need a common one. Matching itself is untouched here.
--
-- `years_experience` stays on `clinicians` as a plain nullable column: it is a
-- single scalar, read with the profile every time, never a set.

ALTER TABLE clinicians
    ADD COLUMN IF NOT EXISTS years_experience INTEGER;

-- Nullable because an existing clinician has not stated it yet; the upper rail
-- rejects a typo'd 200 rather than a plausible career.
ALTER TABLE clinicians
    DROP CONSTRAINT IF EXISTS chk_clinicians_years_experience;
ALTER TABLE clinicians
    ADD CONSTRAINT chk_clinicians_years_experience CHECK (
        years_experience IS NULL OR (years_experience >= 0 AND years_experience <= 70)
    );

CREATE TABLE IF NOT EXISTS clinician_languages (
    id           UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    clinician_id UUID         NOT NULL REFERENCES clinicians (id) ON DELETE CASCADE,
    language     VARCHAR(100) NOT NULL,
    created_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_clinician_language UNIQUE (clinician_id, language)
);

CREATE INDEX IF NOT EXISTS idx_clinician_languages_clinician_id
    ON clinician_languages (clinician_id);

CREATE INDEX IF NOT EXISTS idx_clinician_languages_lower
    ON clinician_languages (LOWER(language));

-- Rollback:
--   DROP INDEX IF EXISTS idx_clinician_languages_lower;
--   DROP INDEX IF EXISTS idx_clinician_languages_clinician_id;
--   DROP TABLE IF EXISTS clinician_languages;
--   ALTER TABLE clinicians DROP CONSTRAINT IF EXISTS chk_clinicians_years_experience;
--   ALTER TABLE clinicians DROP COLUMN IF EXISTS years_experience;
