-- A delete wins (D89): what depends on a deleted row goes with it (required
-- reference, ON DELETE CASCADE) or loses just that reference (optional one).
--
-- `editions.work_id` and `work_contributors.person_id` had no rule, so the
-- tables are rebuilt (SQLite cannot change a foreign key in place), with foreign
-- keys off (see db::migrate): dropping `editions` with them on would delete its
-- copies and details along the way.
--
-- The optional references (`editions.publisher_id`, `copies.owner_id`,
-- `devices.member_id`) cannot use ON DELETE SET NULL: the foreign key spans
-- (library_id, id) and SET NULL would also clear `library_id`, which is NOT NULL.
-- A trigger clears only the reference, before the parent goes.

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
    barcode TEXT,
    notes TEXT,
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, publisher_id) REFERENCES publishers (library_id, id)
) STRICT;
INSERT INTO editions_new
    (library_id, id, work_id, title, subtitle, publisher_id, published_year, language,
     cover, created_at, barcode, notes)
SELECT library_id, id, work_id, title, subtitle, publisher_id, published_year, language,
       cover, created_at, barcode, notes
FROM editions;
DROP TABLE editions;
ALTER TABLE editions_new RENAME TO editions;
CREATE INDEX editions_work ON editions (library_id, work_id);
CREATE INDEX editions_publisher ON editions (library_id, publisher_id);
CREATE INDEX editions_barcode ON editions (library_id, barcode);

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
    FOREIGN KEY (library_id, person_id) REFERENCES people (library_id, id) ON DELETE CASCADE
) STRICT;
INSERT INTO work_contributors_new (library_id, id, work_id, person_id, role, position)
SELECT library_id, id, work_id, person_id, role, position FROM work_contributors;
DROP TABLE work_contributors;
ALTER TABLE work_contributors_new RENAME TO work_contributors;
CREATE INDEX work_contributors_work ON work_contributors (library_id, work_id);

CREATE TRIGGER publishers_delete_clears_editions
BEFORE DELETE ON publishers
BEGIN
    UPDATE editions SET publisher_id = NULL
     WHERE library_id = OLD.library_id AND publisher_id = OLD.id;
END;

CREATE TRIGGER members_delete_clears_copies
BEFORE DELETE ON members
BEGIN
    UPDATE copies SET owner_id = NULL
     WHERE library_id = OLD.library_id AND owner_id = OLD.id;
    UPDATE devices SET member_id = NULL
     WHERE library_id = OLD.library_id AND member_id = OLD.id;
END;
