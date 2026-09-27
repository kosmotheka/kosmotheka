//! Permissions checked on this device (D55, D73): the rules live in the domain
//! (`kosmotheka_domain::permission`); this module answers their questions from
//! the database, and rebuilds a library from its log when roles change late.

use std::collections::BTreeSet;

use kosmotheka_domain::{Action, Change, Entity, LibraryView, Operation, Role, is_allowed};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::error::Result;
use crate::ops;
use crate::search;
use crate::sync;

/// The library as it stands in this database, for the permission rules.
pub struct SqlView<'a> {
    conn: &'a Connection,
    library: &'a str,
}

impl<'a> SqlView<'a> {
    pub fn new(conn: &'a Connection, library: &'a str) -> Self {
        Self { conn, library }
    }

    fn member_of(&self, device: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
                [self.library, device],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .ok()
            .flatten()
            .flatten()
    }
}

// The trait's methods cannot fail, so a database error reads as "no": a device
// that cannot be checked gets the least rights, never more.
impl LibraryView for SqlView<'_> {
    fn has_roles(&self) -> bool {
        has_roles(self.conn, self.library).unwrap_or(false)
    }

    fn role_of_device(&self, device: &str) -> Option<Role> {
        let member = self.member_of(device)?;
        let role: Option<String> = self
            .conn
            .query_row(
                "SELECT role FROM members WHERE library_id = ?1 AND id = ?2",
                [self.library, &member],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()?;
        Role::parse(&role?)
    }

    fn created_by_same_person(&self, entity: Entity, id: &str, device: &str) -> bool {
        let creator: Option<String> = self
            .conn
            .query_row(
                "SELECT device_id FROM row_authors
                  WHERE library_id = ?1 AND entity = ?2 AND entity_id = ?3",
                [self.library, entity.table(), id],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();
        creator.is_some_and(|creator| self.same_person(&creator, device))
    }

    fn requested_by_same_person(&self, request: &str, device: &str) -> bool {
        let asker: Option<String> = self
            .conn
            .query_row(
                "SELECT device_id FROM operations WHERE library_id = ?1 AND id = ?2",
                [self.library, request],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten();
        asker.is_some_and(|asker| self.same_person(&asker, device))
    }
}

impl SqlView<'_> {
    /// The same device, or two devices of the same person.
    fn same_person(&self, one: &str, other: &str) -> bool {
        one == other
            || (self.member_of(one).is_some() && self.member_of(one) == self.member_of(other))
    }
}

pub fn has_roles(conn: &Connection, library: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM members WHERE library_id = ?1 AND role IS NOT NULL)",
        [library],
        |row| row.get(0),
    )?)
}

/// Remembers which device created a row. The first create counts: a later create
/// of the same row is an edit.
pub fn note_author(conn: &Connection, library: &str, change: &Change, device: &str) -> Result<()> {
    if change.action == Action::Create {
        conn.execute(
            "INSERT OR IGNORE INTO row_authors (library_id, entity, entity_id, device_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![library, change.entity.table(), change.id, device],
        )?;
    }
    Ok(())
}

pub fn allowed(conn: &Connection, operation: &Operation) -> bool {
    is_allowed(operation, &SqlView::new(conn, &operation.library))
}

/// Applies the library's whole log again, in HLC order, judging each operation
/// against the roles in force at its moment (D73). Needed when an operation that
/// changes roles arrives after later ones were already judged. The library row
/// itself stays: the library migrated from before libraries has no create operation.
pub fn rebuild(tx: &Transaction<'_>, library: &str) -> Result<()> {
    let me = ops::device_id(tx)?;
    let was_applied: BTreeSet<String> = tx
        .prepare(
            "SELECT id FROM operations
              WHERE library_id = ?1 AND device_id = ?2 AND status = 'applied'",
        )?
        .query_map([library, &me], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    ops::clear_library(tx, library)?;
    let log = sync::library_log(tx, library)?;
    let mut editions = BTreeSet::new();
    for operation in log {
        let status = if allowed(tx, &operation) {
            let hlc = operation.hlc.to_string();
            for change in &operation.changes {
                if sync::apply_change(tx, library, change, &hlc)? {
                    note_author(tx, library, change, &operation.hlc.device)?;
                }
                if change.entity == Entity::Edition {
                    editions.insert(change.id.clone());
                }
            }
            ops::Status::Applied
        } else {
            // Made here, applied here, and now a request: tell whoever made it, once.
            if was_applied.contains(&operation.id) {
                tx.execute(
                    "INSERT OR IGNORE INTO request_notices (library_id, operation_id)
                     VALUES (?1, ?2)",
                    [library, &operation.id],
                )?;
            }
            ops::Status::Request
        };
        ops::set_status(tx, &operation.id, status)?;
    }
    let all_editions: BTreeSet<String> = tx
        .prepare("SELECT id FROM editions WHERE library_id = ?1")?
        .query_map([library], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    editions.extend(all_editions);
    search::reindex(tx, library, &editions)?;
    Ok(())
}
