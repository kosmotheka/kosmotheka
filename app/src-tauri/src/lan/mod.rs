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
use crate::ops::{read_state, write_state};
use crate::sync::{MergeReport, SyncOperation, VersionVector};

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
    pub vector: VersionVector,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullResponse {
    pub operations: Vec<SyncOperation>,
    /// What the server holds, so the client knows what to push.
    pub vector: VersionVector,
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
        "Library".to_owned()
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

pub fn forget_device(conn: &Connection, device_id: &str) -> Result<()> {
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
