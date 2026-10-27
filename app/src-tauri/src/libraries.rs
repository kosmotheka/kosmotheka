//! The libraries on this device (D46, D54, D65): list them, create, rename and
//! switch between them, delete one everywhere or leave it on this device only, and
//! withdraw or give back a device's access to one (D71).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use kosmotheka_domain::{Change, Entity, ErrorCode, FieldError, Hlc, validate_library_name};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
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
    let active = ops::current_library(conn)?;
    let mut statement = conn.prepare(
        "SELECT l.id, l.name, (SELECT count(*) FROM editions e WHERE e.library_id = l.id AND e.trashed_at IS NULL)
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
                active: active.as_deref() == Some(id.as_str()),
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
              WHERE library_id = ?1 AND intent = 'library.found' ORDER BY hlc LIMIT 1",
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

/// Every device that has told its person's name has a library to work in (D85, D97): a random id, founded like
/// any other, so it travels like any other. Until the welcome has the name there is none, and the app shows
/// nothing but the welcome.
pub fn ensure_first_library(conn: &mut Connection) -> Result<()> {
    if first_library(conn)?.is_none() && crate::settings::person_name(conn)?.is_some() {
        let tx = conn.transaction()?;
        let id = create_in(&tx, "")?;
        ops::set_active_library(&tx, &id)?;
        tx.commit()?;
    }
    Ok(())
}

/// The end of the welcome's first question (D97): the name this device's person goes by, and with it «Mi
/// biblioteca», founded with that person as its administrator. Asking again with a library already there only
/// changes the name.
pub fn begin(conn: &mut Connection, raw_name: &str) -> Result<()> {
    crate::settings::set_person_name(conn, raw_name)?;
    ensure_first_library(conn)
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
    found(tx, name)
}

/// Founds a library (D97): one operation in which this device registers itself, its person is created as an
/// administrator and the library row is created with the nonce its id derives from, together with this device's
/// key. Whoever holds the key is the only one who can found that id, whatever date they put on the operation.
/// Returns the library's id.
pub fn found(tx: &Transaction<'_>, name: &str) -> Result<String> {
    let key = ops::signing_key(tx)?.public_key();
    let nonce_bytes: [u8; kosmotheka_domain::NONCE_BYTES] = rand::random();
    let nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);
    let id =
        kosmotheka_domain::library_id(&key, &nonce).ok_or(Error::CorruptState("signing_seed"))?;
    found_as(tx, &id, &nonce, name)
}

/// Whether the id `library` derives from this device's key and `nonce`: this device could have founded it.
pub fn derives_here(conn: &Connection, library: &str, nonce: &str) -> Result<bool> {
    let key = ops::signing_key(conn)?.public_key();
    Ok(kosmotheka_domain::library_id(&key, nonce)
        .is_some_and(|id| id == library.to_ascii_lowercase()))
}

/// Founds the library `id`, which derives from this device's key and `nonce` (`found` makes a new one; a restore
/// founds again one this device had founded).
pub fn found_as(tx: &Transaction<'_>, id: &str, nonce: &str, name: &str) -> Result<String> {
    let key = ops::signing_key(tx)?.public_key();
    let id = id.to_ascii_lowercase();
    let id = id.as_str();
    // The creator is the administrator under the name the welcome asked for; without it nothing is founded.
    let person = crate::settings::person_name(tx)?.ok_or_else(|| {
        Error::Validation(vec![FieldError::new("person_name", ErrorCode::Required)])
    })?;
    let member = new_id();
    let device = ops::device_id(tx)?;
    ops::record(
        tx,
        id,
        "library.found",
        vec![
            Change::create(
                Entity::Library,
                id,
                json!({ "name": name, "founding_nonce": nonce }),
            ),
            // The person comes before the device that names it: a row cannot point at one that is not there yet.
            Change::create(
                Entity::Member,
                &member,
                json!({ "name": person, "role": "admin", "can_share": true }),
            ),
            Change::create(
                Entity::Device,
                &device,
                json!({
                    "public_key": key,
                    "name": lan::device_name(tx)?,
                    "member_id": member,
                    "vouched_key": key,
                }),
            ),
        ],
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
///
/// Only where another device of the local network has it: a library that is only here
/// would be lost. `leave_library_in_cloud` is for the one the cloud has too.
pub fn leave_library(conn: &mut Connection, id: &str) -> Result<String> {
    leave(conn, id, false)
}

/// `leave_library` for a library the caller has just checked the cloud still offers to this account
/// (`cloud::leave`): the cloud is the other place it stays, and it can be brought down again (`left_libraries`).
pub(crate) fn leave_library_in_cloud(conn: &mut Connection, id: &str) -> Result<String> {
    leave(conn, id, true)
}

fn leave(conn: &mut Connection, id: &str, in_cloud: bool) -> Result<String> {
    ensure_exists(conn, id)?;
    if !in_cloud && lan::devices_sharing(conn, id)?.is_empty() {
        return Err(Error::OnlyOnThisDevice);
    }
    let tx = conn.transaction()?;
    // Read before the library's row goes.
    let left = LeftLibrary::of(&tx, id)?;
    let active = remove_here(&tx, id, None)?;
    if let Some(left) = left {
        remember_left(&tx, left)?;
    }
    tx.commit()?;
    Ok(active)
}

/// A library that lives in the cloud of a server that is not the official one (`account::OFFICIAL_SERVER`), run by
/// somebody else. Only said, never followed: the app talks to the account's server and no other (D84).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignCloud {
    pub library: String,
    /// Empty for a library never named: the UI shows its own default name.
    pub name: String,
    pub cloud_server: String,
}

/// Which of `ids` this device has, linked to a cloud on a server that is not the official one.
/// A library with no cloud, or in the official one, is not among them.
pub fn foreign_clouds(conn: &Connection, ids: &[String]) -> Result<Vec<ForeignCloud>> {
    let mut found = Vec::new();
    for id in ids {
        let row = conn
            .query_row(
                "SELECT name, cloud_server FROM libraries WHERE id = ?1 AND cloud_library_id IS NOT NULL",
                [id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?;
        if let Some((name, Some(cloud_server))) = row
            && cloud_server != crate::account::OFFICIAL_SERVER
        {
            found.push(ForeignCloud {
                library: id.clone(),
                name,
                cloud_server,
            });
        }
    }
    Ok(found)
}

/// A library this device left by its own choice, and that was linked to a cloud library: what it takes to ask for
/// it again (`cloud::redownload`) and to show it in the meantime. Local state, not a table: the library's row
/// is gone. Whoever withdrew the device's access is never listed here (`forget_here` drops it), and only a
/// library also in `forgotten_libraries` counts (SEG-017): a row from before this list existed means «withdrawn».
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftLibrary {
    pub id: String,
    /// Empty for a library never named: the UI shows its own default name.
    pub name: String,
    pub cloud_library: String,
    pub cloud_server: String,
}

const LEFT_LIBRARIES: &str = "left_libraries";

impl LeftLibrary {
    /// `None` for a library that is not linked to a cloud one: it comes back by pairing, as it always did.
    fn of(conn: &Connection, id: &str) -> Result<Option<Self>> {
        Ok(conn
            .query_row(
                "SELECT name, cloud_library_id, cloud_server FROM libraries WHERE id = ?1",
                [id],
                |row| {
                    let cloud: (Option<String>, Option<String>) = (row.get(1)?, row.get(2)?);
                    Ok(match cloud {
                        (Some(cloud_library), Some(cloud_server)) => Some(Self {
                            id: id.to_owned(),
                            name: row.get(0)?,
                            cloud_library,
                            cloud_server,
                        }),
                        _ => None,
                    })
                },
            )
            .optional()?
            .flatten())
    }
}

fn read_left(conn: &Connection) -> Result<Vec<LeftLibrary>> {
    Ok(match ops::read_state(conn, LEFT_LIBRARIES)? {
        // A list that cannot be read is an empty one: nothing comes back, which is the safe side.
        Some(text) => serde_json::from_str(&text).unwrap_or_default(),
        None => Vec::new(),
    })
}

fn write_left(conn: &Connection, left: &[LeftLibrary]) -> Result<()> {
    let text = serde_json::to_string(left).map_err(|_| Error::CorruptState(LEFT_LIBRARIES))?;
    ops::write_state(
        conn,
        LEFT_LIBRARIES,
        (!left.is_empty()).then_some(text.as_str()),
    )
}

fn remember_left(conn: &Connection, entry: LeftLibrary) -> Result<()> {
    let mut left = read_left(conn)?;
    left.retain(|other| other.id != entry.id);
    left.push(entry);
    write_left(conn, &left)
}

fn drop_left(conn: &Connection, id: &str) -> Result<()> {
    let mut left = read_left(conn)?;
    left.retain(|other| other.id != id);
    write_left(conn, &left)
}

/// The libraries this device left by choice and may ask for again. Withdrawn ones are not there, whatever the
/// list says: they must be in `forgotten_libraries` too, and a withdrawal drops them from the list.
pub fn left_libraries(conn: &Connection) -> Result<Vec<LeftLibrary>> {
    let mut left = read_left(conn)?;
    for entry in &mut left {
        entry.name = entry.name.trim().to_owned();
    }
    left.retain(|entry| library_forgotten(conn, &entry.id).unwrap_or(false));
    Ok(left)
}

/// Gives a left library back its place before it is brought down: out of both memories. The caller puts them back
/// with `restore_left` if the cloud does not hand it over.
pub(crate) fn take_left(conn: &Connection, id: &str) -> Result<Option<LeftLibrary>> {
    let Some(entry) = left_libraries(conn)?
        .into_iter()
        .find(|entry| entry.id == id)
    else {
        return Ok(None);
    };
    conn.execute(
        "DELETE FROM forgotten_libraries WHERE library_id = ?1",
        [id],
    )?;
    drop_left(conn, id)?;
    Ok(Some(entry))
}

/// Undoes `take_left`. `asked_again` is false when the cloud has shown that the access was withdrawn: then the
/// library stays forgotten and is no longer one the person left.
pub(crate) fn restore_left(conn: &Connection, entry: LeftLibrary, asked_again: bool) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO forgotten_libraries (library_id) VALUES (?1)",
        [&entry.id],
    )?;
    if asked_again {
        remember_left(conn, entry)?;
    }
    Ok(())
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
    // A withdrawal beats leaving: whatever the person once chose, this library does not come back by asking.
    drop_left(tx, id)?;
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
    if let Some(revocation) = revocation_change(&tx, id, device_id, &name)? {
        let mut changes = vec![revocation];
        // The revoked device is fenced in the same operation (D84).
        changes.extend(crate::permissions::fence_changes(
            &tx,
            id,
            &[device_id.to_owned()],
        )?);
        ops::record(&tx, id, "library.revoke", changes)?;
    }
    forget_peer(&tx, id, device_id)?;
    tx.commit()?;
    Ok(())
}

/// The change that withdraws a device's access (D71), unless it already is withdrawn. Shared with
/// echar a una persona (D98), which writes it together with the rest of its step.
pub fn revocation_change(
    conn: &Connection,
    library: &str,
    device_id: &str,
    name: &str,
) -> Result<Option<Change>> {
    if !lan::revocations_of(conn, library, device_id)?.is_empty() {
        return Ok(None);
    }
    Ok(Some(Change::create(
        Entity::Revocation,
        new_id(),
        json!({ "device_id": device_id, "device_name": name }),
    )))
}

/// This device stops sharing the library with `device_id` over the local network.
pub fn forget_peer(conn: &Connection, library: &str, device_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM library_peers WHERE library_id = ?1 AND device_id = ?2",
        [library, device_id],
    )?;
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
    fn a_library_in_the_cloud_of_another_server_is_marked_and_the_others_are_not() {
        let mut conn = db::open_in_memory().unwrap();
        let official = create_library(&mut conn, "Official").unwrap();
        let foreign = create_library(&mut conn, "Foreign").unwrap();
        let local = create_library(&mut conn, "Local").unwrap();
        let unlinked_server = create_library(&mut conn, "Odd").unwrap();
        for (id, cloud, server) in [
            (&official, Some("c1"), Some(crate::account::OFFICIAL_SERVER)),
            (&foreign, Some("c2"), Some("https://biblio.casa-ruiz.es")),
            (&unlinked_server, None, Some("https://biblio.casa-ruiz.es")),
        ] {
            conn.execute(
                "UPDATE libraries SET cloud_library_id = ?2, cloud_server = ?3 WHERE id = ?1",
                params![id, cloud, server],
            )
            .unwrap();
        }
        let ids = [official, foreign.clone(), local, unlinked_server];

        let marked = foreign_clouds(&conn, &ids).unwrap();

        assert_eq!(
            marked,
            vec![ForeignCloud {
                library: foreign,
                name: "Foreign".to_owned(),
                cloud_server: "https://biblio.casa-ruiz.es".to_owned(),
            }]
        );
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

    fn library_ids(conn: &Connection) -> Vec<String> {
        conn.prepare("SELECT id FROM libraries ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }

    #[test]
    fn nothing_is_founded_until_the_welcome_has_the_persons_name() {
        let mut conn = db::open_in_memory_blank().unwrap();
        assert_eq!(first_library(&conn).unwrap(), None);
        ensure_first_library(&mut conn).unwrap();
        assert_eq!(first_library(&conn).unwrap(), None, "no name, no library");
        // Not even by creating one: the creator is the administrator under a name.
        assert!(matches!(
            create_library(&mut conn, "Club"),
            Err(Error::Validation(_))
        ));
        assert_eq!(first_library(&conn).unwrap(), None);

        begin(&mut conn, "  Ana ").unwrap();

        let id = first_library(&conn).unwrap().expect("founded");
        assert_eq!(
            ops::current_library(&conn).unwrap().as_deref(),
            Some(id.as_str())
        );
        let admin: (String, String) = conn
            .query_row(
                "SELECT name, role FROM members WHERE library_id = ?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(admin, ("Ana".to_owned(), "admin".to_owned()));
        // The libraries created afterwards carry the same name, and asking again changes nothing else.
        let club = create_library(&mut conn, "Club").unwrap();
        let in_club: String = conn
            .query_row(
                "SELECT name FROM members WHERE library_id = ?1",
                [&club],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(in_club, "Ana");
        begin(&mut conn, "Ana B").unwrap();
        assert_eq!(all_ids(&conn), 2);
    }

    fn all_ids(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM libraries", [], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn a_blank_name_does_not_begin_anything() {
        let mut conn = db::open_in_memory_blank().unwrap();
        for name in ["", "   ", &"x".repeat(101)] {
            assert!(matches!(begin(&mut conn, name), Err(Error::Validation(_))));
        }
        assert_eq!(first_library(&conn).unwrap(), None);
    }

    #[test]
    fn a_fresh_database_has_one_library_founded_by_this_device_with_a_signed_operation() {
        let conn = db::open_in_memory().unwrap();

        let ids = library_ids(&conn);
        assert_eq!(ids.len(), 1);
        assert_eq!(ops::active_library(&conn).unwrap(), ids[0]);
        let signed: i64 = conn
            .query_row(
                "SELECT count(*) FROM operations
                  WHERE library_id = ?1 AND intent = 'library.found' AND signature IS NOT NULL",
                [&ids[0]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(signed, 1);
    }
}
