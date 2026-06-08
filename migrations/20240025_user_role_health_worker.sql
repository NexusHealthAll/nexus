-- FRS v2.0 §2: spec calls the role "Health Worker"; code previously named it
-- "Staff". Rename the enum value to match. Postgres ≥ 10 supports this directly.
ALTER TYPE user_role RENAME VALUE 'staff' TO 'health_worker';
