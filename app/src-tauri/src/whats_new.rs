//! What is new after an update. The lines live in one file, `app/src/whats-new.json`, written when each change is
//! made; the CI script `check:whats-new` reads the same file. Which versions a person still has to see is decided
//! here, not in the screen.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::metadata::Source;
use crate::settings::{self, Settings};
use kosmotheka_domain::{ErrorCode, FieldError};

const FILE: &str = include_str!("../../src/whats-new.json");

/// The language every line has text in, shown when a line has none in the person's.
const FALLBACK_LANGUAGE: &str = "en";

/// Lines of the version that has no number yet.
const NEXT: &str = "next";

/// The last version published before this screen existed: whoever already saw the welcome and has no record of
/// news starts from it.
const BASELINE: &str = "0.2.0";

/// How many versions «About» shows.
const RECENT: usize = 3;

#[derive(Deserialize)]
struct Entry {
    /// The text in each language, by language code. English is always there (`notes` checks), the fallback of the
    /// screen for a language a line has no text in.
    #[serde(flatten)]
    text: BTreeMap<String, String>,
    /// A setting this change is tied to, shown with its switch.
    switch: Option<String>,
}

type Notes = BTreeMap<String, Vec<Entry>>;

/// The settings a line may carry a switch for: only ones a person can already change.
enum Switch {
    Source(Source),
    GoogleBooksViaServer,
}

impl Switch {
    fn parse(key: &str) -> Option<Self> {
        if key == "google_books_via_server" {
            return Some(Self::GoogleBooksViaServer);
        }
        let id = key.strip_prefix("source:")?;
        Source::ALL
            .into_iter()
            .find(|source| source.id() == id)
            .map(Self::Source)
    }

    fn is_on(&self, settings: &Settings) -> bool {
        match self {
            Self::Source(source) => !settings.disabled_sources.contains(source.id()),
            Self::GoogleBooksViaServer => settings.google_books_via_server,
        }
    }

    fn set(&self, settings: &mut Settings, on: bool) {
        match self {
            Self::Source(source) if on => {
                settings.disabled_sources.remove(source.id());
            }
            Self::Source(source) => {
                settings.disabled_sources.insert(source.id().to_owned());
            }
            Self::GoogleBooksViaServer => settings.google_books_via_server = on,
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchState {
    pub key: String,
    pub on: bool,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// The text in each language, by language code, flattened next to `switch` in what the screen gets.
    #[serde(flatten)]
    pub text: BTreeMap<String, String>,
    pub switch: Option<SwitchState>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionNotes {
    /// `next` for lines that have no version yet.
    pub version: String,
    pub lines: Vec<Line>,
}

/// What the screen shows. What changes what leaves the device goes first and apart, with its switch.
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsNew {
    pub switches: Vec<Line>,
    pub versions: Vec<VersionNotes>,
}

#[cfg(test)]
impl WhatsNew {
    fn is_empty(&self) -> bool {
        self.switches.is_empty() && self.versions.is_empty()
    }
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|part| part.parse::<u64>().ok());
    let found = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(found)
}

fn notes() -> Result<Notes> {
    let notes: Notes = serde_json::from_str(FILE)?;
    let without_english = notes.values().flatten().any(|entry| {
        entry
            .text
            .get(FALLBACK_LANGUAGE)
            .is_none_or(String::is_empty)
    });
    if without_english {
        return Err(Error::Validation(vec![FieldError::new(
            FALLBACK_LANGUAGE,
            ErrorCode::Required,
        )]));
    }
    Ok(notes)
}

/// The versions after `after` (all when `None`) up to the installed one, newest first. `next` comes first, only
/// while the installed version has no lines of its own: that is a build between releases.
fn select<'a>(
    notes: &'a Notes,
    installed: &str,
    after: Option<&str>,
) -> Vec<(&'a str, &'a [Entry])> {
    let installed = parse_version(installed);
    let after = after.and_then(parse_version);
    let mut numbered: Vec<_> = notes
        .iter()
        .filter_map(|(version, lines)| Some((parse_version(version)?, version, lines)))
        .filter(|(number, _, _)| Some(*number) <= installed && after.is_none_or(|a| *number > a))
        .collect();
    numbered.sort_by_key(|(number, _, _)| std::cmp::Reverse(*number));
    let has_own = notes
        .keys()
        .any(|version| parse_version(version) == installed);
    let mut chosen: Vec<(&str, &[Entry])> = Vec::new();
    if !has_own && let Some(lines) = notes.get(NEXT) {
        chosen.push((NEXT, lines));
    }
    chosen.extend(
        numbered
            .into_iter()
            .map(|(_, version, lines)| (version.as_str(), lines.as_slice())),
    );
    chosen
}

fn build(conn: &Connection, chosen: &[(&str, &[Entry])]) -> Result<WhatsNew> {
    let settings = settings::load(conn)?;
    let mut shown = WhatsNew::default();
    for (version, entries) in chosen {
        let mut lines = Vec::new();
        for entry in *entries {
            let line = |switch| Line {
                text: entry.text.clone(),
                switch,
            };
            match &entry.switch {
                None => lines.push(line(None)),
                Some(key) => {
                    let known = Switch::parse(key).ok_or_else(unknown_switch)?;
                    shown.switches.push(line(Some(SwitchState {
                        key: key.clone(),
                        on: known.is_on(&settings),
                    })));
                }
            }
        }
        if !lines.is_empty() {
            shown.versions.push(VersionNotes {
                version: (*version).to_owned(),
                lines,
            });
        }
    }
    Ok(shown)
}

fn unknown_switch() -> Error {
    Error::Validation(vec![FieldError::new("switch", ErrorCode::InvalidFormat)])
}

/// What this device still has to see. A device that never saw the welcome is new: it has nothing to catch up on,
/// so the installed version is recorded and nothing shows.
pub fn pending(conn: &Connection, installed: &str) -> Result<WhatsNew> {
    pending_in(conn, &notes()?, installed)
}

fn pending_in(conn: &Connection, notes: &Notes, installed: &str) -> Result<WhatsNew> {
    let seen = settings::whats_new_seen(conn)?;
    if seen.is_none() && settings::welcome_seen(conn)? == 0 {
        settings::set_whats_new_seen(conn, installed)?;
        return Ok(WhatsNew::default());
    }
    let after = seen
        .as_deref()
        .filter(|version| parse_version(version).is_some())
        .unwrap_or(BASELINE);
    build(conn, &select(notes, installed, Some(after)))
}

/// The latest versions, to read again.
pub fn recent(conn: &Connection, installed: &str) -> Result<WhatsNew> {
    let notes = notes()?;
    let mut chosen = select(&notes, installed, None);
    chosen.truncate(RECENT);
    build(conn, &chosen)
}

/// Turns the setting a line is tied to on or off, through the same settings «Data providers» changes.
pub fn set_switch(conn: &Connection, key: &str, on: bool) -> Result<()> {
    let switch = Switch::parse(key).ok_or_else(unknown_switch)?;
    let mut current = settings::load(conn)?;
    switch.set(&mut current, on);
    settings::save(conn, &current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory_blank;

    fn notes_from(json: &str) -> Notes {
        serde_json::from_str(json).unwrap()
    }

    fn line(text: &str) -> String {
        format!(r#"{{"es":"{text}","en":"{text}"}}"#)
    }

    fn sample() -> Notes {
        notes_from(&format!(
            r#"{{"next":[{}],"0.2.0":[{}],"0.3.0":[{}],"0.4.0":[{}]}}"#,
            line("soon"),
            line("two"),
            line("three"),
            line("four"),
        ))
    }

    fn versions(shown: &WhatsNew) -> Vec<&str> {
        shown.versions.iter().map(|v| v.version.as_str()).collect()
    }

    fn welcomed() -> Connection {
        let conn = open_in_memory_blank().unwrap();
        settings::set_welcome_seen(&conn, 2).unwrap();
        conn
    }

    #[test]
    fn the_real_file_is_valid_and_its_switches_exist() {
        let notes = notes().unwrap();
        assert!(notes.contains_key(env!("CARGO_PKG_VERSION")));
        for (version, entries) in &notes {
            assert!(version == NEXT || parse_version(version).is_some());
            assert!((1..=5).contains(&entries.len()), "{version}");
            for entry in entries {
                for language in ["es", "en"] {
                    assert!(
                        entry
                            .text
                            .get(language)
                            .is_some_and(|text| !text.is_empty())
                    );
                }
                if let Some(key) = &entry.switch {
                    assert!(Switch::parse(key).is_some(), "{key}");
                }
            }
        }
        // And it can be shown whatever the device has.
        let conn = welcomed();
        recent(&conn, env!("CARGO_PKG_VERSION")).unwrap();
    }

    #[test]
    fn a_new_device_sees_nothing_and_is_recorded_as_up_to_date() {
        let conn = open_in_memory_blank().unwrap();
        let shown = pending_in(&conn, &sample(), "0.3.0").unwrap();
        assert!(shown.is_empty());
        assert_eq!(
            settings::whats_new_seen(&conn).unwrap().as_deref(),
            Some("0.3.0")
        );
        // The next update shows only what came after.
        assert_eq!(
            versions(&pending_in(&conn, &sample(), "0.4.0").unwrap()),
            ["0.4.0"]
        );
    }

    #[test]
    fn a_device_from_before_the_screen_starts_from_the_last_version_without_it() {
        let conn = welcomed();
        assert!(settings::whats_new_seen(&conn).unwrap().is_none());
        let shown = pending_in(&conn, &sample(), "0.3.0").unwrap();
        assert_eq!(versions(&shown), ["0.3.0"], "0.2.0 is the base, not news");
    }

    #[test]
    fn who_skipped_versions_sees_them_together_newest_first() {
        let conn = welcomed();
        settings::set_whats_new_seen(&conn, "0.2.0").unwrap();
        let shown = pending_in(&conn, &sample(), "0.4.0").unwrap();
        assert_eq!(versions(&shown), ["0.4.0", "0.3.0"]);
    }

    #[test]
    fn what_was_seen_is_not_shown_again() {
        let conn = welcomed();
        settings::set_whats_new_seen(&conn, "0.4.0").unwrap();
        assert!(pending_in(&conn, &sample(), "0.4.0").unwrap().is_empty());
        // Nor does a downgrade show a newer version.
        assert!(pending_in(&conn, &sample(), "0.3.0").unwrap().is_empty());
    }

    #[test]
    fn a_garbled_record_falls_back_to_the_base() {
        let conn = welcomed();
        settings::set_whats_new_seen(&conn, "soon").unwrap();
        let shown = pending_in(&conn, &sample(), "0.3.0").unwrap();
        assert_eq!(versions(&shown), ["0.3.0"]);
    }

    #[test]
    fn the_lines_without_a_number_show_only_between_releases() {
        let conn = welcomed();
        settings::set_whats_new_seen(&conn, "0.4.0").unwrap();
        // Installed 0.4.0 has lines of its own: «next» is for a later build.
        assert!(pending_in(&conn, &sample(), "0.4.0").unwrap().is_empty());
        // A build whose version has none yet shows them first.
        let shown = pending_in(&conn, &sample(), "0.4.1").unwrap();
        assert_eq!(versions(&shown), ["next"]);
    }

    #[test]
    fn a_line_with_a_switch_goes_first_and_apart_with_the_setting_as_it_is() {
        let conn = welcomed();
        let notes = notes_from(
            r#"{"0.3.0":[
                {"es":"a","en":"a"},
                {"es":"fuente","en":"source","switch":"source:library_of_congress"},
                {"es":"nube","en":"cloud","switch":"google_books_via_server"}
            ]}"#,
        );
        let mut current = settings::load(&conn).unwrap();
        current
            .disabled_sources
            .insert("library_of_congress".into());
        settings::save(&conn, &current).unwrap();

        let shown = pending_in(&conn, &notes, "0.3.0").unwrap();
        let state: Vec<_> = shown
            .switches
            .iter()
            .map(|l| {
                let s = l.switch.as_ref().unwrap();
                (s.key.as_str(), s.on)
            })
            .collect();
        assert_eq!(
            state,
            [
                ("source:library_of_congress", false),
                ("google_books_via_server", true)
            ]
        );
        assert_eq!(
            shown.versions[0].lines.len(),
            1,
            "the plain line stays in its version"
        );
    }

    #[test]
    fn an_unknown_switch_is_refused_and_nothing_shows() {
        let conn = welcomed();
        for key in ["source:nope", "disabled_sources", "api_key.tmdb", ""] {
            let notes = notes_from(&format!(
                r#"{{"0.3.0":[{{"es":"a","en":"a","switch":"{key}"}}]}}"#
            ));
            assert!(pending_in(&conn, &notes, "0.3.0").is_err(), "{key}");
            assert!(set_switch(&conn, key, true).is_err(), "{key}");
        }
    }

    #[test]
    fn the_switch_changes_the_same_setting_providers_does() {
        let conn = welcomed();
        set_switch(&conn, "source:tmdb", false).unwrap();
        set_switch(&conn, "google_books_via_server", false).unwrap();
        let current = settings::load(&conn).unwrap();
        assert!(current.disabled_sources.contains("tmdb"));
        assert!(!current.google_books_via_server);
        set_switch(&conn, "source:tmdb", true).unwrap();
        assert!(settings::load(&conn).unwrap().disabled_sources.is_empty());
    }

    #[test]
    fn about_shows_the_latest_versions_even_if_seen() {
        let conn = welcomed();
        settings::set_whats_new_seen(&conn, "0.4.0").unwrap();
        let notes = sample();
        let mut chosen = select(&notes, "0.4.0", None);
        chosen.truncate(RECENT);
        let shown = build(&conn, &chosen).unwrap();
        assert_eq!(versions(&shown), ["0.4.0", "0.3.0", "0.2.0"]);
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert!(parse_version("0.10.0") > parse_version("0.9.0"));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("next"), None);
    }
}
