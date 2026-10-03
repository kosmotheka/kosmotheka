//! Replication between devices without an arbiter (D31-D33). See docs/architecture/sync.md.
//!
//! Every device keeps every replicable operation of the libraries it has. Syncing
//! exchanges the ones the other side lacks, library by library, and `merge` applies
//! them with rules that make all devices converge on the same state whatever order
//! they arrive in.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use kosmotheka_domain::{Action, Change, Entity, FieldType, Hlc, HlcClock, Operation, verify};
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
        "SELECT id, library_id, hlc, intent, changes, schema_version, signature FROM operations
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
                ))
            })?;
        for row in rows {
            let (id, library, hlc, intent, changes, schema_version, signature) = row?;
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
                for (field, value) in &change.fields {
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
    const MERGE_CASES: &str = include_str!("../../../spec/conformance/library-merge.json");

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
        let cases: Vec<MergeCase> = serde_json::from_str(MERGE_CASES).unwrap();
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
                    merge(
                        &mut device.0,
                        vec![SyncOperation {
                            schema_version,
                            operation,
                        }],
                    )
                    .unwrap();
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
}
