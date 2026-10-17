-- Make years-of-experience and languages filterable, not just storable.
--
-- `20240070` added both fields and gave `clinician_languages` the same
-- `LOWER()` index `clinician_qualifications` has, so a language lookup was
-- already index-backed. What it did NOT add was anything for the numeric
-- field: `years_experience` only ever appeared in `WHERE id = $1` reads, so an
-- index would have been dead weight.
--
-- `GET /api/v1/shifts/{shift_id}/interested` now filters candidates by
-- `min_years_experience`, which is a range predicate over the table, so the
-- index earns its place here and not before.
--
-- Partial (`WHERE years_experience IS NOT NULL`) because the filter always
-- excludes workers who have not stated a figure — an unknown cannot be shown
-- to meet a minimum — so the NULL rows are never scanned through this path and
-- keeping them out leaves the index roughly the size of the stated population.

CREATE INDEX IF NOT EXISTS idx_clinicians_years_experience
    ON clinicians (years_experience)
    WHERE years_experience IS NOT NULL;

-- Rollback:
--   DROP INDEX IF EXISTS idx_clinicians_years_experience;
