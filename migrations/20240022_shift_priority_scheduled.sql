-- Add `scheduled` variant to the shift_priority enum to match FRS v2.0 §3.1.3 F1-F07.
ALTER TYPE shift_priority ADD VALUE IF NOT EXISTS 'scheduled';
