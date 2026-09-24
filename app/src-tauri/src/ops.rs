//! The only way to write catalog data: record an operation and apply its changes
//! in the same transaction (D05, D25).

use std::time::{SystemTime, UNIX_EPOCH};

use library_domain::{Action, Change, Hlc, HlcClock, Operation};
use rusqlite::types::Value as SqlValue;
use rusqlite::{Connection, OptionalExtension, Transaction, params, params_from_iter};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{Error, Result};

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
    let device_id = read_state(tx, DEVICE_ID_KEY)?.ok_or(Error::MissingDeviceId)?;
    let last = read_state(tx, LAST_HLC_KEY)?
        .map(|text| text.parse::<Hlc>())
        .transpose()
        .map_err(|_| Error::CorruptState(LAST_HLC_KEY))?;
    let hlc = HlcClock::new(device_id.clone(), last.as_ref()).tick(now_millis());

    let operation = Operation {
        id: new_id(),
        hlc,
        intent: intent.to_owned(),
        changes,
    };
    operation.validate()?;

    for change in &operation.changes {
        apply(tx, change)?;
    }
    tx.execute(
        "INSERT INTO operations (id, hlc, device_id, intent, changes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            operation.id,
            operation.hlc.to_string(),
            device_id,
            operation.intent,
            serde_json::to_string(&operation.changes)?,
        ],
    )?;
    tx.execute(
        "INSERT OR REPLACE INTO local_state (key, value) VALUES (?1, ?2)",
        params![LAST_HLC_KEY, operation.hlc.to_string()],
    )?;
    Ok(operation)
}

/// Table and column names come from the domain schema, never from the change
/// itself: `validate` has already rejected any field the schema does not know.
fn apply(tx: &Transaction<'_>, change: &Change) -> Result<()> {
    let entity = change.entity;
    let table = entity.table();
    let id_column = entity.id_column();
    let (columns, values): (Vec<&str>, Vec<SqlValue>) = change
        .fields
        .iter()
        .filter_map(|(name, value)| entity.field(name).map(|spec| (spec.name, to_sql(value))))
        .unzip();

    let affected = match change.action {
        Action::Create => {
            let placeholders = vec!["?"; columns.len() + 1].join(", ");
            let sql = format!(
                "INSERT INTO {table} ({id_column}{}{}) VALUES ({placeholders})",
                if columns.is_empty() { "" } else { ", " },
                columns.join(", "),
            );
            let args = std::iter::once(SqlValue::Text(change.id.clone())).chain(values);
            tx.execute(&sql, params_from_iter(args))?
        }
        Action::Update => {
            let assignments: Vec<String> = columns.iter().map(|c| format!("{c} = ?")).collect();
            let sql = format!(
                "UPDATE {table} SET {} WHERE {id_column} = ?",
                assignments.join(", ")
            );
            let args = values
                .into_iter()
                .chain(std::iter::once(SqlValue::Text(change.id.clone())));
            tx.execute(&sql, params_from_iter(args))?
        }
        Action::Delete => tx.execute(
            &format!("DELETE FROM {table} WHERE {id_column} = ?1"),
            [&change.id],
        )?,
    };

    if affected != 1 {
        return Err(Error::MissingRow {
            entity: table,
            id: change.id.clone(),
        });
    }
    Ok(())
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
