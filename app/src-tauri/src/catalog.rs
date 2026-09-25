use std::collections::HashMap;

use library_domain::{
    Change, Entity, ErrorCode, FieldError, Format, Hlc, ValidBook, ValidCopy, ValidEdition,
};
use rusqlite::params_from_iter;
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::ops::{self, new_id};
use crate::search;

// group_concat separator that cannot appear in names typed by users.
const LIST_SEPARATOR: char = '\u{1f}';

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedBook {
    pub edition_id: String,
    /// True when the ISBN was already catalogued and only a new copy was added.
    pub existing_edition: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSummary {
    pub edition_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub isbn13: Option<String>,
    /// File name inside the covers directory.
    pub cover: Option<String>,
    pub copy_count: i64,
    pub formats: Vec<Format>,
    pub owners: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDetail {
    pub edition_id: String,
    pub work_id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub isbn13: Option<String>,
    pub cover: Option<String>,
    pub google_books_id: Option<String>,
    pub open_library_id: Option<String>,
    pub copies: Vec<CopyDetail>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyDetail {
    pub id: String,
    pub owner: Option<String>,
    pub format: Format,
    pub quantity: i64,
    pub location: Option<String>,
    pub acquired_on: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub operation_id: String,
    pub intent: String,
    /// Milliseconds since the Unix epoch, from the operation's HLC.
    pub timestamp: u64,
}

/// `cover` is the file name of an already stored cover, if any.
pub fn create_book(
    conn: &mut Connection,
    book: &ValidBook,
    cover: Option<&str>,
) -> Result<CreatedBook> {
    let tx = conn.transaction()?;
    let mut plan = Plan::new(&tx);

    let existing_edition_id = match &book.edition.isbn {
        Some(isbn) => edition_with_isbn(&tx, isbn.as_str())?,
        None => None,
    };
    let existing_edition = existing_edition_id.is_some();
    let edition_id = match existing_edition_id {
        Some(id) => id,
        None => plan.add_edition(&book.edition, cover)?,
    };
    plan.add_copy(&edition_id, &book.copy)?;

    let intent = if existing_edition {
        "copy.add"
    } else {
        "book.add"
    };
    plan.record(intent)?;
    tx.commit()?;
    Ok(CreatedBook {
        edition_id,
        existing_edition,
    })
}

pub fn update_edition(
    conn: &mut Connection,
    edition_id: &str,
    edition: &ValidEdition,
    new_cover: Option<&str>,
) -> Result<()> {
    let tx = conn.transaction()?;
    let work_id: String = tx
        .query_row(
            "SELECT work_id FROM editions WHERE id = ?1",
            [edition_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| missing(Entity::Edition, edition_id))?;

    if let Some(isbn) = &edition.isbn
        && edition_with_isbn(&tx, isbn.as_str())?.is_some_and(|other| other != edition_id)
    {
        return Err(Error::Validation(vec![FieldError::new(
            "isbn",
            ErrorCode::IsbnInUse,
        )]));
    }

    let mut plan = Plan::new(&tx);
    let publisher_id = plan.publisher_id(edition)?;
    plan.update_if_changed(
        Entity::Edition,
        edition_id,
        json!({
            "title": edition.title,
            "subtitle": edition.subtitle,
            "publisher_id": publisher_id,
            "published_year": edition.published_year,
            "language": edition.language,
        }),
    )?;
    // Without a new cover the current one stays; clearing covers is not supported yet.
    if let Some(cover) = new_cover {
        plan.update_if_changed(Entity::Edition, edition_id, json!({ "cover": cover }))?;
    }
    plan.update_if_changed(
        Entity::BookDetails,
        edition_id,
        book_details_fields(edition),
    )?;

    // A work with a single edition takes its title from it.
    let edition_count: i64 = tx.query_row(
        "SELECT count(*) FROM editions WHERE work_id = ?1",
        [&work_id],
        |row| row.get(0),
    )?;
    if edition_count == 1 {
        plan.update_if_changed(Entity::Work, &work_id, json!({ "title": edition.title }))?;
    }
    plan.sync_authors(&work_id, &edition.authors)?;

    plan.record("edition.edit")?;
    tx.commit()?;
    Ok(())
}

pub fn add_copy(conn: &mut Connection, edition_id: &str, copy: &ValidCopy) -> Result<String> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, Entity::Edition, edition_id)?;
    let mut plan = Plan::new(&tx);
    let copy_id = plan.add_copy(edition_id, copy)?;
    plan.record("copy.add")?;
    tx.commit()?;
    Ok(copy_id)
}

pub fn update_copy(conn: &mut Connection, copy_id: &str, copy: &ValidCopy) -> Result<()> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, Entity::Copy, copy_id)?;
    let mut plan = Plan::new(&tx);
    let owner_id = plan.owner_id(copy)?;
    plan.update_if_changed(Entity::Copy, copy_id, copy_fields(copy, owner_id))?;
    plan.record("copy.edit")?;
    tx.commit()?;
    Ok(())
}

pub fn delete_copy(conn: &mut Connection, copy_id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    ensure_exists(&tx, Entity::Copy, copy_id)?;
    ops::record(
        &tx,
        "copy.remove",
        vec![Change::delete(Entity::Copy, copy_id)],
    )?;
    tx.commit()?;
    Ok(())
}

/// Removes the edition with its copies, and the work too if no other edition remains.
/// People and members stay: they are catalogues of their own.
pub fn delete_book(conn: &mut Connection, edition_id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    let work_id: String = tx
        .query_row(
            "SELECT work_id FROM editions WHERE id = ?1",
            [edition_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| missing(Entity::Edition, edition_id))?;

    // Children before parents, so every change finds its row.
    let mut changes: Vec<Change> = ids(
        &tx,
        "SELECT id FROM copies WHERE edition_id = ?1",
        edition_id,
    )?
    .into_iter()
    .map(|id| Change::delete(Entity::Copy, id))
    .collect();
    if exists(&tx, Entity::BookDetails, edition_id)? {
        changes.push(Change::delete(Entity::BookDetails, edition_id));
    }
    changes.push(Change::delete(Entity::Edition, edition_id));

    let other_editions: i64 = tx.query_row(
        "SELECT count(*) FROM editions WHERE work_id = ?1 AND id <> ?2",
        [&work_id, edition_id],
        |row| row.get(0),
    )?;
    if other_editions == 0 {
        changes.extend(
            ids(
                &tx,
                "SELECT id FROM work_contributors WHERE work_id = ?1",
                &work_id,
            )?
            .into_iter()
            .map(|id| Change::delete(Entity::WorkContributor, id)),
        );
        changes.push(Change::delete(Entity::Work, &work_id));
    }

    ops::record(&tx, "book.remove", changes)?;
    tx.commit()?;
    Ok(())
}

/// Collects the changes of one operation, reusing rows that already exist.
struct Plan<'a> {
    tx: &'a Transaction<'a>,
    changes: Vec<Change>,
    /// Rows created earlier in this same plan, so a repeated name is created once.
    planned: HashMap<(Entity, String), String>,
}

impl<'a> Plan<'a> {
    fn new(tx: &'a Transaction<'a>) -> Self {
        Self {
            tx,
            changes: Vec::new(),
            planned: HashMap::new(),
        }
    }

    /// Records the collected changes as one operation; does nothing if there are none.
    fn record(self, intent: &str) -> Result<()> {
        if !self.changes.is_empty() {
            ops::record(self.tx, intent, self.changes)?;
        }
        Ok(())
    }

    fn add_edition(&mut self, edition: &ValidEdition, cover: Option<&str>) -> Result<String> {
        let work_id = new_id();
        self.changes.push(Change::create(
            Entity::Work,
            &work_id,
            json!({ "kind": "book", "title": edition.title }),
        ));
        self.sync_authors(&work_id, &edition.authors)?;
        let publisher_id = self.publisher_id(edition)?;

        let edition_id = new_id();
        self.changes.push(Change::create(
            Entity::Edition,
            &edition_id,
            json!({
                "work_id": work_id,
                "title": edition.title,
                "subtitle": edition.subtitle,
                "publisher_id": publisher_id,
                "published_year": edition.published_year,
                "language": edition.language,
                "cover": cover,
            }),
        ));
        self.changes.push(Change::create(
            Entity::BookDetails,
            &edition_id,
            book_details_fields(edition),
        ));
        Ok(edition_id)
    }

    fn add_copy(&mut self, edition_id: &str, copy: &ValidCopy) -> Result<String> {
        let owner_id = self.owner_id(copy)?;
        let mut fields = copy_fields(copy, owner_id);
        fields["edition_id"] = json!(edition_id);
        let copy_id = new_id();
        self.changes
            .push(Change::create(Entity::Copy, &copy_id, fields));
        Ok(copy_id)
    }

    fn publisher_id(&mut self, edition: &ValidEdition) -> Result<Option<String>> {
        edition
            .publisher
            .as_deref()
            .map(|name| self.find_or_create(Entity::Publisher, name))
            .transpose()
    }

    fn owner_id(&mut self, copy: &ValidCopy) -> Result<Option<String>> {
        copy.owner
            .as_deref()
            .map(|name| self.find_or_create(Entity::Member, name))
            .transpose()
    }

    /// Makes the work's author list match `names`, in order: adds, moves and removes
    /// contributor rows as needed. Works for new works too (nothing to compare against).
    fn sync_authors(&mut self, work_id: &str, names: &[String]) -> Result<()> {
        let mut current: HashMap<String, (String, i64)> = HashMap::new();
        {
            let mut statement = self.tx.prepare(
                "SELECT person_id, id, position FROM work_contributors
                  WHERE work_id = ?1 AND role = 'author'",
            )?;
            let rows = statement.query_map([work_id], |row| {
                Ok((row.get(0)?, (row.get(1)?, row.get(2)?)))
            })?;
            for row in rows {
                let (person_id, entry) = row?;
                current.insert(person_id, entry);
            }
        }

        for (position, name) in names.iter().enumerate() {
            let position = position as i64;
            let person_id = self.find_or_create(Entity::Person, name)?;
            match current.remove(&person_id) {
                Some((_, old_position)) if old_position == position => {}
                Some((contributor_id, _)) => self.changes.push(Change::update(
                    Entity::WorkContributor,
                    contributor_id,
                    json!({ "position": position }),
                )),
                None => self.changes.push(Change::create(
                    Entity::WorkContributor,
                    new_id(),
                    json!({ "work_id": work_id, "person_id": person_id, "role": "author", "position": position }),
                )),
            }
        }
        for (contributor_id, _) in current.into_values() {
            self.changes
                .push(Change::delete(Entity::WorkContributor, contributor_id));
        }
        Ok(())
    }

    /// For entities identified by a `name` (people, members), matched case-insensitively.
    fn find_or_create(&mut self, entity: Entity, name: &str) -> Result<String> {
        let key = (entity, name.to_lowercase());
        if let Some(id) = self.planned.get(&key) {
            return Ok(id.clone());
        }
        let table = entity.table();
        let existing: Option<String> = self
            .tx
            .query_row(
                &format!(
                    "SELECT id FROM {table} WHERE name = ?1 COLLATE NOCASE ORDER BY id LIMIT 1"
                ),
                [name],
                |row| row.get(0),
            )
            .optional()?;
        let id = match existing {
            Some(id) => id,
            None => {
                let id = new_id();
                self.changes
                    .push(Change::create(entity, &id, json!({ "name": name })));
                id
            }
        };
        self.planned.insert(key, id.clone());
        Ok(id)
    }

    /// Adds an update with only the fields whose value differs from the stored row.
    /// Fewer fields per change means fewer conflicts when devices sync (D13).
    fn update_if_changed(&mut self, entity: Entity, id: &str, desired: Value) -> Result<()> {
        let Value::Object(desired) = desired else {
            return Ok(());
        };
        let columns: Vec<&str> = desired
            .keys()
            .filter_map(|name| entity.field(name).map(|spec| spec.name))
            .collect();
        let current: Vec<Value> = self
            .tx
            .query_row(
                &format!(
                    "SELECT {} FROM {} WHERE {} = ?1",
                    columns.join(", "),
                    entity.table(),
                    entity.id_column()
                ),
                [id],
                |row| {
                    (0..columns.len())
                        .map(|i| row.get_ref(i).map(to_json))
                        .collect()
                },
            )
            .optional()?
            .ok_or_else(|| missing(entity, id))?;

        let changed: Map<String, Value> = columns
            .iter()
            .zip(current)
            .filter(|(column, old)| desired[**column] != *old)
            .map(|(column, _)| (column.to_string(), desired[*column].clone()))
            .collect();
        if !changed.is_empty() {
            self.changes
                .push(Change::update(entity, id, Value::Object(changed)));
        }
        Ok(())
    }
}

fn copy_fields(copy: &ValidCopy, owner_id: Option<String>) -> Value {
    json!({
        "owner_id": owner_id,
        "format": copy.format.as_str(),
        "quantity": copy.quantity,
        "location": copy.location,
        "acquired_on": copy.acquired_on.map(|date| date.to_string()),
    })
}

fn to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => json!(String::from_utf8_lossy(bytes)),
    }
}

fn book_details_fields(edition: &ValidEdition) -> Value {
    json!({
        "isbn13": edition.isbn.as_ref().map(|isbn| isbn.as_str()),
        "google_books_id": edition.google_books_id,
        "open_library_id": edition.open_library_id,
    })
}

pub fn edition_with_isbn(conn: &Connection, isbn13: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT edition_id FROM book_details WHERE isbn13 = ?1 ORDER BY edition_id LIMIT 1",
            [isbn13],
            |row| row.get(0),
        )
        .optional()?)
}

fn exists(conn: &Connection, entity: Entity, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT 1 FROM {} WHERE {} = ?1",
                entity.table(),
                entity.id_column()
            ),
            [id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn ensure_exists(conn: &Connection, entity: Entity, id: &str) -> Result<()> {
    if exists(conn, entity, id)? {
        Ok(())
    } else {
        Err(missing(entity, id))
    }
}

fn missing(entity: Entity, id: &str) -> Error {
    Error::MissingRow {
        entity: entity.table(),
        id: id.to_owned(),
    }
}

fn ids(conn: &Connection, sql: &str, param: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([param], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Search text and copy filters; empty means "everything".
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BookFilter {
    pub query: Option<String>,
    pub format: Option<Format>,
    /// Owner name: duplicates of the same member created on other devices match too (D33).
    pub owner: Option<String>,
    pub location: Option<String>,
}

pub fn list_books(conn: &Connection, filter: &BookFilter) -> Result<Vec<BookSummary>> {
    let mut conditions: Vec<&str> = Vec::new();
    let mut args: Vec<SqlValue> = Vec::new();

    if let Some(query) = filter.query.as_deref().and_then(search::fts_query) {
        conditions.push("e.id IN (SELECT edition_id FROM search_index WHERE search_index MATCH ?)");
        args.push(SqlValue::Text(query));
    }
    // One copy must match every copy filter at once.
    if filter.format.is_some() || filter.owner.is_some() || filter.location.is_some() {
        conditions.push(
            "EXISTS (SELECT 1 FROM copies c WHERE c.edition_id = e.id
                      AND (? IS NULL OR c.format = ?)
                      AND (? IS NULL OR c.owner_id IN (SELECT id FROM members WHERE name = ? COLLATE NOCASE))
                      AND (? IS NULL OR c.location = ?))",
        );
        let text =
            |value: Option<&str>| value.map_or(SqlValue::Null, |v| SqlValue::Text(v.to_owned()));
        for value in [
            text(filter.format.map(Format::as_str)),
            text(filter.owner.as_deref()),
            text(filter.location.as_deref()),
        ] {
            args.push(value.clone());
            args.push(value);
        }
    }
    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let mut statement = conn.prepare(&format!(
        "SELECT e.id, e.title, e.subtitle, pu.name, e.published_year, bd.isbn13, e.cover,
            (SELECT group_concat(p.name, char(31) ORDER BY wc.position)
               FROM work_contributors wc JOIN people p ON p.id = wc.person_id
              WHERE wc.work_id = e.work_id AND wc.role = 'author'),
            (SELECT coalesce(sum(c.quantity), 0) FROM copies c WHERE c.edition_id = e.id),
            (SELECT group_concat(format, char(31))
               FROM (SELECT DISTINCT c.format FROM copies c WHERE c.edition_id = e.id)),
            (SELECT group_concat(name, char(31) ORDER BY name COLLATE NOCASE)
               FROM (SELECT DISTINCT m.name FROM copies c JOIN members m ON m.id = c.owner_id
                      WHERE c.edition_id = e.id))
         FROM editions e
         LEFT JOIN book_details bd ON bd.edition_id = e.id
         LEFT JOIN publishers pu ON pu.id = e.publisher_id
         {where_clause}
         ORDER BY e.title COLLATE NOCASE, e.id"
    ))?;
    let rows = statement.query_map(params_from_iter(args), |row| {
        Ok(BookSummary {
            edition_id: row.get(0)?,
            title: row.get(1)?,
            subtitle: row.get(2)?,
            publisher: row.get(3)?,
            published_year: row.get(4)?,
            isbn13: row.get(5)?,
            cover: row.get(6)?,
            authors: split_list(row.get(7)?),
            copy_count: row.get(8)?,
            formats: split_list(row.get(9)?)
                .iter()
                .filter_map(|format| format.parse().ok())
                .collect(),
            owners: split_list(row.get(10)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Values already in the catalog, for autocompletion and filters.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogOptions {
    pub authors: Vec<String>,
    pub publishers: Vec<String>,
    pub owners: Vec<String>,
    pub locations: Vec<String>,
}

pub fn catalog_options(conn: &Connection) -> Result<CatalogOptions> {
    let names = |sql: &str| -> Result<Vec<String>> {
        Ok(conn
            .prepare(sql)?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    };
    // Each name once, even if devices created it separately (D33).
    let unique = |table: &str| {
        names(&format!(
            "SELECT min(name) FROM {table} GROUP BY name COLLATE NOCASE ORDER BY 1 COLLATE NOCASE"
        ))
    };
    Ok(CatalogOptions {
        authors: unique("people")?,
        publishers: unique("publishers")?,
        owners: unique("members")?,
        locations: names(
            "SELECT DISTINCT location FROM copies WHERE location IS NOT NULL ORDER BY location COLLATE NOCASE",
        )?,
    })
}

pub fn get_book(conn: &Connection, edition_id: &str) -> Result<BookDetail> {
    let mut book = conn
        .query_row(
            "SELECT e.id, e.work_id, e.title, e.subtitle, pu.name, e.published_year, e.language,
                    bd.isbn13, e.cover, bd.google_books_id, bd.open_library_id,
                    (SELECT group_concat(p.name, char(31) ORDER BY wc.position)
                       FROM work_contributors wc JOIN people p ON p.id = wc.person_id
                      WHERE wc.work_id = e.work_id AND wc.role = 'author')
               FROM editions e
               LEFT JOIN book_details bd ON bd.edition_id = e.id
               LEFT JOIN publishers pu ON pu.id = e.publisher_id
              WHERE e.id = ?1",
            [edition_id],
            |row| {
                Ok(BookDetail {
                    edition_id: row.get(0)?,
                    work_id: row.get(1)?,
                    title: row.get(2)?,
                    subtitle: row.get(3)?,
                    publisher: row.get(4)?,
                    published_year: row.get(5)?,
                    language: row.get(6)?,
                    isbn13: row.get(7)?,
                    cover: row.get(8)?,
                    google_books_id: row.get(9)?,
                    open_library_id: row.get(10)?,
                    authors: split_list(row.get(11)?),
                    copies: Vec::new(),
                })
            },
        )
        .optional()?
        .ok_or_else(|| missing(Entity::Edition, edition_id))?;

    let mut statement = conn.prepare(
        "SELECT c.id, m.name, c.format, c.quantity, c.location, c.acquired_on
           FROM copies c LEFT JOIN members m ON m.id = c.owner_id
          WHERE c.edition_id = ?1
          ORDER BY c.created_at, c.id",
    )?;
    let rows = statement.query_map([edition_id], |row| {
        let format: String = row.get(2)?;
        Ok(CopyDetail {
            id: row.get(0)?,
            owner: row.get(1)?,
            format: format.parse().unwrap_or(Format::Other),
            quantity: row.get(3)?,
            location: row.get(4)?,
            acquired_on: row.get(5)?,
        })
    })?;
    book.copies = rows.collect::<rusqlite::Result<_>>()?;
    Ok(book)
}

/// Operations that touched the edition, its work or any of its copies, oldest first.
pub fn book_history(conn: &Connection, edition_id: &str) -> Result<Vec<HistoryEntry>> {
    let mut statement = conn.prepare(
        "WITH related (id) AS (
             SELECT ?1
             UNION SELECT work_id FROM editions WHERE id = ?1
             UNION SELECT id FROM copies WHERE edition_id = ?1
         )
         SELECT DISTINCT o.id, o.intent, o.hlc
           FROM operations o, json_each(o.changes) AS change
          WHERE json_extract(change.value, '$.id') IN (SELECT id FROM related)
          ORDER BY o.hlc",
    )?;
    let rows = statement.query_map([edition_id], |row| {
        let hlc: String = row.get(2)?;
        Ok(HistoryEntry {
            operation_id: row.get(0)?,
            intent: row.get(1)?,
            timestamp: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn split_list(joined: Option<String>) -> Vec<String> {
    joined
        .map(|s| s.split(LIST_SEPARATOR).map(str::to_owned).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use library_domain::{CopyInput, EditionInput, NewBook};

    use super::*;
    use crate::db;

    fn edition(title: &str, isbn: Option<&str>, authors: &[&str]) -> ValidEdition {
        EditionInput {
            title: title.to_owned(),
            authors: authors.iter().map(|a| a.to_string()).collect(),
            published_year: Some(2007),
            language: Some("es".to_owned()),
            isbn: isbn.map(str::to_owned),
            ..EditionInput::default()
        }
        .validate()
        .unwrap()
    }

    fn copy(owner: &str, format: Format) -> ValidCopy {
        CopyInput {
            owner: Some(owner.to_owned()),
            format,
            quantity: 1,
            location: None,
            acquired_on: None,
        }
        .validate()
        .unwrap()
    }

    fn new_book(title: &str, isbn: Option<&str>, owner: &str, format: Format) -> ValidBook {
        ValidBook {
            edition: edition(title, isbn, &["Patrick Rothfuss"]),
            copy: copy(owner, format),
        }
    }

    fn intents(conn: &Connection) -> Vec<String> {
        ids(
            conn,
            "SELECT intent FROM operations WHERE ?1 = ?1 ORDER BY hlc",
            "",
        )
        .unwrap()
    }

    fn last_changes(conn: &Connection) -> Vec<Change> {
        let json: String = conn
            .query_row(
                "SELECT changes FROM operations ORDER BY hlc DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    #[test]
    fn creates_work_edition_and_copy() {
        let mut conn = db::open_in_memory().unwrap();
        let book = new_book(
            "El nombre del viento",
            Some("0306406152"),
            "Tersoal",
            Format::Hardcover,
        );

        let created = create_book(&mut conn, &book, None).unwrap();
        let books = list_books(&conn, &BookFilter::default()).unwrap();

        assert!(!created.existing_edition);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].title, "El nombre del viento");
        assert_eq!(books[0].authors, vec!["Patrick Rothfuss"]);
        assert_eq!(books[0].isbn13.as_deref(), Some("9780306406157"));
        assert_eq!(books[0].formats, vec![Format::Hardcover]);
        assert_eq!(books[0].owners, vec!["Tersoal"]);
    }

    #[test]
    fn same_isbn_adds_a_copy_to_the_existing_edition() {
        let mut conn = db::open_in_memory().unwrap();
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        let second = create_book(
            &mut conn,
            &new_book("A", Some("0-306-40615-2"), "tersoal", Format::Ebook),
            None,
        )
        .unwrap();
        let books = list_books(&conn, &BookFilter::default()).unwrap();

        assert!(second.existing_edition);
        assert_eq!(books.len(), 1);
        assert_eq!(books[0].copy_count, 2);
        assert_eq!(
            books[0].owners,
            vec!["Tersoal"],
            "owners match case-insensitively"
        );
    }

    #[test]
    fn books_without_isbn_are_never_merged() {
        let mut conn = db::open_in_memory().unwrap();
        create_book(
            &mut conn,
            &new_book("B", None, "Tersoal", Format::Paperback),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &new_book("B", None, "Tersoal", Format::Paperback),
            None,
        )
        .unwrap();

        assert_eq!(list_books(&conn, &BookFilter::default()).unwrap().len(), 2);
    }

    #[test]
    fn every_write_is_logged_and_history_follows_the_edition() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Ebook),
            None,
        )
        .unwrap();
        create_book(
            &mut conn,
            &new_book("Other", None, "Tersoal", Format::Ebook),
            None,
        )
        .unwrap();

        let history = book_history(&conn, &created.edition_id).unwrap();

        let intents: Vec<&str> = history.iter().map(|entry| entry.intent.as_str()).collect();
        assert_eq!(intents, vec!["book.add", "copy.add"]);
        assert!(history[0].timestamp <= history[1].timestamp);
    }

    #[test]
    fn get_book_returns_edition_and_copies() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        add_copy(&mut conn, &created.edition_id, &copy("Ana", Format::Ebook)).unwrap();

        let book = get_book(&conn, &created.edition_id).unwrap();

        assert_eq!(book.title, "A");
        assert_eq!(book.language.as_deref(), Some("es"));
        assert_eq!(book.copies.len(), 2);
        assert_eq!(book.copies[1].owner.as_deref(), Some("Ana"));
        assert_eq!(book.copies[1].format, Format::Ebook);
    }

    #[test]
    fn editing_records_only_changed_fields() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let mut edited = edition("A", None, &["Patrick Rothfuss"]);
        edited.published_year = Some(2011);

        update_edition(&mut conn, &created.edition_id, &edited, None).unwrap();

        let changes = last_changes(&conn);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].entity, Entity::Edition);
        assert_eq!(
            Value::Object(changes[0].fields.clone()),
            json!({ "published_year": 2011 })
        );
        assert_eq!(
            get_book(&conn, &created.edition_id).unwrap().published_year,
            Some(2011)
        );
    }

    #[test]
    fn editing_without_changes_records_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        update_edition(
            &mut conn,
            &created.edition_id,
            &edition("A", None, &["Patrick Rothfuss"]),
            None,
        )
        .unwrap();

        assert_eq!(intents(&conn), vec!["book.add"]);
    }

    #[test]
    fn editing_the_title_of_a_single_edition_renames_its_work() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("Old", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        update_edition(
            &mut conn,
            &created.edition_id,
            &edition("New", None, &["Patrick Rothfuss"]),
            None,
        )
        .unwrap();

        let work_title: String = conn
            .query_row("SELECT title FROM works", [], |r| r.get(0))
            .unwrap();
        assert_eq!(work_title, "New");
    }

    #[test]
    fn editing_authors_adds_reorders_and_removes() {
        let mut conn = db::open_in_memory().unwrap();
        let mut book = new_book("A", None, "Tersoal", Format::Hardcover);
        book.edition.authors = vec!["Ana".into(), "Bea".into(), "Carla".into()];
        let created = create_book(&mut conn, &book, None).unwrap();

        update_edition(
            &mut conn,
            &created.edition_id,
            &edition("A", None, &["Carla", "Ana", "Dora"]),
            None,
        )
        .unwrap();

        assert_eq!(
            get_book(&conn, &created.edition_id).unwrap().authors,
            vec!["Carla", "Ana", "Dora"]
        );
        let people: i64 = conn
            .query_row("SELECT count(*) FROM people", [], |r| r.get(0))
            .unwrap();
        assert_eq!(people, 4, "removed authors stay in the people catalogue");
    }

    #[test]
    fn isbn_of_another_edition_is_rejected() {
        let mut conn = db::open_in_memory().unwrap();
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let second = create_book(
            &mut conn,
            &new_book("B", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        let error = update_edition(
            &mut conn,
            &second.edition_id,
            &edition("B", Some("9780306406157"), &[]),
            None,
        )
        .unwrap_err();

        assert!(
            matches!(error, Error::Validation(errors) if errors[0].code == ErrorCode::IsbnInUse)
        );
    }

    #[test]
    fn copies_can_be_edited_and_removed() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        let copy_id = get_book(&conn, &created.edition_id).unwrap().copies[0]
            .id
            .clone();

        let mut moved = copy("Tersoal", Format::Hardcover);
        moved.location = Some("Salón, balda 2".into());
        update_copy(&mut conn, &copy_id, &moved).unwrap();
        assert_eq!(
            Value::Object(last_changes(&conn)[0].fields.clone()),
            json!({ "location": "Salón, balda 2" })
        );

        delete_copy(&mut conn, &copy_id).unwrap();
        assert!(
            get_book(&conn, &created.edition_id)
                .unwrap()
                .copies
                .is_empty()
        );
        assert_eq!(intents(&conn), vec!["book.add", "copy.edit", "copy.remove"]);
    }

    #[test]
    fn deleting_a_book_removes_everything_but_people_and_members() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
        add_copy(&mut conn, &created.edition_id, &copy("Ana", Format::Ebook)).unwrap();

        delete_book(&mut conn, &created.edition_id).unwrap();

        let count = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        for table in [
            "works",
            "work_contributors",
            "editions",
            "book_details",
            "copies",
        ] {
            assert_eq!(count(table), 0, "{table} should be empty");
        }
        assert_eq!(count("people"), 1);
        assert_eq!(count("members"), 2);
        // The same ISBN can be catalogued again.
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();
    }

    #[test]
    fn missing_rows_are_reported() {
        let mut conn = db::open_in_memory().unwrap();
        let unknown = new_id();
        assert!(matches!(
            delete_book(&mut conn, &unknown),
            Err(Error::MissingRow { .. })
        ));
        assert!(matches!(
            delete_copy(&mut conn, &unknown),
            Err(Error::MissingRow { .. })
        ));
        assert!(matches!(
            get_book(&conn, &unknown),
            Err(Error::MissingRow { .. })
        ));
    }

    #[test]
    fn new_book_deserializes_from_ui_json() {
        let book: NewBook = serde_json::from_value(json!({
            "title": "Dune", "copy": { "format": "pocket" }
        }))
        .unwrap();
        assert_eq!(book.validate().unwrap().edition.title, "Dune");
    }

    fn search(conn: &Connection, query: &str) -> Vec<String> {
        let filter = BookFilter {
            query: Some(query.into()),
            ..BookFilter::default()
        };
        list_books(conn, &filter)
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect()
    }

    #[test]
    fn search_ignores_accents_and_case_and_matches_prefixes() {
        let mut conn = db::open_in_memory().unwrap();
        let mut book = new_book(
            "El señor de los anillos",
            None,
            "Tersoal",
            Format::Hardcover,
        );
        book.edition.authors = vec!["J. R. R. Tolkien".into()];
        book.edition.publisher = Some("Minotauro".into());
        create_book(&mut conn, &book, None).unwrap();
        create_book(
            &mut conn,
            &new_book("Dune", None, "Tersoal", Format::Paperback),
            None,
        )
        .unwrap();

        assert_eq!(search(&conn, "SENOR"), vec!["El señor de los anillos"]);
        assert_eq!(search(&conn, "tolk anill"), vec!["El señor de los anillos"]);
        assert_eq!(search(&conn, "minotauro"), vec!["El señor de los anillos"]);
        assert!(
            search(&conn, "tolkien dune").is_empty(),
            "every word must match"
        );
        assert_eq!(
            search(&conn, "   ").len(),
            2,
            "an empty query lists everything"
        );
    }

    #[test]
    fn search_finds_isbn_in_either_format() {
        let mut conn = db::open_in_memory().unwrap();
        create_book(
            &mut conn,
            &new_book("A", Some("9780306406157"), "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        assert_eq!(search(&conn, "0-306-40615-2"), vec!["A"]);
        assert_eq!(search(&conn, "978-0-306-40615-7"), vec!["A"]);
    }

    #[test]
    fn index_follows_edits_renames_and_deletions() {
        let mut conn = db::open_in_memory().unwrap();
        let created = create_book(
            &mut conn,
            &new_book("Old title", None, "Tersoal", Format::Hardcover),
            None,
        )
        .unwrap();

        update_edition(
            &mut conn,
            &created.edition_id,
            &edition("New title", None, &["Ursula"]),
            None,
        )
        .unwrap();
        assert!(search(&conn, "old").is_empty());
        assert_eq!(search(&conn, "ursula"), vec!["New title"]);

        // Renaming a person reindexes their books.
        let person_id: String = conn
            .query_row("SELECT id FROM people WHERE name = 'Ursula'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            "person.rename",
            vec![Change::update(
                Entity::Person,
                &person_id,
                json!({ "name": "Le Guin" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        assert_eq!(search(&conn, "guin"), vec!["New title"]);

        delete_book(&mut conn, &created.edition_id).unwrap();
        assert!(search(&conn, "new").is_empty());
        let rows: i64 = conn
            .query_row("SELECT count(*) FROM search_index", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 0);
    }

    #[test]
    fn copy_filters_need_one_copy_matching_all_of_them() {
        let mut conn = db::open_in_memory().unwrap();
        let mut a = new_book("A", None, "Ana", Format::Hardcover);
        a.copy.location = Some("Salón".into());
        create_book(&mut conn, &a, None).unwrap();
        let b = create_book(&mut conn, &new_book("B", None, "Ana", Format::Ebook), None).unwrap();
        add_copy(&mut conn, &b.edition_id, &copy("Bea", Format::Hardcover)).unwrap();
        let ana = "ana".to_owned();

        let titles = |filter: BookFilter| -> Vec<String> {
            list_books(&conn, &filter)
                .unwrap()
                .into_iter()
                .map(|b| b.title)
                .collect()
        };
        assert_eq!(
            titles(BookFilter {
                format: Some(Format::Hardcover),
                ..Default::default()
            }),
            vec!["A", "B"]
        );
        assert_eq!(
            titles(BookFilter {
                owner: Some(ana.clone()),
                ..Default::default()
            }),
            vec!["A", "B"]
        );
        assert_eq!(
            titles(BookFilter {
                format: Some(Format::Hardcover),
                owner: Some(ana),
                ..Default::default()
            }),
            vec!["A"],
            "B has a hardcover (Bea's) and a copy of Ana's (ebook), but no hardcover of Ana's"
        );
        assert_eq!(
            titles(BookFilter {
                location: Some("Salón".into()),
                ..Default::default()
            }),
            vec!["A"]
        );
    }

    #[test]
    fn publishers_are_a_shared_catalogue() {
        let mut conn = db::open_in_memory().unwrap();
        for (title, publisher) in [("A", "Minotauro"), ("B", "minotauro")] {
            let mut book = new_book(title, None, "Tersoal", Format::Paperback);
            book.edition.publisher = Some(publisher.into());
            create_book(&mut conn, &book, None).unwrap();
        }

        let options = catalog_options(&conn).unwrap();

        assert_eq!(options.publishers, vec!["Minotauro"]);
        let books = list_books(&conn, &BookFilter::default()).unwrap();
        assert!(
            books
                .iter()
                .all(|b| b.publisher.as_deref() == Some("Minotauro"))
        );
    }
}
