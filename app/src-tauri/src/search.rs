//! Full-text search index (FTS5), kept current by `ops::record` (D29).

use std::collections::BTreeSet;

use library_domain::{Action, Change, Entity, Isbn};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;
use crate::ops::{read_state, write_state};

/// Bump when the indexed content changes, so existing indexes are rebuilt on startup.
const INDEX_VERSION: &str = "1";
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
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM editions")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    reindex(conn, &ids)
}

/// Editions whose indexed text may change because of `change`. Call it both before
/// and after applying the change: before catches rows about to vanish, after catches new ones.
pub fn affected_editions(conn: &Connection, change: &Change) -> Result<BTreeSet<String>> {
    let id = change.id.as_str();
    let field = |name: &str| change.fields.get(name).and_then(|v| v.as_str());
    let editions_of_work = |work_id: &str| -> Result<BTreeSet<String>> {
        query_ids(conn, "SELECT id FROM editions WHERE work_id = ?1", work_id)
    };

    let found = match change.entity {
        Entity::Edition | Entity::BookDetails => BTreeSet::from([id.to_owned()]),
        Entity::Work => editions_of_work(id)?,
        Entity::WorkContributor => {
            let work_id = match field("work_id") {
                Some(work_id) => Some(work_id.to_owned()),
                None => conn
                    .query_row(
                        "SELECT work_id FROM work_contributors WHERE id = ?1",
                        [id],
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
            "SELECT e.id FROM editions e JOIN work_contributors wc ON wc.work_id = e.work_id
              WHERE wc.person_id = ?1",
            id,
        )?,
        Entity::Publisher if change.action == Action::Update => {
            query_ids(conn, "SELECT id FROM editions WHERE publisher_id = ?1", id)?
        }
        Entity::Library
        | Entity::Person
        | Entity::Publisher
        | Entity::Member
        | Entity::Copy
        | Entity::Wish => BTreeSet::new(),
    };
    Ok(found)
}

pub fn reindex<'a>(
    conn: &Connection,
    edition_ids: impl IntoIterator<Item = &'a String>,
) -> Result<()> {
    for id in edition_ids {
        conn.execute("DELETE FROM search_index WHERE edition_id = ?1", [id])?;
        if let Some(content) = document(conn, id)? {
            conn.execute(
                "INSERT INTO search_index (edition_id, content) VALUES (?1, ?2)",
                params![id, content],
            )?;
        }
    }
    Ok(())
}

/// The text a book is found by. `None` if the edition no longer exists.
fn document(conn: &Connection, edition_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT concat_ws(' ', e.title, e.subtitle, w.title, p.name, bd.isbn13,
                    (SELECT group_concat(pe.name, ' ')
                       FROM work_contributors wc JOIN people pe ON pe.id = wc.person_id
                      WHERE wc.work_id = e.work_id))
               FROM editions e
               JOIN works w ON w.id = e.work_id
               LEFT JOIN publishers p ON p.id = e.publisher_id
               LEFT JOIN book_details bd ON bd.edition_id = e.id
              WHERE e.id = ?1",
            [edition_id],
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

fn query_ids(conn: &Connection, sql: &str, param: &str) -> Result<BTreeSet<String>> {
    Ok(conn
        .prepare(sql)?
        .query_map([param], |row| row.get(0))?
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
