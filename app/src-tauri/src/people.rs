//! The people of a library and their roles (D55, D73), from this device's side:
//! what the screens list and what an administrator changes. The permission rules
//! decide whether each change is allowed, as for any other operation.

use kosmotheka_domain::{Change, Entity, LibraryView};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::{Error, Result};
use crate::lan;
use crate::libraries;
use crate::ops::{self, new_id};
use crate::permissions;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonDevice {
    pub id: String,
    pub name: String,
    /// The device this app runs on.
    pub this_device: bool,
    /// Removing it would leave the library without an administrator who can act (SEG-036): refused.
    pub last_device_of_last_admin: bool,
    /// The operation that said whose it is was signed by the cloud's device (D76, D96).
    pub linked_by_cloud: bool,
    /// The person whose device signed that operation, when it was not the cloud, nor the device itself.
    pub linked_by: Option<String>,
    /// When it last signed anything in this library, in milliseconds since 1970: its own registration at least.
    pub last_change: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub can_share: bool,
    /// The person may no longer reach the library (D98); they stay in the catalog.
    pub access_withdrawn: bool,
    /// The last administrator who can act: they cannot be demoted nor withdrawn (D58, SEG-036).
    pub last_admin: bool,
    /// An account is linked to this person (D76). Never says which one.
    pub has_account: bool,
    /// Whether this person can be invited, from what this device knows: only a reader or an editor who is not
    /// withdrawn and has no account (D43). Who may invite, and whether the server does, is another matter.
    pub invitable: Invitable,
    pub devices: Vec<PersonDevice>,
    /// Other libraries on this device where the same person is (same member id).
    pub elsewhere: Vec<String>,
}

/// How a person can be invited: by email, or not at all. A link is for somebody new, and a person who is
/// already in the library is not new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Invitable {
    Email,
    None,
}

impl Invitable {
    fn of(role: Option<&str>, access_withdrawn: bool, has_account: bool) -> Self {
        let may = matches!(role, Some("reader" | "editor")) && !access_withdrawn && !has_account;
        if may { Self::Email } else { Self::None }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct People {
    /// The role of the person this device belongs to, if any.
    pub my_role: Option<String>,
    pub people: Vec<Person>,
    /// Devices that write to the library but belong to nobody: they count as readers.
    pub unassigned: Vec<PersonDevice>,
    /// Only one person administers the library: losing their devices would leave
    /// nobody who can change permissions (D58).
    pub lone_admin: bool,
    /// What reinstalling the app left behind: see [`earlier_installs`].
    pub earlier_installs: Vec<PersonDevice>,
}

pub fn list_people(conn: &Connection, library: &str) -> Result<People> {
    let me = ops::device_id(conn)?;
    let devices: Vec<(String, String, Option<String>, Option<String>)> = conn
        .prepare(&format!(
            "SELECT id, name, {person},
                    (SELECT v.hlc FROM field_versions v
                      WHERE v.library_id = d.library_id AND v.entity = 'devices'
                        AND v.entity_id = d.id AND v.field = 'member_id')
               FROM devices d WHERE library_id = ?1 ORDER BY name",
            person = permissions::person_of("d")
        ))?
        .query_map([library], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let cloud = permissions::SqlView::new(conn, library).cloud_device();
    let device = |id: &str, name: &str, assigned_at: Option<&str>| {
        // Who signed the operation that said whose it is: read from the field's version, no extra record.
        let author = assigned_at
            .and_then(|hlc| hlc.parse::<kosmotheka_domain::Hlc>().ok())
            .map(|hlc| hlc.device);
        let linked_by_cloud = author.is_some() && author == cloud;
        let linked_by = author
            .filter(|author| !linked_by_cloud && author != id)
            .and_then(|author| {
                let person = permissions::person_of_device(conn, library, &author).ok()??;
                conn.query_row(
                    "SELECT name FROM members WHERE library_id = ?1 AND id = ?2",
                    [library, &person],
                    |row| row.get(0),
                )
                .optional()
                .ok()?
            });
        PersonDevice {
            id: id.to_owned(),
            name: name.to_owned(),
            this_device: id == me,
            last_device_of_last_admin: false,
            linked_by_cloud,
            linked_by,
            last_change: last_change(conn, library, id).ok().flatten(),
        }
    };
    let mut people: Vec<Person> = conn
        .prepare(
            "SELECT id, name, role, can_share, coalesce(access_withdrawn, 0),
                    coalesce(account_id, '') <> '' FROM members
              WHERE library_id = ?1 ORDER BY name COLLATE NOCASE, id",
        )?
        .query_map([library], |row| {
            let role: Option<String> = row.get(2)?;
            let access_withdrawn = row.get::<_, i64>(4)? != 0;
            let has_account: bool = row.get(5)?;
            Ok(Person {
                id: row.get(0)?,
                name: row.get(1)?,
                invitable: Invitable::of(role.as_deref(), access_withdrawn, has_account),
                role,
                can_share: row.get::<_, Option<i64>>(3)?.unwrap_or(0) != 0,
                access_withdrawn,
                last_admin: false,
                has_account,
                devices: Vec::new(),
                elsewhere: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut unassigned = Vec::new();
    // The device the library names as its cloud is nobody's by rule (D76) and is never offered a person.
    let named_cloud = cloud_device_of(conn, library)?;
    for (id, name, member, assigned_at) in &devices {
        if member.is_none() && named_cloud.as_ref() == Some(id) {
            continue;
        }
        match people.iter_mut().find(|p| Some(&p.id) == member.as_ref()) {
            Some(person) => person
                .devices
                .push(device(id, name, assigned_at.as_deref())),
            None => unassigned.push(device(id, name, None)),
        }
    }
    // A withdrawn administrator no longer administers (D98).
    let lone_admin = people
        .iter()
        .filter(|p| p.role.as_deref() == Some("admin") && !p.access_withdrawn)
        .count()
        == 1;
    if let [only] = acting_admins(conn, library)?.as_slice() {
        for person in people.iter_mut().filter(|p| &p.id == only) {
            person.last_admin = true;
            if let [device] = person.devices.as_mut_slice() {
                device.last_device_of_last_admin = true;
            }
        }
    }
    for person in &mut people {
        person.elsewhere = libraries_of(conn, &person.id, library)?;
    }
    let my_role = people
        .iter()
        .find(|p| p.devices.iter().any(|d| d.this_device))
        .and_then(|p| p.role.clone());
    let earlier_installs = earlier_installs(conn, library)?
        .iter()
        .filter_map(|id| people.iter().flat_map(|p| &p.devices).find(|d| &d.id == id))
        .cloned()
        .collect();
    Ok(People {
        my_role,
        people,
        unassigned,
        lone_admin,
        earlier_installs,
    })
}

/// The time of the last operation `device` signed in `library`.
fn last_change(conn: &Connection, library: &str, device: &str) -> Result<Option<i64>> {
    let hlc: Option<String> = conn.query_row(
        "SELECT max(hlc) FROM operations WHERE library_id = ?1 AND device_id = ?2",
        [library, device],
        |row| row.get(0),
    )?;
    Ok(hlc
        .and_then(|hlc| hlc.parse::<kosmotheka_domain::Hlc>().ok())
        .and_then(|hlc| i64::try_from(hlc.millis).ok()))
}

/// The devices that look like earlier installs of this one: reinstalling the app makes a new device (D81), and the
/// old one stays with its person for good. Only devices of this device's own person, with its name, that have signed
/// nothing since this one first did: an install that is gone can sign nothing more, while a second device with the
/// same name that is still in use soon does.
pub fn earlier_installs(conn: &Connection, library: &str) -> Result<Vec<String>> {
    let me = ops::device_id(conn)?;
    let Some(person) = permissions::person_of_device(conn, library, &me)? else {
        return Ok(Vec::new());
    };
    Ok(conn
        .prepare(&format!(
            "SELECT d.id FROM devices d JOIN devices mine
                 ON mine.library_id = d.library_id AND mine.id = ?2
              WHERE d.library_id = ?1 AND d.id <> ?2 AND {person} = ?3
                AND lower(trim(d.name)) = lower(trim(mine.name))
                AND (SELECT max(hlc) FROM operations WHERE library_id = ?1 AND device_id = d.id)
                  < (SELECT min(hlc) FROM operations WHERE library_id = ?1 AND device_id = ?2)
              ORDER BY d.id",
            person = permissions::person_of("d")
        ))?
        .query_map([library, &me, &person], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// Removes, in every library, the earlier installs of this device that `library` shows (see [`remove_device`]).
pub fn remove_earlier_installs(conn: &mut Connection, library: &str) -> Result<usize> {
    let devices = earlier_installs(conn, library)?;
    for device in &devices {
        remove_device(conn, device)?;
    }
    Ok(devices.len())
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
    let (old_role, old_can_share): (Option<String>, bool) = tx
        .query_row(
            "SELECT role, COALESCE(can_share, 0) FROM members WHERE library_id = ?1 AND id = ?2",
            [library, member],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .unwrap_or((None, false));
    let mut changes = vec![Change::update(
        Entity::Member,
        member,
        json!({ "role": role, "can_share": can_share }),
    )];
    // Taking power away fences the person's devices, in the same operation (D84).
    if power_rank(role) < power_rank(old_role.as_deref()) || (old_can_share && !can_share) {
        let devices: Vec<String> = tx
            .prepare("SELECT id FROM devices WHERE library_id = ?1 AND member_id = ?2")?
            .query_map([library, member], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        changes.extend(crate::permissions::fence_changes(&tx, library, &devices)?);
    }
    ops::record(&tx, library, "member.role", changes)?;
    tx.commit()?;
    Ok(())
}

/// The device `library.cloud_device_id` names now, if any (D76, D95).
pub(crate) fn cloud_device_of(conn: &Connection, library: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT cloud_device_id FROM libraries WHERE id = ?1",
            [library],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .filter(|device| !device.is_empty()))
}

/// Names `device` as the cloud of `library` (D76, D95). Naming another one takes the
/// power from the old one, so the same operation fences it (D84).
#[cfg_attr(not(test), allow(dead_code))] // no command yet: the screen comes with 7.5.5
pub fn name_cloud_device(conn: &mut Connection, library: &str, device: &str) -> Result<()> {
    let tx = conn.transaction()?;
    let old = cloud_device_of(&tx, library)?;
    let mut changes = vec![Change::update(
        Entity::Library,
        library,
        json!({ "cloud_device_id": device }),
    )];
    if let Some(old) = old.filter(|old| old != device) {
        changes.push(permissions::fence_cloud_change(&tx, library, &old)?);
    }
    ops::record(&tx, library, "library.cloud", changes)?;
    tx.commit()?;
    Ok(())
}

/// Takes the cloud's power away: `cloud_device_id` is emptied and, in the same
/// operation, the cloud device is fenced (also if it is not registered here yet) with the newest operation of it this device
/// holds, so nothing it dated before and this device never saw is believed (D84, D87).
/// Does nothing if the library names no cloud.
#[cfg_attr(not(test), allow(dead_code))] // no command yet: the screen comes with 7.5.5
pub fn remove_cloud_device(conn: &mut Connection, library: &str) -> Result<()> {
    let tx = conn.transaction()?;
    let Some(old) = cloud_device_of(&tx, library)? else {
        return Ok(());
    };
    let mut changes = vec![Change::update(
        Entity::Library,
        library,
        json!({ "cloud_device_id": null }),
    )];
    changes.push(permissions::fence_cloud_change(&tx, library, &old)?);
    ops::record(&tx, library, "library.cloud", changes)?;
    tx.commit()?;
    Ok(())
}

fn power_rank(role: Option<&str>) -> u8 {
    match role {
        Some("admin") => 3,
        Some("editor") => 2,
        Some("reader") => 1,
        _ => 0,
    }
}

/// Says whose a device is, or that it is nobody's (`None`).
pub fn assign_device(
    conn: &mut Connection,
    library: &str,
    device: &str,
    member: Option<&str>,
) -> Result<()> {
    // The cloud with a person would be the server with that person's role, administrator included.
    if member.is_some() && cloud_device_of(conn, library)?.as_deref() == Some(device) {
        return Err(Error::NotAllowed);
    }
    let tx = conn.transaction()?;
    let current: Option<String> = tx
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, device],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let mut fields = permissions::assignment_fields(&tx, library, device, member)?;
    // A device that stops belonging to its person is fenced (D84).
    if current.as_deref().is_some_and(|c| Some(c) != member) {
        fields["trusted_through"] = json!(crate::permissions::fence_mark(&tx, library, device)?);
    }
    ops::record(
        &tx,
        library,
        "device.assign",
        vec![Change::update(Entity::Device, device, fields)],
    )?;
    tx.commit()?;
    Ok(())
}

/// Whether this device may take `library` to another device (D73): administrators and people with «Can
/// share». It is checked here, where the library is handed over; a modified app could still hand over what it
/// has, but the device it reaches belongs to nobody and only reads.
pub fn may_share(conn: &Connection, library: &str) -> Result<bool> {
    let people = list_people(conn, library)?;
    Ok(people
        .people
        .iter()
        .find(|p| p.devices.iter().any(|d| d.this_device))
        .is_some_and(|me| me.role.as_deref() == Some("admin") || me.can_share))
}

/// Whether this device's person administers `library`. Only they take it to the cloud (D76, CON-009): going up names a device the library trusts
/// with power, a decision for whoever administers it, not for whoever is merely in it.
pub fn administers(conn: &Connection, library: &str) -> Result<bool> {
    let people = list_people(conn, library)?;
    Ok(people.my_role.as_deref() == Some("admin"))
}

pub fn ensure_may_share(conn: &Connection, libraries: &[String]) -> Result<()> {
    for library in libraries {
        if !may_share(conn, library)? {
            return Err(Error::CannotShare);
        }
    }
    Ok(())
}

/// Who a newly paired device may belong to: the people of the libraries it
/// receives where this device may say whose it is (administrator).
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
        if listed.my_role.as_deref() != Some("admin") {
            continue;
        }
        // Somebody sent away is not offered a new device: they come back by being given the access first.
        for person in listed.people.into_iter().filter(|p| !p.access_withdrawn) {
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

/// Says whose a newly paired device is (D73): `person` joins each of the libraries the step showed (`shown`, from
/// `pairing_owners`) with the role given for it where they have none yet (a role is never inherited), and the device
/// becomes theirs in each one. Only those: a library that arrived while the step was on screen was never asked
/// about. The device may not have registered there yet, since only it can create its row: then the assignment waits
/// until its registration arrives (`apply_pending_owners`).
pub fn give_access(
    conn: &mut Connection,
    device: &str,
    person: &PersonPick,
    roles: &std::collections::BTreeMap<String, String>,
    can_share: bool,
    shown: &[String],
) -> Result<()> {
    let libraries: Vec<String> = conn
        .prepare("SELECT library_id FROM library_peers WHERE device_id = ?1")?
        .query_map([device], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?
        .into_iter()
        .filter(|library| shown.contains(library))
        .collect();
    let mut member_id = person.id.clone();
    let tx = conn.transaction()?;
    for library in &libraries {
        // Only where this device may say whose a device is.
        let listed = list_people(&tx, library)?;
        if listed.my_role.as_deref() != Some("admin") {
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
                permissions::assignment_fields(&tx, library, device, Some(&member))?,
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
        let fields = permissions::assignment_fields(tx, &library, &device, Some(&member))?;
        let change = Change::update(Entity::Device, &device, fields);
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

pub(crate) fn is_registered(conn: &Connection, library: &str, device: &str) -> Result<bool> {
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
    // Not the last device of the last administrator who can act (SEG-036), in any library.
    for (library, _) in &libraries {
        let Some(person) = permissions::person_of_device(conn, library, device)? else {
            continue;
        };
        let their_devices: i64 = conn.query_row(
            &format!(
                "SELECT count(*) FROM devices d WHERE d.library_id = ?1 AND {} = ?2",
                permissions::person_of("d")
            ),
            [library, &person],
            |row| row.get(0),
        )?;
        if their_devices == 1 && is_last_admin(conn, library, &person)? {
            return Err(Error::LastAdmin);
        }
    }
    let tx = conn.transaction()?;
    for (library, registered_name) in &libraries {
        let mut changes = vec![Change::update(
            Entity::Device,
            device,
            json!({ "member_id": null, "vouched_key": null }),
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

/// Echar a una persona (D98): their access is withdrawn and they stay in the catalog. **One** operation
/// (`member.withdraw`) writes the mark, and for each device of theirs takes the person away (with the
/// vouched key), fences it (D84, D87) and withdraws its access (D71), so those devices remove the
/// library when they next connect, over the local network or through the cloud. Not the last
/// administrator, and not the person this device belongs to.
pub fn withdraw_person(conn: &mut Connection, library: &str, member: &str) -> Result<()> {
    let me = ops::device_id(conn)?;
    if permissions::person_of_device(conn, library, &me)?.as_deref() == Some(member) {
        return Err(Error::NotAllowed);
    }
    if is_last_admin(conn, library, member)? {
        return Err(Error::LastAdmin);
    }
    let tx = conn.transaction()?;
    let devices: Vec<(String, String)> = tx
        .prepare(
            "SELECT id, name FROM devices WHERE library_id = ?1 AND member_id = ?2 ORDER BY id",
        )?
        .query_map([library, member], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut changes = vec![Change::update(
        Entity::Member,
        member,
        json!({ "access_withdrawn": true }),
    )];
    changes.extend(permissions::cut_off_changes(
        &tx,
        library,
        &devices,
        |_| true,
    )?);
    // A device paired and given to this person that has not registered yet has no row in
    // `devices`, only a waiting assignment (SEG-035): it is cut off the same way, with nothing
    // to fence since it has written nothing.
    let waiting: Vec<String> = tx
        .prepare(
            "SELECT device_id FROM pending_owners WHERE library_id = ?1 AND member_id = ?2
              ORDER BY device_id",
        )?
        .query_map([library, member], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for device in &waiting {
        let name = lan::known_device_name(&tx, device)?.unwrap_or_default();
        changes.extend(libraries::revocation_change(&tx, library, device, &name)?);
    }
    ops::record(&tx, library, "member.withdraw", changes)?;
    for device in devices.iter().map(|(id, _)| id).chain(&waiting) {
        libraries::forget_peer(&tx, library, device)?;
    }
    tx.execute(
        "DELETE FROM pending_owners WHERE library_id = ?1 AND member_id = ?2",
        [library, member],
    )?;
    tx.commit()?;
    Ok(())
}

/// Leaving the library for good (D102, rule 15): this device's person withdraws their own access. One operation
/// (`member.leave`) writes the mark and, with the same changes as expelling (`cut_off_changes`), takes the person away
/// from each of their devices and withdraws their access; only this device, the signing one, fences itself, at its
/// previous operation, so nothing it did is disbelieved. The other devices are not fenced: they may hold work no
/// device has seen, and others may have built on it.
///
/// The library does not vanish here at once: the operation has to reach another device first (it is not anywhere
/// else yet), and then the usual way takes over: whoever receives it sends this device its revocation, with the
/// proof, and it removes the library (`lan::remove_if_proved`, `cloud::cut_off`). Not the last administrator.
pub fn leave_for_good(conn: &mut Connection, library: &str) -> Result<()> {
    let me = ops::device_id(conn)?;
    let Some(person) = permissions::person_of_device(conn, library, &me)? else {
        return Err(Error::NotAllowed);
    };
    if is_last_admin(conn, library, &person)? {
        return Err(Error::LastAdmin);
    }
    let tx = conn.transaction()?;
    let devices: Vec<(String, String)> = tx
        .prepare(
            "SELECT id, name FROM devices WHERE library_id = ?1 AND member_id = ?2 ORDER BY id",
        )?
        .query_map([library, &person], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let mut changes = vec![Change::update(
        Entity::Member,
        &person,
        json!({ "access_withdrawn": true }),
    )];
    changes.extend(permissions::cut_off_changes(
        &tx,
        library,
        &devices,
        |device| device == me,
    )?);
    ops::record(&tx, library, "member.leave", changes)?;
    tx.commit()?;
    Ok(())
}

/// Gives the access back: only removes the mark (D98). The devices stay without their person and
/// the revocations stay; whoever administers links a device again, the usual way.
pub fn restore_person(conn: &mut Connection, library: &str, member: &str) -> Result<()> {
    let tx = conn.transaction()?;
    // Devices the withdrawal did not name still point at the person; cut them off first, in the
    // same operation, or removing the mark would give them their access back (SEG-033).
    let strays = permissions::devices_of_withdrawn(&tx, library, Some(member))?;
    let mut changes = permissions::cut_off_changes(&tx, library, &strays, |_| true)?;
    changes.push(Change::update(
        Entity::Member,
        member,
        json!({ "access_withdrawn": false }),
    ));
    ops::record(&tx, library, "member.restore", changes)?;
    for (device, _) in &strays {
        libraries::forget_peer(&tx, library, device)?;
    }
    tx.commit()?;
    Ok(())
}

/// The administrators who can act (D58, SEG-036): with access, and with at least one device that is
/// theirs. One with no device cannot sign anything, so if the others were taken away nobody could
/// change permissions again.
fn acting_admins(conn: &Connection, library: &str) -> Result<Vec<String>> {
    Ok(conn
        .prepare(&format!(
            "SELECT m.id FROM members m
              WHERE m.library_id = ?1 AND m.role = 'admin' AND coalesce(m.access_withdrawn, 0) = 0
                AND EXISTS (SELECT 1 FROM devices d WHERE d.library_id = m.library_id
                               AND {} = m.id)",
            permissions::person_of("d")
        ))?
        .query_map([library], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// Whether `member` is the only administrator who can act.
fn is_last_admin(conn: &Connection, library: &str, member: &str) -> Result<bool> {
    Ok(matches!(acting_admins(conn, library)?.as_slice(), [only] if only == member))
}

/// For tests: names the person this device founded `library` as (the welcome gives that name, D97), and returns
/// its id.
#[cfg(test)]
pub(crate) fn name_founder(conn: &mut Connection, library: &str, name: &str) -> Result<String> {
    let me = ops::device_id(conn)?;
    let member: String = conn.query_row(
        "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
        [library, &me],
        |row| row.get(0),
    )?;
    let tx = conn.transaction()?;
    ops::record(
        &tx,
        library,
        "member.rename",
        vec![Change::update(
            Entity::Member,
            &member,
            json!({ "name": name }),
        )],
    )?;
    tx.commit()?;
    Ok(member)
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};

    use super::*;
    use crate::backup;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::ops;
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

    /// Everything `from` has, merged into `to`, as a sync would.
    fn pull(to: &mut Connection, from: &Connection, library: &str) {
        let libraries = vec![library.to_owned()];
        let mine = sync::version_vectors(to, &libraries).unwrap();
        let missing: Vec<SyncOperation> = sync::operations_since(from, &mine).unwrap();
        sync::merge(to, missing).unwrap();
    }

    /// The old name of the helper that named the founder, for the tests that used the activation.
    fn start_roles(
        conn: &mut Connection,
        library: &str,
        name: &str,
        _owners: &[()],
    ) -> Result<String> {
        name_founder(conn, library, name)
    }

    #[test]
    fn a_library_is_founded_with_its_creator_as_the_administrator() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("Dune"), None).unwrap();

        let people = list_people(&conn, &lib).unwrap();

        assert_eq!(people.my_role.as_deref(), Some("admin"));
        let me = people.people.iter().find(|p| p.can_share).unwrap();
        assert_eq!(me.role.as_deref(), Some("admin"));
        assert!(people.lone_admin, "one administrator with one device");
    }

    #[test]
    fn the_last_administrator_stays_one() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let member = start_roles(&mut conn, &lib, "Tersoal", &[]).unwrap();

        let result = set_role(&mut conn, &lib, &member, Some("editor"), false);

        assert!(matches!(result, Err(Error::LastAdmin)));
    }

    /// An administrator nobody has a device of (and one who has been withdrawn) does not count as one who
    /// could take over: the screens ask Rust who the last one is instead of counting people.
    fn with_a_ghost_administrator() -> (Connection, String, String, String) {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let me = start_roles(&mut conn, &lib, "Tersoal", &[]).unwrap();
        let ghost = new_id();
        let tx = conn.transaction().unwrap();
        ops::record(
            &tx,
            &lib,
            "member.add",
            vec![Change::create(
                Entity::Member,
                &ghost,
                json!({ "name": "Ghost", "role": "admin" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        (conn, lib, me, ghost)
    }

    #[test]
    fn the_last_administrator_who_can_act_is_marked_with_her_last_device() {
        let (conn, lib, me, ghost) = with_a_ghost_administrator();

        let listed = list_people(&conn, &lib).unwrap();

        let person = |id: &str| listed.people.iter().find(|p| p.id == id).unwrap();
        assert!(person(&me).last_admin, "the ghost cannot sign anything");
        assert!(!person(&ghost).last_admin);
        assert_eq!(person(&me).devices.len(), 1);
        assert!(person(&me).devices[0].last_device_of_last_admin);
        assert!(!listed.people.iter().all(|p| p.last_admin));
    }

    #[test]
    fn a_withdrawn_administrator_neither_counts_for_the_warning_nor_is_offered_a_device() {
        let (mut conn, lib, _me, ghost) = with_a_ghost_administrator();
        assert!(!list_people(&conn, &lib).unwrap().lone_admin);
        let phone = db::open_in_memory().unwrap();
        let phone_id = ops::device_id(&phone).unwrap();
        lan::share_library(&conn, &lib, &phone_id).unwrap();
        assert_eq!(pairing_owners(&conn, &phone_id).unwrap().people.len(), 2);

        withdraw_person(&mut conn, &lib, &ghost).unwrap();

        assert!(list_people(&conn, &lib).unwrap().lone_admin);
        let owners = pairing_owners(&conn, &phone_id).unwrap();
        assert_eq!(owners.people.len(), 1);
        assert!(owners.people[0].me);
    }

    #[test]
    fn a_lost_phone_is_removed_and_what_it_still_sends_becomes_a_request() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        let lib = ops::active_library(&laptop).unwrap();
        // The library is the laptop's; the phone receives it and registers before it is given a place.
        pull(&mut phone, &laptop, &lib);
        let tx = phone.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        pull(&mut laptop, &phone, &lib);
        let member = sync::own_person(&laptop, &lib);
        let phone_id = ops::device_id(&phone).unwrap();
        assign_device(&mut laptop, &lib, &phone_id, Some(&member)).unwrap();
        pull(&mut phone, &laptop, &lib);
        catalog::create_book(&mut phone, &lib, &book("Tehanu"), None).unwrap();
        pull(&mut laptop, &phone, &lib);
        assert!(
            list_people(&laptop, &lib).unwrap().lone_admin,
            "one person administers it, however many devices she has"
        );

        assert_eq!(remove_device(&mut laptop, &phone_id).unwrap(), 1);
        // Whoever has the phone now keeps writing, and it reaches the laptop somehow.
        catalog::create_book(&mut phone, &lib, &book("Stolen"), None).unwrap();
        pull(&mut laptop, &phone, &lib);

        assert!(!titles(&laptop, &lib).contains(&"Stolen".to_owned()));
        let requests = crate::requests::list_requests(&laptop, &lib).unwrap();
        assert_eq!(requests.len(), 1);
        let people = list_people(&laptop, &lib).unwrap();
        assert_eq!(people.unassigned.len(), 1);
        assert!(lan::revocations_of(&laptop, &lib, &phone_id).unwrap().len() == 1);
    }

    /// Laptop (administrator), a phone that is the cloud, and Laura, a person to link an account to.
    fn laptop_with_a_cloud_phone() -> (Connection, Connection, String, String) {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        permissions::become_a_cloud_device(&phone, 7);
        let lib = ops::active_library(&laptop).unwrap();
        pull(&mut phone, &laptop, &lib);
        // The cloud registers itself as a device of the library, and the laptop gets
        // that registration before naming it.
        let tx = phone.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        pull(&mut laptop, &phone, &lib);
        let admin = start_roles(&mut laptop, &lib, "Tersoal", &[]).unwrap();
        let phone_id = ops::device_id(&phone).unwrap();
        let mut changes = Vec::new();
        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let laura =
            member_changes(&laptop, &lib, &laura, Some("editor"), None, &mut changes).unwrap();
        let tx = laptop.transaction().unwrap();
        ops::record(&tx, &lib, "member.add", changes).unwrap();
        tx.commit().unwrap();
        assert_ne!(admin, laura);
        name_cloud_device(&mut laptop, &lib, &phone_id).unwrap();
        pull(&mut phone, &laptop, &lib);
        (laptop, phone, lib, laura)
    }

    #[test]
    fn the_cloud_is_not_a_device_of_nobody_waiting_for_a_person() {
        let (laptop, phone, lib, _) = laptop_with_a_cloud_phone();
        let cloud = ops::device_id(&phone).unwrap();
        let people = list_people(&laptop, &lib).unwrap();
        assert!(
            people.unassigned.iter().all(|device| device.id != cloud),
            "the cloud is shown apart, never with «Assign»"
        );
    }

    #[test]
    fn the_cloud_is_never_given_a_person() {
        // Giving the cloud a person would give the server that person's role, administrator included.
        let (mut laptop, phone, lib, laura) = laptop_with_a_cloud_phone();
        let cloud = ops::device_id(&phone).unwrap();
        let admin = sync::own_person(&laptop, &lib);
        for person in [&admin, &laura] {
            assert!(matches!(
                assign_device(&mut laptop, &lib, &cloud, Some(person)),
                Err(Error::NotAllowed)
            ));
        }
        assert_eq!(
            permissions::person_of_device(&laptop, &lib, &cloud).unwrap(),
            None
        );
    }

    const LAPTOP: &str = "sergio-PROX-AMD5";

    /// A new install of the app that joins `lib` and is given to `member` by `admin`, the way the cloud links a
    /// device that signs in with its person's account (D76): the same `member_id` and `vouched_key`.
    fn new_install(admin: &mut Connection, lib: &str, name: &str, member: &str) -> Connection {
        let mut install = db::open_in_memory().unwrap();
        lan::set_device_name(&mut install, name).unwrap();
        pull(&mut install, admin, lib);
        let tx = install.transaction().unwrap();
        ops::ensure_registered(&tx, lib).unwrap();
        tx.commit().unwrap();
        pull(admin, &install, lib);
        let id = ops::device_id(&install).unwrap();
        assign_device(admin, lib, &id, Some(member)).unwrap();
        pull(&mut install, admin, lib);
        install
    }

    /// The maintainer's case (25/10/2026): Sergio founds the library on his laptop, adds his phone, and installs
    /// the app on the same laptop twice more. Each install is a new device (D81). Returns the phone, the third
    /// install, the library, Sergio and the two earlier installs.
    fn a_laptop_installed_three_times() -> (Connection, Connection, String, String, [String; 2]) {
        let mut first = db::open_in_memory().unwrap();
        lan::set_device_name(&mut first, LAPTOP).unwrap();
        let lib = ops::active_library(&first).unwrap();
        let sergio = name_founder(&mut first, &lib, "Sergio").unwrap();
        let mut phone = new_install(&mut first, &lib, "Sergio android", &sergio);
        catalog::create_book(&mut first, &lib, &book("Dune"), None).unwrap();
        pull(&mut phone, &first, &lib);
        let first_id = ops::device_id(&first).unwrap();
        drop(first);

        let mut second = new_install(&mut phone, &lib, LAPTOP, &sergio);
        catalog::create_book(&mut second, &lib, &book("Tehanu"), None).unwrap();
        pull(&mut phone, &second, &lib);
        let second_id = ops::device_id(&second).unwrap();
        drop(second);

        let third = new_install(&mut phone, &lib, LAPTOP, &sergio);
        (phone, third, lib, sergio, [first_id, second_id])
    }

    #[test]
    fn reinstalling_leaves_the_earlier_installs_with_their_person() {
        let (_phone, third, lib, sergio, _) = a_laptop_installed_three_times();

        let people = list_people(&third, &lib).unwrap();

        let sergio = people.people.iter().find(|p| p.id == sergio).unwrap();
        let names: Vec<&str> = sergio.devices.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names.iter().filter(|n| **n == LAPTOP).count(),
            3,
            "{names:?}"
        );
        assert_eq!(sergio.devices.len(), 4, "three installs and the phone");
    }

    #[test]
    fn the_new_install_is_offered_its_earlier_ones_with_their_last_change() {
        let (_phone, third, lib, _, earlier) = a_laptop_installed_three_times();

        let people = list_people(&third, &lib).unwrap();

        let mut offered: Vec<&str> = people
            .earlier_installs
            .iter()
            .map(|d| d.id.as_str())
            .collect();
        offered.sort_unstable();
        let mut expected: Vec<&str> = earlier.iter().map(String::as_str).collect();
        expected.sort_unstable();
        assert_eq!(offered, expected);
        let me = ops::device_id(&third).unwrap();
        let mine = people
            .people
            .iter()
            .flat_map(|p| &p.devices)
            .find(|d| d.id == me)
            .unwrap();
        for device in &people.earlier_installs {
            assert!(device.last_change.unwrap() <= mine.last_change.unwrap());
        }
    }

    #[test]
    fn a_device_of_someone_else_with_the_same_name_is_never_offered() {
        // Hostile: Laura names her laptop like Sergio's so that he removes it.
        let (mut phone, mut third, lib, _, _) = a_laptop_installed_three_times();
        let mut changes = Vec::new();
        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let laura =
            member_changes(&phone, &lib, &laura, Some("editor"), None, &mut changes).unwrap();
        let tx = phone.transaction().unwrap();
        ops::record(&tx, &lib, "member.add", changes).unwrap();
        tx.commit().unwrap();
        let lauras = new_install(&mut phone, &lib, LAPTOP, &laura);
        // And a device of nobody, with that name too.
        let mut nobodys = db::open_in_memory().unwrap();
        lan::set_device_name(&mut nobodys, LAPTOP).unwrap();
        pull(&mut nobodys, &phone, &lib);
        let tx = nobodys.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        pull(&mut phone, &nobodys, &lib);
        pull(&mut third, &phone, &lib);

        let offered = earlier_installs(&third, &lib).unwrap();

        assert!(!offered.contains(&ops::device_id(&lauras).unwrap()));
        assert!(!offered.contains(&ops::device_id(&nobodys).unwrap()));
        assert!(!offered.contains(&ops::device_id(&third).unwrap()));
    }

    #[test]
    fn a_second_device_of_mine_still_in_use_is_never_offered() {
        // Two laptops of Sergio's with the same name: the older one keeps working after the newer came.
        let (mut phone, _, lib, sergio, _) = a_laptop_installed_three_times();
        let mut older = new_install(&mut phone, &lib, LAPTOP, &sergio);
        let mut newer = new_install(&mut phone, &lib, LAPTOP, &sergio);
        pull(&mut older, &phone, &lib);
        catalog::create_book(&mut older, &lib, &book("Still here"), None).unwrap();
        pull(&mut phone, &older, &lib);
        pull(&mut newer, &phone, &lib);

        let offered = earlier_installs(&newer, &lib).unwrap();

        assert!(!offered.contains(&ops::device_id(&older).unwrap()));
    }

    #[test]
    fn removing_the_earlier_installs_leaves_this_one_and_the_phone() {
        let (mut phone, mut third, lib, sergio, earlier) = a_laptop_installed_three_times();

        assert_eq!(remove_earlier_installs(&mut third, &lib).unwrap(), 2);
        pull(&mut phone, &third, &lib);

        for conn in [&third, &phone] {
            let people = list_people(conn, &lib).unwrap();
            let sergio = people.people.iter().find(|p| p.id == sergio).unwrap();
            assert_eq!(sergio.devices.len(), 2, "this install and the phone");
            assert!(people.earlier_installs.is_empty());
            for device in &earlier {
                assert_eq!(lan::revocations_of(conn, &lib, device).unwrap().len(), 1);
            }
        }
    }

    #[test]
    fn only_an_administrator_removes_her_earlier_installs_today() {
        // Removing a device is an administrator's (D58): an editor is shown hers, and the rule that would let her
        // remove them is still to decide.
        let (mut phone, _, lib, _, _) = a_laptop_installed_three_times();
        let mut changes = Vec::new();
        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let laura =
            member_changes(&phone, &lib, &laura, Some("editor"), None, &mut changes).unwrap();
        let tx = phone.transaction().unwrap();
        ops::record(&tx, &lib, "member.add", changes).unwrap();
        tx.commit().unwrap();
        let old = new_install(&mut phone, &lib, "Laura laptop", &laura);
        let old_id = ops::device_id(&old).unwrap();
        drop(old);
        let mut new = new_install(&mut phone, &lib, "Laura laptop", &laura);

        assert_eq!(earlier_installs(&new, &lib).unwrap(), vec![old_id.clone()]);
        assert!(matches!(
            remove_earlier_installs(&mut new, &lib),
            Err(Error::NotAllowed)
        ));
        assert!(lan::revocations_of(&new, &lib, &old_id).unwrap().is_empty());
    }

    fn write_account(conn: &mut Connection, lib: &str, member: &str) -> String {
        let tx = conn.transaction().unwrap();
        let operation = ops::record(
            &tx,
            lib,
            "member.account",
            vec![Change::update(
                Entity::Member,
                member,
                json!({ "account_id": "01997c7e-0000-7000-8000-0000000000a1" }),
            )],
        )
        .unwrap();
        tx.commit().unwrap();
        operation.id
    }

    #[test]
    fn the_cloud_device_writes_an_account_and_no_one_else_does() {
        let (mut laptop, mut phone, lib, laura) = laptop_with_a_cloud_phone();

        let by_the_cloud = write_account(&mut phone, &lib, &laura);
        pull(&mut laptop, &phone, &lib);
        assert_eq!(
            ops::status_of(&laptop, &by_the_cloud).unwrap(),
            ops::Status::Applied
        );

        // An administrator may change the library row, but not this field of a person.
        let tx = laptop.transaction().unwrap();
        let refused = ops::record(
            &tx,
            &lib,
            "member.account",
            vec![Change::update(
                Entity::Member,
                &laura,
                json!({ "account_id": "01997c7e-0000-7000-8000-0000000000a2" }),
            )],
        );
        assert!(matches!(refused, Err(Error::NotAllowed)));
    }

    #[test]
    fn taking_the_cloud_away_fences_it_and_what_it_dated_before_is_not_believed() {
        let (mut laptop, mut phone, lib, laura) = laptop_with_a_cloud_phone();
        let known = write_account(&mut phone, &lib, &laura);
        pull(&mut laptop, &phone, &lib);

        // The cloud writes once more; the laptop never hears of it before removing the power.
        let unseen = write_account(&mut phone, &lib, &laura);
        remove_cloud_device(&mut laptop, &lib).unwrap();
        pull(&mut laptop, &phone, &lib);

        assert_eq!(
            ops::status_of(&laptop, &known).unwrap(),
            ops::Status::Applied
        );
        assert_eq!(
            ops::status_of(&laptop, &unseen).unwrap(),
            ops::Status::Request
        );
        assert_eq!(cloud_device_of(&laptop, &lib).unwrap(), None);
        // Emptying it twice writes nothing the second time.
        let before = ops::latest_of(&laptop, &lib, &ops::device_id(&laptop).unwrap()).unwrap();
        remove_cloud_device(&mut laptop, &lib).unwrap();
        assert_eq!(
            ops::latest_of(&laptop, &lib, &ops::device_id(&laptop).unwrap()).unwrap(),
            before
        );
    }

    #[test]
    fn taking_the_power_from_a_cloud_never_registered_here_fences_it_all_the_same() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut cloud = db::open_in_memory().unwrap();
        permissions::become_a_cloud_device(&cloud, 7);
        let lib = ops::active_library(&laptop).unwrap();
        let admin = start_roles(&mut laptop, &lib, "Tersoal", &[]).unwrap();
        assert_ne!(admin, "");
        let mut changes = Vec::new();
        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let laura =
            member_changes(&laptop, &lib, &laura, Some("editor"), None, &mut changes).unwrap();
        let tx = laptop.transaction().unwrap();
        ops::record(&tx, &lib, "member.add", changes).unwrap();
        tx.commit().unwrap();
        // The library names the cloud before the cloud has written anything in it.
        let cloud_id = ops::device_id(&cloud).unwrap();
        name_cloud_device(&mut laptop, &lib, &cloud_id).unwrap();
        pull(&mut cloud, &laptop, &lib);
        let tx = cloud.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        let account = write_account(&mut cloud, &lib, &laura);

        // The administrator takes the power away without having seen any of it.
        remove_cloud_device(&mut laptop, &lib).unwrap();
        pull(&mut laptop, &cloud, &lib);

        assert_eq!(
            ops::status_of(&laptop, &account).unwrap(),
            ops::Status::Request
        );
    }

    #[test]
    fn naming_another_cloud_fences_the_old_one() {
        let (mut laptop, mut phone, lib, laura) = laptop_with_a_cloud_phone();
        let mut tablet = db::open_in_memory().unwrap();
        pull(&mut tablet, &laptop, &lib);
        let tx = tablet.transaction().unwrap();
        ops::ensure_registered(&tx, &lib).unwrap();
        tx.commit().unwrap();
        pull(&mut laptop, &tablet, &lib);
        let unseen = write_account(&mut phone, &lib, &laura);

        name_cloud_device(&mut laptop, &lib, &ops::device_id(&tablet).unwrap()).unwrap();
        pull(&mut laptop, &phone, &lib);

        assert_eq!(
            ops::status_of(&laptop, &unseen).unwrap(),
            ops::Status::Request
        );
    }

    #[test]
    fn a_duplicate_has_the_books_and_its_maker_administers_it() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("Dune"), None).unwrap();

        let copy = backup::duplicate(&mut conn, &lib, "Casa (copia)").unwrap();

        assert_eq!(titles(&conn, &copy), vec!["Dune"]);
        assert_eq!(
            list_people(&conn, &copy).unwrap().my_role.as_deref(),
            Some("admin")
        );
        assert_eq!(ops::active_library(&conn).unwrap(), copy);
        catalog::create_book(&mut conn, &copy, &book("Tehanu"), None).unwrap();
    }

    #[test]
    fn a_second_administrator_ends_the_warning() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        start_roles(&mut conn, &lib, "Tersoal", &[]).unwrap();
        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let mut changes = Vec::new();
        let id = member_changes(&conn, &lib, &laura, Some("admin"), None, &mut changes).unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(&tx, &lib, "member.add", changes).unwrap();
        tx.commit().unwrap();

        let people = list_people(&conn, &lib).unwrap();

        assert!(!people.lone_admin);
        assert!(people.people.iter().any(|p| p.id == id));
    }

    #[test]
    fn the_same_person_joins_another_library_with_the_same_id() {
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let hugo = PersonPick {
            id: None,
            name: "Hugo".into(),
        };
        let mut changes = Vec::new();
        let id = member_changes(&conn, &lib, &hugo, Some("reader"), None, &mut changes).unwrap();
        let tx = conn.transaction().unwrap();
        ops::record(&tx, &lib, "member.add", changes).unwrap();
        tx.commit().unwrap();
        let games = crate::libraries::create_library(&mut conn, "Juegos").unwrap();

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
    fn only_administrators_and_people_who_may_share_take_a_library_elsewhere() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        let lib = ops::active_library(&laptop).unwrap();
        start_roles(&mut laptop, &lib, "Ana", &[]).unwrap();
        assert!(may_share(&laptop, &lib).unwrap(), "an administrator");

        let phone_id = ops::device_id(&phone).unwrap();
        let laptop_id = ops::device_id(&laptop).unwrap();
        lan::share_library(&laptop, &lib, &phone_id).unwrap();
        lan::share_library(&phone, &lib, &laptop_id).unwrap();
        pull(&mut phone, &laptop, &lib);
        lan::register_in_shared(&mut phone, &laptop_id).unwrap();
        pull(&mut laptop, &phone, &lib);
        assert!(!may_share(&phone, &lib).unwrap(), "a device of nobody");
        assert!(matches!(
            crate::libraries::grant_access(&mut phone, &lib, "tablet"),
            Err(Error::CannotShare)
        ));

        let laura = PersonPick {
            id: None,
            name: "Laura".into(),
        };
        let roles = std::collections::BTreeMap::from([(lib.clone(), "editor".to_owned())]);
        give_access(
            &mut laptop,
            &phone_id,
            &laura,
            &roles,
            false,
            std::slice::from_ref(&lib),
        )
        .unwrap();
        pull(&mut phone, &laptop, &lib);
        assert!(
            !may_share(&phone, &lib).unwrap(),
            "an editor without «Can share»"
        );
        let member = list_people(&laptop, &lib)
            .unwrap()
            .people
            .into_iter()
            .find(|p| p.name == "Laura")
            .unwrap();
        set_role(&mut laptop, &lib, &member.id, Some("editor"), true).unwrap();
        pull(&mut phone, &laptop, &lib);
        assert!(may_share(&phone, &lib).unwrap(), "an editor who may share");
    }

    #[test]
    fn pairing_offers_only_libraries_that_this_device_administers() {
        let mut laptop = db::open_in_memory().unwrap();
        let phone = db::open_in_memory().unwrap();
        let lib = ops::active_library(&laptop).unwrap();
        // Another device founds «Juegos»; the laptop only reads it.
        let mut other = db::open_in_memory().unwrap();
        let games = crate::libraries::create_library(&mut other, "Juegos").unwrap();
        sync::share_as(&other, &games, &mut laptop, "Ana", "reader");
        let phone_id = ops::device_id(&phone).unwrap();
        lan::share_library(&laptop, &lib, &phone_id).unwrap();
        lan::share_library(&laptop, &games, &phone_id).unwrap();

        let owners = pairing_owners(&laptop, &phone_id).unwrap();
        assert_eq!(owners.libraries, vec![lib.clone()]);
        assert_eq!(owners.people.len(), 1);
        assert!(owners.people[0].me);
        assert_eq!(owners.people[0].roles[&lib], "admin");

        let roles = std::collections::BTreeMap::from([(lib.clone(), "reader".to_owned())]);
        let hugo = PersonPick {
            id: None,
            name: "Hugo".into(),
        };
        give_access(
            &mut laptop,
            &phone_id,
            &hugo,
            &roles,
            false,
            &owners.libraries,
        )
        .unwrap();
    }

    /// Seen pairing a computer and a phone, 24/10/2026: while the computer asked whose the phone is, the phone's
    /// own library arrived, and the phone had made the computer an administrator in it. Giving access then reached
    /// that library too, where the chosen person did not exist and no role had been asked for: «not allowed», and
    /// the whole assignment was undone, also in the library on screen.
    #[test]
    fn giving_access_touches_only_the_libraries_the_step_showed() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        let lib = ops::active_library(&laptop).unwrap();
        start_roles(&mut laptop, &lib, "Sergio", &[]).unwrap();
        let phone_id = ops::device_id(&phone).unwrap();
        lan::share_library(&laptop, &lib, &phone_id).unwrap();
        let owners = pairing_owners(&laptop, &phone_id).unwrap();
        assert_eq!(owners.libraries, vec![lib.clone()]);
        let me = owners.people.iter().find(|p| p.me).unwrap().id.clone();

        let rol = crate::libraries::create_library(&mut phone, "Rol").unwrap();
        sync::share_as(&phone, &rol, &mut laptop, "Sergio", "admin");
        lan::share_library(&laptop, &rol, &phone_id).unwrap();
        let people_in_rol = list_people(&laptop, &rol).unwrap().people.len();

        give_access(
            &mut laptop,
            &phone_id,
            &PersonPick {
                id: Some(me.clone()),
                name: "Sergio".into(),
            },
            &std::collections::BTreeMap::new(),
            false,
            &owners.libraries,
        )
        .unwrap();

        let waiting: Vec<(String, String)> = laptop
            .prepare("SELECT library_id, member_id FROM pending_owners WHERE device_id = ?1")
            .unwrap()
            .query_map([&phone_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            waiting,
            vec![(lib.clone(), me)],
            "the phone is mine where I was asked"
        );
        assert_eq!(
            list_people(&laptop, &rol).unwrap().people.len(),
            people_in_rol,
            "nobody is added to a library the step did not show"
        );
    }

    #[test]
    fn a_new_device_becomes_its_person_s_once_it_registers() {
        let mut laptop = db::open_in_memory().unwrap();
        let mut phone = db::open_in_memory().unwrap();
        let lib = ops::active_library(&laptop).unwrap();
        start_roles(&mut laptop, &lib, "Ana", &[]).unwrap();
        let phone_id = ops::device_id(&phone).unwrap();
        lan::share_library(&laptop, &lib, &phone_id).unwrap();
        let roles = std::collections::BTreeMap::from([(lib.clone(), "editor".to_owned())]);

        give_access(
            &mut laptop,
            &phone_id,
            &PersonPick {
                id: None,
                name: "Laura".into(),
            },
            &roles,
            false,
            std::slice::from_ref(&lib),
        )
        .unwrap();
        assert!(
            list_people(&laptop, &lib).unwrap().unassigned.is_empty(),
            "not registered yet"
        );

        // Over the network, the phone registers as soon as the library arrives.
        let laptop_id = ops::device_id(&laptop).unwrap();
        lan::share_library(&phone, &lib, &laptop_id).unwrap();
        pull(&mut phone, &laptop, &lib);
        lan::register_in_shared(&mut phone, &laptop_id).unwrap();
        pull(&mut laptop, &phone, &lib);
        pull(&mut phone, &laptop, &lib);

        let people = list_people(&laptop, &lib).unwrap();
        let laura = people.people.iter().find(|p| p.name == "Laura").unwrap();
        assert_eq!(laura.devices[0].id, phone_id);
        assert_eq!(
            list_people(&phone, &lib).unwrap().my_role.as_deref(),
            Some("editor")
        );
        catalog::create_book(&mut phone, &lib, &book("Tehanu"), None).unwrap();
    }
}
