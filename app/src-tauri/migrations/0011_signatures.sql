-- Signed operations (D56). Each device that writes to a library registers the public
-- key it signs with, and the row travels with the library like any other.
CREATE TABLE devices (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    public_key TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (library_id, id)
) STRICT;

-- The author's signature, kept to pass the operation on to other devices unchanged.
-- Operations from before signatures existed have none.
ALTER TABLE operations ADD COLUMN signature TEXT;
