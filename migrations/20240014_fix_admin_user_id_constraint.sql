-- Fix admin_user_id foreign key constraint to allow NULL values
-- This allows hospital registration without requiring a pre-existing user
-- Drop the existing foreign key constraint
ALTER TABLE hospitals DROP CONSTRAINT IF EXISTS hospitals_admin_user_id_fkey;
-- Re-add the constraint but allow NULL values
ALTER TABLE hospitals
ADD CONSTRAINT hospitals_admin_user_id_fkey FOREIGN KEY (admin_user_id) REFERENCES users(id) ON DELETE
SET NULL;
-- Make admin_user_id nullable if it isn't already
ALTER TABLE hospitals
ALTER COLUMN admin_user_id DROP NOT NULL;
COMMENT ON COLUMN hospitals.admin_user_id IS 'User ID of the hospital administrator who registered (nullable during registration)';