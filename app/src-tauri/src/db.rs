use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension};
use rusqlite_migration::{M, Migrations};

use crate::error::{Error, Result};
use crate::{ops, search, sync};

const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up(include_str!("../migrations/0001_app_launches.sql")),
    M::up(include_str!("../migrations/0002_catalog.sql")),
    M::up(include_str!("../migrations/0003_operations.sql")),
    M::up(include_str!(
        "../migrations/0004_covers_and_external_ids.sql"
    )),
    M::up(include_str!("../migrations/0005_publishers_and_search.sql")),
    M::up(include_str!("../migrations/0006_sync.sql")),
    M::up(include_str!("../migrations/0007_paired_devices.sql")),
    M::up(include_str!("../migrations/0008_wishes.sql")),
    M::up(include_str!("../migrations/0009_libraries.sql")),
    M::up(include_str!("../migrations/0010_revocations.sql")),
    M::up(include_str!("../migrations/0011_signatures.sql")),
    M::up(include_str!("../migrations/0012_roles.sql")),
    M::up(include_str!("../migrations/0013_change_requests.sql")),
    M::up(include_str!("../migrations/0014_request_notes.sql")),
    M::up(include_str!("../migrations/0015_pending_owners.sql")),
    M::up(include_str!("../migrations/0016_marks.sql")),
    M::up(include_str!("../migrations/0017_kinds.sql")),
    M::up(include_str!("../migrations/0018_edition_barcode.sql")),
    M::up(include_str!("../migrations/0019_kind_details.sql")),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

/// Cheap to clone: every clone shares the same connection (the UI commands and the
/// LAN sync server use it from different tasks).
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: PathBuf) -> Result<Self> {
        let mut conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        copy_before_migrating(&conn, &path)?;
        migrate(&mut conn)?;
        ops::ensure_device_id(&conn)?;
        search::ensure_current(&conn)?;
        sync::ensure_baseline(&mut conn)?;
        Ok(Self::from_connection(conn))
    }

    fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::from_connection(open_in_memory().unwrap())
    }

    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| Error::LockPoisoned)
    }
}

/// Before migrations touch an existing database, a copy of it is left next to it
/// (`library.sqlite3.v8.bak`), so a failed update can be undone by putting it back.
fn copy_before_migrating(conn: &Connection, path: &Path) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current == 0 || current >= MIGRATIONS_SLICE.len() as i64 {
        return Ok(());
    }
    let mut copy = path.as_os_str().to_owned();
    copy.push(format!(".v{current}.bak"));
    let copy = PathBuf::from(copy);
    if !copy.exists() {
        // VACUUM INTO writes a consistent copy even in WAL mode, unlike copying the file.
        conn.execute("VACUUM INTO ?1", [copy.to_string_lossy().as_ref()])?;
    }
    Ok(())
}

/// Migrations run with foreign keys off, so a table can be rebuilt without its
/// dependents being deleted by the cascade (SQLite's own advice); afterwards every
/// key is checked and they are turned on again.
fn migrate(conn: &mut Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", false)?;
    MIGRATIONS.to_latest(conn)?;
    let broken = conn
        .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
        .optional()?;
    if broken.is_some() {
        return Err(Error::CorruptState("a migration broke a foreign key"));
    }
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    migrate(&mut conn)?;
    ops::ensure_device_id(&conn)?;
    search::ensure_current(&conn)?;
    sync::ensure_baseline(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(MIGRATIONS.validate().is_ok());
    }

    #[test]
    fn every_migration_file_is_registered() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/migrations");
        let files = std::fs::read_dir(dir).unwrap().count();
        assert_eq!(
            files,
            MIGRATIONS_SLICE.len(),
            "add new files in migrations/ to MIGRATIONS_SLICE"
        );
    }

    #[test]
    fn migration_5_turns_publisher_text_into_a_catalogue() {
        let mut conn = Connection::open_in_memory().unwrap();
        MIGRATIONS.to_version(&mut conn, 4).unwrap();
        conn.execute_batch(
            "INSERT INTO works (id, kind, title) VALUES ('w', 'book', 'T');
             INSERT INTO editions (id, work_id, title, publisher) VALUES
                 ('e1', 'w', 'T', 'Minotauro'), ('e2', 'w', 'T', 'MINOTAURO'), ('e3', 'w', 'T', NULL);",
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        let publishers: i64 = conn
            .query_row("SELECT count(*) FROM publishers", [], |r| r.get(0))
            .unwrap();
        let linked: i64 = conn
            .query_row(
                "SELECT count(*) FROM editions WHERE publisher_id IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(publishers, 1);
        assert_eq!(linked, 2);
    }

    #[test]
    fn migration_6_keeps_data_and_archives_old_history() {
        let mut conn = Connection::open_in_memory().unwrap();
        MIGRATIONS.to_version(&mut conn, 5).unwrap();
        conn.execute_batch(
            "INSERT INTO works (id, kind, title) VALUES ('w', 'book', 'T');
             INSERT INTO editions (id, work_id, title) VALUES ('e', 'w', 'T');
             INSERT INTO book_details (edition_id, isbn13, open_library_id) VALUES ('e', '9780306406157', 'OL1M');
             INSERT INTO operations (id, hlc, device_id, intent, changes) VALUES ('o', 'h', 'd', 'book.add', '[]');",
        )
        .unwrap();

        MIGRATIONS.to_version(&mut conn, 6).unwrap();

        let (isbn, ol): (String, String) = conn
            .query_row(
                "SELECT isbn13, open_library_id FROM book_details",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((isbn.as_str(), ol.as_str()), ("9780306406157", "OL1M"));
        let replicable: i64 = conn
            .query_row("SELECT replicable FROM operations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(replicable, 0, "history from before sync is not replicated");
        // The ISBN is no longer unique: the same book may arrive from another device (D33).
        conn.execute_batch(
            "INSERT INTO editions (id, work_id, title) VALUES ('e2', 'w', 'T');
             INSERT INTO book_details (edition_id, isbn13) VALUES ('e2', '9780306406157');",
        )
        .unwrap();
    }

    #[test]
    fn migration_9_moves_everything_into_the_default_library() {
        let mut conn = Connection::open_in_memory().unwrap();
        MIGRATIONS.to_version(&mut conn, 8).unwrap();
        conn.execute_batch(
            "INSERT INTO works (id, kind, title) VALUES ('w', 'book', 'T');
             INSERT INTO editions (id, work_id, title) VALUES ('e', 'w', 'T');
             INSERT INTO members (id, name) VALUES ('m', 'Ana');
             INSERT INTO copies (id, edition_id, owner_id, format) VALUES ('c', 'e', 'm', 'paperback');
             INSERT INTO field_versions (entity, entity_id, field, hlc) VALUES ('works', 'w', 'title', 'h1');
             INSERT INTO tombstones (entity, entity_id, hlc) VALUES ('copies', 'gone', 'h2');
             INSERT INTO operations (id, hlc, device_id, intent, changes, replicable)
             VALUES ('o', 'h1', 'd', 'book.add', '[]', 1);
             INSERT INTO known_servers (device_id, name, addresses, fingerprint, secret)
             VALUES ('phone', 'Phone', '[]', 'f', 's');",
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        let library = crate::ops::DEFAULT_LIBRARY_ID;
        let in_library = |table: &str| -> i64 {
            conn.query_row(
                &format!("SELECT count(*) FROM {table} WHERE library_id = ?1"),
                [library],
                |r| r.get(0),
            )
            .unwrap()
        };
        for table in [
            "works",
            "editions",
            "members",
            "copies",
            "field_versions",
            "tombstones",
        ] {
            assert_eq!(in_library(table), 1, "{table}");
        }
        let replicable: i64 = conn
            .query_row("SELECT replicable FROM operations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            replicable, 0,
            "operations in the old format stay as history"
        );
        let shared: String = conn
            .query_row(
                "SELECT library_id FROM library_peers WHERE device_id = 'phone'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            shared, library,
            "existing pairings keep sharing the migrated library"
        );
    }

    #[test]
    fn migration_17_keeps_everything_that_hangs_from_a_work() {
        let mut conn = Connection::open_in_memory().unwrap();
        MIGRATIONS.to_version(&mut conn, 16).unwrap();
        conn.execute_batch(
            "INSERT INTO works (library_id, id, kind, title) VALUES ('l', 'w', 'book', 'Dune');
             INSERT INTO people (library_id, id, name) VALUES ('l', 'p', 'Herbert');
             INSERT INTO members (library_id, id, name) VALUES ('l', 'm', 'Ana');
             INSERT INTO work_contributors (library_id, id, work_id, person_id, role, position)
                 VALUES ('l', 'c', 'w', 'p', 'author', 0);
             INSERT INTO editions (library_id, id, work_id, title) VALUES ('l', 'e', 'w', 'Dune');
             INSERT INTO copies (library_id, id, edition_id, format) VALUES ('l', 'k', 'e', 'ebook');
             INSERT INTO marks (library_id, id, member_id, work_id, status)
                 VALUES ('l', 'x', 'm', 'w', 'finished');",
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        for table in ["works", "work_contributors", "editions", "copies", "marks"] {
            let rows: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(rows, 1, "{table} lost its row");
        }
        conn.execute(
            "INSERT INTO works (library_id, id, kind, title) VALUES ('l', 'g', 'boardgame', 'Catan')",
            [],
        )
        .unwrap();
    }

    #[test]
    fn a_copy_is_kept_before_migrating_an_existing_database() {
        let dir = std::env::temp_dir().join(format!("library-db-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("library.sqlite3");
        {
            let mut conn = Connection::open(&path).unwrap();
            MIGRATIONS.to_version(&mut conn, 8).unwrap();
            conn.execute(
                "INSERT INTO members (id, name) VALUES ('01997c7e-0000-7000-8000-000000000001', 'Ana')",
                [],
            )
                .unwrap();
        }

        Database::open(path.clone()).unwrap();

        let copy = Connection::open(dir.join("library.sqlite3.v8.bak")).unwrap();
        let (version, members): (i64, i64) = copy
            .query_row(
                "SELECT (SELECT user_version FROM pragma_user_version), count(*) FROM members",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((version, members), (8, 1));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
