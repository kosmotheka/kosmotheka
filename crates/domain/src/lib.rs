//! Domain model shared by every part of the library app: types, validation and rules.
//! It has no storage or UI code, so it can be reused by any client.

pub mod book;
pub mod hlc;
pub mod isbn;
pub mod operation;
pub mod schema;

pub use book::{ErrorCode, FieldError, Format, NewBook, NewCopy, ValidBook, ValidCopy};
pub use hlc::{Hlc, HlcClock};
pub use isbn::{Isbn, IsbnError};
pub use operation::{Action, Change, Operation, OperationError, OperationErrorCode};
pub use schema::{Entity, FieldSpec, FieldType};
