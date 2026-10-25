//! The HTTPS endpoint other devices sync with. It runs only while this device has
//! paired clients or a pairing window is open.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;

use super::pairing::{PairingWindow, PinSession};
use super::pin;
use super::{
    Hello, PairRequest, PairResponse, PinAnswer, PinFinish, PinStart, PullRequest, PullResponse,
    PushRequest, client_is_authorized, device_name, is_known_client, notices_for, only_shared,
    only_shared_operations, random_secret, reconcile, record_client_sync, register_in_shared,
    save_client, share_library, shared_libraries,
};
use crate::cover_exchange::{self, Inventory, Swap};
use crate::covers::{Covers, MAX_COVER_BYTES};
use crate::db::Database;
use crate::error::Result;
use crate::ops;
use crate::sync::{self, MergeReport};

pub const APP_ID: &str = "kosmotheka";

/// Called when a client's push brought operations this device did not have yet.
pub type ArrivalHook = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct ServerState {
    pub database: Database,
    pub pairing: Arc<Mutex<Option<PairingWindow>>>,
    /// Set once by the app (it tells the UI to reload); read at each push, so it does
    /// not matter whether the server started before or after it was set.
    pub arrival: Arc<OnceLock<ArrivalHook>>,
    /// Set once by the app, like `arrival`: without it no cover is served or taken (D82).
    pub covers: Arc<OnceLock<Covers>>,
    /// The fingerprint of the certificate this device serves with, which pairing by number confirms (D88).
    pub fingerprint: String,
}

pub struct RunningServer {
    pub address: SocketAddr,
    task: JoinHandle<()>,
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Listens on `listener` until the returned handle is dropped.
pub fn start(
    listener: TcpListener,
    tls: Arc<rustls::ServerConfig>,
    state: ServerState,
) -> Result<RunningServer> {
    let address = listener.local_addr()?;
    let acceptor = TlsAcceptor::from(tls);
    let app = router(state);
    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            let (acceptor, app) = (acceptor.clone(), app.clone());
            tokio::spawn(async move {
                // A peer that fails the handshake (wrong pin, not our app) is simply dropped.
                let Ok(tls_stream) = acceptor.accept(stream).await else {
                    return;
                };
                let _ = Builder::new(TokioExecutor::new())
                    .serve_connection(TokioIo::new(tls_stream), TowerToHyperService::new(app))
                    .await;
            });
        }
    });
    Ok(RunningServer { address, task })
}

fn router(state: ServerState) -> Router {
    Router::new()
        .route("/v1/hello", get(hello))
        .route("/v1/pair", post(pair))
        .route("/v1/pair/pin/start", post(pin_start))
        .route("/v1/pair/pin/finish", post(pin_finish))
        .route("/v1/sync/pull", post(pull))
        .route("/v1/sync/push", post(push))
        .route("/v1/covers/inventory", post(cover_inventory))
        .route(
            "/v1/covers/{name}",
            get(cover_get)
                .put(cover_put)
                .layer(DefaultBodyLimit::max(MAX_COVER_BYTES)),
        )
        .with_state(state)
}

/// Errors as HTTP statuses the client maps back to reasons.
enum ApiError {
    Unauthorized,
    AlreadyPaired,
    OtherVersion,
    NotFound,
    Rejected,
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::AlreadyPaired => StatusCode::FORBIDDEN,
            ApiError::OtherVersion => StatusCode::CONFLICT,
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Rejected => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response()
    }
}

impl From<crate::error::Error> for ApiError {
    fn from(_: crate::error::Error) -> Self {
        ApiError::Internal
    }
}

/// Database work runs on the blocking pool: SQLite calls must not stall the async runtime.
async fn blocking<T: Send + 'static>(
    job: impl FnOnce() -> std::result::Result<T, ApiError> + Send + 'static,
) -> std::result::Result<T, ApiError> {
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|_| ApiError::Internal)?
}

async fn hello(State(state): State<ServerState>) -> std::result::Result<Json<Hello>, ApiError> {
    blocking(move || {
        let conn = state.database.lock()?;
        Ok(Json(Hello {
            app: APP_ID.to_owned(),
            device_id: ops::device_id(&conn)?,
            device_name: device_name(&conn)?,
            schema_version: ops::schema_version(&conn)?,
            pairing_open: state
                .pairing
                .lock()
                .map_err(|_| ApiError::Internal)?
                .as_ref()
                .is_some_and(PairingWindow::is_open),
        }))
    })
    .await
}

async fn pair(
    State(state): State<ServerState>,
    Json(request): Json<PairRequest>,
) -> std::result::Result<Json<PairResponse>, ApiError> {
    let libraries = {
        // One use only: a code seen by someone else cannot pair a second device.
        let mut window = state.pairing.lock().map_err(|_| ApiError::Internal)?;
        match window.take() {
            Some(open) if open.accepts(&request.token) => open.libraries,
            other => {
                *window = other;
                return Err(ApiError::Unauthorized);
            }
        }
    };
    accept(state, libraries, request.device_id, request.device_name).await
}

/// The first half of pairing by number (D88): this device answers the exchange with the number it shows, and
/// confirms the key bound to its certificate. Every exchange is one try of the window's few.
async fn pin_start(
    State(state): State<ServerState>,
    Json(request): Json<PinStart>,
) -> std::result::Result<Json<PinAnswer>, ApiError> {
    let message = decode(&request.message)?;
    let mut window = state.pairing.lock().map_err(|_| ApiError::Internal)?;
    let open = window
        .as_mut()
        .filter(|open| open.is_open())
        .ok_or(ApiError::Unauthorized)?;
    // A few tries per window, each a whole exchange: guessing the number means talking to this device each time.
    if open.pin_attempts >= pin::PIN_ATTEMPTS {
        *window = None;
        return Err(ApiError::Unauthorized);
    }
    open.pin_attempts += 1;
    let (key, answer) = pin::answer(&open.pin, &message).ok_or(ApiError::Rejected)?;
    let session = random_secret();
    let confirmation = encode(&pin::showing_confirmation(&key, &state.fingerprint));
    open.pin_session = Some(PinSession {
        id: session.clone(),
        key,
    });
    Ok(Json(PinAnswer {
        session,
        message: encode(&answer),
        confirmation,
    }))
}

/// The second half: the joining device proves it reached the same key, so it knew the number, and pairs.
async fn pin_finish(
    State(state): State<ServerState>,
    Json(request): Json<PinFinish>,
) -> std::result::Result<Json<PairResponse>, ApiError> {
    let given = decode(&request.confirmation)?;
    let libraries = {
        let mut window = state.pairing.lock().map_err(|_| ApiError::Internal)?;
        let open = window.take().ok_or(ApiError::Unauthorized)?;
        let confirmed = open.is_open()
            && open.pin_session.as_ref().is_some_and(|session| {
                session.id == request.session
                    && pin::confirms(
                        &pin::joining_confirmation(
                            &session.key,
                            &state.fingerprint,
                            &request.device_id,
                        ),
                        &given,
                    )
            });
        if !confirmed {
            // The last try spent closes the window; otherwise it stays open for the next.
            if open.pin_attempts < pin::PIN_ATTEMPTS {
                *window = Some(PairingWindow {
                    pin_session: None,
                    ..open
                });
            }
            return Err(ApiError::Unauthorized);
        }
        open.libraries
    };
    accept(state, libraries, request.device_id, request.device_name).await
}

/// What both ways of pairing end in: the new device is remembered with a secret of its own, and gets what the window
/// shares.
async fn accept(
    state: ServerState,
    libraries: Vec<String>,
    device_id: String,
    name: String,
) -> std::result::Result<Json<PairResponse>, ApiError> {
    blocking(move || {
        let conn = state.database.lock()?;
        // The id is the client's word: taking over a paired one would cut that device
        // off and hand over what was shared with it. It has to be forgotten first.
        if is_known_client(&conn, &device_id)? {
            return Err(ApiError::AlreadyPaired);
        }
        let secret = random_secret();
        save_client(&conn, &device_id, &name, &secret)?;
        for library in &libraries {
            share_library(&conn, library, &device_id)?;
        }
        Ok(Json(PairResponse {
            device_id: ops::device_id(&conn)?,
            device_name: device_name(&conn)?,
            secret,
        }))
    })
    .await
}

fn encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn decode(text: &str) -> std::result::Result<Vec<u8>, ApiError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ApiError::Rejected)
}

async fn pull(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<PullRequest>,
) -> std::result::Result<Json<PullResponse>, ApiError> {
    blocking(move || {
        let mut conn = state.database.lock()?;
        let client = authorized_client(&conn, &headers)?;
        if request.schema_version != ops::schema_version(&conn)? {
            return Err(ApiError::OtherVersion);
        }
        let offered: Vec<String> = request.vectors.keys().cloned().collect();
        let shared = reconcile(&mut conn, &client, &offered, &request.notices)?;
        let theirs = only_shared(request.vectors, &shared);
        let response = PullResponse {
            operations: sync::operations_since(&conn, &theirs)?,
            vectors: sync::version_vectors(&conn, &shared)?,
            notices: notices_for(&conn, &client)?,
        };
        record_client_sync(&conn, &client)?;
        Ok(Json(response))
    })
    .await
}

async fn push(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(request): Json<PushRequest>,
) -> std::result::Result<Json<MergeReport>, ApiError> {
    blocking(move || {
        let mut conn = state.database.lock()?;
        let client = authorized_client(&conn, &headers)?;
        let shared = shared_libraries(&conn, &client)?;
        let operations = only_shared_operations(request.operations, &shared);
        let report = sync::merge(&mut conn, operations)?;
        register_in_shared(&mut conn, &client)?;
        // The device that only serves never pulls: this is when it has what the peer had (D96).
        crate::permissions::vouch_if_missing(&mut conn, &shared)?;
        crate::permissions::complete_withdrawals(&mut conn, &shared)?;
        drop(conn);
        if report.received > 0
            && let Some(arrival) = state.arrival.get()
        {
            arrival();
        }
        Ok(Json(report))
    })
    .await
}

fn authorized_client(
    conn: &rusqlite::Connection,
    headers: &HeaderMap,
) -> std::result::Result<String, ApiError> {
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let device_id = header("x-device-id").ok_or(ApiError::Unauthorized)?;
    let secret = header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;
    if client_is_authorized(conn, device_id, secret)? {
        Ok(device_id.to_owned())
    } else {
        Err(ApiError::Unauthorized)
    }
}

// --- Covers (D82) ---------------------------------------------------------------------------------------------------

/// The covers of the editions of the libraries shared with the client, which are all it may get or give, and where
/// this device keeps covers. Nothing before the app set them.
fn shared_covers(
    state: &ServerState,
    headers: &HeaderMap,
) -> std::result::Result<(std::collections::BTreeSet<String>, Covers), ApiError> {
    let covers = state.covers.get().cloned().ok_or(ApiError::NotFound)?;
    let conn = state.database.lock()?;
    let client = authorized_client(&conn, headers)?;
    let shared = shared_libraries(&conn, &client)?;
    Ok((cover_exchange::referenced(&conn, &shared)?, covers))
}

async fn cover_inventory(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(theirs): Json<Inventory>,
) -> std::result::Result<Json<Swap>, ApiError> {
    blocking(move || {
        let (referenced, covers) = shared_covers(&state, &headers)?;
        Ok(Json(cover_exchange::answer(&covers, &referenced, &theirs)))
    })
    .await
}

async fn cover_get(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> std::result::Result<Vec<u8>, ApiError> {
    blocking(move || {
        let (referenced, covers) = shared_covers(&state, &headers)?;
        // Only covers of editions this device may receive: knowing a name opens nothing.
        if !referenced.contains(&name) {
            return Err(ApiError::NotFound);
        }
        covers.read(&name).ok_or(ApiError::NotFound)
    })
    .await
}

async fn cover_put(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    body: Bytes,
) -> std::result::Result<StatusCode, ApiError> {
    blocking(move || {
        let (referenced, covers) = shared_covers(&state, &headers)?;
        if !referenced.contains(&name) || covers.has(&name) {
            return Err(ApiError::NotFound);
        }
        covers
            .store_arrived(&name, &body)
            .map_err(|_| ApiError::Rejected)?;
        if let Some(arrival) = state.arrival.get() {
            arrival();
        }
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}
