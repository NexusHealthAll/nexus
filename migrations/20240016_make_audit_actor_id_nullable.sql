-- Make actor_id nullable in hospital_registration_audit for registration flow
-- During registration, we don't have a user account yet, so we can't reference it
-- First, drop the foreign key constraint
ALTER TABLE hospital_registration_audit DROP CONSTRAINT IF EXISTS hospital_registration_audit_actor_id_fkey;
-- Make the column nullable
ALTER TABLE hospital_registration_audit
ALTER COLUMN actor_id DROP NOT NULL;
-- Re-add the foreign key constraint with ON DELETE SET NULL
ALTER TABLE hospital_registration_audit
ADD CONSTRAINT hospital_registration_audit_actor_id_fkey FOREIGN KEY (actor_id) REFERENCES users (id) ON DELETE
SET NULL;
COMMENT ON COLUMN hospital_registration_audit.actor_id IS 'User who performed the action. NULL during initial registration before user account is created.';