//! Per-device settings, kept in `local_state`: never synced, exported or committed.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ops::{read_state, write_state};

const GOOGLE_BOOKS_API_KEY: &str = "google_books_api_key";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub google_books_api_key: Option<String>,
}

pub fn load(conn: &Connection) -> Result<Settings> {
    Ok(Settings {
        google_books_api_key: read_state(conn, GOOGLE_BOOKS_API_KEY)?,
    })
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<()> {
    let key = settings
        .google_books_api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty());
    write_state(conn, GOOGLE_BOOKS_API_KEY, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn saves_trims_and_clears() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(load(&conn).unwrap(), Settings::default());

        save(
            &conn,
            &Settings {
                google_books_api_key: Some("  abc ".into()),
            },
        )
        .unwrap();
        assert_eq!(
            load(&conn).unwrap().google_books_api_key.as_deref(),
            Some("abc")
        );

        save(
            &conn,
            &Settings {
                google_books_api_key: Some("".into()),
            },
        )
        .unwrap();
        assert_eq!(load(&conn).unwrap().google_books_api_key, None);
    }
}
