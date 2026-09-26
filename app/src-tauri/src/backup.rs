//! Backups of one library: a zip with its catalogue as JSON, its covers, and its
//! operation log as history (D30). Restoring rebuilds an empty library as one
//! operation. Whole-device backups come with milestone 4.6.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use kosmotheka_domain::{Change, Entity};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::covers::{Covers, MAX_COVER_BYTES};
use crate::error::{Error, Result};
use crate::ops;
use crate::sync::entity_rows;

/// What a library holds; the library row itself is where it is restored to.
/// Same order as `sync::ORDER`: parents before children.
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
/// 2: one library per backup (D46).
const VERSION: u32 = 2;
const DATA_FILE: &str = "backup.json";
const COVERS_PREFIX: &str = "covers/";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupFile {
    format: String,
    version: u32,
    /// Database migration level; a backup restores only into the same level.
    schema_version: i64,
    exported_at: String,
    /// Rows per table, with the same field names operations use.
    entities: BTreeMap<String, Vec<Map<String, Value>>>,
    /// The operation log, kept as history. It is not replayed on restore.
    operations: Vec<Value>,
}

#[derive(Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    pub books: usize,
    pub copies: usize,
    pub covers: usize,
}

pub fn export(
    conn: &Connection,
    library: &str,
    covers: &Covers,
    path: &Path,
) -> Result<BackupSummary> {
    let mut entities = BTreeMap::new();
    for entity in CONTENTS {
        entities.insert(
            entity.table().to_owned(),
            entity_rows(conn, library, entity)?,
        );
    }
    let backup = BackupFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        schema_version: schema_version(conn)?,
        exported_at: conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |r| {
            r.get(0)
        })?,
        operations: operations(conn, library)?,
        entities,
    };

    let mut zip = ZipWriter::new(File::create(path)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(DATA_FILE, options)?;
    zip.write_all(&serde_json::to_vec_pretty(&backup)?)?;

    let cover_names: Vec<String> = backup.entities[Entity::Edition.table()]
        .iter()
        .filter_map(|row| row.get("cover")?.as_str().map(str::to_owned))
        .collect();
    let mut stored_covers = 0;
    for name in cover_names {
        // A cover file that went missing on disk should not make the whole backup fail.
        if let Ok(bytes) = std::fs::read(covers.dir().join(&name)) {
            zip.start_file(format!("{COVERS_PREFIX}{name}"), options)?;
            zip.write_all(&bytes)?;
            stored_covers += 1;
        }
    }
    zip.finish()?;

    Ok(BackupSummary {
        books: backup.entities[Entity::Edition.table()].len(),
        copies: backup.entities[Entity::Copy.table()].len(),
        covers: stored_covers,
    })
}

/// Restores into an empty library only: merging two catalogues is what sync will do.
pub fn restore(
    conn: &mut Connection,
    library: &str,
    covers: &Covers,
    path: &Path,
) -> Result<BackupSummary> {
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
    if ![FORMAT, LEGACY_FORMAT].contains(&backup.format.as_str()) || backup.version != VERSION {
        return Err(Error::BadBackup("not_a_backup"));
    }
    if backup.schema_version != schema_version(conn)? {
        return Err(Error::BadBackup("other_version"));
    }
    if !library_is_empty(conn, library)? {
        return Err(Error::BadBackup("not_empty"));
    }

    let mut changes = Vec::new();
    for entity in CONTENTS {
        for row in backup.entities.get(entity.table()).into_iter().flatten() {
            let mut fields = row.clone();
            let id = match fields.remove("id") {
                Some(Value::String(id)) => id,
                _ => return Err(Error::BadBackup("unreadable")),
            };
            changes.push(Change::create(entity, id, Value::Object(fields)));
        }
    }
    let covers_in_zip = read_covers(&mut archive)?;

    let tx = conn.transaction()?;
    if !changes.is_empty() {
        ops::record(&tx, library, "backup.restore", changes)?;
    }
    tx.commit()?;

    let mut stored = 0;
    for (name, extension, bytes) in covers_in_zip {
        // The stored name is recomputed from the bytes: a tampered file gets a different
        // name and simply is not the one any edition points to.
        if covers.store(&bytes, extension)? == name {
            stored += 1;
        }
    }

    Ok(BackupSummary {
        books: count(conn, library, Entity::Edition)?,
        copies: count(conn, library, Entity::Copy)?,
        covers: stored,
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

        let exported = export(&source, LIB, &source_covers, &file).unwrap();

        let target_covers = Covers::new(dir.join("target-covers")).unwrap();
        let mut target = db::open_in_memory().unwrap();
        let restored = restore(&mut target, LIB, &target_covers, &file).unwrap();

        assert_eq!(
            exported,
            BackupSummary {
                books: 2,
                copies: 2,
                covers: 1
            }
        );
        assert_eq!(restored, exported);
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
            .query_row("SELECT intent FROM operations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(intent, "backup.restore");
    }

    #[test]
    fn restore_refuses_a_non_empty_catalogue() {
        let dir = temp("nonempty");
        let covers = Covers::new(dir.join("covers")).unwrap();
        let mut conn = db::open_in_memory().unwrap();
        catalog::create_book(&mut conn, LIB, &book("A", None), None).unwrap();
        let file = dir.join("backup.zip");
        export(&conn, LIB, &covers, &file).unwrap();

        let error = restore(&mut conn, LIB, &covers, &file).unwrap_err();

        assert!(matches!(error, Error::BadBackup("not_empty")));
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
