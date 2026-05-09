-- Make added_by nullable in hospital_payment_methods for registration flow
-- During registration, we don't have a user account yet, so we can't reference it
ALTER TABLE hospital_payment_methods
ALTER COLUMN added_by DROP NOT NULL;
COMMENT ON COLUMN hospital_payment_methods.added_by IS 'User who added the payment method. NULL during initial registration before user account is created.';