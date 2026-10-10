//! Syncing libraries with the cloud (D84, milestone 7.2). Server side and routes:
//! docs/architecture/cloud.md.
//!
//! The cloud carries the same signed operations as the LAN, judged by the same
//! `sync::merge`, but its door is different: a library is tied to the cloud only by
//! its own signed link operation (`library.cloud`), never by an id the server says.
//! Network calls never hold the database guard: read, drop it, await, lock again.

pub mod client;
pub mod events;
#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;

use kosmotheka_domain::{Change, Entity};

use crate::db::Database;
use crate::error::{Error, Result};
use crate::sync::{self, LibraryVectors, MergeReport, SyncOperation, VersionVector};
use crate::{libraries, ops, people};

/// The five calls the client needs, behind a trait so tests run against an in-memory
/// cloud instead of the network.
#[allow(async_fn_in_trait)]
pub trait CloudApi {
    /// Base URL of the server this account lives on.
    fn server(&self) -> &str;
    /// Creates the cloud library; the answer carries its id and, if the server has a secret, the cloud's device (D95).
    async fn create_library(&self) -> Result<CloudLibrary>;
    async fn list_libraries(&self) -> Result<Vec<CloudLibrary>>;
    /// Asks the server to link this account with its person wherever this device is already that person's
    /// with a vouched key (D76, D96). Sends nothing: the server knows the device from the session.
    async fn claim(&self) -> Result<Claimed>;
    async fn pull(
        &self,
        cloud: &str,
        schema_version: i64,
        vector: &VersionVector,
    ) -> Result<Pulled>;
    /// Ties the cloud library to the local one with the signed link operation, which
    /// the server checks and does not store: it comes with the log, in its place.
    async fn link(&self, cloud: &str, link: &SyncOperation) -> Result<()>;
    /// `schema_version` is this database's; each operation carries the one it was recorded at.
    async fn push(
        &self,
        cloud: &str,
        schema_version: i64,
        operations: &[SyncOperation],
    ) -> Result<PushReport>;
}

/// What a claim linked: cloud library ids, and those where the account is already another person.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct Claimed {
    pub linked: Vec<String>,
    pub conflicts: Vec<String>,
    /// Libraries where the cloud has just said this device is its person's (D76, D96).
    #[serde(default)]
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CloudLibrary {
    pub id: String,
    /// Nobody links it until the first push brings the link operation.
    pub local_library_id: Option<String>,
    /// The cloud's own device in this library and its public key (D95); none when the server has no secret.
    pub cloud_device_id: Option<String>,
    pub cloud_public_key: Option<String>,
}

impl CloudLibrary {
    /// The id of the cloud's device, if the server gave one **and** it derives from the key it gave (rule 12 of
    /// `spec/README.md`). A server that lies about it gains nothing, and is not followed.
    fn trusted_cloud_device(&self) -> Option<&str> {
        let (id, key) = self
            .cloud_device_id
            .as_deref()
            .zip(self.cloud_public_key.as_deref())?;
        kosmotheka_domain::cloud_device_id(key)
            .is_some_and(|derived| derived.eq_ignore_ascii_case(id))
            .then_some(id)
    }
}

#[derive(Debug, Deserialize)]
pub struct Pulled {
    pub operations: Vec<SyncOperation>,
    pub vector: VersionVector,
}

#[derive(Debug, Default, Deserialize)]
pub struct PushReport {
    pub received: usize,
    pub known: usize,
    pub rejected: usize,
}

/// What happened to one library in a sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Synced,
    /// First time here: the library came down from the cloud.
    Downloaded,
    /// This device has a library with the cloud library's local id that its own log
    /// never linked to it: nothing was merged.
    LibraryConflict,
    /// What the cloud offered under that id was not a properly linked library: what
    /// had been merged was removed again.
    NotLinked,
    /// Linked here to a cloud library the account no longer lists.
    MissingInCloud,
    /// Linked to a cloud on another server than the account's.
    OtherServer,
    /// This device deleted or left it: the cloud does not bring it back by itself.
    DeletedHere,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryReport {
    pub library: String,
    pub cloud_library: Option<String>,
    pub outcome: Outcome,
    pub received: usize,
    pub sent: usize,
    /// Operations dropped: from another library, malformed or unsigned, or refused
    /// by the cloud.
    pub rejected: usize,
}

impl LibraryReport {
    fn new(library: &str, cloud: Option<&str>, outcome: Outcome) -> Self {
        Self {
            library: library.to_owned(),
            cloud_library: cloud.map(str::to_owned),
            outcome,
            received: 0,
            sent: 0,
            rejected: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCloudStatus {
    pub library: String,
    pub cloud_library: Option<String>,
    pub cloud_server: Option<String>,
    /// Linked to a cloud library on the account's current server.
    pub linked_here: bool,
    pub signed_in: bool,
    pub may_share: bool,
    /// Its log has operations without a signature: it cannot go up (D58 offers
    /// «Duplicate as new»).
    pub unsigned_history: bool,
    pub can_upload: bool,
}

struct Link {
    cloud_library: Option<String>,
    cloud_server: Option<String>,
}

/// `None` when this device has no such library.
fn link_of(conn: &Connection, library: &str) -> Result<Option<Link>> {
    Ok(conn
        .query_row(
            "SELECT cloud_library_id, cloud_server FROM libraries WHERE id = ?1",
            [library],
            |row| {
                Ok(Link {
                    cloud_library: row.get(0)?,
                    cloud_server: row.get(1)?,
                })
            },
        )
        .optional()?)
}

impl Link {
    fn is(&self, cloud: &str, server: &str) -> bool {
        self.cloud_library.as_deref() == Some(cloud) && self.cloud_server.as_deref() == Some(server)
    }
}

fn has_unsigned_history(conn: &Connection, library: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM operations
          WHERE library_id = ?1 AND replicable = 1 AND signature IS NULL)",
        [library],
        |row| row.get(0),
    )?)
}

/// The link, which also names the cloud's device when the server gave one (D76, D95). The device's own row is
/// not here: only a device creates its row, so the cloud registers itself.
fn link_change(library: &str, cloud: &str, server: &str, cloud_device: Option<&str>) -> Change {
    let mut fields = json!({ "cloud_library_id": cloud, "cloud_server": server });
    if let Some(device) = cloud_device {
        fields["cloud_device_id"] = json!(device);
    }
    Change::update(Entity::Library, library, fields)
}

/// Local only, no network: what each library can do about the cloud.
pub fn status(database: &Database) -> Result<Vec<LibraryCloudStatus>> {
    let conn = database.lock()?;
    let signed_in = crate::account::is_signed_in(&conn)?;
    let server = crate::account::server(&conn)?;
    let mut statuses = Vec::new();
    for library in sync::libraries(&conn)? {
        let link = link_of(&conn, &library)?;
        let may_share = people::may_share(&conn, &library)?;
        let administers = people::administers(&conn, &library)?;
        let unsigned_history = has_unsigned_history(&conn, &library)?;
        let linked_here = link.as_ref().is_some_and(|link| {
            link.cloud_library
                .as_deref()
                .is_some_and(|cloud| link.is(cloud, &server))
        });
        let is_linked = link.as_ref().is_some_and(|l| l.cloud_library.is_some());
        statuses.push(LibraryCloudStatus {
            can_upload: signed_in && administers && !unsigned_history && !is_linked,
            cloud_library: link.as_ref().and_then(|l| l.cloud_library.clone()),
            cloud_server: link.and_then(|l| l.cloud_server),
            linked_here,
            signed_in,
            may_share,
            unsigned_history,
            library,
        });
    }
    Ok(statuses)
}

/// Takes a library to the cloud: creates its cloud id, records the signed link,
/// links the cloud library with it and sends the whole log (the link is in it). Every check comes before anything is
/// created or written, so a refusal changes nothing.
pub async fn upload<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
) -> Result<LibraryReport> {
    let server = api.server().to_owned();
    {
        let conn = database.lock()?;
        if !libraries::library_exists(&conn, library)? {
            return Err(Error::MissingRow {
                entity: "library",
                id: library.to_owned(),
            });
        }
        if !people::administers(&conn, library)? {
            return Err(Error::NotAllowed);
        }
        if has_unsigned_history(&conn, library)? {
            return Err(Error::Cloud("unsigned_history"));
        }
        ensure_not_linked(&conn, library)?;
        // Linking is a change to the library row: with roles on, administrators only.
        let probe = vec![link_change(library, "pending", &server, None)];
        if !ops::would_allow(&conn, library, probe)? {
            return Err(Error::NotAllowed);
        }
    }

    let created = api.create_library().await?;
    let cloud = created.id.clone();
    let cloud_device = created.trusted_cloud_device().map(str::to_owned);

    {
        let mut conn = database.lock()?;
        // Something may have linked it while the server answered.
        ensure_not_linked(&conn, library)?;
        let tx = conn.transaction()?;
        ops::record(
            &tx,
            library,
            "library.cloud",
            vec![link_change(
                library,
                &cloud,
                &server,
                cloud_device.as_deref(),
            )],
        )?;
        tx.commit()?;
    }
    sync_library(database, api, library).await
}

fn ensure_not_linked(conn: &Connection, library: &str) -> Result<()> {
    match link_of(conn, library)? {
        Some(link) if link.cloud_library.is_some() => Err(Error::Cloud("already_linked")),
        _ => Ok(()),
    }
}

/// One session for one library linked to the account's server: pull, merge, push
/// what the cloud lacks. Anything else is skipped and said so in the report.
pub async fn sync_library<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
) -> Result<LibraryReport> {
    let link = link_of(&*database.lock()?, library)?.ok_or_else(|| Error::MissingRow {
        entity: "library",
        id: library.to_owned(),
    })?;
    let Some(cloud) = link.cloud_library.as_deref() else {
        return Ok(LibraryReport::new(library, None, Outcome::NotLinked));
    };
    if !link.is(cloud, api.server()) {
        return Ok(LibraryReport::new(
            library,
            Some(cloud),
            Outcome::OtherServer,
        ));
    }
    let mut report = LibraryReport::new(library, Some(cloud), Outcome::Synced);
    let vector = {
        let conn = database.lock()?;
        sync::version_vectors(&conn, &[library.to_owned()])?
            .remove(library)
            .unwrap_or_default()
    };
    let server_vector = pull_and_merge(database, api, library, cloud, &vector, &mut report).await?;
    push_missing(database, api, library, cloud, server_vector, &mut report).await?;
    Ok(report)
}

/// Pulls `cloud` and merges only the operations of `library`: the server's word
/// about which library an operation belongs to is not trusted (D84), so anything
/// else it sends is dropped and counted. Returns the server's vector.
async fn pull_and_merge<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    cloud: &str,
    vector: &VersionVector,
    report: &mut LibraryReport,
) -> Result<VersionVector> {
    let schema_version = ops::schema_version(&*database.lock()?)?;
    let pulled = api.pull(cloud, schema_version, vector).await?;
    let (kept, dropped): (Vec<_>, Vec<_>) = pulled
        .operations
        .into_iter()
        .partition(|synced| synced.operation.library == library);
    report.rejected += dropped.len();
    let merged: MergeReport = {
        let mut conn = database.lock()?;
        let merged = sync::merge(&mut conn, kept)?;
        // With what the cloud had already merged; push_missing then sends the seal (D94).
        crate::permissions::seal_if_missing(&mut conn, &[library.to_owned()])?;
        crate::permissions::vouch_if_missing(&mut conn, &[library.to_owned()])?;
        merged
    };
    report.received += merged.received;
    report.rejected += merged.rejected;
    Ok(pulled.vector)
}

async fn push_missing<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    cloud: &str,
    server_vector: VersionVector,
    report: &mut LibraryReport,
) -> Result<()> {
    let (schema_version, to_send) = {
        let conn = database.lock()?;
        let theirs = LibraryVectors::from([(library.to_owned(), server_vector)]);
        (
            ops::schema_version(&conn)?,
            sync::operations_since(&conn, &theirs)?,
        )
    };
    if to_send.is_empty() {
        return Ok(());
    }
    // The link is the newest operation of an upload and the cloud takes nothing before
    // it, so it goes first on its own. It is in what is left to send only until the
    // cloud stores it.
    if let Some(link) = to_send
        .iter()
        .find(|synced| links_to(&synced.operation, cloud))
    {
        api.link(cloud, link).await?;
    }
    let pushed = api.push(cloud, schema_version, &to_send).await?;
    report.sent += to_send.len();
    report.rejected += pushed.rejected;
    Ok(())
}

/// Syncs every library of the account: the ones linked here, and the ones this
/// device does not have yet, which come down.
pub async fn sync_all<A: CloudApi>(database: &Database, api: &A) -> Result<Vec<LibraryReport>> {
    // Before listing, so a library this account was just given access to is in the list. A server that
    // does not know the route, or a network that fails here, must not stop the session: the list says
    // what there is.
    let _ = api.claim().await;
    let listed = api.list_libraries().await?;
    let server = api.server();
    let linked = linked_libraries(&*database.lock()?)?;
    let mut reports = Vec::new();

    for (library, link) in &linked {
        if link.cloud_server.as_deref() != Some(server) {
            reports.push(LibraryReport::new(
                library,
                link.cloud_library.as_deref(),
                Outcome::OtherServer,
            ));
        }
    }
    for cloud in &listed {
        let here = linked
            .iter()
            .find(|(_, link)| link.is(&cloud.id, server))
            .map(|(library, _)| library);
        if let Some(library) = here {
            reports.push(sync_library(database, api, library).await?);
            continue;
        }
        let Some(local) = cloud.local_library_id.as_deref() else {
            continue; // created but never pushed: nothing to bring down
        };
        let (exists, known) = {
            let conn = database.lock()?;
            (
                libraries::library_exists(&conn, local)?,
                libraries::library_known(&conn, local)?
                    || libraries::library_forgotten(&conn, local)?,
            )
        };
        reports.push(if exists {
            if adopt_link(database, api, &cloud.id, local).await? {
                sync_library(database, api, local).await?
            } else {
                // Same id, but nothing this device's own log trusts links it to this
                // cloud library: a stranger's library that happens to share the id.
                LibraryReport::new(local, Some(&cloud.id), Outcome::LibraryConflict)
            }
        } else if known {
            LibraryReport::new(local, Some(&cloud.id), Outcome::DeletedHere)
        } else {
            download(database, api, &cloud.id, local).await?
        });
    }
    // A new phone has just registered itself in a library and sent the registration: the cloud may now say the
    // device is its account's person's. Ask only if something went up, and bring down what the cloud wrote. A
    // failure here costs the session nothing: the next one asks again.
    if reports.iter().any(|report| report.sent > 0)
        && let Ok(claimed) = api.claim().await
    {
        for cloud in claimed.devices {
            let Some(local) = reports
                .iter()
                .find(|report| report.cloud_library.as_deref() == Some(cloud.as_str()))
                .map(|report| report.library.clone())
            else {
                continue;
            };
            if let Ok(again) = sync_library(database, api, &local).await
                && let Some(report) = reports.iter_mut().find(|report| report.library == local)
            {
                report.received += again.received;
                report.sent += again.sent;
                report.rejected += again.rejected;
            }
        }
    }
    for (library, link) in &linked {
        let Some(cloud) = link.cloud_library.as_deref() else {
            continue;
        };
        if link.is(cloud, server) && !listed.iter().any(|listed| listed.id == cloud) {
            reports.push(LibraryReport::new(
                library,
                Some(cloud),
                Outcome::MissingInCloud,
            ));
        }
    }
    Ok(reports)
}

fn linked_libraries(conn: &Connection) -> Result<Vec<(String, Link)>> {
    let mut statement = conn.prepare(
        "SELECT id, cloud_library_id, cloud_server FROM libraries
          WHERE cloud_library_id IS NOT NULL ORDER BY id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            Link {
                cloud_library: row.get(1)?,
                cloud_server: row.get(2)?,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// A library this device already has, unlinked here, may have been uploaded by
/// another device of the family before this one heard of it over the LAN. The
/// link operation decides (D84): only if the pull holds one from `cloud` written by
/// a device already registered in this library is anything merged, and then all of
/// the library's operations, judged like any other (signature, permissions).
/// Whether the library ends up linked to `cloud` is the answer.
async fn adopt_link<A: CloudApi>(
    database: &Database,
    api: &A,
    cloud: &str,
    local: &str,
) -> Result<bool> {
    let (schema_version, vector) = {
        let conn = database.lock()?;
        let vectors = sync::version_vectors(&conn, &[local.to_owned()])?;
        (
            ops::schema_version(&conn)?,
            vectors.get(local).cloned().unwrap_or_default(),
        )
    };
    let pulled = api.pull(cloud, schema_version, &vector).await?;
    let mut conn = database.lock()?;
    let mut ours = Vec::new();
    let mut linked = false;
    for synced in pulled.operations {
        let operation = &synced.operation;
        if operation.library != local {
            continue;
        }
        if links_to(operation, cloud) && is_registered(&conn, local, &operation.hlc.device)? {
            linked = true;
        }
        ours.push(synced);
    }
    if !linked {
        return Ok(false);
    }
    // All of it, not just the link: the author's vector would otherwise jump over
    // what this device lacks between its last sync and the link (DAT-002).
    sync::merge(&mut conn, ours)?;
    Ok(link_of(&conn, local)?.is_some_and(|link| link.is(cloud, api.server())))
}

fn links_to(operation: &kosmotheka_domain::Operation, cloud: &str) -> bool {
    operation.changes.iter().any(|change| {
        change.entity == Entity::Library
            && change.id == operation.library
            && change
                .fields
                .get("cloud_library_id")
                .and_then(|v| v.as_str())
                == Some(cloud)
    })
}

fn is_registered(conn: &Connection, library: &str, device: &str) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM devices WHERE library_id = ?1 AND id = ?2)",
        [library, device],
        |row| row.get(0),
    )?)
}

/// Brings down a library this device has never had. What merged is kept only if the
/// library's own row says it is `cloud` on this server: the link operation came
/// with the log. Otherwise it was not a properly linked library and goes again.
async fn download<A: CloudApi>(
    database: &Database,
    api: &A,
    cloud: &str,
    local: &str,
) -> Result<LibraryReport> {
    let mut report = LibraryReport::new(local, Some(cloud), Outcome::Downloaded);
    let server_vector = pull_and_merge(
        database,
        api,
        local,
        cloud,
        &VersionVector::new(),
        &mut report,
    )
    .await?;
    let linked = {
        let conn = database.lock()?;
        link_of(&conn, local)?.is_some_and(|link| link.is(cloud, api.server()))
    };
    if !linked {
        remove_unlinked(&mut *database.lock()?, local)?;
        report.outcome = Outcome::NotLinked;
        report.received = 0;
        return Ok(report);
    }
    // A device registers itself the first time it writes in a library (D56); a new phone writes nothing here
    // yet, so it does it now, and the registration goes up with the rest: the cloud can then say whose it is
    // (D76, D96). Always allowed, whatever the role.
    {
        let mut conn = database.lock()?;
        let tx = conn.transaction()?;
        ops::ensure_registered(&tx, local)?;
        tx.commit()?;
    }
    push_missing(database, api, local, cloud, server_vector, &mut report).await?;
    Ok(report)
}

/// Forgets everything merged for a library that was never here before.
fn remove_unlinked(conn: &mut Connection, library: &str) -> Result<()> {
    let tx: Transaction<'_> = conn.transaction()?;
    ops::delete_library(&tx, library)?;
    for table in [
        "operations",
        "library_peers",
        "pending_owners",
        "request_notices",
    ] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE library_id = ?1"),
            [library],
        )?;
    }
    tx.commit()?;
    Ok(())
}
