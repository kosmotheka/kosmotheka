-- Libraries (D46, D54): every catalog row belongs to one, keyed by (library_id, id).
-- Existing data moves to a library with the same fixed id on every device, so devices
-- that already synced merge without duplicates. SQLite cannot change a primary key,
-- so each table is rebuilt: new tables first, then the old ones are dropped (children
-- before parents) and the new ones take their names, which also renames the references.

CREATE TABLE libraries (
    id TEXT PRIMARY KEY NOT NULL,
    -- Empty means "never named": the app shows its own default name, in the user's language.
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
INSERT INTO libraries (id, name) VALUES ('00000000-0000-7000-8000-000000000001', '');

-- No foreign key to libraries: a change applies even before its library row arrives.

CREATE TABLE works_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('book')),
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO works_new (library_id, id, kind, title, created_at)
SELECT '00000000-0000-7000-8000-000000000001', id, kind, title, created_at FROM works;

CREATE TABLE people_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO people_new (library_id, id, name)
SELECT '00000000-0000-7000-8000-000000000001', id, name FROM people;

CREATE TABLE publishers_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO publishers_new (library_id, id, name)
SELECT '00000000-0000-7000-8000-000000000001', id, name FROM publishers;

CREATE TABLE members_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO members_new (library_id, id, name)
SELECT '00000000-0000-7000-8000-000000000001', id, name FROM members;

CREATE TABLE work_contributors_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('author')),
    position INTEGER NOT NULL,
    PRIMARY KEY (library_id, id),
    UNIQUE (library_id, work_id, person_id, role),
    FOREIGN KEY (library_id, work_id) REFERENCES works_new (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, person_id) REFERENCES people_new (library_id, id)
) STRICT;
INSERT INTO work_contributors_new (library_id, id, work_id, person_id, role, position)
SELECT '00000000-0000-7000-8000-000000000001', id, work_id, person_id, role, position
FROM work_contributors;

CREATE TABLE editions_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    title TEXT NOT NULL,
    subtitle TEXT,
    publisher_id TEXT,
    published_year INTEGER,
    language TEXT,
    cover TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, work_id) REFERENCES works_new (library_id, id),
    FOREIGN KEY (library_id, publisher_id) REFERENCES publishers_new (library_id, id)
) STRICT;
INSERT INTO editions_new
    (library_id, id, work_id, title, subtitle, publisher_id, published_year, language, cover, created_at)
SELECT '00000000-0000-7000-8000-000000000001', id, work_id, title, subtitle, publisher_id,
       published_year, language, cover, created_at
FROM editions;

CREATE TABLE book_details_new (
    library_id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    isbn13 TEXT CHECK (isbn13 IS NULL OR length(isbn13) = 13),
    google_books_id TEXT,
    open_library_id TEXT,
    PRIMARY KEY (library_id, edition_id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions_new (library_id, id) ON DELETE CASCADE
) STRICT;
INSERT INTO book_details_new (library_id, edition_id, isbn13, google_books_id, open_library_id)
SELECT '00000000-0000-7000-8000-000000000001', edition_id, isbn13, google_books_id, open_library_id
FROM book_details;

CREATE TABLE copies_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    owner_id TEXT,
    format TEXT NOT NULL
        CHECK (format IN ('hardcover', 'paperback', 'pocket', 'ebook', 'audiobook', 'other')),
    quantity INTEGER NOT NULL DEFAULT 1 CHECK (quantity >= 1),
    location TEXT,
    acquired_on TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions_new (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, owner_id) REFERENCES members_new (library_id, id)
) STRICT;
INSERT INTO copies_new
    (library_id, id, edition_id, owner_id, format, quantity, location, acquired_on, created_at)
SELECT '00000000-0000-7000-8000-000000000001', id, edition_id, owner_id, format, quantity,
       location, acquired_on, created_at
FROM copies;

CREATE TABLE wishes_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    title TEXT NOT NULL,
    author TEXT,
    isbn13 TEXT CHECK (isbn13 IS NULL OR length(isbn13) = 13),
    note TEXT,
    cover_url TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO wishes_new (library_id, id, title, author, isbn13, note, cover_url, created_at)
SELECT '00000000-0000-7000-8000-000000000001', id, title, author, isbn13, note, cover_url, created_at
FROM wishes;

DROP TABLE copies;
DROP TABLE book_details;
DROP TABLE editions;
DROP TABLE work_contributors;
DROP TABLE works;
DROP TABLE people;
DROP TABLE publishers;
DROP TABLE members;
DROP TABLE wishes;

ALTER TABLE works_new RENAME TO works;
ALTER TABLE people_new RENAME TO people;
ALTER TABLE publishers_new RENAME TO publishers;
ALTER TABLE members_new RENAME TO members;
ALTER TABLE work_contributors_new RENAME TO work_contributors;
ALTER TABLE editions_new RENAME TO editions;
ALTER TABLE book_details_new RENAME TO book_details;
ALTER TABLE copies_new RENAME TO copies;
ALTER TABLE wishes_new RENAME TO wishes;

CREATE INDEX people_name ON people (library_id, name COLLATE NOCASE);
CREATE INDEX publishers_name ON publishers (library_id, name COLLATE NOCASE);
CREATE INDEX members_name ON members (library_id, name COLLATE NOCASE);
CREATE INDEX work_contributors_work ON work_contributors (library_id, work_id);
CREATE INDEX editions_work ON editions (library_id, work_id);
CREATE INDEX editions_publisher ON editions (library_id, publisher_id);
CREATE INDEX book_details_isbn ON book_details (isbn13);
CREATE INDEX copies_edition ON copies (library_id, edition_id);
CREATE INDEX wishes_isbn ON wishes (isbn13);

-- Versions and tombstones are per library too: nothing done in one touches another.
CREATE TABLE field_versions_new (
    library_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    field TEXT NOT NULL,
    hlc TEXT NOT NULL,
    PRIMARY KEY (library_id, entity, entity_id, field)
) STRICT, WITHOUT ROWID;
INSERT INTO field_versions_new (library_id, entity, entity_id, field, hlc)
SELECT '00000000-0000-7000-8000-000000000001', entity, entity_id, field, hlc FROM field_versions;
DROP TABLE field_versions;
ALTER TABLE field_versions_new RENAME TO field_versions;

CREATE TABLE tombstones_new (
    library_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    hlc TEXT NOT NULL,
    PRIMARY KEY (library_id, entity, entity_id)
) STRICT, WITHOUT ROWID;
INSERT INTO tombstones_new (library_id, entity, entity_id, hlc)
SELECT '00000000-0000-7000-8000-000000000001', entity, entity_id, hlc FROM tombstones;
DROP TABLE tombstones;
ALTER TABLE tombstones_new RENAME TO tombstones;

-- Operations from before libraries have another format: they stay as history (D31),
-- and a new baseline per library makes the data travel again.
ALTER TABLE operations ADD COLUMN library_id TEXT NOT NULL
    DEFAULT '00000000-0000-7000-8000-000000000001';
UPDATE operations SET replicable = 0;
CREATE INDEX operations_library ON operations (library_id, device_id, hlc);
DELETE FROM local_state WHERE key = 'sync_baseline';

-- The search index is rebuilt on startup with its library.
DROP TABLE search_index;
CREATE VIRTUAL TABLE search_index USING fts5(
    library_id UNINDEXED,
    edition_id UNINDEXED,
    content,
    tokenize = 'unicode61 remove_diacritics 2'
);
DELETE FROM local_state WHERE key = 'search_index_version';

-- Which libraries go to each paired device (D46). Local to this device, never synced.
-- Existing pairings keep sharing the migrated library, as they shared everything before.
CREATE TABLE library_peers (
    library_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    PRIMARY KEY (library_id, device_id)
) STRICT, WITHOUT ROWID;
INSERT OR IGNORE INTO library_peers (library_id, device_id)
SELECT '00000000-0000-7000-8000-000000000001', device_id FROM known_servers
UNION SELECT '00000000-0000-7000-8000-000000000001', device_id FROM known_clients;
