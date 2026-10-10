//! Domain model shared by every part of Kosmotheka: types, validation and rules.
//! It has no storage or UI code, so it can be reused by any client.

pub mod book;
pub mod cloud;
pub mod genre;
pub mod hlc;
pub mod isbn;
pub mod library;
pub mod mark;
pub mod operation;
pub mod permission;
pub mod schema;
pub mod signature;

pub use book::{
    COVER_HOSTS, ContributorInput, ContributorRole, CopyInput, EditionInput, ErrorCode, FieldError,
    Format, Kind, KindDetails, MAX_TAG_LENGTH, NewBook, ValidBook, ValidContributor, ValidCopy,
    ValidEdition, parse_cover_url,
};
pub use cloud::cloud_device_id;
pub use genre::{GENRES, genres_of};
pub use hlc::{Hlc, HlcClock};
pub use isbn::{Isbn, IsbnError};
pub use library::validate_library_name;
pub use mark::mark_id;
pub use operation::{Action, Change, Operation, OperationError, OperationErrorCode};
pub use permission::{LibraryView, Role, is_allowed};
pub use schema::{DETAILS, Entity, FieldSpec, FieldType, WORK_RELATIONS};
pub use signature::{DeviceKey, SignatureError, canonical_bytes, verify};
