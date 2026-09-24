//! Domain model shared by every part of the library app: types, validation and rules.
//! It has no storage or UI code, so it can be reused by any client.

pub mod book;
pub mod isbn;

pub use book::{ErrorCode, FieldError, Format, NewBook, NewCopy, ValidBook, ValidCopy};
pub use isbn::{Isbn, IsbnError};
