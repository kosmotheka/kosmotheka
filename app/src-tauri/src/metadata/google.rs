//! Google Books volumes API. Works without a key, but anonymous requests share a
//! global quota and often get 429; an optional per-user key fixes that (D26).

use library_domain::Isbn;
use reqwest::StatusCode;
use serde_json::Value;

use super::{Metadata, ProviderError, text, year_in};

const API: &str = "https://www.googleapis.com/books/v1/volumes";

pub async fn lookup(
    client: &reqwest::Client,
    isbn: &Isbn,
    api_key: Option<&str>,
) -> Result<Option<Metadata>, ProviderError> {
    let mut request = client.get(API).query(&[("q", format!("isbn:{isbn}"))]);
    if let Some(key) = api_key {
        request = request.query(&[("key", key)]);
    }
    let response = request.send().await?;
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::Quota),
        status if !status.is_success() => {
            return Err(ProviderError::Unexpected(format!("HTTP {status}")));
        }
        _ => {}
    }
    let body: Value = response.json().await?;
    Ok(parse(&body))
}

/// Reads the first volume of a search response.
pub fn parse(body: &Value) -> Option<Metadata> {
    let volume = body.get("items")?.get(0)?;
    let info = volume.get("volumeInfo")?;
    let str_field = |name: &str| text(info.get(name).and_then(Value::as_str));

    Some(Metadata {
        title: str_field("title"),
        subtitle: str_field("subtitle"),
        authors: info
            .get("authors")
            .and_then(Value::as_array)
            .map(|names| names.iter().filter_map(|n| text(n.as_str())).collect())
            .unwrap_or_default(),
        publisher: str_field("publisher"),
        published_year: str_field("publishedDate").as_deref().and_then(year_in),
        language: str_field("language"),
        cover_url: info
            .pointer("/imageLinks/thumbnail")
            .and_then(Value::as_str)
            .map(secure_thumbnail),
        external_id: text(volume.get("id").and_then(Value::as_str)),
    })
}

/// Google returns `http://` thumbnails with a page-curl effect; ask for https and a flat image.
fn secure_thumbnail(url: &str) -> String {
    url.replacen("http://", "https://", 1)
        .replace("&edge=curl", "")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // Follows the documented response format. No real response could be recorded
    // on 2026-09-25 because the anonymous quota was exhausted (see D26).
    const SAMPLE: &str = include_str!("../../tests/fixtures/google-volumes-sample.json");

    #[test]
    fn reads_the_first_volume() {
        let metadata = parse(&serde_json::from_str(SAMPLE).unwrap()).unwrap();

        assert_eq!(metadata.title.as_deref(), Some("Fantastic Mr. Fox"));
        assert_eq!(metadata.authors, vec!["Roald Dahl"]);
        assert_eq!(metadata.publisher.as_deref(), Some("Puffin"));
        assert_eq!(metadata.published_year, Some(1988));
        assert_eq!(metadata.language.as_deref(), Some("en"));
        assert_eq!(metadata.external_id.as_deref(), Some("wBqhPgAACAAJ"));
        let cover = metadata.cover_url.unwrap();
        assert!(cover.starts_with("https://books.google.com/"));
        assert!(!cover.contains("edge=curl"));
    }

    #[test]
    fn no_items_means_not_found() {
        assert_eq!(
            parse(&json!({ "kind": "books#volumes", "totalItems": 0 })),
            None
        );
    }
}
