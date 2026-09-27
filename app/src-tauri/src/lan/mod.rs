//! Sync over the local network (D34): one device serves over HTTPS with a pinned
//! self-signed certificate, the other connects. See docs/architecture/sync.md.

pub mod client;
pub mod identity;
pub mod pairing;
pub mod server;
pub mod service;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngExt;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
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
    /// Libraries the receiver no longer has access to: it removes them.
    pub revoked: Vec<String>,
    /// Libraries the sender left: the receiver stops offering them.
    pub left: Vec<String>,
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

pub fn set_device_name(conn: &Connection, name: &str) -> Result<()> {
    let name = name.trim();
    write_state(conn, DEVICE_NAME_KEY, (!name.is_empty()).then_some(name))
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

pub fn save_client(conn: &Connection, device_id: &str, name: &str, secret: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO known_clients (device_id, name, secret_hash) VALUES (?1, ?2, ?3)
         ON CONFLICT (device_id) DO UPDATE SET name = excluded.name, secret_hash = excluded.secret_hash",
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

/// What this device has to tell `peer`: where it lost access, and what this device left.
pub fn notices_for(conn: &Connection, peer: &str) -> Result<SharingNotices> {
    let list = |sql: &str| -> Result<Vec<String>> {
        Ok(conn
            .prepare(sql)?
            .query_map([peer], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?)
    };
    Ok(SharingNotices {
        revoked: list(
            "SELECT DISTINCT library_id FROM revocations WHERE device_id = ?1 ORDER BY library_id",
        )?,
        left: list(
            "SELECT library_id FROM leave_notices WHERE device_id = ?1 ORDER BY library_id",
        )?,
    })
}

/// Agrees with `peer` on which libraries both share, from what it offers and its
/// notices, and returns them. A library travels only when both sides share it.
/// An offer is taken up only for a library this device does not have: sharing one
/// of its own is always this device's choice ("nothing leaves without saying").
pub fn reconcile(
    conn: &mut Connection,
    peer: &str,
    offered: &[String],
    notices: &SharingNotices,
) -> Result<Vec<String>> {
    let shared = shared_libraries(conn, peer)?;
    for library in &notices.revoked {
        // Only the peers it was shared with can take a library away.
        if shared.contains(library) {
            libraries::remove_revoked(conn, library, peer)?;
        }
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
            || libraries::library_exists(conn, library)?
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
                hlc: "0001727222222222-00000000-dev".parse().unwrap(),
                intent: "member.add".into(),
                changes: Vec::new(),
                signature: None,
            },
        }
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
