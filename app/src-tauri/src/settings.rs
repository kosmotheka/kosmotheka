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
const GOOGLE_BOOKS_VIA_SERVER_KEY: &str = "google_books_via_server";

/// Provider ids that need a key (see `metadata::Source` for every source,
/// keyless ones included).
pub const KEYED_PROVIDERS: &[&str] = &["google_books", "gameupc", "tmdb", "rawg", "comic_vine"];

fn api_key_state_key(provider: &str) -> String {
    format!("api_key.{provider}")
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub api_keys: BTreeMap<String, String>,
    pub disabled_sources: BTreeSet<String>,
    /// Ask Google Books through the account's server when there is no key of their own (7.8, D83). On by default;
    /// it only matters with a session on a server that offers it.
    #[serde(default = "on")]
    pub google_books_via_server: bool,
}

fn on() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            api_keys: BTreeMap::new(),
            disabled_sources: BTreeSet::new(),
            google_books_via_server: true,
        }
    }
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
    // Absent means on; only an explicit «0» turns it off.
    let google_books_via_server =
        read_state(conn, GOOGLE_BOOKS_VIA_SERVER_KEY)?.is_none_or(|value| value != "0");
    Ok(Settings {
        api_keys,
        disabled_sources,
        google_books_via_server,
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
    write_state(conn, DISABLED_SOURCES_KEY, disabled.as_deref())?;
    write_state(
        conn,
        GOOGLE_BOOKS_VIA_SERVER_KEY,
        (!settings.google_books_via_server).then_some("0"),
    )
}

const WELCOME_SEEN_KEY: &str = "welcome_seen";

/// The version of the first-run welcome this device has seen (D91); 0 when none.
/// A welcome that adds a search service raises its version, so it shows again.
pub fn welcome_seen(conn: &Connection) -> Result<u32> {
    Ok(read_state(conn, WELCOME_SEEN_KEY)?
        .and_then(|value| value.parse().ok())
        .unwrap_or(0))
}

pub fn set_welcome_seen(conn: &Connection, version: u32) -> Result<()> {
    write_state(conn, WELCOME_SEEN_KEY, Some(&version.to_string()))
}

const WHATS_NEW_SEEN_KEY: &str = "whats_new_seen";

/// The app version whose news this device has seen; `None` when it never has (what-s-new screen).
pub fn whats_new_seen(conn: &Connection) -> Result<Option<String>> {
    read_state(conn, WHATS_NEW_SEEN_KEY)
}

pub fn set_whats_new_seen(conn: &Connection, version: &str) -> Result<()> {
    write_state(conn, WHATS_NEW_SEEN_KEY, Some(version))
}

const PERSON_NAME_KEY: &str = "person_name";

/// The name this device's person founds libraries with (D97): one per device, asked at the welcome. `None`
/// until it is given.
pub fn person_name(conn: &Connection) -> Result<Option<String>> {
    Ok(read_state(conn, PERSON_NAME_KEY)?.filter(|name| !name.trim().is_empty()))
}

/// The longest name a person can give.
const PERSON_NAME_MAX: usize = 100;

/// Saves the name this device's person founds libraries with. Blank or too long is refused.
pub fn set_person_name(conn: &Connection, raw: &str) -> Result<String> {
    let name = raw.trim();
    let code = if name.is_empty() {
        Some(ErrorCode::Required)
    } else if name.chars().count() > PERSON_NAME_MAX {
        Some(ErrorCode::OutOfRange)
    } else {
        None
    };
    if let Some(code) = code {
        return Err(Error::Validation(vec![FieldError::new("name", code)]));
    }
    write_state(conn, PERSON_NAME_KEY, Some(name))?;
    Ok(name.to_owned())
}

const APP_LOCK_KEY: &str = "app_lock";

/// Whether the fingerprint/face lock is on (D78). Off by default, like the key being absent.
pub fn app_lock(conn: &Connection) -> Result<bool> {
    Ok(read_state(conn, APP_LOCK_KEY)?.is_some())
}

pub fn set_app_lock(conn: &Connection, enabled: bool) -> Result<()> {
    write_state(conn, APP_LOCK_KEY, enabled.then_some("1"))
}

const APP_LOCK_AFTER_KEY: &str = "app_lock_after";

/// The choices for how long the app may stay in the background before it locks
/// again, in minutes: right away, 5 minutes, 1 hour, 12 hours or 1 day.
pub const APP_LOCK_AFTER_CHOICES: [u32; 5] = [0, 5, 60, 720, 1440];
const APP_LOCK_AFTER_DEFAULT: u32 = 60;

/// Minutes in the background before the lock comes back. An unknown stored value
/// (from a later version, say) falls back to the default instead of failing.
pub fn app_lock_after(conn: &Connection) -> Result<u32> {
    let stored = read_state(conn, APP_LOCK_AFTER_KEY)?.and_then(|value| value.parse().ok());
    Ok(stored
        .filter(|minutes| APP_LOCK_AFTER_CHOICES.contains(minutes))
        .unwrap_or(APP_LOCK_AFTER_DEFAULT))
}

pub fn set_app_lock_after(conn: &Connection, minutes: u32) -> Result<()> {
    if !APP_LOCK_AFTER_CHOICES.contains(&minutes) {
        return Err(Error::Validation(vec![FieldError::new(
            "minutes",
            ErrorCode::OutOfRange,
        )]));
    }
    write_state(conn, APP_LOCK_AFTER_KEY, Some(&minutes.to_string()))
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
    fn google_books_through_the_cloud_is_on_until_switched_off() {
        let conn = db::open_in_memory().unwrap();
        assert!(load(&conn).unwrap().google_books_via_server);

        let off = Settings {
            google_books_via_server: false,
            ..Settings::default()
        };
        save(&conn, &off).unwrap();
        assert!(!load(&conn).unwrap().google_books_via_server);

        save(&conn, &Settings::default()).unwrap();
        assert!(load(&conn).unwrap().google_books_via_server);
    }

    #[test]
    fn a_stored_value_nobody_wrote_leaves_it_on() {
        let conn = db::open_in_memory().unwrap();
        write_state(&conn, "google_books_via_server", Some("maybe")).unwrap();

        assert!(load(&conn).unwrap().google_books_via_server);
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
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(
            load(&conn)
                .unwrap()
                .api_keys
                .get("google_books")
                .map(String::as_str),
            Some("abc")
        );

        save(
            &conn,
            &Settings {
                api_keys: keys(&[("google_books", "")]),
                disabled_sources: BTreeSet::new(),
                ..Settings::default()
            },
        )
        .unwrap();
        assert_eq!(load(&conn).unwrap().api_keys.get("google_books"), None);
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
            load(&conn)
                .unwrap()
                .api_keys
                .get("google_books")
                .map(String::as_str),
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
                ..Settings::default()
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
    fn the_welcome_is_unseen_until_marked() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(welcome_seen(&conn).unwrap(), 0);

        set_welcome_seen(&conn, 1).unwrap();
        assert_eq!(welcome_seen(&conn).unwrap(), 1);
    }

    #[test]
    fn app_lock_is_off_until_turned_on() {
        let conn = db::open_in_memory().unwrap();
        assert!(!app_lock(&conn).unwrap());

        set_app_lock(&conn, true).unwrap();
        assert!(app_lock(&conn).unwrap());

        set_app_lock(&conn, false).unwrap();
        assert!(!app_lock(&conn).unwrap());
    }

    #[test]
    fn app_lock_after_defaults_to_an_hour_and_only_takes_the_choices() {
        let conn = db::open_in_memory().unwrap();
        assert_eq!(app_lock_after(&conn).unwrap(), 60);

        set_app_lock_after(&conn, 720).unwrap();
        assert_eq!(app_lock_after(&conn).unwrap(), 720);
        set_app_lock_after(&conn, 0).unwrap();
        assert_eq!(app_lock_after(&conn).unwrap(), 0);

        assert!(matches!(
            set_app_lock_after(&conn, 30),
            Err(Error::Validation(_))
        ));
        assert_eq!(app_lock_after(&conn).unwrap(), 0);
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
                ..Settings::default()
            },
        )
        .unwrap();

        assert_eq!(load(&conn).unwrap().disabled_sources, disabled);
    }
}
