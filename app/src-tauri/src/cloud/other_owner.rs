//! «This device is somebody else's» (D76, D96). `claim` names the libraries where the account is already
//! one person while this device belongs to another. The app only tells the person: it does not know which
//! person the account is (the server never gives that), so it cannot ask an administrator to change it, and
//! this device may not change it itself (rule 3 of `spec/README.md`). What is kept here is local state, never
//! an operation: the question, and whether it was answered.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ops::{read_state, write_state};
use crate::{account, people, permissions};

const PREFIX: &str = "cloud_other_owner.";

/// What is kept for one library: which account was signed in and whose the device was when it was seen,
/// so the question comes back only if either changes.
#[derive(Serialize, Deserialize)]
struct Seen {
    account: String,
    person: String,
    answered: bool,
}

/// One library where the question is open.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherOwner {
    pub library: String,
    pub library_name: String,
    /// The person this device belongs to there.
    pub person: String,
    pub role: Option<String>,
}

fn key(library: &str) -> String {
    format!("{PREFIX}{library}")
}

fn read(conn: &Connection, library: &str) -> Result<Option<Seen>> {
    Ok(read_state(conn, &key(library))?.and_then(|raw| serde_json::from_str(&raw).ok()))
}

/// The signed-in account, or empty when there is none to tell (it is then one account, as far as this goes).
fn account_now(conn: &Connection) -> Result<String> {
    Ok(account::signed_in_user_id(conn)?.unwrap_or_default())
}

/// A claim said the account is another person in `library`. Keeps the answer if it is the same question.
pub fn note(conn: &Connection, library: &str) -> Result<()> {
    let me = crate::ops::device_id(conn)?;
    let Some(person) = permissions::person_of_device(conn, library, &me)? else {
        return clear(conn, library);
    };
    let account = account_now(conn)?;
    let answered = read(conn, library)?
        .is_some_and(|seen| seen.account == account && seen.person == person && seen.answered);
    let seen = Seen {
        account,
        person,
        answered,
    };
    write_state(conn, &key(library), Some(&serde_json::to_string(&seen)?))
}

pub fn clear(conn: &Connection, library: &str) -> Result<()> {
    write_state(conn, &key(library), None)
}

/// «Leave it as it is»: not asked again for this account and this person.
pub fn answer(conn: &Connection, library: &str) -> Result<()> {
    if let Some(mut seen) = read(conn, library)? {
        seen.answered = true;
        write_state(conn, &key(library), Some(&serde_json::to_string(&seen)?))?;
    }
    Ok(())
}

/// The questions still open for the account signed in now, with the person the device is today.
pub fn pending(conn: &Connection) -> Result<Vec<OtherOwner>> {
    let account = account_now(conn)?;
    let me = crate::ops::device_id(conn)?;
    let libraries: Vec<String> = conn
        .prepare("SELECT key FROM local_state WHERE key LIKE ?1 ORDER BY key")?
        .query_map([format!("{PREFIX}%")], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|k| k.strip_prefix(PREFIX).map(str::to_owned))
        .collect();
    let mut open = Vec::new();
    for library in libraries {
        let Some(seen) = read(conn, &library)? else {
            continue;
        };
        let person = permissions::person_of_device(conn, &library, &me)?;
        if seen.answered || seen.account != account || person.as_deref() != Some(&seen.person) {
            continue;
        }
        let listed = people::list_people(conn, &library)?;
        let Some(who) = listed.people.into_iter().find(|p| p.id == seen.person) else {
            continue;
        };
        let library_name = conn
            .query_row(
                "SELECT name FROM libraries WHERE id = ?1",
                [&library],
                |row| row.get(0),
            )
            .unwrap_or_default();
        open.push(OtherOwner {
            library,
            library_name,
            person: who.name,
            role: who.role,
        });
    }
    Ok(open)
}
