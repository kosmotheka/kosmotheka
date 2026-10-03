//! Backups (D30): a zip with the catalogue of one or more libraries as JSON, their
//! covers, and their operation logs as history. A whole-device backup is simply one
//! with every library (milestone 4.6). Restoring rebuilds each library as one
//! operation.

use std::collections::BTreeMap;
use std::io::{Read, Write};

use age::secrecy::SecretString;
use kosmotheka_domain::{Change, Entity};
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::covers::{Covers, MAX_COVER_BYTES};
use crate::error::{Error, Result};
use crate::libraries;
use crate::ops;
use crate::sync::{entity_rows, is_tombstoned};

/// What a library holds; the library row itself is where it is restored to.
/// Same order as `sync::ORDER`: parents before children. Revocations stay out:
/// who may sync a library is not part of its catalogue.
const CONTENTS: [Entity; 19] = [
    Entity::Publisher,
    Entity::Tag,
    Entity::Series,
    Entity::Person,
    Entity::Member,
    Entity::Work,
    Entity::WorkContributor,
    Entity::WorkTag,
    Entity::WorkGenre,
    Entity::SeriesEntry,
    Entity::WorkRelation,
    Entity::Edition,
    Entity::BookDetails,
    Entity::ComicDetails,
    Entity::BoardgameDetails,
    Entity::FilmDetails,
    Entity::TvSeriesDetails,
    Entity::Copy,
    Entity::Mark,
];

const FORMAT: &str = "kosmotheka-backup";
/// What backups were called before the app was named Kosmotheka (D70).
const LEGACY_FORMAT: &str = "library-backup";
/// 3: one or more libraries, each with its id and name (milestone 4.6).
const VERSION: u32 = 3;
/// 2: a single library, without its id or name (D46).
const SINGLE_LIBRARY_VERSION: u32 = 2;
/// Migrations after this one did not change the catalogue tables, so backups made
/// since still restore. Raise it when a migration changes a catalogue field.
const OLDEST_RESTORABLE_SCHEMA: i64 = 9;
const DATA_FILE: &str = "backup.json";
const COVERS_PREFIX: &str = "covers/";
/// How every `age` file starts (D86); a plain backup is a zip and starts with `PK`.
const AGE_HEADER: &[u8] = b"age-encryption.org/v1";

pub(crate) type Rows = BTreeMap<String, Vec<Map<String, Value>>>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    format: String,
    version: u32,
    /// Database migration level the backup was made with.
    schema_version: i64,
    exported_at: String,
    #[serde(default)]
    libraries: Vec<LibraryBackup>,
    /// Version 2 only: the rows of its single library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entities: Option<Rows>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryBackup {
    id: String,
    name: String,
    /// Rows per table, with the same field names operations use.
    entities: Rows,
    /// The operation log, kept as history. It is not replayed on restore.
    operations: Vec<Value>,
}

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub libraries: usize,
    pub books: usize,
    pub copies: usize,
    pub covers: usize,
}

/// What a restore did, library by library.
#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSummary {
    pub restored: Vec<RestoredLibrary>,
    /// Names of libraries skipped because this device already has them with content.
    pub skipped: Vec<String>,
    pub covers: usize,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredLibrary {
    pub id: String,
    pub name: String,
    pub books: usize,
    pub copies: usize,
}

/// Every library on this device, for a whole-device backup.
pub fn all_libraries(conn: &Connection) -> Result<Vec<String>> {
    Ok(conn
        .prepare("SELECT id FROM libraries ORDER BY name COLLATE NOCASE, id")?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// With a passphrase the bytes written are the zip encrypted as an `age` file (D86),
/// which the `age` tool opens without this app; without one, the plain zip.
pub fn export(
    conn: &Connection,
    ids: &[String],
    covers: &Covers,
    mut out: impl Write,
    passphrase: Option<&str>,
) -> Result<BackupSummary> {
    if passphrase == Some("") {
        return Err(Error::BadBackup("passphrase_empty"));
    }
    let mut summary = BackupSummary::default();
    let mut backed_up = Vec::new();
    for id in ids {
        let name: String =
            conn.query_row("SELECT name FROM libraries WHERE id = ?1", [id], |row| {
                row.get(0)
            })?;
        let mut entities = Rows::new();
        for entity in CONTENTS {
            entities.insert(entity.table().to_owned(), entity_rows(conn, id, entity)?);
        }
        summary.books += entities[Entity::Edition.table()].len();
        summary.copies += entities[Entity::Copy.table()].len();
        backed_up.push(LibraryBackup {
            id: id.clone(),
            name,
            operations: operations(conn, id)?,
            entities,
        });
    }
    summary.libraries = backed_up.len();
    let backup = BackupFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        schema_version: schema_version(conn)?,
        exported_at: conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |r| {
            r.get(0)
        })?,
        libraries: backed_up,
        entities: None,
    };

    // Built in memory and written at once: a file picked on Android may be a stream
    // (a cloud folder, say) that cannot seek, and ZIP writing needs to.
    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(DATA_FILE, options)?;
    zip.write_all(&serde_json::to_vec_pretty(&backup)?)?;

    let mut cover_names: Vec<String> = backup
        .libraries
        .iter()
        .flat_map(|library| &library.entities[Entity::Edition.table()])
        .filter_map(|row| row.get("cover")?.as_str().map(str::to_owned))
        .collect();
    cover_names.sort();
    cover_names.dedup();
    for name in cover_names {
        // A cover file that went missing on disk should not make the whole backup fail.
        if let Ok(bytes) = std::fs::read(covers.dir().join(&name)) {
            zip.start_file(format!("{COVERS_PREFIX}{name}"), options)?;
            zip.write_all(&bytes)?;
            summary.covers += 1;
        }
    }
    let zip = zip.finish()?.into_inner();
    let bytes = match passphrase {
        Some(passphrase) => age::encrypt(
            &age::scrypt::Recipient::new(SecretString::from(passphrase.to_owned())),
            &zip,
        )
        .map_err(|_| Error::BadBackup("unreadable"))?,
        None => zip,
    };
    out.write_all(&bytes)?;
    out.flush()?;
    Ok(summary)
}

/// Restores every library in the backup. Each one comes back with its own id, so a
/// device that pairs again merges with the other copies instead of duplicating them.
/// It gets a new id if it was deleted here, and is skipped if this device already has
/// it with content: merging two catalogues is what sync does. A version 2 backup,
/// which has no id, goes into `active` if that is empty. An encrypted backup needs its
/// passphrase; it is decrypted before anything is written (D86).
pub fn restore(
    conn: &mut Connection,
    active: &str,
    covers: &Covers,
    mut input: impl Read,
    passphrase: Option<&str>,
) -> Result<RestoreSummary> {
    // Read whole for the same reason as `export`: reading a ZIP needs to seek.
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes)?;
    if bytes.starts_with(AGE_HEADER) {
        let passphrase = passphrase
            .filter(|passphrase| !passphrase.is_empty())
            .ok_or(Error::BadBackup("passphrase_needed"))?;
        bytes = age::decrypt(
            &age::scrypt::Identity::new(SecretString::from(passphrase.to_owned())),
            &bytes,
        )
        .map_err(|error| match error {
            age::DecryptError::DecryptionFailed
            | age::DecryptError::NoMatchingKeys
            | age::DecryptError::KeyDecryptionFailed
            | age::DecryptError::InvalidMac => Error::BadBackup("wrong_passphrase"),
            _ => Error::BadBackup("unreadable"),
        })?;
    }
    let mut archive = ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| Error::BadBackup("not_a_backup"))?;
    let backup: BackupFile = {
        let mut data = String::new();
        archive
            .by_name(DATA_FILE)
            .map_err(|_| Error::BadBackup("not_a_backup"))?
            .read_to_string(&mut data)?;
        serde_json::from_str(&data).map_err(|_| Error::BadBackup("unreadable"))?
    };
    if ![FORMAT, LEGACY_FORMAT].contains(&backup.format.as_str())
        || ![VERSION, SINGLE_LIBRARY_VERSION].contains(&backup.version)
    {
        return Err(Error::BadBackup("not_a_backup"));
    }
    if !(OLDEST_RESTORABLE_SCHEMA..=schema_version(conn)?).contains(&backup.schema_version) {
        return Err(Error::BadBackup("other_version"));
    }
    let covers_in_zip = read_covers(&mut archive)?;

    let mut summary = RestoreSummary::default();
    let tx = conn.transaction()?;
    if backup.version == SINGLE_LIBRARY_VERSION {
        if !library_is_empty(&tx, active)? {
            return Err(Error::BadBackup("not_empty"));
        }
        let rows = backup.entities.unwrap_or_default();
        record_rows(&tx, active, &rows)?;
        let name = tx.query_row("SELECT name FROM libraries WHERE id = ?1", [active], |r| {
            r.get(0)
        })?;
        summary.restored.push(restored(&tx, active, name)?);
    } else {
        for library in backup.libraries {
            let target = if !libraries::library_exists(&tx, &library.id)? {
                if is_tombstoned(&tx, &library.id, Entity::Library, &library.id)? {
                    None
                } else {
                    Some(library.id.clone())
                }
            } else if library_is_empty(&tx, &library.id)? {
                Some(library.id.clone())
            } else {
                summary.skipped.push(library.name);
                continue;
            };
            let id = place_library(&tx, target.as_deref(), &library.name)?;
            record_rows(&tx, &id, &library.entities)?;
            let name = tx.query_row("SELECT name FROM libraries WHERE id = ?1", [&id], |r| {
                r.get(0)
            })?;
            summary.restored.push(restored(&tx, &id, name)?);
        }
    }
    tx.commit()?;

    for (name, extension, bytes) in covers_in_zip {
        // The stored name is recomputed from the bytes: a tampered file gets a different
        // name and simply is not the one any edition points to.
        if covers.store(&bytes, extension)? == name {
            summary.covers += 1;
        }
    }
    Ok(summary)
}

/// Makes sure the library a backup goes into exists, named as in the backup unless
/// another library here already has that name. Returns its id.
fn place_library(tx: &Transaction<'_>, id: Option<&str>, name: &str) -> Result<String> {
    let name = libraries::unused_name(tx, name, id)?;
    match id {
        Some(id) if libraries::library_exists(tx, id)? => {
            libraries::rename_in(tx, id, &name)?;
            Ok(id.to_owned())
        }
        Some(id) => libraries::create_with_id(tx, id, &name),
        None => libraries::create_with_id(tx, &ops::new_id(), &name),
    }
}

fn record_rows(tx: &Transaction<'_>, library: &str, rows: &Rows) -> Result<()> {
    record_rows_as(tx, library, rows, "backup.restore")
}

/// Records every row as created, in one operation. Roles stay behind: whoever
/// restores or duplicates a library starts it without roles and can take them on;
/// restored roles would point at devices that are not there (D58).
fn record_rows_as(tx: &Transaction<'_>, library: &str, rows: &Rows, intent: &str) -> Result<()> {
    let mut changes = Vec::new();
    for entity in CONTENTS {
        for row in rows.get(entity.table()).into_iter().flatten() {
            let mut fields = row.clone();
            let id = match fields.remove("id") {
                Some(Value::String(id)) => id,
                _ => return Err(Error::BadBackup("unreadable")),
            };
            if entity == Entity::Member {
                fields.remove("role");
                fields.remove("can_share");
            }
            changes.push(Change::create(entity, id, Value::Object(fields)));
        }
    }
    if !changes.is_empty() {
        ops::record(tx, library, intent, changes)?;
    }
    Ok(())
}

/// Copies a library's catalogue into a new one, without its roles, its devices or
/// its history: the last resort when its only administrator lost their only device
/// (D58). Returns the new library's id.
pub fn duplicate(conn: &mut Connection, from: &str, name: &str) -> Result<String> {
    let rows = read_library(conn, from)?;
    let tx = conn.transaction()?;
    let to = copy_as_new(&tx, &rows, name)?;
    tx.commit()?;
    Ok(to)
}

pub(crate) fn read_library(conn: &Connection, library: &str) -> Result<Rows> {
    let mut rows = Rows::new();
    for entity in CONTENTS {
        rows.insert(
            entity.table().to_owned(),
            entity_rows(conn, library, entity)?,
        );
    }
    Ok(rows)
}

/// Records `rows` as a new library with a random id and makes it the active one.
pub(crate) fn copy_as_new(tx: &Transaction<'_>, rows: &Rows, name: &str) -> Result<String> {
    let to = place_library(tx, None, name)?;
    record_rows_as(tx, &to, rows, "library.duplicate")?;
    ops::set_active_library(tx, &to)?;
    Ok(to)
}

fn restored(conn: &Connection, id: &str, name: String) -> Result<RestoredLibrary> {
    Ok(RestoredLibrary {
        id: id.to_owned(),
        name,
        books: count(conn, id, Entity::Edition)?,
        copies: count(conn, id, Entity::Copy)?,
    })
}

/// Cover entries whose name is exactly `<sha256>.<known extension>`. Paths inside the
/// zip are never used to build file paths, so a crafted archive cannot write elsewhere.
fn read_covers(
    archive: &mut ZipArchive<std::io::Cursor<Vec<u8>>>,
) -> Result<Vec<(String, &'static str, Vec<u8>)>> {
    let mut found = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(name) = entry.name().strip_prefix(COVERS_PREFIX).map(str::to_owned) else {
            continue;
        };
        let Some(extension) = cover_extension(&name) else {
            continue;
        };
        if entry.size() > MAX_COVER_BYTES as u64 {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes)?;
        found.push((name, extension, bytes));
    }
    Ok(found)
}

fn cover_extension(name: &str) -> Option<&'static str> {
    let (hash, extension) = name.split_once('.')?;
    let valid_hash = hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    let extension = ["jpg", "png", "webp", "gif"]
        .into_iter()
        .find(|e| *e == extension)?;
    valid_hash.then_some(extension)
}

fn operations(conn: &Connection, library: &str) -> Result<Vec<Value>> {
    let mut statement = conn.prepare(
        "SELECT id, hlc, device_id, intent, changes, recorded_at FROM operations
          WHERE library_id = ?1 ORDER BY hlc",
    )?;
    let rows = statement.query_map([library], |row| {
        let changes: String = row.get(4)?;
        Ok(json!({
            "id": row.get::<_, String>(0)?,
            "hlc": row.get::<_, String>(1)?,
            "deviceId": row.get::<_, String>(2)?,
            "intent": row.get::<_, String>(3)?,
            "changes": serde_json::from_str::<Value>(&changes).unwrap_or(Value::Null),
            "recordedAt": row.get::<_, String>(5)?,
        }))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn schema_version(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

fn count(conn: &Connection, library: &str, entity: Entity) -> Result<usize> {
    let n: i64 = conn.query_row(
        &format!(
            "SELECT count(*) FROM {} WHERE {} = ?1",
            entity.table(),
            entity.library_column()
        ),
        [library],
        |r| r.get(0),
    )?;
    Ok(usize::try_from(n).unwrap_or_default())
}

fn library_is_empty(conn: &Connection, library: &str) -> Result<bool> {
    for entity in CONTENTS {
        if count(conn, library, entity)? > 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};
    use rusqlite::OptionalExtension;

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::libraries;
    use crate::marks::{self, Mark};
    use crate::people::PersonPick;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kosmotheka-backup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn book(title: &str, isbn: Option<&str>) -> ValidBook {
        ValidBook {
            edition: EditionInput {
                title: title.into(),
                authors: vec!["Ursula K. Le Guin".into()],
                publisher: Some("Minotauro".into()),
                isbn: isbn.map(str::to_owned),
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: Some("Tersoal".into()),
                format: Format::Hardcover,
                quantity: 2,
                location: Some("Salón".into()),
                acquired_on: Some("2026-09-25".into()),
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

    fn name_of(conn: &Connection, library: &str) -> String {
        conn.query_row("SELECT name FROM libraries WHERE id = ?1", [library], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn mark_status(conn: &Connection, library: &str, work: &str) -> Option<String> {
        conn.query_row(
            "SELECT status FROM marks WHERE library_id = ?1 AND work_id = ?2",
            [library, work],
            |r| r.get(0),
        )
        .optional()
        .unwrap()
        .flatten()
    }

    #[test]
    fn export_and_restore_round_trip() {
        let dir = temp("roundtrip");
        let source_covers = Covers::new(dir.join("source-covers")).unwrap();
        let cover = source_covers.store(b"fake image", "jpg").unwrap();
        let mut source = db::open_in_memory().unwrap();
        let lib = ops::active_library(&source).unwrap();
        catalog::create_book(
            &mut source,
            &lib,
            &book("A Wizard of Earthsea", Some("9780306406157")),
            Some(&cover),
        )
        .unwrap();
        catalog::create_book(&mut source, &lib, &book("The Dispossessed", None), None).unwrap();
        marks::claim_person(
            &mut source,
            &lib,
            &PersonPick {
                id: None,
                name: "Tersoal".into(),
            },
        )
        .unwrap();
        let earthsea_edition = catalog::list_books(&source, &lib, &BookFilter::default()).unwrap()
            [0]
        .edition_id
        .clone();
        let earthsea_work = catalog::get_book(&source, &lib, &earthsea_edition)
            .unwrap()
            .work_id;
        marks::set_mark(
            &mut source,
            &lib,
            &earthsea_work,
            &Mark {
                status: Some("finished".into()),
                ..Mark::default()
            },
        )
        .unwrap();
        let file = dir.join("backup.zip");

        let exported = export(
            &source,
            std::slice::from_ref(&lib),
            &source_covers,
            File::create(&file).unwrap(),
            None,
        )
        .unwrap();

        let target_covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        let restored = restore(
            &mut target,
            &to,
            &target_covers,
            File::open(&file).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(
            exported,
            BackupSummary {
                libraries: 1,
                books: 2,
                copies: 2,
                covers: 1
            }
        );
        assert_eq!(restored.restored.len(), 1);
        assert_eq!(
            restored.restored[0].id, lib,
            "same id, so it merges on sync"
        );
        assert_eq!(
            (restored.restored[0].books, restored.restored[0].copies),
            (2, 2)
        );
        assert_eq!(restored.covers, 1);
        let books = catalog::list_books(&target, &lib, &BookFilter::default()).unwrap();
        assert_eq!(books.len(), 2);
        assert_eq!(books[0].title, "A Wizard of Earthsea");
        assert_eq!(books[0].authors, vec!["Ursula K. Le Guin"]);
        assert_eq!(books[0].publisher.as_deref(), Some("Minotauro"));
        assert_eq!(books[0].cover.as_deref(), Some(cover.as_str()));
        assert!(target_covers.dir().join(&cover).exists());
        // Restored books are searchable and have a history entry.
        let filter = BookFilter {
            query: Some("earthsea".into()),
            ..BookFilter::default()
        };
        assert_eq!(
            catalog::list_books(&target, &lib, &filter).unwrap().len(),
            1
        );
        let intent: String = target
            .query_row(
                "SELECT intent FROM operations
                  WHERE library_id = ?1 AND intent NOT IN ('device.register', 'library.create')",
                [&lib],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(intent, "backup.restore");
        assert_eq!(
            mark_status(&target, &lib, &earthsea_work),
            Some("finished".into()),
            "a mark survives the backup and the restore"
        );
    }

    #[test]
    fn a_device_backup_brings_back_every_library_with_its_id_and_name() {
        let dir = temp("device");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut source = db::open_in_memory().unwrap();
        let lib = ops::active_library(&source).unwrap();
        libraries::rename_library(&mut source, &lib, "Casa").unwrap();
        catalog::create_book(&mut source, &lib, &book("Home book", None), None).unwrap();
        let club = libraries::create_library(&mut source, "Club").unwrap();
        catalog::create_book(&mut source, &club, &book("Club book", None), None).unwrap();
        let file = dir.join("device.zip");

        let exported = export(
            &source,
            &all_libraries(&source).unwrap(),
            &covers,
            File::create(&file).unwrap(),
            None,
        )
        .unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        let restored =
            restore(&mut target, &to, &covers, File::open(&file).unwrap(), None).unwrap();

        assert_eq!((exported.libraries, exported.books), (2, 2));
        assert_eq!(restored.restored.len(), 2);
        assert!(restored.skipped.is_empty());
        assert_eq!(name_of(&target, &lib), "Casa");
        assert_eq!(name_of(&target, &club), "Club");
        assert_eq!(titles(&target, &lib), vec!["Home book"]);
        assert_eq!(titles(&target, &club), vec!["Club book"]);
    }

    /// What `sync` does between two devices: everything each lacks, both ways.
    /// Returns how many operations were refused in each direction.
    fn sync_pair(one: &mut Connection, other: &mut Connection, library: &str) -> (usize, usize) {
        let libraries = [library.to_owned()];
        let mut refused = (0, 0);
        let missing = crate::sync::operations_since(
            other,
            &crate::sync::version_vectors(one, &libraries).unwrap(),
        )
        .unwrap();
        refused.0 = crate::sync::merge(one, missing).unwrap().rejected;
        let missing = crate::sync::operations_since(
            one,
            &crate::sync::version_vectors(other, &libraries).unwrap(),
        )
        .unwrap();
        refused.1 = crate::sync::merge(other, missing).unwrap().rejected;
        refused
    }

    #[test]
    fn a_restored_device_writes_a_new_chain_and_syncs_with_peers_that_hold_the_old_one() {
        // A backup carries neither the device id nor the key (D58): the device that
        // restores keeps its own identity, so its chain starts from its own log and
        // never collides with the one the backed-up device left on its peers.
        let dir = temp("chain");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut old = db::open_in_memory().unwrap();
        let lib = ops::active_library(&old).unwrap();
        catalog::create_book(&mut old, &lib, &book("Before the backup", None), None).unwrap();
        let file = dir.join("old.zip");
        export(
            &old,
            std::slice::from_ref(&lib),
            &covers,
            File::create(&file).unwrap(),
            None,
        )
        .unwrap();
        let mut peer = db::open_in_memory().unwrap();
        assert_eq!(sync_pair(&mut peer, &mut old, &lib), (0, 0));
        // The old device goes on after the backup; the peer holds those operations.
        catalog::create_book(&mut old, &lib, &book("After the backup", None), None).unwrap();
        assert_eq!(sync_pair(&mut peer, &mut old, &lib), (0, 0));

        let mut restored = db::open_in_memory().unwrap();
        let first = ops::active_library(&restored).unwrap();
        let summary = restore(
            &mut restored,
            &first,
            &covers,
            File::open(&file).unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(summary.restored.len(), 1);
        catalog::create_book(&mut restored, &lib, &book("After the restore", None), None).unwrap();
        catalog::create_book(&mut restored, &lib, &book("And again", None), None).unwrap();

        assert_eq!(sync_pair(&mut restored, &mut peer, &lib), (0, 0));
        assert_eq!(sync_pair(&mut restored, &mut old, &lib), (0, 0));
        assert_eq!(sync_pair(&mut peer, &mut old, &lib), (0, 0));
        let sorted = |conn: &Connection| {
            let mut titles = titles(conn, &lib);
            titles.sort();
            titles.dedup();
            titles
        };
        assert_eq!(sorted(&restored), sorted(&peer));
        assert_eq!(sorted(&peer), sorted(&old));
        assert!(sorted(&peer).contains(&"After the restore".to_owned()));
        assert!(sorted(&peer).contains(&"After the backup".to_owned()));
    }

    #[test]
    fn a_library_already_here_with_content_is_skipped() {
        let dir = temp("nonempty");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("A", None), None).unwrap();
        let file = dir.join("backup.zip");
        export(
            &conn,
            std::slice::from_ref(&lib),
            &covers,
            File::create(&file).unwrap(),
            None,
        )
        .unwrap();

        let summary = restore(&mut conn, &lib, &covers, File::open(&file).unwrap(), None).unwrap();

        assert!(summary.restored.is_empty());
        assert_eq!(summary.skipped, vec![String::new()]);
        assert_eq!(titles(&conn, &lib), vec!["A"], "nothing duplicated");
    }

    #[test]
    fn a_library_deleted_here_comes_back_under_a_new_id() {
        let dir = temp("deleted");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let club = libraries::create_library(&mut conn, "Club").unwrap();
        catalog::create_book(&mut conn, &club, &book("Club book", None), None).unwrap();
        let file = dir.join("club.zip");
        export(
            &conn,
            std::slice::from_ref(&club),
            &covers,
            File::create(&file).unwrap(),
            None,
        )
        .unwrap();
        libraries::delete_library(&mut conn, &club).unwrap();

        let summary = restore(&mut conn, &lib, &covers, File::open(&file).unwrap(), None).unwrap();

        let back = &summary.restored[0];
        assert_ne!(back.id, club, "the deletion still holds for the old id");
        assert_eq!(back.name, "Club");
        assert_eq!(titles(&conn, &back.id), vec!["Club book"]);
    }

    #[test]
    fn a_name_in_use_gets_a_number() {
        let dir = temp("name");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut source = db::open_in_memory().unwrap();
        let club = libraries::create_library(&mut source, "Club").unwrap();
        let file = dir.join("club.zip");
        export(
            &source,
            &[club],
            &covers,
            File::create(&file).unwrap(),
            None,
        )
        .unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        libraries::create_library(&mut target, "club").unwrap();

        let summary = restore(&mut target, &to, &covers, File::open(&file).unwrap(), None).unwrap();

        assert_eq!(summary.restored[0].name, "Club (2)");
    }

    #[test]
    fn a_version_2_backup_goes_into_the_active_library() {
        let dir = temp("v2");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut source = db::open_in_memory().unwrap();
        let lib = ops::active_library(&source).unwrap();
        catalog::create_book(&mut source, &lib, &book("Old backup", None), None).unwrap();
        let mut entities = Rows::new();
        for entity in CONTENTS {
            entities.insert(
                entity.table().to_owned(),
                entity_rows(&source, &lib, entity).unwrap(),
            );
        }
        let old = json!({
            "format": LEGACY_FORMAT,
            "version": 2,
            "schemaVersion": 9,
            "exportedAt": "2026-09-26T10:00:00.000Z",
            "entities": entities,
            "operations": [],
        });
        let file = dir.join("old.zip");
        let mut zip = ZipWriter::new(File::create(&file).unwrap());
        zip.start_file(DATA_FILE, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(old.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
        let mut target = db::open_in_memory().unwrap();
        let other = libraries::create_library(&mut target, "Other").unwrap();

        let summary = restore(
            &mut target,
            &other,
            &covers,
            File::open(&file).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.restored[0].id, other);
        assert_eq!(titles(&target, &other), vec!["Old backup"]);
    }

    /// A backup made before migration 0023 has a `videogame_details` table (dropped
    /// since) and copies with neither `platform` nor `seasons` (added since): restore
    /// only reads the tables in `CONTENTS`, so the stray table is ignored and the
    /// copy comes back with those two columns simply unset, nothing lost that ever
    /// existed (D08: no video game details were ever created before this change).
    #[test]
    fn a_backup_from_before_the_copy_had_platform_and_seasons_still_restores() {
        let dir = temp("v22");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let work_id = "01997c7e-0000-7000-8000-000000000901";
        let edition_id = "01997c7e-0000-7000-8000-000000000902";
        let copy_id = "01997c7e-0000-7000-8000-000000000903";

        let mut entities = Rows::new();
        entities.insert(
            "works".to_owned(),
            vec![
                json!({ "id": work_id, "kind": "videogame", "title": "Hollow Knight" })
                    .as_object()
                    .unwrap()
                    .clone(),
            ],
        );
        entities.insert(
            "editions".to_owned(),
            vec![
                json!({ "id": edition_id, "work_id": work_id, "title": "Hollow Knight" })
                    .as_object()
                    .unwrap()
                    .clone(),
            ],
        );
        // Gone since migration 0023; a pre-migration export still has it.
        entities.insert(
            "videogame_details".to_owned(),
            vec![
                json!({ "id": edition_id, "platform": "Nintendo Switch" })
                    .as_object()
                    .unwrap()
                    .clone(),
            ],
        );
        entities.insert(
            "copies".to_owned(),
            vec![
                json!({
                    "id": copy_id,
                    "edition_id": edition_id,
                    "format": "physical",
                    "quantity": 1,
                })
                .as_object()
                .unwrap()
                .clone(),
            ],
        );

        let old = json!({
            "format": LEGACY_FORMAT,
            "version": 2,
            "schemaVersion": 22,
            "exportedAt": "2026-09-26T10:00:00.000Z",
            "entities": entities,
            "operations": [],
        });
        let file = dir.join("old.zip");
        let mut zip = ZipWriter::new(File::create(&file).unwrap());
        zip.start_file(DATA_FILE, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(old.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
        let mut target = db::open_in_memory().unwrap();
        let other = libraries::create_library(&mut target, "Other").unwrap();

        let summary = restore(
            &mut target,
            &other,
            &covers,
            File::open(&file).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.restored[0].id, other);
        assert_eq!(titles(&target, &other), vec!["Hollow Knight"]);
        let (platform, seasons): (Option<String>, Option<String>) = target
            .query_row(
                "SELECT platform, seasons FROM copies WHERE library_id = ?1",
                [&other],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(platform, None);
        assert_eq!(seasons, None);
    }

    /// A backup made before the wishlist (entity `wish`) was dropped has a `wishes`
    /// key: restore only reads the tables in `CONTENTS`, so it is ignored and the
    /// rest of the library still comes back (D61).
    #[test]
    fn a_backup_with_old_wishes_still_restores() {
        let dir = temp("wishes");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut entities = Rows::new();
        entities.insert(
            "wishes".to_owned(),
            vec![
                json!({ "id": "01997c7e-0000-7000-8000-000000000904", "title": "Tehanu" })
                    .as_object()
                    .unwrap()
                    .clone(),
            ],
        );

        let old = json!({
            "format": LEGACY_FORMAT,
            "version": 2,
            "schemaVersion": 9,
            "exportedAt": "2026-09-26T10:00:00.000Z",
            "entities": entities,
            "operations": [],
        });
        let file = dir.join("old.zip");
        let mut zip = ZipWriter::new(File::create(&file).unwrap());
        zip.start_file(DATA_FILE, SimpleFileOptions::default())
            .unwrap();
        zip.write_all(old.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
        let mut target = db::open_in_memory().unwrap();
        let other = libraries::create_library(&mut target, "Other").unwrap();

        let summary = restore(
            &mut target,
            &other,
            &covers,
            File::open(&file).unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(summary.restored[0].id, other);
        assert!(titles(&target, &other).is_empty());
    }

    #[test]
    fn restore_rejects_files_that_are_not_backups() {
        let dir = temp("garbage");
        let file = dir.join("not-a-backup.zip");
        std::fs::write(&file, b"hello").unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();

        assert!(matches!(
            restore(&mut conn, &lib, &covers, File::open(&file).unwrap(), None),
            Err(Error::BadBackup("not_a_backup"))
        ));
    }

    #[test]
    fn only_hash_named_covers_are_accepted() {
        let hash = "a".repeat(64);
        assert_eq!(cover_extension(&format!("{hash}.jpg")), Some("jpg"));
        assert_eq!(cover_extension("../../etc/passwd"), None);
        assert_eq!(cover_extension(&format!("{hash}.exe")), None);
        assert_eq!(cover_extension(&format!("../{hash}.jpg")), None);
        assert_eq!(cover_extension(&format!("{}.jpg", "A".repeat(64))), None);
    }

    const PASSPHRASE: &str = "correct horse battery staple";

    /// A library with one book, exported to `file` (encrypted when `passphrase` is set).
    fn export_one_book(
        dir: &std::path::Path,
        file: &str,
        passphrase: Option<&str>,
    ) -> std::path::PathBuf {
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("A Wizard of Earthsea", None), None).unwrap();
        let path = dir.join(file);
        export(
            &conn,
            std::slice::from_ref(&lib),
            &covers,
            File::create(&path).unwrap(),
            passphrase,
        )
        .unwrap();
        path
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
    }

    #[test]
    fn no_backup_carries_a_secret() {
        use crate::account::{self, AccountUser};
        use crate::lan::{self, KnownServer, identity::Identity};
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;

        let dir = temp("no-secrets");
        let data_dir = dir.join("data");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        catalog::create_book(&mut conn, &lib, &book("A Wizard of Earthsea", None), None).unwrap();

        // Everything that identifies or authenticates this device.
        ops::signing_key(&conn).unwrap();
        let seed_b64 = ops::read_state(&conn, "signing_seed").unwrap().unwrap();
        let seed = URL_SAFE_NO_PAD.decode(&seed_b64).unwrap();
        let token = "tok-7f3a9c1e5b2d4a8f90c6e1d3b5a7f2c4";
        let user = AccountUser {
            id: "u1".into(),
            email: "reader@example.org".into(),
            name: "Reader".into(),
            locale: "en".into(),
        };
        account::store_session(&conn, token, &user, &user.email).unwrap();
        let server_secret = "pairing-secret-of-the-server-0123456789";
        lan::save_server(
            &conn,
            &KnownServer {
                device_id: "phone".into(),
                name: "Phone".into(),
                addresses: vec![],
                fingerprint: "ff".repeat(32),
                secret: server_secret.into(),
            },
        )
        .unwrap();
        let client_secret = "pairing-secret-of-the-client-9876543210";
        lan::save_client(&conn, "tablet", "Tablet", client_secret).unwrap();
        let client_hash = lan::hash_secret(client_secret);
        Identity::load_or_create(&data_dir).unwrap();
        let tls_key = std::fs::read(data_dir.join("sync-key.der")).unwrap();

        let mut secrets: Vec<(&str, Vec<u8>)> = vec![
            ("signing seed", seed.clone()),
            (
                "signing seed (hex)",
                seed.iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into_bytes(),
            ),
            ("signing seed (base64url)", seed_b64.into_bytes()),
            ("account token", token.into()),
            ("server pairing secret", server_secret.into()),
            ("client pairing secret", client_secret.into()),
            ("client secret hash", client_hash.into_bytes()),
            ("TLS key", tls_key.clone()),
            (
                "TLS key (hex)",
                tls_key
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
                    .into_bytes(),
            ),
            (
                "TLS key (base64)",
                base64::engine::general_purpose::STANDARD
                    .encode(&tls_key)
                    .into_bytes(),
            ),
        ];
        secrets.push(("account email", user.email.clone().into_bytes()));

        for passphrase in [None, Some(PASSPHRASE)] {
            let mut out = Vec::new();
            export(
                &conn,
                std::slice::from_ref(&lib),
                &covers,
                &mut out,
                passphrase,
            )
            .unwrap();
            // The zip itself, and every file inside it once decrypted.
            let zip_bytes = match passphrase {
                Some(passphrase) => age::decrypt(
                    &age::scrypt::Identity::new(SecretString::from(passphrase.to_owned())),
                    &out,
                )
                .unwrap(),
                None => out.clone(),
            };
            let mut files = vec![out, zip_bytes.clone()];
            let mut archive = ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
            for index in 0..archive.len() {
                let mut content = Vec::new();
                archive
                    .by_index(index)
                    .unwrap()
                    .read_to_end(&mut content)
                    .unwrap();
                files.push(content);
            }
            for (what, secret) in &secrets {
                for file in &files {
                    assert!(
                        !contains(file, secret),
                        "{what} found in a backup (passphrase: {})",
                        passphrase.is_some()
                    );
                }
            }
        }
    }

    #[test]
    fn an_encrypted_backup_is_an_age_file_that_restores_with_its_passphrase() {
        let dir = temp("encrypted-round-trip");
        let file = export_one_book(&dir, "backup.age", Some(PASSPHRASE));
        let bytes = std::fs::read(&file).unwrap();
        assert!(bytes.starts_with(b"age-encryption.org/v1"));
        assert!(
            !contains(&bytes, b"Earthsea"),
            "the catalogue is not readable"
        );

        let covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        let summary = restore(
            &mut target,
            &to,
            &covers,
            File::open(&file).unwrap(),
            Some(PASSPHRASE),
        )
        .unwrap();

        assert_eq!(summary.restored.len(), 1);
        assert_eq!(summary.restored[0].books, 1);
        assert_eq!(
            titles(&target, &summary.restored[0].id),
            vec!["A Wizard of Earthsea"]
        );
    }

    #[test]
    fn a_wrong_passphrase_restores_nothing() {
        let dir = temp("wrong-passphrase");
        let file = export_one_book(&dir, "backup.age", Some(PASSPHRASE));
        let covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();
        let libraries_before = all_libraries(&target).unwrap();

        let result = restore(
            &mut target,
            &to,
            &covers,
            File::open(&file).unwrap(),
            Some("not the passphrase"),
        );

        assert!(matches!(result, Err(Error::BadBackup("wrong_passphrase"))));
        assert_eq!(all_libraries(&target).unwrap(), libraries_before);
        assert!(titles(&target, &to).is_empty());
    }

    #[test]
    fn an_encrypted_backup_asks_for_its_passphrase() {
        let dir = temp("passphrase-needed");
        let file = export_one_book(&dir, "backup.age", Some(PASSPHRASE));
        let covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();

        for passphrase in [None, Some("")] {
            let result = restore(
                &mut target,
                &to,
                &covers,
                File::open(&file).unwrap(),
                passphrase,
            );
            assert!(matches!(result, Err(Error::BadBackup("passphrase_needed"))));
        }
        assert!(titles(&target, &to).is_empty());
    }

    #[test]
    fn a_plain_backup_still_restores_and_ignores_a_passphrase() {
        let dir = temp("plain-with-passphrase");
        let file = export_one_book(&dir, "backup.zip", None);
        assert!(std::fs::read(&file).unwrap().starts_with(b"PK"));
        let covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let to = ops::active_library(&target).unwrap();

        let summary = restore(
            &mut target,
            &to,
            &covers,
            File::open(&file).unwrap(),
            Some("unneeded"),
        )
        .unwrap();

        assert_eq!(summary.restored[0].books, 1);
    }

    #[test]
    fn an_empty_passphrase_is_refused_when_exporting() {
        let conn = db::open_in_memory().unwrap();
        let lib = ops::active_library(&conn).unwrap();
        let dir = temp("empty-passphrase");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut out = Vec::new();

        let result = export(&conn, &[lib], &covers, &mut out, Some(""));

        assert!(matches!(result, Err(Error::BadBackup("passphrase_empty"))));
        assert!(out.is_empty());
    }
}
