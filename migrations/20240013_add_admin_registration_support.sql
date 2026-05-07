-- Add support for hospital admin registration workflow (AC-01 to AC-05)
-- This migration adds the registration_status enum and related fields
-- Create registration_status enum for admin approval workflow
CREATE TYPE registration_status AS ENUM ('pending', 'approved', 'rejected');
-- Create audit event types for registration tracking
CREATE TYPE audit_event_type AS ENUM (
    'registration_created',
    'status_changed',
    'document_uploaded',
    'payment_method_added',
    'location_updated'
);
-- Create actor type for audit logs
CREATE TYPE actor_type AS ENUM ('user', 'admin', 'system');
-- Create payment_method_type enum
CREATE TYPE payment_method_type AS ENUM ('card', 'bank_account');
-- Add registration status and approval fields to hospitals table
ALTER TABLE hospitals
ADD COLUMN IF NOT EXISTS admin_registration_status registration_status DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS approved_by UUID REFERENCES users (id) ON DELETE
SET NULL,
    ADD COLUMN IF NOT EXISTS rejection_reason TEXT,
    ADD COLUMN IF NOT EXISTS admin_user_id UUID REFERENCES users (id) ON DELETE
SET NULL;
-- Create index for registration status queries
CREATE INDEX IF NOT EXISTS idx_hospitals_admin_registration_status ON hospitals (admin_registration_status);
-- Create audit log table for registration events
CREATE TABLE IF NOT EXISTS hospital_registration_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hospital_id UUID NOT NULL REFERENCES hospitals (id) ON DELETE CASCADE,
    event_type audit_event_type NOT NULL,
    actor_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    actor_type actor_type NOT NULL,
    old_value JSONB,
    new_value JSONB,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_registration_audit_hospital_id ON hospital_registration_audit (hospital_id);
CREATE INDEX IF NOT EXISTS idx_registration_audit_event_type ON hospital_registration_audit (event_type);
CREATE INDEX IF NOT EXISTS idx_registration_audit_created_at ON hospital_registration_audit (created_at DESC);
-- Add payment_method_type to hospital_payment_methods if not exists
DO $$ BEGIN IF NOT EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_name = 'hospital_payment_methods'
        AND column_name = 'payment_method_type'
) THEN
ALTER TABLE hospital_payment_methods
ADD COLUMN payment_method_type payment_method_type DEFAULT 'card';
END IF;
END $$;
-- Add encrypted_token column for storing encrypted payment tokens
ALTER TABLE hospital_payment_methods
ADD COLUMN IF NOT EXISTS encrypted_token TEXT;
COMMENT ON TABLE hospital_registration_audit IS 'Immutable audit trail for hospital admin registration workflow (AC-01 to AC-05)';
COMMENT ON COLUMN hospitals.admin_registration_status IS 'Registration status for admin approval workflow (pending/approved/rejected)';
COMMENT ON COLUMN hospitals.approved_at IS 'Timestamp when hospital was approved by system admin';
COMMENT ON COLUMN hospitals.approved_by IS 'System admin who approved the hospital registration';