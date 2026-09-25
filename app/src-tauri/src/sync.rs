//! Replication between devices without an arbiter (D31-D33). See docs/architecture/sync.md.
//!
//! Every device keeps every replicable operation. Syncing exchanges the ones the
//! other side lacks, and `merge` applies them with rules that make all devices
//! converge on the same state whatever order they arrive in.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use library_domain::{Action, Change, Entity, Hlc, HlcClock, Operation};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::Result;
use crate::ops::{self, read_state, write_state};
use crate::search;

/// Parents before children, so every reference exists when a row is created.
pub const ORDER: [Entity; 9] = [
    Entity::Publisher,
    Entity::Person,
    Entity::Member,
    Entity::Work,
    Entity::WorkContributor,
    Entity::Edition,
    Entity::BookDetails,
    Entity::Copy,
    Entity::Wish,
];

const BASELINE_KEY: &str = "sync_baseline";

/// Last HLC this device holds from each author device.
pub type VersionVector = BTreeMap<String, String>;

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

/// Rows of one entity, with the field names operations use (`id` included).
pub fn entity_rows(conn: &Connection, entity: Entity) -> Result<Vec<Map<String, Value>>> {
    let fields: Vec<&str> = entity.fields().iter().map(|spec| spec.name).collect();
    let sql = format!(
        "SELECT {}, {} FROM {} ORDER BY {}",
        entity.id_column(),
        fields.join(", "),
        entity.table(),
        entity.id_column()
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        let mut map = Map::new();
        map.insert("id".into(), json!(row.get::<_, String>(0)?));
        for (i, field) in fields.iter().enumerate() {
            map.insert((*field).into(), to_json(row.get_ref(i + 1)?));
        }
        Ok(map)
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Once per device: logs everything it already has as one replicable operation, so
/// rows from before sync existed can travel too (D31). Empty catalogues log nothing.
pub fn ensure_baseline(conn: &mut Connection) -> Result<()> {
    if read_state(conn, BASELINE_KEY)?.is_some() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    let mut changes = Vec::new();
    for entity in ORDER {
        for mut row in entity_rows(&tx, entity)? {
            let id = row
                .remove("id")
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            changes.push(Change::create(entity, id, Value::Object(row)));
        }
    }
    if !changes.is_empty() {
        ops::record_existing(&tx, "sync.baseline", changes)?;
    }
    write_state(&tx, BASELINE_KEY, Some("done"))?;
    tx.commit()?;
    Ok(())
}

pub fn version_vector(conn: &Connection) -> Result<VersionVector> {
    let mut statement = conn.prepare(
        "SELECT device_id, max(hlc) FROM operations WHERE replicable = 1 GROUP BY device_id",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Replicable operations the owner of `vector` lacks, oldest first.
pub fn operations_since(conn: &Connection, vector: &VersionVector) -> Result<Vec<SyncOperation>> {
    let mut statement = conn.prepare(
        "SELECT id, hlc, intent, changes, schema_version FROM operations
          WHERE replicable = 1
            AND hlc > coalesce((SELECT value FROM json_each(?1) WHERE key = device_id), '')
          ORDER BY hlc",
    )?;
    let rows = statement.query_map([serde_json::to_string(vector)?], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    let mut operations = Vec::new();
    for row in rows {
        let (id, hlc, intent, changes, schema_version) = row?;
        operations.push(SyncOperation {
            schema_version,
            operation: Operation {
                id,
                hlc: hlc
                    .parse()
                    .map_err(|_| crate::error::Error::CorruptState("operations.hlc"))?,
                intent,
                changes: serde_json::from_str(&changes)?,
            },
        });
    }
    Ok(operations)
}

/// Applies operations from other devices (D32). Never fails because of their content:
/// what cannot be applied is skipped and counted. Only storage errors abort.
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
    let mut affected = BTreeSet::new();
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

        let hlc = operation.hlc.to_string();
        for change in &operation.changes {
            affected.extend(search::affected_editions(tx, change)?);
            if !apply_remote(tx, change, &hlc)? {
                report.skipped_changes += 1;
            }
            affected.extend(search::affected_editions(tx, change)?);
        }
        ops::log_operation(tx, &operation, schema_version)?;
        report.received += 1;
    }

    search::reindex(tx, &affected)?;
    if let Some(latest) = latest {
        ops::set_last_hlc(tx, &latest)?;
    }
    Ok(report)
}

/// One change under the merge rules. Returns whether anything was written.
fn apply_remote(tx: &Transaction<'_>, change: &Change, hlc: &str) -> Result<bool> {
    let entity = change.entity;
    if is_tombstoned(tx, entity, &change.id)? {
        return Ok(false); // the delete wins
    }

    // A savepoint lets a change that breaks a constraint (a reference to a row
    // deleted elsewhere) be undone alone, without losing the rest of the sync.
    tx.execute_batch("SAVEPOINT change")?;
    let outcome = (|| -> Result<bool> {
        match change.action {
            Action::Delete => {
                ops::delete_row(tx, entity, &change.id)?;
                ops::write_versions(tx, change, hlc)?;
                Ok(true)
            }
            Action::Create | Action::Update => {
                let exists = row_exists(tx, entity, &change.id)?;
                if !exists && change.action == Action::Update {
                    return Ok(false); // deleted in cascade with its parent
                }
                let mut newer = Map::new();
                for (field, value) in &change.fields {
                    let stored = field_version(tx, entity, &change.id, field)?;
                    if stored.as_deref().is_none_or(|stored| stored < hlc) {
                        newer.insert(field.clone(), value.clone());
                    }
                }
                if !exists {
                    ops::insert_row(tx, entity, &change.id, newer.iter())?;
                } else if newer.is_empty() {
                    return Ok(false);
                } else {
                    ops::update_row(tx, entity, &change.id, newer.iter())?;
                }
                let written = Change {
                    fields: newer,
                    ..change.clone()
                };
                ops::write_versions(tx, &written, hlc)?;
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

fn is_tombstoned(conn: &Connection, entity: Entity, id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM tombstones WHERE entity = ?1 AND entity_id = ?2",
            params![entity.table(), id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn row_exists(conn: &Connection, entity: Entity, id: &str) -> Result<bool> {
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

fn field_version(
    conn: &Connection,
    entity: Entity,
    id: &str,
    field: &str,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT hlc FROM field_versions WHERE entity = ?1 AND entity_id = ?2 AND field = ?3",
            params![entity.table(), id, field],
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
    use library_domain::{CopyInput, EditionInput, Format, ValidBook, ValidEdition};

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::db;

    /// A simulated device: its own database, as a phone or a laptop would have.
    struct Device(Connection);

    impl Device {
        fn new() -> Self {
            Device(db::open_in_memory().unwrap())
        }

        fn add(&mut self, title: &str, isbn: Option<&str>, owner: &str) -> String {
            catalog::create_book(&mut self.0, &book(title, isbn, owner), None)
                .unwrap()
                .edition_id
        }

        fn titles(&self) -> Vec<String> {
            catalog::list_books(&self.0, &BookFilter::default())
                .unwrap()
                .into_iter()
                .map(|b| b.title)
                .collect()
        }

        /// Pull what `other` has and this device lacks.
        fn pull_from(&mut self, other: &Device) -> MergeReport {
            let missing = operations_since(&other.0, &version_vector(&self.0).unwrap()).unwrap();
            merge(&mut self.0, missing).unwrap()
        }

        /// A full exchange, both ways, as a real sync session does.
        fn sync_with(&mut self, other: &mut Device) {
            self.pull_from(other);
            other.pull_from(self);
        }

        /// Every catalogue row, for comparing devices. Creation times are local, so left out.
        fn state(&self) -> Vec<(String, Vec<Map<String, Value>>)> {
            ORDER
                .iter()
                .map(|entity| {
                    (
                        entity.table().to_owned(),
                        entity_rows(&self.0, *entity).unwrap(),
                    )
                })
                .collect()
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

        catalog::update_edition(&mut a.0, &id, &edition("From A", None), None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        catalog::update_edition(&mut b.0, &id, &edition("From B", None), None).unwrap();
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
        catalog::update_edition(&mut a.0, &id, &year, None).unwrap();
        let mut language = edition("Book", None);
        language.language = Some("en".into());
        catalog::update_edition(&mut b.0, &id, &language, None).unwrap();
        a.sync_with(&mut b);

        let detail = catalog::get_book(&a.0, &id).unwrap();
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

        catalog::delete_book(&mut a.0, &id).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        catalog::update_edition(&mut b.0, &id, &edition("Edited later", None), None).unwrap();
        catalog::add_copy(&mut b.0, &id, &book("x", None, "Bea").copy).unwrap();
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
        let options = catalog::catalog_options(&a.0).unwrap();
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
            "INSERT INTO works (id, kind, title) VALUES ('01997c7e-0000-7000-8000-000000000001', 'book', 'Old');
             INSERT INTO editions (id, work_id, title)
             VALUES ('01997c7e-0000-7000-8000-000000000002', '01997c7e-0000-7000-8000-000000000001', 'Old');",
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
            operations_since(&a.0, &version_vector(&b.0).unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn operations_from_another_schema_version_are_rejected() {
        let mut a = Device::new();
        a.add("A", None, "Ana");
        let mut b = Device::new();
        let mut incoming = operations_since(&a.0, &VersionVector::new()).unwrap();
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
        let incoming = operations_since(&a.0, &VersionVector::new()).unwrap();
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
        assert_eq!(catalog::list_books(&b.0, &filter).unwrap().len(), 1);
    }

    #[test]
    fn wishes_noted_on_the_phone_reach_the_desktop() {
        let mut desk = Device::new();
        let mut phone = Device::new();
        let wish = library_domain::WishInput {
            title: "Tehanu".into(),
            ..Default::default()
        }
        .validate()
        .unwrap();
        crate::wishes::add_wish(&mut phone.0, &wish).unwrap();

        desk.sync_with(&mut phone);

        let titles: Vec<String> = crate::wishes::list_wishes(&desk.0)
            .unwrap()
            .into_iter()
            .map(|w| w.title)
            .collect();
        assert_eq!(titles, vec!["Tehanu"]);
        assert_converged(&[&desk, &phone]);
    }
}
