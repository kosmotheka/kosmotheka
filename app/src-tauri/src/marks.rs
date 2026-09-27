//! Marks (D68): what each person notes about each work, from this device's side.
//! A mark belongs to the person this device belongs to; the permission rules let
//! everyone keep their own.

use kosmotheka_domain::{Change, Entity, mark_id};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::ops::{self, new_id};
use crate::people::PersonPick;
use crate::permissions;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mark {
    /// `pending`, `in_progress`, `finished` or `abandoned`.
    pub status: Option<String>,
    /// `YYYY-MM-DD`: when the status was set.
    pub status_on: Option<String>,
    /// From 1 to 5.
    pub rating: Option<i64>,
    pub favorite: bool,
}

impl Mark {
    fn is_empty(&self) -> bool {
        self.status.is_none() && self.rating.is_none() && !self.favorite
    }

    fn fields(&self) -> Map<String, Value> {
        let mut fields = Map::new();
        fields.insert("status".into(), json!(self.status));
        fields.insert("status_on".into(), json!(self.status_on));
        fields.insert("rating".into(), json!(self.rating));
        fields.insert("favorite".into(), json!(self.favorite));
        fields
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonMark {
    pub person: String,
    #[serde(flatten)]
    pub mark: Mark,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkMarks {
    /// This device's person's mark, if any.
    pub mine: Option<Mark>,
    pub others: Vec<PersonMark>,
    /// This device belongs to a person, so it can mark.
    pub can_mark: bool,
    /// It belongs to nobody, and the library has no roles: it may say who it is.
    pub may_claim: bool,
}

/// The person this device belongs to in `library`.
pub fn my_member(conn: &Connection, library: &str) -> Result<Option<String>> {
    let me = ops::device_id(conn)?;
    Ok(conn
        .query_row(
            "SELECT member_id FROM devices WHERE library_id = ?1 AND id = ?2",
            [library, &me],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten())
}

pub fn work_marks(conn: &Connection, library: &str, work: &str) -> Result<WorkMarks> {
    let me = my_member(conn, library)?;
    let mut statement = conn.prepare(
        "SELECT k.member_id, m.name, k.status, k.status_on, k.rating, k.favorite
           FROM marks k JOIN members m ON m.library_id = k.library_id AND m.id = k.member_id
          WHERE k.library_id = ?1 AND k.work_id = ?2
          ORDER BY m.name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([library, work], |row| {
        Ok((
            row.get::<_, String>(0)?,
            PersonMark {
                person: row.get(1)?,
                mark: Mark {
                    status: row.get(2)?,
                    status_on: row.get(3)?,
                    rating: row.get(4)?,
                    favorite: row.get::<_, Option<i64>>(5)?.unwrap_or(0) != 0,
                },
            },
        ))
    })?;
    let mut mine = None;
    let mut others = Vec::new();
    for row in rows {
        let (member, mark) = row?;
        if Some(&member) == me.as_ref() {
            mine = Some(mark.mark);
        } else {
            others.push(mark);
        }
    }
    Ok(WorkMarks {
        mine,
        others,
        can_mark: me.is_some(),
        may_claim: me.is_none() && !permissions::has_roles(conn, library)?,
    })
}

/// Saves this device's person's mark on `work` as given: only the fields that
/// changed travel. An emptied mark stays as an empty row: deleting it would leave
/// a tombstone on its derived id, and the person could never mark the work again.
pub fn set_mark(conn: &mut Connection, library: &str, work: &str, mark: &Mark) -> Result<()> {
    let member = my_member(conn, library)?.ok_or(Error::NotAllowed)?;
    let id = mark_id(&member, work);
    let current = work_marks(conn, library, work)?.mine;
    let change = match current {
        None if mark.is_empty() => return Ok(()),
        None => {
            // Only what is set: an empty field would overwrite what another device
            // of the same person set meanwhile, as the newer value.
            let mut fields: Map<String, Value> = mark
                .fields()
                .into_iter()
                .filter(|(_, value)| !value.is_null() && *value != json!(false))
                .collect();
            fields.insert("member_id".into(), json!(member));
            fields.insert("work_id".into(), json!(work));
            Change::create(Entity::Mark, &id, Value::Object(fields))
        }
        Some(current) => {
            let before = current.fields();
            let changed: Map<String, Value> = mark
                .fields()
                .into_iter()
                .filter(|(name, value)| before.get(name) != Some(value))
                .collect();
            if changed.is_empty() {
                return Ok(());
            }
            Change::update(Entity::Mark, &id, Value::Object(changed))
        }
    };
    let tx = conn.transaction()?;
    ops::record(&tx, library, "mark.set", vec![change])?;
    tx.commit()?;
    Ok(())
}

/// «Who are you?»: in a library without roles, this device says which person it
/// belongs to, an existing one or a new one without a role, so it can mark.
pub fn claim_person(conn: &mut Connection, library: &str, pick: &PersonPick) -> Result<()> {
    if permissions::has_roles(conn, library)? {
        return Err(Error::NotAllowed);
    }
    let name = pick.name.trim();
    let existing: Option<String> = match &pick.id {
        Some(id) => Some(id.clone()),
        None => conn
            .query_row(
                "SELECT id FROM members WHERE library_id = ?1 AND name = ?2 COLLATE NOCASE",
                [library, name],
                |row| row.get(0),
            )
            .optional()?,
    };
    let mut changes = Vec::new();
    let member = match existing {
        Some(id) => id,
        None => {
            let id = new_id();
            changes.push(Change::create(Entity::Member, &id, json!({ "name": name })));
            id
        }
    };
    let tx = conn.transaction()?;
    // The device row has to exist before it can say whose it is.
    ops::ensure_registered(&tx, library)?;
    changes.push(Change::update(
        Entity::Device,
        ops::device_id(&tx)?,
        json!({ "member_id": member }),
    ));
    ops::record(&tx, library, "person.claim", changes)?;
    tx.commit()?;
    Ok(())
}
