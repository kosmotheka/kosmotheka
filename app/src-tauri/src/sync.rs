//! Replication between devices without an arbiter (D31-D33). See docs/architecture/sync.md.
//!
//! Every device keeps every replicable operation of the libraries it has. Syncing
//! exchanges the ones the other side lacks, library by library, and `merge` applies
//! them with rules that make all devices converge on the same state whatever order
//! they arrive in.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use kosmotheka_domain::{
    Action, Change, Entity, FieldSpec, FieldType, Hlc, HlcClock, Operation, verify,
};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::Result;
use crate::ops::{self, read_state, write_state};
use crate::permissions;
use crate::search;

/// Parents before children, so every reference exists when a row is created.
pub const ORDER: [Entity; 23] = [
    Entity::Library,
    Entity::Device,
    Entity::Publisher,
    Entity::Tag,
    Entity::Series,
    Entity::Person,
    Entity::Member,
    Entity::Work,
    Entity::WorkContributor,
    Entity::WorkTag,
    Entity::WorkGenre,
    Entity::SeriesEntry,
    Entity::WorkRelation,
    Entity::Edition,
    Entity::BookDetails,
    Entity::ComicDetails,
    Entity::BoardgameDetails,
    Entity::FilmDetails,
    Entity::TvSeriesDetails,
    Entity::Copy,
    Entity::Mark,
    Entity::Revocation,
    Entity::ChangeRequest,
];

const BASELINE_KEY: &str = "sync_baseline";

/// Last HLC this device holds from each author device, within one library.
pub type VersionVector = BTreeMap<String, String>;

/// One version vector per library.
pub type LibraryVectors = BTreeMap<String, VersionVector>;

/// An operation as it travels between devices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOperation {
    pub schema_version: i64,
    pub operation: Operation,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeReport {
    /// New operations stored.
    pub received: usize,
    /// Operations this device already had.
    pub known: usize,
    /// Operations refused: another schema version or malformed.
    pub rejected: usize,
    /// Changes skipped by the rules (older than the stored field, deleted rows...).
    pub skipped_changes: usize,
    /// Operations received that their author's role did not allow: kept as requests (D73).
    pub requests: usize,
}

impl MergeReport {
    pub fn add(&mut self, other: &MergeReport) {
        self.received += other.received;
        self.known += other.known;
        self.rejected += other.rejected;
        self.skipped_changes += other.skipped_changes;
        self.requests += other.requests;
    }
}

/// Rows of one entity in one library, with the field names operations use (`id` included).
pub fn entity_rows(
    conn: &Connection,
    library: &str,
    entity: Entity,
) -> Result<Vec<Map<String, Value>>> {
    let fields: Vec<&str> = entity.fields().iter().map(|spec| spec.name).collect();
    let sql = format!(
        "SELECT {}, {} FROM {} WHERE {} = ?1 ORDER BY {}",
        entity.id_column(),
        fields.join(", "),
        entity.table(),
        entity.library_column(),
        entity.id_column()
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([library], |row| {
        let mut map = Map::new();
        map.insert("id".into(), json!(row.get::<_, String>(0)?));
        for (i, field) in entity.fields().iter().enumerate() {
            let mut value = to_json(row.get_ref(i + 1)?);
            // SQLite keeps booleans as 0 and 1; operations carry true and false.
            if field.field_type == FieldType::Boolean
                && let Some(n) = value.as_i64()
            {
                value = json!(n != 0);
            }
            map.insert(field.name.into(), value);
        }
        Ok(map)
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Every library this device holds.
pub fn libraries(conn: &Connection) -> Result<Vec<String>> {
    Ok(conn
        .prepare("SELECT id FROM libraries ORDER BY id")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// Once per device: logs everything it already has as one replicable operation per
/// library, so rows from before the current format can travel too (D31). Empty
/// libraries log nothing.
pub fn ensure_baseline(conn: &mut Connection) -> Result<()> {
    if read_state(conn, BASELINE_KEY)?.is_some() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    for library in libraries(&tx)? {
        let mut changes = Vec::new();
        for entity in ORDER {
            for mut row in entity_rows(&tx, &library, entity)? {
                let id = row
                    .remove("id")
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default();
                changes.push(Change::create(entity, id, Value::Object(row)));
            }
        }
        // A library row alone is not worth a baseline.
        if changes
            .iter()
            .any(|change| change.entity != Entity::Library)
        {
            ops::record_existing(&tx, &library, "sync.baseline", changes)?;
        }
    }
    write_state(&tx, BASELINE_KEY, Some("done"))?;
    tx.commit()?;
    Ok(())
}

/// What this device holds of each of `libraries`, whether it has anything of them or not.
pub fn version_vectors(conn: &Connection, libraries: &[String]) -> Result<LibraryVectors> {
    let mut statement = conn.prepare(
        "SELECT device_id, max(hlc) FROM operations
          WHERE replicable = 1 AND library_id = ?1 GROUP BY device_id",
    )?;
    let mut vectors = LibraryVectors::new();
    for library in libraries {
        let vector = statement
            .query_map([library], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        vectors.insert(library.clone(), vector);
    }
    Ok(vectors)
}

/// Replicable operations of the libraries in `vectors` that their owner lacks, oldest first.
pub fn operations_since(conn: &Connection, vectors: &LibraryVectors) -> Result<Vec<SyncOperation>> {
    let mut statement = conn.prepare(
        "SELECT id, library_id, hlc, intent, changes, schema_version, signature, previous FROM operations
          WHERE replicable = 1 AND library_id = ?1
            AND hlc > coalesce((SELECT value FROM json_each(?2) WHERE key = device_id), '')
          ORDER BY hlc",
    )?;
    let mut operations = Vec::new();
    for (library, vector) in vectors {
        let rows =
            statement.query_map(params![library, serde_json::to_string(vector)?], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })?;
        for row in rows {
            let (id, library, hlc, intent, changes, schema_version, signature, previous) = row?;
            // An entity dropped from the schema since this operation was recorded
            // (like `wish`, D61) makes it unparsable: it can no longer be applied
            // anywhere, so it is left out entirely rather than sent with some of its
            // changes silently missing, which would no longer match its signature.
            let Ok(changes) = serde_json::from_str(&changes) else {
                continue;
            };
            operations.push(SyncOperation {
                schema_version,
                operation: Operation {
                    id,
                    library,
                    hlc: hlc
                        .parse()
                        .map_err(|_| crate::error::Error::CorruptState("operations.hlc"))?,
                    intent,
                    changes,
                    previous,
                    signature,
                },
            });
        }
    }
    operations.sort_by(|a, b| a.operation.hlc.cmp(&b.operation.hlc));
    Ok(operations)
}

/// Applies operations from other devices (D32). Never fails because of their content:
/// what cannot be applied is skipped and counted. Only storage errors abort. Which
/// libraries a peer may send is checked by the caller, before merging.
pub fn merge(conn: &mut Connection, incoming: Vec<SyncOperation>) -> Result<MergeReport> {
    let tx = conn.transaction()?;
    let report = merge_in(&tx, incoming)?;
    // Assignments that were waiting for a device's registration (milestone 5.7).
    crate::people::apply_pending_owners(&tx)?;
    tx.commit()?;
    Ok(report)
}

fn merge_in(tx: &Transaction<'_>, mut incoming: Vec<SyncOperation>) -> Result<MergeReport> {
    let local_schema = ops::schema_version(tx)?;
    let mut clock = HlcClock::new(ops::device_id(tx)?, ops::last_hlc(tx)?.as_ref());
    let mut latest: Option<Hlc> = None;
    let mut report = MergeReport::default();

    // HLC order respects causality: a row's creation comes before its edits.
    incoming.sort_by(|a, b| a.operation.hlc.cmp(&b.operation.hlc));
    // The newest operation each library had before this batch: one older than
    // that may change what later ones were allowed to do (D73).
    let mut newest_before: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut arrived_late = BTreeSet::new();
    let mut received = Vec::new();
    let mut held_back = BTreeSet::new();
    for SyncOperation {
        schema_version,
        operation,
    } in incoming
    {
        if is_known(tx, &operation.id)? {
            report.known += 1;
            continue;
        }
        // Once an operation is refused its author's later ones wait too: the vector
        // must stay below it so the next sync sends it again (DAT-004).
        let author = (operation.library.clone(), operation.hlc.device.clone());
        // An older version is history from before an update (D31); a newer one
        // may mean something this app does not know. A clock far ahead waits until
        // ours catches up, rather than winning every field it touches (DAT-005).
        if held_back.contains(&author)
            || schema_version > local_schema
            || operation.hlc.is_ahead_of(now_millis())
            || operation.validate().is_err()
            || !signature_holds(tx, &operation)?
            || !previous_holds(tx, &operation)?
            || hlc_taken(tx, &operation)?
        {
            held_back.insert(author);
            report.rejected += 1;
            continue;
        }
        latest = Some(clock.receive(&operation.hlc, now_millis()));

        let library = operation.library.as_str();
        let hlc = operation.hlc.to_string();
        if !newest_before.contains_key(library) {
            newest_before.insert(library.to_owned(), newest_hlc(tx, library)?);
        }
        if newest_before[library]
            .as_ref()
            .is_some_and(|newest| hlc < *newest)
        {
            arrived_late.insert(library.to_owned());
        }
        let status = if permissions::allowed(tx, &operation) {
            let mut affected = BTreeSet::new();
            for change in &operation.changes {
                affected.extend(search::affected_editions(tx, library, change)?);
                if apply_change(tx, library, change, &hlc)? {
                    permissions::note_author(tx, library, change, &operation.hlc.device)?;
                } else {
                    report.skipped_changes += 1;
                }
                affected.extend(search::affected_editions(tx, library, change)?);
            }
            search::reindex(tx, library, &affected)?;
            ops::Status::Applied
        } else {
            ops::Status::Request
        };
        ops::log_operation(tx, &operation, schema_version, status)?;
        // After logging, so an operation of the fenced device already in the log is seen.
        if status == ops::Status::Applied && permissions::note_fences(tx, &operation)? {
            arrived_late.insert(library.to_owned());
        }
        // A seal that is a request here may be the one that puts the operations before it
        // under a fence, and with them out of the way the seal stands (D94): judge again.
        if status == ops::Status::Request && permissions::is_seal(&operation) {
            arrived_late.insert(library.to_owned());
        }
        received.push(operation.id.clone());
        report.received += 1;
    }

    // Something older arrived where the order decides: judge the whole library again.
    for library in &arrived_late {
        if permissions::judged_by_order(tx, library)? {
            permissions::rebuild(tx, library)?;
        }
    }
    for id in &received {
        let status: String =
            tx.query_row("SELECT status FROM operations WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        if status == "request" {
            report.requests += 1;
        }
    }

    if let Some(latest) = latest {
        ops::set_last_hlc(tx, &latest)?;
    }
    Ok(report)
}

/// Whether the operation follows an operation of its author that this device
/// already has (D87). One whose previous is missing waits for it, with the rest
/// of its author's; one that chains to someone else's operation, to another
/// library's, or to a later one, is malformed. Either way it is not logged.
/// An operation with no previous is taken as it comes: a fork is left to the
/// fences (D87).
fn previous_holds(conn: &Connection, operation: &Operation) -> Result<bool> {
    let Some(previous) = &operation.previous else {
        return Ok(true);
    };
    let found: Option<(String, String, String)> = conn
        .query_row(
            "SELECT library_id, device_id, hlc FROM operations WHERE id = ?1",
            [previous],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    Ok(found.is_some_and(|(library, device, hlc)| {
        library == operation.library
            && device == operation.hlc.device
            && hlc < operation.hlc.to_string()
    }))
}

fn newest_hlc(conn: &Connection, library: &str) -> Result<Option<String>> {
    Ok(conn.query_row(
        "SELECT max(hlc) FROM operations WHERE library_id = ?1 AND replicable = 1",
        [library],
        |row| row.get(0),
    )?)
}

/// The library's replicable log in HLC order, as operations, for a rebuild.
pub fn library_log(conn: &Connection, library: &str) -> Result<Vec<Operation>> {
    let vectors = LibraryVectors::from([(library.to_owned(), VersionVector::new())]);
    Ok(operations_since(conn, &vectors)?
        .into_iter()
        .map(|synced| synced.operation)
        .collect())
}

/// Whether the operation's signature is its author's (D56). The author's key is the
/// one registered in the library, or the one the operation itself registers. A
/// device's key never changes: an operation that sets another one is refused, and
/// so is one without a signature.
fn signature_holds(tx: &Transaction<'_>, operation: &Operation) -> Result<bool> {
    let library = operation.library.as_str();
    for change in &operation.changes {
        let offered = change.fields.get("public_key").and_then(Value::as_str);
        if change.entity == Entity::Device
            && let Some(offered) = offered
            && registered_key(tx, library, &change.id)?.is_some_and(|key| key != offered)
        {
            return Ok(false);
        }
    }
    // Every operation is signed (D56): one without a signature is refused anywhere.
    if operation.signature.is_none() {
        return Ok(false);
    }
    let author = operation.hlc.device.as_str();
    let key = match registered_key(tx, library, author)? {
        Some(key) => key,
        None => match operation.changes.iter().find(|change| {
            change.entity == Entity::Device
                && change.action == Action::Create
                && change.id == author
        }) {
            Some(change) => match change.fields.get("public_key").and_then(Value::as_str) {
                Some(key) => key.to_owned(),
                None => return Ok(false),
            },
            None => return Ok(false),
        },
    };
    Ok(verify(operation, &key).is_ok())
}

fn registered_key(conn: &Connection, library: &str, device: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT public_key FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, device],
            |row| row.get(0),
        )
        .optional()?)
}

/// One change under the merge rules. Returns whether anything was written.
pub fn apply_change(
    tx: &Transaction<'_>,
    library: &str,
    change: &Change,
    hlc: &str,
) -> Result<bool> {
    let entity = change.entity;
    // A deleted library ignores everything that comes after, whatever its HLC (D65).
    if is_tombstoned(tx, library, Entity::Library, library)?
        || is_tombstoned(tx, library, entity, &change.id)?
    {
        return Ok(false); // the delete wins
    }

    // A savepoint lets a change that breaks a constraint (a reference to a row
    // deleted elsewhere) be undone alone, without losing the rest of the sync.
    tx.execute_batch("SAVEPOINT change")?;
    let outcome = (|| -> Result<bool> {
        match change.action {
            Action::Delete => {
                if entity == Entity::Library {
                    ops::delete_library(tx, library)?;
                } else {
                    ops::delete_row(tx, entity, library, &change.id)?;
                }
                ops::write_versions(tx, library, change, hlc)?;
                Ok(true)
            }
            Action::Create | Action::Update => {
                let exists = row_exists(tx, library, entity, &change.id)?;
                if !exists && change.action == Action::Update {
                    return Ok(false); // deleted in cascade with its parent
                }
                let mut newer = Map::new();
                for (field, value) in &without_deleted_references(tx, library, change)? {
                    let stored = field_version(tx, library, entity, &change.id, field)?;
                    if stored.as_deref().is_none_or(|stored| stored < hlc) {
                        newer.insert(field.clone(), value.clone());
                    }
                }
                if !exists {
                    ops::insert_row(tx, entity, library, &change.id, newer.iter())?;
                } else if newer.is_empty() {
                    return Ok(false);
                } else {
                    ops::update_row(tx, entity, library, &change.id, newer.iter())?;
                }
                let written = Change {
                    fields: newer,
                    ..change.clone()
                };
                ops::write_versions(tx, library, &written, hlc)?;
                Ok(true)
            }
        }
    })();

    match outcome {
        Ok(applied) => {
            tx.execute_batch("RELEASE change")?;
            Ok(applied)
        }
        Err(crate::error::Error::Sqlite(_)) => {
            tx.execute_batch("ROLLBACK TO change; RELEASE change")?;
            Ok(false)
        }
        Err(other) => {
            tx.execute_batch("ROLLBACK TO change; RELEASE change")?;
            Err(other)
        }
    }
}

/// The change's fields, with every optional reference to a row that was deleted set
/// to null: the delete wins, and the rest of the change still applies, so the order
/// the changes arrive in does not matter (D89). A required reference is left alone:
/// its row would have been deleted with the one it points to anyway.
fn without_deleted_references(
    conn: &Connection,
    library: &str,
    change: &Change,
) -> Result<Map<String, Value>> {
    let mut fields = change.fields.clone();
    for (name, value) in &mut fields {
        if let Some(FieldSpec {
            field_type: FieldType::Ref(target),
            required: false,
            ..
        }) = change.entity.field(name)
            && let Some(id) = value.as_str()
            && is_tombstoned(conn, library, *target, id)?
        {
            *value = Value::Null;
        }
    }
    Ok(fields)
}

/// Another operation already holds this HLC: only a modified app signs two with one
/// time, and logging the second would fail the whole batch.
fn hlc_taken(conn: &Connection, operation: &Operation) -> Result<bool> {
    let hlc = operation.hlc.to_string();
    Ok(conn
        .query_row("SELECT 1 FROM operations WHERE hlc = ?1", [hlc], |_| Ok(()))
        .optional()?
        .is_some())
}

fn is_known(conn: &Connection, operation_id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM operations WHERE id = ?1",
            [operation_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn is_tombstoned(conn: &Connection, library: &str, entity: Entity, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM tombstones WHERE library_id = ?1 AND entity = ?2 AND entity_id = ?3",
            params![library, entity.table(), id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn row_exists(conn: &Connection, library: &str, entity: Entity, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT 1 FROM {} WHERE {} = ?1 AND {} = ?2",
                entity.table(),
                entity.library_column(),
                entity.id_column()
            ),
            [library, id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn field_version(
    conn: &Connection,
    library: &str,
    entity: Entity,
    id: &str,
    field: &str,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT hlc FROM field_versions
              WHERE library_id = ?1 AND entity = ?2 AND entity_id = ?3 AND field = ?4",
            params![library, entity.table(), id, field],
            |row| row.get(0),
        )
        .optional()?)
}

fn to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => json!(i),
        ValueRef::Real(f) => json!(f),
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => json!(String::from_utf8_lossy(bytes)),
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook, ValidEdition};

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::series;

    /// A simulated device: its own database, as a phone or a laptop would have, and
    /// the library it works in (`.1`). A library is born on one device and reaches
    /// the others by syncing (D85): `new` is the device it is born on, `sharing`
    /// and `empty_for` are devices without one of their own.
    struct Device(Connection, String);

    impl Device {
        /// A device on its first start: it has a library of its own.
        fn new() -> Self {
            let conn = db::open_in_memory().unwrap();
            let library = ops::active_library(&conn).unwrap();
            Device(conn, library)
        }

        /// A device with no library at all and nothing in its log.
        fn blank() -> Self {
            let mut device = Device::new();
            let tx = device.0.transaction().unwrap();
            ops::delete_library(&tx, &device.1).unwrap();
            tx.execute("DELETE FROM operations", []).unwrap();
            ops::forget_active_library(&tx).unwrap();
            tx.commit().unwrap();
            device.1.clear();
            device
        }

        /// A blank device that will work in the library `origin` works in, once it
        /// arrives.
        fn empty_for(origin: &Device) -> Self {
            let mut device = Device::blank();
            device.1 = origin.1.clone();
            device
        }

        /// `empty_for`, with the library already received: `origin` as it is now.
        fn sharing(origin: &Device) -> Self {
            let mut device = Device::empty_for(origin);
            let mine = version_vectors(&device.0, std::slice::from_ref(&origin.1)).unwrap();
            let missing = operations_since(&origin.0, &mine).unwrap();
            merge(&mut device.0, missing).unwrap();
            device
        }

        fn add(&mut self, title: &str, isbn: Option<&str>, owner: &str) -> String {
            catalog::create_book(&mut self.0, &self.1, &book(title, isbn, owner), None)
                .unwrap()
                .edition_id
        }

        fn titles(&self) -> Vec<String> {
            catalog::list_books(&self.0, &self.1, &BookFilter::default())
                .unwrap()
                .into_iter()
                .map(|b| b.title)
                .collect()
        }

        /// Pull what `other` has and this device lacks.
        fn pull_from(&mut self, other: &Device) -> MergeReport {
            // What the real app reads from library_peers: every library either side knows,
            // deleted ones included, so the delete itself can travel.
            let mut theirs = known_libraries(&other.0);
            theirs.extend(known_libraries(&self.0));
            theirs.sort();
            theirs.dedup();
            let mine = version_vectors(&self.0, &theirs).unwrap();
            let missing = operations_since(&other.0, &mine).unwrap();
            merge(&mut self.0, missing).unwrap()
        }

        /// A full exchange, both ways, as a real sync session does.
        fn sync_with(&mut self, other: &mut Device) {
            self.pull_from(other);
            other.pull_from(self);
        }

        /// Every catalogue row of every library, for comparing devices. Creation
        /// times are local, so left out.
        fn state(&self) -> State {
            state(&self.0)
        }
    }

    fn book(title: &str, isbn: Option<&str>, owner: &str) -> ValidBook {
        ValidBook {
            edition: edition(title, isbn),
            copy: CopyInput {
                owner: Some(owner.into()),
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

    fn edition(title: &str, isbn: Option<&str>) -> ValidEdition {
        EditionInput {
            title: title.into(),
            authors: vec!["Ursula K. Le Guin".into()],
            isbn: isbn.map(str::to_owned),
            ..EditionInput::default()
        }
        .validate()
        .unwrap()
    }

    /// Rows per library and table.
    type State = Vec<(String, String, Vec<Map<String, Value>>)>;

    fn known_libraries(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT id FROM libraries UNION SELECT library_id FROM operations")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    fn state(conn: &Connection) -> State {
        let mut state = Vec::new();
        for library in all_libraries(conn) {
            for entity in ORDER {
                let rows = entity_rows(conn, &library, entity).unwrap();
                if !rows.is_empty() {
                    state.push((library.clone(), entity.table().to_owned(), rows));
                }
            }
        }
        state
    }

    /// Libraries with any row, whether or not their library row has arrived.
    fn all_libraries(conn: &Connection) -> Vec<String> {
        let mut statement = conn
            .prepare(
                "SELECT id FROM libraries UNION SELECT library_id FROM members
                  UNION SELECT library_id FROM editions
                  UNION SELECT library_id FROM operations
                  ORDER BY 1",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// Vectors of a device that has nothing of `library` yet.
    fn everything(library: &str) -> LibraryVectors {
        LibraryVectors::from([(library.to_owned(), VersionVector::new())])
    }

    fn assert_converged(devices: &[&Device]) {
        let first = devices[0].state();
        for other in &devices[1..] {
            assert_eq!(other.state(), first, "devices diverged");
        }
        for device in devices {
            assert_rebuild_changes_nothing(device);
        }
    }

    /// Rebuilding a library from its log must give back exactly what incremental
    /// merging produced: it is what a device does when roles change late (D73).
    /// Checked inside a transaction that is rolled back, so nothing changes.
    fn assert_rebuild_changes_nothing(device: &Device) {
        let before = device.state();
        let tx = device.0.unchecked_transaction().unwrap();
        for library in known_libraries(&tx) {
            permissions::rebuild(&tx, &library).unwrap();
        }
        assert_eq!(
            state(&tx),
            before,
            "rebuilding from the log changed the library"
        );
        tx.rollback().unwrap();
    }

    /// Everything `device` has that an empty device would lack, ready to merge.
    fn everything_from(device: &Device) -> Vec<SyncOperation> {
        let libraries = known_libraries(&device.0);
        let empty = version_vectors(&db::open_in_memory().unwrap(), &libraries).unwrap();
        operations_since(&device.0, &empty).unwrap()
    }

    fn device_id_of(device: &Device) -> String {
        ops::device_id(&device.0).unwrap()
    }

    fn registered_keys(device: &Device) -> Vec<(String, String)> {
        device
            .0
            .prepare("SELECT id, public_key FROM devices WHERE library_id = ?1 ORDER BY id")
            .unwrap()
            .query_map([&device.1], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn signed_operations_are_checked_even_through_a_third_device() {
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        let mut tablet = Device::empty_for(&laptop);
        laptop.add("A Wizard of Earthsea", None, "Ana");

        phone.pull_from(&laptop);
        // The tablet never talks to the laptop: it checks the laptop's signature with
        // the key that came inside the library.
        let report = tablet.pull_from(&phone);

        // The laptop's key registration, the library it created and its book.
        assert_eq!((report.received, report.rejected), (3, 0));
        assert_converged(&[&laptop, &phone, &tablet]);
        let laptop_key = ops::signing_key(&laptop.0).unwrap().public_key();
        assert_eq!(
            registered_keys(&tablet),
            vec![(device_id_of(&laptop), laptop_key)]
        );
        assert!(
            everything_from(&tablet)
                .iter()
                .all(|op| op.operation.signature.is_some())
        );
    }

    #[test]
    fn an_operation_changed_on_the_way_is_refused() {
        let mut laptop = Device::new();
        let mut phone = Device::empty_for(&laptop);
        laptop.add("A Wizard of Earthsea", None, "Ana");
        let mut operations = everything_from(&laptop);
        let book = operations
            .iter_mut()
            .find(|op| op.operation.intent == "book.add")
            .unwrap();
        let fields = &mut book.operation.changes.last_mut().unwrap().fields;
        fields.insert("quantity".into(), json!(99));

        let report = merge(&mut phone.0, operations).unwrap();

        // The laptop's registration and its library arrive; its book, changed, does not.
        assert_eq!((report.received, report.rejected), (2, 1));
        assert!(phone.titles().is_empty());
    }

    #[test]
    fn nobody_can_sign_in_the_name_of_another_device() {
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        laptop.add("A Wizard of Earthsea", None, "Ana");
        phone.pull_from(&laptop);
        // An impostor makes an operation that claims to be the laptop's.
        let impostor = kosmotheka_domain::DeviceKey::from_seed([9; 32]);
        let mut forged = everything_from(&laptop)
            .into_iter()
            .find(|op| op.operation.intent == "book.add")
            .unwrap();
        forged.operation.id = ops::new_id();
        forged.operation.hlc.millis += 1;
        forged
            .operation
            .changes
            .retain(|c| c.entity == Entity::Member);
        forged.operation.changes[0]
            .fields
            .insert("name".into(), json!("Mallory"));
        forged.operation.changes[0].action = Action::Update;
        forged.operation.signature = Some(impostor.sign(&forged.operation));

        let report = merge(&mut phone.0, vec![forged]).unwrap();

        assert_eq!(report.rejected, 1);
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_device_key_cannot_be_replaced() {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        laptop.add("A Wizard of Earthsea", None, "Ana");
        let mut phone = Device::sharing(&laptop);
        phone.pull_from(&laptop);
        let laptop_id = device_id_of(&laptop);
        // The phone, legitimately signing its own operation, tries to swap the laptop's key.
        let swap = Change::update(
            Entity::Device,
            &laptop_id,
            json!({ "public_key": ops::signing_key(&phone.0).unwrap().public_key() }),
        );
        let result = {
            let tx = phone.0.transaction().unwrap();
            let op = ops::record(&tx, &lib, "device.swap", vec![swap]).unwrap();
            tx.rollback().unwrap();
            op
        };
        let mut other = Device::sharing(&laptop);
        other.pull_from(&laptop);

        let schema_version = ops::schema_version(&other.0).unwrap();

        let report = merge(
            &mut other.0,
            vec![SyncOperation {
                schema_version,
                operation: result,
            }],
        )
        .unwrap();

        assert_eq!(report.rejected, 1);
        assert_converged(&[&laptop, &other]);
    }

    #[test]
    fn an_unsigned_operation_is_refused() {
        // D56: every operation is signed; nobody can tell who wrote one that is not.
        let mut laptop = Device::new();
        let mut phone = Device::empty_for(&laptop);
        laptop.add("A Wizard of Earthsea", None, "Ana");
        let mut operations = everything_from(&laptop);
        for op in &mut operations {
            if op.operation.intent != "device.register" {
                op.operation.signature = None;
            }
        }

        let report = merge(&mut phone.0, operations).unwrap();

        // The registration is accepted (it is how a key becomes known); the library and
        // the book, unsigned, are not.
        assert_eq!((report.received, report.rejected), (1, 2));
        assert!(phone.titles().is_empty());
    }

    #[test]
    fn books_added_offline_on_two_devices_end_up_on_both() {
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        laptop.add("A Wizard of Earthsea", None, "Ana");
        phone.add("The Dispossessed", None, "Bea");

        laptop.sync_with(&mut phone);

        assert_eq!(
            laptop.titles(),
            vec!["A Wizard of Earthsea", "The Dispossessed"]
        );
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn merging_operations_from_another_device_does_not_wake_the_local_change_notifier() {
        let mut a = Device::new();
        let mut b = Device::sharing(&a);
        a.add("A", None, "Ana");

        let before = ops::recorded_on_this_thread();
        let report = b.pull_from(&a);

        assert!(report.received > 0, "something was merged");
        assert_eq!(ops::recorded_on_this_thread(), before);
    }

    #[test]
    fn syncing_again_changes_nothing() {
        let mut a = Device::new();
        let mut b = Device::sharing(&a);
        a.add("A", None, "Ana");
        a.sync_with(&mut b);

        let again = b.pull_from(&a);

        assert_eq!(again, MergeReport::default());
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn later_edit_of_the_same_field_wins_everywhere() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        let id = a.add("Draft", None, "Ana");
        a.sync_with(&mut b);

        catalog::update_edition(&mut a.0, &lib, &id, &edition("From A", None), None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        catalog::update_edition(&mut b.0, &lib, &id, &edition("From B", None), None).unwrap();
        a.sync_with(&mut b);

        assert_eq!(a.titles(), vec!["From B"]);
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn edits_of_different_fields_are_both_kept() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        let id = a.add("Book", None, "Ana");
        a.sync_with(&mut b);

        let mut year = edition("Book", None);
        year.published_year = Some(1968);
        catalog::update_edition(&mut a.0, &lib, &id, &year, None).unwrap();
        let mut language = edition("Book", None);
        language.language = Some("en".into());
        catalog::update_edition(&mut b.0, &lib, &id, &language, None).unwrap();
        a.sync_with(&mut b);

        let detail = catalog::get_book(&a.0, &lib, &id).unwrap();
        assert_eq!(detail.published_year, Some(1968));
        assert_eq!(detail.language.as_deref(), Some("en"));
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn a_delete_beats_a_concurrent_edit() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        let id = a.add("Doomed", None, "Ana");
        a.sync_with(&mut b);

        catalog::delete_book(&mut a.0, &lib, &id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        catalog::update_edition(&mut b.0, &lib, &id, &edition("Edited later", None), None).unwrap();
        catalog::add_copy(&mut b.0, &lib, &id, &book("x", None, "Bea").copy).unwrap();
        a.sync_with(&mut b);

        assert!(a.titles().is_empty());
        assert!(b.titles().is_empty());
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn a_tag_merge_from_renaming_converges_like_any_other_operation() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);

        let mut tagged = edition("Tagged", None);
        tagged.tags = vec!["favorite".into(), "gift".into()];
        let id = catalog::create_book(
            &mut a.0,
            &lib,
            &ValidBook {
                edition: tagged,
                copy: book("Tagged", None, "Ana").copy,
            },
            None,
        )
        .unwrap()
        .edition_id;
        a.sync_with(&mut b);

        let favorite_id = catalog::list_tags(&a.0, &lib)
            .unwrap()
            .into_iter()
            .find(|t| t.name == "favorite")
            .unwrap()
            .id;
        catalog::rename_tag(&mut a.0, &lib, &favorite_id, "gift").unwrap();
        a.sync_with(&mut b);

        for device in [&a, &b] {
            let detail = catalog::get_book(&device.0, &lib, &id).unwrap();
            assert_eq!(detail.tags, vec!["gift"]);
            assert_eq!(catalog::list_tags(&device.0, &lib).unwrap().len(), 1);
        }
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn the_same_book_added_on_both_devices_stays_duplicated() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        a.add("Same", Some("9780306406157"), "Ana");
        b.add("Same", Some("9780306406157"), "Ana");

        a.sync_with(&mut b);

        assert_eq!(a.titles(), vec!["Same", "Same"]);
        let options = catalog::catalog_options(&a.0, &lib).unwrap();
        assert_eq!(
            options.authors,
            vec!["Ursula K. Le Guin"],
            "names are listed once"
        );
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn operations_travel_through_a_third_device() {
        let mut a = Device::new();
        let mut b = Device::sharing(&a);
        let mut c = Device::sharing(&a);
        a.add("From A", None, "Ana");
        c.add("From C", None, "Carla");

        a.sync_with(&mut b);
        b.sync_with(&mut c);
        a.sync_with(&mut b);

        assert_eq!(c.titles(), vec!["From A", "From C"]);
        assert_converged(&[&a, &b, &c]);
    }

    #[test]
    fn rows_from_before_the_operation_log_travel_in_the_baseline() {
        let mut a = Device::new();
        let mut b = Device::sharing(&a);
        a.0.execute_batch(&format!(
            "INSERT INTO works (library_id, id, kind, title)
             VALUES ('{lib}', '01997c7e-0000-7000-8000-000000000001', 'book', 'Old');
             INSERT INTO editions (library_id, id, work_id, title)
             VALUES ('{lib}', '01997c7e-0000-7000-8000-000000000002',
                     '01997c7e-0000-7000-8000-000000000001', 'Old');",
            lib = a.1
        ))
        .unwrap();
        a.0.execute("DELETE FROM local_state WHERE key = ?1", [BASELINE_KEY])
            .unwrap();
        ensure_baseline(&mut a.0).unwrap();

        b.pull_from(&a);

        assert_eq!(b.titles(), vec!["Old"]);
        let baselines: i64 =
            a.0.query_row(
                "SELECT count(*) FROM operations WHERE replicable = 1 AND intent = 'sync.baseline'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(baselines, 1);
    }

    #[test]
    fn history_from_before_sync_is_not_replicated() {
        let mut a = Device::new();
        let lib = a.1.clone();
        a.add("A", None, "Ana");
        a.0.execute("UPDATE operations SET replicable = 0", [])
            .unwrap();
        let b = Device::sharing(&a);

        assert!(
            operations_since(
                &a.0,
                &version_vectors(&b.0, std::slice::from_ref(&lib)).unwrap()
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn operations_from_an_older_schema_version_are_accepted() {
        let mut a = Device::new();
        let mut b = Device::empty_for(&a);
        a.add("A", None, "Ana");
        let mut incoming = operations_since(&a.0, &everything(&a.1)).unwrap();
        for op in &mut incoming {
            op.schema_version -= 1;
        }

        let report = merge(&mut b.0, incoming).unwrap();

        assert_eq!(report.rejected, 0);
        assert_eq!(b.titles(), vec!["A"]);
    }

    #[test]
    fn operations_from_a_newer_schema_version_are_rejected() {
        let mut a = Device::new();
        let mut b = Device::empty_for(&a);
        a.add("A", None, "Ana");
        let mut incoming = operations_since(&a.0, &everything(&a.1)).unwrap();
        for op in &mut incoming {
            op.schema_version += 1;
        }
        let sent = incoming.len();

        let report = merge(&mut b.0, incoming).unwrap();

        assert_eq!(report.rejected, sent);
        assert!(b.titles().is_empty());
    }

    #[test]
    fn a_device_paired_after_an_app_update_gets_the_old_history() {
        let mut a = Device::new();
        a.add("A", None, "Ana");
        a.add("B", None, "Ana");
        // What operations recorded before the last migration look like.
        a.0.execute(
            "UPDATE operations SET schema_version = schema_version - 1",
            [],
        )
        .unwrap();
        let mut b = Device::empty_for(&a);

        b.pull_from(&a);

        assert_eq!(b.titles().len(), 2);
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn an_operation_from_a_clock_far_ahead_waits_and_does_not_drag_the_clock() {
        let mut a = Device::new();
        let mut b = Device::empty_for(&a);
        b.pull_from(&a);
        // A's clock says two hours from now: its operations carry that time, signed.
        let ahead = Hlc {
            millis: now_millis() + 2 * 60 * 60 * 1000,
            counter: 0,
            device: ops::device_id(&a.0).unwrap(),
        };
        ops::set_last_hlc(&a.0, &ahead).unwrap();
        a.add("Tomorrow", None, "Ana");

        let report = b.pull_from(&a);

        assert!(report.rejected >= 1);
        assert!(b.titles().is_empty());
        let clock = ops::last_hlc(&b.0).unwrap().unwrap();
        assert!(!clock.is_ahead_of(now_millis()) && clock.millis < ahead.millis);
    }

    #[test]
    fn a_rejected_operation_holds_back_the_later_ones_of_its_author() {
        let mut a = Device::new();
        let mut b = Device::empty_for(&a);
        a.add("One", None, "Ana");
        a.add("Two", None, "Ana");
        let mut incoming = operations_since(&a.0, &everything(&a.1)).unwrap();
        let n = incoming.len();
        // a1 carries a2's signature: it does not verify.
        incoming[n - 2].operation.signature = incoming[n - 1].operation.signature.clone();

        let report = merge(&mut b.0, incoming).unwrap();

        assert!(report.rejected >= 2);
        assert!(b.titles().is_empty());
        let vector = &version_vectors(&b.0, std::slice::from_ref(&a.1)).unwrap()[&a.1];
        let a2 = everything_from(&a).pop().unwrap().operation.hlc;
        assert!(
            vector
                .get(&a2.device)
                .is_none_or(|seen| *seen < a2.to_string())
        );

        b.pull_from(&a);

        assert_eq!(b.titles().len(), 2);
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn receiving_moves_the_local_clock_past_remote_timestamps() {
        let mut a = Device::new();
        let mut b = Device::empty_for(&a);
        a.add("A", None, "Ana");
        let incoming = operations_since(&a.0, &everything(&a.1)).unwrap();
        let remote = incoming[0].operation.hlc.clone();

        merge(&mut b.0, incoming).unwrap();
        b.add("B", None, "Bea");

        let newest: String =
            b.0.query_row(
                "SELECT hlc FROM operations ORDER BY hlc DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(newest.parse::<Hlc>().unwrap() > remote);
    }

    #[test]
    fn synced_books_are_searchable() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        a.add("Tehanu", None, "Ana");

        b.pull_from(&a);

        let filter = BookFilter {
            query: Some("tehanu".into()),
            ..BookFilter::default()
        };
        assert_eq!(catalog::list_books(&b.0, &lib, &filter).unwrap().len(), 1);
    }

    #[test]
    fn missing_works_noted_on_the_phone_reach_the_desktop() {
        let mut desk = Device::new();
        let lib = desk.1.clone();
        let mut phone = Device::sharing(&desk);
        let missing = kosmotheka_domain::EditionInput {
            title: "Tehanu".into(),
            ..Default::default()
        }
        .validate()
        .unwrap();
        catalog::create_missing(&mut phone.0, &lib, &missing, None).unwrap();

        desk.sync_with(&mut phone);

        let titles: Vec<String> = catalog::list_books(&desk.0, &lib, &BookFilter::default())
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect();
        assert_eq!(titles, vec!["Tehanu"]);
        assert_converged(&[&desk, &phone]);
    }

    /// An entity dropped from the schema (like `wish`, D61) may still sit in an old
    /// local log. Sending it as-is with the unknown change simply left out would no
    /// longer match its signature, so the whole operation stays home; every other
    /// operation is unaffected.
    #[test]
    fn an_operation_with_an_unknown_entity_is_not_sent_but_the_rest_still_converges() {
        let mut desk = Device::new();
        let mut phone = Device::sharing(&desk);
        phone.add("Tehanu", None, "Ana");

        let phone_id = device_id_of(&phone);
        phone
            .0
            .execute(
                "INSERT INTO operations (id, hlc, device_id, intent, changes) VALUES (
                    '01997c7e-0000-7000-8000-000000000998',
                    '0001727222200099-00000000-' || ?1,
                    ?1,
                    'wish.add',
                    '[{\"entity\":\"wish\",\"id\":\"01997c7e-0000-7000-8000-000000000997\",\"action\":\"create\",\"fields\":{\"title\":\"Hijos de Dune\"}}]'
                )",
                [&phone_id],
            )
            .unwrap();

        desk.sync_with(&mut phone);

        assert_eq!(
            desk.titles(),
            vec!["Tehanu"],
            "the normal operation arrives"
        );
        let sent: i64 = desk
            .0
            .query_row(
                "SELECT count(*) FROM operations
                  WHERE id = '01997c7e-0000-7000-8000-000000000998'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            sent, 0,
            "the operation with the unknown entity is never sent"
        );
        assert_converged(&[&desk, &phone]);
    }

    #[test]
    fn a_board_game_and_its_details_converge_field_by_field() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        let (work, edition) = (ops::new_id(), ops::new_id());
        let tx = a.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "book.add",
            vec![
                Change::create(
                    Entity::Work,
                    &work,
                    json!({ "kind": "boardgame", "title": "Catan" }),
                ),
                Change::create(
                    Entity::Edition,
                    &edition,
                    json!({ "work_id": work, "title": "Catan", "barcode": "0029877030712" }),
                ),
                Change::create(
                    Entity::BoardgameDetails,
                    &edition,
                    json!({ "players_min": 3, "players_max": 4 }),
                ),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
        a.sync_with(&mut b);

        // Each device fixes a different field of the same details.
        for (device, field, value) in [(&mut a, "players_max", 6), (&mut b, "duration_minutes", 90)]
        {
            let tx = device.0.transaction().unwrap();
            ops::record(
                &tx,
                &lib,
                "book.edit",
                vec![Change::update(
                    Entity::BoardgameDetails,
                    &edition,
                    json!({ field: value }),
                )],
            )
            .unwrap();
            tx.commit().unwrap();
        }
        a.sync_with(&mut b);

        assert_converged(&[&a, &b]);
        let (max, minutes): (i64, i64) =
            b.0.query_row(
                "SELECT players_max, duration_minutes FROM boardgame_details",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((max, minutes), (6, 90));
    }

    #[test]
    fn deleting_a_library_empties_it_on_every_device() {
        let mut a = Device::new();
        let lib = a.1.clone();
        let mut b = Device::sharing(&a);
        a.add("Doomed", None, "Ana");
        a.sync_with(&mut b);

        let tx = a.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "library.delete",
            vec![Change::delete(Entity::Library, &lib)],
        )
        .unwrap();
        tx.commit().unwrap();
        b.add("Added meanwhile", None, "Bea");
        a.sync_with(&mut b);

        assert!(a.titles().is_empty());
        assert!(b.titles().is_empty());
        assert_converged(&[&a, &b]);
        let kept: String =
            a.0.query_row(
                "SELECT intent FROM operations WHERE intent = 'library.delete'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, "library.delete", "the delete stays on record (D66)");
    }

    #[test]
    fn a_new_library_and_its_books_reach_the_other_device() {
        let mut a = Device::new();
        let mut b = Device::sharing(&a);
        let rol = crate::libraries::create_library(&mut a.0, "Rol").unwrap();
        catalog::create_book(&mut a.0, &rol, &book("Earthsea", None, "Ana"), None).unwrap();
        a.sync_with(&mut b);

        crate::libraries::rename_library(&mut b.0, &rol, "Rol de los jueves").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        a.sync_with(&mut b);

        let name: String =
            a.0.query_row("SELECT name FROM libraries WHERE id = ?1", [&rol], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(name, "Rol de los jueves");
        assert_converged(&[&a, &b]);
    }

    /// Merge cases shared with every other implementation of the spec (the PHP
    /// server included): every delivery order must end in the same rows.
    const MERGE_CASES: [&str; 2] = [
        include_str!("../../../spec/conformance/library-merge.json"),
        include_str!("../../../spec/conformance/merge.json"),
    ];

    #[derive(serde::Deserialize)]
    struct MergeCase {
        name: String,
        operations: Vec<Operation>,
        deliveries: Vec<Vec<usize>>,
        expect: ExpectedRows,
    }

    #[derive(serde::Deserialize)]
    struct ExpectedRows {
        rows: Vec<ExpectedRow>,
        /// When present, the ids of the operations the merge refuses (not stored) and
        /// that every other one is taken. Absent: the cases do not say.
        rejected: Option<Vec<String>>,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct ExpectedRow {
        library: String,
        entity: String,
        id: String,
        fields: std::collections::BTreeMap<String, Value>,
    }

    #[test]
    fn a_reader_cannot_write_on_their_own_device_either() {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        // Both register their keys by writing something before roles exist.
        laptop.add("Dune", None, "Tersoal");
        phone.add("Tehanu", None, "Hugo");
        laptop.sync_with(&mut phone);
        let (laptop_id, phone_id) = (device_id_of(&laptop), device_id_of(&phone));
        let (admin, reader) = (ops::new_id(), ops::new_id());
        let tx = laptop.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.roles",
            vec![
                Change::create(
                    Entity::Member,
                    &admin,
                    json!({ "name": "Tersoal", "role": "admin" }),
                ),
                Change::create(
                    Entity::Member,
                    &reader,
                    json!({ "name": "Hugo", "role": "reader" }),
                ),
                Change::update(Entity::Device, &laptop_id, json!({ "member_id": admin })),
                Change::update(Entity::Device, &phone_id, json!({ "member_id": reader })),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
        phone.pull_from(&laptop);

        let refused =
            catalog::create_book(&mut phone.0, &lib, &book("Wingspan", None, "Hugo"), None);

        assert!(
            matches!(refused, Err(crate::error::Error::NotAllowed)),
            "{refused:?}"
        );
        assert!(!phone.titles().contains(&"Wingspan".to_owned()));
        laptop.add("The Dispossessed", None, "Tersoal");
        assert!(laptop.titles().contains(&"The Dispossessed".to_owned()));
    }

    /// A library where the laptop is Tersoal's (admin) and the phone Hugo's (reader).
    fn laptop_admin_phone_reader() -> (Device, Device) {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        laptop.add("Dune", None, "Tersoal");
        phone.add("Tehanu", None, "Hugo");
        laptop.sync_with(&mut phone);
        let (laptop_id, phone_id) = (device_id_of(&laptop), device_id_of(&phone));
        let (admin, reader) = (ops::new_id(), ops::new_id());
        let tx = laptop.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.roles",
            vec![
                Change::create(
                    Entity::Member,
                    &admin,
                    json!({ "name": "Tersoal", "role": "admin" }),
                ),
                Change::create(
                    Entity::Member,
                    &reader,
                    json!({ "name": "Hugo", "role": "reader" }),
                ),
                Change::update(Entity::Device, &laptop_id, json!({ "member_id": admin })),
                Change::update(Entity::Device, &phone_id, json!({ "member_id": reader })),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
        phone.pull_from(&laptop);
        (laptop, phone)
    }

    #[test]
    fn a_demoted_editor_cannot_backdate_a_change() {
        // D84, «Abierto»: the role is judged at the operation's HLC, and the author
        // sets the HLC. Hugo is an editor; the laptop lowers him to reader. His
        // phone, with a modified app, then signs an edit dated just before that.
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let hugo = phone_person(&laptop, &phone);
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("editor"), false).unwrap();
        phone.pull_from(&laptop);
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("reader"), false).unwrap();
        let lowered = ops::last_hlc(&laptop.0).unwrap().unwrap();
        let work: String = laptop
            .0
            .query_row(
                "SELECT id FROM works WHERE library_id = ?1 AND title = 'Dune'",
                [&lib],
                |row| row.get(0),
            )
            .unwrap();

        let mut backdated = Operation {
            id: ops::new_id(),
            library: lib.clone(),
            hlc: kosmotheka_domain::Hlc {
                millis: lowered.millis - 1,
                counter: 0,
                device: device_id_of(&phone),
            },
            intent: "book.edit".into(),
            changes: vec![Change::update(
                Entity::Work,
                &work,
                json!({ "title": "Backdated" }),
            )],
            previous: ops::latest_of(&phone.0, &lib, &device_id_of(&phone)).unwrap(),
            signature: None,
        };
        backdated.signature = Some(ops::signing_key(&phone.0).unwrap().sign(&backdated));
        let schema_version = ops::schema_version(&laptop.0).unwrap();
        let report = merge(
            &mut laptop.0,
            vec![SyncOperation {
                schema_version,
                operation: backdated,
            }],
        )
        .unwrap();
        let title: String = laptop
            .0
            .query_row(
                "SELECT title FROM works WHERE library_id = ?1 AND id = ?2",
                [&lib, &work],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(report.rejected, 0, "signed by the phone, so it is valid");
        assert_eq!(
            (title.as_str(), report.requests),
            ("Dune", 1),
            "a change dated before the demotion but made after it must not apply"
        );
    }

    /// The status of the operation that made `title`, as this device knows it.
    fn status_of_edit_titled(device: &Device, title: &str) -> String {
        device
            .0
            .query_row(
                "SELECT status FROM operations
                  WHERE library_id = ?1 AND changes LIKE ?2 AND intent <> 'device.register'",
                params![device.1, format!("%\"{title}\"%")],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn an_offline_edit_made_before_hearing_of_the_demotion_becomes_a_request() {
        // The honest case the fence costs (D84): Hugo's phone edits offline, the
        // laptop lowers him without having seen that edit, and then they sync.
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let hugo = phone_person(&laptop, &phone);
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("editor"), false).unwrap();
        phone.pull_from(&laptop);
        phone.add("Made offline", None, "Hugo");
        std::thread::sleep(std::time::Duration::from_millis(5));
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("reader"), false).unwrap();

        laptop.sync_with(&mut phone);

        for device in [&laptop, &phone] {
            assert!(!device.titles().contains(&"Made offline".to_owned()));
            assert_eq!(status_of_edit_titled(device, "Made offline"), "request");
        }
        let notices: i64 = phone
            .0
            .query_row("SELECT count(*) FROM request_notices", [], |row| row.get(0))
            .unwrap();
        assert_eq!(notices, 1, "the phone is told its edit became a request");
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_revoked_device_cannot_backdate_a_change_either() {
        // The revocation carries a fence like a demotion does (D71, D84).
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let hugo = phone_person(&laptop, &phone);
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("editor"), false).unwrap();
        phone.pull_from(&laptop);
        let phone_id = device_id_of(&phone);
        crate::libraries::revoke_access(&mut laptop.0, &lib, &phone_id).unwrap();
        let revoked = ops::last_hlc(&laptop.0).unwrap().unwrap();
        let work: String = laptop
            .0
            .query_row(
                "SELECT id FROM works WHERE library_id = ?1 AND title = 'Dune'",
                [&lib],
                |row| row.get(0),
            )
            .unwrap();
        let mut backdated = Operation {
            id: ops::new_id(),
            library: lib.clone(),
            hlc: kosmotheka_domain::Hlc {
                millis: revoked.millis - 1,
                counter: 0,
                device: phone_id,
            },
            intent: "book.edit".into(),
            changes: vec![Change::update(
                Entity::Work,
                &work,
                json!({ "title": "Backdated" }),
            )],
            previous: ops::latest_of(&phone.0, &lib, &device_id_of(&phone)).unwrap(),
            signature: None,
        };
        backdated.signature = Some(ops::signing_key(&phone.0).unwrap().sign(&backdated));
        let schema_version = ops::schema_version(&laptop.0).unwrap();
        let report = merge(
            &mut laptop.0,
            vec![SyncOperation {
                schema_version,
                operation: backdated,
            }],
        )
        .unwrap();

        assert_eq!(report.requests, 1);
        assert!(!laptop.titles().contains(&"Backdated".to_owned()));
        assert_converged(&[&laptop]);
    }

    /// Records an edit of `work`'s title on `device`, as its own app would.
    fn edit_title(device: &mut Device, work: &str, title: &str) -> Operation {
        let tx = device.0.transaction().unwrap();
        let operation = ops::record(
            &tx,
            &device.1,
            "book.edit",
            vec![Change::update(
                Entity::Work,
                work,
                json!({ "title": title }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        operation
    }

    /// An edit of `work` signed by `device` with a modified app: any HLC, any previous.
    fn forged_edit(
        device: &Device,
        work: &str,
        title: &str,
        millis: u64,
        previous: Option<&str>,
    ) -> SyncOperation {
        let mut operation = Operation {
            id: ops::new_id(),
            library: device.1.clone(),
            hlc: kosmotheka_domain::Hlc {
                millis,
                counter: 0,
                device: device_id_of(device),
            },
            intent: "book.edit".into(),
            changes: vec![Change::update(
                Entity::Work,
                work,
                json!({ "title": title }),
            )],
            previous: previous.map(str::to_owned),
            signature: None,
        };
        operation.signature = Some(ops::signing_key(&device.0).unwrap().sign(&operation));
        SyncOperation {
            schema_version: ops::schema_version(&device.0).unwrap(),
            operation,
        }
    }

    fn vector_of(device: &Device, author: &Device) -> Option<String> {
        version_vectors(&device.0, std::slice::from_ref(&device.1)).unwrap()[&device.1]
            .get(&device_id_of(author))
            .cloned()
    }

    #[test]
    fn an_operation_whose_previous_is_missing_waits_for_it() {
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        let one = phone.add("One", None, "Hugo");
        let two = phone.add("Two", None, "Hugo");
        let _ = (one, two);
        let first = ops::latest_of(&phone.0, &phone.1, &device_id_of(&phone)).unwrap();
        let mut batch = everything_from(&phone);
        // Take the operation that added "One" out of the batch: "Two" chains to it.
        batch.retain(|synced| {
            !serde_json::to_string(&synced.operation.changes)
                .unwrap()
                .contains("\"One\"")
        });
        let held = batch.len();
        assert!(held > 0);

        let report = merge(&mut laptop.0, batch).unwrap();

        assert!(report.rejected > 0, "{report:?}");
        assert!(!laptop.titles().contains(&"Two".to_owned()));
        let newest_of_phone = vector_of(&laptop, &phone).unwrap();
        let two_hlc: String = phone
            .0
            .query_row(
                "SELECT hlc FROM operations WHERE id = ?1",
                [first.unwrap()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            newest_of_phone < two_hlc,
            "the vector must stay below the operation that waits"
        );

        laptop.sync_with(&mut phone);
        assert!(laptop.titles().contains(&"One".to_owned()));
        assert!(laptop.titles().contains(&"Two".to_owned()));
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn two_operations_with_one_hlc_do_not_break_the_merge() {
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        laptop.add("Dune", None, "Tersoal");
        laptop.sync_with(&mut phone);
        let work = work_titled(&laptop, "Dune");
        let last = ops::latest_of(&laptop.0, &laptop.1, &device_id_of(&laptop)).unwrap();
        // A modified app signs two different operations with the same time and counter.
        let millis = now_millis();
        let first = forged_edit(&laptop, &work, "Arrakis", millis, last.as_deref());
        let twin = forged_edit(&laptop, &work, "Caladan", millis, last.as_deref());

        let report = merge(&mut phone.0, vec![first, twin]).unwrap();

        assert_eq!((report.received, report.rejected), (1, 1), "{report:?}");
        assert_eq!(phone.titles().len(), 1);
    }

    #[test]
    fn a_previous_that_is_not_an_earlier_operation_of_the_same_author_and_library_is_refused() {
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        let mut tablet = Device::sharing(&laptop);
        laptop.add("Dune", None, "Tersoal");
        let work = work_titled(&laptop, "Dune");
        laptop.sync_with(&mut phone);
        phone.add("Tehanu", None, "Hugo");
        let other_library = crate::libraries::create_library(&mut laptop.0, "Other").unwrap();
        let in_other_library = ops::latest_of(&laptop.0, &other_library, &device_id_of(&laptop))
            .unwrap()
            .unwrap();
        let by_another_author = ops::latest_of(&laptop.0, &laptop.1, &device_id_of(&laptop))
            .unwrap()
            .unwrap();
        let own_latest = ops::latest_of(&phone.0, &phone.1, &device_id_of(&phone))
            .unwrap()
            .unwrap();
        let own_latest_millis: u64 = phone
            .0
            .query_row(
                "SELECT hlc FROM operations WHERE id = ?1",
                [&own_latest],
                |row| row.get::<_, String>(0),
            )
            .unwrap()
            .parse::<Hlc>()
            .unwrap()
            .millis;
        tablet.pull_from(&laptop);
        tablet.pull_from(&phone);
        let later = own_latest_millis + 10;

        for (name, forged) in [
            (
                "another author",
                forged_edit(&phone, &work, "Forged", later, Some(&by_another_author)),
            ),
            (
                "another library",
                forged_edit(&phone, &work, "Forged", later, Some(&in_other_library)),
            ),
            (
                "not older than the operation",
                forged_edit(
                    &phone,
                    &work,
                    "Forged",
                    own_latest_millis,
                    Some(&own_latest),
                ),
            ),
        ] {
            let before = tablet.state();
            let report = merge(&mut tablet.0, vec![forged]).unwrap();
            assert_eq!(report.rejected, 1, "{name}: {report:?}");
            assert_eq!(report.received, 0, "{name}");
            assert_eq!(tablet.state(), before, "{name}");
        }
        // The honest chain still goes through.
        let honest = forged_edit(&phone, &work, "Honest", later, Some(&own_latest));
        let report = merge(&mut tablet.0, vec![honest]).unwrap();
        assert_eq!(report.received, 1, "{report:?}");
    }

    /// The timeline of SEG-018, with Hugo's phone as the demoted editor. His
    /// admin last saw `known` (an edit of his); with a modified app he signs a new
    /// edit dated before it and chained to an older operation of his own. Returns
    /// the laptop (which demoted him and then received the forgery), the tablet
    /// (which received the forgery before hearing of the demotion) and the id of
    /// the forged operation.
    fn demoted_editor_forks_his_chain() -> (Device, Device, String) {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let hugo = phone_person(&laptop, &phone);
        let work = work_titled(&laptop, "Dune");
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("editor"), false).unwrap();
        phone.pull_from(&laptop);
        let older = edit_title(&mut phone, &work, "Older edit");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let known = edit_title(&mut phone, &work, "Known edit");
        laptop.pull_from(&phone);
        let mut tablet = Device::sharing(&laptop);
        tablet.pull_from(&laptop);

        // Dated below what the administrator knows, chained to an older operation.
        let forged = forged_edit(
            &phone,
            &work,
            "Forged",
            known.hlc.millis - 1,
            Some(&older.id),
        );
        let forged_id = forged.operation.id.clone();
        // The tablet has not heard of the demotion yet: it sees an editor's edit.
        merge(&mut tablet.0, vec![forged.clone()]).unwrap();
        assert_eq!(status_of_id(&tablet, &forged_id), "applied");

        std::thread::sleep(std::time::Duration::from_millis(5));
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("reader"), false).unwrap();
        merge(&mut laptop.0, vec![forged]).unwrap();
        tablet.pull_from(&laptop);
        laptop.pull_from(&tablet);
        (laptop, tablet, forged_id)
    }

    fn status_of_id(device: &Device, id: &str) -> String {
        device
            .0
            .query_row("SELECT status FROM operations WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn a_device_that_left_a_library_and_took_it_up_again_continues_its_own_chain() {
        // Leaving removes the log here; the peers still hold this device's
        // operations and send them back, so the chain is whole again (D87).
        let mut laptop = Device::new();
        let mut phone = Device::sharing(&laptop);
        phone.add("Before leaving", None, "Hugo");
        laptop.sync_with(&mut phone);
        let lib = laptop.1.clone();
        // What `libraries::forget_here` does to the log, without the pairing it needs.
        let tx = phone.0.transaction().unwrap();
        ops::delete_library(&tx, &lib).unwrap();
        tx.execute("DELETE FROM operations WHERE library_id = ?1", [&lib])
            .unwrap();
        tx.commit().unwrap();
        assert!(
            ops::latest_of(&phone.0, &lib, &device_id_of(&phone))
                .unwrap()
                .is_none()
        );

        phone.pull_from(&laptop);
        assert!(phone.titles().contains(&"Before leaving".to_owned()));
        phone.add("After returning", None, "Hugo");
        laptop.sync_with(&mut phone);

        assert!(laptop.titles().contains(&"After returning".to_owned()));
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_fork_dated_below_the_fence_is_a_request_whatever_the_order() {
        let (laptop, tablet, forged) = demoted_editor_forks_his_chain();
        for device in [&laptop, &tablet] {
            assert_eq!(status_of_id(device, &forged), "request");
        }
        assert_converged(&[&laptop, &tablet]);
    }

    #[test]
    fn a_fence_that_arrives_before_its_operation_is_judged_again_when_it_arrives() {
        // The tablet hears of the demotion before it has Hugo's last operation, so
        // at first everything of his looks fenced; when the marked operation comes,
        // the library is judged again and the operations up to it count (D87).
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let mut tablet = Device::sharing(&laptop);
        let hugo = phone_person(&laptop, &phone);
        let work = work_titled(&laptop, "Dune");
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("editor"), false).unwrap();
        phone.pull_from(&laptop);
        let older = edit_title(&mut phone, &work, "Older edit");
        let known = edit_title(&mut phone, &work, "Known edit");
        laptop.pull_from(&phone);
        let phone_ops = [older.id.clone(), known.id.clone()];
        std::thread::sleep(std::time::Duration::from_millis(5));
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("reader"), false).unwrap();
        // Everything the laptop has but what the phone wrote after registering.
        let theirs = version_vectors(&tablet.0, std::slice::from_ref(&lib)).unwrap();
        let from_laptop = operations_since(&laptop.0, &theirs)
            .unwrap()
            .into_iter()
            .filter(|synced| !phone_ops.contains(&synced.operation.id))
            .collect();
        merge(&mut tablet.0, from_laptop).unwrap();

        tablet.pull_from(&phone);

        for id in &phone_ops {
            assert_eq!(status_of_id(&tablet, id), "applied");
        }
        laptop.sync_with(&mut tablet);
        assert_converged(&[&laptop, &tablet]);
    }

    /// The id of the work titled `title` in the default library.
    fn work_titled(device: &Device, title: &str) -> String {
        device
            .0
            .query_row(
                "SELECT id FROM works WHERE library_id = ?1 AND title = ?2",
                [&device.1, title],
                |row| row.get(0),
            )
            .unwrap()
    }

    fn work_title(device: &Device, work: &str) -> String {
        device
            .0
            .query_row(
                "SELECT title FROM works WHERE library_id = ?1 AND id = ?2",
                [&device.1, work],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// The phone, revoked on the laptop but not told yet, edits a title; a tablet
    /// that has not heard of the revocation either passes the edit on. `roles`
    /// says whether the library has roles (the phone is then an editor).
    fn edit_relayed_after_revocation(roles: bool) -> (Device, Device, Device, String) {
        let (mut laptop, mut phone) = if roles {
            let (mut laptop, mut phone) = laptop_admin_phone_reader();
            let hugo = phone_person(&laptop, &phone);
            crate::people::set_role(&mut laptop.0, &laptop.1, &hugo, Some("editor"), false)
                .unwrap();
            phone.pull_from(&laptop);
            (laptop, phone)
        } else {
            let mut laptop = Device::new();
            let mut phone = Device::sharing(&laptop);
            laptop.add("Dune", None, "Tersoal");
            phone.add("Tehanu", None, "Hugo");
            laptop.sync_with(&mut phone);
            (laptop, phone)
        };
        let lib = laptop.1.clone();
        let mut tablet = Device::sharing(&laptop);
        tablet.pull_from(&laptop);
        tablet.pull_from(&phone);
        let work = work_titled(&laptop, "Dune");

        crate::libraries::revoke_access(&mut laptop.0, &lib, &device_id_of(&phone)).unwrap();
        let tx = phone.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "book.edit",
            vec![Change::update(
                Entity::Work,
                &work,
                json!({ "title": "After the revocation" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        tablet.pull_from(&phone);
        laptop.pull_from(&tablet);
        tablet.pull_from(&laptop);
        (laptop, phone, tablet, work)
    }

    #[test]
    fn a_revoked_device_s_edit_relayed_by_a_third_device_does_not_apply() {
        let (laptop, _phone, tablet, work) = edit_relayed_after_revocation(true);
        assert_eq!(work_title(&laptop, &work), "Dune");
        assert_eq!(work_title(&tablet, &work), "Dune");
        assert_converged(&[&laptop, &tablet]);
    }

    #[test]
    fn without_roles_a_revoked_device_s_relayed_edit_does_not_apply_either() {
        let (laptop, _phone, tablet, work) = edit_relayed_after_revocation(false);
        assert_eq!(work_title(&laptop, &work), "Dune");
        assert_eq!(work_title(&tablet, &work), "Dune");
        assert_converged(&[&laptop, &tablet]);
    }

    #[test]
    fn a_reader_asks_and_an_administrator_accepts_on_another_device() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();

        ops::as_request(None, || {
            catalog::create_book(&mut phone.0, &lib, &book("Wingspan", None, "Hugo"), None)
        })
        .unwrap();
        assert!(
            !phone.titles().contains(&"Wingspan".to_owned()),
            "asked, not added"
        );
        laptop.pull_from(&phone);
        let pending = crate::requests::list_requests(&laptop.0, &lib).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, crate::requests::RequestStatus::Pending);
        assert_eq!(pending[0].person.as_deref(), Some("Hugo"));

        crate::requests::accept(&mut laptop.0, &lib, &pending[0].id).unwrap();
        phone.sync_with(&mut laptop);

        assert!(laptop.titles().contains(&"Wingspan".to_owned()));
        assert!(phone.titles().contains(&"Wingspan".to_owned()));
        let seen = crate::requests::list_requests(&phone.0, &lib).unwrap();
        assert_eq!(seen[0].status, crate::requests::RequestStatus::Accepted);
        assert!(matches!(
            crate::requests::reject(&mut laptop.0, &lib, &pending[0].id),
            Err(crate::error::Error::RequestResolved)
        ));
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_rejected_request_changes_nothing() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        ops::as_request(None, || {
            catalog::create_book(&mut phone.0, &lib, &book("Wingspan", None, "Hugo"), None)
        })
        .unwrap();
        laptop.pull_from(&phone);
        let id = crate::requests::list_requests(&laptop.0, &lib).unwrap()[0]
            .id
            .clone();

        crate::requests::reject(&mut laptop.0, &lib, &id).unwrap();
        phone.sync_with(&mut laptop);

        assert!(!laptop.titles().contains(&"Wingspan".to_owned()));
        let seen = crate::requests::list_requests(&phone.0, &lib).unwrap();
        assert_eq!(seen[0].status, crate::requests::RequestStatus::Rejected);
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_request_carries_its_note_and_says_who_resolved_it() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        ops::as_request(Some("  Me lo regaló la abuela "), || {
            catalog::create_book(&mut phone.0, &lib, &book("Wingspan", None, "Hugo"), None)
        })
        .unwrap();
        laptop.pull_from(&phone);
        let pending = crate::requests::list_requests(&laptop.0, &lib).unwrap();
        assert_eq!(pending[0].note.as_deref(), Some("Me lo regaló la abuela"));
        assert!(
            pending[0]
                .changes
                .iter()
                .all(|change| change.entity != Entity::ChangeRequest),
            "the note row is not part of what was asked"
        );
        assert!(!pending[0].mine);

        crate::requests::accept(&mut laptop.0, &lib, &pending[0].id).unwrap();
        phone.sync_with(&mut laptop);

        let seen = crate::requests::list_requests(&phone.0, &lib).unwrap();
        assert!(seen[0].mine);
        assert_eq!(seen[0].status, crate::requests::RequestStatus::Accepted);
        assert_eq!(seen[0].resolved_by.as_deref(), Some("Tersoal"));
        assert!(seen[0].resolved_at.is_some());
        assert!(phone.titles().contains(&"Wingspan".to_owned()));
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn an_administrator_corrects_a_book_request_before_accepting_it() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        ops::as_request(None, || {
            catalog::create_book(&mut phone.0, &lib, &book("Wingspam", None, "Hugo"), None)
        })
        .unwrap();
        laptop.pull_from(&phone);
        let request = &crate::requests::list_requests(&laptop.0, &lib).unwrap()[0];
        assert_eq!(request.summary.action, "add_book");
        assert_eq!(request.summary.title.as_deref(), Some("Wingspam"));
        assert!(request.applies && request.editable);
        let draft = crate::requests::requested_book(&laptop.0, &lib, &request.id).unwrap();
        assert_eq!(draft.copy.owner.as_deref(), Some("Hugo"));

        catalog::create_book_accepting(
            &mut laptop.0,
            &lib,
            &book("Wingspan", None, "Hugo"),
            None,
            Some(&request.id),
        )
        .unwrap();
        phone.sync_with(&mut laptop);

        assert!(phone.titles().contains(&"Wingspan".to_owned()));
        assert!(!phone.titles().contains(&"Wingspam".to_owned()));
        let seen = crate::requests::list_requests(&phone.0, &lib).unwrap();
        assert_eq!(seen[0].status, crate::requests::RequestStatus::Accepted);
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn one_person_marking_on_two_devices_offline_ends_with_one_merged_mark() {
        use crate::marks::{self, Mark};
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        laptop.add("Dune", None, "Tersoal");
        let hugo = crate::people::PersonPick {
            id: None,
            name: "Hugo".into(),
        };
        marks::claim_person(&mut laptop.0, &lib, &hugo).unwrap();
        laptop.sync_with(&mut phone);
        let member = marks::my_member(&laptop.0, &lib).unwrap();
        let same = crate::people::PersonPick {
            id: member,
            name: "Hugo".into(),
        };
        marks::claim_person(&mut phone.0, &lib, &same).unwrap();
        let work = |device: &Device| {
            let edition = catalog::list_books(&device.0, &lib, &BookFilter::default()).unwrap()[0]
                .edition_id
                .clone();
            catalog::get_book(&device.0, &lib, &edition)
                .unwrap()
                .work_id
        };
        let dune = work(&laptop);

        let finished = Mark {
            status: Some("finished".into()),
            status_on: Some("2026-03-14".into()),
            ..Mark::default()
        };
        marks::set_mark(&mut laptop.0, &lib, &dune, &finished).unwrap();
        let loved = Mark {
            rating: Some(5),
            favorite: true,
            wished: true,
            ..Mark::default()
        };
        marks::set_mark(&mut phone.0, &lib, &dune, &loved).unwrap();
        laptop.sync_with(&mut phone);
        laptop.sync_with(&mut phone);

        assert_converged(&[&laptop, &phone]);
        let seen = marks::work_marks(&phone.0, &lib, &dune).unwrap();
        assert!(seen.others.is_empty(), "one row, not two");
        let mine = seen.mine.unwrap();
        assert_eq!(mine.status.as_deref(), Some("finished"));
        assert!(mine.favorite, "both devices' changes are there");
        assert!(mine.wished, "wished travels like favorite");
        assert!(
            marks::work_marks(&laptop.0, &lib, &dune)
                .unwrap()
                .mine
                .unwrap()
                .wished,
            "the other device sees it too"
        );
        let listed = |statuses: &[&str], favorites: bool| {
            let filter = BookFilter {
                my_statuses: statuses.iter().map(|s| s.to_string()).collect(),
                my_favorites: favorites,
                ..BookFilter::default()
            };
            catalog::list_books(&phone.0, &lib, &filter).unwrap()
        };
        assert_eq!(listed(&[], false)[0].my_status.as_deref(), Some("finished"));
        assert_eq!(listed(&["finished", "pending"], false).len(), 1);
        assert!(listed(&["none"], false).is_empty(), "Dune is marked");
        assert_eq!(listed(&[], true).len(), 1);

        // Emptied and marked again: an emptied mark is never a tombstone.
        marks::set_mark(&mut phone.0, &lib, &dune, &Mark::default()).unwrap();
        marks::set_mark(&mut phone.0, &lib, &dune, &finished).unwrap();
        laptop.sync_with(&mut phone);
        let again = marks::work_marks(&laptop.0, &lib, &dune)
            .unwrap()
            .mine
            .unwrap();
        assert_eq!(again.status.as_deref(), Some("finished"));
        assert!(!again.favorite);

        let edition = catalog::list_books(&laptop.0, &lib, &BookFilter::default()).unwrap()[0]
            .edition_id
            .clone();
        catalog::delete_book(&mut laptop.0, &lib, &edition).unwrap();
        laptop.sync_with(&mut phone);
        assert!(
            marks::work_marks(&phone.0, &lib, &dune)
                .unwrap()
                .mine
                .is_none()
        );
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn the_book_says_what_each_device_may_delete_and_asking_says_what_happened() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let dune = |device: &Device| {
            let edition = catalog::list_books(&device.0, &lib, &BookFilter::default())
                .unwrap()
                .into_iter()
                .find(|b| b.title == "Dune")
                .unwrap()
                .edition_id;
            catalog::get_book(&device.0, &lib, &edition).unwrap()
        };

        let on_phone = dune(&phone);
        assert!(!on_phone.can_delete);
        assert!(!on_phone.copies[0].can_delete);
        let on_laptop = dune(&laptop);
        assert!(on_laptop.can_delete);
        assert!(on_laptop.copies[0].can_delete);

        let (_, requested) = ops::as_request(None, || {
            catalog::delete_copy(&mut phone.0, &lib, &on_phone.copies[0].id)
        })
        .unwrap();
        assert!(requested, "a reader's delete becomes a request");
        assert!(phone.titles().contains(&"Dune".to_owned()));
        let (_, requested) = ops::as_request(None, || {
            catalog::create_book(&mut laptop.0, &lib, &book("Solaris", None, "Tersoal"), None)
        })
        .unwrap();
        assert!(!requested, "what the administrator may do is just done");
        laptop.sync_with(&mut phone);
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_request_on_a_deleted_book_no_longer_applies() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let dune = |device: &Device| {
            catalog::list_books(&device.0, &lib, &BookFilter::default())
                .unwrap()
                .into_iter()
                .find(|b| b.title == "Dune")
                .unwrap()
                .edition_id
        };
        let edition = dune(&phone);
        ops::as_request(None, || catalog::delete_book(&mut phone.0, &lib, &edition)).unwrap();
        laptop.pull_from(&phone);
        let request = crate::requests::list_requests(&laptop.0, &lib).unwrap()[0].clone();
        assert_eq!(request.summary.action, "delete_book");
        assert_eq!(request.summary.title.as_deref(), Some("Dune"));
        assert!(request.applies);

        let on_laptop = dune(&laptop);
        catalog::delete_book(&mut laptop.0, &lib, &on_laptop).unwrap();

        let request = &crate::requests::list_requests(&laptop.0, &lib).unwrap()[0];
        assert!(!request.applies);
    }

    #[test]
    fn whoever_asked_withdraws_and_it_can_no_longer_be_accepted() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        ops::as_request(None, || {
            catalog::create_book(&mut phone.0, &lib, &book("Wingspan", None, "Hugo"), None)
        })
        .unwrap();
        let id = crate::requests::list_requests(&phone.0, &lib).unwrap()[0]
            .id
            .clone();

        crate::requests::withdraw(&mut phone.0, &lib, &id).unwrap();
        laptop.pull_from(&phone);

        let seen = crate::requests::list_requests(&laptop.0, &lib).unwrap();
        assert_eq!(seen[0].status, crate::requests::RequestStatus::Withdrawn);
        assert!(matches!(
            crate::requests::accept(&mut laptop.0, &lib, &id),
            Err(crate::error::Error::RequestResolved)
        ));
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn nobody_withdraws_someone_else_s_request() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        laptop.add("Solaris", None, "Tersoal");
        phone.pull_from(&laptop);
        let tersoal_s = ops::as_request(None, || {
            let edition = catalog::list_books(&phone.0, &lib, &BookFilter::default())
                .unwrap()
                .into_iter()
                .find(|b| b.title == "Solaris")
                .unwrap()
                .edition_id;
            catalog::delete_book(&mut phone.0, &lib, &edition)
        });
        assert!(tersoal_s.is_ok(), "the phone asks to delete it");
        let mut tablet = Device::sharing(&laptop);
        tablet.pull_from(&laptop);
        tablet.pull_from(&phone);
        let id = crate::requests::list_requests(&tablet.0, &lib).unwrap()[0]
            .id
            .clone();

        let result = crate::requests::withdraw(&mut tablet.0, &lib, &id);

        assert!(
            matches!(result, Err(crate::error::Error::NotAllowed)),
            "{result:?}"
        );
    }

    #[test]
    fn my_offline_change_that_becomes_a_request_is_noticed_once() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let hugo = phone_person(&laptop, &phone);
        // Hugo becomes an editor, and the phone learns it.
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("editor"), false).unwrap();
        phone.pull_from(&laptop);
        // Offline, the laptop takes it back first; the phone keeps editing.
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("reader"), false).unwrap();
        phone.add("Wingspan", None, "Hugo");
        assert!(phone.titles().contains(&"Wingspan".to_owned()));

        phone.pull_from(&laptop);

        assert!(!phone.titles().contains(&"Wingspan".to_owned()));
        let notices = crate::requests::notices(&phone.0, &lib).unwrap();
        assert_eq!(notices.len(), 1);
        crate::requests::dismiss_notices(&phone.0, &lib).unwrap();
        assert!(crate::requests::notices(&phone.0, &lib).unwrap().is_empty());
    }

    fn phone_person(laptop: &Device, phone: &Device) -> String {
        laptop
            .0
            .query_row(
                "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
                [&laptop.1, &device_id_of(phone)],
                |row| row.get(0),
            )
            .unwrap()
    }

    const PERMISSION_CASES: &str = include_str!("../../../spec/conformance/permissions.json");

    #[derive(serde::Deserialize)]
    struct PermissionCase {
        name: String,
        operations: Vec<Map<String, Value>>,
        deliveries: Vec<Vec<usize>>,
        expect: PermissionExpect,
    }

    #[derive(serde::Deserialize)]
    struct PermissionExpect {
        applied: Vec<String>,
        requests: Vec<String>,
        #[serde(default)]
        rejected: Vec<String>,
    }

    #[test]
    fn permission_conformance_cases() {
        let cases: Vec<PermissionCase> = serde_json::from_str(PERMISSION_CASES).unwrap();
        assert!(!cases.is_empty());
        // The test key every device in the cases registers (spec/conformance/signatures.json).
        let key = kosmotheka_domain::DeviceKey::from_seed([1; 32]);

        for case in &cases {
            for order in &case.deliveries {
                let mut device = Device::blank();
                let schema_version = ops::schema_version(&device.0).unwrap();
                for &index in order {
                    let mut fields = case.operations[index].clone();
                    let unsigned = fields.remove("unsigned").is_some();
                    let mut operation: Operation =
                        serde_json::from_value(Value::Object(fields)).unwrap();
                    if !unsigned {
                        operation.signature = Some(key.sign(&operation));
                    }
                    let id = operation.id.clone();
                    let report = merge(
                        &mut device.0,
                        vec![SyncOperation {
                            schema_version,
                            operation,
                        }],
                    )
                    .unwrap();
                    let refused = case.expect.rejected.contains(&id);
                    assert_eq!(
                        report.rejected,
                        usize::from(refused),
                        "case '{}': op {index} rejected or not",
                        case.name
                    );
                }

                let status_of = |status: &str| -> BTreeSet<String> {
                    device
                        .0
                        .prepare("SELECT id FROM operations WHERE status = ?1")
                        .unwrap()
                        .query_map([status], |row| row.get(0))
                        .unwrap()
                        .collect::<rusqlite::Result<_>>()
                        .unwrap()
                };
                let expected = |ids: &[String]| ids.iter().cloned().collect::<BTreeSet<_>>();
                assert_eq!(
                    status_of("applied"),
                    expected(&case.expect.applied),
                    "case '{}', delivery {order:?}: applied",
                    case.name
                );
                assert_eq!(
                    status_of("request"),
                    expected(&case.expect.requests),
                    "case '{}', delivery {order:?}: requests",
                    case.name
                );
                assert_rebuild_changes_nothing(&device);
            }
        }
    }

    #[test]
    fn merge_conformance_cases() {
        let cases: Vec<MergeCase> = MERGE_CASES
            .iter()
            .flat_map(|file| serde_json::from_str::<Vec<MergeCase>>(file).unwrap())
            .collect();
        assert!(!cases.is_empty());

        // Every operation is signed (D56): each author registers the test key of
        // spec/conformance/signatures.json first, and the runner signs the case's
        // operations with it, as for the permission cases.
        let key = kosmotheka_domain::DeviceKey::from_seed([1; 32]);
        for case in cases {
            let authors: BTreeSet<(String, String)> = case
                .operations
                .iter()
                .map(|op| (op.library.clone(), op.hlc.device.clone()))
                .collect();
            for order in &case.deliveries {
                let mut device = Device::blank();
                let schema_version = ops::schema_version(&device.0).unwrap();
                for (counter, (library, author)) in (0u32..).zip(&authors) {
                    let mut register = Operation {
                        id: ops::new_id(),
                        library: library.clone(),
                        // HLCs are unique in the log, also for one author in two libraries.
                        hlc: kosmotheka_domain::Hlc {
                            millis: 0,
                            counter,
                            device: author.clone(),
                        },
                        intent: "device.register".into(),
                        changes: vec![Change::create(
                            Entity::Device,
                            author,
                            json!({ "public_key": key.public_key(), "name": "test" }),
                        )],
                        previous: None,
                        signature: None,
                    };
                    register.signature = Some(key.sign(&register));
                    merge(
                        &mut device.0,
                        vec![SyncOperation {
                            schema_version,
                            operation: register,
                        }],
                    )
                    .unwrap();
                }
                // Only rows the case creates count: the registrations above are the
                // runner's (skipped below).
                let before = state(&device.0);
                for &index in order {
                    let mut operation = case.operations[index].clone();
                    operation.signature = Some(key.sign(&operation));
                    let refused = case
                        .expect
                        .rejected
                        .as_ref()
                        .map(|ids| usize::from(ids.contains(&operation.id)));
                    let report = merge(
                        &mut device.0,
                        vec![SyncOperation {
                            schema_version,
                            operation,
                        }],
                    )
                    .unwrap();
                    if let Some(refused) = refused {
                        assert_eq!(
                            report.rejected, refused,
                            "case '{}', delivery {order:?}: op {index} rejected or not",
                            case.name
                        );
                    }
                }

                let mut rows = Vec::new();
                for (library, table, entity_rows) in state(&device.0) {
                    let entity = ORDER.iter().find(|e| e.table() == table).unwrap();
                    for mut row in entity_rows {
                        let id = row.remove("id").unwrap().as_str().unwrap().to_owned();
                        let untouched = before.iter().any(|(l, t, r)| {
                            *l == library && *t == table && r.iter().any(|old| old["id"] == id)
                        });
                        let registration = entity == &Entity::Device
                            && authors.contains(&(library.clone(), id.clone()));
                        if untouched || registration {
                            continue;
                        }
                        rows.push(ExpectedRow {
                            library: library.clone(),
                            entity: serde_json::to_value(entity)
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_owned(),
                            id,
                            // An optional field nobody set is null: cases list only the rest.
                            fields: row.into_iter().filter(|(_, v)| !v.is_null()).collect(),
                        });
                    }
                }
                let key =
                    |row: &&ExpectedRow| (row.library.clone(), row.entity.clone(), row.id.clone());
                let expected_rows: Vec<ExpectedRow> = case
                    .expect
                    .rows
                    .iter()
                    .map(|row| ExpectedRow {
                        fields: row
                            .fields
                            .iter()
                            .filter(|(_, v)| !v.is_null())
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                        library: row.library.clone(),
                        entity: row.entity.clone(),
                        id: row.id.clone(),
                    })
                    .collect();
                let mut expected = expected_rows.iter().collect::<Vec<_>>();
                expected.sort_by_key(key);
                rows.sort_by_key(|row| key(&row));
                assert_eq!(
                    rows.iter().collect::<Vec<_>>(),
                    expected,
                    "case '{}', delivery {order:?}",
                    case.name
                );
            }
        }
    }

    fn record_changes(device: &mut Device, intent: &str, changes: Vec<Change>) {
        let tx = device.0.transaction().unwrap();
        ops::record(&tx, &device.1, intent, changes).unwrap();
        tx.commit().unwrap();
    }

    /// D89: a delete wins over what another device adds meanwhile. Three devices:
    /// the laptop deletes a work while the phone adds an edition and a copy to it,
    /// and the tablet deletes a publisher the phone names. The devices meet in the
    /// given order.
    fn delete_wins_scenario(meeting: &[(usize, usize)]) {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let edition_id = laptop.add("Dune", None, "Ana");
        let work_id = catalog::get_book(&laptop.0, &lib, &edition_id)
            .unwrap()
            .work_id;
        let publisher = ops::new_id();
        record_changes(
            &mut laptop,
            "publisher.add",
            vec![Change::create(
                Entity::Publisher,
                &publisher,
                json!({ "name": "Minotauro" }),
            )],
        );
        let mut phone = Device::sharing(&laptop);
        let mut tablet = Device::sharing(&laptop);

        record_changes(
            &mut laptop,
            "work.remove",
            vec![Change::delete(Entity::Work, &work_id)],
        );
        let (new_edition, new_copy) = (ops::new_id(), ops::new_id());
        record_changes(
            &mut phone,
            "edition.add",
            vec![
                Change::create(
                    Entity::Edition,
                    &new_edition,
                    json!({ "work_id": work_id, "title": "Dune 2", "publisher_id": publisher }),
                ),
                Change::create(
                    Entity::Copy,
                    &new_copy,
                    json!({ "edition_id": new_edition, "format": "ebook", "quantity": 1 }),
                ),
            ],
        );
        // An update of a row that the delete removes in cascade: no tombstone of its
        // own, and the devices still agree.
        record_changes(
            &mut phone,
            "edition.edit",
            vec![Change::update(
                Entity::Edition,
                &new_edition,
                json!({ "subtitle": "Revised" }),
            )],
        );
        record_changes(
            &mut tablet,
            "publisher.remove",
            vec![Change::delete(Entity::Publisher, &publisher)],
        );

        for &(from, to) in meeting {
            let mut devices = [&mut laptop, &mut phone, &mut tablet];
            let (low, high) = (from.min(to), from.max(to));
            let (head, tail) = devices.split_at_mut(high);
            head[low].sync_with(tail[0]);
        }
        // One more round so every device has seen everything.
        laptop.sync_with(&mut phone);
        phone.sync_with(&mut tablet);
        laptop.sync_with(&mut tablet);
        assert_converged(&[&laptop, &phone, &tablet]);

        for table in ["works", "editions", "copies", "publishers"] {
            let rows: i64 = laptop
                .0
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(rows, 0, "{table} should be empty: the deletes won");
        }
    }

    #[test]
    fn a_delete_wins_whatever_the_order_the_devices_meet_in() {
        delete_wins_scenario(&[(0, 1), (0, 2), (1, 2)]);
        delete_wins_scenario(&[(1, 2), (0, 2), (0, 1)]);
        delete_wins_scenario(&[(0, 2), (1, 2), (0, 1)]);
    }

    fn add_genre(conn: &mut Connection, library: &str, work_id: &str, genre: &str) {
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            library,
            "work.genre_add",
            vec![Change::create(
                Entity::WorkGenre,
                ops::new_id(),
                json!({ "work_id": work_id, "genre": genre }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    fn remove_genre(conn: &mut Connection, library: &str, work_id: &str, genre: &str) {
        let tx = conn.transaction().unwrap();
        let rows: Vec<String> = tx
            .prepare(
                "SELECT id FROM work_genres WHERE library_id = ?1 AND work_id = ?2 AND genre = ?3",
            )
            .unwrap()
            .query_map(params![library, work_id, genre], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let changes = rows
            .into_iter()
            .map(|id| Change::delete(Entity::WorkGenre, id))
            .collect();
        ops::record(&tx, library, "work.genre_remove", changes).unwrap();
        tx.commit().unwrap();
    }

    fn genres_of_work(conn: &Connection, library: &str, work_id: &str) -> Vec<String> {
        conn.prepare(
            "SELECT DISTINCT genre FROM work_genres WHERE library_id = ?1 AND work_id = ?2",
        )
        .unwrap()
        .query_map(params![library, work_id], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
    }

    /// Two devices add the same genre to the same work offline: no unique constraint
    /// means both rows arrive, but readers deduplicate, so it converges as one genre;
    /// removing it then removes every row of the pair, on both devices.
    #[test]
    fn duplicate_genre_added_offline_converges() {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        let edition_id = laptop.add("Dune", None, "Ana");
        let work_id = catalog::get_book(&laptop.0, &lib, &edition_id)
            .unwrap()
            .work_id;
        phone.pull_from(&laptop);

        add_genre(&mut laptop.0, &lib, &work_id, "science_fiction");
        add_genre(&mut phone.0, &lib, &work_id, "science_fiction");
        laptop.sync_with(&mut phone);

        assert_eq!(
            genres_of_work(&laptop.0, &lib, &work_id),
            vec!["science_fiction".to_owned()]
        );
        assert_eq!(
            genres_of_work(&phone.0, &lib, &work_id),
            vec!["science_fiction".to_owned()]
        );
        assert_converged(&[&laptop, &phone]);

        remove_genre(&mut laptop.0, &lib, &work_id, "science_fiction");
        laptop.sync_with(&mut phone);

        assert!(genres_of_work(&laptop.0, &lib, &work_id).is_empty());
        assert!(genres_of_work(&phone.0, &lib, &work_id).is_empty());
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn the_same_work_added_to_a_series_offline_on_two_devices_converges() {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        let edition_id = laptop.add("Dune", None, "Ana");
        let work_id = catalog::get_book(&laptop.0, &lib, &edition_id)
            .unwrap()
            .work_id;
        let series_id = series::create_series(&mut laptop.0, &lib, "Dune", true).unwrap();
        phone.pull_from(&laptop);

        series::add_to_series(&mut laptop.0, &lib, &series_id, &work_id, Some(1)).unwrap();
        series::add_to_series(&mut phone.0, &lib, &series_id, &work_id, Some(1)).unwrap();
        laptop.sync_with(&mut phone);

        let laptop_detail = series::get_series(&laptop.0, &lib, &series_id).unwrap();
        let phone_detail = series::get_series(&phone.0, &lib, &series_id).unwrap();
        assert_eq!(laptop_detail.entries.len(), 1);
        assert_eq!(phone_detail.entries.len(), 1);
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn deleting_a_series_while_another_device_adds_to_it_offline_converges() {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        let edition_id = laptop.add("Dune", None, "Ana");
        let work_id = catalog::get_book(&laptop.0, &lib, &edition_id)
            .unwrap()
            .work_id;
        let series_id = series::create_series(&mut laptop.0, &lib, "Dune", true).unwrap();
        phone.pull_from(&laptop);

        series::delete_series(&mut laptop.0, &lib, &series_id).unwrap();
        series::add_to_series(&mut phone.0, &lib, &series_id, &work_id, Some(1)).unwrap();
        laptop.sync_with(&mut phone);

        assert!(series::get_series(&laptop.0, &lib, &series_id).is_err());
        assert!(series::get_series(&phone.0, &lib, &series_id).is_err());
        assert_converged(&[&laptop, &phone]);
    }

    // Tests that pin merge rules found by mutation testing (docs/audits/mutaciones-2026-10-04.md).

    #[test]
    fn a_merge_report_adds_up_field_by_field() {
        let mut total = MergeReport {
            received: 1,
            known: 2,
            rejected: 3,
            skipped_changes: 4,
            requests: 5,
        };
        total.add(&MergeReport {
            received: 10,
            known: 20,
            rejected: 30,
            skipped_changes: 40,
            requests: 50,
        });
        assert_eq!(
            total,
            MergeReport {
                received: 11,
                known: 22,
                rejected: 33,
                skipped_changes: 44,
                requests: 55,
            }
        );
    }

    /// An operation signed by `device`, dated now, with any changes and no previous.
    fn forged(device: &Device, changes: Vec<Change>) -> SyncOperation {
        let mut operation = Operation {
            id: ops::new_id(),
            library: device.1.clone(),
            hlc: Hlc {
                millis: now_millis(),
                counter: 0,
                device: device_id_of(device),
            },
            intent: "test.forged".into(),
            changes,
            previous: None,
            signature: None,
        };
        operation.signature = Some(ops::signing_key(&device.0).unwrap().sign(&operation));
        SyncOperation {
            schema_version: ops::schema_version(&device.0).unwrap(),
            operation,
        }
    }

    #[test]
    fn the_report_counts_known_operations_and_skipped_changes() {
        let a = Device::new();
        let mut b = Device::sharing(&a);
        // An update of a row that does not exist is skipped, not an error.
        let missing = forged(
            &a,
            vec![Change::update(
                Entity::Work,
                "01997c7e-0000-7000-8000-0000000000aa",
                json!({ "title": "Nowhere" }),
            )],
        );

        let first = merge(&mut b.0, vec![missing.clone()]).unwrap();
        let again = merge(&mut b.0, vec![missing]).unwrap();

        assert_eq!((first.received, first.skipped_changes), (1, 1));
        assert_eq!((again.received, again.known), (0, 1));
    }

    #[test]
    fn a_library_row_alone_is_not_worth_a_baseline() {
        let mut device = Device::new();
        // What a new library has is its own row and its device's registration.
        device.0.execute("DELETE FROM operations", []).unwrap();
        device.0.execute("DELETE FROM devices", []).unwrap();
        device
            .0
            .execute("DELETE FROM local_state WHERE key = ?1", [BASELINE_KEY])
            .unwrap();

        ensure_baseline(&mut device.0).unwrap();

        let baselines: i64 = device
            .0
            .query_row(
                "SELECT count(*) FROM operations WHERE intent = 'sync.baseline'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(baselines, 0);
    }

    const UNTRACKED_TAG: &str = "01997c7e-0000-7000-8000-0000000000e1";

    /// A row no operation made: a rebuild from the log would not bring it back, so
    /// finding it after a merge proves the library was not rebuilt.
    fn plant_untracked_row(device: &Device) {
        device
            .0
            .execute(
                "INSERT INTO tags (library_id, id, name) VALUES (?1, ?2, 'Untracked')",
                [&device.1, UNTRACKED_TAG],
            )
            .unwrap();
    }

    fn untracked_row_survives(device: &Device) -> bool {
        device
            .0
            .query_row(
                "SELECT count(*) FROM tags WHERE library_id = ?1 AND id = ?2",
                [&device.1, UNTRACKED_TAG],
                |r| r.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

    #[test]
    fn an_operation_that_arrives_in_order_does_not_rebuild_the_library() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        plant_untracked_row(&phone);
        laptop.add("In order", None, "Tersoal");

        let report = phone.pull_from(&laptop);

        assert!(report.received >= 1);
        assert!(phone.titles().contains(&"In order".to_owned()));
        assert!(untracked_row_survives(&phone), "rebuilt without need");
    }

    #[test]
    fn a_late_operation_in_a_library_without_roles_does_not_rebuild_it() {
        let mut a = Device::new();
        let mut b = Device::sharing(&a);
        b.add("Older, on b", None, "Bea");
        std::thread::sleep(std::time::Duration::from_millis(5));
        a.add("Newer, on a", None, "Ana");
        plant_untracked_row(&a);

        let report = a.pull_from(&b);

        assert!(report.received >= 1, "b's older operation arrived");
        assert!(!permissions::judged_by_order(&a.0, &a.1).unwrap());
        assert!(untracked_row_survives(&a), "order does not matter here");
    }

    #[test]
    fn a_fence_set_by_a_request_is_not_noted() {
        // The phone is a reader: asking to fence the laptop is a request, and a
        // request must not take effect, fence included (D84).
        let (mut laptop, phone) = laptop_admin_phone_reader();
        let mark = ops::latest_of(&laptop.0, &laptop.1, &device_id_of(&laptop))
            .unwrap()
            .unwrap();
        let fence = forged(
            &phone,
            vec![Change::update(
                Entity::Device,
                device_id_of(&laptop),
                json!({ "trusted_through": mark }),
            )],
        );

        let report = merge(&mut laptop.0, vec![fence]).unwrap();

        assert_eq!((report.rejected, report.requests), (0, 1));
        let fences: i64 = laptop
            .0
            .query_row("SELECT count(*) FROM device_fences", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fences, 0);
    }

    /// An operation signed by `device` with a modified app: any intent, HLC and previous.
    fn forged_at(
        device: &Device,
        intent: &str,
        changes: Vec<Change>,
        millis: u64,
        previous: Option<&str>,
    ) -> SyncOperation {
        let mut operation = Operation {
            id: ops::new_id(),
            library: device.1.clone(),
            hlc: Hlc {
                millis,
                counter: 0,
                device: device_id_of(device),
            },
            intent: intent.into(),
            changes,
            previous: previous.map(str::to_owned),
            signature: None,
        };
        operation.signature = Some(ops::signing_key(&device.0).unwrap().sign(&operation));
        SyncOperation {
            schema_version: ops::schema_version(&device.0).unwrap(),
            operation,
        }
    }

    fn operations_with_intent(device: &Device, intent: &str) -> Vec<String> {
        device
            .0
            .prepare("SELECT id FROM operations WHERE library_id = ?1 AND intent = ?2 ORDER BY hlc")
            .unwrap()
            .query_map([&device.1, intent], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    fn seals_in(device: &Device) -> i64 {
        device
            .0
            .query_row(
                "SELECT count(*) FROM device_seals WHERE library_id = ?1",
                [&device.1],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// The laptop and the phone are administrators and nobody ever sealed the library:
    /// a library that got its roles before D94.
    fn two_administrators_without_a_seal() -> (Device, Device) {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let hugo = phone_person(&laptop, &phone);
        crate::people::set_role(&mut laptop.0, &lib, &hugo, Some("admin"), true).unwrap();
        phone.pull_from(&laptop);
        assert_eq!(seals_in(&laptop), 0, "set_role seals nothing");
        (laptop, phone)
    }

    #[test]
    fn the_cloud_s_power_reaching_a_device_after_what_it_did_changes_nothing_in_the_end() {
        let (mut laptop, _phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        // The cloud: a device whose id is the one its key gives (D95).
        let mut phone = Device::sharing(&laptop);
        permissions::become_a_cloud_device(&phone.0, 7);
        let tx = phone.0.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        laptop.pull_from(&phone);
        let phone_id = device_id_of(&phone);
        let mut tablet = Device::sharing(&laptop);
        let tx = tablet.0.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        laptop.pull_from(&tablet);
        let tablet_id = device_id_of(&tablet);
        let laura = ops::new_id();
        let tx = laptop.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                &laura,
                json!({ "name": "Laura", "role": "editor", "can_share": false }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        crate::people::name_cloud_device(&mut laptop.0, &lib, &phone_id).unwrap();
        phone.pull_from(&laptop);

        // The phone is the cloud: it writes Laura's account and links the tablet to her.
        let tx = phone.0.transaction().unwrap();
        let account = ops::record(
            &tx,
            &lib,
            "member.account",
            vec![Change::update(
                Entity::Member,
                &laura,
                json!({ "account_id": ops::new_id() }),
            )],
        )
        .unwrap();
        let link = ops::record(
            &tx,
            &lib,
            "device.link",
            vec![Change::update(
                Entity::Device,
                &tablet_id,
                json!({ "member_id": laura }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();

        // A third device hears of all of it before it hears that the phone is the cloud.
        let naming = operations_with_intent(&laptop, "library.cloud");
        let (late, early): (Vec<_>, Vec<_>) = everything_from(&phone)
            .into_iter()
            .partition(|op| naming.contains(&op.operation.id));
        let mut observer = Device::empty_for(&laptop);
        merge(&mut observer.0, early).unwrap();
        for id in [&account.id, &link.id] {
            assert_eq!(
                ops::status_of(&observer.0, id).unwrap(),
                ops::Status::Request,
                "until the phone is known to be the cloud"
            );
        }
        merge(&mut observer.0, late).unwrap();

        for id in [&account.id, &link.id] {
            assert_eq!(
                ops::status_of(&observer.0, id).unwrap(),
                ops::Status::Applied
            );
        }
        laptop.pull_from(&phone);
        tablet.pull_from(&laptop);
        assert_converged(&[&laptop, &phone, &tablet, &observer]);
        assert_eq!(
            ops::status_of(&laptop.0, &link.id).unwrap(),
            ops::Status::Applied
        );
    }

    /// D95: the id of the cloud is tied to its key. A modified app registers the cloud's id with a key of its
    /// own before the real registration, over the LAN, and signs with it: where that registration arrived first,
    /// the key registered for the id is not the one the id derives from, so it has no power.
    #[test]
    fn the_clouds_id_registered_with_another_key_gives_no_cloud_power() {
        let (mut laptop, _phone) = laptop_admin_phone_reader();
        let lib = laptop.1.clone();
        let cloud_id = kosmotheka_domain::cloud_device_id(
            &kosmotheka_domain::DeviceKey::from_seed([7; 32]).public_key(),
        )
        .unwrap();
        let laura = ops::new_id();
        let tx = laptop.0.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                &laura,
                json!({ "name": "Laura", "role": "editor", "can_share": false }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        crate::people::name_cloud_device(&mut laptop.0, &lib, &cloud_id).unwrap();

        // The impostor holds the cloud's id and a key of its own; it skips every local check.
        let impostor = Device::sharing(&laptop);
        ops::set_identity(&impostor.0, &cloud_id, [9; 32]).unwrap();
        let key = ops::signing_key(&impostor.0).unwrap().public_key();
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
            + 60_000;
        let registration = forged_at(
            &impostor,
            "device.register",
            vec![Change::create(
                Entity::Device,
                &cloud_id,
                json!({ "public_key": key, "name": "The cloud, he says" }),
            )],
            now,
            None,
        );
        let account = forged_at(
            &impostor,
            "member.account",
            vec![Change::update(
                Entity::Member,
                &laura,
                json!({ "account_id": ops::new_id() }),
            )],
            now + 1,
            Some(&registration.operation.id),
        );
        let account_id = account.operation.id.clone();

        merge(&mut laptop.0, vec![registration, account]).unwrap();

        assert_eq!(
            ops::status_of(&laptop.0, &account_id).unwrap(),
            ops::Status::Request
        );
    }

    #[test]
    fn turning_roles_on_seals_the_library_naming_every_device_it_knows() {
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        let mut phone = Device::sharing(&laptop);
        laptop.add("Dune", None, "Tersoal");
        phone.add("Tehanu", None, "Hugo");
        laptop.pull_from(&phone);
        let (laptop_id, phone_id) = (device_id_of(&laptop), device_id_of(&phone));
        let laptop_last = ops::latest_of(&laptop.0, &lib, &laptop_id).unwrap();
        let phone_last = ops::latest_of(&laptop.0, &lib, &phone_id).unwrap();

        crate::people::start_roles(&mut laptop.0, &lib, "Tersoal", &[]).unwrap();

        let roles = operations_with_intent(&laptop, "member.roles");
        assert_eq!(roles.len(), 1);
        assert_eq!(seals_in(&laptop), 1);
        let marks: Vec<(String, String)> = laptop
            .0
            .prepare(
                "SELECT device_id, through FROM device_fences WHERE operation_id = ?1
                  ORDER BY device_id",
            )
            .unwrap()
            .query_map([&roles[0]], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        let mut expected = vec![
            (laptop_id, laptop_last.unwrap()),
            (phone_id, phone_last.unwrap()),
        ];
        expected.sort();
        assert_eq!(
            marks, expected,
            "every device, the author's with its previous"
        );
        phone.pull_from(&laptop);
        assert_eq!(status_of_id(&phone, &roles[0]), "applied");
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_stranger_cannot_date_a_takeover_before_the_activation() {
        // SEG-025: a device the administrator has not heard of signs, dated before the
        // activation, an operation that makes it an administrator. Without the seal it
        // is believed (nobody has a role yet) and the true activation becomes a request.
        let mut laptop = Device::new();
        let lib = laptop.1.clone();
        laptop.add("Dune", None, "Tersoal");
        let mut stranger = Device::sharing(&laptop);
        stranger.add("Hello", None, "Mallory");
        let mallory = ops::new_id();
        let grab = forged_at(
            &stranger,
            "member.roles",
            vec![
                Change::create(
                    Entity::Member,
                    &mallory,
                    json!({ "name": "Mallory", "role": "admin", "can_share": true }),
                ),
                Change::update(
                    Entity::Device,
                    device_id_of(&stranger),
                    json!({ "member_id": mallory }),
                ),
            ],
            now_millis(),
            ops::latest_of(&stranger.0, &lib, &device_id_of(&stranger))
                .unwrap()
                .as_deref(),
        );
        std::thread::sleep(std::time::Duration::from_millis(5));

        crate::people::start_roles(&mut laptop.0, &lib, "Tersoal", &[]).unwrap();
        let activation = operations_with_intent(&laptop, "member.roles").remove(0);
        let mut batch = everything_from(&stranger);
        batch.push(grab.clone());
        merge(&mut laptop.0, batch).unwrap();

        assert_eq!(status_of_id(&laptop, &grab.operation.id), "request");
        assert_eq!(status_of_id(&laptop, &activation), "applied");
        assert_eq!(
            crate::people::list_people(&laptop.0, &lib).unwrap().my_role,
            Some("admin".to_owned())
        );
        let stranger_role: Option<String> = laptop
            .0
            .query_row(
                "SELECT m.role FROM devices d JOIN members m
                   ON m.library_id = d.library_id AND m.id = d.member_id
                  WHERE d.library_id = ?1 AND d.id = ?2",
                [&lib, &device_id_of(&stranger)],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(stranger_role, None, "the stranger stays a device of nobody");
        assert_converged(&[&laptop]);
    }

    #[test]
    fn a_library_with_roles_and_no_seal_is_sealed_once() {
        let (mut laptop, mut phone) = laptop_admin_phone_reader();

        permissions::seal_if_missing(&mut laptop.0, std::slice::from_ref(&laptop.1)).unwrap();
        permissions::seal_if_missing(&mut laptop.0, std::slice::from_ref(&laptop.1)).unwrap();

        let seals = operations_with_intent(&laptop, "library.seal");
        assert_eq!(
            seals.len(),
            1,
            "a second session finds the seal and does nothing"
        );
        assert_eq!(seals_in(&laptop), 1);
        phone.pull_from(&laptop);
        assert_eq!(status_of_id(&phone, &seals[0]), "applied");
        assert_converged(&[&laptop, &phone]);
    }

    /// Why the seal waits for a sync session (D94): it believes of each device only what
    /// the sealer holds. An administrator's device that has been away and sealed on start
    /// would turn what the others did meanwhile into requests.
    #[test]
    fn a_device_that_was_away_seals_after_merging_and_keeps_what_the_others_did() {
        let (mut laptop, mut phone) = two_administrators_without_a_seal();
        let meanwhile = phone.add("The Farthest Shore", None, "Hugo");
        let added = operations_with_intent(&phone, "book.add").pop().unwrap();

        laptop.pull_from(&phone);
        permissions::seal_if_missing(&mut laptop.0, std::slice::from_ref(&laptop.1)).unwrap();
        laptop.sync_with(&mut phone);

        assert_eq!(operations_with_intent(&laptop, "library.seal").len(), 1);
        for device in [&laptop, &phone] {
            assert_eq!(status_of_id(device, &added), "applied");
        }
        assert!(!meanwhile.is_empty());
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_seal_written_before_merging_leaves_what_the_others_did_as_requests() {
        let (mut laptop, mut phone) = two_administrators_without_a_seal();
        phone.add("The Farthest Shore", None, "Hugo");
        let added = operations_with_intent(&phone, "book.add").pop().unwrap();

        permissions::seal_if_missing(&mut laptop.0, std::slice::from_ref(&laptop.1)).unwrap();
        laptop.sync_with(&mut phone);

        for device in [&laptop, &phone] {
            assert_eq!(status_of_id(device, &added), "request");
        }
        assert_converged(&[&laptop, &phone]);
    }

    #[test]
    fn a_device_that_is_not_an_administrators_does_not_seal() {
        let (laptop, mut phone) = laptop_admin_phone_reader();
        let before = operations_with_intent(&phone, "library.seal");

        permissions::seal_if_missing(&mut phone.0, std::slice::from_ref(&phone.1)).unwrap();

        assert_eq!(operations_with_intent(&phone, "library.seal"), before);
        assert_eq!(seals_in(&phone), 0);
        assert!(operations_with_intent(&laptop, "library.seal").is_empty());
    }

    #[test]
    fn a_library_without_roles_is_not_sealed() {
        let mut device = Device::new();
        device.add("Dune", None, "Tersoal");

        permissions::seal_if_missing(&mut device.0, std::slice::from_ref(&device.1)).unwrap();

        assert!(operations_with_intent(&device, "library.seal").is_empty());
    }

    #[test]
    fn two_administrators_who_seal_at_once_without_having_seen_each_other_converge_on_the_later_seal()
     {
        let (mut laptop, mut phone) = two_administrators_without_a_seal();

        permissions::seal_if_missing(&mut laptop.0, std::slice::from_ref(&laptop.1)).unwrap();
        permissions::seal_if_missing(&mut phone.0, std::slice::from_ref(&phone.1)).unwrap();
        laptop.sync_with(&mut phone);

        // The later seal does not know the earlier one, so it fences it like anything else
        // its administrator had not received: the earlier is a request, the later stands.
        let seals = operations_with_intent(&laptop, "library.seal");
        assert_eq!(seals.len(), 2);
        for device in [&laptop, &phone] {
            assert_eq!(status_of_id(device, &seals[0]), "request");
            assert_eq!(status_of_id(device, &seals[1]), "applied");
            assert_eq!(seals_in(device), 1);
        }
        assert_converged(&[&laptop, &phone]);
        // A later session finds an applied seal and writes none.
        permissions::seal_if_missing(&mut laptop.0, std::slice::from_ref(&laptop.1)).unwrap();
        assert_eq!(operations_with_intent(&laptop, "library.seal").len(), 2);
    }

    #[test]
    fn the_same_hlc_does_not_overwrite_a_field() {
        let device = Device::new();
        let mut conn = device.0;
        let lib = device.1;
        let work = "01997c7e-0000-7000-8000-0000000000b1";
        let at = "0001727222222222-00000001-dev";
        let tx = conn.transaction().unwrap();

        let create = Change::create(
            Entity::Work,
            work,
            json!({ "kind": "book", "title": "First" }),
        );
        let rename = Change::update(Entity::Work, work, json!({ "title": "Second" }));

        assert!(apply_change(&tx, &lib, &create, at).unwrap());
        assert!(
            !apply_change(&tx, &lib, &rename, at).unwrap(),
            "a tie is not newer"
        );
        let title: String = tx
            .query_row("SELECT title FROM works WHERE id = ?1", [work], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(title, "First");
    }

    #[test]
    fn a_change_keeps_the_versions_of_the_fields_it_lost() {
        let device = Device::new();
        let mut conn = device.0;
        let lib = device.1;
        let work = "01997c7e-0000-7000-8000-0000000000b2";
        let (h1, h2, h3) = (
            "0001727222222222-00000001-dev",
            "0001727222222222-00000002-dev",
            "0001727222222222-00000003-dev",
        );
        let tx = conn.transaction().unwrap();
        let change = |action_create: bool, fields: Value| {
            if action_create {
                Change::create(Entity::Work, work, fields)
            } else {
                Change::update(Entity::Work, work, fields)
            }
        };
        apply_change(
            &tx,
            &lib,
            &change(true, json!({ "kind": "book", "title": "T1" })),
            h1,
        )
        .unwrap();
        apply_change(&tx, &lib, &change(false, json!({ "title": "T3" })), h3).unwrap();

        // Older than the title, newer than the kind: only the kind is written...
        let wrote = apply_change(
            &tx,
            &lib,
            &change(false, json!({ "kind": "comic", "title": "T2" })),
            h2,
        )
        .unwrap();

        assert!(wrote);
        let row: (String, String) = tx
            .query_row("SELECT kind, title FROM works WHERE id = ?1", [work], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(row, ("comic".to_owned(), "T3".to_owned()));
        // ...and the title keeps its version, so nothing between h2 and h3 can win later.
        let version = |field| field_version(&tx, &lib, Entity::Work, work, field).unwrap();
        assert_eq!(version("title").as_deref(), Some(h3));
        assert_eq!(version("kind").as_deref(), Some(h2));
    }

    /// The laptop and the phone know each other, so the phone's key is registered
    /// on a third device that shares the library.
    fn third_device_that_knows_both() -> (Device, Device, Device) {
        let (laptop, mut phone) = laptop_admin_phone_reader();
        phone.pull_from(&laptop);
        let mut laptop = laptop;
        laptop.pull_from(&phone);
        let mut other = Device::sharing(&laptop);
        other.pull_from(&laptop);
        (laptop, phone, other)
    }

    #[test]
    fn a_registered_device_cannot_change_a_key_even_signing_with_its_own() {
        let (laptop, phone, mut other) = third_device_that_knows_both();
        let phone_key = ops::signing_key(&phone.0).unwrap().public_key();
        assert!(
            registered_keys(&other)
                .iter()
                .any(|(id, key)| *id == device_id_of(&phone) && *key == phone_key),
            "the phone is registered"
        );
        let swap = |target: String| {
            forged(
                &phone,
                vec![Change::update(
                    Entity::Device,
                    target,
                    json!({ "public_key": "AAAA" }),
                )],
            )
        };
        let before = registered_keys(&other);

        // The laptop's key, and the phone's own.
        let report = merge(
            &mut other.0,
            vec![swap(device_id_of(&laptop)), swap(device_id_of(&phone))],
        )
        .unwrap();

        assert_eq!(report.rejected, 2);
        assert_eq!(registered_keys(&other), before);
    }

    #[test]
    fn only_a_device_registering_itself_may_bring_its_own_key() {
        let laptop = Device::new();
        let stranger = Device::empty_for(&laptop);
        let key = ops::signing_key(&stranger.0).unwrap().public_key();
        let mut target = Device::sharing(&laptop);
        let someone_else = "01997c7e-0000-7000-8000-0000000000c1";
        let stranger_id = device_id_of(&stranger);

        let for_another_device = forged(
            &stranger,
            vec![Change::update(
                Entity::Device,
                someone_else,
                json!({ "public_key": key }),
            )],
        );
        let updating_itself = forged(
            &stranger,
            vec![Change::update(
                Entity::Device,
                &stranger_id,
                json!({ "public_key": key }),
            )],
        );
        let registering_itself = forged(
            &stranger,
            vec![Change::create(
                Entity::Device,
                &stranger_id,
                json!({ "public_key": key, "name": "Stranger" }),
            )],
        );

        let a = merge(&mut target.0, vec![for_another_device]).unwrap();
        let b = merge(&mut target.0, vec![updating_itself]).unwrap();
        let c = merge(&mut target.0, vec![registering_itself]).unwrap();

        assert_eq!(a.rejected, 1, "a key offered for another device");
        assert_eq!(b.rejected, 1, "an update is not a registration");
        assert_eq!(
            c.rejected, 0,
            "the registration itself is how a key becomes known"
        );
    }
}
