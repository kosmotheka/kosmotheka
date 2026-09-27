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
    /// Other libraries on this device where the same person is (same member id).
    pub elsewhere: Vec<String>,
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
    /// Only one person administers the library: losing their devices would leave
    /// nobody who can change permissions (D58).
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
                elsewhere: Vec::new(),
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
    let lone_admin = admins.len() == 1;
    for person in &mut people {
        person.elsewhere = libraries_of(conn, &person.id, library)?;
    }
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

/// Someone known in other libraries of this device but not in this one: the same
/// person can join it with the same member id (D73).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownPerson {
    pub id: String,
    pub name: String,
    pub libraries: Vec<String>,
}

pub fn known_elsewhere(conn: &Connection, library: &str) -> Result<Vec<KnownPerson>> {
    let found: Vec<(String, String)> = conn
        .prepare(
            "SELECT id, min(name) FROM members
              WHERE library_id != ?1
                AND id NOT IN (SELECT id FROM members WHERE library_id = ?1)
              GROUP BY id ORDER BY min(name) COLLATE NOCASE",
        )?
        .query_map([library], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    found
        .into_iter()
        .map(|(id, name)| {
            Ok(KnownPerson {
                libraries: libraries_of(conn, &id, library)?,
                id,
                name,
            })
        })
        .collect()
}

/// The names of the libraries, other than `except`, where `member` is.
fn libraries_of(conn: &Connection, member: &str, except: &str) -> Result<Vec<String>> {
    Ok(conn
        .prepare(
            "SELECT l.name FROM members m JOIN libraries l ON l.id = m.library_id
              WHERE m.id = ?1 AND m.library_id != ?2 ORDER BY l.name COLLATE NOCASE",
        )?
        .query_map([member, except], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// A person picked on screen: one already in the library or in another one
/// (`id`), or somebody new (`id` empty, `name` given).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonPick {
    pub id: Option<String>,
    pub name: String,
}

/// Whose a device is, when roles are turned on, and with which role.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceOwner {
    pub device_id: String,
    /// `None`: nobody's, so it counts as a reader.
    pub person: Option<PersonPick>,
    pub role: Option<String>,
}

/// The member row for `pick` in `library`: an update when it is already there
/// (the role only if it has none), a create otherwise, keeping the id it has in
/// other libraries. Returns its id.
fn member_changes(
    conn: &Connection,
    library: &str,
    pick: &PersonPick,
    role: Option<&str>,
    can_share: Option<bool>,
    changes: &mut Vec<Change>,
) -> Result<String> {
    let current: Option<Option<String>> = match &pick.id {
        Some(id) => conn
            .query_row(
                "SELECT role FROM members WHERE library_id = ?1 AND id = ?2",
                [library, id],
                |row| row.get(0),
            )
            .optional()?,
        None => None,
    };
    let id = pick.id.clone().unwrap_or_else(new_id);
    let mut fields = serde_json::Map::new();
    if let Some(role) = role {
        fields.insert("role".into(), json!(role));
    }
    if let Some(can_share) = can_share {
        fields.insert("can_share".into(), json!(can_share));
    }
    match current {
        // A role is given, never inherited: an existing one stays as it is.
        Some(Some(_)) => {}
        Some(None) if !fields.is_empty() => {
            changes.push(Change::update(Entity::Member, &id, Value::Object(fields)));
        }
        Some(None) => {}
        None => {
            if role.is_none() {
                return Err(Error::NotAllowed);
            }
            fields.insert("name".into(), json!(pick.name.trim()));
            changes.push(Change::create(Entity::Member, &id, Value::Object(fields)));
        }
    }
    Ok(id)
}

/// Turns roles on: this device's person becomes the first administrator. `name`
/// names that person; an existing member with that name (the owner of copies) is
/// reused. Returns the member's id.
pub fn start_roles(
    conn: &mut Connection,
    library: &str,
    name: &str,
    owners: &[DeviceOwner],
) -> Result<String> {
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
    let mut changes = vec![
        first,
        Change::update(Entity::Device, &me, json!({ "member_id": member })),
    ];
    // Everyone already sharing the library says whose device it is, in the same step.
    for owner in owners.iter().filter(|owner| owner.device_id != me) {
        let member = match &owner.person {
            Some(pick) if pick.id.as_deref() == Some(member.as_str()) => Some(member.clone()),
            Some(pick) => Some(member_changes(
                conn,
                library,
                pick,
                Some(owner.role.as_deref().unwrap_or("editor")),
                None,
                &mut changes,
            )?),
            None => None,
        };
        changes.push(Change::update(
            Entity::Device,
            &owner.device_id,
            json!({ "member_id": member }),
        ));
    }
    let tx = conn.transaction()?;
    ops::record(&tx, library, "member.roles", changes)?;
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

/// Who a newly paired device may belong to: the people of the libraries it
/// receives where this device may say whose it is (roles on, and administrator).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingOwners {
    /// Library ids, in the order the screens list them.
    pub libraries: Vec<String>,
    pub people: Vec<Candidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub name: String,
    /// The person this device belongs to.
    pub me: bool,
    /// Their role in each of those libraries where they have one.
    pub roles: std::collections::BTreeMap<String, String>,
    pub devices: usize,
}

pub fn pairing_owners(conn: &Connection, device: &str) -> Result<PairingOwners> {
    let shared: Vec<String> = conn
        .prepare(
            "SELECT p.library_id FROM library_peers p JOIN libraries l ON l.id = p.library_id
              WHERE p.device_id = ?1 ORDER BY l.name COLLATE NOCASE, l.id",
        )?
        .query_map([device], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut libraries = Vec::new();
    let mut people: Vec<Candidate> = Vec::new();
    for library in shared {
        let listed = list_people(conn, &library)?;
        if !listed.has_roles || listed.my_role.as_deref() != Some("admin") {
            continue;
        }
        for person in listed.people {
            let index = match people.iter().position(|c| c.id == person.id) {
                Some(index) => index,
                None => {
                    people.push(Candidate {
                        id: person.id.clone(),
                        name: person.name.clone(),
                        me: false,
                        roles: Default::default(),
                        devices: 0,
                    });
                    people.len() - 1
                }
            };
            let candidate = &mut people[index];
            candidate.me |= person.devices.iter().any(|d| d.this_device);
            candidate.devices = candidate.devices.max(person.devices.len());
            if let Some(role) = person.role {
                candidate.roles.insert(library.clone(), role);
            }
        }
        libraries.push(library);
    }
    people.sort_by_key(|c| (!c.me, c.name.to_lowercase()));
    Ok(PairingOwners { libraries, people })
}

/// Says whose a newly paired device is (D73): `person` joins each of `libraries`
/// with the role given for it where they have none yet (a role is never inherited),
/// and the device becomes theirs in each one. The device may not have registered
/// there yet, since only it can create its row: then the assignment waits until
/// its registration arrives (`apply_pending_owners`).
pub fn give_access(
    conn: &mut Connection,
    device: &str,
    person: &PersonPick,
    roles: &std::collections::BTreeMap<String, String>,
    can_share: bool,
) -> Result<()> {
    let libraries: Vec<String> = conn
        .prepare("SELECT library_id FROM library_peers WHERE device_id = ?1")?
        .query_map([device], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let mut member_id = person.id.clone();
    let tx = conn.transaction()?;
    for library in &libraries {
        // Only where roles are on and this device may say whose a device is.
        let listed = list_people(&tx, library)?;
        if !listed.has_roles || listed.my_role.as_deref() != Some("admin") {
            continue;
        }
        let pick = PersonPick {
            id: member_id.clone(),
            name: person.name.clone(),
        };
        let mut changes = Vec::new();
        let role = roles.get(library).map(String::as_str);
        let member = member_changes(&tx, library, &pick, role, Some(can_share), &mut changes)?;
        member_id = Some(member.clone());
        if is_registered(&tx, library, device)? {
            changes.push(Change::update(
                Entity::Device,
                device,
                json!({ "member_id": member }),
            ));
        } else {
            tx.execute(
                "INSERT OR REPLACE INTO pending_owners (library_id, device_id, member_id)
                 VALUES (?1, ?2, ?3)",
                [library, device, &member],
            )?;
        }
        if !changes.is_empty() {
            ops::record(&tx, library, "device.give_access", changes)?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Records the assignments that were waiting for their device to register. One
/// this device may not make (it stopped being an administrator) is dropped.
pub fn apply_pending_owners(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    let ready: Vec<(String, String, String)> = tx
        .prepare(
            "SELECT p.library_id, p.device_id, p.member_id FROM pending_owners p
               JOIN devices d ON d.library_id = p.library_id AND d.id = p.device_id",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (library, device, member) in ready {
        let change = Change::update(Entity::Device, &device, json!({ "member_id": member }));
        match ops::record(tx, &library, "device.assign", vec![change]) {
            Ok(_) | Err(Error::NotAllowed) => {}
            Err(error) => return Err(error),
        }
        tx.execute(
            "DELETE FROM pending_owners WHERE library_id = ?1 AND device_id = ?2",
            [&library, &device],
        )?;
    }
    Ok(())
}

fn is_registered(conn: &Connection, library: &str, device: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM devices WHERE library_id = ?1 AND id = ?2)",
        [library, device],
        |row| row.get(0),
    )?)
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

        let member = start_roles(&mut conn, LIB, "tersoal", &[]).unwrap();
        let people = list_people(&conn, LIB).unwrap();

        assert!(people.has_roles);
        assert_eq!(people.my_role.as_deref(), Some("admin"));
        let me = people.people.iter().find(|p| p.id == member).unwrap();
        assert_eq!(me.name, "Tersoal", "the owner of the copy is reused");
        assert!(me.can_share);
        assert!(people.lone_admin, "one administrator with one device");
        assert!(matches!(
            start_roles(&mut conn, LIB, "Ana", &[]),
            Err(Error::NotAllowed)
        ));
    }

    #[test]
    fn the_last_administrator_stays_one() {
        let mut conn = db::open_in_memory().unwrap();
        let member = start_roles(&mut conn, LIB, "Tersoal", &[]).unwrap();

        let result = set_role(&mut conn, LIB, &member, Some("editor"), false);

        assert!(matches!(result, Err(Error::LastAdmin)));
    }

    #[test]
    fn a_lost_phone_is_removed_and_what_it_still_sends_becomes_a_request() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        catalog::create_book(&mut phone, LIB, &book("Tehanu"), None).unwrap();
        let member = start_roles(&mut laptop, LIB, "Tersoal", &[]).unwrap();
        pull(&mut laptop, &phone);
        let phone_id = ops::device_id(&phone).unwrap();
        assign_device(&mut laptop, LIB, &phone_id, Some(&member)).unwrap();
        pull(&mut phone, &laptop);
        assert!(
            list_people(&laptop, LIB).unwrap().lone_admin,
            "one person administers it, however many devices she has"
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
        start_roles(&mut conn, LIB, "Tersoal", &[]).unwrap();

        let copy = backup::duplicate(&mut conn, LIB, "Casa (copia)").unwrap();

        assert_eq!(titles(&conn, &copy), vec!["Dune"]);
        assert!(!permissions::has_roles(&conn, &copy).unwrap());
        assert_eq!(ops::active_library(&conn).unwrap(), copy);
        catalog::create_book(&mut conn, &copy, &book("Tehanu"), None).unwrap();
    }

    #[test]
    fn a_second_administrator_ends_the_warning() {
        let mut conn = db::open_in_memory().unwrap();
        start_roles(&mut conn, LIB, "Tersoal", &[]).unwrap();
        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let mut changes = Vec::new();
        let id = member_changes(&conn, LIB, &laura, Some("admin"), None, &mut changes).unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(&tx, LIB, "member.add", changes).unwrap();
        tx.commit().unwrap();

        let people = list_people(&conn, LIB).unwrap();

        assert!(!people.lone_admin);
        assert!(people.people.iter().any(|p| p.id == id));
    }

    #[test]
    fn turning_roles_on_says_whose_every_device_is() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        let mut tablet = db::open_in_memory().unwrap();
        catalog::create_book(&mut phone, LIB, &book("Tehanu"), None).unwrap();
        catalog::create_book(&mut tablet, LIB, &book("Solaris"), None).unwrap();
        pull(&mut laptop, &phone);
        pull(&mut laptop, &tablet);
        let (phone_id, tablet_id) = (
            ops::device_id(&phone).unwrap(),
            ops::device_id(&tablet).unwrap(),
        );

        start_roles(
            &mut laptop,
            LIB,
            "Ana",
            &[
                DeviceOwner {
                    device_id: phone_id.clone(),
                    person: Some(PersonPick {
                        id: None,
                        name: "Laura".into(),
                    }),
                    role: None,
                },
                DeviceOwner {
                    device_id: tablet_id.clone(),
                    person: None,
                    role: None,
                },
            ],
        )
        .unwrap();

        let people = list_people(&laptop, LIB).unwrap();
        let laura = people.people.iter().find(|p| p.name == "Laura").unwrap();
        assert_eq!(laura.role.as_deref(), Some("editor"), "editor by default");
        assert_eq!(laura.devices[0].id, phone_id);
        assert_eq!(people.unassigned[0].id, tablet_id, "nobody's: a reader");
    }

    #[test]
    fn the_same_person_joins_another_library_with_the_same_id() {
        let mut conn = db::open_in_memory().unwrap();
        let hugo = PersonPick {
            id: None,
            name: "Hugo".into(),
        };
        let mut changes = Vec::new();
        let id = member_changes(&conn, LIB, &hugo, Some("reader"), None, &mut changes).unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(&tx, LIB, "member.add", changes).unwrap();
        tx.commit().unwrap();
        let games = crate::libraries::create_library(&mut conn, "Juegos").unwrap();

        let known = known_elsewhere(&conn, &games).unwrap();
        assert_eq!(known.len(), 1);
        assert_eq!(known[0].id, id);

        let again = PersonPick {
            id: Some(id.clone()),
            name: "Hugo".into(),
        };
        let mut changes = Vec::new();
        assert!(
            member_changes(&conn, &games, &again, None, None, &mut changes).is_err(),
            "joining a library needs a role chosen for it"
        );
        let joined =
            member_changes(&conn, &games, &again, Some("editor"), None, &mut changes).unwrap();
        assert_eq!(joined, id);
    }

    #[test]
    fn pairing_offers_only_libraries_with_roles_that_this_device_administers() {
        let mut laptop = db::open_in_memory().unwrap();
        let phone = db::open_in_memory().unwrap();
        start_roles(&mut laptop, LIB, "Ana", &[]).unwrap();
        let games = crate::libraries::create_library(&mut laptop, "Juegos").unwrap();
        let phone_id = ops::device_id(&phone).unwrap();
        lan::share_library(&laptop, LIB, &phone_id).unwrap();
        lan::share_library(&laptop, &games, &phone_id).unwrap();

        let owners = pairing_owners(&laptop, &phone_id).unwrap();
        assert_eq!(owners.libraries, vec![LIB.to_owned()]);
        assert_eq!(owners.people.len(), 1);
        assert!(owners.people[0].me);
        assert_eq!(owners.people[0].roles[LIB], "admin");

        let roles = std::collections::BTreeMap::from([(LIB.to_owned(), "reader".to_owned())]);
        let hugo = PersonPick {
            id: None,
            name: "Hugo".into(),
        };
        give_access(&mut laptop, &phone_id, &hugo, &roles, false).unwrap();
        assert!(
            !list_people(&laptop, &games).unwrap().has_roles,
            "a library without roles keeps working without them"
        );
    }

    #[test]
    fn a_new_device_becomes_its_person_s_once_it_registers() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        start_roles(&mut laptop, LIB, "Ana", &[]).unwrap();
        let phone_id = ops::device_id(&phone).unwrap();
        lan::share_library(&laptop, LIB, &phone_id).unwrap();
        let roles = std::collections::BTreeMap::from([(LIB.to_owned(), "editor".to_owned())]);

        give_access(
            &mut laptop,
            &phone_id,
            &PersonPick {
                id: None,
                name: "Laura".into(),
            },
            &roles,
            false,
        )
        .unwrap();
        assert!(
            list_people(&laptop, LIB).unwrap().unassigned.is_empty(),
            "not registered yet"
        );

        // Over the network, the phone registers as soon as the library arrives.
        let laptop_id = ops::device_id(&laptop).unwrap();
        lan::share_library(&phone, LIB, &laptop_id).unwrap();
        pull(&mut phone, &laptop);
        lan::register_in_shared(&mut phone, &laptop_id).unwrap();
        pull(&mut laptop, &phone);
        pull(&mut phone, &laptop);

        let people = list_people(&laptop, LIB).unwrap();
        let laura = people.people.iter().find(|p| p.name == "Laura").unwrap();
        assert_eq!(laura.devices[0].id, phone_id);
        assert_eq!(
            list_people(&phone, LIB).unwrap().my_role.as_deref(),
            Some("editor")
        );
        catalog::create_book(&mut phone, LIB, &book("Tehanu"), None).unwrap();
    }
}
