-- Phase 0 placeholder, replaced by the real catalog.
DROP TABLE app_launches;

CREATE TABLE works (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('book')),
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE people (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX people_name ON people (name COLLATE NOCASE);

CREATE TABLE work_contributors (
    work_id TEXT NOT NULL REFERENCES works (id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people (id),
    role TEXT NOT NULL CHECK (role IN ('author')),
    position INTEGER NOT NULL,
    PRIMARY KEY (work_id, person_id, role)
) STRICT;

CREATE TABLE editions (
    id TEXT PRIMARY KEY NOT NULL,
    work_id TEXT NOT NULL REFERENCES works (id),
    title TEXT NOT NULL,
    subtitle TEXT,
    publisher TEXT,
    published_year INTEGER,
    language TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
CREATE INDEX editions_work ON editions (work_id);

CREATE TABLE book_details (
    edition_id TEXT PRIMARY KEY NOT NULL REFERENCES editions (id) ON DELETE CASCADE,
    isbn13 TEXT UNIQUE CHECK (isbn13 IS NULL OR length(isbn13) = 13)
) STRICT;

CREATE TABLE members (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX members_name ON members (name COLLATE NOCASE);

CREATE TABLE copies (
    id TEXT PRIMARY KEY NOT NULL,
    edition_id TEXT NOT NULL REFERENCES editions (id) ON DELETE CASCADE,
    owner_id TEXT REFERENCES members (id),
    format TEXT NOT NULL
        CHECK (format IN ('hardcover', 'paperback', 'pocket', 'ebook', 'audiobook', 'other')),
    quantity INTEGER NOT NULL DEFAULT 1 CHECK (quantity >= 1),
    location TEXT,
    acquired_on TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
CREATE INDEX copies_edition ON copies (edition_id);
