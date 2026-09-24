use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::isbn::{Isbn, IsbnError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Hardcover,
    Paperback,
    Pocket,
    Ebook,
    Audiobook,
    Other,
}

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Hardcover => "hardcover",
            Format::Paperback => "paperback",
            Format::Pocket => "pocket",
            Format::Ebook => "ebook",
            Format::Audiobook => "audiobook",
            Format::Other => "other",
        }
    }
}

impl FromStr for Format {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hardcover" => Ok(Format::Hardcover),
            "paperback" => Ok(Format::Paperback),
            "pocket" => Ok(Format::Pocket),
            "ebook" => Ok(Format::Ebook),
            "audiobook" => Ok(Format::Audiobook),
            "other" => Ok(Format::Other),
            _ => Err(()),
        }
    }
}

/// A book as entered by the user, before validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBook {
    pub title: String,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub published_year: Option<i32>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub isbn: Option<String>,
    pub copy: NewCopy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCopy {
    #[serde(default)]
    pub owner: Option<String>,
    pub format: Format,
    #[serde(default = "one")]
    pub quantity: u32,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub acquired_on: Option<String>,
}

fn one() -> u32 {
    1
}

/// A book that passed validation: text is trimmed, empty values are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidBook {
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub isbn: Option<Isbn>,
    pub copy: ValidCopy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidCopy {
    pub owner: Option<String>,
    pub format: Format,
    pub quantity: u32,
    pub location: Option<String>,
    pub acquired_on: Option<NaiveDate>,
}

/// Codes rather than messages, so the UI can translate them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Required,
    OutOfRange,
    InvalidDate,
    IsbnLength,
    IsbnCharacter,
    IsbnChecksum,
}

impl From<IsbnError> for ErrorCode {
    fn from(error: IsbnError) -> Self {
        match error {
            IsbnError::InvalidLength => ErrorCode::IsbnLength,
            IsbnError::InvalidCharacter => ErrorCode::IsbnCharacter,
            IsbnError::InvalidChecksum => ErrorCode::IsbnChecksum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub field: &'static str,
    pub code: ErrorCode,
}

pub const MAX_YEAR: i32 = 2100;
pub const MAX_QUANTITY: u32 = 999;

impl NewBook {
    /// Checks every field and reports all problems at once, not just the first.
    pub fn validate(self) -> Result<ValidBook, Vec<FieldError>> {
        let mut errors = Vec::new();
        let mut fail = |field, code| errors.push(FieldError { field, code });

        let title = clean(Some(self.title));
        if title.is_none() {
            fail("title", ErrorCode::Required);
        }

        let published_year = self.published_year;
        if published_year.is_some_and(|year| !(1..=MAX_YEAR).contains(&year)) {
            fail("publishedYear", ErrorCode::OutOfRange);
        }

        let isbn = match clean(self.isbn) {
            Some(raw) => match Isbn::parse(&raw) {
                Ok(isbn) => Some(isbn),
                Err(error) => {
                    fail("isbn", error.into());
                    None
                }
            },
            None => None,
        };

        let quantity = self.copy.quantity;
        if !(1..=MAX_QUANTITY).contains(&quantity) {
            fail("copy.quantity", ErrorCode::OutOfRange);
        }

        let acquired_on = match clean(self.copy.acquired_on) {
            Some(raw) => match NaiveDate::parse_from_str(&raw, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    fail("copy.acquiredOn", ErrorCode::InvalidDate);
                    None
                }
            },
            None => None,
        };

        match title {
            Some(title) if errors.is_empty() => Ok(ValidBook {
                title,
                subtitle: clean(self.subtitle),
                authors: self
                    .authors
                    .into_iter()
                    .filter_map(|a| clean(Some(a)))
                    .collect(),
                publisher: clean(self.publisher),
                published_year,
                language: clean(self.language),
                isbn,
                copy: ValidCopy {
                    owner: clean(self.copy.owner),
                    format: self.copy.format,
                    quantity,
                    location: clean(self.copy.location),
                    acquired_on,
                },
            }),
            _ => Err(errors),
        }
    }
}

/// Trims whitespace and turns empty strings into `None`.
fn clean(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal(title: &str) -> NewBook {
        NewBook {
            title: title.to_owned(),
            subtitle: None,
            authors: vec![],
            publisher: None,
            published_year: None,
            language: None,
            isbn: None,
            copy: NewCopy {
                owner: None,
                format: Format::Paperback,
                quantity: 1,
                location: None,
                acquired_on: None,
            },
        }
    }

    #[test]
    fn trims_text_and_drops_empty_values() {
        let mut book = minimal("  The Name of the Wind ");
        book.subtitle = Some("   ".to_owned());
        book.authors = vec![" Patrick Rothfuss ".to_owned(), "".to_owned()];

        let valid = book.validate().unwrap();

        assert_eq!(valid.title, "The Name of the Wind");
        assert_eq!(valid.subtitle, None);
        assert_eq!(valid.authors, vec!["Patrick Rothfuss"]);
    }

    #[test]
    fn normalizes_isbn_and_parses_date() {
        let mut book = minimal("Title");
        book.isbn = Some("0-306-40615-2".to_owned());
        book.copy.acquired_on = Some("2026-09-24".to_owned());

        let valid = book.validate().unwrap();

        assert_eq!(valid.isbn.unwrap().as_str(), "9780306406157");
        assert_eq!(valid.copy.acquired_on, NaiveDate::from_ymd_opt(2026, 9, 24));
    }

    #[test]
    fn reports_every_invalid_field() {
        let mut book = minimal(" ");
        book.published_year = Some(0);
        book.isbn = Some("9780306406158".to_owned());
        book.copy.quantity = 0;
        book.copy.acquired_on = Some("2026-02-30".to_owned());

        let errors = book.validate().unwrap_err();

        assert_eq!(
            errors,
            vec![
                FieldError {
                    field: "title",
                    code: ErrorCode::Required
                },
                FieldError {
                    field: "publishedYear",
                    code: ErrorCode::OutOfRange
                },
                FieldError {
                    field: "isbn",
                    code: ErrorCode::IsbnChecksum
                },
                FieldError {
                    field: "copy.quantity",
                    code: ErrorCode::OutOfRange
                },
                FieldError {
                    field: "copy.acquiredOn",
                    code: ErrorCode::InvalidDate
                },
            ]
        );
    }

    #[test]
    fn format_round_trips_through_its_database_name() {
        for format in [
            Format::Hardcover,
            Format::Paperback,
            Format::Pocket,
            Format::Ebook,
            Format::Audiobook,
            Format::Other,
        ] {
            assert_eq!(format.as_str().parse(), Ok(format));
        }
    }
}
