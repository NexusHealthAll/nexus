-- Tier 3.1 — Re-broadcast cadence (BR-F1-11..14) needs multiple
-- shift_broadcast_records rows per shift over time. The original schema
-- pinned shift_id as UNIQUE which blocked that. Drop the unique constraint
-- and add an index supporting "last broadcast for shift X".

ALTER TABLE shift_broadcast_records
    DROP CONSTRAINT IF EXISTS shift_broadcast_records_shift_id_key;

-- Replace the unique constraint's implicit index with a regular index. The
-- existing `idx_broadcast_records_shift_id` covers single-shift lookups; add
-- a (shift_id, broadcast_at DESC) index to make "most recent broadcast"
-- queries cheap.
CREATE INDEX IF NOT EXISTS idx_broadcast_records_shift_at
    ON shift_broadcast_records (shift_id, broadcast_at DESC);

-- broadcast_by may be NULL for system-driven re-broadcasts (the cadence
-- scheduler has no user context).
ALTER TABLE shift_broadcast_records
    ALTER COLUMN broadcast_by DROP NOT NULL;
