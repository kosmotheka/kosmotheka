-- Devices whose access to a library was withdrawn (D71). Synced with the library, so
-- every device that has it stops sharing it with them.
CREATE TABLE revocations (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    PRIMARY KEY (library_id, id)
) STRICT;
CREATE INDEX revocations_device ON revocations (library_id, device_id);

-- Libraries this device left, still to be told to a peer that shared them, so the
-- peer stops offering them. Local, never synced; a row goes once the peer stops
-- offering that library.
CREATE TABLE leave_notices (
    library_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    PRIMARY KEY (library_id, device_id)
) STRICT, WITHOUT ROWID;
