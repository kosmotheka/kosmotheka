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

    /// The person `device` belongs to (D96): its `member_id`, unless someone vouched for another key.
    fn member_of(&self, device: &str) -> Option<String> {
        person_of_device(self.conn, self.library, device)
            .ok()
            .flatten()
    }
}

/// SQL for the person of the `devices` row aliased `d` (D96, D98): its `member_id`, unless it carries a
/// `vouched_key` other than the key it is registered with, or the person had their access withdrawn,
/// which makes it nobody's. Compared byte by byte, like `public_key`.
pub fn person_of(d: &str) -> String {
    format!(
        "CASE WHEN ({d}.vouched_key IS NULL OR {d}.vouched_key = '' \
         OR {d}.vouched_key = {d}.public_key) \
         AND NOT EXISTS (SELECT 1 FROM members wd WHERE wd.library_id = {d}.library_id \
                         AND wd.id = {d}.member_id AND wd.access_withdrawn = 1) \
         THEN {d}.member_id END"
    )
}

/// The fields of a change that says whose `device` is (D96): `member_id` and, in the same
/// change, the key that whoever writes it has registered for that device. Taking the person
/// away empties both. Without a key to vouch for, nobody is assigned: this device's own key
/// is known even before it registers; another device's only through its `device` row.
pub fn assignment_fields(
    conn: &Connection,
    library: &str,
    device: &str,
    member: Option<&str>,
) -> Result<serde_json::Value> {
    let Some(member) = member else {
        return Ok(serde_json::json!({ "member_id": null, "vouched_key": null }));
    };
    let key = if device == ops::device_id(conn)? {
        ops::signing_key(conn)?.public_key()
    } else {
        conn.query_row(
            "SELECT public_key FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, device],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or_else(|| crate::error::Error::MissingRow {
            entity: "device",
            id: device.to_owned(),
        })?
    };
    Ok(serde_json::json!({ "member_id": member, "vouched_key": key }))
}

/// The person `device` belongs to in `library`, with the rules of D96 and D98. A device that is
/// not registered, or whose vouched key is not its own, belongs to nobody.
pub fn person_of_device(conn: &Connection, library: &str, device: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {} FROM devices d WHERE d.library_id = ?1 AND d.id = ?2",
                person_of("d")
            ),
            [library, device],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
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

    fn device_has_member_id(&self, device: &str) -> bool {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM devices
                  WHERE library_id = ?1 AND id = ?2 AND member_id IS NOT NULL)",
                [self.library, device],
                |row| row.get(0),
            )
            .unwrap_or(true)
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
                     OR EXISTS (SELECT 1 FROM device_seals s
                                 WHERE s.library_id = ?1 AND ?3 < s.until_hlc
                                   AND NOT EXISTS (SELECT 1 FROM device_fences f
                                                    WHERE f.library_id = ?1
                                                      AND f.operation_id = s.operation_id
                                                      AND f.device_id = ?2))
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

    fn cloud_device(&self) -> Option<String> {
        let named: String = self
            .conn
            .query_row(
                "SELECT cloud_device_id FROM libraries WHERE id = ?1",
                [self.library],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .ok()
            .flatten()
            .flatten()
            .filter(|device| !device.is_empty())?;
        // Only if the key registered for that device in the library is the one its id derives from (D95).
        let key: String = self
            .conn
            .query_row(
                "SELECT public_key FROM devices WHERE library_id = ?1 AND id = ?2",
                [self.library, &named],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()?;
        kosmotheka_domain::cloud_device_id(&key)
            .filter(|derived| *derived == named)
            .map(|_| named)
    }

    fn member_exists(&self, member: &str) -> bool {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM members WHERE library_id = ?1 AND id = ?2)",
                [self.library, member],
                |row| row.get(0),
            )
            .unwrap_or(false)
    }

    fn member_has_account(&self, member: &str) -> bool {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM members
                  WHERE library_id = ?1 AND id = ?2 AND account_id IS NOT NULL AND account_id <> '')",
                [self.library, member],
                |row| row.get(0),
            )
            .unwrap_or(false)
    }

    fn member_access_withdrawn(&self, member: &str) -> bool {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM members
                  WHERE library_id = ?1 AND id = ?2 AND access_withdrawn = 1)",
                [self.library, member],
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

#[cfg(test)]
impl SqlView<'_> {
    /// `LibraryView::cloud_device` for tests that do not import the trait.
    pub fn cloud_device_for_tests(&self) -> Option<String> {
        self.cloud_device()
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

/// The fence of the cloud device when its power is taken away (D84, D95): like `fence_changes`, but also when
/// the device is not registered in this library yet, with the empty mark. The library may name the cloud
/// before it has written anything; if its registration and what it signed arrive after the withdrawal, dated
/// before it, nothing of that may be believed.
pub fn fence_cloud_change(conn: &Connection, library: &str, device: &str) -> Result<Change> {
    Ok(Change::update(
        Entity::Device,
        device,
        serde_json::json!({ "trusted_through": fence_mark(conn, library, device)? }),
    ))
}

/// Turns `changes` into a seal (D94): a fence for every device registered in the library
/// and for this one, each with the newest operation this device holds of it (this
/// device's own is its `previous`). A change that already updates a device row gets the
/// fence as one more field, so no row is touched twice in an operation. Whoever
/// calls it must have registered this device first, so the mark is the true last one.
pub fn seal_changes(conn: &Connection, library: &str, changes: &mut Vec<Change>) -> Result<()> {
    let me = ops::device_id(conn)?;
    let mut devices: Vec<String> = conn
        .prepare("SELECT id FROM devices WHERE library_id = ?1 ORDER BY id")?
        .query_map([library], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if !devices.contains(&me) {
        devices.push(me);
    }
    for device in devices {
        let mark = serde_json::Value::String(fence_mark(conn, library, &device)?);
        let update = changes.iter_mut().find(|change| {
            change.entity == Entity::Device
                && change.action == Action::Update
                && change.id == device
        });
        match update {
            Some(change) => {
                change.fields.insert("trusted_through".into(), mark);
            }
            None => changes.push(Change::update(
                Entity::Device,
                &device,
                serde_json::json!({ "trusted_through": mark }),
            )),
        }
    }
    Ok(())
}

/// A library that got its roles before seals existed is sealed by an administrator's
/// device (D94), once: nothing is written where a seal is already applied, nor on a
/// device that is not an administrator's. Called when a sync session has merged what
/// the other side had, never on start: a seal believes of each device only what this
/// one holds, and a device that has been away would fence what the others did meanwhile.
pub fn seal_if_missing(conn: &mut Connection, libraries: &[String]) -> Result<()> {
    let me = ops::device_id(conn)?;
    for library in libraries {
        let sealed: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM device_seals s
                              JOIN operations o ON o.library_id = s.library_id AND o.id = s.operation_id
                             WHERE s.library_id = ?1 AND o.status = 'applied')",
            [library],
            |row| row.get(0),
        )?;
        let admin = SqlView::new(conn, library).role_of_device(&me) == Some(Role::Admin);
        if sealed || !admin || !has_roles(conn, library)? {
            continue;
        }
        let tx = conn.transaction()?;
        ops::ensure_registered(&tx, library)?;
        let mut changes = Vec::new();
        seal_changes(&tx, library, &mut changes)?;
        ops::record(&tx, library, "library.seal", changes)?;
        tx.commit()?;
    }
    Ok(())
}

/// Vouches, once, for the assignments made before D96: an administrator's device writes, in one
/// operation, the `vouched_key` of every registered device that belongs to a person and has none,
/// with the key this device has of it. Nothing is written where there is nothing to vouch for, nor
/// by a device that is not an administrator's. Called with `seal_if_missing`, after it.
pub fn vouch_if_missing(conn: &mut Connection, libraries: &[String]) -> Result<()> {
    let me = ops::device_id(conn)?;
    for library in libraries {
        let admin = SqlView::new(conn, library).role_of_device(&me) == Some(Role::Admin);
        if !admin || !has_roles(conn, library)? {
            continue;
        }
        let changes: Vec<Change> = conn
            .prepare(
                "SELECT id, public_key FROM devices
                  WHERE library_id = ?1 AND member_id IS NOT NULL
                    AND (vouched_key IS NULL OR vouched_key = '')
                  ORDER BY id",
            )?
            .query_map([library], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .map(|row| {
                let (id, key) = row?;
                Ok(Change::update(
                    Entity::Device,
                    id,
                    serde_json::json!({ "vouched_key": key }),
                ))
            })
            .collect::<Result<_>>()?;
        if changes.is_empty() {
            continue;
        }
        let tx = conn.transaction()?;
        ops::record(&tx, library, "library.vouch", changes)?;
        tx.commit()?;
    }
    Ok(())
}

/// A fence an operation of the log sets (D84, D87), not yet known to be honoured.
struct Fence {
    author: String,
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
                author: operation.hlc.device.clone(),
                device: change.id.clone(),
                through: through.to_owned(),
                until: operation.hlc.to_string(),
                operation_id: operation.id.clone(),
            })
        })
        .collect()
}

/// Whether the operation fences its own author's device: a seal (D94), if it is applied.
pub fn is_seal(operation: &Operation) -> bool {
    fences_of(operation)
        .iter()
        .any(|fence| fence.device == fence.author)
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
    if fence.device == fence.author {
        conn.execute(
            "INSERT OR IGNORE INTO device_seals (library_id, operation_id, until_hlc)
             VALUES (?1, ?2, ?3)",
            params![library, fence.operation_id, fence.until],
        )?;
    }
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
    let fences = fences_of(operation);
    for fence in &fences {
        insert_fence(conn, &operation.library, fence)?;
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
    if is_seal(operation) {
        // A seal also puts under suspicion what the devices it does not name did before it.
        late |= conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM operations o
              WHERE o.library_id = ?1 AND o.replicable = 1 AND o.hlc < ?2 AND o.id <> ?3
                AND NOT EXISTS (SELECT 1 FROM device_fences f
                                 WHERE f.library_id = ?1 AND f.operation_id = ?3
                                   AND f.device_id = o.device_id))",
            params![operation.library, operation.hlc.to_string(), operation.id],
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

/// Makes this blank device the cloud of a library in a test: its id is the one its key gives (D95), the only
/// kind of device `library.cloud_device_id` can name with effect. Call it before the device writes anything.
#[cfg(test)]
pub fn become_a_cloud_device(conn: &Connection, seed: u8) -> String {
    let key = kosmotheka_domain::DeviceKey::from_seed([seed; 32]);
    let id = kosmotheka_domain::cloud_device_id(&key.public_key()).unwrap();
    ops::set_identity(conn, &id, [seed; 32]).unwrap();
    id
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
