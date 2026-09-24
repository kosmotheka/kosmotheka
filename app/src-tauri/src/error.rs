use library_domain::FieldError;
use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid input")]
    Validation(Vec<FieldError>),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Migration(#[from] rusqlite_migration::Error),
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
