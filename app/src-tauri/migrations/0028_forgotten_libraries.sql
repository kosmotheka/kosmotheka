-- Libraries this device left or was cut off from. Local only: the cloud must not
-- hand them back (SEG-017).
CREATE TABLE forgotten_libraries (
    library_id TEXT PRIMARY KEY
) STRICT, WITHOUT ROWID;
