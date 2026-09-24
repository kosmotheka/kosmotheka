use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::{Error, Result};
use crate::{ops, search};

const MIGRATIONS_SLICE: &[M<'_>] = &[
    M::up(include_str!("../migrations/0001_app_launches.sql")),
    M::up(include_str!("../migrations/0002_catalog.sql")),
    M::up(include_str!("../migrations/0003_operations.sql")),
    M::up(include_str!(
        "../migrations/0004_covers_and_external_ids.sql"
    )),
    M::up(include_str!("../migrations/0005_publishers_and_search.sql")),
];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: PathBuf) -> Result<Self> {
        let mut conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&mut conn)?;
        ops::ensure_device_id(&conn)?;
        search::ensure_current(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
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
}
