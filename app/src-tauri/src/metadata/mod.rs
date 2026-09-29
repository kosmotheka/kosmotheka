//! Book data from external catalogues, looked up by ISBN (D26). Every provider is
//! optional: failures become warnings and never block the user (D18).

pub mod candidate;
pub mod google;
pub mod inventaire;
pub mod musicbrainz;
pub mod openlibrary;

use std::time::Duration;

use kosmotheka_domain::{Isbn, Kind};
use serde::Serialize;

pub use candidate::{Candidate, CandidateContributor};

/// What one provider knows about an edition.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub cover_url: Option<String>,
    /// The edition's id in that provider's catalogue.
    pub external_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    GoogleBooks,
    OpenLibrary,
    Inventaire,
    MusicBrainz,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error("quota exceeded")]
    Quota,
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningKind {
    Network,
    Quota,
    Unexpected,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWarning {
    pub source: Source,
    pub kind: WarningKind,
}

/// Merged answer for the UI, which uses it to fill the form.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupResult {
    pub isbn13: String,
    pub found: bool,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub publisher: Option<String>,
    pub published_year: Option<i32>,
    pub language: Option<String>,
    pub cover_url: Option<String>,
    pub google_books_id: Option<String>,
    pub open_library_id: Option<String>,
    pub sources: Vec<Source>,
    pub warnings: Vec<ProviderWarning>,
    /// Set when this ISBN is already in the catalog.
    pub existing_edition_id: Option<String>,
}

/// Candidates a person can pick from, and warnings from providers that failed.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub candidates: Vec<Candidate>,
    pub warnings: Vec<ProviderWarning>,
}

/// Turns one provider's outcome into a warning, kept for `merge` and `collect_candidates`.
fn warning_kind(error: &ProviderError) -> WarningKind {
    match error {
        ProviderError::Network(_) => WarningKind::Network,
        ProviderError::Quota => WarningKind::Quota,
        ProviderError::Unexpected(_) => WarningKind::Unexpected,
    }
}

/// Candidates keep provider order: the first provider's candidates come first.
pub fn collect_candidates(
    results: Vec<(Source, Result<Vec<Candidate>, ProviderError>)>,
) -> SearchResult {
    let mut result = SearchResult::default();
    for (source, outcome) in results {
        match outcome {
            Ok(candidates) => result.candidates.extend(candidates),
            Err(error) => result.warnings.push(ProviderWarning {
                source,
                kind: warning_kind(&error),
            }),
        }
    }
    result
}

/// Answer to looking up a scanned or typed code, which may be an ISBN or another
/// kind of barcode (D26). Parts 2 and 3 add providers for non-book codes.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeResult {
    pub kind: Option<Kind>,
    pub candidates: Vec<Candidate>,
    pub warnings: Vec<ProviderWarning>,
    /// Set when this code is already in the catalog.
    pub existing_edition_id: Option<String>,
}

/// Turns a merged ISBN lookup into the one candidate it represents.
fn candidate_from_lookup(result: &LookupResult) -> Candidate {
    let source = result
        .sources
        .first()
        .copied()
        .unwrap_or(Source::OpenLibrary);
    let mut candidate =
        Candidate::new(Kind::Book, source, result.title.clone().unwrap_or_default());
    candidate.subtitle = result.subtitle.clone();
    candidate.authors = result.authors.clone();
    candidate.publisher = result.publisher.clone();
    candidate.published_year = result.published_year;
    candidate.language = result.language.clone();
    candidate.cover_url = result.cover_url.clone();
    candidate.isbn = Some(result.isbn13.clone());
    candidate.external_id = match source {
        Source::GoogleBooks => result.google_books_id.clone(),
        Source::OpenLibrary => result.open_library_id.clone(),
        Source::Inventaire | Source::MusicBrainz => None,
    };
    candidate
}

type ProviderResult = Result<Option<Metadata>, ProviderError>;

/// Shared HTTP client, kept in Tauri state so connections are reused.
pub struct Lookup {
    client: reqwest::Client,
}

impl Lookup {
    pub fn new() -> reqwest::Result<Self> {
        Ok(Self {
            client: http_client()?,
        })
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Asks every provider at the same time and merges the answers.
    pub async fn run(&self, isbn: &Isbn, google_api_key: Option<&str>) -> LookupResult {
        let (google, open_library) = tokio::join!(
            google::lookup(&self.client, isbn, google_api_key),
            openlibrary::lookup(&self.client, isbn),
        );
        merge(isbn, google, open_library)
    }

    /// Title search: books, and now music (part 3 adds board games).
    pub async fn search(&self, kind: Kind, query: &str) -> SearchResult {
        let query = query.trim();
        if query.chars().count() < 2 {
            return SearchResult::default();
        }
        match kind {
            Kind::Book => {
                let (open_library, inventaire) = tokio::join!(
                    openlibrary::search(&self.client, query),
                    inventaire::search(&self.client, query),
                );
                collect_candidates(vec![
                    (Source::OpenLibrary, open_library),
                    (Source::Inventaire, inventaire),
                ])
            }
            Kind::Music => {
                let musicbrainz = musicbrainz::search(&self.client, query).await;
                collect_candidates(vec![(Source::MusicBrainz, musicbrainz)])
            }
            _ => SearchResult::default(),
        }
    }

    /// A scanned or typed code: an ISBN is a book, anything else is tried
    /// against the non-ISBN providers (part 3 adds board games alongside music).
    pub async fn lookup_code(&self, code: &str, google_api_key: Option<&str>) -> CodeResult {
        match Isbn::parse(code) {
            Ok(isbn) => self.lookup_isbn_code(&isbn, google_api_key).await,
            Err(_) => self.lookup_non_isbn_code(code).await,
        }
    }

    async fn lookup_isbn_code(&self, isbn: &Isbn, google_api_key: Option<&str>) -> CodeResult {
        let (merged, inventaire) = tokio::join!(
            self.run(isbn, google_api_key),
            inventaire::lookup(&self.client, isbn)
        );

        let mut result = CodeResult {
            warnings: merged.warnings.clone(),
            ..CodeResult::default()
        };
        if merged.found {
            result.candidates.push(candidate_from_lookup(&merged));
        }
        match inventaire {
            Ok(Some(candidate)) => result.candidates.push(candidate),
            Ok(None) => {}
            Err(error) => result.warnings.push(ProviderWarning {
                source: Source::Inventaire,
                kind: warning_kind(&error),
            }),
        }
        result.kind = (!result.candidates.is_empty()).then_some(Kind::Book);
        result
    }

    /// A non-ISBN code so far only goes to MusicBrainz; GameUPC (board games)
    /// needs an API key and is deferred (see the brief for part 2). Once a
    /// keyless board-game provider lands, wire it in here alongside MusicBrainz
    /// and pass its outcome to `decide_kind`.
    async fn lookup_non_isbn_code(&self, code: &str) -> CodeResult {
        let mut result = CodeResult::default();
        match musicbrainz::lookup_barcode(&self.client, code).await {
            Ok(candidates) => {
                result.kind = decide_kind(!candidates.is_empty(), false);
                result.candidates = candidates;
            }
            Err(error) => result.warnings.push(ProviderWarning {
                source: Source::MusicBrainz,
                kind: warning_kind(&error),
            }),
        }
        result
    }
}

/// The ambiguity rule for a non-ISBN code: `Kind::Music` when only MusicBrainz
/// found something, `Kind::Boardgame` when only the board-game provider did,
/// `None` when both or neither did (the UI then asks "¿Qué es?"). Pure so it
/// can be tested without the network.
fn decide_kind(music_found: bool, boardgame_found: bool) -> Option<Kind> {
    match (music_found, boardgame_found) {
        (true, false) => Some(Kind::Music),
        (false, true) => Some(Kind::Boardgame),
        _ => None,
    }
}

pub fn http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!(
            "Kosmotheka/",
            env!("CARGO_PKG_VERSION"),
            " (+https://kosmotheka.app)"
        ))
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.url().scheme() == "https" && attempt.previous().len() < 5 {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
}

/// Field by field, Google Books first (better Spanish coverage), then Open Library.
/// The cover prefers Open Library, whose large images beat Google's thumbnails.
pub fn merge(isbn: &Isbn, google: ProviderResult, open_library: ProviderResult) -> LookupResult {
    let mut result = LookupResult {
        isbn13: isbn.as_str().to_owned(),
        ..LookupResult::default()
    };

    let mut found = |source, outcome: ProviderResult| match outcome {
        Ok(Some(metadata)) => {
            result.sources.push(source);
            Some(metadata)
        }
        Ok(None) => None,
        Err(error) => {
            result.warnings.push(ProviderWarning {
                source,
                kind: warning_kind(&error),
            });
            None
        }
    };
    let google = found(Source::GoogleBooks, google).unwrap_or_default();
    let open_library = found(Source::OpenLibrary, open_library).unwrap_or_default();

    result.found = !result.sources.is_empty();
    result.title = google.title.or(open_library.title);
    result.subtitle = google.subtitle.or(open_library.subtitle);
    result.authors = if google.authors.is_empty() {
        open_library.authors
    } else {
        google.authors
    };
    result.publisher = google.publisher.or(open_library.publisher);
    result.published_year = google.published_year.or(open_library.published_year);
    result.language = google.language.or(open_library.language);
    result.cover_url = open_library.cover_url.or(google.cover_url);
    result.google_books_id = google.external_id;
    result.open_library_id = open_library.external_id;
    result
}

/// First run of four digits, e.g. `2007` from "2007-03-27" or "October 1, 1988".
pub fn year_in(text: &str) -> Option<i32> {
    text.as_bytes()
        .windows(4)
        .enumerate()
        .find(|(i, window)| {
            window.iter().all(u8::is_ascii_digit)
                && !text.as_bytes().get(i + 4).is_some_and(u8::is_ascii_digit)
                && (*i == 0 || !text.as_bytes()[i - 1].is_ascii_digit())
        })
        .and_then(|(_, window)| std::str::from_utf8(window).ok()?.parse().ok())
}

/// Trims and drops empty strings from provider data.
fn text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn isbn() -> Isbn {
        Isbn::parse("9780140328721").unwrap()
    }

    #[test]
    fn year_is_taken_from_several_date_formats() {
        assert_eq!(year_in("2007-03-27"), Some(2007));
        assert_eq!(year_in("October 1, 1988"), Some(1988));
        assert_eq!(year_in("1999"), Some(1999));
        assert_eq!(year_in("c. 12345"), None);
        assert_eq!(year_in(""), None);
    }

    #[test]
    fn merge_prefers_google_text_and_open_library_cover() {
        let google = Metadata {
            title: Some("Fantastic Mr. Fox".into()),
            authors: vec!["Roald Dahl".into()],
            cover_url: Some("https://books.google.com/thumb".into()),
            external_id: Some("g1".into()),
            ..Metadata::default()
        };
        let open_library = Metadata {
            title: Some("Fantastic Mr Fox".into()),
            publisher: Some("Puffin".into()),
            cover_url: Some("https://covers.openlibrary.org/b/id/1-L.jpg".into()),
            external_id: Some("OL1M".into()),
            ..Metadata::default()
        };

        let result = merge(&isbn(), Ok(Some(google)), Ok(Some(open_library)));

        assert!(result.found);
        assert_eq!(result.title.as_deref(), Some("Fantastic Mr. Fox"));
        assert_eq!(
            result.publisher.as_deref(),
            Some("Puffin"),
            "gaps are filled from Open Library"
        );
        assert_eq!(
            result.cover_url.as_deref(),
            Some("https://covers.openlibrary.org/b/id/1-L.jpg")
        );
        assert_eq!(result.google_books_id.as_deref(), Some("g1"));
        assert_eq!(result.open_library_id.as_deref(), Some("OL1M"));
        assert_eq!(
            result.sources,
            vec![Source::GoogleBooks, Source::OpenLibrary]
        );
    }

    #[test]
    fn a_failing_provider_becomes_a_warning() {
        let open_library = Metadata {
            title: Some("T".into()),
            ..Metadata::default()
        };

        let result = merge(&isbn(), Err(ProviderError::Quota), Ok(Some(open_library)));

        assert!(result.found);
        assert_eq!(result.title.as_deref(), Some("T"));
        assert_eq!(
            result.warnings,
            vec![ProviderWarning {
                source: Source::GoogleBooks,
                kind: WarningKind::Quota
            }]
        );
    }

    #[test]
    fn nothing_found_anywhere() {
        let result = merge(&isbn(), Ok(None), Ok(None));
        assert!(!result.found);
        assert!(result.warnings.is_empty());
        assert_eq!(result.isbn13, "9780140328721");
    }

    #[test]
    fn kind_is_music_when_only_musicbrainz_found_something() {
        assert_eq!(decide_kind(true, false), Some(Kind::Music));
    }

    #[test]
    fn kind_is_boardgame_when_only_the_boardgame_provider_found_something() {
        assert_eq!(decide_kind(false, true), Some(Kind::Boardgame));
    }

    #[test]
    fn kind_is_ambiguous_when_both_or_neither_found_something() {
        assert_eq!(decide_kind(true, true), None, "let the UI ask");
        assert_eq!(decide_kind(false, false), None);
    }
}

/// Hit the real services. Not run by default (CI must not depend on the network):
/// `cargo test -p kosmotheka -- --ignored --nocapture live_`
#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_lookup() {
        let lookup = Lookup::new().unwrap();
        let isbn = Isbn::parse("9780140328721").unwrap();

        let result = lookup.run(&isbn, None).await;

        println!("{result:#?}");
        assert!(result.sources.contains(&Source::OpenLibrary));
        assert_eq!(result.authors, vec!["Roald Dahl"]);
        assert!(result.cover_url.is_some());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_cover_download() {
        let dir = std::env::temp_dir().join(format!("library-live-covers-{}", std::process::id()));
        let covers = crate::covers::Covers::new(dir).unwrap();
        let url = kosmotheka_domain::parse_cover_url(
            "https://covers.openlibrary.org/b/id/15152634-L.jpg",
        )
        .unwrap();

        let name = covers
            .download(&http_client().unwrap(), &url)
            .await
            .unwrap();

        println!("stored as {name}");
        assert!(name.ends_with(".jpg"));
        assert!(covers.dir().join(&name).metadata().unwrap().len() > 10_000);
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_open_library() {
        let client = http_client().unwrap();
        let candidates = openlibrary::search(&client, "dune").await.unwrap();
        println!("open library: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_inventaire() {
        let client = http_client().unwrap();
        let candidates = inventaire::search(&client, "dune").await.unwrap();
        println!("inventaire: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_inventaire_isbn_lookup() {
        let client = http_client().unwrap();
        let isbn = Isbn::parse("9788445000663").unwrap();
        let candidate = inventaire::lookup(&client, &isbn).await.unwrap();
        println!("inventaire lookup: {candidate:#?}");
        assert!(candidate.is_some());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_musicbrainz() {
        let client = http_client().unwrap();
        let candidates = musicbrainz::search(&client, "Random Access Memories Daft Punk")
            .await
            .unwrap();
        println!("musicbrainz: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_musicbrainz_barcode_lookup() {
        let client = http_client().unwrap();
        let candidates = musicbrainz::lookup_barcode(&client, "196588010323")
            .await
            .unwrap();
        println!("musicbrainz barcode: {candidates:#?}");
        assert!(!candidates.is_empty());
    }
}
