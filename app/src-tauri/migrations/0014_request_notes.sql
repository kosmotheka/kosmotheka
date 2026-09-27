-- Requests grow a note and can be withdrawn (D73, milestone 5.7). The note row
-- travels inside the request and is never applied, but its status is optional in
-- the schema, so the column allows null like any optional field.
CREATE TABLE change_requests_new (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    status TEXT CHECK (status IN ('accepted', 'rejected', 'withdrawn')),
    note TEXT,
    PRIMARY KEY (library_id, id)
) STRICT;
INSERT INTO change_requests_new (library_id, id, status)
SELECT library_id, id, status FROM change_requests;
DROP TABLE change_requests;
ALTER TABLE change_requests_new RENAME TO change_requests;

-- This device's own operations that were applied here and later, on recalculating
-- the library, turned out to be requests: shown once, then dismissed. Local only.
CREATE TABLE request_notices (
    library_id TEXT NOT NULL,
    operation_id TEXT NOT NULL,
    PRIMARY KEY (library_id, operation_id)
) STRICT;
