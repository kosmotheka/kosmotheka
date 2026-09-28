-- Every kind of work (D60): the lists of kinds, contributor roles and formats
-- grow, and SQLite cannot change a CHECK in place. They stay out of the schema:
-- the operation rules (spec/) are the one place that lists them, so adding a
-- value later does not mean rebuilding tables again. Everything already stored
-- stays as it is: books.
--
-- The tables are rebuilt with foreign keys off (see db::migrate); dropping `works`
-- with them on would delete its editions and marks along the way.

CREATE TABLE works_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO works_new (library_id, id, kind, title, created_at)
SELECT library_id, id, kind, title, created_at FROM works;
DROP TABLE works;
ALTER TABLE works_new RENAME TO works;

CREATE TABLE work_contributors_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    person_id TEXT NOT NULL,
    role TEXT NOT NULL,
    position INTEGER NOT NULL,
    PRIMARY KEY (library_id, id),
    UNIQUE (library_id, work_id, person_id, role),
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, person_id) REFERENCES people (library_id, id)
) STRICT;
INSERT INTO work_contributors_new (library_id, id, work_id, person_id, role, position)
SELECT library_id, id, work_id, person_id, role, position FROM work_contributors;
DROP TABLE work_contributors;
ALTER TABLE work_contributors_new RENAME TO work_contributors;
CREATE INDEX work_contributors_work ON work_contributors (library_id, work_id);

CREATE TABLE copies_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    owner_id TEXT,
    format TEXT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1 CHECK (quantity >= 1),
    location TEXT,
    acquired_on TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, owner_id) REFERENCES members (library_id, id)
) STRICT;
INSERT INTO copies_new
    (library_id, id, edition_id, owner_id, format, quantity, location, acquired_on, created_at)
SELECT library_id, id, edition_id, owner_id, format, quantity, location, acquired_on, created_at
FROM copies;
DROP TABLE copies;
ALTER TABLE copies_new RENAME TO copies;
CREATE INDEX copies_edition ON copies (library_id, edition_id);
