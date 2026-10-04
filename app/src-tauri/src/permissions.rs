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

    fn member_of_device(&self, device: &str) -> Option<String> {
        self.member_of(device)
    }

    fn fenced(&self, operation: &Operation) -> bool {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM device_fences f
                  WHERE f.library_id = ?1 AND f.device_id = ?2 AND ?3 < f.until_hlc
                    AND ?4 <> f.through
                    AND NOT EXISTS (SELECT 1 FROM device_fence_chains c
                                     WHERE c.library_id = ?1 AND c.fence_operation_id = f.operation_id
                                       AND c.device_id = ?2 AND c.operation_id = ?4))
                     OR EXISTS (SELECT 1 FROM revocations r
                                  JOIN field_versions v
                                    ON v.library_id = r.library_id AND v.entity = 'revocations'
                                   AND v.entity_id = r.id AND v.field = 'device_id'
                                 WHERE r.library_id = ?1 AND r.device_id = ?2 AND v.hlc < ?3)",
                params![
                    self.library,
                    operation.hlc.device,
                    operation.hlc.to_string(),
                    operation.id
                ],
                |row| row.get(0),
            )
            .unwrap_or(true)
    }

    fn mark_member(&self, id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT member_id FROM marks WHERE library_id = ?1 AND id = ?2",
                [self.library, id],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }
}

impl SqlView<'_> {
    /// The same device, or two devices of the same person.
    fn same_person(&self, one: &str, other: &str) -> bool {
        one == other
            || (self.member_of(one).is_some() && self.member_of(one) == self.member_of(other))
    }
}

/// Whether operations of the library must be judged again when something arrives
/// late: roles, fences or revocations depend on the order of the log (D73, D84).
pub fn judged_by_order(conn: &Connection, library: &str) -> Result<bool> {
    Ok(has_roles(conn, library)?
        || conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM device_fences WHERE library_id = ?1)
                 OR EXISTS (SELECT 1 FROM revocations WHERE library_id = ?1)",
            [library],
            |row| row.get(0),
        )?)
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
    let log = sync::library_log(tx, library)?;
    // Every fence in the log is a candidate; only those set by an operation that
    // is itself allowed count (D84). Dropping one can make another operation a
    // request or not, so replay until no candidate is dropped. The set only
    // shrinks, so it ends, and every device reaches the same result.
    let mut fences = candidate_fences(&log);
    let mut editions = BTreeSet::new();
    loop {
        replay(tx, library, &log, &fences, &mut editions)?;
        let before = fences.len();
        let mut kept = Vec::new();
        for fence in fences {
            if ops::status_of(tx, &fence.operation_id)? == ops::Status::Applied {
                kept.push(fence);
            }
        }
        fences = kept;
        if fences.len() == before {
            break;
        }
    }
    for operation in &log {
        // Made here, applied here, and now a request: tell whoever made it, once.
        if was_applied.contains(&operation.id)
            && ops::status_of(tx, &operation.id)? == ops::Status::Request
        {
            tx.execute(
                "INSERT OR IGNORE INTO request_notices (library_id, operation_id)
                 VALUES (?1, ?2)",
                [library, &operation.id],
            )?;
        }
    }
    let all_editions: BTreeSet<String> = tx
        .prepare("SELECT id FROM editions WHERE library_id = ?1")?
        .query_map([library], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    editions.extend(all_editions);
    search::reindex(tx, library, &editions)?;
    Ok(())
}

/// The mark of a fence for `device` (D87): the id of the newest operation of it
/// this device holds, or the empty string if none. What the administrator who takes
/// power away knows of the device is that operation and the ones it chains to.
pub fn fence_mark(conn: &Connection, library: &str, device: &str) -> Result<String> {
    Ok(ops::latest_of(conn, library, device)?.unwrap_or_default())
}

/// The changes that fence each of `devices` that is registered in the library. They
/// go in the same operation as the change that takes the power away.
pub fn fence_changes(conn: &Connection, library: &str, devices: &[String]) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for device in devices {
        let registered: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM devices WHERE library_id = ?1 AND id = ?2)",
            [library, device],
            |row| row.get(0),
        )?;
        if registered {
            changes.push(Change::update(
                Entity::Device,
                device,
                serde_json::json!({ "trusted_through": fence_mark(conn, library, device)? }),
            ));
        }
    }
    Ok(changes)
}

/// A fence an operation of the log sets (D84, D87), not yet known to be honoured.
struct Fence {
    device: String,
    through: String,
    until: String,
    operation_id: String,
}

fn candidate_fences(log: &[Operation]) -> Vec<Fence> {
    log.iter().flat_map(fences_of).collect()
}

fn fences_of(operation: &Operation) -> Vec<Fence> {
    operation
        .changes
        .iter()
        .filter(|change| change.entity == Entity::Device && change.action != Action::Delete)
        .filter_map(|change| {
            let through = change.fields.get("trusted_through")?.as_str()?;
            Some(Fence {
                device: change.id.clone(),
                through: through.to_owned(),
                until: operation.hlc.to_string(),
                operation_id: operation.id.clone(),
            })
        })
        .collect()
}

/// Notes the fence and the operations it still counts. Worked out here, once,
/// so judging each operation of a long log never walks the chain again. If
/// `through` is not in the log yet, the chain is empty and the fence covers all
/// of the device's operations before it: they are judged again when it arrives.
fn insert_fence(conn: &Connection, library: &str, fence: &Fence) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO device_fences
            (library_id, device_id, through, until_hlc, operation_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            library,
            fence.device,
            fence.through,
            fence.until,
            fence.operation_id
        ],
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO device_fence_chains
            (library_id, fence_operation_id, device_id, operation_id)
         WITH RECURSIVE chain(id, previous) AS (
            SELECT id, previous FROM operations
             WHERE library_id = ?1 AND device_id = ?2 AND id = ?3
            UNION
            SELECT o.id, o.previous FROM operations o JOIN chain ON o.id = chain.previous
             WHERE o.library_id = ?1 AND o.device_id = ?2)
         SELECT ?1, ?4, ?2, id FROM chain",
        params![library, fence.device, fence.through, fence.operation_id],
    )?;
    Ok(())
}

/// Remembers the fences an applied operation sets (D84). Returns whether the log
/// already holds an operation the new fence puts under suspicion: judged before the
/// fence was known, it has to be judged again.
pub fn note_fences(conn: &Connection, operation: &Operation) -> Result<bool> {
    let mut late = false;
    for fence in fences_of(operation) {
        insert_fence(conn, &operation.library, &fence)?;
        late |= conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM operations o
              WHERE o.library_id = ?1 AND o.device_id = ?2 AND o.replicable = 1
                AND o.hlc < ?3 AND o.id <> ?4
                AND NOT EXISTS (SELECT 1 FROM device_fence_chains c
                                 WHERE c.library_id = ?1 AND c.fence_operation_id = ?5
                                   AND c.device_id = ?2 AND c.operation_id = o.id))",
            params![
                operation.library,
                fence.device,
                fence.until,
                fence.through,
                fence.operation_id
            ],
            |row| row.get::<_, bool>(0),
        )?;
    }
    Ok(late)
}

/// One pass over the log with the given fences in force.
fn replay(
    tx: &Transaction<'_>,
    library: &str,
    log: &[Operation],
    fences: &[Fence],
    editions: &mut BTreeSet<String>,
) -> Result<()> {
    ops::clear_library(tx, library)?;
    for fence in fences {
        insert_fence(tx, library, fence)?;
    }
    for operation in log {
        let status = if allowed(tx, operation) {
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
            ops::Status::Request
        };
        ops::set_status(tx, &operation.id, status)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn the_same_person_is_the_same_device_or_two_devices_of_one_member() {
        let conn = db::open_in_memory().unwrap();
        let library = ops::active_library(&conn).unwrap();
        for (device, member) in [("d1", Some("m1")), ("d2", Some("m1")), ("d3", Some("m2"))] {
            conn.execute(
                "INSERT INTO devices (library_id, id, public_key, name, member_id)
                 VALUES (?1, ?2, 'k', ?2, ?3)",
                params![library, device, member],
            )
            .unwrap();
        }
        let view = SqlView::new(&conn, &library);

        assert!(view.same_person("d1", "d1"));
        assert!(
            view.same_person("ghost", "ghost"),
            "a device is its own person"
        );
        assert!(view.same_person("d1", "d2"), "two devices of one member");
        assert!(!view.same_person("d1", "d3"), "two members");
        assert!(!view.same_person("d1", "ghost"));
        assert!(!view.same_person("ghost", "d1"));
        assert!(
            !view.same_person("ghost", "other-ghost"),
            "no member, no person"
        );
    }
}
