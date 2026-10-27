//! The trash (D105): what a person deletes from the app (an edition, a copy, a tag, a series) stays 30 days with
//! `trashed_at` and `trashed_by` set, and everything hanging from it untouched, so restoring brings it back whole.
//! Deleting for good is what deleting was before the trash. Who may do each is the judge's rule 16.

use std::collections::HashSet;

use kosmotheka_domain::{Change, Entity, timestamp_millis, timestamp_text};
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalog;
use crate::error::{Error, Result};
use crate::{marks, ops};

const DAY_MILLIS: u64 = 24 * 60 * 60 * 1000;

/// One of the four things the trash holds, as the screen names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrashKind {
    Edition,
    Copy,
    Tag,
    Series,
}

impl TrashKind {
    fn entity(self) -> Entity {
        match self {
            TrashKind::Edition => Entity::Edition,
            TrashKind::Copy => Entity::Copy,
            TrashKind::Tag => Entity::Tag,
            TrashKind::Series => Entity::Series,
        }
    }
}

/// Something in the trash, picked on the screen.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashRef {
    pub kind: TrashKind,
    pub id: String,
}

/// A row of the trash screen.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub kind: TrashKind,
    pub id: String,
    /// The edition's title, the copy's edition's title, the tag's or the series' name.
    pub title: String,
    /// What else tells it apart: a copy's format, an edition's kind of work.
    pub detail: Option<String>,
    /// What comes back with it: an edition's copies, a tag's works, a series' works.
    pub count: i64,
    pub trashed_at: String,
    /// The name of whoever sent it, if that person is still in the library.
    pub trashed_by: Option<String>,
    /// When it goes for good, unless restored first (`trashed_at` and 30 days, for now).
    pub purge_at: String,
}

/// The fields that send a row to the trash: now, by this device's person. A device of nobody sends nothing.
fn send_fields(conn: &Connection, library: &str) -> Result<serde_json::Value> {
    let me = marks::my_member(conn, library)?.ok_or(Error::NotAllowed)?;
    Ok(json!({ "trashed_at": timestamp_text(ops::now_millis()), "trashed_by": me }))
}

/// The change that sends `id` to the trash, for whoever builds a bigger operation (moving a book out).
pub fn send_change(conn: &Connection, library: &str, entity: Entity, id: &str) -> Result<Change> {
    Ok(Change::update(entity, id, send_fields(conn, library)?))
}

/// Sends one thing to the trash, in an operation of its own.
pub fn send(
    conn: &mut Connection,
    library: &str,
    kind: TrashKind,
    id: &str,
    intent: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    catalog::ensure_exists(&tx, library, kind.entity(), id)?;
    let change = send_change(&tx, library, kind.entity(), id)?;
    ops::record(&tx, library, intent, vec![change])?;
    tx.commit()?;
    Ok(())
}

/// Brings things back from the trash, all in one operation.
pub fn restore(conn: &mut Connection, library: &str, items: &[TrashRef]) -> Result<()> {
    let tx = conn.transaction()?;
    let mut changes = Vec::new();
    for item in unique(items) {
        catalog::ensure_exists(&tx, library, item.kind.entity(), &item.id)?;
        changes.push(Change::update(
            item.kind.entity(),
            &item.id,
            json!({ "trashed_at": null, "trashed_by": null }),
        ));
    }
    if !changes.is_empty() {
        ops::record(&tx, library, "trash.restore", changes)?;
    }
    tx.commit()?;
    Ok(())
}

/// Deletes things for good, with everything hanging from them, all in one operation: all or nothing.
pub fn purge(conn: &mut Connection, library: &str, items: &[TrashRef]) -> Result<()> {
    let tx = conn.transaction()?;
    let changes = purge_changes(&tx, library, items)?;
    if !changes.is_empty() {
        ops::record(&tx, library, "trash.purge", changes)?;
    }
    tx.commit()?;
    Ok(())
}

/// The deletes of `items` and what hangs from them, children first, each row once (an edition and one of its
/// copies, picked together, delete that copy once).
pub fn purge_changes(
    tx: &Transaction<'_>,
    library: &str,
    items: &[TrashRef],
) -> Result<Vec<Change>> {
    let mut seen = HashSet::new();
    let mut changes = Vec::new();
    for item in unique(items) {
        catalog::ensure_exists(tx, library, item.kind.entity(), &item.id)?;
        for change in removal(tx, library, item.kind, &item.id)? {
            if seen.insert((change.entity, change.id.clone())) {
                changes.push(change);
            }
        }
    }
    Ok(changes)
}

/// What deleting one thing for good takes with it: what deleting it took before the trash.
fn removal(conn: &Connection, library: &str, kind: TrashKind, id: &str) -> Result<Vec<Change>> {
    let links = |sql: &str, entity: Entity| -> Result<Vec<Change>> {
        Ok(catalog::ids(conn, sql, library, id)?
            .into_iter()
            .map(|row| Change::delete(entity, row))
            .collect())
    };
    let mut changes = match kind {
        TrashKind::Edition => return catalog::book_removal(conn, library, id),
        TrashKind::Copy => Vec::new(),
        TrashKind::Tag => links(
            "SELECT id FROM work_tags WHERE library_id = ?1 AND tag_id = ?2",
            Entity::WorkTag,
        )?,
        TrashKind::Series => links(
            "SELECT id FROM series_entries WHERE library_id = ?1 AND series_id = ?2",
            Entity::SeriesEntry,
        )?,
    };
    changes.push(Change::delete(kind.entity(), id));
    Ok(changes)
}

/// Each picked thing once, in the order given.
fn unique(items: &[TrashRef]) -> Vec<&TrashRef> {
    let mut seen = HashSet::new();
    items.iter().filter(|item| seen.insert(*item)).collect()
}

/// When something sent at `trashed_at` goes for good.
fn purge_at(trashed_at: &str) -> String {
    timestamp_millis(trashed_at)
        .map(|since| timestamp_text(since + 30 * DAY_MILLIS))
        .unwrap_or_default()
}

/// The trash of `library`, the latest first. A copy whose edition is in the trash too is not listed apart: it
/// comes back with its edition.
pub fn list(conn: &Connection, library: &str) -> Result<Vec<TrashItem>> {
    let queries: [(TrashKind, &str); 4] = [
        (
            TrashKind::Edition,
            "SELECT e.id, e.title, w.kind, e.trashed_at, m.name,
                    (SELECT count(*) FROM copies c WHERE c.library_id = e.library_id AND c.edition_id = e.id)
               FROM editions e
               JOIN works w ON w.library_id = e.library_id AND w.id = e.work_id
               LEFT JOIN members m ON m.library_id = e.library_id AND m.id = e.trashed_by
              WHERE e.library_id = ?1 AND e.trashed_at IS NOT NULL",
        ),
        (
            TrashKind::Copy,
            "SELECT c.id, e.title, c.format, c.trashed_at, m.name, c.quantity
               FROM copies c
               JOIN editions e ON e.library_id = c.library_id AND e.id = c.edition_id
               LEFT JOIN members m ON m.library_id = c.library_id AND m.id = c.trashed_by
              WHERE c.library_id = ?1 AND c.trashed_at IS NOT NULL AND e.trashed_at IS NULL",
        ),
        (
            TrashKind::Tag,
            "SELECT t.id, t.name, NULL, t.trashed_at, m.name,
                    (SELECT count(DISTINCT wt.work_id) FROM work_tags wt
                      WHERE wt.library_id = t.library_id AND wt.tag_id = t.id)
               FROM tags t
               LEFT JOIN members m ON m.library_id = t.library_id AND m.id = t.trashed_by
              WHERE t.library_id = ?1 AND t.trashed_at IS NOT NULL",
        ),
        (
            TrashKind::Series,
            "SELECT s.id, s.name, NULL, s.trashed_at, m.name,
                    (SELECT count(DISTINCT se.work_id) FROM series_entries se
                      WHERE se.library_id = s.library_id AND se.series_id = s.id)
               FROM series s
               LEFT JOIN members m ON m.library_id = s.library_id AND m.id = s.trashed_by
              WHERE s.library_id = ?1 AND s.trashed_at IS NOT NULL",
        ),
    ];
    let mut items = Vec::new();
    for (kind, sql) in queries {
        let mut statement = conn.prepare(sql)?;
        let rows = statement.query_map([library], |row| {
            let trashed_at: String = row.get(3)?;
            Ok(TrashItem {
                kind,
                id: row.get(0)?,
                title: row.get(1)?,
                detail: row.get(2)?,
                count: row.get(5)?,
                purge_at: purge_at(&trashed_at),
                trashed_at,
                trashed_by: row.get(4)?,
            })
        })?;
        items.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    items.sort_by(|a, b| b.trashed_at.cmp(&a.trashed_at));
    Ok(items)
}
