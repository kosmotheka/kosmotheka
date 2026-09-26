//! The only way to write catalog data: record an operation and apply its changes
//! in the same transaction (D05, D25).

use std::time::{SystemTime, UNIX_EPOCH};

use library_domain::{Action, Change, Entity, Hlc, HlcClock, Operation};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::search;

const DEVICE_ID_KEY: &str = "device_id";

/// The library that holds what existed before libraries did. The same id on every
/// device, so devices that already synced merge without duplicates (D46).
pub const DEFAULT_LIBRARY_ID: &str = "00000000-0000-7000-8000-000000000001";
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

fn record_operation(
    tx: &Transaction<'_>,
    library: &str,
    intent: &str,
    changes: Vec<Change>,
    apply_changes: bool,
) -> Result<Operation> {
    let device_id = device_id(tx)?;
    let hlc = HlcClock::new(device_id.clone(), last_hlc(tx)?.as_ref()).tick(now_millis());

    let operation = Operation {
        id: new_id(),
        library: library.to_owned(),
        hlc,
        intent: intent.to_owned(),
        changes,
    };
    operation.validate()?;
    let hlc_text = operation.hlc.to_string();

    // Search text can change for rows that vanish (known before) or appear (known after).
    let mut affected = std::collections::BTreeSet::new();
    for change in &operation.changes {
        affected.extend(search::affected_editions(tx, library, change)?);
    }
    for change in &operation.changes {
        if apply_changes {
            apply(tx, library, change)?;
        }
        write_versions(tx, library, change, &hlc_text)?;
    }
    for change in &operation.changes {
        affected.extend(search::affected_editions(tx, library, change)?);
    }
    search::reindex(tx, library, &affected)?;
    log_operation(tx, &operation, schema_version(tx)?)?;
    write_state(tx, LAST_HLC_KEY, Some(&hlc_text))?;
    Ok(operation)
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

/// The library the user is working in. Until libraries can be created (milestone
/// 4.5) it is always the one migrated from before libraries existed.
pub fn active_library(conn: &Connection) -> Result<String> {
    Ok(read_state(conn, ACTIVE_LIBRARY_KEY)?.unwrap_or_else(|| DEFAULT_LIBRARY_ID.to_owned()))
}

/// Appends to the log. The author device is the one inside the HLC.
pub fn log_operation(conn: &Connection, operation: &Operation, schema_version: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO operations (id, library_id, hlc, device_id, intent, changes, schema_version, replicable)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![
            operation.id,
            operation.library,
            operation.hlc.to_string(),
            operation.hlc.device,
            operation.intent,
            serde_json::to_string(&operation.changes)?,
            schema_version,
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
    delete_row(conn, Entity::Library, library, library)
}

/// Every entity inside a library, children before parents.
const LIBRARY_CONTENTS_CHILDREN_FIRST: [Entity; 9] = [
    Entity::Copy,
    Entity::BookDetails,
    Entity::Edition,
    Entity::WorkContributor,
    Entity::Work,
    Entity::Person,
    Entity::Publisher,
    Entity::Member,
    Entity::Wish,
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
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use library_domain::{Entity, OperationErrorCode};
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
        let member_id = new_id();
        let tx = conn.transaction().unwrap();

        let op = record(
            &tx,
            DEFAULT_LIBRARY_ID,
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

    #[test]
    fn each_operation_gets_a_later_hlc() {
        let mut conn = db::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let first = record(
            &tx,
            DEFAULT_LIBRARY_ID,
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
            DEFAULT_LIBRARY_ID,
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
        let id = new_id();
        let tx = conn.transaction().unwrap();
        record(
            &tx,
            DEFAULT_LIBRARY_ID,
            "member.add",
            vec![Change::create(Entity::Member, &id, json!({ "name": "A" }))],
        )
        .unwrap();
        record(
            &tx,
            DEFAULT_LIBRARY_ID,
            "member.rename",
            vec![Change::update(Entity::Member, &id, json!({ "name": "B" }))],
        )
        .unwrap();
        let renamed: String = tx
            .query_row("SELECT name FROM members", [], |r| r.get(0))
            .unwrap();
        record(
            &tx,
            DEFAULT_LIBRARY_ID,
            "member.remove",
            vec![Change::delete(Entity::Member, &id)],
        )
        .unwrap();

        assert_eq!(renamed, "B");
        assert_eq!(count(&tx, "members"), 0);
        assert_eq!(count(&tx, "operations"), 3);
    }

    #[test]
    fn invalid_operation_writes_nothing() {
        let mut conn = db::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();

        let error = record(
            &tx,
            DEFAULT_LIBRARY_ID,
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
        assert_eq!(count(&tx, "operations"), 0);
    }

    #[test]
    fn updating_a_missing_row_fails() {
        let mut conn = db::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();
        let error = record(
            &tx,
            DEFAULT_LIBRARY_ID,
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
