-- Antedating fences (D84): the HLC up to which the administrator knew a device's
-- operations when it lost power, set by the operation that took the power away.
ALTER TABLE devices ADD COLUMN trusted_until TEXT;

-- Every fence ever honoured, not only the latest: a later one must not reopen the
-- window of an earlier one. Derived from the log, so it is rebuilt with the library.
-- An operation of `device_id` with an HLC between the two is made as a request.
CREATE TABLE device_fences (
    library_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    after_hlc TEXT NOT NULL,
    until_hlc TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    PRIMARY KEY (library_id, operation_id, device_id)
) STRICT;
