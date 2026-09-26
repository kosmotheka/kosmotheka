//! The libraries on this device (D46, D54, D65): list them, create, rename and
//! switch between them, delete one everywhere or leave it on this device only, and
//! withdraw or give back a device's access to one (D71).

use kosmotheka_domain::{Change, Entity, ErrorCode, FieldError, Hlc, validate_library_name};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::json;

use crate::error::{Error, Result};
use crate::lan;
use crate::ops::{self, new_id};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    pub id: String,
    /// Empty for a library never named: the UI shows its own default name.
    pub name: String,
    /// Editions, as the catalog counts them.
    pub works: i64,
    pub active: bool,
    pub created: Option<Creation>,
}

/// Who created a library and when, read from the operation that created it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Creation {
    /// Name of the device, if this device knows it.
    pub device: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub at: u64,
}

pub fn list_libraries(conn: &Connection) -> Result<Vec<LibrarySummary>> {
    let active = ops::active_library(conn)?;
    let mut statement = conn.prepare(
        "SELECT l.id, l.name, (SELECT count(*) FROM editions e WHERE e.library_id = l.id)
           FROM libraries l ORDER BY l.name COLLATE NOCASE, l.id",
    )?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?))
        })?
        .collect::<rusqlite::Result<Vec<(String, String, i64)>>>()?;
    rows.into_iter()
        .map(|(id, name, works)| {
            Ok(LibrarySummary {
                created: creation(conn, &id)?,
                active: id == active,
                id,
                name,
                works,
            })
        })
        .collect()
}

fn creation(conn: &Connection, library: &str) -> Result<Option<Creation>> {
    let found: Option<(String, String)> = conn
        .query_row(
            "SELECT hlc, device_id FROM operations
              WHERE library_id = ?1 AND intent = 'library.create' ORDER BY hlc LIMIT 1",
            [library],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((hlc, device_id)) = found else {
        return Ok(None);
    };
    let device = if device_id == ops::device_id(conn)? {
        Some(lan::device_name(conn)?)
    } else {
        conn.query_row(
            "SELECT name FROM known_servers WHERE device_id = ?1
              UNION SELECT name FROM known_clients WHERE device_id = ?1",
            [&device_id],
            |row| row.get(0),
        )
        .optional()?
    };
    Ok(Some(Creation {
        device,
        at: hlc.parse::<Hlc>().map_or(0, |hlc| hlc.millis),
    }))
}

/// Two libraries on one device may not share a name (Design, 26/09/2026). Between
/// devices it cannot be guaranteed, so this only guards what the user types here.
fn check_name(conn: &Connection, raw: &str, except: Option<&str>) -> Result<String> {
    let name = validate_library_name(raw).map_err(Error::Validation)?;
    let taken: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM libraries WHERE name = ?1 COLLATE NOCASE AND id <> ?2)",
        params![name, except.unwrap_or("")],
        |row| row.get(0),
    )?;
    if taken {
        return Err(Error::Validation(vec![FieldError::new(
            "name",
            ErrorCode::NameInUse,
        )]));
    }
    Ok(name)
}

/// Creates a library and makes it the active one. Returns its id.
pub fn create_library(conn: &mut Connection, raw_name: &str) -> Result<String> {
    let name = check_name(conn, raw_name, None)?;
    let tx = conn.transaction()?;
    let id = create_in(&tx, &name)?;
    ops::set_active_library(&tx, &id)?;
    tx.commit()?;
    Ok(id)
}

fn create_in(tx: &Transaction<'_>, name: &str) -> Result<String> {
    create_with_id(tx, &new_id(), name)
}

/// Creates a library with a known id: a restored one keeps the id it had (D46).
pub fn create_with_id(tx: &Transaction<'_>, id: &str, name: &str) -> Result<String> {
    ops::record(
        tx,
        id,
        "library.create",
        vec![Change::create(Entity::Library, id, json!({ "name": name }))],
    )?;
    Ok(id.to_owned())
}

/// Renames inside a transaction, recording nothing if the name is already that one.
pub fn rename_in(tx: &Transaction<'_>, id: &str, name: &str) -> Result<()> {
    let current: String =
        tx.query_row("SELECT name FROM libraries WHERE id = ?1", [id], |row| {
            row.get(0)
        })?;
    if current != name {
        ops::record(
            tx,
            id,
            "library.rename",
            vec![Change::update(Entity::Library, id, json!({ "name": name }))],
        )?;
    }
    Ok(())
}

/// `wanted`, or `wanted (2)`, `wanted (3)`… if another library here has that name.
/// An unnamed library keeps no name: the app shows its default one.
pub fn unused_name(conn: &Connection, wanted: &str, except: Option<&str>) -> Result<String> {
    if wanted.is_empty() {
        return Ok(String::new());
    }
    let taken = |name: &str| -> Result<bool> {
        Ok(conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM libraries WHERE name = ?1 COLLATE NOCASE AND id <> ?2)",
            params![name, except.unwrap_or("")],
            |row| row.get(0),
        )?)
    };
    let mut name = wanted.to_owned();
    let mut n = 2;
    while taken(&name)? {
        name = format!("{wanted} ({n})");
        n += 1;
    }
    Ok(name)
}

pub fn rename_library(conn: &mut Connection, id: &str, raw_name: &str) -> Result<()> {
    ensure_exists(conn, id)?;
    let name = check_name(conn, raw_name, Some(id))?;
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        id,
        "library.rename",
        vec![Change::update(Entity::Library, id, json!({ "name": name }))],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn set_active_library(conn: &Connection, id: &str) -> Result<()> {
    ensure_exists(conn, id)?;
    ops::set_active_library(conn, id)
}

/// Deletes the library everywhere it is shared (D65): an operation that travels
/// like any other, and is kept on record (D66). Returns the library now active.
pub fn delete_library(conn: &mut Connection, id: &str) -> Result<String> {
    ensure_exists(conn, id)?;
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        id,
        "library.delete",
        vec![Change::delete(Entity::Library, id)],
    )?;
    let active = fall_back_from(&tx, id)?;
    tx.commit()?;
    Ok(active)
}

/// Leaves the library on this device only: its rows and its log go, the other
/// devices keep it, and this device stops syncing it. Not an operation, since
/// nothing changes for anyone else. Returns the library now active.
pub fn leave_library(conn: &mut Connection, id: &str) -> Result<String> {
    ensure_exists(conn, id)?;
    if lan::devices_sharing(conn, id)?.is_empty() {
        return Err(Error::OnlyOnThisDevice);
    }
    let tx = conn.transaction()?;
    let active = remove_here(&tx, id, None)?;
    tx.commit()?;
    Ok(active)
}

/// A peer withdrew this device's access (D71): the library goes from here, as if
/// leaving it. Returns the library now active.
pub fn remove_revoked(conn: &mut Connection, id: &str, by: &str) -> Result<String> {
    let tx = conn.transaction()?;
    let active = remove_here(&tx, id, Some(by))?;
    tx.commit()?;
    Ok(active)
}

/// Removes a library from this device only and tells its other peers, on the next
/// sync, to stop offering it. `told` already knows.
fn remove_here(tx: &Transaction<'_>, id: &str, told: Option<&str>) -> Result<String> {
    tx.execute(
        "INSERT OR IGNORE INTO leave_notices (library_id, device_id)
         SELECT library_id, device_id FROM library_peers
          WHERE library_id = ?1 AND device_id IS NOT ?2",
        params![id, told],
    )?;
    ops::delete_library(tx, id)?;
    tx.execute("DELETE FROM operations WHERE library_id = ?1", [id])?;
    tx.execute("DELETE FROM library_peers WHERE library_id = ?1", [id])?;
    fall_back_from(tx, id)
}

/// Withdraws a paired device's access to a library (D71). The revocation is an
/// operation, so every device that has the library stops sharing it with that one,
/// and the device itself removes it the next time it syncs with any of them.
pub fn revoke_access(conn: &mut Connection, id: &str, device_id: &str) -> Result<()> {
    ensure_exists(conn, id)?;
    let name = lan::known_device_name(conn, device_id)?.unwrap_or_else(|| device_id.to_owned());
    let tx = conn.transaction()?;
    if lan::revocations_of(&tx, id, device_id)?.is_empty() {
        ops::record(
            &tx,
            id,
            "library.revoke",
            vec![Change::create(
                Entity::Revocation,
                new_id(),
                json!({ "device_id": device_id, "device_name": name }),
            )],
        )?;
    }
    tx.execute(
        "DELETE FROM library_peers WHERE library_id = ?1 AND device_id = ?2",
        [id, device_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// Shares a library with a paired device, lifting any earlier revocation.
pub fn grant_access(conn: &mut Connection, id: &str, device_id: &str) -> Result<()> {
    ensure_exists(conn, id)?;
    let tx = conn.transaction()?;
    let revocations = lan::revocations_of(&tx, id, device_id)?;
    if !revocations.is_empty() {
        ops::record(
            &tx,
            id,
            "library.grant",
            revocations
                .iter()
                .map(|revocation| Change::delete(Entity::Revocation, revocation))
                .collect(),
        )?;
    }
    lan::share_library(&tx, id, device_id)?;
    tx.execute(
        "DELETE FROM leave_notices WHERE library_id = ?1 AND device_id = ?2",
        [id, device_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// After `gone` disappears, the active library is another one; with none left, a
/// new unnamed one, so the app always has somewhere to work.
fn fall_back_from(tx: &Transaction<'_>, gone: &str) -> Result<String> {
    let active = ops::active_library(tx)?;
    if active != gone && library_exists(tx, &active)? {
        return Ok(active);
    }
    let other: Option<String> = tx
        .query_row(
            "SELECT id FROM libraries WHERE id <> ?1 ORDER BY name COLLATE NOCASE, id LIMIT 1",
            [gone],
            |row| row.get(0),
        )
        .optional()?;
    let next = match other {
        Some(id) => id,
        None => create_in(tx, "")?,
    };
    ops::set_active_library(tx, &next)?;
    Ok(next)
}

pub fn library_exists(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM libraries WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )?)
}

fn ensure_exists(conn: &Connection, id: &str) -> Result<()> {
    if library_exists(conn, id)? {
        Ok(())
    } else {
        Err(Error::MissingRow {
            entity: Entity::Library.table(),
            id: id.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::ops::DEFAULT_LIBRARY_ID;

    fn book(title: &str) -> ValidBook {
        ValidBook {
            edition: EditionInput {
                title: title.into(),
                authors: vec!["Ursula K. Le Guin".into()],
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: Some("Ana".into()),
                format: Format::Paperback,
                quantity: 2,
                location: Some("Salón".into()),
                acquired_on: None,
            }
            .validate()
            .unwrap(),
        }
    }

    fn titles(conn: &Connection, library: &str) -> Vec<String> {
        catalog::list_books(conn, library, &BookFilter::default())
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect()
    }

    fn name_error(result: Result<impl std::fmt::Debug>) -> ErrorCode {
        match result {
            Err(Error::Validation(errors)) => errors[0].code,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    #[test]
    fn a_new_library_becomes_active_and_names_stay_unique() {
        let mut conn = db::open_in_memory().unwrap();
        let rol = create_library(&mut conn, " Rol ").unwrap();

        assert_eq!(ops::active_library(&conn).unwrap(), rol);
        let listed = list_libraries(&conn).unwrap();
        let created = listed.iter().find(|l| l.id == rol).unwrap();
        assert_eq!(created.name, "Rol");
        assert!(created.active);
        assert!(created.created.as_ref().unwrap().device.is_some());
        assert_eq!(
            name_error(create_library(&mut conn, "rol")),
            ErrorCode::NameInUse
        );
        assert_eq!(
            name_error(create_library(&mut conn, "  ")),
            ErrorCode::Required
        );

        let familia = create_library(&mut conn, "Familia").unwrap();
        assert_eq!(
            name_error(rename_library(&mut conn, &familia, "ROL")),
            ErrorCode::NameInUse
        );
        rename_library(&mut conn, &rol, "Rol").unwrap();
        rename_library(&mut conn, &rol, "Rol de los jueves").unwrap();
    }

    #[test]
    fn deleting_the_active_library_falls_back_and_keeps_the_record() {
        let mut conn = db::open_in_memory().unwrap();
        let rol = create_library(&mut conn, "Rol").unwrap();
        catalog::create_book(&mut conn, &rol, &book("Tehanu"), None).unwrap();

        let active = delete_library(&mut conn, &rol).unwrap();

        assert_eq!(active, DEFAULT_LIBRARY_ID);
        assert!(titles(&conn, &rol).is_empty());
        assert!(!list_libraries(&conn).unwrap().iter().any(|l| l.id == rol));
        let kept: i64 = conn
            .query_row(
                "SELECT count(*) FROM operations WHERE library_id = ?1 AND intent = 'library.delete'",
                [&rol],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kept, 1, "the delete stays on record (D66)");
    }

    #[test]
    fn a_library_only_on_this_device_cannot_be_left() {
        let mut conn = db::open_in_memory().unwrap();
        catalog::create_book(&mut conn, DEFAULT_LIBRARY_ID, &book("Tehanu"), None).unwrap();

        let refused = leave_library(&mut conn, DEFAULT_LIBRARY_ID);

        assert!(matches!(refused, Err(Error::OnlyOnThisDevice)));
        assert_eq!(titles(&conn, DEFAULT_LIBRARY_ID), vec!["Tehanu"]);
    }

    #[test]
    fn leaving_the_last_library_leaves_an_empty_one_to_work_in() {
        let mut conn = db::open_in_memory().unwrap();
        catalog::create_book(&mut conn, DEFAULT_LIBRARY_ID, &book("Tehanu"), None).unwrap();
        lan::share_library(&conn, DEFAULT_LIBRARY_ID, "phone").unwrap();
        crate::lan::save_client(&conn, "phone", "Phone", "secret").unwrap();

        let active = leave_library(&mut conn, DEFAULT_LIBRARY_ID).unwrap();

        assert_ne!(active, DEFAULT_LIBRARY_ID);
        let listed = list_libraries(&conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "");
        let left: i64 = conn
            .query_row(
                "SELECT (SELECT count(*) FROM operations WHERE library_id = ?1)
                      + (SELECT count(*) FROM library_peers WHERE library_id = ?1)",
                [DEFAULT_LIBRARY_ID],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(left, 0, "nothing of it remains on this device");
    }

    #[test]
    fn a_moved_book_lives_in_its_new_library_with_new_ids() {
        let mut conn = db::open_in_memory().unwrap();
        let old_id = catalog::create_book(&mut conn, DEFAULT_LIBRARY_ID, &book("Tehanu"), None)
            .unwrap()
            .edition_id;
        let rol = create_library(&mut conn, "Rol").unwrap();

        let new_id = catalog::move_book(&mut conn, DEFAULT_LIBRARY_ID, &old_id, &rol).unwrap();

        assert_ne!(new_id, old_id);
        assert!(titles(&conn, DEFAULT_LIBRARY_ID).is_empty());
        assert_eq!(titles(&conn, &rol), vec!["Tehanu"]);
        let moved = catalog::get_book(&conn, &rol, &new_id).unwrap();
        assert_eq!(moved.authors, vec!["Ursula K. Le Guin"]);
        assert_eq!(moved.copies[0].quantity, 2);
        assert_eq!(moved.copies[0].owner.as_deref(), Some("Ana"));
        let history = catalog::book_history(&conn, DEFAULT_LIBRARY_ID, &old_id).unwrap();
        assert_eq!(history.last().unwrap().intent, "book.move_out");
    }
}
