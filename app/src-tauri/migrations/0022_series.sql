-- Series group works (D61): a name and whether they are ordered (a saga) or not
-- (a product line). A work relation says one work requires another (an expansion
-- requires its base game). Both link rows have no UNIQUE constraint on purpose:
-- two devices may add the same one offline, and both rows must survive for the
-- devices to converge (readers deduplicate; removing one deletes every row of
-- that pair). No CHECK on `relation`: the closed list lives in spec/ only.
CREATE TABLE series (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    ordered INTEGER NOT NULL CHECK (ordered IN (0, 1)),
    PRIMARY KEY (library_id, id)
) STRICT;

CREATE TABLE series_entries (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    series_id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    position INTEGER,
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, series_id) REFERENCES series (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE TABLE work_relations (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    related_work_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, related_work_id) REFERENCES works (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE INDEX series_entries_series ON series_entries (library_id, series_id);
CREATE INDEX series_entries_work ON series_entries (library_id, work_id);
CREATE INDEX work_relations_work ON work_relations (library_id, work_id);
CREATE INDEX work_relations_related_work ON work_relations (library_id, related_work_id);
CREATE INDEX series_name ON series (library_id, name COLLATE NOCASE);
