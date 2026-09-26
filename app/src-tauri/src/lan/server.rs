//! The HTTPS endpoint other devices sync with. It runs only while this device has
//! paired clients or a pairing window is open.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::State;
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

use super::pairing::PairingWindow;
use super::{
    Hello, PairRequest, PairResponse, PullRequest, PullResponse, PushRequest, client_is_authorized,
    device_name, notices_for, only_shared, only_shared_operations, random_secret, reconcile,
    record_client_sync, save_client, share_library, shared_libraries,
};
use crate::db::Database;
use crate::error::Result;
use crate::ops;
use crate::sync::{self, MergeReport};

pub const APP_ID: &str = "kosmotheka";

#[derive(Clone)]
pub struct ServerState {
    pub database: Database,
    pub pairing: Arc<Mutex<Option<PairingWindow>>>,
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
        .route("/v1/sync/pull", post(pull))
        .route("/v1/sync/push", post(push))
        .with_state(state)
}

/// Errors as HTTP statuses the client maps back to reasons.
enum ApiError {
    Unauthorized,
    OtherVersion,
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::OtherVersion => StatusCode::CONFLICT,
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
    blocking(move || {
        let conn = state.database.lock()?;
        let secret = random_secret();
        save_client(&conn, &request.device_id, &request.device_name, &secret)?;
        for library in &libraries {
            share_library(&conn, library, &request.device_id)?;
        }
        Ok(Json(PairResponse {
            device_id: ops::device_id(&conn)?,
            device_name: device_name(&conn)?,
            secret,
        }))
    })
    .await
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
        Ok(Json(sync::merge(&mut conn, operations)?))
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
