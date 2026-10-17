-- 20240072 — the clock-in geofence is 250 m, and it is for physical shifts only.
--
-- Two corrections in one migration:
--
--   1. The default clock-in radius moves from 100 m to 250 m. 100 m is tight
--      enough that ordinary phone GPS drift inside a hospital building pushes a
--      worker outside it; 250 m covers the entrance, car park and lobby without
--      letting someone clock in from down the road.
--   2. Nothing here applies to virtual shifts. The 10 km geofence that used to
--      guard the LiveKit join token is gone (see `VideoService::issue_join_token`)
--      — a virtual consultation is attended from anywhere, so only the physical
--      GPS clock-in reads this column.
--
-- The existing CHECK (clock_in_radius_meters BETWEEN 50 AND 5000) already admits
-- 250, so the constraint is untouched.

ALTER TABLE hospital_locations
    ALTER COLUMN clock_in_radius_meters SET DEFAULT 250;

-- Rows still sitting on the old column default were never customised by a
-- hospital, so they move to the new one. A radius a hospital deliberately set to
-- anything other than 100 is left exactly as it is.
UPDATE hospital_locations
SET    clock_in_radius_meters = 250,
       updated_at             = NOW()
WHERE  clock_in_radius_meters = 100;
