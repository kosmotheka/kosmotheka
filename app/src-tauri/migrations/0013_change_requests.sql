-- How administrators resolved requests (D73). The row's id is the id of the
-- requested operation, which stays in the log with status 'request'.
CREATE TABLE change_requests (
    library_id TEXT NOT NULL,
    id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('accepted', 'rejected')),
    PRIMARY KEY (library_id, id)
) STRICT;
