//! Replication between devices without an arbiter (D31-D33). See docs/architecture/sync.md.
//!
//! Every device keeps every replicable operation of the libraries it has. Syncing
//! exchanges the ones the other side lacks, library by library, and `merge` applies
//! them with rules that make all devices converge on the same state whatever order
//! they arrive in.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use kosmotheka_domain::{Action, Change, Entity, Hlc, HlcClock, Operation};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::Result;
use crate::ops::{self, read_state, write_state};
use crate::search;

/// Parents before children, so every reference exists when a row is created.
pub const ORDER: [Entity; 11] = [
    Entity::Library,
    Entity::Publisher,
    Entity::Person,
    Entity::Member,
    Entity::Work,
    Entity::WorkContributor,
    Entity::Edition,
    Entity::BookDetails,
    Entity::Copy,
    Entity::Wish,
    Entity::Revocation,
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
}

impl MergeReport {
    pub fn add(&mut self, other: &MergeReport) {
        self.received += other.received;
        self.known += other.known;
        self.rejected += other.rejected;
        self.skipped_changes += other.skipped_changes;
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
        for (i, field) in fields.iter().enumerate() {
            map.insert((*field).into(), to_json(row.get_ref(i + 1)?));
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
        // Every device has the default library row; alone it is not worth a baseline.
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
        "SELECT id, library_id, hlc, intent, changes, schema_version FROM operations
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
                ))
            })?;
        for row in rows {
            let (id, library, hlc, intent, changes, schema_version) = row?;
            operations.push(SyncOperation {
                schema_version,
                operation: Operation {
                    id,
                    library,
                    hlc: hlc
                        .parse()
                        .map_err(|_| crate::error::Error::CorruptState("operations.hlc"))?,
                    intent,
                    changes: serde_json::from_str(&changes)?,
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
    for SyncOperation {
        schema_version,
        operation,
    } in incoming
    {
        if is_known(tx, &operation.id)? {
            report.known += 1;
            continue;
        }
        if schema_version != local_schema || operation.validate().is_err() {
            report.rejected += 1;
            continue;
        }
        latest = Some(clock.receive(&operation.hlc, now_millis()));

        let library = operation.library.as_str();
        let hlc = operation.hlc.to_string();
        let mut affected = BTreeSet::new();
        for change in &operation.changes {
            affected.extend(search::affected_editions(tx, library, change)?);
            if !apply_remote(tx, library, change, &hlc)? {
                report.skipped_changes += 1;
            }
            affected.extend(search::affected_editions(tx, library, change)?);
        }
        search::reindex(tx, library, &affected)?;
        ops::log_operation(tx, &operation, schema_version)?;
        report.received += 1;
    }

    if let Some(latest) = latest {
        ops::set_last_hlc(tx, &latest)?;
    }
    Ok(report)
}

/// One change under the merge rules. Returns whether anything was written.
fn apply_remote(tx: &Transaction<'_>, library: &str, change: &Change, hlc: &str) -> Result<bool> {
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

fn is_tombstoned(conn: &Connection, library: &str, entity: Entity, id: &str) -> Result<bool> {
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
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook, ValidEdition};

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::ops::DEFAULT_LIBRARY_ID as LIB;

    /// A simulated device: its own database, as a phone or a laptop would have.
    struct Device(Connection);

    impl Device {
        fn new() -> Self {
            Device(db::open_in_memory().unwrap())
        }

        fn add(&mut self, title: &str, isbn: Option<&str>, owner: &str) -> String {
            catalog::create_book(&mut self.0, LIB, &book(title, isbn, owner), None)
                .unwrap()
                .edition_id
        }

        fn titles(&self) -> Vec<String> {
            catalog::list_books(&self.0, LIB, &BookFilter::default())
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
                  UNION SELECT library_id FROM editions UNION SELECT library_id FROM wishes
                  ORDER BY 1",
            )
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    /// Vectors of a device that has nothing of the default library yet.
    fn everything() -> LibraryVectors {
        LibraryVectors::from([(LIB.to_owned(), VersionVector::new())])
    }

    fn assert_converged(devices: &[&Device]) {
        let first = devices[0].state();
        for other in &devices[1..] {
            assert_eq!(other.state(), first, "devices diverged");
        }
    }

    #[test]
    fn books_added_offline_on_two_devices_end_up_on_both() {
        let mut laptop = Device::new();
        let mut phone = Device::new();
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
        let mut b = Device::new();
        a.add("A", None, "Ana");
        a.sync_with(&mut b);

        let again = b.pull_from(&a);

        assert_eq!(again, MergeReport::default());
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn later_edit_of_the_same_field_wins_everywhere() {
        let mut a = Device::new();
        let mut b = Device::new();
        let id = a.add("Draft", None, "Ana");
        a.sync_with(&mut b);

        catalog::update_edition(&mut a.0, LIB, &id, &edition("From A", None), None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        catalog::update_edition(&mut b.0, LIB, &id, &edition("From B", None), None).unwrap();
        a.sync_with(&mut b);

        assert_eq!(a.titles(), vec!["From B"]);
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn edits_of_different_fields_are_both_kept() {
        let mut a = Device::new();
        let mut b = Device::new();
        let id = a.add("Book", None, "Ana");
        a.sync_with(&mut b);

        let mut year = edition("Book", None);
        year.published_year = Some(1968);
        catalog::update_edition(&mut a.0, LIB, &id, &year, None).unwrap();
        let mut language = edition("Book", None);
        language.language = Some("en".into());
        catalog::update_edition(&mut b.0, LIB, &id, &language, None).unwrap();
        a.sync_with(&mut b);

        let detail = catalog::get_book(&a.0, LIB, &id).unwrap();
        assert_eq!(detail.published_year, Some(1968));
        assert_eq!(detail.language.as_deref(), Some("en"));
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn a_delete_beats_a_concurrent_edit() {
        let mut a = Device::new();
        let mut b = Device::new();
        let id = a.add("Doomed", None, "Ana");
        a.sync_with(&mut b);

        catalog::delete_book(&mut a.0, LIB, &id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        catalog::update_edition(&mut b.0, LIB, &id, &edition("Edited later", None), None).unwrap();
        catalog::add_copy(&mut b.0, LIB, &id, &book("x", None, "Bea").copy).unwrap();
        a.sync_with(&mut b);

        assert!(a.titles().is_empty());
        assert!(b.titles().is_empty());
        assert_converged(&[&a, &b]);
    }

    #[test]
    fn the_same_book_added_on_both_devices_stays_duplicated() {
        let mut a = Device::new();
        let mut b = Device::new();
        a.add("Same", Some("9780306406157"), "Ana");
        b.add("Same", Some("9780306406157"), "Ana");

        a.sync_with(&mut b);

        assert_eq!(a.titles(), vec!["Same", "Same"]);
        let options = catalog::catalog_options(&a.0, LIB).unwrap();
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
        let mut b = Device::new();
        let mut c = Device::new();
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
        a.0.execute_batch(
            "INSERT INTO works (library_id, id, kind, title)
             VALUES ('00000000-0000-7000-8000-000000000001', '01997c7e-0000-7000-8000-000000000001', 'book', 'Old');
             INSERT INTO editions (library_id, id, work_id, title)
             VALUES ('00000000-0000-7000-8000-000000000001', '01997c7e-0000-7000-8000-000000000002',
                     '01997c7e-0000-7000-8000-000000000001', 'Old');",
        )
        .unwrap();
        a.0.execute("DELETE FROM local_state WHERE key = ?1", [BASELINE_KEY])
            .unwrap();
        ensure_baseline(&mut a.0).unwrap();
        let mut b = Device::new();

        b.pull_from(&a);

        assert_eq!(b.titles(), vec!["Old"]);
        let intent: String =
            a.0.query_row(
                "SELECT intent FROM operations WHERE replicable = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(intent, "sync.baseline");
    }

    #[test]
    fn history_from_before_sync_is_not_replicated() {
        let mut a = Device::new();
        a.add("A", None, "Ana");
        a.0.execute("UPDATE operations SET replicable = 0", [])
            .unwrap();
        let b = Device::new();

        assert!(
            operations_since(&a.0, &version_vectors(&b.0, &[LIB.to_owned()]).unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn operations_from_another_schema_version_are_rejected() {
        let mut a = Device::new();
        a.add("A", None, "Ana");
        let mut b = Device::new();
        let mut incoming = operations_since(&a.0, &everything()).unwrap();
        incoming[0].schema_version -= 1;

        let report = merge(&mut b.0, incoming).unwrap();

        assert_eq!(report.rejected, 1);
        assert!(b.titles().is_empty());
    }

    #[test]
    fn receiving_moves_the_local_clock_past_remote_timestamps() {
        let mut a = Device::new();
        a.add("A", None, "Ana");
        let mut b = Device::new();
        let incoming = operations_since(&a.0, &everything()).unwrap();
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
        let mut b = Device::new();
        a.add("Tehanu", None, "Ana");

        b.pull_from(&a);

        let filter = BookFilter {
            query: Some("tehanu".into()),
            ..BookFilter::default()
        };
        assert_eq!(catalog::list_books(&b.0, LIB, &filter).unwrap().len(), 1);
    }

    #[test]
    fn wishes_noted_on_the_phone_reach_the_desktop() {
        let mut desk = Device::new();
        let mut phone = Device::new();
        let wish = kosmotheka_domain::WishInput {
            title: "Tehanu".into(),
            ..Default::default()
        }
        .validate()
        .unwrap();
        crate::wishes::add_wish(&mut phone.0, LIB, &wish).unwrap();

        desk.sync_with(&mut phone);

        let titles: Vec<String> = crate::wishes::list_wishes(&desk.0, LIB)
            .unwrap()
            .into_iter()
            .map(|w| w.title)
            .collect();
        assert_eq!(titles, vec!["Tehanu"]);
        assert_converged(&[&desk, &phone]);
    }

    #[test]
    fn deleting_a_library_empties_it_on_every_device() {
        let mut a = Device::new();
        let mut b = Device::new();
        a.add("Doomed", None, "Ana");
        a.sync_with(&mut b);

        let tx = a.0.transaction().unwrap();
        ops::record(
            &tx,
            LIB,
            "library.delete",
            vec![Change::delete(Entity::Library, LIB)],
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
        let mut b = Device::new();
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
    fn merge_conformance_cases() {
        let cases: Vec<MergeCase> = serde_json::from_str(MERGE_CASES).unwrap();
        assert!(!cases.is_empty());

        for case in cases {
            for order in &case.deliveries {
                let mut device = Device::new();
                // Only rows the case creates count: every device has the default library.
                let before = state(&device.0);
                let schema_version = ops::schema_version(&device.0).unwrap();
                for &index in order {
                    let operation = case.operations[index].clone();
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
                        if untouched {
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
                            fields: row.into_iter().collect(),
                        });
                    }
                }
                let key =
                    |row: &&ExpectedRow| (row.library.clone(), row.entity.clone(), row.id.clone());
                let mut expected = case.expect.rows.iter().collect::<Vec<_>>();
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
}
