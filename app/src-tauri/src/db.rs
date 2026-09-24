use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};
use serde::Serialize;

use crate::error::{Error, Result};

const MIGRATIONS_SLICE: &[M<'_>] = &[M::up(include_str!("../migrations/0001_app_launches.sql"))];
const MIGRATIONS: Migrations<'_> = Migrations::from_slice(MIGRATIONS_SLICE);

pub struct Database {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn open(path: PathBuf) -> Result<Self> {
        let mut conn = Connection::open(&path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path,
        })
    }

    pub fn lock(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| Error::LockPoisoned)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", true)?;
    MIGRATIONS.to_latest(conn)?;
    Ok(())
}

pub fn record_launch(conn: &Connection) -> Result<()> {
    conn.execute("INSERT INTO app_launches DEFAULT VALUES", [])?;
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub launch_count: i64,
    pub first_launch: Option<String>,
    pub sqlite_version: &'static str,
    pub database_path: String,
}

pub fn app_info(conn: &Connection, path: &Path) -> Result<AppInfo> {
    let (launch_count, first_launch) = conn.query_row(
        "SELECT COUNT(*), MIN(launched_at) FROM app_launches",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(AppInfo {
        launch_count,
        first_launch,
        sqlite_version: rusqlite::version(),
        database_path: path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        assert!(MIGRATIONS.validate().is_ok());
    }

    #[test]
    fn counts_launches() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        record_launch(&conn).unwrap();
        record_launch(&conn).unwrap();

        let info = app_info(&conn, Path::new(":memory:")).unwrap();

        assert_eq!(info.launch_count, 2);
        assert!(info.first_launch.is_some());
    }
}
