-- Per-device values: device id and last HLC issued.
CREATE TABLE local_state (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
) STRICT;

-- Append-only log of every write (D05, D25).
CREATE TABLE operations (
    id TEXT PRIMARY KEY NOT NULL,
    hlc TEXT NOT NULL UNIQUE,
    device_id TEXT NOT NULL,
    intent TEXT NOT NULL,
    changes TEXT NOT NULL CHECK (json_valid(changes)),
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- Operations address every row by a single id, so contributors need their own.
-- Rows from before this migration get a random (v4) UUID.
CREATE TABLE work_contributors_new (
    id TEXT PRIMARY KEY NOT NULL,
    work_id TEXT NOT NULL REFERENCES works (id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES people (id),
    role TEXT NOT NULL CHECK (role IN ('author')),
    position INTEGER NOT NULL,
    UNIQUE (work_id, person_id, role)
) STRICT;

INSERT INTO work_contributors_new (id, work_id, person_id, role, position)
SELECT lower(
           hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' ||
           substr(hex(randomblob(2)), 2) || '-' ||
           substr('89ab', 1 + abs(random()) % 4, 1) || substr(hex(randomblob(2)), 2) || '-' ||
           hex(randomblob(6))
       ),
       work_id, person_id, role, position
FROM work_contributors;

DROP TABLE work_contributors;
ALTER TABLE work_contributors_new RENAME TO work_contributors;
CREATE INDEX work_contributors_work ON work_contributors (work_id);
