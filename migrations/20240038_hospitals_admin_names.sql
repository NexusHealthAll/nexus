-- Persist the hospital admin's first + last name on the hospitals row so we
-- can create the users row at APPROVAL time (not registration). This means
-- there's no orphan users row for a pending hospital.
ALTER TABLE hospitals
    ADD COLUMN IF NOT EXISTS admin_first_name VARCHAR(100),
    ADD COLUMN IF NOT EXISTS admin_last_name  VARCHAR(100);
