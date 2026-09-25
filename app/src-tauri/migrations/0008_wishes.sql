-- Basic wishlist (D36), synced like the rest of the catalogue.
CREATE TABLE wishes (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    author TEXT,
    isbn13 TEXT CHECK (isbn13 IS NULL OR length(isbn13) = 13),
    note TEXT,
    cover_url TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
CREATE INDEX wishes_isbn ON wishes (isbn13);
