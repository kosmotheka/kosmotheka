//! Glue between the app and LAN sync: owns the identity, the pairing window and the
//! running server, which starts only when needed (paired clients or pairing open).

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::net::TcpListener;

use super::identity::Identity;
use super::pairing::{PAIRING_WINDOW, PairingCode, PairingWindow};
use super::server::{self, RunningServer, ServerState};
use super::{
    KnownDevice, device_name, has_clients, list_devices, local_addresses, random_secret,
    store_port, stored_port,
};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::ops;

#[derive(Clone)]
pub struct LanService {
    database: Database,
    identity: Arc<Identity>,
    pairing: Arc<Mutex<Option<PairingWindow>>>,
    server: Arc<tokio::sync::Mutex<Option<RunningServer>>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanStatus {
    pub device_id: String,
    pub device_name: String,
    /// Port this device serves on, if it is serving.
    pub serving_port: Option<u16>,
    pub addresses: Vec<String>,
    pub pairing_open: bool,
    /// Devices this one syncs with (it connects to them).
    pub servers: Vec<KnownDevice>,
    /// Devices that sync with this one (they connect here).
    pub clients: Vec<KnownDevice>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub code: String,
    pub expires_in_seconds: u64,
}

impl LanService {
    pub fn new(database: Database, identity: Identity) -> Self {
        Self {
            database,
            identity: Arc::new(identity),
            pairing: Arc::new(Mutex::new(None)),
            server: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// At startup: serve only if some device already syncs with this one.
    pub async fn start_if_paired(&self) -> Result<()> {
        let paired = has_clients(&*self.database.lock()?)?;
        if paired {
            self.ensure_serving().await?;
        }
        Ok(())
    }

    /// Starts the server if needed and returns its port. The port is kept between runs
    /// so paired devices find this one where they left it.
    async fn ensure_serving(&self) -> Result<u16> {
        let mut server = self.server.lock().await;
        if let Some(running) = server.as_ref() {
            return Ok(running.address.port());
        }
        let wanted = stored_port(&*self.database.lock()?)?.unwrap_or(0);
        let listener = match TcpListener::bind(("0.0.0.0", wanted)).await {
            Ok(listener) => listener,
            // The old port is taken by something else: take any free one.
            Err(_) => TcpListener::bind(("0.0.0.0", 0)).await?,
        };
        let state = ServerState {
            database: self.database.clone(),
            pairing: self.pairing.clone(),
        };
        let running = server::start(listener, self.identity.server_config()?, state)?;
        let port = running.address.port();
        store_port(&*self.database.lock()?, port)?;
        *server = Some(running);
        Ok(port)
    }

    pub async fn open_pairing(&self, libraries: Vec<String>) -> Result<PairingInfo> {
        crate::people::ensure_may_share(&*self.database.lock()?, &libraries)?;
        let port = self.ensure_serving().await?;
        let token = random_secret();
        *self.pairing.lock().map_err(|_| Error::LockPoisoned)? =
            Some(PairingWindow::open(token.clone(), libraries));
        let conn = self.database.lock()?;
        let addresses = local_addresses(port);
        if addresses.is_empty() {
            return Err(Error::Lan("no_network"));
        }
        let code = PairingCode {
            device_id: ops::device_id(&conn)?,
            name: device_name(&conn)?,
            addresses,
            fingerprint: self.identity.fingerprint().to_owned(),
            token,
        };
        Ok(PairingInfo {
            code: code.encode(),
            expires_in_seconds: PAIRING_WINDOW.as_secs(),
        })
    }

    pub fn close_pairing(&self) -> Result<()> {
        *self.pairing.lock().map_err(|_| Error::LockPoisoned)? = None;
        Ok(())
    }

    pub async fn status(&self) -> Result<LanStatus> {
        let serving_port = self.server.lock().await.as_ref().map(|s| s.address.port());
        let pairing_open = self
            .pairing
            .lock()
            .map_err(|_| Error::LockPoisoned)?
            .as_ref()
            .is_some_and(|w| w.expires > std::time::Instant::now());
        let conn = self.database.lock()?;
        Ok(LanStatus {
            device_id: ops::device_id(&conn)?,
            device_name: device_name(&conn)?,
            addresses: serving_port.map(local_addresses).unwrap_or_default(),
            serving_port,
            pairing_open,
            servers: list_devices(&conn, "known_servers")?,
            clients: list_devices(&conn, "known_clients")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::client;
    use crate::ops::DEFAULT_LIBRARY_ID;

    #[tokio::test(flavor = "multi_thread")]
    async fn pairs_and_syncs_through_the_local_network_address() {
        let desk = LanService::new(Database::in_memory(), Identity::generate().unwrap());
        let phone = Database::in_memory();
        let before = desk.status().await.unwrap();
        assert_eq!(before.serving_port, None, "no server until it is needed");

        let pairing = desk
            .open_pairing(vec![DEFAULT_LIBRARY_ID.to_owned()])
            .await
            .unwrap();
        let code = PairingCode::decode(&pairing.code).unwrap();
        assert!(
            code.addresses.iter().all(|a| !a.starts_with("127.")),
            "{:?}",
            code.addresses
        );
        let server = client::pair(&phone, &pairing.code, &[DEFAULT_LIBRARY_ID.to_owned()])
            .await
            .unwrap();
        client::sync(&phone, &server.device_id).await.unwrap();

        let after = desk.status().await.unwrap();
        assert!(after.serving_port.is_some());
        assert_eq!(after.clients.len(), 1);
        assert!(after.clients[0].last_sync_at.is_some());
    }
}
