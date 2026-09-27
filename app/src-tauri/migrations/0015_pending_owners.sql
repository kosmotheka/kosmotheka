-- Whose a newly paired device is, decided by an administrator before that device
-- has registered in the library (milestone 5.7). Only the device itself can create
-- its `device` row, so the assignment waits here and is recorded as soon as the
-- registration arrives. Local only.
CREATE TABLE pending_owners (
    library_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    PRIMARY KEY (library_id, device_id)
) STRICT;
