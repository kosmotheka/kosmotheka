-- Publishers become a catalogue of their own, like people (D29).
CREATE TABLE publishers (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL
) STRICT;
CREATE UNIQUE INDEX publishers_name ON publishers (name COLLATE NOCASE);

-- One publisher per spelling, ignoring case. Random (v4) UUIDs for existing data.
INSERT INTO publishers (id, name)
SELECT lower(
           hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
           substr(hex(randomblob(2)), 2) || '-' ||
           substr('89ab', 1 + abs(random()) % 4, 1) || substr(hex(randomblob(2)), 2) || '-' ||
           hex(randomblob(6))
       ),
       min(publisher)
FROM editions
WHERE publisher IS NOT NULL
GROUP BY publisher COLLATE NOCASE;

ALTER TABLE editions ADD COLUMN publisher_id TEXT REFERENCES publishers (id);
UPDATE editions
SET publisher_id = (SELECT id FROM publishers p WHERE p.name = editions.publisher COLLATE NOCASE)
WHERE publisher IS NOT NULL;
ALTER TABLE editions DROP COLUMN publisher;
CREATE INDEX editions_publisher ON editions (publisher_id);

-- Full-text search, accent-insensitive. Filled and kept current by the app (search.rs).
CREATE VIRTUAL TABLE search_index USING fts5(
    edition_id UNINDEXED,
    content,
    tokenize = 'unicode61 remove_diacritics 2'
);
