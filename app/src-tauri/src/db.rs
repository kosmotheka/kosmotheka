use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::{Error, Result};
use crate::{libraries, ops, search, sync};

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
    M::up(include_str!("../migrations/0017_kinds.sql")).foreign_key_check(),
    M::up(include_str!("../migrations/0018_edition_barcode.sql")),
    M::up(include_str!("../migrations/0019_kind_details.sql")),
    M::up(include_str!("../migrations/0020_mark_wished.sql")),
    M::up(include_str!("../migrations/0021_genres_and_tags.sql")),
    M::up(include_str!("../migrations/0022_series.sql")),
    M::up(include_str!("../migrations/0023_copy_platform.sql")).foreign_key_check(),
    M::up(include_str!("../migrations/0024_edition_notes.sql")),
    M::up(include_str!("../migrations/0025_drop_wishes.sql")),
    M::up(include_str!("../migrations/0026_device_fences.sql")),
    M::up(include_str!("../migrations/0027_library_cloud.sql")),
    M::up(include_str!("../migrations/0028_forgotten_libraries.sql")),
    M::up(include_str!("../migrations/0029_operation_chain.sql")),
    M::up(include_str!("../migrations/0030_delete_wins.sql")).foreign_key_check(),
    M::up(include_str!("../migrations/0031_seals.sql")),
    M::up(include_str!("../migrations/0032_cloud_power.sql")),
    M::up(include_str!("../migrations/0033_vouched_key.sql")),
    M::up(include_str!("../migrations/0034_access_withdrawn.sql")),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

/// Cheap to clone: every clone shares the same connection (the UI commands and the
/// LAN sync server use it from different tasks).
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    /// `None` for an in-memory database (tests): there is no file to reset.
    path: Option<PathBuf>,
}

impl Database {
    pub fn open(path: PathBuf) -> Result<Self> {
        Self::open_as(path, None)
    }

    /// Opens with a chosen identity (device id and signing seed) already in place, so
    /// the first library is created and signed as that device. For fixtures only
    /// (`examples/fixture_devices.rs`).
    pub fn open_as(path: PathBuf, identity: Option<(&str, [u8; 32])>) -> Result<Self> {
        let conn = open_file(&path, identity)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path),
        })
    }

    #[cfg(test)]
    fn from_connection(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        }
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::from_connection(open_in_memory().unwrap())
    }

    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| Error::LockPoisoned)
    }

    /// Leaves the database as if just installed (D81, device reset): a fresh file
    /// (new device id, new signing key, no libraries), and the old one, its `-wal`
    /// and `-shm` siblings and any pre-migration `.bak` copy deleted, since they
    /// hold the previous owner's data. An in-memory database is simply replaced.
    pub fn reset(&self) -> Result<()> {
        let mut guard = self.conn.lock().map_err(|_| Error::LockPoisoned)?;
        // Drop the file connection first: on every OS, a file can only be deleted
        // once nothing has it open.
        *guard = Connection::open_in_memory()?;
        let fresh = match &self.path {
            Some(path) => {
                delete_if_exists(path)?;
                delete_if_exists(&wal_path(path))?;
                delete_if_exists(&shm_path(path))?;
                delete_bak_siblings(path)?;
                open_file(path, None)?
            }
            None => open_in_memory()?,
        };
        *guard = fresh;
        Ok(())
    }
}

fn wal_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push("-wal");
    PathBuf::from(name)
}

fn shm_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push("-shm");
    PathBuf::from(name)
}

fn delete_if_exists(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

/// Removes every `.bak` copy `copy_before_migrating` left next to `path`.
fn delete_bak_siblings(path: &Path) -> Result<()> {
    let Some(dir) = path.parent() else {
        return Ok(());
    };
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return Ok(());
    };
    let prefix = format!("{file_name}.");
    for entry in std::fs::read_dir(dir)?.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let is_bak = Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("bak"));
        if name.starts_with(&prefix) && is_bak {
            delete_if_exists(&entry.path())?;
        }
    }
    Ok(())
}

/// Opens (creating if needed) and fully initialises a file-backed database: the
/// same steps `Database::open` and `Database::reset` both need.
fn open_file(path: &Path, identity: Option<(&str, [u8; 32])>) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    copy_before_migrating(&conn, path)?;
    migrate(&mut conn)?;
    if let Some((device_id, seed)) = identity {
        ops::set_identity(&conn, device_id, seed)?;
    }
    ops::ensure_device_id(&conn)?;
    search::ensure_current(&conn)?;
    sync::ensure_baseline(&mut conn)?;
    first_libraries(&mut conn)?;
    Ok(conn)
}

/// After the baseline, so a database from before it logs its old library first and
/// the retire step takes that log away with the library (D85).
fn first_libraries(conn: &mut Connection) -> Result<()> {
    libraries::retire_fixed_library(conn)?;
    libraries::ensure_first_library(conn)
}

/// Before migrations touch an existing database, a copy of it is left next to it
/// (`library.sqlite3.v8.bak`), so a failed update can be undone by putting it back.
fn copy_before_migrating(conn: &Connection, path: &Path) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current == 0 || current >= i64::try_from(MIGRATIONS_SLICE.len()).unwrap_or(i64::MAX) {
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
/// dependents being deleted by the cascade (SQLite's own advice); they are turned on
/// again afterwards. A migration that rebuilds tables checks the keys itself, inside
/// its own transaction, via `foreign_key_check()` (see the 0017 entry above), so a
/// broken reference rolls back that migration instead of being committed.
fn migrate(conn: &mut Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", false)?;
    MIGRATIONS.to_latest(conn)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    Ok(())
}

/// Also used by `Database::reset` for an in-memory database (tests only reach
/// that path, but the function itself is not test-only).
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    migrate(&mut conn)?;
    ops::ensure_device_id(&conn)?;
    search::ensure_current(&conn)?;
    sync::ensure_baseline(&mut conn)?;
    first_libraries(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spec_has_the_same_schema_version_as_the_migrations() {
        // The server reads spec/sync.json to know which devices it may sync with (D31, D84).
        let spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../spec/sync.json")).unwrap();
        let conn = open_in_memory().unwrap();
        assert_eq!(
            spec["schemaVersion"].as_i64(),
            Some(crate::ops::schema_version(&conn).unwrap())
        );
    }

    /// D89: every reference between catalog tables says what happens when the row
    /// it points to is deleted. A required one (NOT NULL) goes with it; an optional
    /// one keeps the NO ACTION rule and a trigger clears it first (SET NULL would
    /// also clear `library_id`).
    #[test]
    fn every_reference_between_catalog_tables_has_a_delete_rule() {
        let conn = open_in_memory().unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut optional = Vec::new();
        for table in tables {
            let mut statement = conn
                .prepare(&format!(
                    "SELECT \"from\", on_delete FROM pragma_foreign_key_list('{table}')
                      WHERE \"from\" <> 'library_id'"
                ))
                .unwrap();
            let keys: Vec<(String, String)> = statement
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            for (column, rule) in keys {
                if rule != "CASCADE" {
                    optional.push((table.clone(), column, rule));
                }
            }
        }
        optional.sort();
        let expected = [("copies", "owner_id"), ("editions", "publisher_id")]
            .map(|(table, column)| (table.to_owned(), column.to_owned(), "NO ACTION".to_owned()));
        assert_eq!(optional, expected);
    }

    #[test]
    fn migration_30_keeps_its_rows_and_makes_a_delete_win() {
        let mut conn = Connection::open_in_memory().unwrap();
        MIGRATIONS.to_version(&mut conn, 29).unwrap();
        conn.execute_batch(
            "INSERT INTO works (library_id, id, kind, title) VALUES ('l', 'w', 'book', 'Dune');
             INSERT INTO people (library_id, id, name) VALUES ('l', 'p', 'Herbert');
             INSERT INTO publishers (library_id, id, name) VALUES ('l', 'pub', 'Minotauro');
             INSERT INTO members (library_id, id, name) VALUES ('l', 'm', 'Ana');
             INSERT INTO work_contributors (library_id, id, work_id, person_id, role, position)
                 VALUES ('l', 'c', 'w', 'p', 'author', 0);
             INSERT INTO editions (library_id, id, work_id, title, publisher_id, barcode, notes)
                 VALUES ('l', 'e', 'w', 'Dune', 'pub', '123', 'a note');
             INSERT INTO book_details (library_id, edition_id, isbn13) VALUES ('l', 'e', '9780441172719');
             INSERT INTO copies (library_id, id, edition_id, owner_id, format)
                 VALUES ('l', 'k', 'e', 'm', 'ebook');
             INSERT INTO search_index (library_id, edition_id, content) VALUES ('l', 'e', 'dune herbert');",
        )
        .unwrap();

        migrate(&mut conn).unwrap();

        for table in [
            "works",
            "people",
            "publishers",
            "members",
            "work_contributors",
            "editions",
            "book_details",
            "copies",
            "search_index",
        ] {
            let rows: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(rows, 1, "{table} lost its row");
        }
        let (barcode, notes): (String, String) = conn
            .query_row("SELECT barcode, notes FROM editions", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((barcode.as_str(), notes.as_str()), ("123", "a note"));
        let broken: i64 = conn
            .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(broken, 0);
        let indexes: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'index'
                  AND name IN ('editions_work', 'editions_publisher', 'editions_barcode',
                               'work_contributors_work')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(indexes, 4);

        // The rules themselves, with foreign keys on as in the running app.
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.execute("DELETE FROM members", []).unwrap();
        conn.execute("DELETE FROM publishers", []).unwrap();
        let (owner, publisher): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT (SELECT owner_id FROM copies), (SELECT publisher_id FROM editions)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((owner, publisher), (None, None));
        conn.execute("DELETE FROM people", []).unwrap();
        let links: i64 = conn
            .query_row("SELECT count(*) FROM work_contributors", [], |r| r.get(0))
            .unwrap();
        assert_eq!(links, 0, "the link goes with the person");
        conn.execute("DELETE FROM works", []).unwrap();
        for table in ["editions", "copies", "book_details"] {
            let rows: i64 = conn
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(rows, 0, "{table} goes with the work");
        }
    }

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

        // The id migration 0009 gives it; D85 retires it at startup, after migrating.
        let library = "00000000-0000-7000-8000-000000000001";
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
    fn migration_17_is_undone_if_it_leaves_a_broken_reference() {
        let mut conn = Connection::open_in_memory().unwrap();
        MIGRATIONS.to_version(&mut conn, 16).unwrap();
        // Keys off, as `migrate` itself would have them mid-migration, so this
        // broken insert is accepted.
        conn.pragma_update(None, "foreign_keys", false).unwrap();
        conn.execute(
            "INSERT INTO copies (library_id, id, edition_id, format) VALUES ('l', 'k', 'missing', 'ebook')",
            [],
        )
        .unwrap();

        assert!(migrate(&mut conn).is_err());

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 16);
    }

    /// A local log written before the `wish` entity was dropped (D61) may still have
    /// an operation that touches it. The migration only drops the `wishes` table; the
    /// operation stays in the log, and opening the database, then rebuilding a
    /// library from its log, must not fail on it.
    #[test]
    fn a_wish_operation_left_in_the_log_does_not_stop_the_database_from_opening() {
        let dir = std::env::temp_dir().join(format!("library-db-wish-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("library.sqlite3");
        {
            let mut conn = Connection::open(&path).unwrap();
            MIGRATIONS.to_version(&mut conn, 24).unwrap();
            conn.execute(
                "INSERT INTO operations (id, hlc, device_id, intent, changes) VALUES (
                    'o1',
                    '0001727222200010-00000000-01997c7e-0000-7000-8000-0000000000fa',
                    '01997c7e-0000-7000-8000-0000000000fa',
                    'wish.add',
                    '[{\"entity\":\"wish\",\"id\":\"w1\",\"action\":\"create\",\"fields\":{\"title\":\"Tehanu\"}}]'
                )",
                [],
            )
            .unwrap();
        }

        let db = Database::open(path.clone()).unwrap();

        let conn = db.lock().unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name = 'wishes'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 0, "the wishes table is dropped");
        let log = sync::library_log(&conn, &ops::active_library(&conn).unwrap()).unwrap();
        assert!(
            log.iter().all(|op| op.id != "o1"),
            "the unparsable operation is left out entirely, not replayed with some changes missing"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
