use kosmotheka_domain::{FieldError, OperationError};
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input")]
    Validation(Vec<FieldError>),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
    #[error(transparent)]
    InvalidOperation(#[from] OperationError),
    #[error("{entity} {id} does not exist")]
    MissingRow { entity: &'static str, id: String },
    #[error("this device has no id; the database was not initialised")]
    MissingDeviceId,
    #[error("corrupt local state: {0}")]
    CorruptState(&'static str),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("cover rejected: {0}")]
    CoverRejected(&'static str),
    /// The reason is a code the UI translates: not_a_backup, unreadable, other_version, not_empty.
    #[error("backup rejected: {0}")]
    BadBackup(&'static str),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    /// LAN sync failures, as codes the UI translates: bad_code, same_device, unreachable,
    /// wrong_certificate, rejected, other_version, peer_error, unknown_device.
    #[error("sync failed: {0}")]
    Lan(&'static str),
    #[error("TLS setup failed: {0}")]
    Tls(&'static str),
    #[error(transparent)]
    Csv(#[from] csv::Error),
    #[error("database lock poisoned by a panicked thread")]
    LockPoisoned,
    /// Leaving a library no other device has would lose it: it has to be deleted,
    /// which offers a backup first.
    #[error("this library is only on this device")]
    OnlyOnThisDevice,
    /// The author's role does not allow this operation (D73). Asking for it instead
    /// comes with requests (milestone 5.5).
    #[error("your role in this library does not allow this")]
    NotAllowed,
    /// Another administrator, maybe on another device, got there first.
    #[error("this request was already accepted or rejected")]
    RequestResolved,
    /// The library would be left without anyone who can manage it (D65).
    #[error("this is the library's only administrator")]
    LastAdmin,
}

/// Shape of an error as the UI receives it.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ErrorPayload<'a> {
    Validation { errors: &'a [FieldError] },
    Backup { reason: &'static str },
    Sync { reason: &'static str },
    Internal { message: String },
}

// Tauri sends command errors to the UI as JSON, so they must be serializable.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let payload = match self {
            Error::Validation(errors) => ErrorPayload::Validation { errors },
            Error::BadBackup(reason) => ErrorPayload::Backup { reason },
            Error::Lan(reason) => ErrorPayload::Sync { reason },
            other => ErrorPayload::Internal {
                message: other.to_string(),
            },
        };
        payload.serialize(serializer)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
