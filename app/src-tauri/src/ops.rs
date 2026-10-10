//! The only way to write catalog data: record an operation and apply its changes
//! in the same transaction (D05, D25).

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use kosmotheka_domain::{Action, Change, DeviceKey, Entity, Hlc, HlcClock, Operation};
use rand::RngExt;
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::search;

const DEVICE_ID_KEY: &str = "device_id";
/// 32 random bytes, base64url: the seed of this device's signing key (D56).
const SIGNING_SEED_KEY: &str = "signing_seed";

const LAST_HLC_KEY: &str = "last_hlc";
const ACTIVE_LIBRARY_KEY: &str = "active_library";

pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// Gives this installation a stable identity the first time it runs.
pub fn ensure_device_id(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO local_state (key, value) VALUES (?1, ?2)",
        params![DEVICE_ID_KEY, new_id()],
    )?;
    Ok(())
}

/// Woken every time this device records an operation of its own, so the app can sync
/// the change soon (D75). Process-wide because recording happens deep in the call tree.
/// Operations merged from other devices go through `sync::merge`, which never records,
/// so they never wake it.
static LOCAL_CHANGES: tokio::sync::Notify = tokio::sync::Notify::const_new();

#[cfg(test)]
thread_local! {
    /// How many operations this test thread recorded: tests share the process-wide
    /// notifier, so each one counts its own.
    static RECORDED_ON_THIS_THREAD: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Resolves after this device records an operation (a burst of them wakes it once).
pub async fn local_change() {
    LOCAL_CHANGES.notified().await;
}

/// Waits for a local change, then `gap` more to let a burst of them finish, and counts
/// the whole burst as one.
pub async fn local_change_burst(gap: std::time::Duration) {
    local_change().await;
    tokio::time::sleep(gap).await;
    // Changes recorded while waiting left a permit: they belong to this burst.
    let _ = tokio::time::timeout(std::time::Duration::ZERO, local_change()).await;
}

fn note_local_change() {
    LOCAL_CHANGES.notify_one();
    #[cfg(test)]
    RECORDED_ON_THIS_THREAD.with(|count| count.set(count.get() + 1));
}

#[cfg(test)]
pub(crate) fn recorded_on_this_thread() -> u32 {
    RECORDED_ON_THIS_THREAD.with(std::cell::Cell::get)
}

pub fn record(
    tx: &Transaction<'_>,
    library: &str,
    intent: &str,
    changes: Vec<Change>,
) -> Result<Operation> {
    record_operation(tx, library, intent, changes, true)
}

/// Logs changes that are already in the tables (the sync baseline, D31): versions
/// and log are written, the rows themselves are left as they are.
pub fn record_existing(
    tx: &Transaction<'_>,
    library: &str,
    intent: &str,
    changes: Vec<Change>,
) -> Result<Operation> {
    record_operation(tx, library, intent, changes, false)
}

/// All or nothing, even inside the caller's transaction: the registration this may
/// record first goes away if the operation itself fails.
fn record_operation(
    tx: &Transaction<'_>,
    library: &str,
    intent: &str,
    changes: Vec<Change>,
    apply_changes: bool,
) -> Result<Operation> {
    tx.execute_batch("SAVEPOINT record")?;
    match record_inner(tx, library, intent, changes, apply_changes) {
        Ok(operation) => {
            tx.execute_batch("RELEASE record")?;
            note_local_change();
            Ok(operation)
        }
        Err(error) => {
            tx.execute_batch("ROLLBACK TO record; RELEASE record")?;
            Err(error)
        }
    }
}

fn record_inner(
    tx: &Transaction<'_>,
    library: &str,
    intent: &str,
    changes: Vec<Change>,
    apply_changes: bool,
) -> Result<Operation> {
    let device_id = device_id(tx)?;
    let key = signing_key(tx)?;

    // The first time this device writes to a library, it registers the key it signs
    // with (D56), in an operation of its own: every device may register itself, so it
    // never turns the operation it comes with into a request (D73).
    let registers_itself = changes.iter().any(|change| {
        change.entity == Entity::Device && change.action == Action::Create && change.id == device_id
    });
    if !registers_itself {
        ensure_registered(tx, library)?;
    }
    let hlc = HlcClock::new(device_id.clone(), last_hlc(tx)?.as_ref()).tick(now_millis());

    // After the registration above, which may be the operation this one follows.
    let mut operation = Operation {
        id: new_id(),
        library: library.to_owned(),
        hlc,
        intent: intent.to_owned(),
        changes,
        previous: latest_of(tx, library, &device_id)?,
        signature: None,
    };
    operation.validate()?;
    operation.signature = Some(key.sign(&operation));
    // Judged like any operation received from another device, so all of them agree.
    if !crate::permissions::allowed(tx, &operation) {
        if !AS_REQUEST.get() {
            return Err(Error::NotAllowed);
        }
        // Asked for instead of done: kept, and travels, but changes nothing (D73).
        // The asker's note rides along as a row only an administrator could create.
        if let Some(note) = REQUEST_NOTE.with_borrow(Clone::clone) {
            operation.changes.push(Change::create(
                Entity::ChangeRequest,
                &operation.id,
                json!({ "note": note }),
            ));
            operation.validate()?;
            operation.signature = Some(key.sign(&operation));
        }
        REQUESTED.set(true);
        log_operation(tx, &operation, schema_version(tx)?, Status::Request)?;
        write_state(tx, LAST_HLC_KEY, Some(&operation.hlc.to_string()))?;
        return Ok(operation);
    }
    let hlc_text = operation.hlc.to_string();

    // Search text can change for rows that vanish (known before) or appear (known after).
    let mut affected = std::collections::BTreeSet::new();
    for change in &operation.changes {
        affected.extend(search::affected_editions(tx, library, change)?);
    }
    for change in &operation.changes {
        if apply_changes {
            match apply(tx, library, change) {
                // A fence on a device that is not registered here yet (the cloud's, D95): the merge skips a
                // change to a row that is not there, and so does this; the fence is read from the operation.
                Err(Error::MissingRow { .. })
                    if change.entity == Entity::Device && change.action == Action::Update =>
                {
                    continue;
                }
                other => other?,
            }
        }
        crate::permissions::note_author(tx, library, change, &device_id)?;
        write_versions(tx, library, change, &hlc_text)?;
    }
    for change in &operation.changes {
        affected.extend(search::affected_editions(tx, library, change)?);
    }
    search::reindex(tx, library, &affected)?;
    // The administrator knew everything up to the fence, so nothing is late here.
    crate::permissions::note_fences(tx, &operation)?;
    log_operation(tx, &operation, schema_version(tx)?, Status::Applied)?;
    write_state(tx, LAST_HLC_KEY, Some(&hlc_text))?;
    Ok(operation)
}

/// Registers this device in `library` if it has not yet (D56): its own `device`
/// row, with the key it signs with. Every device may register itself, so it never
/// turns into a request (D73).
pub fn ensure_registered(tx: &Transaction<'_>, library: &str) -> Result<()> {
    let device_id = device_id(tx)?;
    if is_registered(tx, library, &device_id)? {
        return Ok(());
    }
    let registration = Change::create(
        Entity::Device,
        &device_id,
        json!({
            "public_key": signing_key(tx)?.public_key(),
            "name": crate::lan::device_name(tx)?,
        }),
    );
    record_operation(tx, library, "device.register", vec![registration], true)?;
    Ok(())
}

/// Gives this database a chosen identity instead of a random one. `pub` for
/// `examples/fixture_devices.rs`, so a fixture device signs as the one the server
/// already knows; call it before anything is recorded.
pub fn set_identity(conn: &Connection, device_id: &str, signing_seed: [u8; 32]) -> Result<()> {
    write_state(conn, DEVICE_ID_KEY, Some(device_id))?;
    write_state(
        conn,
        SIGNING_SEED_KEY,
        Some(&URL_SAFE_NO_PAD.encode(signing_seed)),
    )
}

/// This device's signing key (D56), made the first time it is needed. It stays in
/// this database and never goes into a backup (D58).
pub fn signing_key(conn: &Connection) -> Result<DeviceKey> {
    let seed: [u8; 32] = match read_state(conn, SIGNING_SEED_KEY)? {
        Some(text) => URL_SAFE_NO_PAD
            .decode(text)
            .ok()
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(Error::CorruptState(SIGNING_SEED_KEY))?,
        None => {
            let seed: [u8; 32] = rand::rng().random();
            write_state(conn, SIGNING_SEED_KEY, Some(&URL_SAFE_NO_PAD.encode(seed)))?;
            seed
        }
    };
    Ok(DeviceKey::from_seed(seed))
}

fn is_registered(conn: &Connection, library: &str, device: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM devices WHERE library_id = ?1 AND id = ?2)",
        [library, device],
        |row| row.get(0),
    )?)
}

pub fn device_id(conn: &Connection) -> Result<String> {
    read_state(conn, DEVICE_ID_KEY)?.ok_or(Error::MissingDeviceId)
}

pub fn last_hlc(conn: &Connection) -> Result<Option<Hlc>> {
    read_state(conn, LAST_HLC_KEY)?
        .map(|text| text.parse::<Hlc>())
        .transpose()
        .map_err(|_| Error::CorruptState(LAST_HLC_KEY))
}

pub fn set_last_hlc(conn: &Connection, hlc: &Hlc) -> Result<()> {
    write_state(conn, LAST_HLC_KEY, Some(&hlc.to_string()))
}

pub fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

/// The library the user is working in: the stored one if it still exists, otherwise
/// the first one (D85). Startup guarantees there is always one.
pub fn active_library(conn: &Connection) -> Result<String> {
    current_library(conn)?.ok_or(Error::CorruptState(ACTIVE_LIBRARY_KEY))
}

/// `active_library`, or `None` while no library exists (mid-removal of the last one).
pub fn current_library(conn: &Connection) -> Result<Option<String>> {
    if let Some(id) = read_state(conn, ACTIVE_LIBRARY_KEY)?
        && crate::libraries::library_exists(conn, &id)?
    {
        return Ok(Some(id));
    }
    crate::libraries::first_library(conn)
}

pub fn set_active_library(conn: &Connection, library: &str) -> Result<()> {
    write_state(conn, ACTIVE_LIBRARY_KEY, Some(library))
}

pub fn forget_active_library(conn: &Connection) -> Result<()> {
    write_state(conn, ACTIVE_LIBRARY_KEY, None)
}

thread_local! {
    static AS_REQUEST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static REQUESTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static REQUEST_NOTE: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Runs `write` so that an operation its author may not make is kept as a request
/// for the administrators instead of refused (D73): the «Ask to add» of a reader,
/// with an optional note for whoever decides. Operations the author may make are
/// applied as usual, without the note. Also says whether anything became a request.
pub fn as_request<T>(note: Option<&str>, write: impl FnOnce() -> Result<T>) -> Result<(T, bool)> {
    let note = note.map(str::trim).filter(|note| !note.is_empty());
    AS_REQUEST.set(true);
    REQUESTED.set(false);
    REQUEST_NOTE.set(note.map(str::to_owned));
    let result = write();
    AS_REQUEST.set(false);
    REQUEST_NOTE.set(None);
    Ok((result?, REQUESTED.take()))
}

/// Whether this device may make `changes` in `library` now, without writing
/// anything: the screens offer «Ask to delete» instead of «Delete» when it may not.
pub fn would_allow(conn: &Connection, library: &str, changes: Vec<Change>) -> Result<bool> {
    let hlc = HlcClock::new(device_id(conn)?, last_hlc(conn)?.as_ref()).tick(now_millis());
    let mut operation = Operation {
        id: new_id(),
        library: library.to_owned(),
        hlc,
        intent: "check".to_owned(),
        changes,
        previous: latest_of(conn, library, &device_id(conn)?)?,
        signature: None,
    };
    // Unsigned operations count as a reader's.
    operation.signature = Some(signing_key(conn)?.sign(&operation));
    Ok(crate::permissions::allowed(conn, &operation))
}

/// What became of an operation: applied, or kept as a request because its author
/// was not allowed to make it (D73).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Applied,
    Request,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Applied => "applied",
            Status::Request => "request",
        }
    }
}

pub fn status_of(conn: &Connection, operation_id: &str) -> Result<Status> {
    let status: String = conn.query_row(
        "SELECT status FROM operations WHERE id = ?1",
        [operation_id],
        |row| row.get(0),
    )?;
    Ok(if status == "request" {
        Status::Request
    } else {
        Status::Applied
    })
}

pub fn set_status(conn: &Connection, operation_id: &str, status: Status) -> Result<()> {
    conn.execute(
        "UPDATE operations SET status = ?2 WHERE id = ?1",
        [operation_id, status.as_str()],
    )?;
    Ok(())
}

/// The id of the latest replicable operation `device` has in `library`: the one
/// the next operation of that device chains to (D87).
pub fn latest_of(conn: &Connection, library: &str, device: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM operations
              WHERE library_id = ?1 AND device_id = ?2 AND replicable = 1
              ORDER BY hlc DESC LIMIT 1",
            [library, device],
            |row| row.get(0),
        )
        .optional()?)
}

/// Appends to the log. The author device is the one inside the HLC.
pub fn log_operation(
    conn: &Connection,
    operation: &Operation,
    schema_version: i64,
    status: Status,
) -> Result<()> {
    conn.execute(
        "INSERT INTO operations (id, library_id, hlc, device_id, intent, changes, schema_version, replicable, signature, status, previous)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10)",
        params![
            operation.id,
            operation.library,
            operation.hlc.to_string(),
            operation.hlc.device,
            operation.intent,
            serde_json::to_string(&operation.changes)?,
            schema_version,
            operation.signature,
            status.as_str(),
            operation.previous,
        ],
    )?;
    Ok(())
}

/// Remembers which HLC last wrote each field, and leaves a tombstone for deletes (D32).
pub fn write_versions(conn: &Connection, library: &str, change: &Change, hlc: &str) -> Result<()> {
    let entity = change.entity.table();
    match change.action {
        Action::Create | Action::Update => {
            for field in change.fields.keys() {
                conn.execute(
                    "INSERT OR REPLACE INTO field_versions (library_id, entity, entity_id, field, hlc)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![library, entity, change.id, field, hlc],
                )?;
            }
        }
        Action::Delete => {
            conn.execute(
                "DELETE FROM field_versions WHERE library_id = ?1 AND entity = ?2 AND entity_id = ?3",
                params![library, entity, change.id],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO tombstones (library_id, entity, entity_id, hlc)
                 VALUES (?1, ?2, ?3, ?4)",
                params![library, entity, change.id, hlc],
            )?;
        }
    }
    Ok(())
}

/// Table and column names come from the domain schema, never from the change
/// itself: `validate` has already rejected any field the schema does not know.
fn apply(tx: &Transaction<'_>, library: &str, change: &Change) -> Result<()> {
    let affected = match change.action {
        Action::Create => insert_row(tx, change.entity, library, &change.id, change.fields.iter())?,
        Action::Update => update_row(tx, change.entity, library, &change.id, change.fields.iter())?,
        Action::Delete if change.entity == Entity::Library => delete_library(tx, library)?,
        Action::Delete => delete_row(tx, change.entity, library, &change.id)?,
    };
    if affected != 1 {
        return Err(Error::MissingRow {
            entity: change.entity.table(),
            id: change.id.clone(),
        });
    }
    Ok(())
}

pub fn insert_row<'a>(
    conn: &Connection,
    entity: Entity,
    library: &str,
    id: &str,
    fields: impl Iterator<Item = (&'a String, &'a Value)>,
) -> Result<usize> {
    let (columns, values) = known_columns(entity, fields);
    // A library row is keyed by its own id; every other row also names its library.
    let mut keys = vec![entity.id_column()];
    let mut args = vec![SqlValue::Text(id.to_owned())];
    if entity != Entity::Library {
        keys.insert(0, entity.library_column());
        args.insert(0, SqlValue::Text(library.to_owned()));
    }
    let all: Vec<&str> = keys.into_iter().chain(columns).collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        entity.table(),
        all.join(", "),
        vec!["?"; all.len()].join(", "),
    );
    Ok(conn.execute(&sql, params_from_iter(args.into_iter().chain(values)))?)
}

pub fn update_row<'a>(
    conn: &Connection,
    entity: Entity,
    library: &str,
    id: &str,
    fields: impl Iterator<Item = (&'a String, &'a Value)>,
) -> Result<usize> {
    let (columns, values) = known_columns(entity, fields);
    if columns.is_empty() {
        return Ok(0);
    }
    let assignments: Vec<String> = columns.iter().map(|c| format!("{c} = ?")).collect();
    let sql = format!(
        "UPDATE {} SET {} WHERE {} = ? AND {} = ?",
        entity.table(),
        assignments.join(", "),
        entity.library_column(),
        entity.id_column()
    );
    let args = values.into_iter().chain([
        SqlValue::Text(library.to_owned()),
        SqlValue::Text(id.to_owned()),
    ]);
    Ok(conn.execute(&sql, params_from_iter(args))?)
}

pub fn delete_row(conn: &Connection, entity: Entity, library: &str, id: &str) -> Result<usize> {
    Ok(conn.execute(
        &format!(
            "DELETE FROM {} WHERE {} = ?1 AND {} = ?2",
            entity.table(),
            entity.library_column(),
            entity.id_column()
        ),
        [library, id],
    )?)
}

/// Removes a library and everything in it (D65). Returns the library rows deleted,
/// like `delete_row`. The operation log stays: it is the record of what happened (D66).
pub fn delete_library(conn: &Connection, library: &str) -> Result<usize> {
    clear_library(conn, library)?;
    delete_row(conn, Entity::Library, library, library)
}

/// Removes everything a library holds but the library row itself.
pub fn clear_library(conn: &Connection, library: &str) -> Result<()> {
    // Children before parents, so no foreign key is left dangling.
    for entity in LIBRARY_CONTENTS_CHILDREN_FIRST {
        conn.execute(
            &format!("DELETE FROM {} WHERE library_id = ?1", entity.table()),
            [library],
        )?;
    }
    conn.execute(
        "DELETE FROM field_versions WHERE library_id = ?1",
        [library],
    )?;
    conn.execute("DELETE FROM tombstones WHERE library_id = ?1", [library])?;
    conn.execute("DELETE FROM search_index WHERE library_id = ?1", [library])?;
    conn.execute("DELETE FROM row_authors WHERE library_id = ?1", [library])?;
    conn.execute("DELETE FROM device_fences WHERE library_id = ?1", [library])?;
    conn.execute("DELETE FROM device_seals WHERE library_id = ?1", [library])?;
    conn.execute(
        "DELETE FROM device_fence_chains WHERE library_id = ?1",
        [library],
    )?;
    Ok(())
}

/// Every entity inside a library, children before parents.
pub(crate) const LIBRARY_CONTENTS_CHILDREN_FIRST: [Entity; 21] = [
    Entity::Copy,
    Entity::BookDetails,
    Entity::ComicDetails,
    Entity::BoardgameDetails,
    Entity::FilmDetails,
    Entity::TvSeriesDetails,
    Entity::Edition,
    Entity::WorkTag,
    Entity::WorkGenre,
    Entity::SeriesEntry,
    Entity::WorkRelation,
    Entity::WorkContributor,
    Entity::Work,
    Entity::Person,
    Entity::Publisher,
    Entity::Tag,
    Entity::Series,
    Entity::Member,
    Entity::Revocation,
    Entity::Device,
    Entity::ChangeRequest,
];

/// Column names are taken from the schema, so unknown keys can never reach SQL.
fn known_columns<'a>(
    entity: Entity,
    fields: impl Iterator<Item = (&'a String, &'a Value)>,
) -> (Vec<&'static str>, Vec<SqlValue>) {
    fields
        .filter_map(|(name, value)| entity.field(name).map(|spec| (spec.name, to_sql(value))))
        .unzip()
}

fn to_sql(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => n.as_i64().map_or(SqlValue::Null, SqlValue::Integer),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// Per-device values (never synced or exported).
pub fn read_state(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM local_state WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn write_state(conn: &Connection, key: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => conn.execute(
            "INSERT OR REPLACE INTO local_state (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?,
        None => conn.execute("DELETE FROM local_state WHERE key = ?1", [key])?,
    };
    Ok(())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Parses a stored operation's `changes` column, skipping any change whose entity
/// this build no longer knows: an entity dropped from the schema (like `wish`,
/// D61) can still sit in an old local log, and it must not stop it from being
/// read for local, read-only purposes (listing or describing a request). Lenient
/// on purpose: never use it to rebuild what is sent to another device, since
/// dropping a change would no longer match the operation's signature (see
/// `sync::operations_since`, which skips the whole operation instead).
pub fn parse_known_changes(raw: &str) -> Result<Vec<Change>> {
    let values: Vec<Value> = serde_json::from_str(raw)?;
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{Entity, OperationErrorCode};
    use serde_json::json;

    use super::*;
    use crate::db;

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn records_the_operation_and_applies_its_changes() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = active_library(&conn).unwrap();
        let member_id = new_id();
        let tx = conn.transaction().unwrap();

        let op = record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                &member_id,
                json!({ "name": "Tersoal" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();

        let name: String = conn
            .query_row(
                "SELECT name FROM members WHERE id = ?1",
                [&member_id],
                |r| r.get(0),
            )
            .unwrap();
        let logged_intent: String = conn
            .query_row(
                "SELECT intent FROM operations WHERE id = ?1",
                [&op.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Tersoal");
        assert_eq!(logged_intent, "member.add");
    }

    #[tokio::test]
    async fn recording_an_operation_wakes_the_local_change_notifier() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = active_library(&conn).unwrap();
        let before = recorded_on_this_thread();
        let tx = conn.transaction().unwrap();
        record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                new_id(),
                json!({ "name": "Tersoal" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();

        assert!(recorded_on_this_thread() > before);
        // The permit is stored, so waiting after the fact still resolves.
        tokio::time::timeout(std::time::Duration::from_secs(1), local_change())
            .await
            .expect("the notifier was woken");
    }

    #[test]
    fn each_operation_gets_a_later_hlc() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = active_library(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        let first = record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                new_id(),
                json!({ "name": "A" }),
            )],
        )
        .unwrap();
        let second = record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                new_id(),
                json!({ "name": "B" }),
            )],
        )
        .unwrap();
        assert!(second.hlc > first.hlc);
    }

    #[test]
    fn updates_and_deletes_apply_to_existing_rows() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = active_library(&conn).unwrap();
        let id = new_id();
        let tx = conn.transaction().unwrap();
        record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(Entity::Member, &id, json!({ "name": "A" }))],
        )
        .unwrap();
        record(
            &tx,
            &lib,
            "member.rename",
            vec![Change::update(Entity::Member, &id, json!({ "name": "B" }))],
        )
        .unwrap();
        let renamed: String = tx
            .query_row("SELECT name FROM members", [], |r| r.get(0))
            .unwrap();
        record(
            &tx,
            &lib,
            "member.remove",
            vec![Change::delete(Entity::Member, &id)],
        )
        .unwrap();

        assert_eq!(renamed, "B");
        assert_eq!(count(&tx, "members"), 0);
        // Three, plus the two of the first start: the library's creation and the
        // registration of this device's key that came with it.
        assert_eq!(count(&tx, "operations"), 5);
    }

    #[test]
    fn invalid_operation_writes_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = active_library(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        let logged = count(&tx, "operations");

        let error = record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                new_id(),
                json!({ "name": "A", "colour": "blue" }),
            )],
        )
        .unwrap_err();

        assert!(
            matches!(error, Error::InvalidOperation(e) if e.code == OperationErrorCode::UnknownField)
        );
        assert_eq!(count(&tx, "members"), 0);
        assert_eq!(count(&tx, "operations"), logged);
    }

    #[test]
    fn updating_a_missing_row_fails() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = active_library(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        let error = record(
            &tx,
            &lib,
            "member.rename",
            vec![Change::update(
                Entity::Member,
                new_id(),
                json!({ "name": "B" }),
            )],
        )
        .unwrap_err();
        assert!(matches!(error, Error::MissingRow { .. }));
    }
}
