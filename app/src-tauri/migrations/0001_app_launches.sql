CREATE TABLE app_launches (
    id INTEGER PRIMARY KEY,
    launched_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;
