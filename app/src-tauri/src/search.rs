//! Full-text search index (FTS5), kept current by `ops::record` (D29).

use std::collections::BTreeSet;

use kosmotheka_domain::{Action, Change, Entity, Isbn};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;
use crate::ops::{read_state, write_state};

/// Bump when the indexed content changes, so existing indexes are rebuilt on startup.
const INDEX_VERSION: &str = "2";
const INDEX_VERSION_KEY: &str = "search_index_version";

pub fn ensure_current(conn: &Connection) -> Result<()> {
    if read_state(conn, INDEX_VERSION_KEY)?.as_deref() != Some(INDEX_VERSION) {
        rebuild(conn)?;
        write_state(conn, INDEX_VERSION_KEY, Some(INDEX_VERSION))?;
    }
    Ok(())
}

pub fn rebuild(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM search_index", [])?;
    let rows: Vec<(String, String)> = conn
        .prepare("SELECT library_id, id FROM editions")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (library, id) in &rows {
        reindex(conn, library, [id])?;
    }
    Ok(())
}

/// Editions whose indexed text may change because of `change`. Call it both before
/// and after applying the change: before catches rows about to vanish, after catches new ones.
pub fn affected_editions(
    conn: &Connection,
    library: &str,
    change: &Change,
) -> Result<BTreeSet<String>> {
    let id = change.id.as_str();
    let field = |name: &str| change.fields.get(name).and_then(|v| v.as_str());
    let editions_of_work = |work_id: &str| -> Result<BTreeSet<String>> {
        query_ids(
            conn,
            "SELECT id FROM editions WHERE library_id = ?1 AND work_id = ?2",
            library,
            work_id,
        )
    };

    let found = match change.entity {
        Entity::Edition | Entity::BookDetails => BTreeSet::from([id.to_owned()]),
        Entity::Work => editions_of_work(id)?,
        Entity::WorkContributor => {
            let work_id = match field("work_id") {
                Some(work_id) => Some(work_id.to_owned()),
                None => conn
                    .query_row(
                        "SELECT work_id FROM work_contributors WHERE library_id = ?1 AND id = ?2",
                        [library, id],
                        |r| r.get(0),
                    )
                    .optional()?,
            };
            match work_id {
                Some(work_id) => editions_of_work(&work_id)?,
                None => BTreeSet::new(),
            }
        }
        // Only renames change indexed text; creations have no editions yet.
        Entity::Person if change.action == Action::Update => query_ids(
            conn,
            "SELECT e.id FROM editions e
               JOIN work_contributors wc ON wc.library_id = e.library_id AND wc.work_id = e.work_id
              WHERE e.library_id = ?1 AND wc.person_id = ?2",
            library,
            id,
        )?,
        Entity::Publisher if change.action == Action::Update => query_ids(
            conn,
            "SELECT id FROM editions WHERE library_id = ?1 AND publisher_id = ?2",
            library,
            id,
        )?,
        Entity::Library
        | Entity::Person
        | Entity::Publisher
        | Entity::Member
        | Entity::Copy
        | Entity::Wish
        | Entity::Revocation
        | Entity::Device
        | Entity::ChangeRequest
        | Entity::Mark => BTreeSet::new(),
    };
    Ok(found)
}

pub fn reindex<'a>(
    conn: &Connection,
    library: &str,
    edition_ids: impl IntoIterator<Item = &'a String>,
) -> Result<()> {
    for id in edition_ids {
        conn.execute(
            "DELETE FROM search_index WHERE library_id = ?1 AND edition_id = ?2",
            [library, id],
        )?;
        if let Some(content) = document(conn, library, id)? {
            conn.execute(
                "INSERT INTO search_index (library_id, edition_id, content) VALUES (?1, ?2, ?3)",
                params![library, id, content],
            )?;
        }
    }
    Ok(())
}

/// The text a book is found by. `None` if the edition no longer exists.
fn document(conn: &Connection, library: &str, edition_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT concat_ws(' ', e.title, e.subtitle, w.title, p.name, bd.isbn13,
                    (SELECT group_concat(pe.name, ' ')
                       FROM work_contributors wc
                       JOIN people pe ON pe.library_id = wc.library_id AND pe.id = wc.person_id
                      WHERE wc.library_id = e.library_id AND wc.work_id = e.work_id))
               FROM editions e
               JOIN works w ON w.library_id = e.library_id AND w.id = e.work_id
               LEFT JOIN publishers p ON p.library_id = e.library_id AND p.id = e.publisher_id
               LEFT JOIN book_details bd ON bd.library_id = e.library_id AND bd.edition_id = e.id
              WHERE e.library_id = ?1 AND e.id = ?2",
            [library, edition_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Turns what the user typed into a safe FTS5 query: every word must match, as a
/// prefix. Anything that is not a letter or digit is dropped, so FTS5 syntax
/// characters typed by the user can never break or change the query.
pub fn fts_query(input: &str) -> Option<String> {
    let input = input.trim();
    if let Ok(isbn) = Isbn::parse(input) {
        return Some(format!("\"{}\"", isbn.as_str()));
    }
    let terms: Vec<String> = input
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| format!("\"{word}\"*"))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" "))
}

fn query_ids(conn: &Connection, sql: &str, library: &str, id: &str) -> Result<BTreeSet<String>> {
    Ok(conn
        .prepare(sql)?
        .query_map([library, id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_become_prefix_terms() {
        assert_eq!(fts_query("  el señor  "), Some("\"el\"* \"señor\"*".into()));
    }

    #[test]
    fn syntax_characters_are_dropped() {
        assert_eq!(
            fts_query("tolkien\" OR NEAR(*)"),
            Some("\"tolkien\"* \"OR\"* \"NEAR\"*".into())
        );
        assert_eq!(fts_query(" - * \" "), None);
    }

    #[test]
    fn isbns_are_normalised() {
        assert_eq!(fts_query("0-306-40615-2"), Some("\"9780306406157\"".into()));
    }
}
