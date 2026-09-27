//! The people of a library and their roles (D55, D73), from this device's side:
//! what the screens list and what an administrator changes. The permission rules
//! decide whether each change is allowed, as for any other operation.

use kosmotheka_domain::{Change, Entity};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::lan;
use crate::ops::{self, new_id};
use crate::permissions;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDevice {
    pub id: String,
    pub name: String,
    /// The device this app runs on.
    pub this_device: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub can_share: bool,
    pub devices: Vec<PersonDevice>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct People {
    /// Whether the library has roles yet. Without them everyone may do everything.
    pub has_roles: bool,
    /// The role of the person this device belongs to, if any.
    pub my_role: Option<String>,
    pub people: Vec<Person>,
    /// Devices that write to the library but belong to nobody: they count as readers.
    pub unassigned: Vec<PersonDevice>,
    /// Only one administrator, with only one device: losing it would leave the
    /// library without anyone who can change permissions (D58).
    pub lone_admin: bool,
}

pub fn list_people(conn: &Connection, library: &str) -> Result<People> {
    let me = ops::device_id(conn)?;
    let devices: Vec<(String, String, Option<String>)> = conn
        .prepare("SELECT id, name, member_id FROM devices WHERE library_id = ?1 ORDER BY name")?
        .query_map([library], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let device = |id: &str, name: &str| PersonDevice {
        id: id.to_owned(),
        name: name.to_owned(),
        this_device: id == me,
    };
    let mut people: Vec<Person> = conn
        .prepare(
            "SELECT id, name, role, can_share FROM members
              WHERE library_id = ?1 ORDER BY name COLLATE NOCASE, id",
        )?
        .query_map([library], |row| {
            Ok(Person {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                can_share: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
                devices: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut unassigned = Vec::new();
    for (id, name, member) in &devices {
        match people.iter_mut().find(|p| Some(&p.id) == member.as_ref()) {
            Some(person) => person.devices.push(device(id, name)),
            None => unassigned.push(device(id, name)),
        }
    }
    let admins: Vec<&Person> = people
        .iter()
        .filter(|p| p.role.as_deref() == Some("admin"))
        .collect();
    let lone_admin = admins.len() == 1 && admins[0].devices.len() <= 1;
    let my_role = people
        .iter()
        .find(|p| p.devices.iter().any(|d| d.this_device))
        .and_then(|p| p.role.clone());
    Ok(People {
        has_roles: permissions::has_roles(conn, library)?,
        my_role,
        people,
        unassigned,
        lone_admin,
    })
}

/// Turns roles on: this device's person becomes the first administrator. `name`
/// names that person; an existing member with that name (the owner of copies) is
/// reused. Returns the member's id.
pub fn start_roles(conn: &mut Connection, library: &str, name: &str) -> Result<String> {
    if permissions::has_roles(conn, library)? {
        return Err(Error::NotAllowed);
    }
    let me = ops::device_id(conn)?;
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM members WHERE library_id = ?1 AND name = ?2 COLLATE NOCASE
              ORDER BY id LIMIT 1",
            params![library, name.trim()],
            |row| row.get(0),
        )
        .optional()?;
    let admin = json!({ "role": "admin", "can_share": true });
    let (member, first) = match existing {
        Some(id) => (id.clone(), Change::update(Entity::Member, id, admin)),
        None => {
            let id = new_id();
            let mut fields = admin;
            fields["name"] = Value::String(name.trim().to_owned());
            (id.clone(), Change::create(Entity::Member, id, fields))
        }
    };
    // One operation, judged while the library has no roles yet: made in two, the
    // second would come from a device that belongs to nobody, a reader's.
    let link = Change::update(Entity::Device, &me, json!({ "member_id": member }));
    let tx = conn.transaction()?;
    ops::record(&tx, library, "member.roles", vec![first, link])?;
    tx.commit()?;
    Ok(member)
}

/// Changes a person's role and whether they may share. The last administrator
/// cannot stop being one (D65): someone must be left to manage the library.
pub fn set_role(
    conn: &mut Connection,
    library: &str,
    member: &str,
    role: Option<&str>,
    can_share: bool,
) -> Result<()> {
    if role != Some("admin") && is_last_admin(conn, library, member)? {
        return Err(Error::LastAdmin);
    }
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        library,
        "member.role",
        vec![Change::update(
            Entity::Member,
            member,
            json!({ "role": role, "can_share": can_share }),
        )],
    )?;
    tx.commit()?;
    Ok(())
}

/// Says whose a device is, or that it is nobody's (`None`).
pub fn assign_device(
    conn: &mut Connection,
    library: &str,
    device: &str,
    member: Option<&str>,
) -> Result<()> {
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        library,
        "device.assign",
        vec![Change::update(
            Entity::Device,
            device,
            json!({ "member_id": member }),
        )],
    )?;
    tx.commit()?;
    Ok(())
}

/// A lost or stolen device (D58): in every library it writes to, its access is
/// withdrawn (D71) and it stops belonging to its person, so whatever it still
/// sends counts as a reader's and becomes a request. Returns how many libraries.
pub fn remove_device(conn: &mut Connection, device: &str) -> Result<usize> {
    if device == ops::device_id(conn)? {
        return Err(Error::NotAllowed);
    }
    let name = lan::known_device_name(conn, device)?;
    let libraries: Vec<(String, String)> = conn
        .prepare("SELECT library_id, name FROM devices WHERE id = ?1")?
        .query_map([device], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let tx = conn.transaction()?;
    for (library, registered_name) in &libraries {
        let mut changes = vec![Change::update(
            Entity::Device,
            device,
            json!({ "member_id": null }),
        )];
        if lan::revocations_of(&tx, library, device)?.is_empty() {
            changes.push(Change::create(
                Entity::Revocation,
                new_id(),
                json!({
                    "device_id": device,
                    "device_name": name.clone().unwrap_or_else(|| registered_name.clone()),
                }),
            ));
        }
        ops::record(&tx, library, "device.remove", changes)?;
        tx.execute(
            "DELETE FROM library_peers WHERE library_id = ?1 AND device_id = ?2",
            [library, device],
        )?;
    }
    tx.commit()?;
    Ok(libraries.len())
}

fn is_last_admin(conn: &Connection, library: &str, member: &str) -> Result<bool> {
    let (admins, this_one): (i64, bool) = conn.query_row(
        "SELECT count(*), coalesce(max(id = ?2), 0) FROM members
          WHERE library_id = ?1 AND role = 'admin'",
        [library, member],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(admins == 1 && this_one)
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};

    use super::*;
    use crate::backup;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::ops::DEFAULT_LIBRARY_ID as LIB;
    use crate::sync::{self, SyncOperation};

    fn book(title: &str) -> ValidBook {
        ValidBook {
            edition: EditionInput {
                title: title.into(),
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: Some("Tersoal".into()),
                format: Format::Paperback,
                quantity: 1,
                location: None,
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

    /// Everything `from` has, merged into `to`, as a sync would.
    fn pull(to: &mut Connection, from: &Connection) {
        let libraries = vec![LIB.to_owned()];
        let mine = sync::version_vectors(to, &libraries).unwrap();
        let missing: Vec<SyncOperation> = sync::operations_since(from, &mine).unwrap();
        sync::merge(to, missing).unwrap();
    }

    #[test]
    fn starting_roles_makes_this_device_s_person_the_administrator() {
        let mut conn = db::open_in_memory().unwrap();
        catalog::create_book(&mut conn, LIB, &book("Dune"), None).unwrap();

        let member = start_roles(&mut conn, LIB, "tersoal").unwrap();
        let people = list_people(&conn, LIB).unwrap();

        assert!(people.has_roles);
        assert_eq!(people.my_role.as_deref(), Some("admin"));
        let me = people.people.iter().find(|p| p.id == member).unwrap();
        assert_eq!(me.name, "Tersoal", "the owner of the copy is reused");
        assert!(me.can_share);
        assert!(people.lone_admin, "one administrator with one device");
        assert!(matches!(
            start_roles(&mut conn, LIB, "Ana"),
            Err(Error::NotAllowed)
        ));
    }

    #[test]
    fn the_last_administrator_stays_one() {
        let mut conn = db::open_in_memory().unwrap();
        let member = start_roles(&mut conn, LIB, "Tersoal").unwrap();

        let result = set_role(&mut conn, LIB, &member, Some("editor"), false);

        assert!(matches!(result, Err(Error::LastAdmin)));
    }

    #[test]
    fn a_lost_phone_is_removed_and_what_it_still_sends_becomes_a_request() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        catalog::create_book(&mut phone, LIB, &book("Tehanu"), None).unwrap();
        let member = start_roles(&mut laptop, LIB, "Tersoal").unwrap();
        pull(&mut laptop, &phone);
        let phone_id = ops::device_id(&phone).unwrap();
        assign_device(&mut laptop, LIB, &phone_id, Some(&member)).unwrap();
        pull(&mut phone, &laptop);
        assert!(
            !list_people(&laptop, LIB).unwrap().lone_admin,
            "two devices"
        );

        assert_eq!(remove_device(&mut laptop, &phone_id).unwrap(), 1);
        // Whoever has the phone now keeps writing, and it reaches the laptop somehow.
        catalog::create_book(&mut phone, LIB, &book("Stolen"), None).unwrap();
        pull(&mut laptop, &phone);

        assert!(!titles(&laptop, LIB).contains(&"Stolen".to_owned()));
        let requests = crate::requests::list_requests(&laptop, LIB).unwrap();
        assert_eq!(requests.len(), 1);
        let people = list_people(&laptop, LIB).unwrap();
        assert_eq!(people.unassigned.len(), 1);
        assert!(lan::revocations_of(&laptop, LIB, &phone_id).unwrap().len() == 1);
    }

    #[test]
    fn a_duplicate_has_the_books_and_no_roles() {
        let mut conn = db::open_in_memory().unwrap();
        catalog::create_book(&mut conn, LIB, &book("Dune"), None).unwrap();
        start_roles(&mut conn, LIB, "Tersoal").unwrap();

        let copy = backup::duplicate(&mut conn, LIB, "Casa (copia)").unwrap();

        assert_eq!(titles(&conn, &copy), vec!["Dune"]);
        assert!(!permissions::has_roles(&conn, &copy).unwrap());
        assert_eq!(ops::active_library(&conn).unwrap(), copy);
        catalog::create_book(&mut conn, &copy, &book("Tehanu"), None).unwrap();
    }
}
