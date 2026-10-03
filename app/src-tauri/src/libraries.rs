//! The libraries on this device (D46, D54, D65): list them, create, rename and
//! switch between them, delete one everywhere or leave it on this device only, and
//! withdraw or give back a device's access to one (D71).

use kosmotheka_domain::{Change, Entity, ErrorCode, FieldError, Hlc, validate_library_name};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use serde_json::json;

use crate::backup;
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
    /// This device may take it to other devices (D73).
    pub can_share: bool,
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
                can_share: crate::people::may_share(conn, &id)?,
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

/// The id every installation gave its first library before D85. It survives only to
/// find that library on update and replace it.
const LEGACY_FIXED_LIBRARY_ID: &str = "00000000-0000-7000-8000-000000000001";

/// The library the app opens in when none is chosen: the first by name, then id.
pub fn first_library(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT id FROM libraries ORDER BY name COLLATE NOCASE, id LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

/// Once, on update (D85): the library with the old fixed id goes. Empty, it is just
/// deleted; with data, its catalogue is copied into a new library with a random id
/// (D58) and the old one is removed from this device, as if leaving it: nothing
/// changes for anyone else, so no operation. Does nothing when it is not there.
pub fn retire_fixed_library(conn: &mut Connection) -> Result<()> {
    let legacy = LEGACY_FIXED_LIBRARY_ID;
    let name: Option<String> = conn
        .query_row(
            "SELECT name FROM libraries WHERE id = ?1",
            [legacy],
            |row| row.get(0),
        )
        .optional()?;
    let Some(name) = name else {
        return Ok(());
    };
    let tx = conn.transaction()?;
    if is_untouched(&tx, legacy)? {
        for table in ["libraries", "field_versions", "tombstones"] {
            let column = if table == "libraries" {
                "id"
            } else {
                "library_id"
            };
            tx.execute(
                &format!("DELETE FROM {table} WHERE {column} = ?1"),
                [legacy],
            )?;
        }
        if ops::read_state(&tx, "active_library")?.as_deref() == Some(legacy) {
            ops::forget_active_library(&tx)?;
        }
    } else {
        // An unnamed library stays unnamed: the app shows its default name, in the
        // user's language.
        let rows = backup::read_library(&tx, legacy)?;
        forget_here(&tx, legacy, None)?;
        backup::copy_as_new(&tx, &rows, &name)?;
    }
    tx.commit()?;
    Ok(())
}

/// Nothing in it, and nothing about it anywhere on this device.
fn is_untouched(conn: &Connection, library: &str) -> Result<bool> {
    let exists = |sql: &str| -> Result<bool> { Ok(conn.query_row(sql, [library], |r| r.get(0))?) };
    for entity in ops::LIBRARY_CONTENTS_CHILDREN_FIRST {
        let sql = format!(
            "SELECT EXISTS (SELECT 1 FROM {} WHERE {} = ?1)",
            entity.table(),
            entity.library_column()
        );
        if exists(&sql)? {
            return Ok(false);
        }
    }
    for table in [
        "operations",
        "library_peers",
        "leave_notices",
        "revocations",
        "pending_owners",
        "request_notices",
    ] {
        if exists(&format!(
            "SELECT EXISTS (SELECT 1 FROM {table} WHERE library_id = ?1)"
        ))? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Every device has a library to work in from the first start (D85): a random id,
/// created like any other, so it travels like any other.
pub fn ensure_first_library(conn: &mut Connection) -> Result<()> {
    if first_library(conn)?.is_none() {
        let tx = conn.transaction()?;
        let id = create_in(&tx, "")?;
        ops::set_active_library(&tx, &id)?;
        tx.commit()?;
    }
    Ok(())
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
    forget_here(tx, id, told)?;
    fall_back_from(tx, id)
}

/// The removal itself, without choosing another active library.
fn forget_here(tx: &Transaction<'_>, id: &str, told: Option<&str>) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO leave_notices (library_id, device_id)
         SELECT library_id, device_id FROM library_peers
          WHERE library_id = ?1 AND device_id IS NOT ?2",
        params![id, told],
    )?;
    // So the cloud does not hand it back (SEG-017).
    tx.execute(
        "INSERT OR IGNORE INTO forgotten_libraries (library_id) VALUES (?1)",
        [id],
    )?;
    ops::delete_library(tx, id)?;
    tx.execute("DELETE FROM operations WHERE library_id = ?1", [id])?;
    tx.execute("DELETE FROM library_peers WHERE library_id = ?1", [id])?;
    Ok(())
}

/// Withdraws a paired device's access to a library (D71). The revocation is an
/// operation, so every device that has the library stops sharing it with that one,
/// and the device itself removes it the next time it syncs with any of them.
pub fn revoke_access(conn: &mut Connection, id: &str, device_id: &str) -> Result<()> {
    ensure_exists(conn, id)?;
    let name = lan::known_device_name(conn, device_id)?.unwrap_or_else(|| device_id.to_owned());
    let tx = conn.transaction()?;
    if lan::revocations_of(&tx, id, device_id)?.is_empty() {
        let mut changes = vec![Change::create(
            Entity::Revocation,
            new_id(),
            json!({ "device_id": device_id, "device_name": name }),
        )];
        // The revoked device is fenced in the same operation (D84).
        changes.extend(crate::permissions::fence_changes(
            &tx,
            id,
            &[device_id.to_owned()],
        )?);
        ops::record(&tx, id, "library.revoke", changes)?;
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
    crate::people::ensure_may_share(conn, &[id.to_owned()])?;
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
    if let Some(active) = ops::current_library(tx)?
        && active != gone
    {
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

/// Whether this device left the library or was cut off from it (SEG-017).
pub fn library_forgotten(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM forgotten_libraries WHERE library_id = ?1)",
        [id],
        |row| row.get(0),
    )?)
}

/// Whether this device has or had the library: its row, or anything in its log.
/// Deleting a library keeps its log, delete included (D65, D66).
pub fn library_known(conn: &Connection, id: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM libraries WHERE id = ?1)
             OR EXISTS (SELECT 1 FROM operations WHERE library_id = ?1)",
        [id],
        |row| row.get(0),
    )?)
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
                platform: None,
                seasons: None,
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
        let lib = ops::active_library(&conn).unwrap();
        let rol = create_library(&mut conn, "Rol").unwrap();
        catalog::create_book(&mut conn, &rol, &book("Tehanu"), None).unwrap();

        let active = delete_library(&mut conn, &rol).unwrap();

        assert_eq!(active, lib);
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
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("Tehanu"), None).unwrap();

        let refused = leave_library(&mut conn, &lib);

        assert!(matches!(refused, Err(Error::OnlyOnThisDevice)));
        assert_eq!(titles(&conn, &lib), vec!["Tehanu"]);
    }

    #[test]
    fn leaving_the_last_library_leaves_an_empty_one_to_work_in() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("Tehanu"), None).unwrap();
        lan::share_library(&conn, &lib, "phone").unwrap();
        crate::lan::save_client(&conn, "phone", "Phone", "secret").unwrap();

        let active = leave_library(&mut conn, &lib).unwrap();

        assert_ne!(active, lib);
        let listed = list_libraries(&conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "");
        let left: i64 = conn
            .query_row(
                "SELECT (SELECT count(*) FROM operations WHERE library_id = ?1)
                      + (SELECT count(*) FROM library_peers WHERE library_id = ?1)",
                [&lib],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(left, 0, "nothing of it remains on this device");
    }

    #[test]
    fn a_moved_book_lives_in_its_new_library_with_new_ids() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let old_id = catalog::create_book(&mut conn, &lib, &book("Tehanu"), None)
            .unwrap()
            .edition_id;
        let rol = create_library(&mut conn, "Rol").unwrap();

        let new_id = catalog::move_book(&mut conn, &lib, &old_id, &rol).unwrap();

        assert_ne!(new_id, old_id);
        assert!(titles(&conn, &lib).is_empty());
        assert_eq!(titles(&conn, &rol), vec!["Tehanu"]);
        let moved = catalog::get_book(&conn, &rol, &new_id).unwrap();
        assert_eq!(moved.authors, vec!["Ursula K. Le Guin"]);
        assert_eq!(moved.copies[0].quantity, 2);
        assert_eq!(moved.copies[0].owner.as_deref(), Some("Ana"));
        let history = catalog::book_history(&conn, &lib, &old_id).unwrap();
        assert_eq!(history.last().unwrap().intent, "book.move_out");
    }

    // --- The first library (D85) ---------------------------------------------

    /// What an installation looked like after migration 0009 and before D85: one
    /// library with the fixed id, no log, and no random one.
    fn upgraded_database() -> Connection {
        let conn = db::open_in_memory().unwrap();
        let random = ops::active_library(&conn).unwrap();
        ops::delete_library(&conn, &random).unwrap();
        conn.execute("DELETE FROM operations", []).unwrap();
        ops::forget_active_library(&conn).unwrap();
        conn.execute(
            "INSERT INTO libraries (id, name) VALUES (?1, '')",
            [LEGACY_FIXED_LIBRARY_ID],
        )
        .unwrap();
        conn
    }

    fn library_ids(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT id FROM libraries ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    fn operations_of(conn: &Connection, library: &str) -> i64 {
        conn.query_row(
            "SELECT count(*) FROM operations WHERE library_id = ?1",
            [library],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn a_fresh_database_has_one_library_with_a_random_id_and_a_signed_create() {
        let conn = db::open_in_memory().unwrap();

        let ids = library_ids(&conn);
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], LEGACY_FIXED_LIBRARY_ID);
        assert_eq!(ops::active_library(&conn).unwrap(), ids[0]);
        let signed: i64 = conn
            .query_row(
                "SELECT count(*) FROM operations
                  WHERE library_id = ?1 AND intent = 'library.create' AND signature IS NOT NULL",
                [&ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(signed, 1);
        assert_eq!(operations_of(&conn, LEGACY_FIXED_LIBRARY_ID), 0);
    }

    #[test]
    fn an_empty_legacy_library_is_replaced_by_a_fresh_random_one() {
        let mut conn = upgraded_database();
        ops::set_active_library(&conn, LEGACY_FIXED_LIBRARY_ID).unwrap();

        retire_fixed_library(&mut conn).unwrap();
        ensure_first_library(&mut conn).unwrap();

        let ids = library_ids(&conn);
        assert_eq!(ids.len(), 1);
        assert_ne!(ids[0], LEGACY_FIXED_LIBRARY_ID);
        assert_eq!(ops::active_library(&conn).unwrap(), ids[0]);
        assert!(operations_of(&conn, &ids[0]) > 0);
    }

    #[test]
    fn a_legacy_library_with_data_moves_to_a_random_one_with_the_same_books_and_mark() {
        let mut conn = upgraded_database();
        let legacy = LEGACY_FIXED_LIBRARY_ID;
        catalog::create_book(&mut conn, legacy, &book("Tehanu"), None).unwrap();
        catalog::create_book(&mut conn, legacy, &book("Dune"), None).unwrap();
        crate::marks::claim_person(
            &mut conn,
            legacy,
            &crate::people::PersonPick {
                id: None,
                name: "Ana".into(),
            },
        )
        .unwrap();
        let edition = catalog::list_books(&conn, legacy, &BookFilter::default()).unwrap()[0]
            .edition_id
            .clone();
        let work = catalog::get_book(&conn, legacy, &edition).unwrap().work_id;
        crate::marks::set_mark(
            &mut conn,
            legacy,
            &work,
            &crate::marks::Mark {
                status: Some("finished".into()),
                ..Default::default()
            },
        )
        .unwrap();
        lan::share_library(&conn, legacy, "phone").unwrap();

        retire_fixed_library(&mut conn).unwrap();
        ensure_first_library(&mut conn).unwrap();

        let ids = library_ids(&conn);
        assert_eq!(ids.len(), 1, "no legacy row");
        let new = &ids[0];
        assert_ne!(new, legacy);
        assert_eq!(ops::active_library(&conn).unwrap(), *new);
        assert_eq!(titles(&conn, new), vec!["Dune", "Tehanu"]);
        let finished: i64 = conn
            .query_row(
                "SELECT count(*) FROM marks WHERE library_id = ?1 AND status = 'finished'",
                [new],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(finished, 1);
        let unsigned: i64 = conn
            .query_row(
                "SELECT count(*) FROM operations WHERE library_id = ?1 AND signature IS NULL",
                [new],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(unsigned, 0);
        assert!(operations_of(&conn, new) > 0);
        assert_eq!(operations_of(&conn, legacy), 0, "no legacy operations");
        assert!(titles(&conn, legacy).is_empty());
        assert!(lan::devices_sharing(&conn, legacy).unwrap().is_empty());
    }

    #[test]
    fn a_named_legacy_library_keeps_its_name_in_the_copy() {
        let mut conn = upgraded_database();
        conn.execute(
            "UPDATE libraries SET name = 'Casa' WHERE id = ?1",
            [LEGACY_FIXED_LIBRARY_ID],
        )
        .unwrap();
        catalog::create_book(&mut conn, LEGACY_FIXED_LIBRARY_ID, &book("Tehanu"), None).unwrap();

        retire_fixed_library(&mut conn).unwrap();

        let listed = list_libraries(&conn).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "Casa");
    }

    #[test]
    fn running_the_startup_step_again_changes_nothing() {
        let mut conn = upgraded_database();
        catalog::create_book(&mut conn, LEGACY_FIXED_LIBRARY_ID, &book("Tehanu"), None).unwrap();
        retire_fixed_library(&mut conn).unwrap();
        ensure_first_library(&mut conn).unwrap();
        let snapshot = |conn: &Connection| -> (Vec<String>, i64, String) {
            (
                library_ids(conn),
                conn.query_row("SELECT count(*) FROM operations", [], |r| r.get(0))
                    .unwrap(),
                ops::active_library(conn).unwrap(),
            )
        };
        let before = snapshot(&conn);

        retire_fixed_library(&mut conn).unwrap();
        ensure_first_library(&mut conn).unwrap();

        assert_eq!(snapshot(&conn), before);
    }
}
