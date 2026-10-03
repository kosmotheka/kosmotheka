-- Each operation names the previous one of its author in the library (D87).
-- Operations logged before this have none.
ALTER TABLE operations ADD COLUMN previous TEXT;

-- A fence names the last operation of the device the administrator had, not a
-- date (D87): `trusted_until` becomes `trusted_through`.
ALTER TABLE devices RENAME COLUMN trusted_until TO trusted_through;

-- Derived from the log, so it is dropped and rebuilt with the library.
-- An operation of `device_id` with an HLC below `until_hlc` is a request unless it
-- is `through` itself or one of its ancestors (`device_fence_chains`).
DROP TABLE device_fences;
CREATE TABLE device_fences (
    library_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    through TEXT NOT NULL,
    until_hlc TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    PRIMARY KEY (library_id, operation_id, device_id)
) STRICT;

-- The operations each fence still counts: `through` and its ancestors, worked
-- out once when the fence is noted.
CREATE TABLE device_fence_chains (
    library_id TEXT NOT NULL,
    fence_operation_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    PRIMARY KEY (library_id, fence_operation_id, device_id, operation_id)
) STRICT, WITHOUT ROWID;
