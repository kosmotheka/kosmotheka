//! Connecting to a paired device: pinned TLS, pairing and a two-way sync.

use std::sync::Arc;
use std::time::Duration;

use reqwest::StatusCode;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, DigitallySignedStruct, SignatureScheme};
use serde::Serialize;
use serde::de::DeserializeOwned;

use std::collections::BTreeSet;
use std::net::Ipv4Addr;

use super::identity::fingerprint;
use super::pairing::PairingCode;
use super::{
    Hello, KnownServer, PairRequest, PairResponse, PullRequest, PullResponse, PushRequest,
    SyncReport, device_name, find_server, notices_for, only_shared, only_shared_operations,
    reconcile, record_server_sync, register_in_shared, save_server, share_library,
    shared_libraries,
};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::libraries;
use crate::ops;
use crate::sync;

/// Accepts exactly one certificate: the one whose fingerprint came in the pairing code.
/// No certificate authority is involved, and no other server can impersonate the peer.
#[derive(Debug)]
struct PinnedCertificate {
    fingerprint: String,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedCertificate {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        if fingerprint(end_entity.as_ref()) == self.fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    // The handshake signatures are still checked, so the peer must hold the private key.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn pinned_client(fingerprint: &str) -> Result<reqwest::Client> {
    pinned_client_with(fingerprint, Duration::from_secs(4), Duration::from_secs(60))
}

fn pinned_client_with(
    fingerprint: &str,
    connect: Duration,
    total: Duration,
) -> Result<reqwest::Client> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let tls = rustls::ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|_| Error::Tls("protocol versions"))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(PinnedCertificate {
            fingerprint: fingerprint.to_owned(),
            provider,
        }))
        .with_no_client_auth();
    Ok(reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .no_proxy()
        .connect_timeout(connect)
        .timeout(total)
        .build()?)
}

/// Sends `body` to each address until one answers; returns the answer and the address.
async fn post<B: Serialize, R: DeserializeOwned>(
    client: &reqwest::Client,
    addresses: &[String],
    path: &str,
    auth: Option<(&str, &str)>,
    body: &B,
) -> Result<(R, String)> {
    let mut last_error = Error::Lan("unreachable");
    for address in addresses {
        let mut request = client.post(format!("https://{address}{path}")).json(body);
        if let Some((device_id, secret)) = auth {
            request = request.header("x-device-id", device_id).bearer_auth(secret);
        }
        match request.send().await {
            Ok(response) => {
                return match response.status() {
                    status if status.is_success() => Ok((response.json().await?, address.clone())),
                    StatusCode::UNAUTHORIZED => Err(Error::Lan("rejected")),
                    StatusCode::FORBIDDEN => Err(Error::Lan("already_paired")),
                    StatusCode::CONFLICT => Err(Error::Lan("other_version")),
                    _ => Err(Error::Lan("peer_error")),
                };
            }
            // A certificate that does not match the pin is not "unreachable": it is an impostor.
            Err(error) if is_certificate_error(&error) => {
                last_error = Error::Lan("wrong_certificate")
            }
            Err(_) => {}
        }
    }
    Err(last_error)
}

fn is_certificate_error(error: &reqwest::Error) -> bool {
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        if cause.to_string().to_lowercase().contains("certificate") {
            return true;
        }
        source = cause.source();
    }
    false
}

/// Pairs with the device that showed `code_text`, remembers it and shares `libraries`
/// with it.
pub async fn pair(
    database: &Database,
    code_text: &str,
    libraries: &[String],
) -> Result<KnownServer> {
    let code = PairingCode::decode(code_text).ok_or(Error::Lan("bad_code"))?;
    let request = {
        let conn = database.lock()?;
        if code.device_id == ops::device_id(&conn)? {
            return Err(Error::Lan("same_device"));
        }
        crate::people::ensure_may_share(&conn, libraries)?;
        PairRequest {
            token: code.token.clone(),
            device_id: ops::device_id(&conn)?,
            device_name: device_name(&conn)?,
        }
    };
    let client = pinned_client(&code.fingerprint)?;
    let (response, address): (PairResponse, String) =
        post(&client, &code.addresses, "/v1/pair", None, &request).await?;

    let mut addresses = code.addresses.clone();
    addresses.retain(|a| *a != address);
    addresses.insert(0, address);
    let server = KnownServer {
        device_id: response.device_id,
        name: response.device_name,
        addresses,
        fingerprint: code.fingerprint,
        secret: response.secret,
    };
    {
        let conn = database.lock()?;
        save_server(&conn, &server)?;
        for library in libraries {
            share_library(&conn, library, &server.device_id)?;
        }
    }
    Ok(server)
}

/// Pulls what the peer has, then pushes what it lacks. A library the peer has just
/// started sharing arrives in a second round, once this device has taken it up.
pub async fn sync(database: &Database, device_id: &str) -> Result<SyncReport> {
    let mut report = sync_once(database, device_id).await?;
    if report.taken_up {
        let again = sync_once(database, device_id).await?;
        report.received.add(&again.received);
        report.sent += again.sent;
        report.removed.extend(again.removed);
    }
    Ok(report)
}

async fn sync_once(database: &Database, device_id: &str) -> Result<SyncReport> {
    let (server, local_device, pull) = {
        let conn = database.lock()?;
        let server = find_server(&conn, device_id)?.ok_or(Error::Lan("unknown_device"))?;
        let shared = shared_libraries(&conn, device_id)?;
        let pull = PullRequest {
            schema_version: ops::schema_version(&conn)?,
            vectors: sync::version_vectors(&conn, &shared)?,
            notices: notices_for(&conn, device_id)?,
        };
        (server, ops::device_id(&conn)?, pull)
    };
    let client = pinned_client(&server.fingerprint)?;
    let auth = Some((local_device.as_str(), server.secret.as_str()));

    let (pulled, address): (PullResponse, String) =
        match post(&client, &server.addresses, "/v1/sync/pull", auth, &pull).await {
            // The peer may have a new address: look for it on this network (D37).
            Err(Error::Lan("unreachable")) => {
                let found = rediscover(&server).await.ok_or(Error::Lan("unreachable"))?;
                post(&client, &[found], "/v1/sync/pull", auth, &pull).await?
            }
            other => other?,
        };
    let (received, to_send, taken_up, removed) = {
        let mut conn = database.lock()?;
        let before = shared_libraries(&conn, device_id)?;
        let offered: Vec<String> = pulled.vectors.keys().cloned().collect();
        let shared = reconcile(&mut conn, device_id, &offered, &pulled.notices)?;
        let taken_up = shared.iter().any(|library| !before.contains(library));
        let mut removed = Vec::new();
        for library in &pulled.notices.revoked {
            if before.contains(library) && !libraries::library_exists(&conn, library)? {
                removed.push(library.clone());
            }
        }
        let received = sync::merge(
            &mut conn,
            only_shared_operations(pulled.operations, &shared),
        )?;
        register_in_shared(&mut conn, device_id)?;
        // With what the peer had already merged, and before working out what to send it (D94).
        crate::permissions::seal_if_missing(&mut conn, &shared)?;
        crate::permissions::vouch_if_missing(&mut conn, &shared)?;
        let theirs = only_shared(pulled.vectors, &shared);
        (
            received,
            sync::operations_since(&conn, &theirs)?,
            taken_up,
            removed,
        )
    };

    let sent = to_send.len();
    if sent > 0 {
        let push = PushRequest {
            operations: to_send,
        };
        let _: (sync::MergeReport, String) = post(
            &client,
            std::slice::from_ref(&address),
            "/v1/sync/push",
            auth,
            &push,
        )
        .await?;
    }
    record_server_sync(&*database.lock()?, device_id, &address)?;
    Ok(SyncReport {
        received,
        sent,
        taken_up,
        removed,
    })
}

/// Looks for a paired device that moved to another address on this device's own
/// subnets, at the ports it used before (D37). Only a peer presenting the pinned
/// certificate and the same device id counts as found.
pub async fn rediscover(server: &KnownServer) -> Option<String> {
    let ports: BTreeSet<u16> = server
        .addresses
        .iter()
        .filter_map(|address| address.rsplit_once(':')?.1.parse().ok())
        .collect();
    let candidates: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .filter_map(|interface| match interface.addr {
            if_addrs::IfAddr::V4(v4) => Some(hosts_in(v4.ip, v4.prefixlen)),
            if_addrs::IfAddr::V6(_) => None,
        })
        .flatten()
        .flat_map(|ip| ports.iter().map(move |port| format!("{ip}:{port}")))
        .filter(|candidate| !server.addresses.contains(candidate))
        .collect();
    find_among(&server.fingerprint, &server.device_id, candidates).await
}

/// Other hosts of the subnet of `ip`. Wide networks are narrowed to the /22 around
/// `ip` (at most 1022 hosts), which covers any home network.
fn hosts_in(ip: Ipv4Addr, prefix: u8) -> Vec<Ipv4Addr> {
    let prefix = prefix.clamp(22, 30);
    let mask = u32::MAX << (32 - prefix);
    let network = u32::from(ip) & mask;
    let broadcast = network | !mask;
    ((network + 1)..broadcast)
        .map(Ipv4Addr::from)
        .filter(|host| *host != ip)
        .collect()
}

const PROBES_AT_ONCE: usize = 64;

async fn find_among(fingerprint: &str, device_id: &str, candidates: Vec<String>) -> Option<String> {
    let client = pinned_client_with(
        fingerprint,
        Duration::from_millis(700),
        Duration::from_secs(2),
    )
    .ok()?;
    for batch in candidates.chunks(PROBES_AT_ONCE) {
        let mut probes = tokio::task::JoinSet::new();
        for candidate in batch {
            let (client, candidate) = (client.clone(), candidate.clone());
            probes.spawn(async move {
                let hello: Hello = client
                    .get(format!("https://{candidate}/v1/hello"))
                    .send()
                    .await
                    .ok()?
                    .json()
                    .await
                    .ok()?;
                Some((hello.device_id, candidate))
            });
        }
        while let Some(probe) = probes.join_next().await {
            if let Ok(Some((found_id, candidate))) = probe
                && found_id == device_id
            {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use kosmotheka_domain::{CopyInput, EditionInput, Format, ValidBook};
    use tokio::net::TcpListener;

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::lan::identity::Identity;
    use crate::lan::pairing::PairingWindow;
    use crate::lan::server::{self, RunningServer, ServerState};
    use crate::lan::{SharingNotices, list_devices, random_secret};
    use crate::libraries;

    /// The library a device was born with: the first one its database ever held.
    fn home(database: &Database) -> String {
        database
            .lock()
            .unwrap()
            .query_row("SELECT id FROM libraries ORDER BY rowid LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap()
    }

    /// The same library id on two devices that never synced: what a copied backup
    /// or a hostile peer would bring (D84, D85). It is not how libraries normally
    /// meet, since each is created with a random id on one device only.
    fn create_with_id(database: &Database, id: &str) {
        let mut conn = database.lock().unwrap();
        let tx = conn.transaction().unwrap();
        libraries::create_with_id(&tx, id, "").unwrap();
        tx.commit().unwrap();
    }

    fn add(database: &Database, title: &str) {
        let book = ValidBook {
            edition: EditionInput {
                title: title.into(),
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: None,
                format: Format::Paperback,
                quantity: 1,
                location: None,
                acquired_on: None,
                platform: None,
                seasons: None,
            }
            .validate()
            .unwrap(),
        };
        let library = home(database);
        catalog::create_book(&mut database.lock().unwrap(), &library, &book, None).unwrap();
    }

    fn titles(database: &Database) -> Vec<String> {
        titles_in(database, &home(database))
    }

    struct Serving {
        database: Database,
        identity: Identity,
        state: ServerState,
        server: RunningServer,
    }

    async fn serve() -> Serving {
        let database = Database::in_memory();
        let identity = Identity::generate().unwrap();
        let state = ServerState {
            database: database.clone(),
            pairing: Arc::new(Mutex::new(None)),
            arrival: Arc::new(std::sync::OnceLock::new()),
        };
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server =
            server::start(listener, identity.server_config().unwrap(), state.clone()).unwrap();
        Serving {
            database,
            identity,
            state,
            server,
        }
    }

    impl Serving {
        fn open_pairing(&self) -> String {
            let token = random_secret();
            *self.state.pairing.lock().unwrap() = Some(PairingWindow::open(
                token.clone(),
                vec![home(&self.database)],
            ));
            let conn = self.database.lock().unwrap();
            PairingCode {
                device_id: ops::device_id(&conn).unwrap(),
                name: "Desk".into(),
                addresses: vec![self.server.address.to_string()],
                fingerprint: self.identity.fingerprint().to_owned(),
                token,
            }
            .encode()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn pair_then_sync_both_ways_over_tls() {
        let desk = serve().await;
        add(&desk.database, "On the desk");
        let phone = Database::in_memory();
        let library = home(&desk.database);

        // The desk's library reaches the phone, which then adds to it.
        let server = pair(&phone, &desk.open_pairing(), &[]).await.unwrap();
        let first = sync(&phone, &server.device_id).await.unwrap();
        add_to(&phone, &library, "On the phone");
        let report = sync(&phone, &server.device_id).await.unwrap();

        // The phone gets the desk's library, its book and its key registration (3),
        // and answers with its own key registration (1); then the desk gets the
        // phone's book (1).
        assert_eq!((first.received.received, first.sent), (3, 1));
        assert_eq!((report.received.received, report.sent), (0, 1));
        assert_eq!(
            titles_in(&phone, &library),
            vec!["On the desk", "On the phone"]
        );
        assert_eq!(titles(&desk.database), vec!["On the desk", "On the phone"]);
        let clients = list_devices(&desk.database.lock().unwrap(), "known_clients").unwrap();
        assert_eq!(clients.len(), 1);
        assert!(clients[0].last_sync_at.is_some());

        // A third sync has nothing to do.
        let again = sync(&phone, &server.device_id).await.unwrap();
        assert_eq!((again.received.received, again.sent), (0, 0));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_server_hears_of_a_push_that_brought_something_new() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let desk = serve().await;
        let heard = Arc::new(AtomicUsize::new(0));
        let counter = heard.clone();
        let hook: super::super::server::ArrivalHook = Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        assert!(desk.state.arrival.set(hook).is_ok());
        let phone = Database::in_memory();
        let library = home(&desk.database);
        let server = pair(&phone, &desk.open_pairing(), &[]).await.unwrap();

        // The phone's first sync brings its key registration.
        sync(&phone, &server.device_id).await.unwrap();
        assert_eq!(heard.load(Ordering::SeqCst), 1);
        // Nothing new: nothing to tell.
        sync(&phone, &server.device_id).await.unwrap();
        assert_eq!(heard.load(Ordering::SeqCst), 1);
        add_to(&phone, &library, "On the phone");
        sync(&phone, &server.device_id).await.unwrap();
        assert_eq!(heard.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_paired_device_cannot_pair_again_until_forgotten() {
        let desk = serve().await;
        let phone = Database::in_memory();
        let server = pair(&phone, &desk.open_pairing(), &[]).await.unwrap();

        // Someone who saw the next code and claims the phone's id is turned away,
        // and the phone keeps syncing with its own secret.
        let again = pair(&phone, &desk.open_pairing(), &[]).await;
        assert!(matches!(again, Err(Error::Lan("already_paired"))));
        sync(&phone, &server.device_id).await.unwrap();

        // Once the desk forgets it, it pairs as new.
        let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
        crate::lan::forget_device(&desk.database.lock().unwrap(), &phone_id).unwrap();
        pair(&phone, &desk.open_pairing(), &[]).await.unwrap();
    }

    fn add_to(database: &Database, library: &str, title: &str) {
        let book = ValidBook {
            edition: EditionInput {
                title: title.into(),
                ..EditionInput::default()
            }
            .validate()
            .unwrap(),
            copy: CopyInput {
                owner: None,
                format: Format::Paperback,
                quantity: 1,
                location: None,
                acquired_on: None,
                platform: None,
                seasons: None,
            }
            .validate()
            .unwrap(),
        };
        catalog::create_book(&mut database.lock().unwrap(), library, &book, None).unwrap();
    }

    fn titles_in(database: &Database, library: &str) -> Vec<String> {
        catalog::list_books(&database.lock().unwrap(), library, &BookFilter::default())
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect()
    }

    fn has_library(database: &Database, library: &str) -> bool {
        libraries::library_exists(&database.lock().unwrap(), library).unwrap()
    }

    fn id_of(database: &Database) -> String {
        ops::device_id(&database.lock().unwrap()).unwrap()
    }

    /// A desk with its default library and a "Club" one, each with a book.
    async fn desk_with_club() -> (Serving, String) {
        let desk = serve().await;
        add(&desk.database, "Desk's own");
        let club = libraries::create_library(&mut desk.database.lock().unwrap(), "Club").unwrap();
        add_to(&desk.database, &club, "Club book");
        (desk, club)
    }

    fn open_pairing_for(desk: &Serving, libraries: Vec<String>) -> String {
        let code = desk.open_pairing();
        desk.state
            .pairing
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .libraries = libraries;
        code
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_the_chosen_libraries_travel() {
        let (desk, club) = desk_with_club().await;
        let phone = Database::in_memory();
        add(&phone, "Phone's own");

        let code = open_pairing_for(&desk, vec![club.clone()]);
        let server = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        assert_eq!(titles_in(&phone, &club), vec!["Club book"]);
        assert_eq!(titles(&phone), vec!["Phone's own"]);
        assert_eq!(titles(&desk.database), vec!["Desk's own"]);

        // Changes to the shared library go both ways from then on.
        add_to(&phone, &club, "Added on the phone");
        sync(&phone, &server.device_id).await.unwrap();
        assert_eq!(
            titles_in(&desk.database, &club),
            vec!["Added on the phone", "Club book"]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_library_both_have_travels_only_if_both_share_it() {
        // The same id on both devices, made on purpose (see `create_with_id`).
        let repeated = ops::new_id();
        let desk = serve().await;
        create_with_id(&desk.database, &repeated);
        add_to(&desk.database, &repeated, "On the desk");
        let phone = Database::in_memory();
        create_with_id(&phone, &repeated);
        add_to(&phone, &repeated, "On the phone");

        // The desk offers it; the phone has one with the same id but did not choose
        // to share it, so nothing moves either way.
        let code = open_pairing_for(&desk, vec![repeated.clone()]);
        let server = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        assert_eq!(titles_in(&phone, &repeated), vec!["On the phone"]);
        assert_eq!(titles_in(&desk.database, &repeated), vec!["On the desk"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn revoking_access_removes_the_library_there_until_given_back() {
        let (desk, club) = desk_with_club().await;
        let phone = Database::in_memory();
        let code = open_pairing_for(&desk, vec![club.clone()]);
        let server = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();
        libraries::set_active_library(&phone.lock().unwrap(), &club).unwrap();
        let phone_id = id_of(&phone);

        libraries::revoke_access(&mut desk.database.lock().unwrap(), &club, &phone_id).unwrap();
        let report = sync(&phone, &server.device_id).await.unwrap();

        assert_eq!(report.removed, vec![club.clone()]);
        assert!(!has_library(&phone, &club));
        assert_ne!(ops::active_library(&phone.lock().unwrap()).unwrap(), club);
        assert!(has_library(&desk.database, &club), "the desk keeps it");
        // Nothing comes back on later syncs.
        sync(&phone, &server.device_id).await.unwrap();
        assert!(!has_library(&phone, &club));

        libraries::grant_access(&mut desk.database.lock().unwrap(), &club, &phone_id).unwrap();
        sync(&phone, &server.device_id).await.unwrap();
        assert_eq!(titles_in(&phone, &club), vec!["Club book"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_revoked_device_is_refused_its_old_offer() {
        let (desk, club) = desk_with_club().await;
        let phone = Database::in_memory();
        let code = open_pairing_for(&desk, vec![club.clone()]);
        let server = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        libraries::revoke_access(&mut desk.database.lock().unwrap(), &club, &id_of(&phone))
            .unwrap();
        // The phone changes the library before it hears of the revocation.
        add_to(&phone, &club, "Too late");
        sync(&phone, &server.device_id).await.unwrap();

        assert_eq!(titles_in(&desk.database, &club), vec!["Club book"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_left_library_is_not_offered_again() {
        let (desk, club) = desk_with_club().await;
        let phone = Database::in_memory();
        let code = open_pairing_for(&desk, vec![club.clone()]);
        let server = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        libraries::leave_library(&mut phone.lock().unwrap(), &club).unwrap();
        sync(&phone, &server.device_id).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        assert!(!has_library(&phone, &club));
        let desk_conn = desk.database.lock().unwrap();
        assert!(
            shared_libraries(&desk_conn, &id_of(&phone))
                .unwrap()
                .is_empty()
        );
        let notices: i64 = phone
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM leave_notices", [], |row| row.get(0))
            .unwrap();
        assert_eq!(notices, 0, "the notice goes once the desk stops offering");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_revocation_reaches_the_device_through_another_one() {
        // The desk and the tablet both serve; the phone syncs with both, and the
        // desk also syncs with the tablet.
        let (desk, club) = desk_with_club().await;
        let tablet = serve().await;
        let phone = Database::in_memory();
        let to_tablet = pair(
            &desk.database,
            &tablet.open_pairing(),
            std::slice::from_ref(&club),
        )
        .await
        .unwrap();
        sync(&desk.database, &to_tablet.device_id).await.unwrap();
        let code = open_pairing_for(&desk, vec![club.clone()]);
        let phone_to_desk = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &phone_to_desk.device_id).await.unwrap();
        let code = open_pairing_for(&tablet, vec![club.clone()]);
        let phone_to_tablet = pair(&phone, &code, std::slice::from_ref(&club))
            .await
            .unwrap();
        sync(&phone, &phone_to_tablet.device_id).await.unwrap();
        assert_eq!(titles_in(&tablet.database, &club), vec!["Club book"]);

        libraries::revoke_access(&mut desk.database.lock().unwrap(), &club, &id_of(&phone))
            .unwrap();
        sync(&desk.database, &to_tablet.device_id).await.unwrap();
        // The tablet heard of it from the desk: it stops sharing and tells the phone.
        add_to(&tablet.database, &club, "After the revocation");
        let report = sync(&phone, &phone_to_tablet.device_id).await.unwrap();

        assert_eq!(report.removed, vec![club.clone()]);
        assert!(!has_library(&phone, &club));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_deleted_library_is_not_taken_up_when_offered() {
        // SEG-010: the desk deleted a library long ago; its log keeps the delete. The
        // phone shares a library with the same id (made on purpose, standing for a
        // copied or hostile id), and the desk shares nothing. Nothing may move, and
        // above all the desk's delete must never reach the phone.
        let repeated = ops::new_id();
        let (desk, club) = desk_with_club().await;
        create_with_id(&desk.database, &repeated);
        libraries::set_active_library(&desk.database.lock().unwrap(), &club).unwrap();
        libraries::delete_library(&mut desk.database.lock().unwrap(), &repeated).unwrap();
        let phone = Database::in_memory();
        create_with_id(&phone, &repeated);
        add_to(&phone, &repeated, "Phone's own");

        let code = open_pairing_for(&desk, vec![]);
        let server = pair(&phone, &code, std::slice::from_ref(&repeated))
            .await
            .unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        assert_eq!(titles_in(&phone, &repeated), vec!["Phone's own"]);
        assert!(!has_library(&desk.database, &repeated));
        assert!(
            shared_libraries(&desk.database.lock().unwrap(), &id_of(&phone))
                .unwrap()
                .is_empty(),
            "the desk did not choose to share it"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_forged_revocation_notice_removes_nothing() {
        // SEG-011: a paired phone that only reads sends, with a modified app, a
        // notice saying the desk lost access to the club. No signed revocation
        // backs it, so the desk keeps the library and its log.
        let (desk, club) = desk_with_club().await;
        let phone = Database::in_memory();
        let code = open_pairing_for(&desk, vec![club.clone()]);
        let server = pair(&phone, &code, &[]).await.unwrap();
        sync(&phone, &server.device_id).await.unwrap();
        crate::people::start_roles(&mut desk.database.lock().unwrap(), &club, "Ana", &[]).unwrap();
        sync(&phone, &server.device_id).await.unwrap();

        // First a bare notice, then one with a revocation the phone signed itself,
        // which a reader may not make.
        let desk_id = id_of(&desk.database);
        let phone_id = id_of(&phone);
        let phone_previous = ops::latest_of(&phone.lock().unwrap(), &club, &phone_id).unwrap();
        let mut own_proof = kosmotheka_domain::Operation {
            id: ops::new_id(),
            library: club.clone(),
            hlc: ops::last_hlc(&phone.lock().unwrap()).unwrap().unwrap(),
            intent: "library.revoke".into(),
            changes: vec![kosmotheka_domain::Change::create(
                kosmotheka_domain::Entity::Revocation,
                ops::new_id(),
                serde_json::json!({ "device_id": desk_id, "device_name": "Desk" }),
            )],
            previous: phone_previous,
            signature: None,
        };
        own_proof.hlc.millis += 1;
        own_proof.signature = Some(
            ops::signing_key(&phone.lock().unwrap())
                .unwrap()
                .sign(&own_proof),
        );
        let client = pinned_client(&server.fingerprint).unwrap();
        for proofs in [vec![], vec![own_proof]] {
            let forged = {
                let conn = phone.lock().unwrap();
                PullRequest {
                    schema_version: ops::schema_version(&conn).unwrap(),
                    vectors: sync::version_vectors(&conn, std::slice::from_ref(&club)).unwrap(),
                    notices: SharingNotices {
                        revoked: vec![club.clone()],
                        left: vec![],
                        proofs: proofs
                            .into_iter()
                            .map(|operation| sync::SyncOperation {
                                schema_version: ops::schema_version(&conn).unwrap(),
                                operation,
                            })
                            .collect(),
                    },
                }
            };
            let _: (PullResponse, String) = post(
                &client,
                &server.addresses,
                "/v1/sync/pull",
                Some((phone_id.as_str(), server.secret.as_str())),
                &forged,
            )
            .await
            .unwrap();
        }

        assert!(has_library(&desk.database, &club));
        assert_eq!(titles_in(&desk.database, &club), vec!["Club book"]);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_pairing_code_works_only_once() {
        let desk = serve().await;
        let code = desk.open_pairing();

        pair(&Database::in_memory(), &code, &[]).await.unwrap();
        let second = pair(&Database::in_memory(), &code, &[]).await;

        assert!(matches!(second, Err(Error::Lan("rejected"))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_different_certificate_is_refused() {
        let desk = serve().await;
        let mut code = PairingCode::decode(&desk.open_pairing()).unwrap();
        code.fingerprint = Identity::generate().unwrap().fingerprint().to_owned();

        let result = pair(&Database::in_memory(), &code.encode(), &[]).await;

        assert!(
            matches!(result, Err(Error::Lan("wrong_certificate"))),
            "{result:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_forgotten_device_can_no_longer_sync() {
        let desk = serve().await;
        let phone = Database::in_memory();
        let server = pair(&phone, &desk.open_pairing(), &[]).await.unwrap();
        let phone_id = ops::device_id(&phone.lock().unwrap()).unwrap();
        crate::lan::forget_device(&desk.database.lock().unwrap(), &phone_id).unwrap();

        let result = sync(&phone, &server.device_id).await;

        assert!(matches!(result, Err(Error::Lan("rejected"))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn nobody_listening_is_unreachable() {
        let desk = serve().await;
        let code = desk.open_pairing();
        drop(desk);

        let result = pair(&Database::in_memory(), &code, &[]).await;

        assert!(matches!(result, Err(Error::Lan("unreachable"))));
    }

    #[test]
    fn subnet_hosts_skip_network_broadcast_and_self() {
        let hosts = hosts_in(Ipv4Addr::new(192, 168, 1, 149), 24);
        assert_eq!(hosts.len(), 253);
        assert_eq!(hosts.first(), Some(&Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(hosts.last(), Some(&Ipv4Addr::new(192, 168, 1, 254)));
        assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 149)));
        assert_eq!(
            hosts_in(Ipv4Addr::new(10, 0, 3, 7), 8).len(),
            1021,
            "wide networks are capped"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_moved_device_is_found_only_with_its_certificate_and_id() {
        let desk = serve().await;
        let desk_id = ops::device_id(&desk.database.lock().unwrap()).unwrap();
        let right = desk.server.address.to_string();
        let candidates = vec!["127.0.0.1:1".to_owned(), right.clone()];

        let found = find_among(desk.identity.fingerprint(), &desk_id, candidates.clone()).await;
        assert_eq!(found, Some(right));

        assert_eq!(
            find_among(
                desk.identity.fingerprint(),
                "someone-else",
                candidates.clone()
            )
            .await,
            None
        );
        let impostor = Identity::generate().unwrap();
        assert_eq!(
            find_among(impostor.fingerprint(), &desk_id, candidates).await,
            None
        );
    }
}
