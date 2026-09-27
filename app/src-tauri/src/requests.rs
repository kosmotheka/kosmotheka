//! Requests (D73): operations their author's role did not allow. They stay in the
//! log and travel like any other; an administrator accepts them, in an operation
//! that also makes the requested changes, or rejects them.

use kosmotheka_domain::{Action, Change, Entity, FieldType, Hlc};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::ops;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// The id of the requested operation.
    pub id: String,
    pub intent: String,
    pub device_id: String,
    /// The author device's name in this library, if it registered one.
    pub device_name: Option<String>,
    /// The person that device belongs to, if any.
    pub person: Option<String>,
    /// Asked from this device or another device of the same person.
    pub mine: bool,
    /// Milliseconds since the Unix epoch, from the operation's clock.
    pub at: u64,
    /// What the asker wrote for whoever decides.
    pub note: Option<String>,
    /// The requested changes, without the row that carries the note.
    pub changes: Vec<Change>,
    pub status: RequestStatus,
    /// Who resolved it (a person's name, else a device's) and when, if resolved.
    pub resolved_by: Option<String>,
    pub resolved_at: Option<u64>,
    /// What was asked for, in words the screens can show.
    pub summary: Summary,
    /// Whether accepting would still work: false once a row it needs is gone.
    pub applies: bool,
    /// A new book, so an administrator can correct it before accepting (D73).
    pub editable: bool,
}

/// A request in words: what kind of change and on which book.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// `add_book`, `add_copy`, `delete_copy`, `delete_book`, `edit`, `add_wish` or `other`.
    pub action: &'static str,
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<i64>,
    pub publisher: Option<String>,
    /// A copy's format code and where it is.
    pub format: Option<String>,
    pub location: Option<String>,
}

/// Every request in the library, newest first.
pub fn list_requests(conn: &Connection, library: &str) -> Result<Vec<Request>> {
    list(conn, library, "")
}

/// This device's own operations that became requests when the library was
/// recalculated, and that it has not shown yet.
pub fn notices(conn: &Connection, library: &str) -> Result<Vec<Request>> {
    list(
        conn,
        library,
        "AND o.id IN (SELECT operation_id FROM request_notices WHERE library_id = o.library_id)",
    )
}

pub fn dismiss_notices(conn: &Connection, library: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM request_notices WHERE library_id = ?1",
        [library],
    )?;
    Ok(())
}

fn list(conn: &Connection, library: &str, only: &str) -> Result<Vec<Request>> {
    let me = ops::device_id(conn)?;
    let my_person: Option<String> = conn
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, &me],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let mut statement = conn.prepare(&format!(
        "SELECT o.id, o.intent, o.hlc, o.changes, o.device_id, d.name, m.name, d.member_id,
                r.status, v.hlc
           FROM operations o
           LEFT JOIN devices d ON d.library_id = o.library_id AND d.id = o.device_id
           LEFT JOIN members m ON m.library_id = o.library_id AND m.id = d.member_id
           LEFT JOIN change_requests r ON r.library_id = o.library_id AND r.id = o.id
           LEFT JOIN field_versions v ON v.library_id = o.library_id
                AND v.entity = 'change_requests' AND v.entity_id = o.id AND v.field = 'status'
          WHERE o.library_id = ?1 AND o.status = 'request' {only}
          ORDER BY o.hlc DESC"
    ))?;
    let rows = statement.query_map([library], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
            row.get::<_, Option<String>>(8)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;
    let mut requests = Vec::new();
    for row in rows {
        let (id, intent, hlc, changes, device_id, device_name, person, member, resolved, when) =
            row?;
        let all: Vec<Change> = serde_json::from_str(&changes)?;
        let note = all
            .iter()
            .find(|c| c.entity == Entity::ChangeRequest && c.id == id)
            .and_then(|c| c.fields.get("note"))
            .and_then(|note| note.as_str())
            .map(str::to_owned);
        let status = match resolved.as_deref() {
            Some("accepted") => RequestStatus::Accepted,
            Some("rejected") => RequestStatus::Rejected,
            Some("withdrawn") => RequestStatus::Withdrawn,
            _ => RequestStatus::Pending,
        };
        let resolver = when.as_deref().and_then(|when| when.parse::<Hlc>().ok());
        let resolved_by = match (&resolver, status) {
            (Some(hlc), s) if s != RequestStatus::Pending => {
                resolver_name(conn, library, &hlc.device)?
            }
            _ => None,
        };
        let asked = requested(all);
        let summary = summarize(conn, library, &asked)?;
        let applies = still_applies(conn, library, &asked)?;
        requests.push(Request {
            editable: summary.action == "add_book",
            summary,
            applies,
            mine: device_id == me || (my_person.is_some() && member == my_person),
            id,
            intent,
            device_id,
            device_name,
            person,
            at: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
            note,
            changes: asked,
            status,
            resolved_at: resolver
                .filter(|_| status != RequestStatus::Pending)
                .map(|hlc| hlc.millis),
            resolved_by,
        });
    }
    Ok(requests)
}

/// A person's name, else the device's own.
fn resolver_name(conn: &Connection, library: &str, device: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT coalesce(m.name, d.name) FROM devices d
               LEFT JOIN members m ON m.library_id = d.library_id AND m.id = d.member_id
              WHERE d.library_id = ?1 AND d.id = ?2",
            [library, device],
            |row| row.get(0),
        )
        .optional()?)
}

/// What was asked for: everything but the row that carries the note.
fn requested(changes: Vec<Change>) -> Vec<Change> {
    changes
        .into_iter()
        .filter(|change| change.entity != Entity::ChangeRequest)
        .collect()
}

/// Accepts a pending request: one operation, by this device, that records the
/// decision and makes the requested changes. It fails, changing nothing, if they
/// no longer apply (a row it edits was deleted since).
pub fn accept(conn: &mut Connection, library: &str, id: &str) -> Result<()> {
    let requested = pending(conn, library, id)?;
    let mut changes = vec![resolution(id, "accepted")];
    changes.extend(requested);
    let tx = conn.transaction()?;
    ops::record(&tx, library, "request.accept", changes)?;
    tx.commit()?;
    Ok(())
}

pub fn reject(conn: &mut Connection, library: &str, id: &str) -> Result<()> {
    resolve(conn, library, id, "request.reject", "rejected")
}

/// Whoever asked takes the request back while it is still pending.
pub fn withdraw(conn: &mut Connection, library: &str, id: &str) -> Result<()> {
    resolve(conn, library, id, "request.withdraw", "withdrawn")
}

fn resolve(
    conn: &mut Connection,
    library: &str,
    id: &str,
    intent: &str,
    status: &str,
) -> Result<()> {
    pending(conn, library, id)?;
    let tx = conn.transaction()?;
    ops::record(&tx, library, intent, vec![resolution(id, status)])?;
    tx.commit()?;
    Ok(())
}

fn resolution(id: &str, status: &str) -> Change {
    Change::create(Entity::ChangeRequest, id, json!({ "status": status }))
}

/// The changes of a request that nobody has resolved yet.
fn pending(conn: &Connection, library: &str, id: &str) -> Result<Vec<Change>> {
    let changes: String = conn
        .query_row(
            "SELECT changes FROM operations
              WHERE library_id = ?1 AND id = ?2 AND status = 'request'",
            [library, id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::MissingRow {
            entity: "requests",
            id: id.to_owned(),
        })?;
    let resolved: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM change_requests
                         WHERE library_id = ?1 AND id = ?2 AND status IS NOT NULL)",
        [library, id],
        |row| row.get(0),
    )?;
    if resolved {
        return Err(Error::RequestResolved);
    }
    Ok(requested(serde_json::from_str(&changes)?))
}

/// Fails unless the request exists and nobody has resolved it yet.
pub fn ensure_pending(conn: &Connection, library: &str, id: &str) -> Result<()> {
    pending(conn, library, id).map(|_| ())
}

fn field_text(change: &Change, name: &str) -> Option<String> {
    change
        .fields
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// A row's text field, looked up in the library as it stands.
fn stored_text(
    conn: &Connection,
    library: &str,
    entity: Entity,
    id: &str,
    field: &str,
) -> Option<String> {
    conn.query_row(
        &format!(
            "SELECT {field} FROM {} WHERE {} = ?1 AND {} = ?2",
            entity.table(),
            entity.library_column(),
            entity.id_column()
        ),
        [library, id],
        |row| row.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
}

/// A name for a row the request refers to: created in it, or already in the library.
fn name_of(
    conn: &Connection,
    library: &str,
    changes: &[Change],
    entity: Entity,
    id: &str,
) -> Option<String> {
    changes
        .iter()
        .find(|c| c.entity == entity && c.id == id)
        .and_then(|c| field_text(c, "name"))
        .or_else(|| stored_text(conn, library, entity, id, "name"))
}

fn summarize(conn: &Connection, library: &str, changes: &[Change]) -> Result<Summary> {
    let find = |entity: Entity, action: Action| {
        changes
            .iter()
            .find(|c| c.entity == entity && c.action == action)
    };
    let edition_title = |id: &str| stored_text(conn, library, Entity::Edition, id, "title");
    let copy_details = |summary: &mut Summary, change: &Change| {
        summary.format = field_text(change, "format")
            .or_else(|| stored_text(conn, library, Entity::Copy, &change.id, "format"));
        summary.location = field_text(change, "location")
            .or_else(|| stored_text(conn, library, Entity::Copy, &change.id, "location"));
    };
    let mut summary = Summary {
        action: "other",
        ..Summary::default()
    };
    if let Some(edition) = find(Entity::Edition, Action::Create) {
        summary.action = "add_book";
        summary.title = field_text(edition, "title");
        summary.year = edition.fields.get("published_year").and_then(Value::as_i64);
        summary.publisher = field_text(edition, "publisher_id")
            .and_then(|id| name_of(conn, library, changes, Entity::Publisher, &id));
        let mut authors: Vec<(i64, String)> = changes
            .iter()
            .filter(|c| c.entity == Entity::WorkContributor && c.action == Action::Create)
            .filter_map(|c| {
                let person = field_text(c, "person_id")?;
                let position = c
                    .fields
                    .get("position")
                    .and_then(Value::as_i64)
                    .unwrap_or(0);
                Some((
                    position,
                    name_of(conn, library, changes, Entity::Person, &person)?,
                ))
            })
            .collect();
        authors.sort();
        summary.authors = authors.into_iter().map(|(_, name)| name).collect();
        if let Some(copy) = find(Entity::Copy, Action::Create) {
            copy_details(&mut summary, copy);
        }
    } else if let Some(copy) = find(Entity::Copy, Action::Create) {
        summary.action = "add_copy";
        summary.title = field_text(copy, "edition_id").and_then(|id| edition_title(&id));
        copy_details(&mut summary, copy);
    } else if let Some(edition) = find(Entity::Edition, Action::Delete) {
        summary.action = "delete_book";
        summary.title = edition_title(&edition.id);
    } else if let Some(copy) = find(Entity::Copy, Action::Delete) {
        summary.action = "delete_copy";
        let edition: Option<String> = conn
            .query_row(
                "SELECT edition_id FROM copies WHERE library_id = ?1 AND id = ?2",
                [library, &copy.id],
                |row| row.get(0),
            )
            .optional()?;
        summary.title = edition.and_then(|id| edition_title(&id));
        copy_details(&mut summary, copy);
    } else if let Some(change) = changes
        .iter()
        .find(|c| c.action == Action::Update && matches!(c.entity, Entity::Edition | Entity::Copy))
    {
        summary.action = "edit";
        summary.title = match change.entity {
            Entity::Edition => edition_title(&change.id),
            _ => stored_text(conn, library, Entity::Copy, &change.id, "edition_id")
                .and_then(|id| edition_title(&id)),
        };
    } else if let Some(wish) = find(Entity::Wish, Action::Create) {
        summary.action = "add_wish";
        summary.title = field_text(wish, "title");
    }
    Ok(summary)
}

/// Whether every row the request edits or deletes, and every row it points to,
/// is still there (or is created by the request itself).
fn still_applies(conn: &Connection, library: &str, changes: &[Change]) -> Result<bool> {
    let created = |entity: Entity, id: &str| {
        changes
            .iter()
            .any(|c| c.entity == entity && c.id == id && c.action == Action::Create)
    };
    let exists = |entity: Entity, id: &str| -> Result<bool> {
        Ok(created(entity, id)
            || conn.query_row(
                &format!(
                    "SELECT EXISTS (SELECT 1 FROM {} WHERE {} = ?1 AND {} = ?2)",
                    entity.table(),
                    entity.library_column(),
                    entity.id_column()
                ),
                [library, id],
                |row| row.get(0),
            )?)
    };
    for change in changes {
        if change.action != Action::Create && !exists(change.entity, &change.id)? {
            return Ok(false);
        }
        for (name, value) in &change.fields {
            let refers = change
                .entity
                .field(name)
                .and_then(|spec| match spec.field_type {
                    FieldType::Ref(target) => Some(target),
                    _ => None,
                });
            if let (Some(target), Some(id)) = (refers, value.as_str())
                && !exists(target, id)?
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// A book request as the add form fills it in, for «Edit and accept». Mirrors the
/// UI's `NewBook`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDraft {
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i64>,
    pub language: Option<String>,
    pub isbn: Option<String>,
    pub cover_url: Option<String>,
    pub google_books_id: Option<String>,
    pub open_library_id: Option<String>,
    pub copy: CopyDraft,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyDraft {
    pub owner: Option<String>,
    pub format: String,
    pub quantity: i64,
    pub location: Option<String>,
    pub acquired_on: Option<String>,
}

pub fn requested_book(conn: &Connection, library: &str, id: &str) -> Result<BookDraft> {
    let changes = pending(conn, library, id)?;
    let summary = summarize(conn, library, &changes)?;
    let find = |entity: Entity| {
        changes
            .iter()
            .find(|c| c.entity == entity && c.action == Action::Create)
    };
    let (Some(edition), Some(copy)) = (find(Entity::Edition), find(Entity::Copy)) else {
        return Err(Error::MissingRow {
            entity: "requests",
            id: id.to_owned(),
        });
    };
    let details: Map<String, Value> = find(Entity::BookDetails)
        .map(|c| c.fields.clone())
        .unwrap_or_default();
    let text = |fields: &Map<String, Value>, name: &str| {
        fields.get(name).and_then(Value::as_str).map(str::to_owned)
    };
    Ok(BookDraft {
        title: summary.title.unwrap_or_default(),
        subtitle: text(&edition.fields, "subtitle"),
        authors: summary.authors,
        publisher: summary.publisher,
        published_year: summary.year,
        language: text(&edition.fields, "language"),
        isbn: text(&details, "isbn13"),
        cover_url: None,
        google_books_id: text(&details, "google_books_id"),
        open_library_id: text(&details, "open_library_id"),
        copy: CopyDraft {
            owner: text(&copy.fields, "owner_id")
                .and_then(|owner| name_of(conn, library, &changes, Entity::Member, &owner)),
            format: text(&copy.fields, "format").unwrap_or_else(|| "paperback".into()),
            quantity: copy
                .fields
                .get("quantity")
                .and_then(Value::as_i64)
                .unwrap_or(1),
            location: text(&copy.fields, "location"),
            acquired_on: text(&copy.fields, "acquired_on"),
        },
    })
}
