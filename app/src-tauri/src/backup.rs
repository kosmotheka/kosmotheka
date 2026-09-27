//! Backups (D30): a zip with the catalogue of one or more libraries as JSON, their
//! covers, and their operation logs as history. A whole-device backup is simply one
//! with every library (milestone 4.6). Restoring rebuilds each library as one
//! operation.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

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
const CONTENTS: [Entity; 9] = [
    Entity::Publisher,
    Entity::Person,
    Entity::Member,
    Entity::Work,
    Entity::WorkContributor,
    Entity::Edition,
    Entity::BookDetails,
    Entity::Copy,
    Entity::Wish,
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

type Rows = BTreeMap<String, Vec<Map<String, Value>>>;

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

pub fn export(
    conn: &Connection,
    ids: &[String],
    covers: &Covers,
    path: &Path,
) -> Result<BackupSummary> {
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

    let mut zip = ZipWriter::new(File::create(path)?);
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
    zip.finish()?;
    Ok(summary)
}

/// Restores every library in the backup. Each one comes back with its own id, so a
/// device that pairs again merges with the other copies instead of duplicating them.
/// It gets a new id if it was deleted here, and is skipped if this device already has
/// it with content: merging two catalogues is what sync does. A version 2 backup,
/// which has no id, goes into `active` if that is empty.
pub fn restore(
    conn: &mut Connection,
    active: &str,
    covers: &Covers,
    path: &Path,
) -> Result<RestoreSummary> {
    let mut archive =
        ZipArchive::new(File::open(path)?).map_err(|_| Error::BadBackup("not_a_backup"))?;
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
    let mut changes = Vec::new();
    for entity in CONTENTS {
        for row in rows.get(entity.table()).into_iter().flatten() {
            let mut fields = row.clone();
            let id = match fields.remove("id") {
                Some(Value::String(id)) => id,
                _ => return Err(Error::BadBackup("unreadable")),
            };
            changes.push(Change::create(entity, id, Value::Object(fields)));
        }
    }
    if !changes.is_empty() {
        ops::record(tx, library, "backup.restore", changes)?;
    }
    Ok(())
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
fn read_covers(archive: &mut ZipArchive<File>) -> Result<Vec<(String, &'static str, Vec<u8>)>> {
    let mut found = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let Some(name) = entry.name().strip_prefix(COVERS_PREFIX).map(str::to_owned) else {
            continue;
        };
        let Some(extension) = cover_extension(&name) else {
            continue;
        };
        if entry.size() as usize > MAX_COVER_BYTES {
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
    Ok(n as usize)
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
    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::db;
    use crate::libraries;
    use crate::ops::DEFAULT_LIBRARY_ID as LIB;

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

    #[test]
    fn export_and_restore_round_trip() {
        let dir = temp("roundtrip");
        let source_covers = Covers::new(dir.join("source-covers")).unwrap();
        let cover = source_covers.store(b"fake image", "jpg").unwrap();
        let mut source = db::open_in_memory().unwrap();
        catalog::create_book(
            &mut source,
            LIB,
            &book("A Wizard of Earthsea", Some("9780306406157")),
            Some(&cover),
        )
        .unwrap();
        catalog::create_book(&mut source, LIB, &book("The Dispossessed", None), None).unwrap();
        let file = dir.join("backup.zip");

        let exported = export(&source, &[LIB.to_owned()], &source_covers, &file).unwrap();

        let target_covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let restored = restore(&mut target, LIB, &target_covers, &file).unwrap();

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
            restored.restored[0].id, LIB,
            "same id, so it merges on sync"
        );
        assert_eq!(
            (restored.restored[0].books, restored.restored[0].copies),
            (2, 2)
        );
        assert_eq!(restored.covers, 1);
        let books = catalog::list_books(&target, LIB, &BookFilter::default()).unwrap();
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
        assert_eq!(catalog::list_books(&target, LIB, &filter).unwrap().len(), 1);
        let intent: String = target
            .query_row(
                "SELECT intent FROM operations WHERE intent <> 'device.register'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(intent, "backup.restore");
    }

    #[test]
    fn a_device_backup_brings_back_every_library_with_its_id_and_name() {
        let dir = temp("device");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut source = db::open_in_memory().unwrap();
        libraries::rename_library(&mut source, LIB, "Casa").unwrap();
        catalog::create_book(&mut source, LIB, &book("Home book", None), None).unwrap();
        let club = libraries::create_library(&mut source, "Club").unwrap();
        catalog::create_book(&mut source, &club, &book("Club book", None), None).unwrap();
        let file = dir.join("device.zip");

        let exported = export(&source, &all_libraries(&source).unwrap(), &covers, &file).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let restored = restore(&mut target, LIB, &covers, &file).unwrap();

        assert_eq!((exported.libraries, exported.books), (2, 2));
        assert_eq!(restored.restored.len(), 2);
        assert!(restored.skipped.is_empty());
        assert_eq!(name_of(&target, LIB), "Casa");
        assert_eq!(name_of(&target, &club), "Club");
        assert_eq!(titles(&target, LIB), vec!["Home book"]);
        assert_eq!(titles(&target, &club), vec!["Club book"]);
    }

    #[test]
    fn a_library_already_here_with_content_is_skipped() {
        let dir = temp("nonempty");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        catalog::create_book(&mut conn, LIB, &book("A", None), None).unwrap();
        let file = dir.join("backup.zip");
        export(&conn, &[LIB.to_owned()], &covers, &file).unwrap();

        let summary = restore(&mut conn, LIB, &covers, &file).unwrap();

        assert!(summary.restored.is_empty());
        assert_eq!(summary.skipped, vec![String::new()]);
        assert_eq!(titles(&conn, LIB), vec!["A"], "nothing duplicated");
    }

    #[test]
    fn a_library_deleted_here_comes_back_under_a_new_id() {
        let dir = temp("deleted");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        let club = libraries::create_library(&mut conn, "Club").unwrap();
        catalog::create_book(&mut conn, &club, &book("Club book", None), None).unwrap();
        let file = dir.join("club.zip");
        export(&conn, std::slice::from_ref(&club), &covers, &file).unwrap();
        libraries::delete_library(&mut conn, &club).unwrap();

        let summary = restore(&mut conn, LIB, &covers, &file).unwrap();

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
        export(&source, &[club], &covers, &file).unwrap();
        let mut target = db::open_in_memory().unwrap();
        libraries::create_library(&mut target, "club").unwrap();

        let summary = restore(&mut target, LIB, &covers, &file).unwrap();

        assert_eq!(summary.restored[0].name, "Club (2)");
    }

    #[test]
    fn a_version_2_backup_goes_into_the_active_library() {
        let dir = temp("v2");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut source = db::open_in_memory().unwrap();
        catalog::create_book(&mut source, LIB, &book("Old backup", None), None).unwrap();
        let mut entities = Rows::new();
        for entity in CONTENTS {
            entities.insert(
                entity.table().to_owned(),
                entity_rows(&source, LIB, entity).unwrap(),
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

        let summary = restore(&mut target, &other, &covers, &file).unwrap();

        assert_eq!(summary.restored[0].id, other);
        assert_eq!(titles(&target, &other), vec!["Old backup"]);
    }

    #[test]
    fn restore_rejects_files_that_are_not_backups() {
        let dir = temp("garbage");
        let file = dir.join("not-a-backup.zip");
        std::fs::write(&file, b"hello").unwrap();
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();

        assert!(matches!(
            restore(&mut conn, LIB, &covers, &file),
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
}
