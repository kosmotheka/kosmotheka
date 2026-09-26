//! Where the app keeps its data. The folder is named after the app identifier, so
//! when the identifier changed (D70) the old folder has to be carried over, or
//! the app would start empty. See docs/plans/renombrado-kosmotheka.md.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::error::Result;

/// The identifier the app had before it was called Kosmotheka.
const LEGACY_IDENTIFIER: &str = "com.tersoal.library";
pub const DATABASE_FILE: &str = "library.sqlite3";
/// What is the app's own, besides the database. The webview keeps its data in the
/// same folder and creates it before the app starts, so that is left alone.
const OWN_FILES: [&str; 3] = ["covers", "sync-cert.der", "sync-key.der"];

/// The data folder of the old identifier, next to the current one.
pub fn legacy_dir(current: &Path) -> Option<PathBuf> {
    Some(current.parent()?.join(LEGACY_IDENTIFIER))
}

/// While the current folder has no database yet, copies the old data into it so
/// nothing seems lost. Copies, never moves: the old folder is left as it was, so
/// going back to the previous version of the app still finds everything. Returns
/// whether it copied.
pub fn copy_legacy_data(current: &Path, legacy: &Path) -> Result<bool> {
    let database = current.join(DATABASE_FILE);
    if database.exists() || !legacy.join(DATABASE_FILE).exists() {
        return Ok(false);
    }
    fs::create_dir_all(current)?;
    for name in OWN_FILES {
        let from = legacy.join(name);
        if from.is_dir() {
            copy_dir(&from, &current.join(name))?;
        } else if from.exists() {
            // fs::copy keeps the permissions too: the sync key stays private.
            fs::copy(&from, current.join(name))?;
        }
    }

    // The database goes last, and only takes its real name once complete: if the
    // copy is cut halfway, the next start sees no database and copies again.
    // VACUUM INTO also picks up changes still in the WAL file, and a read-only
    // connection leaves the old files untouched.
    let partial = current.join(format!("{DATABASE_FILE}.partial"));
    if partial.exists() {
        fs::remove_file(&partial)?;
    }
    let old = Connection::open_with_flags(
        legacy.join(DATABASE_FILE),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    old.execute("VACUUM INTO ?1", [partial.to_string_lossy().as_ref()])?;
    drop(old);
    fs::rename(&partial, &database)?;
    Ok(true)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kosmotheka-data-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// An old data folder as the app leaves it: a WAL database with unsaved pages,
    /// covers, the sync identity and webview caches.
    fn legacy_folder(root: &Path) -> (PathBuf, Connection) {
        let legacy = root.join(LEGACY_IDENTIFIER);
        fs::create_dir_all(legacy.join("covers")).unwrap();
        fs::create_dir_all(legacy.join("WebKitCache")).unwrap();
        fs::write(legacy.join("covers/abc.jpg"), b"cover").unwrap();
        fs::write(legacy.join("sync-key.der"), b"key").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                legacy.join("sync-key.der"),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap();
        }
        let conn = Connection::open(legacy.join(DATABASE_FILE)).unwrap();
        conn.pragma_update(None, "journal_mode", "WAL").unwrap();
        conn.execute_batch(
            "PRAGMA wal_autocheckpoint = 0;
             CREATE TABLE members (name TEXT);
             INSERT INTO members VALUES ('Ana');",
        )
        .unwrap();
        // Kept open, so the insert stays in the WAL file, as with the app running.
        (legacy, conn)
    }

    #[test]
    fn copies_the_old_folder_and_leaves_it_as_it_was() {
        let root = temp("copy");
        let (legacy, _open) = legacy_folder(&root);
        let current = root.join("app.kosmotheka");

        assert!(copy_legacy_data(&current, &legacy).unwrap());

        let copied = Connection::open(current.join(DATABASE_FILE)).unwrap();
        let name: String = copied
            .query_row("SELECT name FROM members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "Ana", "changes still in the WAL file are copied too");
        assert_eq!(fs::read(current.join("covers/abc.jpg")).unwrap(), b"cover");
        assert!(
            !current.join("WebKitCache").exists(),
            "webview data is not copied"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(current.join("sync-key.der"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "the sync key stays private");
        }
        assert!(
            legacy.join(DATABASE_FILE).exists(),
            "the old folder is kept"
        );
        assert!(legacy.join("covers/abc.jpg").exists());
        assert!(!current.join("library.sqlite3.partial").exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn copies_into_a_folder_the_webview_already_created() {
        let root = temp("webview");
        let (legacy, _open) = legacy_folder(&root);
        let current = root.join("app.kosmotheka");
        fs::create_dir_all(current.join("WebKitCache")).unwrap();

        assert!(copy_legacy_data(&current, &legacy).unwrap());
        assert!(current.join(DATABASE_FILE).exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn does_nothing_once_the_new_folder_has_a_database_or_without_old_data() {
        let root = temp("noop");
        let (legacy, _open) = legacy_folder(&root);
        let current = root.join("app.kosmotheka");
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join(DATABASE_FILE), b"").unwrap();

        assert!(!copy_legacy_data(&current, &legacy).unwrap());
        assert!(!current.join("covers").exists());
        assert!(!copy_legacy_data(&root.join("other"), &root.join("missing")).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_old_folder_sits_next_to_the_new_one() {
        assert_eq!(
            legacy_dir(Path::new("/home/ana/.local/share/app.kosmotheka")),
            Some(PathBuf::from("/home/ana/.local/share/com.tersoal.library"))
        );
    }
}
