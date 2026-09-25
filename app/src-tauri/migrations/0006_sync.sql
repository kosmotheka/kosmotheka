-- Sync (D31-D33, docs/architecture/sync.md).

-- Operations carry the schema version they were recorded with; only replicable ones travel.
ALTER TABLE operations ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE operations ADD COLUMN replicable INTEGER NOT NULL DEFAULT 1 CHECK (replicable IN (0, 1));
-- Everything recorded before sync existed stays as history only.
UPDATE operations SET replicable = 0;
CREATE INDEX operations_device_hlc ON operations (device_id, hlc);

-- Last HLC that wrote each field, for field-level last-writer-wins.
CREATE TABLE field_versions (
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    field TEXT NOT NULL,
    hlc TEXT NOT NULL,
    PRIMARY KEY (entity, entity_id, field)
) STRICT, WITHOUT ROWID;

-- Deleted rows: a delete beats concurrent edits.
CREATE TABLE tombstones (
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    hlc TEXT NOT NULL,
    PRIMARY KEY (entity, entity_id)
) STRICT, WITHOUT ROWID;

-- The same name or ISBN created offline on two devices must be able to coexist (D33).
DROP INDEX people_name;
CREATE INDEX people_name ON people (name COLLATE NOCASE);
DROP INDEX members_name;
CREATE INDEX members_name ON members (name COLLATE NOCASE);
DROP INDEX publishers_name;
CREATE INDEX publishers_name ON publishers (name COLLATE NOCASE);

-- book_details.isbn13 was UNIQUE as a column constraint, which needs a table rebuild.
CREATE TABLE book_details_new (
    edition_id TEXT PRIMARY KEY NOT NULL REFERENCES editions (id) ON DELETE CASCADE,
    isbn13 TEXT CHECK (isbn13 IS NULL OR length(isbn13) = 13),
    google_books_id TEXT,
    open_library_id TEXT
) STRICT;
INSERT INTO book_details_new (edition_id, isbn13, google_books_id, open_library_id)
SELECT edition_id, isbn13, google_books_id, open_library_id FROM book_details;
DROP TABLE book_details;
ALTER TABLE book_details_new RENAME TO book_details;
CREATE INDEX book_details_isbn ON book_details (isbn13);
