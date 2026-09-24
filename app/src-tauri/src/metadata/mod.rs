//! Book data from external catalogues, looked up by ISBN (D26). Every provider is
//! optional: failures become warnings and never block the user (D18).

pub mod google;
pub mod openlibrary;

use std::time::Duration;

use library_domain::Isbn;
use serde::Serialize;

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
}

pub fn http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!(
            "LibraryApp/",
            env!("CARGO_PKG_VERSION"),
            " (+https://github.com/Tersoal/library)"
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
                kind: match error {
                    ProviderError::Network(_) => WarningKind::Network,
                    ProviderError::Quota => WarningKind::Quota,
                    ProviderError::Unexpected(_) => WarningKind::Unexpected,
                },
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
}

/// Hit the real services. Not run by default (CI must not depend on the network):
/// `cargo test -p library-app -- --ignored --nocapture live_`
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
        let url =
            library_domain::parse_cover_url("https://covers.openlibrary.org/b/id/15152634-L.jpg")
                .unwrap();

        let name = covers
            .download(&http_client().unwrap(), &url)
            .await
            .unwrap();

        println!("stored as {name}");
        assert!(name.ends_with(".jpg"));
        assert!(covers.dir().join(&name).metadata().unwrap().len() > 10_000);
    }
}
