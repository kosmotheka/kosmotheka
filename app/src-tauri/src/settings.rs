//! Per-device settings, kept in `local_state`: never synced, exported or committed.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::ops::{read_state, write_state};
use kosmotheka_domain::{ErrorCode, FieldError};

/// The old, single-provider key. Read for migration, never written again.
const LEGACY_GOOGLE_BOOKS_API_KEY: &str = "google_books_api_key";
const DISABLED_SOURCES_KEY: &str = "disabled_sources";

/// Provider ids that need a key (see `metadata::Source` for every source,
/// keyless ones included).
pub const KEYED_PROVIDERS: &[&str] = &["google_books", "gameupc"];

fn api_key_state_key(provider: &str) -> String {
    format!("api_key.{provider}")
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub api_keys: BTreeMap<String, String>,
    pub disabled_sources: BTreeSet<String>,
}

pub fn load(conn: &Connection) -> Result<Settings> {
    let mut api_keys = BTreeMap::new();
    for &provider in KEYED_PROVIDERS {
        let key = match read_state(conn, &api_key_state_key(provider))? {
            Some(key) => Some(key),
            None if provider == "google_books" => read_state(conn, LEGACY_GOOGLE_BOOKS_API_KEY)?,
            None => None,
        };
        if let Some(key) = key {
            api_keys.insert(provider.to_owned(), key);
        }
    }
    let disabled_sources = read_state(conn, DISABLED_SOURCES_KEY)?
        .map(|value| value.split(',').map(str::to_owned).collect())
        .unwrap_or_default();
    Ok(Settings {
        api_keys,
        disabled_sources,
    })
}

pub fn save(conn: &Connection, settings: &Settings) -> Result<()> {
    for provider in settings.api_keys.keys() {
        if !KEYED_PROVIDERS.contains(&provider.as_str()) {
            return Err(Error::Validation(vec![FieldError::new(
                "apiKeys",
                ErrorCode::InvalidFormat,
            )]));
        }
    }
    for &provider in KEYED_PROVIDERS {
        let key = settings
            .api_keys
            .get(provider)
            .map(String::as_str)
            .map(str::trim)
            .filter(|key| !key.is_empty());
        write_state(conn, &api_key_state_key(provider), key)?;
        if provider == "google_books" {
            // Migrate away from the legacy single-key entry once the new one is written.
            write_state(conn, LEGACY_GOOGLE_BOOKS_API_KEY, None)?;
        }
    }
    let disabled = (!settings.disabled_sources.is_empty()).then(|| {
        settings
            .disabled_sources
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    });
    write_state(conn, DISABLED_SOURCES_KEY, disabled.as_deref())
}

/// The stored key for one provider, trimmed; `None` when unset.
pub fn api_key(conn: &Connection, provider: &str) -> Result<Option<String>> {
    Ok(load(conn)?.api_keys.remove(provider))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn keys(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn saves_trims_and_clears() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(load(&conn).unwrap(), Settings::default());

        save(
            &conn,
            &Settings {
                api_keys: keys(&[("google_books", "  abc ")]),
                disabled_sources: BTreeSet::new(),
            },
        )
        .unwrap();
        assert_eq!(
            api_key(&conn, "google_books").unwrap().as_deref(),
            Some("abc")
        );

        save(
            &conn,
            &Settings {
                api_keys: keys(&[("google_books", "")]),
                disabled_sources: BTreeSet::new(),
            },
        )
        .unwrap();
        assert_eq!(api_key(&conn, "google_books").unwrap(), None);
    }

    #[test]
    fn an_old_key_survives_until_overwritten() {
        let conn = db::open_in_memory().unwrap();
        write_state(&conn, LEGACY_GOOGLE_BOOKS_API_KEY, Some("legacy-key")).unwrap();

        let loaded = load(&conn).unwrap();
        assert_eq!(
            loaded.api_keys.get("google_books").map(String::as_str),
            Some("legacy-key")
        );

        // Saving anything migrates the legacy entry away.
        save(&conn, &loaded).unwrap();
        assert_eq!(
            read_state(&conn, LEGACY_GOOGLE_BOOKS_API_KEY).unwrap(),
            None
        );
        assert_eq!(
            api_key(&conn, "google_books").unwrap().as_deref(),
            Some("legacy-key")
        );
    }

    #[test]
    fn an_unknown_provider_is_rejected() {
        let conn = db::open_in_memory().unwrap();
        let result = save(
            &conn,
            &Settings {
                api_keys: keys(&[("unknown", "x")]),
                disabled_sources: BTreeSet::new(),
            },
        );
        match result {
            Err(Error::Validation(errors)) => {
                assert_eq!(
                    errors,
                    vec![FieldError::new("apiKeys", ErrorCode::InvalidFormat)]
                );
            }
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn disabled_sources_round_trip() {
        let conn = db::open_in_memory().unwrap();
        let mut disabled = BTreeSet::new();
        disabled.insert("wikidata".to_owned());
        disabled.insert("musicbrainz".to_owned());
        save(
            &conn,
            &Settings {
                api_keys: BTreeMap::new(),
                disabled_sources: disabled.clone(),
            },
        )
        .unwrap();

        assert_eq!(load(&conn).unwrap().disabled_sources, disabled);
    }
}
