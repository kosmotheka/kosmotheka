//! Open Library. The edition comes from `/isbn/{isbn}.json` (the old `/api/books`
//! endpoint is gone); author names need one request per author.

use kosmotheka_domain::{Isbn, Kind};
use reqwest::StatusCode;
use serde_json::Value;

use super::{Candidate, Metadata, ProviderError, Source, text, year_in};

const BASE: &str = "https://openlibrary.org";
const MAX_AUTHORS: usize = 5;
const SEARCH_FIELDS: &str =
    "key,title,subtitle,author_name,first_publish_year,publisher,language,isbn,cover_i";

pub async fn lookup(
    client: &reqwest::Client,
    isbn: &Isbn,
) -> Result<Option<Metadata>, ProviderError> {
    let response = client
        .get(format!("{BASE}/isbn/{isbn}.json"))
        .send()
        .await?;
    match response.status() {
        StatusCode::NOT_FOUND => return Ok(None),
        StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::Quota),
        status if !status.is_success() => {
            return Err(ProviderError::Unexpected(format!("HTTP {status}")));
        }
        _ => {}
    }
    let edition: Value = response.json().await?;
    let (mut metadata, author_keys) = parse_edition(&edition);

    // A missing author name is not worth failing the whole lookup.
    for key in author_keys.iter().take(MAX_AUTHORS) {
        let author = async {
            client
                .get(format!("{BASE}{key}.json"))
                .send()
                .await?
                .error_for_status()?
                .json::<Value>()
                .await
        };
        if let Ok(author) = author.await
            && let Some(name) = parse_author(&author)
        {
            metadata.authors.push(name);
        }
    }
    Ok(Some(metadata))
}

/// Title search, for the add-by-search flow (D26).
pub async fn search(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let response = client
        .get(format!("{BASE}/search.json"))
        .query(&[("q", query), ("limit", "10"), ("fields", SEARCH_FIELDS)])
        .send()
        .await?;
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::Quota),
        status if !status.is_success() => {
            return Err(ProviderError::Unexpected(format!("HTTP {status}")));
        }
        _ => {}
    }
    let body: Value = response.json().await?;
    Ok(parse_search(&body))
}

pub fn parse_search(body: &Value) -> Vec<Candidate> {
    let Some(docs) = body.get("docs").and_then(Value::as_array) else {
        return Vec::new();
    };
    docs.iter().map(candidate_from_doc).collect()
}

fn candidate_from_doc(doc: &Value) -> Candidate {
    let str_field = |name: &str| text(doc.get(name).and_then(Value::as_str));
    let mut candidate = Candidate::new(
        Kind::Book,
        Source::OpenLibrary,
        str_field("title").unwrap_or_default(),
    );
    candidate.subtitle = str_field("subtitle");
    candidate.authors = doc
        .get("author_name")
        .and_then(Value::as_array)
        .map(|names| names.iter().filter_map(|n| text(n.as_str())).collect())
        .unwrap_or_default();
    candidate.published_year = doc
        .get("first_publish_year")
        .and_then(Value::as_i64)
        .and_then(|y| i32::try_from(y).ok());
    candidate.publisher = doc
        .get("publisher")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .and_then(|s| text(Some(s)));
    candidate.language = doc
        .get("language")
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .map(language_code);
    candidate.isbn = doc
        .get("isbn")
        .and_then(Value::as_array)
        .and_then(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .find_map(|s| Isbn::parse(s).ok())
        })
        .map(|isbn| isbn.as_str().to_owned());
    candidate.cover_url = doc
        .get("cover_i")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"));
    candidate.external_id = str_field("key");
    candidate
}

/// Edition data plus the keys of its authors (`/authors/OL34184A`).
pub fn parse_edition(edition: &Value) -> (Metadata, Vec<String>) {
    let str_field = |name: &str| text(edition.get(name).and_then(Value::as_str));
    let first_str = |name: &str| {
        text(
            edition
                .get(name)
                .and_then(|v| v.get(0))
                .and_then(Value::as_str),
        )
    };
    let author_keys = edition
        .get("authors")
        .and_then(Value::as_array)
        .map(|authors| {
            authors
                .iter()
                .filter_map(|a| a.get("key").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();

    let metadata = Metadata {
        title: str_field("title"),
        subtitle: str_field("subtitle"),
        authors: Vec::new(),
        publisher: first_str("publishers"),
        published_year: str_field("publish_date").as_deref().and_then(year_in),
        language: edition
            .pointer("/languages/0/key")
            .and_then(Value::as_str)
            .and_then(|key| key.strip_prefix("/languages/"))
            .map(language_code),
        // Negative ids mean "no cover".
        cover_url: edition
            .get("covers")
            .and_then(Value::as_array)
            .and_then(|ids| ids.iter().filter_map(Value::as_i64).find(|id| *id > 0))
            .map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg")),
        external_id: str_field("key")
            .as_deref()
            .and_then(|key| key.strip_prefix("/books/"))
            .map(str::to_owned),
    };
    (metadata, author_keys)
}

pub fn parse_author(author: &Value) -> Option<String> {
    text(author.get("name").and_then(Value::as_str))
}

/// Open Library uses three-letter MARC codes; the app uses two-letter ISO 639-1
/// codes where they exist, as Google Books does.
fn language_code(marc: &str) -> String {
    let iso = match marc {
        "eng" => "en",
        "spa" => "es",
        "cat" => "ca",
        "glg" => "gl",
        "baq" | "eus" => "eu",
        "fre" | "fra" => "fr",
        "ger" | "deu" => "de",
        "ita" => "it",
        "por" => "pt",
        "dut" | "nld" => "nl",
        "jpn" => "ja",
        "chi" | "zho" => "zh",
        "rus" => "ru",
        "lat" => "la",
        other => other,
    };
    iso.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real responses recorded on 2026-09-25.
    const EDITION: &str =
        include_str!("../../tests/fixtures/openlibrary-edition-9780140328721.json");
    const AUTHOR: &str = include_str!("../../tests/fixtures/openlibrary-author-OL34184A.json");
    // Trimmed to 3 docs, recorded on 2026-09-29.
    const SEARCH: &str = include_str!("../../tests/fixtures/openlibrary-search-dune.json");

    #[test]
    fn reads_a_real_edition() {
        let (metadata, author_keys) = parse_edition(&serde_json::from_str(EDITION).unwrap());

        assert_eq!(metadata.title.as_deref(), Some("Fantastic Mr. Fox"));
        assert_eq!(metadata.publisher.as_deref(), Some("Puffin"));
        assert_eq!(metadata.published_year, Some(1988));
        assert_eq!(metadata.language.as_deref(), Some("en"));
        assert_eq!(
            metadata.cover_url.as_deref(),
            Some("https://covers.openlibrary.org/b/id/15152634-L.jpg")
        );
        assert_eq!(metadata.external_id.as_deref(), Some("OL7353617M"));
        assert_eq!(author_keys, vec!["/authors/OL34184A"]);
    }

    #[test]
    fn reads_a_real_author() {
        assert_eq!(
            parse_author(&serde_json::from_str(AUTHOR).unwrap()).as_deref(),
            Some("Roald Dahl")
        );
    }

    #[test]
    fn negative_cover_ids_are_ignored() {
        let (metadata, _) = parse_edition(&serde_json::json!({ "title": "T", "covers": [-1] }));
        assert_eq!(metadata.cover_url, None);
    }

    #[test]
    fn spanish_marc_code_becomes_iso() {
        assert_eq!(language_code("spa"), "es");
        assert_eq!(language_code("xyz"), "xyz");
    }

    #[test]
    fn reads_a_real_search() {
        let candidates = parse_search(&serde_json::from_str(SEARCH).unwrap());

        assert_eq!(candidates.len(), 3);
        let dune = &candidates[0];
        assert_eq!(dune.kind, Kind::Book);
        assert_eq!(dune.source, Source::OpenLibrary);
        assert_eq!(dune.title, "Dune");
        assert_eq!(dune.authors, vec!["Frank Herbert"]);
        assert_eq!(dune.published_year, Some(1965));
        assert_eq!(
            dune.publisher.as_deref(),
            Some("Dom Wydawniczy REBIS Sp. z o.o.")
        );
        assert_eq!(dune.language.as_deref(), Some("fr"));
        assert_eq!(dune.isbn.as_deref(), Some("9780441013593"));
        assert_eq!(
            dune.cover_url.as_deref(),
            Some("https://covers.openlibrary.org/b/id/11481354-L.jpg")
        );
        assert_eq!(dune.external_id.as_deref(), Some("/works/OL893414W"));
    }

    #[test]
    fn a_search_with_no_docs_is_empty() {
        assert!(parse_search(&serde_json::json!({})).is_empty());
    }
}
