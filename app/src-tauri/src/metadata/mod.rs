//! Book data from external catalogues, looked up by ISBN (D26). Every provider is
//! optional: failures become warnings and never block the user (D18).

pub mod bne;
pub mod candidate;
pub mod gameupc;
pub mod google;
pub mod inventaire;
pub mod musicbrainz;
pub mod openlibrary;
pub mod via_server;
pub mod wikidata;

use std::collections::BTreeSet;
use std::future::Future;
use std::time::Duration;

use kosmotheka_domain::{Isbn, Kind};

use crate::cloud::CloudApi;
use serde::Serialize;
use via_server::{GoogleAccess, lookup_google};

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
    Bne,
    GoogleBooks,
    OpenLibrary,
    Inventaire,
    MusicBrainz,
    Wikidata,
    GameUpc,
}

impl Source {
    /// Stable id used for `Settings::disabled_sources`, matching the snake_case
    /// serialization above.
    pub fn id(self) -> &'static str {
        match self {
            Source::Bne => "bne",
            Source::GoogleBooks => "google_books",
            Source::OpenLibrary => "open_library",
            Source::Inventaire => "inventaire",
            Source::MusicBrainz => "music_brainz",
            Source::Wikidata => "wikidata",
            Source::GameUpc => "game_upc",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error(transparent)]
    Network(#[from] reqwest::Error),
    #[error("quota exceeded")]
    Quota,
    /// The stored key was rejected (HTTP 401/403): the UI says so instead of a generic warning.
    #[error("api key rejected")]
    BadKey,
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningKind {
    Network,
    Quota,
    BadKey,
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
        ProviderError::BadKey => WarningKind::BadKey,
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
        Source::Bne
        | Source::Inventaire
        | Source::MusicBrainz
        | Source::Wikidata
        | Source::GameUpc => None,
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
    pub async fn run<C: CloudApi>(
        &self,
        isbn: &Isbn,
        google: GoogleAccess<'_, C>,
        disabled: &BTreeSet<String>,
    ) -> LookupResult {
        let (bne, google, open_library) = tokio::join!(
            maybe(
                disabled.contains(Source::Bne.id()),
                bne::lookup(&self.client, isbn),
            ),
            maybe(
                disabled.contains(Source::GoogleBooks.id()),
                lookup_google(google, isbn, |key| async move {
                    google::lookup(&self.client, isbn, key.as_deref()).await
                }),
            ),
            maybe(
                disabled.contains(Source::OpenLibrary.id()),
                openlibrary::lookup(&self.client, isbn),
            ),
        );
        merge(isbn, bne, google, open_library)
    }

    /// Title search: books, music, and now everything Wikidata covers (films,
    /// TV series, video games, comics, board games).
    pub async fn search(
        &self,
        kind: Kind,
        query: &str,
        disabled: &BTreeSet<String>,
    ) -> SearchResult {
        let query = query.trim();
        if query.chars().count() < 2 {
            return SearchResult::default();
        }
        match kind {
            Kind::Book => {
                let (bne, open_library, inventaire, wikidata) = tokio::join!(
                    maybe_vec(
                        disabled.contains(Source::Bne.id()),
                        bne::search(&self.client, query),
                    ),
                    maybe_vec(
                        disabled.contains(Source::OpenLibrary.id()),
                        openlibrary::search(&self.client, query),
                    ),
                    maybe_vec(
                        disabled.contains(Source::Inventaire.id()),
                        inventaire::search(&self.client, query),
                    ),
                    maybe_vec(
                        disabled.contains(Source::Wikidata.id()),
                        wikidata::search(&self.client, kind, query),
                    ),
                );
                collect_candidates(vec![
                    (Source::Bne, bne),
                    (Source::OpenLibrary, open_library),
                    (Source::Inventaire, inventaire),
                    // Last: the other two providers are book-specific and
                    // usually better for Spanish editions.
                    (Source::Wikidata, wikidata),
                ])
            }
            Kind::Music => {
                if disabled.contains(Source::MusicBrainz.id()) {
                    return SearchResult::default();
                }
                let musicbrainz = musicbrainz::search(&self.client, query).await;
                collect_candidates(vec![(Source::MusicBrainz, musicbrainz)])
            }
            Kind::Film | Kind::TvSeries | Kind::Videogame | Kind::Comic | Kind::Boardgame => {
                if disabled.contains(Source::Wikidata.id()) {
                    return SearchResult::default();
                }
                let wikidata = wikidata::search(&self.client, kind, query).await;
                collect_candidates(vec![(Source::Wikidata, wikidata)])
            }
            Kind::Other => SearchResult::default(),
        }
    }

    /// A scanned or typed code: an ISBN is a book, anything else is tried
    /// against the non-ISBN providers (music, board games).
    pub async fn lookup_code<C: CloudApi>(
        &self,
        code: &str,
        google: GoogleAccess<'_, C>,
        gameupc_api_key: Option<&str>,
        disabled: &BTreeSet<String>,
    ) -> CodeResult {
        match Isbn::parse(code) {
            Ok(isbn) => self.lookup_isbn_code(&isbn, google, disabled).await,
            Err(_) => {
                self.lookup_non_isbn_code(code, gameupc_api_key, disabled)
                    .await
            }
        }
    }

    async fn lookup_isbn_code<C: CloudApi>(
        &self,
        isbn: &Isbn,
        google: GoogleAccess<'_, C>,
        disabled: &BTreeSet<String>,
    ) -> CodeResult {
        let (merged, inventaire) = tokio::join!(
            self.run(isbn, google, disabled),
            maybe(
                disabled.contains(Source::Inventaire.id()),
                inventaire::lookup(&self.client, isbn),
            ),
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

    /// A non-ISBN code: MusicBrainz always runs (it needs no key); GameUPC
    /// (board games) only runs when a key is set. `decide_kind` resolves the
    /// ambiguity when a code could plausibly be either.
    async fn lookup_non_isbn_code(
        &self,
        code: &str,
        gameupc_api_key: Option<&str>,
        disabled: &BTreeSet<String>,
    ) -> CodeResult {
        let mut result = CodeResult::default();
        let run_gameupc = gameupc_api_key.filter(|_| !disabled.contains(Source::GameUpc.id()));
        let gameupc_fut = async {
            match run_gameupc {
                Some(key) => gameupc::lookup(&self.client, code, key).await,
                None => Ok(None),
            }
        };

        let (music, gameupc) = tokio::join!(
            maybe_vec(
                disabled.contains(Source::MusicBrainz.id()),
                musicbrainz::lookup_barcode(&self.client, code),
            ),
            gameupc_fut,
        );

        let mut music_found = false;
        match music {
            Ok(candidates) => {
                music_found = !candidates.is_empty();
                result.candidates.extend(candidates);
            }
            Err(error) => result.warnings.push(ProviderWarning {
                source: Source::MusicBrainz,
                kind: warning_kind(&error),
            }),
        }

        let mut boardgame_found = false;
        match gameupc {
            Ok(Some(candidate)) => {
                boardgame_found = true;
                result.candidates.push(candidate);
            }
            Ok(None) => {}
            Err(error) => result.warnings.push(ProviderWarning {
                source: Source::GameUpc,
                kind: warning_kind(&error),
            }),
        }

        result.kind = decide_kind(music_found, boardgame_found);
        result
    }
}

/// Skips a provider call entirely (no network) when its source is disabled.
async fn maybe<T>(
    disabled: bool,
    fut: impl Future<Output = Result<Option<T>, ProviderError>>,
) -> Result<Option<T>, ProviderError> {
    if disabled { Ok(None) } else { fut.await }
}

async fn maybe_vec(
    disabled: bool,
    fut: impl Future<Output = Result<Vec<Candidate>, ProviderError>>,
) -> Result<Vec<Candidate>, ProviderError> {
    if disabled { Ok(Vec::new()) } else { fut.await }
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

/// Field by field: the National Library of Spain first (librarian-cataloged,
/// best for Spanish editions), then Google Books, then Open Library. The cover
/// prefers Open Library, whose large images beat Google's thumbnails (the BNE has none).
pub fn merge(
    isbn: &Isbn,
    bne: ProviderResult,
    google: ProviderResult,
    open_library: ProviderResult,
) -> LookupResult {
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
    let bne = found(Source::Bne, bne).unwrap_or_default();
    let google = found(Source::GoogleBooks, google).unwrap_or_default();
    let open_library = found(Source::OpenLibrary, open_library).unwrap_or_default();

    result.found = !result.sources.is_empty();
    result.title = bne.title.or(google.title).or(open_library.title);
    result.subtitle = bne.subtitle.or(google.subtitle).or(open_library.subtitle);
    result.authors = [bne.authors, google.authors, open_library.authors]
        .into_iter()
        .find(|authors| !authors.is_empty())
        .unwrap_or_default();
    result.publisher = bne
        .publisher
        .or(google.publisher)
        .or(open_library.publisher);
    result.published_year = bne
        .published_year
        .or(google.published_year)
        .or(open_library.published_year);
    result.language = bne.language.or(google.language).or(open_library.language);
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

        let result = merge(&isbn(), Ok(None), Ok(Some(google)), Ok(Some(open_library)));

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
    fn merge_prefers_bne_text_and_fills_gaps_from_the_others() {
        let bne = Metadata {
            title: Some("El temor de un hombre sabio".into()),
            authors: vec!["Patrick Rothfuss".into()],
            publisher: Some("Plaza & Janés".into()),
            ..Metadata::default()
        };
        let google = Metadata {
            title: Some("The Wise Man's Fear".into()),
            authors: vec!["P. Rothfuss".into()],
            publisher: Some("Other".into()),
            published_year: Some(2012),
            cover_url: Some("https://books.google.com/thumb".into()),
            ..Metadata::default()
        };

        let result = merge(&isbn(), Ok(Some(bne)), Ok(Some(google)), Ok(None));

        assert_eq!(result.title.as_deref(), Some("El temor de un hombre sabio"));
        assert_eq!(result.authors, vec!["Patrick Rothfuss"]);
        assert_eq!(result.publisher.as_deref(), Some("Plaza & Janés"));
        assert_eq!(result.published_year, Some(2012), "gap filled from Google");
        assert_eq!(
            result.cover_url.as_deref(),
            Some("https://books.google.com/thumb")
        );
        assert_eq!(result.sources, vec![Source::Bne, Source::GoogleBooks]);
    }

    #[test]
    fn a_failing_provider_becomes_a_warning() {
        let open_library = Metadata {
            title: Some("T".into()),
            ..Metadata::default()
        };

        let result = merge(
            &isbn(),
            Ok(None),
            Err(ProviderError::Quota),
            Ok(Some(open_library)),
        );

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
        let result = merge(&isbn(), Ok(None), Ok(None), Ok(None));
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
    use crate::cloud::client::HttpCloud;

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_lookup() {
        let lookup = Lookup::new().unwrap();
        let isbn = Isbn::parse("9780140328721").unwrap();

        let result = lookup
            .run(
                &isbn,
                GoogleAccess::<HttpCloud>::Anonymous,
                &BTreeSet::new(),
            )
            .await;

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
        assert!(
            std::path::Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == "jpg")
        );
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
    async fn live_bne_finds_a_spanish_isbn() {
        let client = http_client().unwrap();
        let isbn = Isbn::parse("9788401339639").unwrap();
        let metadata = bne::lookup(&client, &isbn).await.unwrap().unwrap();
        println!("bne: {metadata:#?}");
        assert_eq!(metadata.language.as_deref(), Some("es"));
        assert_eq!(metadata.authors, vec!["Patrick Rothfuss"]);
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

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_wikidata_film() {
        let client = http_client().unwrap();
        let candidates = wikidata::search(&client, Kind::Film, "Dune").await.unwrap();
        println!("wikidata film: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_wikidata_tv_series() {
        let client = http_client().unwrap();
        let candidates = wikidata::search(&client, Kind::TvSeries, "Breaking Bad")
            .await
            .unwrap();
        println!("wikidata tv series: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_wikidata_videogame() {
        let client = http_client().unwrap();
        let candidates = wikidata::search(&client, Kind::Videogame, "Portal 2")
            .await
            .unwrap();
        println!("wikidata videogame: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_wikidata_boardgame() {
        let client = http_client().unwrap();
        let candidates = wikidata::search(&client, Kind::Boardgame, "Pandemic")
            .await
            .unwrap();
        println!("wikidata boardgame: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_wikidata_comic() {
        let client = http_client().unwrap();
        let candidates = wikidata::search(&client, Kind::Comic, "Maus")
            .await
            .unwrap();
        println!("wikidata comic: {candidates:#?}");
        assert!(!candidates.is_empty());
    }

    #[tokio::test]
    #[ignore = "needs network"]
    async fn live_search_wikidata_book() {
        let client = http_client().unwrap();
        let candidates = wikidata::search(&client, Kind::Book, "Dune").await.unwrap();
        println!("wikidata book: {candidates:#?}");
        assert!(!candidates.is_empty());
    }
}
