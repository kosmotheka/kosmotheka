//! Series (D61): works grouped as a saga (ordered) or a product line (not), and
//! what one work requires of another (an expansion requires its base game).

use kosmotheka_domain::{Change, Entity, ErrorCode, FieldError, Kind};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::catalog;
use crate::error::{Error, Result};
use crate::ops::{self, new_id};

const MAX_NAME_LENGTH: usize = 120;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesSummary {
    pub id: String,
    pub name: String,
    pub ordered: bool,
    /// Distinct works in the series.
    pub works: i64,
    /// Works in the series with no copies.
    pub missing: i64,
    /// Works you have at least one copy of.
    pub have: i64,
    /// What the series holds counting its gaps: the same number the series page shows.
    pub total: i64,
    /// The first covers in the series' order, at most three, so the list shows what it is.
    pub covers: Vec<SeriesCover>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesCover {
    pub file: String,
    pub kind: Kind,
}

/// Covers shown for a series in the list.
const LIST_COVERS: usize = 3;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetail {
    pub id: String,
    pub name: String,
    pub ordered: bool,
    pub entries: Vec<SeriesEntry>,
    pub have: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesEntry {
    pub entry_id: String,
    pub work_id: String,
    /// The work's first edition by title.
    pub edition_id: Option<String>,
    pub title: String,
    pub kind: Kind,
    pub cover: Option<String>,
    pub position: Option<i64>,
    pub copies: i64,
    /// Of the edition shown, like the cover.
    pub published_year: Option<i32>,
    /// The formats of its copies, each once (`paperback`, `ebook`…), for the row's second line.
    pub formats: Vec<String>,
    /// A work it requires and nobody owns (an expansion's base game), for the «needs» notice.
    pub needs: Option<String>,
}

fn validate_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(Error::Validation(vec![FieldError::new(
            "name",
            ErrorCode::Required,
        )]));
    }
    if trimmed.chars().count() > MAX_NAME_LENGTH {
        return Err(Error::Validation(vec![FieldError::new(
            "name",
            ErrorCode::OutOfRange,
        )]));
    }
    Ok(trimmed.to_owned())
}

/// `exclude_id` is the series' own id on rename, so it does not collide with itself.
fn ensure_name_free(
    conn: &Connection,
    library: &str,
    name: &str,
    exclude_id: Option<&str>,
) -> Result<()> {
    let taken: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM series WHERE library_id = ?1 AND name = ?2 COLLATE NOCASE
                           AND id <> ?3)",
        [library, name, exclude_id.unwrap_or("")],
        |row| row.get(0),
    )?;
    if taken {
        return Err(Error::Validation(vec![FieldError::new(
            "name",
            ErrorCode::NameInUse,
        )]));
    }
    Ok(())
}

pub fn create_series(
    conn: &mut Connection,
    library: &str,
    name: &str,
    ordered: bool,
) -> Result<String> {
    let name = validate_name(name)?;
    let tx = conn.transaction()?;
    ensure_name_free(&tx, library, &name, None)?;
    let id = new_id();
    ops::record(
        &tx,
        library,
        "series.add",
        vec![Change::create(
            Entity::Series,
            &id,
            json!({ "name": name, "ordered": ordered }),
        )],
    )?;
    tx.commit()?;
    Ok(id)
}

pub fn update_series(
    conn: &mut Connection,
    library: &str,
    id: &str,
    name: &str,
    ordered: bool,
) -> Result<()> {
    let name = validate_name(name)?;
    let tx = conn.transaction()?;
    let (current_name, current_ordered): (String, bool) = tx
        .query_row(
            "SELECT name, ordered FROM series WHERE library_id = ?1 AND id = ?2",
            [library, id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()?
        .ok_or_else(|| catalog::missing(Entity::Series, id))?;
    if name != current_name {
        ensure_name_free(&tx, library, &name, Some(id))?;
    }

    let mut fields = Map::new();
    if name != current_name {
        fields.insert("name".into(), json!(name));
    }
    if ordered != current_ordered {
        fields.insert("ordered".into(), json!(ordered));
    }
    if !fields.is_empty() {
        ops::record(
            &tx,
            library,
            "series.update",
            vec![Change::update(Entity::Series, id, Value::Object(fields))],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Deletes the series and every entry it has, in one operation.
pub fn delete_series(conn: &mut Connection, library: &str, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    catalog::ensure_exists(&tx, library, Entity::Series, id)?;
    let mut changes: Vec<Change> = catalog::ids(
        &tx,
        "SELECT id FROM series_entries WHERE library_id = ?1 AND series_id = ?2",
        library,
        id,
    )?
    .into_iter()
    .map(|entry_id| Change::delete(Entity::SeriesEntry, entry_id))
    .collect();
    changes.push(Change::delete(Entity::Series, id));
    ops::record(&tx, library, "series.remove", changes)?;
    tx.commit()?;
    Ok(())
}

/// Adding a work already in the series returns the existing entry and changes
/// nothing. In an ordered series with no position given, the next one (max + 1).
pub fn add_to_series(
    conn: &mut Connection,
    library: &str,
    series_id: &str,
    work_id: &str,
    position: Option<i64>,
) -> Result<String> {
    let tx = conn.transaction()?;
    let ordered: bool = tx
        .query_row(
            "SELECT ordered FROM series WHERE library_id = ?1 AND id = ?2",
            [library, series_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .map(|value| value != 0)
        .ok_or_else(|| catalog::missing(Entity::Series, series_id))?;
    catalog::ensure_exists(&tx, library, Entity::Work, work_id)?;

    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM series_entries WHERE library_id = ?1 AND series_id = ?2 AND work_id = ?3
              ORDER BY id LIMIT 1",
            [library, series_id, work_id],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(entry_id) = existing {
        tx.commit()?;
        return Ok(entry_id);
    }

    let position = match position {
        Some(position) => Some(position),
        None if ordered => {
            let max: Option<i64> = tx.query_row(
                "SELECT max(position) FROM series_entries WHERE library_id = ?1 AND series_id = ?2",
                [library, series_id],
                |row| row.get(0),
            )?;
            Some(max.unwrap_or(0) + 1)
        }
        None => None,
    };

    let mut changes = match position {
        Some(position) if ordered => make_room(&tx, library, series_id, position)?,
        _ => Vec::new(),
    };
    let id = new_id();
    changes.push(Change::create(
        Entity::SeriesEntry,
        &id,
        json!({ "series_id": series_id, "work_id": work_id, "position": position }),
    ));
    ops::record(&tx, library, "series.entry_add", changes)?;
    tx.commit()?;
    Ok(id)
}

/// A number already taken makes room for a new work (decided with the maintainer, 25/10/2026): the works from that
/// number on move up one, as far as the first gap, so a later work keeps the number it already had.
fn make_room(
    tx: &Connection,
    library: &str,
    series_id: &str,
    position: i64,
) -> Result<Vec<Change>> {
    let taken: Vec<(String, i64)> = tx
        .prepare(
            "SELECT id, position FROM series_entries
              WHERE library_id = ?1 AND series_id = ?2 AND position >= ?3
              ORDER BY position, id",
        )?
        .query_map(rusqlite::params![library, series_id, position], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut changes = Vec::new();
    let mut next = position;
    for (entry_id, current) in taken {
        // Two works on one number (from before, or from two devices) both move.
        if current > next {
            break;
        }
        if current == next || current + 1 == next {
            next = current + 1;
            changes.push(Change::update(
                Entity::SeriesEntry,
                entry_id,
                json!({ "position": current + 1 }),
            ));
        }
    }
    Ok(changes)
}

/// Deletes every entry of the pair (no unique constraint: two devices may have
/// added the same one offline).
pub fn remove_from_series(
    conn: &mut Connection,
    library: &str,
    series_id: &str,
    work_id: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    let entry_ids: Vec<String> = tx
        .prepare(
            "SELECT id FROM series_entries WHERE library_id = ?1 AND series_id = ?2 AND work_id = ?3",
        )?
        .query_map([library, series_id, work_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if entry_ids.is_empty() {
        tx.commit()?;
        return Ok(());
    }
    let changes = entry_ids
        .into_iter()
        .map(|id| Change::delete(Entity::SeriesEntry, id))
        .collect();
    ops::record(&tx, library, "series.entry_remove", changes)?;
    tx.commit()?;
    Ok(())
}

/// Sets the number of each listed work, as one operation with only the entries
/// whose number changed. Numbers need not be consecutive: a gap is a volume not
/// catalogued yet (D61). `None` takes the work off the numbering. Every row of a
/// work with duplicate entries (converged offline additions) is kept in step.
pub fn reorder(
    conn: &mut Connection,
    library: &str,
    series_id: &str,
    positions: &[(String, Option<i64>)],
) -> Result<()> {
    if positions.iter().any(|(_, p)| p.is_some_and(|p| p < 1)) {
        return Err(Error::Validation(vec![FieldError::new(
            "position",
            ErrorCode::OutOfRange,
        )]));
    }
    let tx = conn.transaction()?;
    catalog::ensure_exists(&tx, library, Entity::Series, series_id)?;
    let mut changes = Vec::new();
    for (work_id, position) in positions {
        let rows: Vec<(String, Option<i64>)> = tx
            .prepare(
                "SELECT id, position FROM series_entries
                  WHERE library_id = ?1 AND series_id = ?2 AND work_id = ?3",
            )?
            .query_map([library, series_id, work_id.as_str()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        for (entry_id, current) in rows {
            if current != *position {
                changes.push(Change::update(
                    Entity::SeriesEntry,
                    entry_id,
                    json!({ "position": position }),
                ));
            }
        }
    }
    if !changes.is_empty() {
        ops::record(&tx, library, "series.reorder", changes)?;
    }
    tx.commit()?;
    Ok(())
}

/// How many entries you have a copy of, out of how many the series holds. An ordered series counts its
/// gaps (positions not logged yet, D61), so the list and the series page say the same.
fn tally(ordered: bool, entries: &[SeriesEntry]) -> (i64, i64) {
    let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    let have = count(entries.iter().filter(|entry| entry.copies > 0).count());
    let mut total = count(entries.len());
    if ordered {
        let highest = entries.iter().filter_map(|entry| entry.position).max();
        total = total.max(highest.unwrap_or(0));
    }
    (have, total)
}

pub fn list_series(conn: &Connection, library: &str) -> Result<Vec<SeriesSummary>> {
    let mut statement = conn.prepare(
        "SELECT s.id, s.name, s.ordered,
                (SELECT count(DISTINCT se.work_id) FROM series_entries se
                  WHERE se.library_id = s.library_id AND se.series_id = s.id),
                (SELECT count(DISTINCT se.work_id) FROM series_entries se
                  WHERE se.library_id = s.library_id AND se.series_id = s.id
                    AND coalesce(
                          (SELECT sum(c.quantity) FROM copies c
                             JOIN editions e ON e.library_id = c.library_id AND e.id = c.edition_id
                            WHERE e.library_id = se.library_id AND e.work_id = se.work_id),
                          0) = 0)
           FROM series s WHERE s.library_id = ?1
          ORDER BY s.name COLLATE NOCASE, s.id",
    )?;
    let rows = statement.query_map([library], |row| {
        Ok(SeriesSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            ordered: row.get::<_, i64>(2)? != 0,
            works: row.get(3)?,
            missing: row.get(4)?,
            have: 0,
            total: 0,
            covers: Vec::new(),
        })
    })?;
    let mut summaries: Vec<SeriesSummary> = rows.collect::<rusqlite::Result<_>>()?;
    for summary in &mut summaries {
        let detail = get_series(conn, library, &summary.id)?;
        summary.have = detail.have;
        summary.total = detail.total;
        summary.covers = detail
            .entries
            .into_iter()
            .filter_map(|entry| {
                entry.cover.map(|file| SeriesCover {
                    file,
                    kind: entry.kind,
                })
            })
            .take(LIST_COVERS)
            .collect();
    }
    Ok(summaries)
}

/// Ordered series by position (entries without one last), unordered by title;
/// deduplicated by work (no unique constraint on `series_entries`).
pub fn get_series(conn: &Connection, library: &str, id: &str) -> Result<SeriesDetail> {
    let (name, ordered): (String, bool) = conn
        .query_row(
            "SELECT name, ordered FROM series WHERE library_id = ?1 AND id = ?2",
            [library, id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()?
        .ok_or_else(|| catalog::missing(Entity::Series, id))?;

    let work_ids: Vec<String> = conn
        .prepare(
            "SELECT DISTINCT work_id FROM series_entries WHERE library_id = ?1 AND series_id = ?2",
        )?
        .query_map([library, id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;

    let mut entries = Vec::with_capacity(work_ids.len());
    for work_id in &work_ids {
        let entry_id: String = conn.query_row(
            "SELECT id FROM series_entries WHERE library_id = ?1 AND series_id = ?2 AND work_id = ?3
              ORDER BY id LIMIT 1",
            [library, id, work_id],
            |row| row.get(0),
        )?;
        let position: Option<i64> = conn.query_row(
            "SELECT position FROM series_entries
              WHERE library_id = ?1 AND series_id = ?2 AND work_id = ?3
              ORDER BY position IS NULL, position LIMIT 1",
            [library, id, work_id],
            |row| row.get(0),
        )?;
        let (title, kind): (String, String) = conn.query_row(
            "SELECT title, kind FROM works WHERE library_id = ?1 AND id = ?2",
            [library, work_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let (edition_id, cover, published_year) = conn
            .query_row(
                "SELECT id, cover, published_year FROM editions WHERE library_id = ?1 AND work_id = ?2
                  ORDER BY title COLLATE NOCASE, id LIMIT 1",
                [library, work_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?
            .unwrap_or((None, None, None));
        let copies: i64 = conn.query_row(
            "SELECT coalesce(sum(c.quantity), 0) FROM copies c
               JOIN editions e ON e.library_id = c.library_id AND e.id = c.edition_id
              WHERE e.library_id = ?1 AND e.work_id = ?2",
            [library, work_id],
            |row| row.get(0),
        )?;
        let formats = conn
            .prepare(
                "SELECT DISTINCT c.format FROM copies c
                   JOIN editions e ON e.library_id = c.library_id AND e.id = c.edition_id
                  WHERE e.library_id = ?1 AND e.work_id = ?2
                  ORDER BY c.format",
            )?
            .query_map([library, work_id], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let needs = conn
            .query_row(
                "SELECT w.title FROM work_relations r
                   JOIN works w ON w.library_id = r.library_id AND w.id = r.related_work_id
                  WHERE r.library_id = ?1 AND r.work_id = ?2 AND r.relation = 'requires'
                    AND NOT EXISTS (
                      SELECT 1 FROM copies c
                        JOIN editions e ON e.library_id = c.library_id AND e.id = c.edition_id
                       WHERE e.library_id = w.library_id AND e.work_id = w.id)
                  ORDER BY w.title COLLATE NOCASE LIMIT 1",
                [library, work_id],
                |row| row.get(0),
            )
            .optional()?;
        entries.push(SeriesEntry {
            entry_id,
            work_id: work_id.clone(),
            edition_id,
            title,
            kind: kind.parse().unwrap_or_default(),
            cover,
            position,
            copies,
            published_year,
            formats,
            needs,
        });
    }
    if ordered {
        entries.sort_by_key(|entry| (entry.position.is_none(), entry.position));
    } else {
        entries.sort_by_key(|entry| entry.title.to_lowercase());
    }
    let (have, total) = tally(ordered, &entries);
    Ok(SeriesDetail {
        id: id.to_owned(),
        name,
        ordered,
        entries,
        have,
        total,
    })
}

/// A work cannot require itself; an existing pair changes nothing.
pub fn add_requirement(
    conn: &mut Connection,
    library: &str,
    work_id: &str,
    required_work_id: &str,
) -> Result<()> {
    if work_id == required_work_id {
        return Err(Error::Validation(vec![FieldError::new(
            "relatedWorkId",
            ErrorCode::SelfReference,
        )]));
    }
    let tx = conn.transaction()?;
    catalog::ensure_exists(&tx, library, Entity::Work, work_id)?;
    catalog::ensure_exists(&tx, library, Entity::Work, required_work_id)?;
    let existing: Option<String> = tx
        .query_row(
            "SELECT id FROM work_relations
              WHERE library_id = ?1 AND work_id = ?2 AND related_work_id = ?3 AND relation = 'requires'",
            [library, work_id, required_work_id],
            |row| row.get(0),
        )
        .optional()?;
    if existing.is_some() {
        tx.commit()?;
        return Ok(());
    }
    let id = new_id();
    ops::record(
        &tx,
        library,
        "work.require",
        vec![Change::create(
            Entity::WorkRelation,
            &id,
            json!({ "work_id": work_id, "related_work_id": required_work_id, "relation": "requires" }),
        )],
    )?;
    tx.commit()?;
    Ok(())
}

/// Deletes every row of the pair (no unique constraint).
pub fn remove_requirement(
    conn: &mut Connection,
    library: &str,
    work_id: &str,
    required_work_id: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    let ids: Vec<String> = tx
        .prepare(
            "SELECT id FROM work_relations
              WHERE library_id = ?1 AND work_id = ?2 AND related_work_id = ?3 AND relation = 'requires'",
        )?
        .query_map([library, work_id, required_work_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if ids.is_empty() {
        tx.commit()?;
        return Ok(());
    }
    let changes = ids
        .into_iter()
        .map(|id| Change::delete(Entity::WorkRelation, id))
        .collect();
    ops::record(&tx, library, "work.unrequire", changes)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};

    use super::*;
    use crate::db;

    fn book(title: &str) -> ValidBook {
        ValidBook {
            edition: EditionInput {
                title: title.into(),
                authors: vec!["Author".into()],
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: Some("Tersoal".into()),
                format: Format::Paperback,
                quantity: 1,
                location: None,
                acquired_on: None,
                platform: None,
                seasons: None,
            }
            .validate()
            .unwrap(),
        }
    }

    fn add_work(conn: &mut Connection, title: &str) -> String {
        let lib = ops::active_library(conn).unwrap();
        let edition_id = catalog::create_book(conn, &lib, &book(title), None)
            .unwrap()
            .edition_id;
        catalog::get_book(conn, &lib, &edition_id).unwrap().work_id
    }

    #[test]
    fn next_position_is_the_highest_plus_one() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let w1 = add_work(&mut conn, "Dune");
        let w2 = add_work(&mut conn, "Dune Messiah");

        add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &w2, None).unwrap();

        let detail = get_series(&conn, &lib, &series_id).unwrap();
        assert_eq!(detail.entries[0].position, Some(1));
        assert_eq!(detail.entries[1].position, Some(2));
    }

    #[test]
    fn adding_an_existing_work_changes_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let w1 = add_work(&mut conn, "Dune");

        let first = add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();
        let second = add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();

        assert_eq!(first, second);
        assert_eq!(
            get_series(&conn, &lib, &series_id).unwrap().entries.len(),
            1
        );
    }

    #[test]
    fn reorder_only_touches_moved_entries() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let w1 = add_work(&mut conn, "Dune");
        let w2 = add_work(&mut conn, "Dune Messiah");
        let w3 = add_work(&mut conn, "Children of Dune");
        add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &w2, None).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &w3, None).unwrap();

        let intent_count_before: i64 = conn
            .query_row("SELECT count(*) FROM operations", [], |r| r.get(0))
            .unwrap();
        // Only w2 and w3 swap; w1 stays at position 1.
        reorder(
            &mut conn,
            &lib,
            &series_id,
            &[
                (w1.clone(), Some(1)),
                (w3.clone(), Some(2)),
                (w2.clone(), Some(3)),
            ],
        )
        .unwrap();
        let intent_count_after: i64 = conn
            .query_row("SELECT count(*) FROM operations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(intent_count_after, intent_count_before + 1);

        let changes: String = conn
            .query_row(
                "SELECT changes FROM operations ORDER BY hlc DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let changes: Vec<Change> = serde_json::from_str(&changes).unwrap();
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn reorder_keeps_the_numbers_typed_gaps_included() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let w1 = add_work(&mut conn, "Dune");
        let w2 = add_work(&mut conn, "Dune Messiah");
        add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &w2, None).unwrap();

        // Volumes 2 to 4 are not catalogued yet: the second book is number 5.
        reorder(
            &mut conn,
            &lib,
            &series_id,
            &[(w1.clone(), Some(1)), (w2.clone(), Some(5))],
        )
        .unwrap();
        let positions: Vec<Option<i64>> = get_series(&conn, &lib, &series_id)
            .unwrap()
            .entries
            .iter()
            .map(|e| e.position)
            .collect();
        assert_eq!(positions, vec![Some(1), Some(5)]);

        let zero = reorder(&mut conn, &lib, &series_id, &[(w1, Some(0))]);
        assert!(matches!(zero, Err(Error::Validation(_))));
    }

    #[test]
    fn deleting_a_series_keeps_its_works() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let w1 = add_work(&mut conn, "Dune");
        add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();

        delete_series(&mut conn, &lib, &series_id).unwrap();

        assert!(get_series(&conn, &lib, &series_id).is_err());
        let title: String = conn
            .query_row("SELECT title FROM works WHERE id = ?1", [&w1], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "Dune");
    }

    #[test]
    fn missing_counts_works_without_copies() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let w1 = add_work(&mut conn, "Dune");
        add_to_series(&mut conn, &lib, &series_id, &w1, None).unwrap();

        // A second work in the series, but with its only copy removed: a gap.
        let edition_id = catalog::create_book(&mut conn, &lib, &book("Dune Messiah"), None)
            .unwrap()
            .edition_id;
        let gap_work = catalog::get_book(&conn, &lib, &edition_id).unwrap().work_id;
        let copy_id = catalog::get_book(&conn, &lib, &edition_id).unwrap().copies[0]
            .id
            .clone();
        catalog::delete_copy(&mut conn, &lib, &copy_id).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &gap_work, None).unwrap();

        let summary = list_series(&conn, &lib)
            .unwrap()
            .into_iter()
            .find(|s| s.id == series_id)
            .unwrap();
        assert_eq!(summary.works, 2);
        assert_eq!(summary.missing, 1);
    }

    #[test]
    fn the_list_and_the_series_count_gaps_the_same() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        // Positions 1, 2 and 5: two gaps; 3 titles, 2 of them with a copy.
        let first = add_work(&mut conn, "Dune");
        let second = add_work(&mut conn, "Dune Messiah");
        add_to_series(&mut conn, &lib, &series_id, &first, Some(1)).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &second, Some(2)).unwrap();
        let edition_id = catalog::create_book(&mut conn, &lib, &book("Heretics"), None)
            .unwrap()
            .edition_id;
        let third = catalog::get_book(&conn, &lib, &edition_id).unwrap().work_id;
        let copy_id = catalog::get_book(&conn, &lib, &edition_id).unwrap().copies[0]
            .id
            .clone();
        catalog::delete_copy(&mut conn, &lib, &copy_id).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &third, Some(5)).unwrap();

        let summary = list_series(&conn, &lib)
            .unwrap()
            .into_iter()
            .find(|s| s.id == series_id)
            .unwrap();
        let detail = get_series(&conn, &lib, &series_id).unwrap();

        assert_eq!((summary.have, summary.total), (2, 5));
        assert_eq!((detail.have, detail.total), (2, 5));
    }

    #[test]
    fn an_unordered_series_total_is_its_works() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Misc", false).unwrap();
        let work = add_work(&mut conn, "Dune");
        add_to_series(&mut conn, &lib, &series_id, &work, None).unwrap();

        let summary = list_series(&conn, &lib).unwrap().remove(0);

        assert_eq!((summary.have, summary.total), (1, 1));
    }

    #[test]
    fn a_work_put_on_a_taken_number_moves_the_following_ones_up_to_the_first_gap() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Mistborn", true).unwrap();
        let [first, second, third, inserted, after_gap] = [
            "The Final Empire",
            "The Well of Ascension",
            "The Hero of Ages",
            "The Alloy of Law",
            "Secret History",
        ]
        .map(|title| add_work(&mut conn, title));
        add_to_series(&mut conn, &lib, &series_id, &first, Some(1)).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &second, Some(2)).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &third, Some(3)).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &after_gap, Some(6)).unwrap();

        add_to_series(&mut conn, &lib, &series_id, &inserted, Some(2)).unwrap();

        let positions: Vec<(String, Option<i64>)> = get_series(&conn, &lib, &series_id)
            .unwrap()
            .entries
            .into_iter()
            .map(|e| (e.work_id, e.position))
            .collect();
        assert_eq!(
            positions,
            [
                (first, Some(1)),
                (inserted, Some(2)),
                (second, Some(3)),
                (third, Some(4)),
                (after_gap, Some(6)),
            ]
        );
    }

    #[test]
    fn a_work_put_in_a_gap_moves_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let [first, third, second] =
            ["Dune", "Children of Dune", "Dune Messiah"].map(|title| add_work(&mut conn, title));
        add_to_series(&mut conn, &lib, &series_id, &first, Some(1)).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &third, Some(3)).unwrap();

        add_to_series(&mut conn, &lib, &series_id, &second, Some(2)).unwrap();

        let positions: Vec<Option<i64>> = get_series(&conn, &lib, &series_id)
            .unwrap()
            .entries
            .iter()
            .map(|e| e.position)
            .collect();
        assert_eq!(positions, [Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn a_work_knows_how_long_its_series_is() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        let messiah = add_work(&mut conn, "Dune Messiah");
        let chapterhouse = add_work(&mut conn, "Chapterhouse: Dune");
        add_to_series(&mut conn, &lib, &series_id, &messiah, Some(2)).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &chapterhouse, Some(6)).unwrap();
        let edition = get_series(&conn, &lib, &series_id).unwrap().entries[0]
            .edition_id
            .clone()
            .unwrap();

        let series = catalog::get_book(&conn, &lib, &edition).unwrap().series;

        assert_eq!((series[0].position, series[0].total), (Some(2), 6));
    }

    #[test]
    fn the_list_shows_the_first_covers_in_the_series_order() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune", true).unwrap();
        for (position, title) in [
            (3, "Children of Dune"),
            (1, "Dune"),
            (2, "Dune Messiah"),
            (4, "God Emperor"),
        ] {
            let cover = format!("{position}.webp");
            let edition_id = catalog::create_book(&mut conn, &lib, &book(title), Some(&cover))
                .unwrap()
                .edition_id;
            let work = catalog::get_book(&conn, &lib, &edition_id).unwrap().work_id;
            add_to_series(&mut conn, &lib, &series_id, &work, Some(position)).unwrap();
        }
        add_work(&mut conn, "No cover");

        let covers: Vec<String> = list_series(&conn, &lib)
            .unwrap()
            .remove(0)
            .covers
            .into_iter()
            .map(|c| c.file)
            .collect();

        assert_eq!(covers, ["1.webp", "2.webp", "3.webp"]);
    }

    #[test]
    fn a_row_says_its_year_its_formats_and_a_base_game_nobody_owns() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let series_id = create_series(&mut conn, &lib, "Dune: Imperium", false).unwrap();
        let mut dated = book("Dune: Imperium");
        dated.edition.published_year = Some(2020);
        let edition = catalog::create_book(&mut conn, &lib, &dated, None)
            .unwrap()
            .edition_id;
        let base = catalog::get_book(&conn, &lib, &edition).unwrap().work_id;
        let expansion = add_work(&mut conn, "Rise of Ix");
        let missing = catalog::create_missing(&mut conn, &lib, &book("Uprising").edition, None)
            .unwrap()
            .edition_id;
        let missing = catalog::get_book(&conn, &lib, &missing).unwrap().work_id;
        add_requirement(&mut conn, &lib, &expansion, &missing).unwrap();
        add_requirement(&mut conn, &lib, &base, &expansion).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &base, None).unwrap();
        add_to_series(&mut conn, &lib, &series_id, &expansion, None).unwrap();

        let detail = get_series(&conn, &lib, &series_id).unwrap();
        let row = |title: &str| detail.entries.iter().find(|e| e.title == title).unwrap();

        assert_eq!(row("Dune: Imperium").published_year, Some(2020));
        assert_eq!(row("Dune: Imperium").formats, ["paperback"]);
        // Its requirement is owned: no notice.
        assert_eq!(row("Dune: Imperium").needs, None);
        assert_eq!(row("Rise of Ix").needs.as_deref(), Some("Uprising"));
    }

    #[test]
    fn self_requirement_is_rejected() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let w1 = add_work(&mut conn, "Dune");

        let error = add_requirement(&mut conn, &lib, &w1, &w1).unwrap_err();
        assert!(
            matches!(error, Error::Validation(errors) if errors[0].code == ErrorCode::SelfReference)
        );
    }
}
