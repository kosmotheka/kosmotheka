-- Genres (a closed list per kind, spec/schema.rs is the one place that lists
-- values) and tags (free words a library creates for itself), both linked to
-- works. Link rows have no UNIQUE constraint on purpose: two devices may add the
-- same genre or tag to a work offline, and both rows must survive so the devices
-- converge (readers deduplicate; removing one deletes every row of that pair).
CREATE TABLE tags (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    PRIMARY KEY (library_id, id)
) STRICT;

CREATE TABLE work_tags (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, tag_id) REFERENCES tags (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE TABLE work_genres (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    genre TEXT NOT NULL,
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE INDEX work_tags_work ON work_tags (library_id, work_id);
CREATE INDEX work_tags_tag ON work_tags (library_id, tag_id);
CREATE INDEX work_genres_work ON work_genres (library_id, work_id);
CREATE INDEX tags_name ON tags (library_id, name COLLATE NOCASE);
