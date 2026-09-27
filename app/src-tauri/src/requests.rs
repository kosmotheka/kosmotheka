//! Requests (D73): operations their author's role did not allow. They stay in the
//! log and travel like any other; an administrator accepts them, in an operation
//! that also makes the requested changes, or rejects them.

use kosmotheka_domain::{Change, Entity, Hlc};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;

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
        requests.push(Request {
            mine: device_id == me || (my_person.is_some() && member == my_person),
            id,
            intent,
            device_id,
            device_name,
            person,
            at: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
            note,
            changes: requested(all),
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
