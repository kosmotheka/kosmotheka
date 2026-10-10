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
    /// The reason is a code the UI translates: not_a_backup, unreadable, other_version, not_empty,
    /// passphrase_needed, wrong_passphrase, passphrase_empty (D86).
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
    /// Taking the library to another device needs «Can share» or administrator (D73).
    #[error("you may not share this library")]
    CannotShare,
    /// Cloud sync failures (D84), as codes the UI translates: not_found, schema_version,
    /// not_signed_in, unsigned_history, already_linked.
    #[error("cloud sync failed: {0}")]
    Cloud(&'static str),
    /// The cloud cut this device or its account from a library (D98, D71), and sent the signed operations that
    /// withdrew the device. It proves nothing by itself: the device checks the proofs before it removes anything.
    #[error("the cloud withdrew the access to this library")]
    AccessWithdrawn(Vec<crate::sync::SyncOperation>),
    /// The sync server rejected an account call: the code is whatever it sent (an
    /// unknown service, so it stays a string), `retry_after` only set for 429s.
    #[error("account error: {code}")]
    Account {
        code: String,
        retry_after: Option<u64>,
    },
}

impl Error {
    pub fn account(code: &str) -> Self {
        Error::Account {
            code: code.to_owned(),
            retry_after: None,
        }
    }
}

/// Shape of an error as the UI receives it.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ErrorPayload<'a> {
    Validation {
        errors: &'a [FieldError],
    },
    Backup {
        reason: &'static str,
    },
    Sync {
        reason: &'static str,
    },
    /// Roles said no (D73): the UI explains it instead of showing an internal error.
    Permission {
        reason: &'static str,
    },
    Cloud {
        reason: &'static str,
    },
    Internal {
        message: String,
    },
    Account {
        reason: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        retry_after: Option<u64>,
    },
}

// Tauri sends command errors to the UI as JSON, so they must be serializable.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let payload = match self {
            Error::Validation(errors) => ErrorPayload::Validation { errors },
            Error::BadBackup(reason) => ErrorPayload::Backup { reason },
            Error::Lan(reason) => ErrorPayload::Sync { reason },
            Error::NotAllowed => ErrorPayload::Permission {
                reason: "not_allowed",
            },
            Error::LastAdmin => ErrorPayload::Permission {
                reason: "last_admin",
            },
            Error::RequestResolved => ErrorPayload::Permission {
                reason: "request_resolved",
            },
            Error::CannotShare => ErrorPayload::Permission {
                reason: "cannot_share",
            },
            Error::Cloud(reason) => ErrorPayload::Cloud { reason },
            Error::AccessWithdrawn(_) => ErrorPayload::Cloud {
                reason: "access_withdrawn",
            },
            Error::Account { code, retry_after } => ErrorPayload::Account {
                reason: code,
                retry_after: *retry_after,
            },
            other => ErrorPayload::Internal {
                message: other.to_string(),
            },
        };
        payload.serialize(serializer)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
