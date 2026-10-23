//! Syncing libraries with the cloud (D84, milestone 7.2). Server side and routes:
//! docs/architecture/cloud.md.
//!
//! The cloud carries the same signed operations as the LAN, judged by the same
//! `sync::merge`, but its door is different: a library is tied to the cloud only by
//! its own signed link operation (`library.cloud`), never by an id the server says.
//! Network calls never hold the database guard: read, drop it, await, lock again.

pub mod client;
pub mod events;
pub mod invitations;
pub mod other_owner;
#[cfg(test)]
mod tests;

use rusqlite::{Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;

use kosmotheka_domain::{Change, Entity, Isbn};

use crate::db::Database;
use crate::error::{Error, Result};
use crate::lan;
use crate::metadata::Metadata;
use crate::sync::{self, LibraryVectors, MergeReport, SyncOperation, VersionVector};
use crate::{libraries, ops, people};

/// The calls the client needs, behind a trait so tests run against an in-memory
/// cloud instead of the network.
#[allow(async_fn_in_trait)]
pub trait CloudApi {
    /// Base URL of the server this account lives on.
    fn server(&self) -> &str;
    /// Creates the cloud library; the answer carries its id and, if the server has a secret, the cloud's device (D95).
    async fn create_library(&self) -> Result<CloudLibrary>;
    /// What the account reaches from this device, and the ids of the cloud libraries it had and was cut from (D98).
    async fn list_libraries(&self) -> Result<Listing>;
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
    /// Whether this server makes invitations (`GET /v1/server`, no session needed).
    async fn offers_invitations(&self) -> Result<bool>;
    /// What the links of this server's invitations start with, if it says (`GET /v1/server`). It is only ever
    /// compared with a link; the app never calls it.
    async fn invitation_base(&self) -> Result<Option<String>>;
    /// Asks for an invitation to `cloud` for the person `member`, which the server compares with its own
    /// catalog (D84). The link of a `link` invitation is in this answer and nowhere else.
    async fn create_invitation(
        &self,
        cloud: &str,
        member: &str,
        kind: invitations::InvitationKind,
        email: Option<&str>,
    ) -> Result<invitations::CreatedInvitation>;
    async fn list_invitations(&self, cloud: &str) -> Result<Vec<invitations::InvitationItem>>;
    async fn revoke_invitation(&self, cloud: &str, invitation: &str) -> Result<()>;
    /// Without a session: what whoever holds the link may know. Always asked of `server()`.
    async fn peek_invitation(&self, token: &str) -> Result<invitations::InvitationPreview>;
    /// With a session: the cloud library the account was let into.
    async fn accept_invitation(&self, token: &str) -> Result<String>;
    /// Whether this server looks books up in `provider` (`google_books`) for its accounts (`GET /v1/server`).
    async fn offers_metadata(&self, provider: &str) -> Result<bool>;
    /// A film or series searched by title in TMDB through the server (7.8, D62). The text goes in the body of a `POST`,
    /// never in an address. The answer is already cut to the fixed shape and to at most eight candidates.
    async fn tmdb_search(
        &self,
        kind: kosmotheka_domain::Kind,
        query: &str,
        language: &str,
    ) -> Result<Vec<crate::metadata::Candidate>>;
    /// A video game searched by title in RAWG through the server (7.8, D62). The text goes in the body of a `POST`,
    /// never in an address. The answer is already cut to the fixed shape and to at most eight candidates.
    async fn rawg_search(&self, query: &str) -> Result<Vec<crate::metadata::Candidate>>;
    /// A board game (BoardGameGeek) or a role-playing book (RPGGeek) searched by title through the server (7.8, D62).
    /// The text goes in the body of a `POST`, never in an address; the answer is already of the fixed shape, and none of
    /// its values is cut (BoardGameGeek's terms forbid changing its data).
    async fn bgg_search(
        &self,
        scope: crate::metadata::bgg::BggScope,
        query: &str,
    ) -> Result<Vec<crate::metadata::Candidate>>;
    /// A comic searched by title in Comic Vine through the server (7.8, D62): series and issues, the series first.
    /// The text goes in the body of a `POST`, never in an address; the answer is already cut to the fixed shape.
    async fn comic_vine_search(&self, query: &str) -> Result<Vec<crate::metadata::Candidate>>;
    /// Which board game a barcode is, asked of GameUPC through the server (D62). Only the barcode leaves the app.
    async fn gameupc_lookup(&self, code: &str) -> Result<crate::metadata::gameupc::GameUpcAnswer>;
    /// The game a person chose for a barcode, sent to GameUPC through the server (D62): GameUPC learns from it. The
    /// server, not the app, says to GameUPC which account it was, with an id that does not say whose it is.
    async fn gameupc_choice(&self, code: &str, external_id: u64) -> Result<()>;
    /// One ISBN in Google Books through the server (7.8, D83): `Ok(None)` is the server saying it is not found.
    /// Only the ISBN leaves the app.
    async fn google_books_isbn(&self, isbn: &Isbn) -> Result<Option<Metadata>>;
}

/// The answer to «what do I have?»: the libraries, and apart the ids of those the account had and was cut from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Listing {
    pub libraries: Vec<CloudLibrary>,
    pub withdrawn: Vec<String>,
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
            .is_some_and(|derived| derived == id)
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
    /// The cloud cut this device or its account from the library and proved it with the signed revocation of
    /// someone who administers it: the library was removed here (D98, D71).
    Removed,
    /// The cloud says the access was withdrawn but showed no proof this device accepts: nothing is removed. The
    /// library stays and is no longer synced with the cloud.
    AccessWithdrawn,
    /// This library's step failed (`LibraryReport::error` says why, if it has a code) and the session went on
    /// with the others (DAT-009).
    Failed,
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
    /// With `Outcome::Failed`: the code of the error, when it has one the UI can translate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The library's name, kept for `Removed`, when the library is gone by the time the UI tells it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    /// The account has just been let into this library as this device's person (D76), the first time this
    /// device hears of it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined: Option<Joined>,
    /// With `Outcome::Downloaded`: who this device is in the library when the session ends, if it already
    /// is someone (the cloud links a new device in the same session, D76).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub me: Option<Joined>,
}

/// Who this device's account became in a library the cloud let it into without an invitation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Joined {
    pub person: String,
    pub role: Option<String>,
}

impl LibraryReport {
    /// A library whose step failed with `error` (DAT-009).
    fn failed(library: &str, cloud: &str, error: &Error) -> Self {
        let mut report = Self::new(library, Some(cloud), Outcome::Failed);
        report.error = match error {
            Error::Cloud(code) => Some((*code).to_owned()),
            Error::Account { code, .. } => Some(code.clone()),
            _ => None,
        };
        report
    }

    fn new(library: &str, cloud: Option<&str>, outcome: Outcome) -> Self {
        Self {
            library: library.to_owned(),
            cloud_library: cloud.map(str::to_owned),
            outcome,
            received: 0,
            sent: 0,
            rejected: 0,
            error: None,
            library_name: None,
            joined: None,
            me: None,
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
    /// This device administers the library: only then it can upload it or change who
    /// the cloud may link (D76).
    pub administers: bool,
    /// The library names a cloud device: the cloud may say that a new device of a linked account is its
    /// person's (D76, D95).
    pub cloud_power: bool,
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
            administers,
            cloud_power: people::cloud_device_of(&conn, &library)?.is_some(),
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
    upload_with(database, api, library, true).await
}

/// `upload`, choosing whether the library names the cloud as a device that may link new ones (D76): the box
/// of the upload screen. Without it each new device is linked by hand, as without the cloud.
pub async fn upload_with<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    autolink: bool,
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
    let cloud_device = created
        .trusted_cloud_device()
        .filter(|_| autolink)
        .map(str::to_owned);

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

/// Gives the cloud the power to say whose a new device is (D76, D95), or takes it away: the switch in the
/// settings of a library that is in the cloud. Only whoever administers it (`people::administers`); the
/// change is a signed operation like any other, so the judge has the last word on who may make it.
pub async fn set_cloud_power<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    on: bool,
) -> Result<()> {
    let (cloud, named) = {
        let conn = database.lock()?;
        if !people::administers(&conn, library)? {
            return Err(Error::NotAllowed);
        }
        let link = link_of(&conn, library)?.ok_or_else(|| Error::MissingRow {
            entity: "library",
            id: library.to_owned(),
        })?;
        let cloud = link
            .cloud_library
            .clone()
            .filter(|cloud| link.is(cloud, api.server()))
            .ok_or(Error::Cloud("not_found"))?;
        (cloud, people::cloud_device_of(&conn, library)?)
    };
    if on == named.is_some() {
        return Ok(());
    }
    if on {
        // The cloud's device id is the server's word, believed only if it derives from the key it gives (D95).
        let listing = api.list_libraries().await?;
        let device = listing
            .libraries
            .iter()
            .find(|listed| listed.id == cloud)
            .and_then(|listed| listed.trusted_cloud_device())
            .map(str::to_owned)
            .ok_or(Error::Cloud("no_cloud_device"))?;
        people::name_cloud_device(&mut *database.lock()?, library, &device)?;
    } else {
        people::remove_cloud_device(&mut *database.lock()?, library)?;
    }
    // Sent now, so the cloud registers itself (or stops being believed) without waiting for a change.
    sync_library(database, api, library).await?;
    Ok(())
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
    let server_vector =
        match pull_and_merge(database, api, library, cloud, &vector, &mut report).await {
            Err(Error::AccessWithdrawn(proofs)) => {
                return cut_off(database, library, cloud, &proofs);
            }
            other => other?,
        };
    match push_missing(database, api, library, cloud, server_vector, &mut report).await {
        Err(Error::AccessWithdrawn(_)) => {
            // The proofs of a push are the revocations alone; a pull, with this device's vector, also brings the
            // earlier operations of who signed them, so the revocation chains.
            let proofs = api_pull_proofs(database, api, library, cloud)
                .await?
                .unwrap_or_default();
            cut_off(database, library, cloud, &proofs)
        }
        other => other.map(|()| report),
    }
}

/// The cloud says this device, or its account, lost the library (D98, D71). It is believed only through the
/// signed revocation of a device allowed to make it, checked here exactly as the local network does
/// (`lan::remove_if_proved`): then the library is removed from this device. Without such a proof nothing is
/// removed: a server that lies cannot make anyone lose a library.
fn cut_off(
    database: &Database,
    library: &str,
    cloud: &str,
    proofs: &[SyncOperation],
) -> Result<LibraryReport> {
    let (removed, name) = {
        let mut conn = database.lock()?;
        let name: Option<String> = conn
            .query_row(
                "SELECT name FROM libraries WHERE id = ?1",
                [library],
                |row| row.get(0),
            )
            .optional()?;
        (
            lan::remove_if_proved(&mut conn, library, proofs, "the cloud")?,
            name,
        )
    };
    let mut report = LibraryReport::new(
        library,
        Some(cloud),
        if removed {
            Outcome::Removed
        } else {
            Outcome::AccessWithdrawn
        },
    );
    if removed {
        report.library_name = name;
    }
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
        // With what the cloud had already merged; push_missing then sends what it wrote (D96).
        crate::permissions::vouch_if_missing(&mut conn, &[library.to_owned()])?;
        crate::permissions::complete_withdrawals(&mut conn, &[library.to_owned()])?;
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

/// Whether `error` belongs to the whole session and not to one library: no session, a schema version the
/// server does not take, the server unreachable or refusing the account (rate limit, device removed), or
/// this device's own database or HTTP stack failing. Anything else is about the library being synced.
fn stops_the_session(error: &Error) -> bool {
    matches!(
        error,
        Error::Cloud("not_signed_in" | "schema_version")
            | Error::Account { .. }
            | Error::Http(_)
            | Error::Io(_)
            | Error::Sqlite(_)
            | Error::LockPoisoned
    )
}

/// The report of one library's step, or, if the step failed for a reason of that library alone, a report
/// saying so: the session goes on with the others (DAT-009).
fn or_failed(result: Result<LibraryReport>, library: &str, cloud: &str) -> Result<LibraryReport> {
    match result {
        Err(error) if !stops_the_session(&error) => {
            Ok(LibraryReport::failed(library, cloud, &error))
        }
        other => other,
    }
}

/// Syncs every library of the account: the ones linked here, and the ones this
/// device does not have yet, which come down. A library that fails for its own reasons is reported as
/// failed and the rest go on; the session stops only for what concerns all of them (`stops_the_session`).
pub async fn sync_all<A: CloudApi>(database: &Database, api: &A) -> Result<Vec<LibraryReport>> {
    // Before listing, so a library this account was just given access to is in the list. A server that
    // does not know the route, or a network that fails here, must not stop the session: the list says
    // what there is.
    let first_claim = api.claim().await.ok();
    let mut joined_in: Vec<String> = first_claim
        .as_ref()
        .map(|claimed| claimed.linked.clone())
        .unwrap_or_default();
    let mut conflicts: Vec<String> = first_claim
        .as_ref()
        .map(|claimed| claimed.conflicts.clone())
        .unwrap_or_default();
    let listing = api.list_libraries().await?;
    let (listed, withdrawn) = (listing.libraries, listing.withdrawn);
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
            reports.push(or_failed(
                sync_library(database, api, library).await,
                library,
                &cloud.id,
            )?);
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
        let step = if exists {
            match adopt_link(database, api, &cloud.id, local).await {
                Ok(true) => sync_library(database, api, local).await,
                // Same id, but nothing this device's own log trusts links it to this
                // cloud library: a stranger's library that happens to share the id.
                Ok(false) => Ok(LibraryReport::new(
                    local,
                    Some(&cloud.id),
                    Outcome::LibraryConflict,
                )),
                Err(error) => Err(error),
            }
        } else if known {
            Ok(LibraryReport::new(
                local,
                Some(&cloud.id),
                Outcome::DeletedHere,
            ))
        } else {
            download(database, api, &cloud.id, local).await
        };
        reports.push(or_failed(step, local, &cloud.id)?);
    }
    // Libraries of this device that the cloud says were cut: it asks for the proof, and removes only if it holds.
    for cloud in &withdrawn {
        let Some(library) = linked
            .iter()
            .find(|(_, link)| link.is(cloud, server))
            .map(|(library, _)| library)
        else {
            continue; // not here: nothing to remove, and nothing to bring down
        };
        match api_pull_proofs(database, api, library, cloud).await? {
            Some(proofs) => reports.push(cut_off(database, library, cloud, &proofs)?),
            // Whatever else it says is not a proof of anything: the library stays as it is.
            None => reports.push(LibraryReport::new(
                library,
                Some(cloud),
                Outcome::AccessWithdrawn,
            )),
        }
    }
    // A new phone has just registered itself in a library and sent the registration: the cloud may now say the
    // device is its account's person's. Ask only if something went up, and bring down what the cloud wrote. A
    // failure here costs the session nothing: the next one asks again.
    if reports.iter().any(|report| report.sent > 0)
        && let Ok(claimed) = api.claim().await
    {
        joined_in.extend(claimed.linked);
        conflicts.extend(claimed.conflicts);
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
        if link.is(cloud, server)
            && !listed.iter().any(|listed| listed.id == cloud)
            && !withdrawn.iter().any(|id| id == cloud)
        {
            reports.push(LibraryReport::new(
                library,
                Some(cloud),
                Outcome::MissingInCloud,
            ));
        }
    }
    // The accounts let in meanwhile, told the first time this device hears of it.
    for report in &mut reports {
        let Some(cloud) = report.cloud_library.as_deref() else {
            continue;
        };
        if joined_in.iter().any(|id| id == cloud)
            && matches!(report.outcome, Outcome::Synced | Outcome::Downloaded)
        {
            let joined = note_joined(&*database.lock()?, &report.library)?;
            // A library that came down in this session already has its own notice.
            report.joined = joined.filter(|_| report.outcome == Outcome::Synced);
        }
    }
    for report in reports
        .iter_mut()
        .filter(|report| report.outcome == Outcome::Downloaded)
    {
        report.me = this_device_as(&*database.lock()?, &report.library)?;
    }
    // Where the account is one person and this device is another's: remembered here, asked once (D76).
    // Only a first claim that worked says which conflicts are over.
    {
        let conn = database.lock()?;
        for report in &reports {
            let Some(cloud) = report.cloud_library.as_deref() else {
                continue;
            };
            if conflicts.iter().any(|id| id == cloud) {
                other_owner::note(&conn, &report.library)?;
            } else if first_claim.is_some() {
                other_owner::clear(&conn, &report.library)?;
            }
        }
    }
    Ok(reports)
}

/// The first time this device learns that its account was let into `library` (D76): who it is there, unless
/// this device itself linked the library to the cloud (then nobody let it in). The mark is local state, never an
/// operation.
fn note_joined(conn: &Connection, library: &str) -> Result<Option<Joined>> {
    let mark = format!("cloud_joined.{library}");
    if ops::read_state(conn, &mark)?.is_some() {
        return Ok(None);
    }
    ops::write_state(conn, &mark, Some("1"))?;
    let me = ops::device_id(conn)?;
    let uploaded_here: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM operations
          WHERE library_id = ?1 AND device_id = ?2 AND intent = 'library.cloud')",
        [library, &me],
        |row| row.get(0),
    )?;
    if uploaded_here {
        return Ok(None);
    }
    this_device_as(conn, library)
}

/// The person this device belongs to in `library`, with their role, if any.
fn this_device_as(conn: &Connection, library: &str) -> Result<Option<Joined>> {
    Ok(people::list_people(conn, library)?
        .people
        .into_iter()
        .find(|person| person.devices.iter().any(|d| d.this_device))
        .map(|person| Joined {
            person: person.name,
            role: person.role,
        }))
}

/// Whether the cloud of this account offers `library` right now: it is linked here, the cloud lists it with this
/// very local id and it is not among those the account was cut from. Asks the cloud, because the link is only what
/// this device remembers (`MissingInCloud`). Any failure, no connection included, is a no: leaving would lose it.
pub async fn offers<A: CloudApi>(database: &Database, api: &A, library: &str) -> bool {
    let cloud = {
        let Ok(conn) = database.lock() else {
            return false;
        };
        // The session is `api`'s: no api without one.
        match link_of(&conn, library) {
            Ok(Some(link)) => link
                .cloud_library
                .clone()
                .filter(|cloud| link.is(cloud, api.server())),
            _ => None,
        }
    };
    let Some(cloud) = cloud else {
        return false;
    };
    let Ok(listing) = api.list_libraries().await else {
        return false;
    };
    !listing.withdrawn.contains(&cloud)
        && listing.libraries.iter().any(|offered| {
            offered.id == cloud && offered.local_library_id.as_deref() == Some(library)
        })
}

/// Leaves a library on this device. Another device of the local network having it is enough; failing that, the
/// cloud has to offer it to this account (`offers`), or it would be lost: `OnlyOnThisDevice`.
pub async fn leave<A: CloudApi>(database: &Database, api: &A, library: &str) -> Result<String> {
    let alone = lan::devices_sharing(&*database.lock()?, library)?.is_empty();
    let in_cloud = alone && offers(database, api, library).await;
    let mut conn = database.lock()?;
    if in_cloud {
        libraries::leave_library_in_cloud(&mut conn, library)
    } else {
        libraries::leave_library(&mut conn, library)
    }
}

/// Brings down a library this device left by its own choice, when the person asks for it. Not `sync_all`'s
/// job: that one never brings back what was left (`Outcome::DeletedHere`).
///
/// Only a library in the list of those left (`libraries::left_libraries`): one that was withdrawn (SEG-017) is
/// not there, and neither is a forgotten one from before the list existed. The cloud has to offer it
/// still, and not as cut. If it does not hand it over, everything is as it was.
pub async fn redownload<A: CloudApi>(
    database: &Database,
    api: &A,
    local: &str,
) -> Result<LibraryReport> {
    let entry = libraries::left_libraries(&*database.lock()?)?
        .into_iter()
        .find(|entry| entry.id == local)
        .ok_or(Error::Cloud("not_left"))?;
    if entry.cloud_server != api.server() {
        return Ok(LibraryReport::new(
            local,
            Some(&entry.cloud_library),
            Outcome::OtherServer,
        ));
    }
    let listing = api.list_libraries().await?;
    if listing.withdrawn.contains(&entry.cloud_library) {
        // The account lost it while this device was away: it is no longer one it can ask for.
        {
            let conn = database.lock()?;
            if let Some(taken) = libraries::take_left(&conn, local)? {
                libraries::restore_left(&conn, taken, false)?;
            }
        }
        return Ok(LibraryReport::new(
            local,
            Some(&entry.cloud_library),
            Outcome::AccessWithdrawn,
        ));
    }
    let offered = listing.libraries.iter().any(|cloud| {
        cloud.id == entry.cloud_library && cloud.local_library_id.as_deref() == Some(local)
    });
    if !offered {
        return Ok(LibraryReport::new(
            local,
            Some(&entry.cloud_library),
            Outcome::MissingInCloud,
        ));
    }
    let taken = libraries::take_left(&*database.lock()?, local)?.ok_or(Error::Cloud("not_left"))?;
    let result = download(database, api, &entry.cloud_library, local).await;
    let (back, asked_again) = match &result {
        Ok(report) if report.outcome == Outcome::Downloaded => (false, false),
        // Cut while it came down: it stays forgotten, and is no longer one that was merely left.
        Ok(report) if matches!(report.outcome, Outcome::AccessWithdrawn | Outcome::Removed) => {
            (true, false)
        }
        _ => (true, true),
    };
    if back {
        libraries::restore_left(&*database.lock()?, taken, asked_again)?;
    }
    result
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
    let server_vector = match pull_and_merge(
        database,
        api,
        local,
        cloud,
        &VersionVector::new(),
        &mut report,
    )
    .await
    {
        // This device never had the library: there is nothing to remove, and nothing to bring down.
        Err(Error::AccessWithdrawn(_)) => {
            return Ok(LibraryReport::new(
                local,
                Some(cloud),
                Outcome::AccessWithdrawn,
            ));
        }
        other => other?,
    };
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
    match push_missing(database, api, local, cloud, server_vector, &mut report).await {
        // Cut while it came down: the same proof as for a library already here.
        Err(Error::AccessWithdrawn(_)) => {
            let proofs = api_pull_proofs(database, api, local, cloud)
                .await?
                .unwrap_or_default();
            cut_off(database, local, cloud, &proofs)
        }
        other => other.map(|()| report),
    }
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

/// Asks for the proof the way that carries the chain: a pull with this device's own vector. `Some(proofs)` when
/// the cloud answers with `access_withdrawn`; `None` for any other answer, which proves nothing.
async fn api_pull_proofs<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    cloud: &str,
) -> Result<Option<Vec<SyncOperation>>> {
    let (schema_version, vector) = {
        let conn = database.lock()?;
        (
            ops::schema_version(&conn)?,
            sync::version_vectors(&conn, &[library.to_owned()])?
                .remove(library)
                .unwrap_or_default(),
        )
    };
    match api.pull(cloud, schema_version, &vector).await {
        Err(Error::AccessWithdrawn(proofs)) => Ok(Some(proofs)),
        _ => Ok(None),
    }
}
