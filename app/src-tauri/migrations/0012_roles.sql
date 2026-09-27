-- People and roles (D55, D73). A library without any role works as before.
ALTER TABLE members ADD COLUMN role TEXT CHECK (role IS NULL OR role IN ('admin', 'editor', 'reader'));
ALTER TABLE members ADD COLUMN can_share INTEGER CHECK (can_share IS NULL OR can_share IN (0, 1));

-- The member a device belongs to. No foreign key: a device of someone removed from
-- the library simply belongs to nobody, and counts as a reader.
ALTER TABLE devices ADD COLUMN member_id TEXT;

-- An operation its author was not allowed to make stays in the log as a request
-- for the administrators, instead of being applied.
ALTER TABLE operations ADD COLUMN status TEXT NOT NULL DEFAULT 'applied'
    CHECK (status IN ('applied', 'request'));

-- Which device created each row, so an editor can delete what their person
-- created (D45). Rebuilt from the log with the rest of the library.
CREATE TABLE row_authors (
    library_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    PRIMARY KEY (library_id, entity, entity_id)
) STRICT, WITHOUT ROWID;
