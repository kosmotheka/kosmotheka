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

use super::identity::fingerprint;
use super::pairing::PairingCode;
use super::{
    KnownServer, PairRequest, PairResponse, PullRequest, PullResponse, PushRequest, SyncReport,
    device_name, find_server, record_server_sync, save_server,
};
use crate::db::Database;
use crate::error::{Error, Result};
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
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(60))
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

/// Pairs with the device that showed `code_text` and remembers it.
pub async fn pair(database: &Database, code_text: &str) -> Result<KnownServer> {
    let code = PairingCode::decode(code_text).ok_or(Error::Lan("bad_code"))?;
    let request = {
        let conn = database.lock()?;
        if code.device_id == ops::device_id(&conn)? {
            return Err(Error::Lan("same_device"));
        }
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
    save_server(&*database.lock()?, &server)?;
    Ok(server)
}

/// Pulls what the peer has, then pushes what it lacks.
pub async fn sync(database: &Database, device_id: &str) -> Result<SyncReport> {
    let (server, local_device, pull) = {
        let conn = database.lock()?;
        let server = find_server(&conn, device_id)?.ok_or(Error::Lan("unknown_device"))?;
        let pull = PullRequest {
            schema_version: ops::schema_version(&conn)?,
            vector: sync::version_vector(&conn)?,
        };
        (server, ops::device_id(&conn)?, pull)
    };
    let client = pinned_client(&server.fingerprint)?;
    let auth = Some((local_device.as_str(), server.secret.as_str()));

    let (pulled, address): (PullResponse, String) =
        post(&client, &server.addresses, "/v1/sync/pull", auth, &pull).await?;
    let (received, to_send) = {
        let mut conn = database.lock()?;
        let received = sync::merge(&mut conn, pulled.operations)?;
        (received, sync::operations_since(&conn, &pulled.vector)?)
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
    Ok(SyncReport { received, sent })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use library_domain::{CopyInput, EditionInput, Format, ValidBook};
    use tokio::net::TcpListener;

    use super::*;
    use crate::catalog::{self, BookFilter};
    use crate::lan::identity::Identity;
    use crate::lan::pairing::PairingWindow;
    use crate::lan::server::{self, RunningServer, ServerState};
    use crate::lan::{list_devices, random_secret};

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
            }
            .validate()
            .unwrap(),
        };
        catalog::create_book(&mut database.lock().unwrap(), &book, None).unwrap();
    }

    fn titles(database: &Database) -> Vec<String> {
        catalog::list_books(&database.lock().unwrap(), &BookFilter::default())
            .unwrap()
            .into_iter()
            .map(|b| b.title)
            .collect()
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
            *self.state.pairing.lock().unwrap() = Some(PairingWindow::open(token.clone()));
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
        add(&phone, "On the phone");

        let server = pair(&phone, &desk.open_pairing()).await.unwrap();
        let report = sync(&phone, &server.device_id).await.unwrap();

        assert_eq!(report.received.received, 1);
        assert_eq!(report.sent, 1);
        assert_eq!(titles(&phone), vec!["On the desk", "On the phone"]);
        assert_eq!(titles(&desk.database), vec!["On the desk", "On the phone"]);
        let clients = list_devices(&desk.database.lock().unwrap(), "known_clients").unwrap();
        assert_eq!(clients.len(), 1);
        assert!(clients[0].last_sync_at.is_some());

        // A second sync has nothing to do.
        let again = sync(&phone, &server.device_id).await.unwrap();
        assert_eq!((again.received.received, again.sent), (0, 0));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_pairing_code_works_only_once() {
        let desk = serve().await;
        let code = desk.open_pairing();

        pair(&Database::in_memory(), &code).await.unwrap();
        let second = pair(&Database::in_memory(), &code).await;

        assert!(matches!(second, Err(Error::Lan("rejected"))));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_different_certificate_is_refused() {
        let desk = serve().await;
        let mut code = PairingCode::decode(&desk.open_pairing()).unwrap();
        code.fingerprint = Identity::generate().unwrap().fingerprint().to_owned();

        let result = pair(&Database::in_memory(), &code.encode()).await;

        assert!(
            matches!(result, Err(Error::Lan("wrong_certificate"))),
            "{result:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_forgotten_device_can_no_longer_sync() {
        let desk = serve().await;
        let phone = Database::in_memory();
        let server = pair(&phone, &desk.open_pairing()).await.unwrap();
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

        let result = pair(&Database::in_memory(), &code).await;

        assert!(matches!(result, Err(Error::Lan("unreachable"))));
    }
}
