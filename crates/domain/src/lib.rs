//! Domain model shared by every part of Kosmotheka: types, validation and rules.
//! It has no storage or UI code, so it can be reused by any client.

pub mod book;
pub mod hlc;
pub mod isbn;
pub mod library;
pub mod mark;
pub mod operation;
pub mod permission;
pub mod schema;
pub mod signature;
pub mod wish;

pub use book::{
    COVER_HOSTS, ContributorInput, ContributorRole, CopyInput, EditionInput, ErrorCode, FieldError,
    Format, Kind, KindDetails, NewBook, ValidBook, ValidContributor, ValidCopy, ValidEdition,
    parse_cover_url,
};
pub use hlc::{Hlc, HlcClock};
pub use isbn::{Isbn, IsbnError};
pub use library::validate_library_name;
pub use mark::mark_id;
pub use operation::{Action, Change, Operation, OperationError, OperationErrorCode};
pub use permission::{LibraryView, Role, is_allowed};
pub use schema::{DETAILS, Entity, FieldSpec, FieldType};
pub use signature::{DeviceKey, SignatureError, canonical_bytes, verify};
pub use wish::{ValidWish, WishInput};
