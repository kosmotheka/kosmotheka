-- What each person notes about each work (D68): status with its date, rating and
-- favorite. One row per person and work; its id is derived from both. It goes
-- with its work or its person when either is deleted.
CREATE TABLE marks (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    member_id TEXT NOT NULL,
    work_id TEXT NOT NULL,
    status TEXT CHECK (status IS NULL OR status IN ('pending', 'in_progress', 'finished', 'abandoned')),
    status_on TEXT,
    rating INTEGER CHECK (rating IS NULL OR rating BETWEEN 1 AND 5),
    favorite INTEGER CHECK (favorite IS NULL OR favorite IN (0, 1)),
    PRIMARY KEY (library_id, id),
    FOREIGN KEY (library_id, work_id) REFERENCES works (library_id, id) ON DELETE CASCADE,
    FOREIGN KEY (library_id, member_id) REFERENCES members (library_id, id) ON DELETE CASCADE
) STRICT;
CREATE INDEX marks_work ON marks (library_id, work_id);
CREATE INDEX marks_member ON marks (library_id, member_id);
