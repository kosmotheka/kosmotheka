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
const LAST_HLC_KEY: &str = "last_hlc";

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

pub fn record(tx: &Transaction<'_>, intent: &str, changes: Vec<Change>) -> Result<Operation> {
    record_operation(tx, intent, changes, true)
}

/// Logs changes that are already in the tables (the sync baseline, D31): versions
/// and log are written, the rows themselves are left as they are.
pub fn record_existing(
    tx: &Transaction<'_>,
    intent: &str,
    changes: Vec<Change>,
) -> Result<Operation> {
    record_operation(tx, intent, changes, false)
}

fn record_operation(
    tx: &Transaction<'_>,
    intent: &str,
    changes: Vec<Change>,
    apply_changes: bool,
) -> Result<Operation> {
    let device_id = device_id(tx)?;
    let hlc = HlcClock::new(device_id.clone(), last_hlc(tx)?.as_ref()).tick(now_millis());

    let operation = Operation {
        id: new_id(),
        hlc,
        intent: intent.to_owned(),
        changes,
    };
    operation.validate()?;
    let hlc_text = operation.hlc.to_string();

    // Search text can change for rows that vanish (known before) or appear (known after).
    let mut affected = std::collections::BTreeSet::new();
    for change in &operation.changes {
        affected.extend(search::affected_editions(tx, change)?);
    }
    for change in &operation.changes {
        if apply_changes {
            apply(tx, change)?;
        }
        write_versions(tx, change, &hlc_text)?;
    }
    for change in &operation.changes {
        affected.extend(search::affected_editions(tx, change)?);
    }
    search::reindex(tx, &affected)?;
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

/// Appends to the log. The author device is the one inside the HLC.
pub fn log_operation(conn: &Connection, operation: &Operation, schema_version: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO operations (id, hlc, device_id, intent, changes, schema_version, replicable)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        params![
            operation.id,
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
pub fn write_versions(conn: &Connection, change: &Change, hlc: &str) -> Result<()> {
    let entity = change.entity.table();
    match change.action {
        Action::Create | Action::Update => {
            for field in change.fields.keys() {
                conn.execute(
                    "INSERT OR REPLACE INTO field_versions (entity, entity_id, field, hlc)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![entity, change.id, field, hlc],
                )?;
            }
        }
        Action::Delete => {
            conn.execute(
                "DELETE FROM field_versions WHERE entity = ?1 AND entity_id = ?2",
                params![entity, change.id],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO tombstones (entity, entity_id, hlc) VALUES (?1, ?2, ?3)",
                params![entity, change.id, hlc],
            )?;
        }
    }
    Ok(())
}

/// Table and column names come from the domain schema, never from the change
/// itself: `validate` has already rejected any field the schema does not know.
fn apply(tx: &Transaction<'_>, change: &Change) -> Result<()> {
    let affected = match change.action {
        Action::Create => insert_row(tx, change.entity, &change.id, change.fields.iter())?,
        Action::Update => update_row(tx, change.entity, &change.id, change.fields.iter())?,
        Action::Delete => delete_row(tx, change.entity, &change.id)?,
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
    id: &str,
    fields: impl Iterator<Item = (&'a String, &'a Value)>,
) -> Result<usize> {
    let (columns, values) = known_columns(entity, fields);
    let placeholders = vec!["?"; columns.len() + 1].join(", ");
    let sql = format!(
        "INSERT INTO {} ({}{}{}) VALUES ({placeholders})",
        entity.table(),
        entity.id_column(),
        if columns.is_empty() { "" } else { ", " },
        columns.join(", "),
    );
    let args = std::iter::once(SqlValue::Text(id.to_owned())).chain(values);
    Ok(conn.execute(&sql, params_from_iter(args))?)
}

pub fn update_row<'a>(
    conn: &Connection,
    entity: Entity,
    id: &str,
    fields: impl Iterator<Item = (&'a String, &'a Value)>,
) -> Result<usize> {
    let (columns, values) = known_columns(entity, fields);
    if columns.is_empty() {
        return Ok(0);
    }
    let assignments: Vec<String> = columns.iter().map(|c| format!("{c} = ?")).collect();
    let sql = format!(
        "UPDATE {} SET {} WHERE {} = ?",
        entity.table(),
        assignments.join(", "),
        entity.id_column()
    );
    let args = values
        .into_iter()
        .chain(std::iter::once(SqlValue::Text(id.to_owned())));
    Ok(conn.execute(&sql, params_from_iter(args))?)
}

pub fn delete_row(conn: &Connection, entity: Entity, id: &str) -> Result<usize> {
    Ok(conn.execute(
        &format!(
            "DELETE FROM {} WHERE {} = ?1",
            entity.table(),
            entity.id_column()
        ),
        [id],
    )?)
}

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
            "member.add",
            vec![Change::create(Entity::Member, &id, json!({ "name": "A" }))],
        )
        .unwrap();
        record(
            &tx,
            "member.rename",
            vec![Change::update(Entity::Member, &id, json!({ "name": "B" }))],
        )
        .unwrap();
        let renamed: String = tx
            .query_row("SELECT name FROM members", [], |r| r.get(0))
            .unwrap();
        record(
            &tx,
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
