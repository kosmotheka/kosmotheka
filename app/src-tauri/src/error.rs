use library_domain::{FieldError, OperationError};
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
    #[error("database lock poisoned by a panicked thread")]
    LockPoisoned,
}

/// Shape of an error as the UI receives it.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ErrorPayload<'a> {
    Validation { errors: &'a [FieldError] },
    Internal { message: String },
}

// Tauri sends command errors to the UI as JSON, so they must be serializable.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let payload = match self {
            Error::Validation(errors) => ErrorPayload::Validation { errors },
            other => ErrorPayload::Internal {
                message: other.to_string(),
            },
        };
        payload.serialize(serializer)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
