-- Add `midwife` variant to role_category to match FRS v2.0 §3.1.3 F1-F01.
ALTER TYPE role_category ADD VALUE IF NOT EXISTS 'midwife';
