-- What is particular to each kind (D08): one row per edition in the table of its
-- kind, like book_details. Music and "other" have nothing of their own: the artist
-- is a contributor and the label is the publisher.
CREATE TABLE comic_details (
    library_id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    number TEXT,
    PRIMARY KEY (library_id, edition_id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE TABLE boardgame_details (
    library_id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    players_min INTEGER,
    players_max INTEGER,
    duration_minutes INTEGER,
    min_age INTEGER,
    PRIMARY KEY (library_id, edition_id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE TABLE film_details (
    library_id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    duration_minutes INTEGER,
    PRIMARY KEY (library_id, edition_id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE TABLE tv_series_details (
    library_id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    seasons INTEGER,
    PRIMARY KEY (library_id, edition_id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions (library_id, id) ON DELETE CASCADE
) STRICT;

CREATE TABLE videogame_details (
    library_id TEXT NOT NULL,
    edition_id TEXT NOT NULL,
    platform TEXT,
    PRIMARY KEY (library_id, edition_id),
    FOREIGN KEY (library_id, edition_id) REFERENCES editions (library_id, id) ON DELETE CASCADE
) STRICT;
