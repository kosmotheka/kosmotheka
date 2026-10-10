//! Sync over the local network (D34): one device serves over HTTPS with a pinned
//! self-signed certificate, the other connects. See docs/architecture/sync.md.

pub mod client;
pub mod identity;
pub mod pairing;
pub mod server;
pub mod service;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use kosmotheka_domain::{Action, Change, Entity};
use rand::RngExt;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::libraries;
use crate::ops::{self, read_state, write_state};
use crate::sync::{LibraryVectors, MergeReport, SyncOperation};

const DEVICE_NAME_KEY: &str = "device_name";
const PORT_KEY: &str = "sync_port";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub app: String,
    pub device_id: String,
    pub device_name: String,
    pub schema_version: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    pub token: String,
    pub device_id: String,
    pub device_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairResponse {
    pub device_id: String,
    pub device_name: String,
    /// The client presents this on every later sync.
    pub secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub schema_version: i64,
    /// What the client holds of each library it shares with the server. The keys
    /// are also its offer: the libraries it wants to share.
    pub vectors: LibraryVectors,
    #[serde(default)]
    pub notices: SharingNotices,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullResponse {
    pub operations: Vec<SyncOperation>,
    /// What the server holds of each library it shares with the client, so the client
    /// knows what to push. As in the request, the keys are the server's offer.
    pub vectors: LibraryVectors,
    #[serde(default)]
    pub notices: SharingNotices,
}

/// Changes in sharing that the other device must act on (D71).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharingNotices {
    /// Libraries the receiver no longer has access to: it removes them, but only
    /// once `proofs` show it (SEG-011).
    pub revoked: Vec<String>,
    /// Libraries the sender left: the receiver stops offering them.
    pub left: Vec<String>,
    /// The signed operations that created the revocations behind `revoked`. The
    /// receiver merges them and judges them like any other operation: a notice
    /// nobody allowed to revoke has signed removes nothing.
    #[serde(default)]
    pub proofs: Vec<SyncOperation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushRequest {
    pub operations: Vec<SyncOperation>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub received: MergeReport,
    pub sent: usize,
    /// This device started sharing a library the peer offered.
    pub taken_up: bool,
    /// Libraries removed here because the peer withdrew this device's access (D71).
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownDevice {
    pub device_id: String,
    pub name: String,
    pub paired_at: String,
    pub last_sync_at: Option<String>,
}

pub fn random_secret() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_secret(secret: &str) -> String {
    Sha256::digest(secret.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// What other devices see; defaults to the computer's host name.
pub fn device_name(conn: &Connection) -> Result<String> {
    if let Some(name) = read_state(conn, DEVICE_NAME_KEY)? {
        return Ok(name);
    }
    let host = gethostname::gethostname()
        .to_string_lossy()
        .trim()
        .to_owned();
    Ok(if host.is_empty() || host == "localhost" {
        "Kosmotheka".to_owned()
    } else {
        host
    })
}

/// Renames this device here and in every library it is registered in, so the
/// other devices (and «Personas y permisos») see the new name after a sync.
/// Every device may rename itself, so this never becomes a request (D73).
pub fn set_device_name(conn: &mut Connection, name: &str) -> Result<()> {
    let tx = conn.transaction()?;
    let name = name.trim();
    write_state(&tx, DEVICE_NAME_KEY, (!name.is_empty()).then_some(name))?;
    publish_device_name(&tx)?;
    tx.commit()?;
    Ok(())
}

/// Brings this device's name up to date in the libraries that still show an old
/// one: run on start, for renames made before they travelled to the libraries.
pub fn refresh_device_name(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    publish_device_name(&tx)?;
    tx.commit()?;
    Ok(())
}

fn publish_device_name(tx: &Transaction<'_>) -> Result<()> {
    let shown = device_name(tx)?;
    let me = ops::device_id(tx)?;
    let stale: Vec<String> = tx
        .prepare("SELECT library_id FROM devices WHERE id = ?1 AND name IS NOT ?2")?
        .query_map(params![me, shown], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for library in stale {
        let rename = Change::update(Entity::Device, &me, json!({ "name": shown }));
        ops::record(tx, &library, "device.rename", vec![rename])?;
    }
    Ok(())
}

/// The port this device listens on, kept stable so paired devices find it again.
pub fn stored_port(conn: &Connection) -> Result<Option<u16>> {
    Ok(read_state(conn, PORT_KEY)?.and_then(|port| port.parse().ok()))
}

pub fn store_port(conn: &Connection, port: u16) -> Result<()> {
    write_state(conn, PORT_KEY, Some(&port.to_string()))
}

/// `ip:port` for every non-loopback IPv4 interface.
pub fn local_addresses(port: u16) -> Vec<String> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .filter_map(|interface| match interface.ip() {
            std::net::IpAddr::V4(ip) => Some(format!("{ip}:{port}")),
            std::net::IpAddr::V6(_) => None,
        })
        .collect()
}

// --- Devices this one connects to ---------------------------------------------

#[derive(Debug, Clone)]
pub struct KnownServer {
    pub device_id: String,
    pub name: String,
    pub addresses: Vec<String>,
    pub fingerprint: String,
    pub secret: String,
}

pub fn save_server(conn: &Connection, server: &KnownServer) -> Result<()> {
    conn.execute(
        "INSERT INTO known_servers (device_id, name, addresses, fingerprint, secret)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (device_id) DO UPDATE SET
             name = excluded.name, addresses = excluded.addresses,
             fingerprint = excluded.fingerprint, secret = excluded.secret",
        params![
            server.device_id,
            server.name,
            serde_json::to_string(&server.addresses)?,
            server.fingerprint,
            server.secret
        ],
    )?;
    Ok(())
}

pub fn find_server(conn: &Connection, device_id: &str) -> Result<Option<KnownServer>> {
    let row = conn
        .query_row(
            "SELECT device_id, name, addresses, fingerprint, secret FROM known_servers WHERE device_id = ?1",
            [device_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    Ok(match row {
        Some((device_id, name, addresses, fingerprint, secret)) => Some(KnownServer {
            device_id,
            name,
            addresses: serde_json::from_str(&addresses)?,
            fingerprint,
            secret,
        }),
        None => None,
    })
}

/// Puts the address that just worked first, so the next sync tries it first.
pub fn record_server_sync(conn: &Connection, device_id: &str, working_address: &str) -> Result<()> {
    if let Some(mut server) = find_server(conn, device_id)? {
        server.addresses.retain(|a| a != working_address);
        server.addresses.insert(0, working_address.to_owned());
        save_server(conn, &server)?;
    }
    conn.execute(
        "UPDATE known_servers SET last_sync_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE device_id = ?1",
        [device_id],
    )?;
    Ok(())
}

// --- Devices that connect to this one ----------------------------------------

pub fn is_known_client(conn: &Connection, device_id: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT 1 FROM known_clients WHERE device_id = ?1",
            [device_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub fn save_client(conn: &Connection, device_id: &str, name: &str, secret: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO known_clients (device_id, name, secret_hash) VALUES (?1, ?2, ?3)",
        params![device_id, name, hash_secret(secret)],
    )?;
    Ok(())
}

pub fn client_is_authorized(conn: &Connection, device_id: &str, secret: &str) -> Result<bool> {
    let stored: Option<String> = conn
        .query_row(
            "SELECT secret_hash FROM known_clients WHERE device_id = ?1",
            [device_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(stored.is_some_and(|hash| hash == hash_secret(secret)))
}

pub fn record_client_sync(conn: &Connection, device_id: &str) -> Result<()> {
    conn.execute(
        "UPDATE known_clients SET last_sync_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE device_id = ?1",
        [device_id],
    )?;
    Ok(())
}

pub fn has_clients(conn: &Connection) -> Result<bool> {
    Ok(
        conn.query_row("SELECT count(*) > 0 FROM known_clients", [], |row| {
            row.get(0)
        })?,
    )
}

// --- Both --------------------------------------------------------------------

pub fn list_devices(conn: &Connection, table: &str) -> Result<Vec<KnownDevice>> {
    let mut statement = conn.prepare(&format!(
        "SELECT device_id, name, paired_at, last_sync_at FROM {table} ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = statement.query_map([], |row| {
        Ok(KnownDevice {
            device_id: row.get(0)?,
            name: row.get(1)?,
            paired_at: row.get(2)?,
            last_sync_at: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

// --- Which libraries go to each device (D46) -----------------------------------

/// The libraries that go to `device_id`. A revocation received from another device
/// wins over this device's own list (D71).
pub fn shared_libraries(conn: &Connection, device_id: &str) -> Result<Vec<String>> {
    Ok(conn
        .prepare(
            "SELECT library_id FROM library_peers p
              WHERE device_id = ?1
                AND NOT EXISTS (SELECT 1 FROM revocations r
                                 WHERE r.library_id = p.library_id AND r.device_id = p.device_id)
              ORDER BY library_id",
        )?
        .query_map([device_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn share_library(conn: &Connection, library: &str, device_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO library_peers (library_id, device_id) VALUES (?1, ?2)",
        [library, device_id],
    )?;
    Ok(())
}

/// The paired devices a library goes to, by name.
pub fn devices_sharing(conn: &Connection, library: &str) -> Result<Vec<KnownDevice>> {
    let mut statement = conn.prepare(
        "SELECT d.device_id, d.name, d.paired_at, d.last_sync_at
           FROM (SELECT device_id, name, paired_at, last_sync_at FROM known_servers
                 UNION ALL
                 SELECT device_id, name, paired_at, last_sync_at FROM known_clients) d
           JOIN library_peers p ON p.device_id = d.device_id AND p.library_id = ?1
          WHERE NOT EXISTS (SELECT 1 FROM revocations r
                             WHERE r.library_id = p.library_id AND r.device_id = p.device_id)
          GROUP BY d.device_id
          ORDER BY d.name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([library], |row| {
        Ok(KnownDevice {
            device_id: row.get(0)?,
            name: row.get(1)?,
            paired_at: row.get(2)?,
            last_sync_at: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// A paired device's name, whichever side of the pairing it is on.
pub fn known_device_name(conn: &Connection, device_id: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT name FROM known_servers WHERE device_id = ?1
             UNION ALL SELECT name FROM known_clients WHERE device_id = ?1 LIMIT 1",
            [device_id],
            |row| row.get(0),
        )
        .optional()?)
}

/// Ids of the revocation rows that keep `device_id` out of `library`.
pub fn revocations_of(conn: &Connection, library: &str, device_id: &str) -> Result<Vec<String>> {
    Ok(conn
        .prepare("SELECT id FROM revocations WHERE library_id = ?1 AND device_id = ?2 ORDER BY id")?
        .query_map([library, device_id], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

/// What this device has to tell `peer`: where it lost access, with the operations
/// that say so, and what this device left.
pub fn notices_for(conn: &Connection, peer: &str) -> Result<SharingNotices> {
    let list = |sql: &str| -> Result<Vec<String>> {
        Ok(conn
            .prepare(sql)?
            .query_map([peer], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    };
    let revoked = list(
        "SELECT DISTINCT library_id FROM revocations WHERE device_id = ?1 ORDER BY library_id",
    )?;
    let mut proofs = Vec::new();
    for library in &revoked {
        let vectors = LibraryVectors::from([(library.clone(), Default::default())]);
        proofs.extend(
            crate::sync::operations_since(conn, &vectors)?
                .into_iter()
                .filter(|synced| revokes(&synced.operation.changes, peer)),
        );
    }
    Ok(SharingNotices {
        revoked,
        left: list(
            "SELECT library_id FROM leave_notices WHERE device_id = ?1 ORDER BY library_id",
        )?,
        proofs,
    })
}

/// Removes `library` from this device **only if** the proofs, merged and judged like any other operation,
/// leave a revocation of this device applied in it (SEG-011, D71). Shared by the local network and the cloud
/// (D98): a peer or a server that says so without a signed revocation of someone allowed to make it removes
/// nothing. Only what concerns this library and this device is merged. `by` is told nothing more later.
pub fn remove_if_proved(
    conn: &mut Connection,
    library: &str,
    proofs: &[SyncOperation],
    by: &str,
) -> Result<bool> {
    let me = ops::device_id(conn)?;
    // The revocations of this device in this library, and the earlier operations of whoever signed them that
    // the cloud sends with them so the revocation chains with what this device has (D87). Nothing of another
    // library, and nothing of another author: those are judged like anything else, but never taken as proof.
    let revokers: Vec<&str> = proofs
        .iter()
        .filter(|p| p.operation.library == library && revokes(&p.operation.changes, &me))
        .map(|p| p.operation.hlc.device.as_str())
        .collect();
    let proofs: Vec<SyncOperation> = proofs
        .iter()
        .filter(|p| {
            p.operation.library == library
                && (revokes(&p.operation.changes, &me)
                    || revokers.contains(&p.operation.hlc.device.as_str()))
        })
        .cloned()
        .collect();
    crate::sync::merge(conn, proofs)?;
    if revocations_of(conn, library, &me)?.is_empty() {
        return Ok(false);
    }
    libraries::remove_revoked(conn, library, by)?;
    Ok(true)
}

/// Whether the changes create a revocation for `device`.
fn revokes(changes: &[Change], device: &str) -> bool {
    changes.iter().any(|change| {
        change.entity == Entity::Revocation
            && change.action == Action::Create
            && change.fields.get("device_id").and_then(|v| v.as_str()) == Some(device)
    })
}

/// Agrees with `peer` on which libraries both share, from what it offers and its
/// notices, and returns them. A library travels only when both sides share it.
/// An offer is taken up only for a library this device never had: sharing one of
/// its own is always this device's choice ("nothing leaves without saying"), and a
/// deleted one keeps its log and its delete, which must not reach anyone it was
/// never shared with (SEG-010).
pub fn reconcile(
    conn: &mut Connection,
    peer: &str,
    offered: &[String],
    notices: &SharingNotices,
) -> Result<Vec<String>> {
    let shared = shared_libraries(conn, peer)?;
    for library in &notices.revoked {
        // Only the peers it was shared with can take a library away, and only with
        // a revocation this device accepts once merged: signed by someone allowed
        // to make it (SEG-011). A bare notice removes nothing.
        if !shared.contains(library) {
            continue;
        }
        remove_if_proved(conn, library, &notices.proofs, peer)?;
    }
    for library in &notices.left {
        conn.execute(
            "DELETE FROM library_peers WHERE library_id = ?1 AND device_id = ?2",
            [library, peer],
        )?;
    }
    // A leave notice is done once the peer stops offering that library.
    let pending: Vec<String> = conn
        .prepare("SELECT library_id FROM leave_notices WHERE device_id = ?1")?
        .query_map([peer], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for library in pending.iter().filter(|library| !offered.contains(library)) {
        conn.execute(
            "DELETE FROM leave_notices WHERE library_id = ?1 AND device_id = ?2",
            [library, peer],
        )?;
    }
    let shared = shared_libraries(conn, peer)?;
    for library in offered {
        if shared.contains(library)
            || pending.contains(library)
            || notices.revoked.contains(library)
            || libraries::library_known(conn, library)?
        {
            continue;
        }
        share_library(conn, library, peer)?;
    }
    shared_libraries(conn, peer)
}

/// Registers this device in the libraries shared with `peer` that it already has
/// but has not written to: an administrator can then say whose it is, as soon as
/// the registration reaches them (milestone 5.7). Called after merging what the
/// peer sent, when a shared library has just arrived.
pub fn register_in_shared(conn: &mut Connection, peer: &str) -> Result<()> {
    let tx = conn.transaction()?;
    for library in shared_libraries(&tx, peer)? {
        if libraries::library_exists(&tx, &library)? {
            ops::ensure_registered(&tx, &library)?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// A paired device and the libraries that go to it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSharing {
    #[serde(flatten)]
    pub device: KnownDevice,
    pub libraries: Vec<String>,
}

/// Every paired device, each with what it receives.
pub fn sharing_overview(conn: &Connection) -> Result<Vec<DeviceSharing>> {
    let mut devices = list_devices(conn, "known_servers")?;
    for client in list_devices(conn, "known_clients")? {
        if !devices.iter().any(|d| d.device_id == client.device_id) {
            devices.push(client);
        }
    }
    devices.sort_by_key(|d| d.name.to_lowercase());
    devices
        .into_iter()
        .map(|device| {
            Ok(DeviceSharing {
                libraries: shared_libraries(conn, &device.device_id)?,
                device,
            })
        })
        .collect()
}

/// Keeps only the entries of `vectors` for libraries shared with that device.
pub fn only_shared(vectors: LibraryVectors, shared: &[String]) -> LibraryVectors {
    vectors
        .into_iter()
        .filter(|(library, _)| shared.contains(library))
        .collect()
}

/// Keeps only operations of libraries shared with that device; the rest never merge.
pub fn only_shared_operations(
    operations: Vec<SyncOperation>,
    shared: &[String],
) -> Vec<SyncOperation> {
    operations
        .into_iter()
        .filter(|op| shared.contains(&op.operation.library))
        .collect()
}

pub fn forget_device(conn: &Connection, device_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM library_peers WHERE device_id = ?1",
        [device_id],
    )?;
    conn.execute(
        "DELETE FROM leave_notices WHERE device_id = ?1",
        [device_id],
    )?;
    conn.execute(
        "DELETE FROM known_servers WHERE device_id = ?1",
        [device_id],
    )?;
    conn.execute(
        "DELETE FROM known_clients WHERE device_id = ?1",
        [device_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use kosmotheka_domain::Operation;

    use super::*;

    fn operation_in(library: &str) -> SyncOperation {
        SyncOperation {
            schema_version: 9,
            operation: Operation {
                id: "01997c7e-0000-7000-8000-000000000001".into(),
                library: library.into(),
                hlc: "0001727222222222-00000000-01997c7e-0000-7000-8000-0000000000ff"
                    .parse()
                    .unwrap(),
                intent: "member.add".into(),
                changes: Vec::new(),
                previous: None,
                signature: None,
            },
        }
    }

    #[test]
    fn renaming_the_device_renames_it_in_every_library() {
        let mut conn = crate::db::open_in_memory().unwrap();
        let family = libraries::create_library(&mut conn, "Family").unwrap();
        let rpg = libraries::create_library(&mut conn, "RPG").unwrap();
        let me = ops::device_id(&conn).unwrap();

        set_device_name(&mut conn, "  Kitchen tablet ").unwrap();

        assert_eq!(device_name(&conn).unwrap(), "Kitchen tablet");
        for library in [&family, &rpg] {
            let name: String = conn
                .query_row(
                    "SELECT name FROM devices WHERE library_id = ?1 AND id = ?2",
                    [library, &me],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(name, "Kitchen tablet");
        }

        // The same name again changes nothing, so it records nothing.
        let count = |conn: &Connection| -> i64 {
            conn.query_row(
                "SELECT COUNT(*) FROM operations WHERE intent = 'device.rename'",
                [],
                |row| row.get(0),
            )
            .unwrap()
        };
        let before = count(&conn);
        set_device_name(&mut conn, "Kitchen tablet").unwrap();
        assert_eq!(count(&conn), before);
    }

    #[test]
    fn only_shared_libraries_travel() {
        let shared = vec!["family".to_owned()];
        let vectors = LibraryVectors::from([
            ("family".to_owned(), Default::default()),
            ("rpg".to_owned(), Default::default()),
        ]);

        let kept =
            only_shared_operations(vec![operation_in("family"), operation_in("rpg")], &shared);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].operation.library, "family");
        assert_eq!(
            only_shared(vectors, &shared)
                .into_keys()
                .collect::<Vec<_>>(),
            shared
        );
    }
}
