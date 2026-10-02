//! Leaves the app as if just installed (D81): run when the server tells this
//! device it was removed from the account (`account::sign_in` and every
//! authenticated call react to that), and on purpose, "Dejar este dispositivo a
//! otra persona" (`account::give_device_away`), with or without a signed-in
//! account.

use std::path::Path;

use crate::covers::Covers;
use crate::db::Database;
use crate::error::Result;
use crate::lan::service::LanService;

/// Tauri state: the data directory computed once in `lib.rs::setup`, so commands
/// that need it (this one) do not recompute it.
#[derive(Clone)]
pub struct DataDir(pub std::path::PathBuf);

/// The testable core of the reset: no `AppHandle`, so it takes plain arguments.
/// The Tauri command wrapping this is the one that emits the `device-reset` event,
/// since only it has the handle to emit with.
pub async fn reset(
    database: &Database,
    covers: &Covers,
    lan: &LanService,
    data_dir: &Path,
) -> Result<()> {
    database.reset()?;
    clear_covers(covers)?;
    lan.reset(data_dir).await?;
    Ok(())
}

fn clear_covers(covers: &Covers) -> Result<()> {
    for entry in std::fs::read_dir(covers.dir())?.flatten() {
        let path = entry.path();
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::identity::Identity;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "library-device-reset-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn resets_the_database_the_covers_and_the_lan_identity() {
        let dir = temp_dir("full");
        let database = Database::in_memory();
        let covers = Covers::new(dir.join("covers")).unwrap();
        covers.store(b"cover bytes", "jpg").unwrap();
        assert_eq!(std::fs::read_dir(covers.dir()).unwrap().count(), 1);
        let lan = LanService::new(database.clone(), Identity::generate().unwrap());
        let before_device_id = { crate::ops::device_id(&database.lock().unwrap()).unwrap() };

        reset(&database, &covers, &lan, &dir).await.unwrap();

        assert_eq!(
            std::fs::read_dir(covers.dir()).unwrap().count(),
            0,
            "covers are gone but the folder stays"
        );
        assert!(covers.dir().exists());
        let after_device_id = { crate::ops::device_id(&database.lock().unwrap()).unwrap() };
        assert_ne!(before_device_id, after_device_id);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
