use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::isbn::{Isbn, IsbnError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Hardcover,
    Paperback,
    Pocket,
    Ebook,
    Pdf,
    Audiobook,
    SingleIssue,
    Box,
    Dvd,
    Bluray,
    Uhd,
    Physical,
    Digital,
    Vinyl,
    Cd,
    Cassette,
    Other,
}

impl Format {
    pub const ALL: [Format; 17] = [
        Format::Hardcover,
        Format::Paperback,
        Format::Pocket,
        Format::Ebook,
        Format::Pdf,
        Format::Audiobook,
        Format::SingleIssue,
        Format::Box,
        Format::Dvd,
        Format::Bluray,
        Format::Uhd,
        Format::Physical,
        Format::Digital,
        Format::Vinyl,
        Format::Cd,
        Format::Cassette,
        Format::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Format::Hardcover => "hardcover",
            Format::Paperback => "paperback",
            Format::Pocket => "pocket",
            Format::Ebook => "ebook",
            Format::Pdf => "pdf",
            Format::Audiobook => "audiobook",
            Format::SingleIssue => "single_issue",
            Format::Box => "box",
            Format::Dvd => "dvd",
            Format::Bluray => "bluray",
            Format::Uhd => "uhd",
            Format::Physical => "physical",
            Format::Digital => "digital",
            Format::Vinyl => "vinyl",
            Format::Cd => "cd",
            Format::Cassette => "cassette",
            Format::Other => "other",
        }
    }
}

impl FromStr for Format {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Format::ALL
            .into_iter()
            .find(|format| format.as_str() == s)
            .ok_or(())
    }
}

/// Edition data as entered by the user, before validation. Used to add and to edit.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditionInput {
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
    /// Remote cover to download when saving; only known cover hosts are accepted.
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub google_books_id: Option<String>,
    #[serde(default)]
    pub open_library_id: Option<String>,
}

/// A copy as entered by the user, before validation. Used to add and to edit.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyInput {
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

/// A new book: its edition plus the first copy.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBook {
    #[serde(flatten)]
    pub edition: EditionInput,
    pub copy: CopyInput,
}

/// Edition that passed validation: text is trimmed, empty values are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidEdition {
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub isbn: Option<Isbn>,
    pub cover_url: Option<Url>,
    pub google_books_id: Option<String>,
    pub open_library_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidCopy {
    pub owner: Option<String>,
    pub format: Format,
    pub quantity: u32,
    pub location: Option<String>,
    pub acquired_on: Option<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidBook {
    pub edition: ValidEdition,
    pub copy: ValidCopy,
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
    /// Another edition already has this ISBN.
    IsbnInUse,
    /// Not an https URL on a known cover host.
    InvalidCoverUrl,
    /// Not one of the known copy formats (only possible when importing files).
    InvalidFormat,
    /// Another library on this device already has this name.
    NameInUse,
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

/// `field` uses the UI's names (camelCase, `copy.` prefix inside a new book).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    pub field: String,
    pub code: ErrorCode,
}

impl FieldError {
    pub fn new(field: impl Into<String>, code: ErrorCode) -> Self {
        Self {
            field: field.into(),
            code,
        }
    }
}

pub const MAX_YEAR: i32 = 2100;

/// Covers are only downloaded from these hosts, so the app cannot be used to make
/// requests to arbitrary servers (or to the local network).
pub const COVER_HOSTS: &[&str] = &[
    "covers.openlibrary.org",
    "books.google.com",
    "books.googleusercontent.com",
];

pub fn parse_cover_url(raw: &str) -> Option<Url> {
    let url = Url::parse(raw).ok()?;
    let trusted = url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url
            .host_str()
            .is_some_and(|host| COVER_HOSTS.contains(&host));
    trusted.then_some(url)
}
pub const MAX_QUANTITY: u32 = 999;

impl EditionInput {
    /// Checks every field and reports all problems at once, not just the first.
    pub fn validate(self) -> Result<ValidEdition, Vec<FieldError>> {
        let mut errors = Vec::new();

        let title = clean(Some(self.title));
        if title.is_none() {
            errors.push(FieldError::new("title", ErrorCode::Required));
        }

        let published_year = self.published_year;
        if published_year.is_some_and(|year| !(1..=MAX_YEAR).contains(&year)) {
            errors.push(FieldError::new("publishedYear", ErrorCode::OutOfRange));
        }

        let isbn = match clean(self.isbn) {
            Some(raw) => match Isbn::parse(&raw) {
                Ok(isbn) => Some(isbn),
                Err(error) => {
                    errors.push(FieldError::new("isbn", error.into()));
                    None
                }
            },
            None => None,
        };

        let cover_url = match clean(self.cover_url) {
            Some(raw) => {
                let url = parse_cover_url(&raw);
                if url.is_none() {
                    errors.push(FieldError::new("coverUrl", ErrorCode::InvalidCoverUrl));
                }
                url
            }
            None => None,
        };

        match title {
            Some(title) if errors.is_empty() => Ok(ValidEdition {
                title,
                subtitle: clean(self.subtitle),
                authors: dedupe_ignoring_case(
                    self.authors.into_iter().filter_map(|a| clean(Some(a))),
                ),
                publisher: clean(self.publisher),
                published_year,
                language: clean(self.language),
                isbn,
                cover_url,
                google_books_id: clean(self.google_books_id),
                open_library_id: clean(self.open_library_id),
            }),
            _ => Err(errors),
        }
    }
}

impl CopyInput {
    pub fn validate(self) -> Result<ValidCopy, Vec<FieldError>> {
        let mut errors = Vec::new();

        let quantity = self.quantity;
        if !(1..=MAX_QUANTITY).contains(&quantity) {
            errors.push(FieldError::new("quantity", ErrorCode::OutOfRange));
        }

        let acquired_on = match clean(self.acquired_on) {
            Some(raw) => match NaiveDate::parse_from_str(&raw, "%Y-%m-%d") {
                Ok(date) => Some(date),
                Err(_) => {
                    errors.push(FieldError::new("acquiredOn", ErrorCode::InvalidDate));
                    None
                }
            },
            None => None,
        };

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(ValidCopy {
            owner: clean(self.owner),
            format: self.format,
            quantity,
            location: clean(self.location),
            acquired_on,
        })
    }
}

impl NewBook {
    pub fn validate(self) -> Result<ValidBook, Vec<FieldError>> {
        let edition = self.edition.validate();
        let copy = self.copy.validate().map_err(|errors| {
            errors
                .into_iter()
                .map(|e| FieldError::new(format!("copy.{}", e.field), e.code))
                .collect::<Vec<_>>()
        });
        match (edition, copy) {
            (Ok(edition), Ok(copy)) => Ok(ValidBook { edition, copy }),
            (edition, copy) => {
                let mut errors = edition.err().unwrap_or_default();
                errors.extend(copy.err().unwrap_or_default());
                Err(errors)
            }
        }
    }
}

/// Trims whitespace and turns empty strings into `None`.
fn clean(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

/// Keeps the first spelling of each name.
fn dedupe_ignoring_case(names: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    let mut result = Vec::new();
    for name in names {
        let key = name.to_lowercase();
        if !seen.contains(&key) {
            seen.push(key);
            result.push(name);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal(title: &str) -> NewBook {
        NewBook {
            edition: EditionInput {
                title: title.to_owned(),
                ..EditionInput::default()
            },
            copy: CopyInput {
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
        book.edition.subtitle = Some("   ".to_owned());
        book.edition.authors = vec![" Patrick Rothfuss ".to_owned(), "".to_owned()];

        let valid = book.validate().unwrap();

        assert_eq!(valid.edition.title, "The Name of the Wind");
        assert_eq!(valid.edition.subtitle, None);
        assert_eq!(valid.edition.authors, vec!["Patrick Rothfuss"]);
    }

    #[test]
    fn repeated_authors_are_kept_once() {
        let mut book = minimal("T");
        book.edition.authors = vec!["Ursula K. Le Guin".into(), "ursula k. le guin".into()];
        assert_eq!(
            book.validate().unwrap().edition.authors,
            vec!["Ursula K. Le Guin"]
        );
    }

    #[test]
    fn normalizes_isbn_and_parses_date() {
        let mut book = minimal("Title");
        book.edition.isbn = Some("0-306-40615-2".to_owned());
        book.copy.acquired_on = Some("2026-09-24".to_owned());

        let valid = book.validate().unwrap();

        assert_eq!(valid.edition.isbn.unwrap().as_str(), "9780306406157");
        assert_eq!(valid.copy.acquired_on, NaiveDate::from_ymd_opt(2026, 9, 24));
    }

    #[test]
    fn reports_every_invalid_field() {
        let mut book = minimal(" ");
        book.edition.published_year = Some(0);
        book.edition.isbn = Some("9780306406158".to_owned());
        book.copy.quantity = 0;
        book.copy.acquired_on = Some("2026-02-30".to_owned());

        let errors = book.validate().unwrap_err();

        assert_eq!(
            errors,
            vec![
                FieldError::new("title", ErrorCode::Required),
                FieldError::new("publishedYear", ErrorCode::OutOfRange),
                FieldError::new("isbn", ErrorCode::IsbnChecksum),
                FieldError::new("copy.quantity", ErrorCode::OutOfRange),
                FieldError::new("copy.acquiredOn", ErrorCode::InvalidDate),
            ]
        );
    }

    #[test]
    fn copy_errors_have_no_prefix_when_validated_alone() {
        let copy = CopyInput {
            owner: None,
            format: Format::Ebook,
            quantity: 0,
            location: None,
            acquired_on: None,
        };
        assert_eq!(
            copy.validate().unwrap_err(),
            vec![FieldError::new("quantity", ErrorCode::OutOfRange)]
        );
    }

    #[test]
    fn new_book_reads_edition_fields_at_the_top_level() {
        let book: NewBook = serde_json::from_value(serde_json::json!({
            "title": "Dune",
            "authors": ["Frank Herbert"],
            "copy": { "format": "pocket" }
        }))
        .unwrap();
        assert_eq!(book.edition.title, "Dune");
        assert_eq!(book.copy.quantity, 1);
    }

    #[test]
    fn only_https_urls_on_cover_hosts_are_accepted() {
        for good in [
            "https://covers.openlibrary.org/b/id/15152634-L.jpg",
            "https://books.google.com/books/content?id=x&printsec=frontcover&img=1&zoom=1",
        ] {
            assert!(parse_cover_url(good).is_some(), "{good}");
        }
        for bad in [
            "http://covers.openlibrary.org/b/id/1-L.jpg",
            "https://evil.example/cover.jpg",
            "https://covers.openlibrary.org@evil.example/x.jpg",
            "https://covers.openlibrary.org:8443/x.jpg",
            "https://covers.openlibrary.org.evil.example/x.jpg",
            "file:///etc/passwd",
            "https://192.168.1.1/x.jpg",
        ] {
            assert!(parse_cover_url(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn untrusted_cover_url_is_a_field_error() {
        let mut book = minimal("T");
        book.edition.cover_url = Some("https://evil.example/c.jpg".into());
        assert_eq!(
            book.validate().unwrap_err(),
            vec![FieldError::new("coverUrl", ErrorCode::InvalidCoverUrl)]
        );
    }

    #[test]
    fn format_round_trips_through_its_database_name() {
        for format in Format::ALL {
            assert_eq!(format.as_str().parse(), Ok(format));
        }
    }
}
