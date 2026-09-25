use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;
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

fn migrate(conn: &mut Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    MIGRATIONS.to_latest(conn)?;
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

        MIGRATIONS.to_latest(&mut conn).unwrap();

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

        MIGRATIONS.to_latest(&mut conn).unwrap();

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
}
