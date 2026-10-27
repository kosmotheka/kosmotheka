//! The trash (D105): what a person deletes from the app (an edition, a copy, a tag, a series) stays 30 days with
//! `trashed_at` and `trashed_by` set, and everything hanging from it untouched, so restoring brings it back whole.
//! Deleting for good is what deleting was before the trash. Who may do each is the judge's rule 16.

use std::collections::HashSet;

use kosmotheka_domain::{Change, Entity, TRASHABLE, timestamp_millis, timestamp_text};
use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalog;
use crate::error::{Error, Result};
use crate::{marks, ops};

const DAY_MILLIS: u64 = 24 * 60 * 60 * 1000;
/// What the judge allows an editor (rule 16); this device waits as long, counted from when it saw it.
const TRASH_MILLIS: u64 = kosmotheka_domain::permission::TRASH_DAYS * DAY_MILLIS;

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
    /// When it goes for good, unless restored first: 30 days from the later of `trashed_at` and when this device
    /// first saw it in the trash.
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

/// The moment from which something counts its 30 days: the later of when it was sent and when this device saw it.
fn counted_from(trashed_at: &str, seen_at: Option<i64>) -> u64 {
    let sent = timestamp_millis(trashed_at).unwrap_or(0);
    let seen = seen_at
        .and_then(|seen| u64::try_from(seen).ok())
        .unwrap_or(0);
    sent.max(seen)
}

/// Notes when this device first sees each thing in the trash of `library`, and forgets what is no longer there
/// (restored or gone), so sending it again later counts from then.
pub fn observe(conn: &Connection, library: &str, now: u64) -> Result<()> {
    let now = i64::try_from(now).unwrap_or(i64::MAX);
    for entity in TRASHABLE {
        let table = entity.table();
        conn.execute(
            &format!(
                "INSERT OR IGNORE INTO trash_seen (library_id, entity, entity_id, seen_at)
                 SELECT library_id, ?2, id, ?3 FROM {table}
                  WHERE library_id = ?1 AND trashed_at IS NOT NULL"
            ),
            rusqlite::params![library, table, now],
        )?;
        conn.execute(
            &format!(
                "DELETE FROM trash_seen WHERE library_id = ?1 AND entity = ?2
                   AND NOT EXISTS (SELECT 1 FROM {table} t WHERE t.library_id = trash_seen.library_id
                                     AND t.id = trash_seen.entity_id AND t.trashed_at IS NOT NULL)"
            ),
            [library, table],
        )?;
    }
    Ok(())
}

/// What has been 30 days in the trash of `library`, by this device's count.
fn due(conn: &Connection, library: &str, now: u64) -> Result<Vec<TrashRef>> {
    let kinds = [
        (TrashKind::Edition, Entity::Edition),
        (TrashKind::Copy, Entity::Copy),
        (TrashKind::Tag, Entity::Tag),
        (TrashKind::Series, Entity::Series),
    ];
    let mut due = Vec::new();
    for (kind, entity) in kinds {
        let table = entity.table();
        let mut statement = conn.prepare(&format!(
            "SELECT t.id, t.trashed_at, s.seen_at FROM {table} t
               LEFT JOIN trash_seen s ON s.library_id = t.library_id AND s.entity = ?2 AND s.entity_id = t.id
              WHERE t.library_id = ?1 AND t.trashed_at IS NOT NULL"
        ))?;
        let rows = statement.query_map([library, table], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })?;
        for row in rows {
            let (id, trashed_at, seen_at) = row?;
            if now >= counted_from(&trashed_at, seen_at).saturating_add(TRASH_MILLIS) {
                due.push(TrashRef { kind, id });
            }
        }
    }
    Ok(due)
}

/// Deletes for good what has been 30 days in the trash of `library`, in one operation, if this device's person
/// may (an editor or an administrator). Never as a request: a reader's device leaves it for others. Returns how
/// many things went.
pub fn expire(conn: &mut Connection, library: &str, now: u64) -> Result<usize> {
    observe(conn, library, now)?;
    let due = due(conn, library, now)?;
    if due.is_empty() {
        return Ok(0);
    }
    let tx = conn.transaction()?;
    let changes = purge_changes(&tx, library, &due)?;
    if changes.is_empty() || !ops::would_allow(&tx, library, changes.clone())? {
        return Ok(0);
    }
    ops::record(&tx, library, "trash.expire", changes)?;
    tx.commit()?;
    observe(conn, library, now)?;
    Ok(due.len())
}

/// `expire` in every library of this device; one that fails does not stop the others.
pub fn expire_all(conn: &mut Connection) -> Result<usize> {
    conn.execute(
        "DELETE FROM trash_seen WHERE library_id NOT IN (SELECT id FROM libraries)",
        [],
    )?;
    let libraries = catalog_libraries(conn)?;
    let now = ops::now_millis();
    Ok(libraries
        .iter()
        .filter_map(|library| expire(conn, library, now).ok())
        .sum())
}

/// The tag or series of that name in the trash, if any, to offer restoring it instead of creating a second one (D105).
pub fn named(
    conn: &Connection,
    library: &str,
    kind: TrashKind,
    name: &str,
) -> Result<Option<TrashItem>> {
    if !matches!(kind, TrashKind::Tag | TrashKind::Series) {
        return Ok(None);
    }
    let id: Option<String> = conn
        .query_row(
            &format!(
                "SELECT id FROM {} WHERE library_id = ?1 AND name = ?2 COLLATE NOCASE
                   AND trashed_at IS NOT NULL ORDER BY trashed_at DESC LIMIT 1",
                kind.entity().table()
            ),
            [library, name.trim()],
            |row| row.get(0),
        )
        .optional()?;
    let Some(id) = id else { return Ok(None) };
    Ok(list(conn, library)?
        .into_iter()
        .find(|item| item.kind == kind && item.id == id))
}

fn catalog_libraries(conn: &Connection) -> Result<Vec<String>> {
    let mut statement = conn.prepare("SELECT id FROM libraries")?;
    let rows = statement.query_map([], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The trash of `library`, the latest first. A copy whose edition is in the trash too is not listed apart: it
/// comes back with its edition.
pub fn list(conn: &Connection, library: &str) -> Result<Vec<TrashItem>> {
    observe(conn, library, ops::now_millis())?;
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
        let seen = |id: &str| -> rusqlite::Result<Option<i64>> {
            conn.query_row(
                "SELECT seen_at FROM trash_seen WHERE library_id = ?1 AND entity = ?2 AND entity_id = ?3",
                [library, kind.entity().table(), id],
                |row| row.get(0),
            )
            .optional()
        };
        let rows = statement.query_map([library], |row| {
            let id: String = row.get(0)?;
            let trashed_at: String = row.get(3)?;
            let from = counted_from(&trashed_at, seen(&id)?);
            Ok(TrashItem {
                kind,
                title: row.get(1)?,
                detail: row.get(2)?,
                count: row.get(5)?,
                purge_at: timestamp_text(from.saturating_add(TRASH_MILLIS)),
                trashed_at,
                trashed_by: row.get(4)?,
                id,
            })
        })?;
        items.extend(rows.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    items.sort_by(|a, b| b.trashed_at.cmp(&a.trashed_at));
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn tag(conn: &mut Connection, library: &str, name: &str) -> String {
        let id = ops::new_id();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            library,
            "tag.add",
            vec![Change::create(Entity::Tag, &id, json!({ "name": name }))],
        )
        .unwrap();
        tx.commit().unwrap();
        id
    }

    #[test]
    fn something_in_the_trash_goes_30_days_after_this_device_saw_it_whenever_it_was_sent() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        crate::people::name_founder(&mut conn, &lib, "Tersoal").unwrap();
        let id = tag(&mut conn, &lib, "regalos");
        send(&mut conn, &lib, TrashKind::Tag, &id, "tag.trash").unwrap();
        let month = 31 * DAY_MILLIS;
        let long_ago = timestamp_text(ops::now_millis() - month);
        conn.execute(
            "UPDATE tags SET trashed_at = ?1 WHERE id = ?2",
            [&long_ago, &id],
        )
        .unwrap();
        conn.execute("DELETE FROM trash_seen", []).unwrap();

        // Sent 31 days ago, but this device sees it now: it waits (a wrong clock, a backup restored late).
        assert_eq!(expire(&mut conn, &lib, ops::now_millis()).unwrap(), 0);

        // Seen 31 days ago too: it goes, in an operation of its own.
        conn.execute(
            "UPDATE trash_seen SET seen_at = ?1",
            [i64::try_from(ops::now_millis() - month).unwrap()],
        )
        .unwrap();
        assert_eq!(expire(&mut conn, &lib, ops::now_millis()).unwrap(), 1);
        let left: i64 = conn
            .query_row("SELECT count(*) FROM tags WHERE id = ?1", [&id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(left, 0);
        let intent: String = conn
            .query_row(
                "SELECT intent FROM operations ORDER BY hlc DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(intent, "trash.expire");
        let seen: i64 = conn
            .query_row("SELECT count(*) FROM trash_seen", [], |row| row.get(0))
            .unwrap();
        assert_eq!(seen, 0, "forgotten once gone");
    }

    #[test]
    fn a_tag_or_series_of_that_name_in_the_trash_is_found_to_restore_it() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        crate::people::name_founder(&mut conn, &lib, "Tersoal").unwrap();
        let id = tag(&mut conn, &lib, "Regalos");
        assert!(
            named(&conn, &lib, TrashKind::Tag, "regalos")
                .unwrap()
                .is_none()
        );
        send(&mut conn, &lib, TrashKind::Tag, &id, "tag.trash").unwrap();
        let found = named(&conn, &lib, TrashKind::Tag, " regalos ")
            .unwrap()
            .unwrap();
        assert_eq!((found.id, found.title), (id, "Regalos".to_owned()));
        assert!(
            named(&conn, &lib, TrashKind::Series, "regalos")
                .unwrap()
                .is_none()
        );
    }
}
