use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

use crate::genre::{GENRES, genres_of};
use crate::isbn::{Isbn, IsbnError};
use crate::schema::Entity;

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

/// What a work is (D60). Each kind has its own details and formats in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Book,
    Comic,
    Boardgame,
    Film,
    TvSeries,
    Videogame,
    Music,
    Other,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Book,
        Kind::Comic,
        Kind::Boardgame,
        Kind::Film,
        Kind::TvSeries,
        Kind::Videogame,
        Kind::Music,
        Kind::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Book => "book",
            Kind::Comic => "comic",
            Kind::Boardgame => "boardgame",
            Kind::Film => "film",
            Kind::TvSeries => "tv_series",
            Kind::Videogame => "videogame",
            Kind::Music => "music",
            Kind::Other => "other",
        }
    }
}

impl FromStr for Kind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Kind::ALL
            .into_iter()
            .find(|kind| kind.as_str() == s)
            .ok_or(())
    }
}

/// What someone did for a work: writing, drawing, designing, directing, performing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributorRole {
    Author,
    Writer,
    Illustrator,
    Designer,
    Director,
    Artist,
    Studio,
}

impl ContributorRole {
    pub const ALL: [ContributorRole; 7] = [
        ContributorRole::Author,
        ContributorRole::Writer,
        ContributorRole::Illustrator,
        ContributorRole::Designer,
        ContributorRole::Director,
        ContributorRole::Artist,
        ContributorRole::Studio,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ContributorRole::Author => "author",
            ContributorRole::Writer => "writer",
            ContributorRole::Illustrator => "illustrator",
            ContributorRole::Designer => "designer",
            ContributorRole::Director => "director",
            ContributorRole::Artist => "artist",
            ContributorRole::Studio => "studio",
        }
    }
}

impl FromStr for ContributorRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ContributorRole::ALL
            .into_iter()
            .find(|role| role.as_str() == s)
            .ok_or(())
    }
}

/// A contributor as entered by the user, before validation.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContributorInput {
    pub name: String,
    pub role: ContributorRole,
}

/// A contributor that passed validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidContributor {
    pub name: String,
    pub role: ContributorRole,
}

/// The data specific to one kind, one per kind that has any (D08).
#[derive(Debug, Clone, PartialEq)]
pub enum KindDetails {
    /// Book (its data stays in isbn/google_books_id/open_library_id), music, other.
    None,
    Comic {
        number: Option<String>,
    },
    Boardgame {
        players_min: Option<i64>,
        players_max: Option<i64>,
        duration_minutes: Option<i64>,
        min_age: Option<i64>,
    },
    Film {
        duration_minutes: Option<i64>,
    },
    TvSeries {
        seasons: Option<i64>,
    },
}

impl KindDetails {
    /// The details table this data belongs to, if any.
    pub fn entity(&self) -> Option<Entity> {
        match self {
            KindDetails::None => None,
            KindDetails::Comic { .. } => Some(Entity::ComicDetails),
            KindDetails::Boardgame { .. } => Some(Entity::BoardgameDetails),
            KindDetails::Film { .. } => Some(Entity::FilmDetails),
            KindDetails::TvSeries { .. } => Some(Entity::TvSeriesDetails),
        }
    }

    /// The fields of the operation that creates or updates this details row.
    pub fn fields(&self) -> serde_json::Value {
        match self {
            KindDetails::None => json!({}),
            KindDetails::Comic { number } => json!({ "number": number }),
            KindDetails::Boardgame {
                players_min,
                players_max,
                duration_minutes,
                min_age,
            } => json!({
                "players_min": players_min,
                "players_max": players_max,
                "duration_minutes": duration_minutes,
                "min_age": min_age,
            }),
            KindDetails::Film { duration_minutes } => {
                json!({ "duration_minutes": duration_minutes })
            }
            KindDetails::TvSeries { seasons } => json!({ "seasons": seasons }),
        }
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
    #[serde(default)]
    pub kind: Kind,
    /// Contributors in any role other than the book's authors (`authors` means `author`).
    #[serde(default)]
    pub contributors: Vec<ContributorInput>,
    /// EAN/UPC, for what has no ISBN.
    #[serde(default)]
    pub barcode: Option<String>,
    #[serde(default)]
    pub number: Option<String>,
    #[serde(default)]
    pub players_min: Option<i64>,
    #[serde(default)]
    pub players_max: Option<i64>,
    #[serde(default)]
    pub duration_minutes: Option<i64>,
    #[serde(default)]
    pub min_age: Option<i64>,
    #[serde(default)]
    pub seasons: Option<i64>,
    #[serde(default)]
    pub genres: Vec<String>,
    /// Tag names; found or created case-insensitively.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Free text, common to all kinds ("Complete, with its original box.").
    #[serde(default)]
    pub notes: Option<String>,
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
    /// A video game copy's platform: belongs to the copy, not the edition or work.
    #[serde(default)]
    pub platform: Option<String>,
    /// A TV series copy's held seasons, as free text ("1-2", "1-3, 5").
    #[serde(default)]
    pub seasons: Option<String>,
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
    pub kind: Kind,
    /// Contributors other than the authors, deduplicated case-insensitively per role.
    pub contributors: Vec<ValidContributor>,
    pub barcode: Option<String>,
    pub details: KindDetails,
    /// Deduplicated, dropped if unknown to `genre::GENRES` or not offered by `kind`.
    pub genres: Vec<String>,
    /// Tag names: trimmed, deduplicated case-insensitively, at most 40 characters each.
    pub tags: Vec<String>,
    /// Trimmed, inner newlines kept, at most `MAX_NOTES_LENGTH` characters.
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidCopy {
    pub owner: Option<String>,
    pub format: Format,
    pub quantity: u32,
    pub location: Option<String>,
    pub acquired_on: Option<NaiveDate>,
    pub platform: Option<String>,
    pub seasons: Option<String>,
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
    /// Not 8, 12, 13 or 14 digits with a valid GS1 check digit.
    InvalidBarcode,
    /// Another edition already has this barcode.
    BarcodeInUse,
    /// Not a key of `genre::GENRES`.
    InvalidGenre,
    /// A work cannot require or relate to itself.
    SelfReference,
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
    "inventaire.io",
    // Redirects (307) to an archive.org host; the http client's redirect
    // policy already follows https redirects, so only this first host needs
    // to be trusted (see metadata::musicbrainz).
    "coverartarchive.org",
    // Special:FilePath redirects (302, then 301) to thumb.wikimedia.org; same
    // reasoning as coverartarchive.org above (see metadata::wikidata).
    "commons.wikimedia.org",
    // BoardGameGeek's own image CDN, returned by GameUPC (see metadata::gameupc).
    "cf.geekdo-images.com",
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
pub const MAX_TAG_LENGTH: usize = 40;
pub const MAX_NOTES_LENGTH: usize = 2000;
/// Longest a copy's `platform` or `seasons` free text may be.
pub const MAX_COPY_TEXT_LENGTH: usize = 60;

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

        // ISBN and the external ids only belong to books; other kinds drop them silently.
        let kind = self.kind;
        let (isbn_raw, google_books_id_raw, open_library_id_raw) = if kind == Kind::Book {
            (self.isbn, self.google_books_id, self.open_library_id)
        } else {
            (None, None, None)
        };

        let isbn = match clean(isbn_raw) {
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

        let barcode = match clean(self.barcode) {
            Some(raw) => match parse_barcode(&raw) {
                Some(barcode) => Some(barcode),
                None => {
                    errors.push(FieldError::new("barcode", ErrorCode::InvalidBarcode));
                    None
                }
            },
            None => None,
        };

        let details = match kind {
            Kind::Book | Kind::Music | Kind::Other | Kind::Videogame => KindDetails::None,
            Kind::Comic => KindDetails::Comic {
                number: clean(self.number),
            },
            Kind::Boardgame => {
                let players_min = self.players_min;
                let players_max = self.players_max;
                if players_min.is_some_and(|v| !(1..=99).contains(&v)) {
                    errors.push(FieldError::new("playersMin", ErrorCode::OutOfRange));
                }
                if players_max.is_some_and(|v| !(1..=99).contains(&v)) {
                    errors.push(FieldError::new("playersMax", ErrorCode::OutOfRange));
                }
                if let (Some(min), Some(max)) = (players_min, players_max)
                    && min > max
                {
                    errors.push(FieldError::new("playersMax", ErrorCode::OutOfRange));
                }
                let duration_minutes = self.duration_minutes;
                if duration_minutes.is_some_and(|v| !(1..=10000).contains(&v)) {
                    errors.push(FieldError::new("durationMinutes", ErrorCode::OutOfRange));
                }
                let min_age = self.min_age;
                if min_age.is_some_and(|v| !(0..=99).contains(&v)) {
                    errors.push(FieldError::new("minAge", ErrorCode::OutOfRange));
                }
                KindDetails::Boardgame {
                    players_min,
                    players_max,
                    duration_minutes,
                    min_age,
                }
            }
            Kind::Film => {
                let duration_minutes = self.duration_minutes;
                if duration_minutes.is_some_and(|v| !(1..=10000).contains(&v)) {
                    errors.push(FieldError::new("durationMinutes", ErrorCode::OutOfRange));
                }
                KindDetails::Film { duration_minutes }
            }
            Kind::TvSeries => {
                let seasons = self.seasons;
                if seasons.is_some_and(|v| !(1..=999).contains(&v)) {
                    errors.push(FieldError::new("seasons", ErrorCode::OutOfRange));
                }
                KindDetails::TvSeries { seasons }
            }
        };

        // Contributors other than authors, grouped and deduplicated per role, in
        // `ContributorRole::ALL` order so a form and `BookDetail` agree on it.
        let mut by_role: Vec<(ContributorRole, Vec<String>)> = ContributorRole::ALL
            .into_iter()
            .filter(|role| *role != ContributorRole::Author)
            .map(|role| (role, Vec::new()))
            .collect();
        // Authors sent through `contributors` (tomorrow's UI) join `authors` (today's),
        // rather than being dropped: the same person, just two paths to the same field.
        let mut extra_authors: Vec<String> = Vec::new();
        for contributor in self.contributors {
            if contributor.role == ContributorRole::Author {
                extra_authors.push(contributor.name);
                continue;
            }
            if let (Some(name), Some(entry)) = (
                clean(Some(contributor.name)),
                by_role
                    .iter_mut()
                    .find(|(role, _)| *role == contributor.role),
            ) {
                entry.1.push(name);
            }
        }
        let contributors: Vec<ValidContributor> = by_role
            .into_iter()
            .flat_map(|(role, names)| {
                dedupe_ignoring_case(names.into_iter())
                    .into_iter()
                    .map(move |name| ValidContributor { name, role })
            })
            .collect();

        // Unknown genres are an error; genres of another kind are just dropped, since
        // the form may still hold them after the user changed the kind.
        let mut genres: Vec<String> = Vec::new();
        for raw in self.genres {
            let Some(genre) = clean(Some(raw)) else {
                continue;
            };
            if !GENRES.contains(&genre.as_str()) {
                errors.push(FieldError::new("genres", ErrorCode::InvalidGenre));
                continue;
            }
            genres.push(genre);
        }
        let allowed = genres_of(kind.as_str());
        genres.retain(|genre| allowed.contains(&genre.as_str()));
        let genres = dedupe_ignoring_case(genres.into_iter());

        let mut tags: Vec<String> = Vec::new();
        for raw in self.tags {
            let Some(tag) = clean(Some(raw)) else {
                continue;
            };
            if tag.chars().count() > MAX_TAG_LENGTH {
                errors.push(FieldError::new("tags", ErrorCode::OutOfRange));
                continue;
            }
            tags.push(tag);
        }
        let tags = dedupe_ignoring_case(tags.into_iter());

        let notes = clean(self.notes);
        if notes
            .as_ref()
            .is_some_and(|n| n.chars().count() > MAX_NOTES_LENGTH)
        {
            errors.push(FieldError::new("notes", ErrorCode::OutOfRange));
        }

        match title {
            Some(title) if errors.is_empty() => Ok(ValidEdition {
                title,
                subtitle: clean(self.subtitle),
                authors: dedupe_ignoring_case(
                    self.authors
                        .into_iter()
                        .chain(extra_authors)
                        .filter_map(|a| clean(Some(a))),
                ),
                publisher: clean(self.publisher),
                published_year,
                language: clean(self.language),
                isbn,
                cover_url,
                google_books_id: clean(google_books_id_raw),
                open_library_id: clean(open_library_id_raw),
                kind,
                contributors,
                barcode,
                details,
                genres,
                tags,
                notes,
            }),
            _ => Err(errors),
        }
    }
}

/// Trims separators and checks length and GS1 check digit (mod 10, weights 3 and 1
/// from the right, excluding the check digit itself). `None` when invalid.
fn parse_barcode(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| !matches!(c, '-' | ' ')).collect();
    if !matches!(cleaned.len(), 8 | 12 | 13 | 14) {
        return None;
    }
    let digits: Vec<u32> = cleaned
        .chars()
        .map(|c| c.to_digit(10))
        .collect::<Option<_>>()?;
    let (body, check) = digits.split_at(digits.len() - 1);
    let sum: u32 = body
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| if i % 2 == 0 { d * 3 } else { *d })
        .sum();
    let expected = (10 - sum % 10) % 10;
    (expected == check[0]).then_some(cleaned)
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

        // The kind is not checked here (a copy is validated alone, like `format`);
        // the UI only shows each field for the work's kind.
        let platform = clean(self.platform);
        if platform
            .as_ref()
            .is_some_and(|p| p.chars().count() > MAX_COPY_TEXT_LENGTH)
        {
            errors.push(FieldError::new("platform", ErrorCode::OutOfRange));
        }
        let seasons = clean(self.seasons);
        if seasons
            .as_ref()
            .is_some_and(|s| s.chars().count() > MAX_COPY_TEXT_LENGTH)
        {
            errors.push(FieldError::new("seasons", ErrorCode::OutOfRange));
        }

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(ValidCopy {
            owner: clean(self.owner),
            format: self.format,
            quantity,
            location: clean(self.location),
            acquired_on,
            platform,
            seasons,
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
                platform: None,
                seasons: None,
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
            platform: None,
            seasons: None,
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
            "https://inventaire.io/img/entities/27186041c3516dd13e117bd7b9a750eef5111a57",
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

    #[test]
    fn kind_round_trips_through_its_database_name() {
        for kind in Kind::ALL {
            assert_eq!(kind.as_str().parse(), Ok(kind));
        }
    }

    #[test]
    fn contributor_role_round_trips_through_its_database_name() {
        for role in ContributorRole::ALL {
            assert_eq!(role.as_str().parse(), Ok(role));
        }
    }

    #[test]
    fn authors_sent_as_contributors_join_the_author_list() {
        let mut book = minimal("A");
        book.edition.authors = vec!["A".into()];
        book.edition.contributors = vec![
            ContributorInput {
                name: "B".into(),
                role: ContributorRole::Author,
            },
            ContributorInput {
                name: "a".into(),
                role: ContributorRole::Author,
            },
            ContributorInput {
                name: "C".into(),
                role: ContributorRole::Writer,
            },
        ];

        let valid = book.validate().unwrap().edition;

        assert_eq!(valid.authors, vec!["A", "B"]);
        assert_eq!(
            valid.contributors,
            vec![ValidContributor {
                name: "C".into(),
                role: ContributorRole::Writer,
            }]
        );
    }

    #[test]
    fn a_boardgame_keeps_its_own_details() {
        let mut book = minimal("Wingspan");
        book.edition.kind = Kind::Boardgame;
        book.edition.players_min = Some(1);
        book.edition.players_max = Some(5);
        book.edition.duration_minutes = Some(70);
        book.edition.min_age = Some(10);
        book.edition.contributors = vec![ContributorInput {
            name: "Elizabeth Hargrave".into(),
            role: ContributorRole::Designer,
        }];

        let valid = book.validate().unwrap().edition;

        assert_eq!(valid.kind, Kind::Boardgame);
        assert_eq!(
            valid.details,
            KindDetails::Boardgame {
                players_min: Some(1),
                players_max: Some(5),
                duration_minutes: Some(70),
                min_age: Some(10),
            }
        );
        assert_eq!(
            valid.contributors,
            vec![ValidContributor {
                name: "Elizabeth Hargrave".into(),
                role: ContributorRole::Designer,
            }]
        );
    }

    #[test]
    fn details_of_another_kind_are_dropped() {
        let mut book = minimal("Wingspan");
        book.edition.kind = Kind::Boardgame;
        book.edition.seasons = Some(3);

        let valid = book.validate().unwrap().edition;

        assert_eq!(
            valid.details,
            KindDetails::Boardgame {
                players_min: None,
                players_max: None,
                duration_minutes: None,
                min_age: None,
            }
        );
    }

    #[test]
    fn isbn_is_dropped_for_a_film() {
        let mut book = minimal("Dune");
        book.edition.kind = Kind::Film;
        book.edition.isbn = Some("978-0-306-40615-7".into());
        book.edition.google_books_id = Some("abc".into());

        let valid = book.validate().unwrap().edition;

        assert_eq!(valid.isbn, None);
        assert_eq!(valid.google_books_id, None);
    }

    #[test]
    fn a_valid_and_an_invalid_barcode() {
        let mut book = minimal("A");
        book.edition.barcode = Some("4006381333931".into());
        assert_eq!(
            book.validate().unwrap().edition.barcode.as_deref(),
            Some("4006381333931")
        );

        let mut book = minimal("A");
        book.edition.barcode = Some("4006381333932".into());
        assert_eq!(
            book.validate().unwrap_err(),
            vec![FieldError::new("barcode", ErrorCode::InvalidBarcode)]
        );
    }

    #[test]
    fn notes_are_trimmed_kept_empty_or_too_long() {
        let mut book = minimal("A");
        book.edition.notes = Some("  Complete, with its original box.\nMissing a candle.  ".into());
        assert_eq!(
            book.validate().unwrap().edition.notes.as_deref(),
            Some("Complete, with its original box.\nMissing a candle.")
        );

        let mut book = minimal("A");
        book.edition.notes = Some("   ".into());
        assert_eq!(book.validate().unwrap().edition.notes, None);

        let mut book = minimal("A");
        book.edition.notes = Some("x".repeat(MAX_NOTES_LENGTH + 1));
        assert_eq!(
            book.validate().unwrap_err(),
            vec![FieldError::new("notes", ErrorCode::OutOfRange)]
        );
    }

    #[test]
    fn players_min_over_max_is_out_of_range() {
        let mut book = minimal("A");
        book.edition.kind = Kind::Boardgame;
        book.edition.players_min = Some(5);
        book.edition.players_max = Some(2);

        assert_eq!(
            book.validate().unwrap_err(),
            vec![FieldError::new("playersMax", ErrorCode::OutOfRange)]
        );
    }

    #[test]
    fn unknown_genre_is_rejected() {
        let mut book = minimal("A");
        book.edition.genres = vec!["not_a_genre".into()];
        assert_eq!(
            book.validate().unwrap_err(),
            vec![FieldError::new("genres", ErrorCode::InvalidGenre)]
        );
    }

    #[test]
    fn a_genre_of_another_kind_is_dropped_and_duplicates_removed() {
        let mut book = minimal("A");
        book.edition.kind = Kind::Boardgame;
        // "manga" belongs to comic, not boardgame; "strategy" is repeated.
        book.edition.genres = vec!["manga".into(), "strategy".into(), "strategy".into()];

        let valid = book.validate().unwrap().edition;

        assert_eq!(valid.genres, vec!["strategy"]);
    }

    #[test]
    fn tags_are_trimmed_deduplicated_and_length_checked() {
        let mut book = minimal("A");
        book.edition.tags = vec![
            "  Fantasía  ".into(),
            "fantasía".into(),
            "".into(),
            "x".repeat(41),
        ];

        let errors = book.validate().unwrap_err();

        assert_eq!(errors, vec![FieldError::new("tags", ErrorCode::OutOfRange)]);

        let mut book = minimal("A");
        book.edition.tags = vec!["  Fantasía  ".into(), "fantasía".into(), "".into()];
        assert_eq!(book.validate().unwrap().edition.tags, vec!["Fantasía"]);
    }

    #[test]
    fn copy_platform_and_seasons_are_trimmed_and_length_checked() {
        let mut copy = minimal("A").copy;
        copy.platform = Some("  Nintendo Switch  ".into());
        copy.seasons = Some("  1-2  ".into());
        let valid = copy.validate().unwrap();
        assert_eq!(valid.platform.as_deref(), Some("Nintendo Switch"));
        assert_eq!(valid.seasons.as_deref(), Some("1-2"));

        let mut copy = minimal("A").copy;
        copy.platform = Some("".into());
        copy.seasons = Some("   ".into());
        let valid = copy.validate().unwrap();
        assert_eq!(valid.platform, None);
        assert_eq!(valid.seasons, None);

        let mut copy = minimal("A").copy;
        copy.platform = Some("x".repeat(61));
        copy.seasons = Some("x".repeat(61));
        assert_eq!(
            copy.validate().unwrap_err(),
            vec![
                FieldError::new("platform", ErrorCode::OutOfRange),
                FieldError::new("seasons", ErrorCode::OutOfRange),
            ]
        );
    }

    #[test]
    fn empty_contributor_names_are_dropped_and_duplicates_removed_per_role() {
        let mut book = minimal("A");
        book.edition.kind = Kind::Boardgame;
        book.edition.contributors = vec![
            ContributorInput {
                name: "  ".into(),
                role: ContributorRole::Designer,
            },
            ContributorInput {
                name: "Ana".into(),
                role: ContributorRole::Designer,
            },
            ContributorInput {
                name: "ana".into(),
                role: ContributorRole::Designer,
            },
            ContributorInput {
                name: "Bea".into(),
                role: ContributorRole::Illustrator,
            },
        ];

        let valid = book.validate().unwrap().edition;

        assert_eq!(
            valid.contributors,
            vec![
                ValidContributor {
                    name: "Bea".into(),
                    role: ContributorRole::Illustrator,
                },
                ValidContributor {
                    name: "Ana".into(),
                    role: ContributorRole::Designer,
                },
            ]
        );
    }
}
