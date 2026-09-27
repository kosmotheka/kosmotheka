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
    /// Milliseconds since the Unix epoch, from the operation's clock.
    pub at: u64,
    pub changes: Vec<Change>,
    pub status: RequestStatus,
}

/// Every request in the library, newest first.
pub fn list_requests(conn: &Connection, library: &str) -> Result<Vec<Request>> {
    let mut statement = conn.prepare(
        "SELECT o.id, o.intent, o.hlc, o.changes, o.device_id, d.name, m.name, r.status
           FROM operations o
           LEFT JOIN devices d ON d.library_id = o.library_id AND d.id = o.device_id
           LEFT JOIN members m ON m.library_id = o.library_id AND m.id = d.member_id
           LEFT JOIN change_requests r ON r.library_id = o.library_id AND r.id = o.id
          WHERE o.library_id = ?1 AND o.status = 'request'
          ORDER BY o.hlc DESC",
    )?;
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
        ))
    })?;
    let mut requests = Vec::new();
    for row in rows {
        let (id, intent, hlc, changes, device_id, device_name, person, resolved) = row?;
        requests.push(Request {
            id,
            intent,
            device_id,
            device_name,
            person,
            at: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
            changes: serde_json::from_str(&changes)?,
            status: match resolved.as_deref() {
                Some("accepted") => RequestStatus::Accepted,
                Some("rejected") => RequestStatus::Rejected,
                _ => RequestStatus::Pending,
            },
        });
    }
    Ok(requests)
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
    pending(conn, library, id)?;
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        library,
        "request.reject",
        vec![resolution(id, "rejected")],
    )?;
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
        "SELECT EXISTS (SELECT 1 FROM change_requests WHERE library_id = ?1 AND id = ?2)",
        [library, id],
        |row| row.get(0),
    )?;
    if resolved {
        return Err(Error::RequestResolved);
    }
    Ok(serde_json::from_str(&changes)?)
}
