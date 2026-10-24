//! Covers travel between devices as files, after the operations (D82): each side says which cover files of their
//! shared libraries it has, and they swap what the other lacks. Only covers an edition of a library the other side
//! may receive points to: a cover is never a way into a library. The transport (local network, cloud) is the
//! caller's; this module decides what to ask for and what to accept.

use std::collections::BTreeSet;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::covers::{Covers, is_cover_name};
use crate::error::Result;

/// At most this many names in one inventory: far more covers than any library has.
pub const MAX_LISTED: usize = 50_000;

/// Covers fetched or sent at the same time.
pub const AT_ONCE: usize = 4;

/// What one side says it has, among the covers of the libraries it shares with the other.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub have: Vec<String>,
}

/// The answer: what the answering side will send, and what it wants sent to it.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Swap {
    pub send: Vec<String>,
    pub want: Vec<String>,
}

/// The cover files the editions of `libraries` point to.
pub fn referenced(conn: &Connection, libraries: &[String]) -> Result<BTreeSet<String>> {
    let mut statement = conn.prepare(
        "SELECT DISTINCT cover FROM editions WHERE library_id = ?1 AND cover IS NOT NULL",
    )?;
    let mut names = BTreeSet::new();
    for library in libraries {
        for name in statement.query_map([library], |row| row.get::<_, String>(0))? {
            let name = name?;
            if is_cover_name(&name) {
                names.insert(name);
            }
        }
    }
    Ok(names)
}

/// What this side has of `referenced`, to tell the other.
pub fn inventory(covers: &Covers, referenced: &BTreeSet<String>) -> Inventory {
    Inventory {
        have: referenced
            .iter()
            .filter(|name| covers.has(name))
            .take(MAX_LISTED)
            .cloned()
            .collect(),
    }
}

/// The answer to the other side's inventory. Only names in `referenced` count, whatever the other side listed.
pub fn answer(covers: &Covers, referenced: &BTreeSet<String>, theirs: &Inventory) -> Swap {
    let theirs: BTreeSet<&String> = theirs
        .have
        .iter()
        .take(MAX_LISTED)
        .filter(|name| referenced.contains(*name))
        .collect();
    let (send, want): (Vec<&String>, Vec<&String>) = referenced
        .iter()
        .filter(|name| covers.has(name) != theirs.contains(name))
        .partition(|name| covers.has(name));
    Swap {
        send: send.into_iter().cloned().collect(),
        want: want.into_iter().cloned().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covers_with(names: &[&str]) -> (Covers, Vec<String>) {
        let dir = std::env::temp_dir().join(format!(
            "kosmotheka-exchange-{}-{}",
            std::process::id(),
            crate::ops::new_id()
        ));
        let covers = Covers::new(dir).unwrap();
        let stored = names
            .iter()
            .map(|seed| {
                let picture = image::RgbImage::from_fn(40, 60, |x, y| {
                    image::Rgb([
                        seed.as_bytes()[0].wrapping_mul(37),
                        u8::try_from(x * 5).unwrap(),
                        u8::try_from(y * 4).unwrap(),
                    ])
                });
                let mut png = std::io::Cursor::new(Vec::new());
                picture.write_to(&mut png, image::ImageFormat::Png).unwrap();
                covers.store_cover(png.get_ref()).unwrap()
            })
            .collect();
        (covers, stored)
    }

    #[test]
    fn each_side_sends_what_the_other_lacks_and_only_covers_of_the_shared_editions() {
        let (mine, names) = covers_with(&["a", "b"]);
        let (_, theirs) = covers_with(&["c", "d"]);
        let referenced: BTreeSet<String> = [&names[0], &names[1], &theirs[0]]
            .into_iter()
            .cloned()
            .collect();
        let inventory = Inventory {
            have: vec![
                names[1].clone(),
                theirs[0].clone(),
                theirs[1].clone(),
                "../secret".into(),
            ],
        };

        let swap = answer(&mine, &referenced, &inventory);

        assert_eq!(swap.send, vec![names[0].clone()]);
        assert_eq!(
            swap.want,
            vec![theirs[0].clone()],
            "never a cover no shared edition uses"
        );
    }
}
