//! Inventaire (inventaire.io): free, keyless, CC0 book data drawn from
//! Wikidata. Its Spanish edition coverage is often better than the other
//! providers'. https://api.inventaire.io/
//!
//! An edition entity's claims point to its work (`wdt:P629`), whose claims
//! have the authors (`wdt:P50`); labels for those ids need a second request.
//! Language is resolved locally from a small table of Wikidata ids, to avoid
//! a third request just for that.

use kosmotheka_domain::{Isbn, Kind};
use reqwest::StatusCode;
use serde_json::Value;

use super::{Candidate, ProviderError, Source, text, year_in};

const BASE: &str = "https://inventaire.io";

/// Wikidata items for the languages this app already has an ISO 639-1 code
/// for (see `openlibrary::language_code`). Anything else is left unset.
const LANGUAGES: &[(&str, &str)] = &[
    ("wd:Q1860", "en"),
    ("wd:Q1321", "es"),
    ("wd:Q150", "fr"),
    ("wd:Q188", "de"),
    ("wd:Q652", "it"),
    ("wd:Q5146", "pt"),
    ("wd:Q7411", "nl"),
    ("wd:Q7026", "ca"),
    ("wd:Q9307", "gl"),
    ("wd:Q8752", "eu"),
    ("wd:Q397", "la"),
];

/// A candidate still missing the labels of its author and publisher ids.
struct EditionFacts {
    candidate: Candidate,
    author_uris: Vec<String>,
    publisher_uri: Option<String>,
}

pub async fn lookup(
    client: &reqwest::Client,
    isbn: &Isbn,
) -> Result<Option<Candidate>, ProviderError> {
    let isbn_uri = format!("isbn:{isbn}");
    let body = fetch_by_uris(client, std::slice::from_ref(&isbn_uri), Some("wdt:P629")).await?;
    let Some(EditionFacts {
        mut candidate,
        author_uris,
        publisher_uri,
    }) = parse_edition(&body, &isbn_uri)
    else {
        return Ok(None);
    };

    let mut label_uris = author_uris.clone();
    label_uris.extend(publisher_uri.clone());
    if !label_uris.is_empty() {
        let label_body = fetch_by_uris(client, &label_uris, None).await?;
        apply_labels(
            &mut candidate,
            &author_uris,
            publisher_uri.as_deref(),
            &label_body,
        );
    }
    Ok(Some(candidate))
}

/// Title search over Wikidata "work" entities (Inventaire has no edition search).
pub async fn search(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let response = client
        .get(format!("{BASE}/api/search"))
        .query(&[
            ("types", "works"),
            ("search", query),
            ("lang", "es"),
            ("limit", "10"),
        ])
        .send()
        .await?;
    let body = read_body(response).await?;
    Ok(parse_search(&body))
}

async fn fetch_by_uris(
    client: &reqwest::Client,
    uris: &[String],
    relatives: Option<&str>,
) -> Result<Value, ProviderError> {
    let joined = uris.join("|");
    let mut request = client
        .get(format!("{BASE}/api/entities/by-uris"))
        .query(&[("uris", joined.as_str())]);
    if let Some(relatives) = relatives {
        request = request.query(&[("relatives", relatives)]);
    }
    read_body(request.send().await?).await
}

async fn read_body(response: reqwest::Response) -> Result<Value, ProviderError> {
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::Quota),
        status if !status.is_success() => {
            return Err(ProviderError::Unexpected(format!("HTTP {status}")));
        }
        _ => {}
    }
    Ok(response.json().await?)
}

/// Pure: reads the edition (and, via `relatives`, its work) out of a `by-uris` response.
fn parse_edition(body: &Value, isbn_uri: &str) -> Option<EditionFacts> {
    let entities = body.get("entities")?.as_object()?;
    let entity_uri = body
        .pointer(&format!("/redirects/{isbn_uri}"))
        .and_then(Value::as_str)
        .unwrap_or(isbn_uri);
    let edition = entities.get(entity_uri)?;
    let claims = edition.get("claims")?;

    let title = edition
        .pointer("/labels/fromclaims")
        .and_then(Value::as_str)
        .or_else(|| first_claim_str(claims, "wdt:P1476"))
        .and_then(|s| text(Some(s)))?;

    let mut candidate = Candidate::new(Kind::Book, Source::Inventaire, title);
    candidate.isbn = first_claim_str(claims, "wdt:P212")
        .or_else(|| first_claim_str(claims, "wdt:P957"))
        .and_then(|s| Isbn::parse(s).ok())
        .map(|isbn| isbn.as_str().to_owned());
    candidate.published_year = first_claim_str(claims, "wdt:P577").and_then(year_in);
    candidate.cover_url = edition
        .pointer("/image/url")
        .and_then(Value::as_str)
        .map(|path| format!("{BASE}{path}"));
    candidate.external_id = edition
        .get("uri")
        .and_then(Value::as_str)
        .map(str::to_owned);
    candidate.language = first_claim_str(claims, "wdt:P407")
        .and_then(|qid| LANGUAGES.iter().find(|(id, _)| *id == qid))
        .map(|(_, code)| (*code).to_owned());

    let publisher_uri = first_claim_str(claims, "wdt:P123").map(str::to_owned);
    let author_uris = first_claim_str(claims, "wdt:P629")
        .and_then(|work_uri| entities.get(work_uri))
        .and_then(|work| work.pointer("/claims/wdt:P50"))
        .and_then(Value::as_array)
        .map(|uris| {
            uris.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();

    Some(EditionFacts {
        candidate,
        author_uris,
        publisher_uri,
    })
}

fn first_claim_str<'a>(claims: &'a Value, key: &str) -> Option<&'a str> {
    claims.get(key)?.get(0)?.as_str()
}

/// Pure: fills in the author and publisher names from a second `by-uris` response.
fn apply_labels(
    candidate: &mut Candidate,
    author_uris: &[String],
    publisher_uri: Option<&str>,
    label_body: &Value,
) {
    let entities = label_body.get("entities").and_then(Value::as_object);
    let label_of = |uri: &str| -> Option<String> {
        let labels = entities?.get(uri)?.get("labels")?.as_object()?;
        labels
            .get("es")
            .or_else(|| labels.get("en"))
            .or_else(|| labels.values().next())
            .and_then(Value::as_str)
            .and_then(|s| text(Some(s)))
    };
    candidate.authors = author_uris.iter().filter_map(|uri| label_of(uri)).collect();
    candidate.publisher = publisher_uri.and_then(label_of);
}

/// Pure: reads work entities out of a `types=works` search response. No structured
/// author data is available without another request per result, so authors stay empty.
fn parse_search(body: &Value) -> Vec<Candidate> {
    let Some(results) = body.get("results").and_then(Value::as_array) else {
        return Vec::new();
    };
    results.iter().filter_map(candidate_from_result).collect()
}

fn candidate_from_result(result: &Value) -> Option<Candidate> {
    let title = text(result.get("label").and_then(Value::as_str))?;
    let mut candidate = Candidate::new(Kind::Book, Source::Inventaire, title);
    candidate.external_id = text(result.get("uri").and_then(Value::as_str));
    candidate.cover_url = result
        .get("image")
        .and_then(Value::as_str)
        .map(|path| format!("{BASE}{path}"));
    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real response for isbn:9788445000663, recorded on 2026-09-29.
    const EDITION_AND_WORK: &str = include_str!("../../tests/fixtures/inventaire-by-isbn.json");
    const LABELS: &str = include_str!("../../tests/fixtures/inventaire-labels.json");
    const SEARCH: &str = include_str!("../../tests/fixtures/inventaire-search-dune.json");
    const NOT_FOUND: &str = include_str!("../../tests/fixtures/inventaire-not-found.json");

    #[test]
    fn reads_a_real_edition_and_its_work() {
        let body: Value = serde_json::from_str(EDITION_AND_WORK).unwrap();
        let facts = parse_edition(&body, "isbn:9788445000663").unwrap();

        assert_eq!(facts.candidate.kind, Kind::Book);
        assert_eq!(facts.candidate.source, Source::Inventaire);
        assert_eq!(facts.candidate.title, "La comunidad del anillo");
        assert_eq!(facts.candidate.isbn.as_deref(), Some("9788445000663"));
        assert_eq!(facts.candidate.published_year, Some(2012));
        assert_eq!(facts.candidate.language.as_deref(), Some("es"));
        assert_eq!(
            facts.candidate.cover_url.as_deref(),
            Some("https://inventaire.io/img/entities/27186041c3516dd13e117bd7b9a750eef5111a57")
        );
        assert_eq!(
            facts.candidate.external_id.as_deref(),
            Some("inv:8094bbeb1b0e1b1dc8c6d98bb3f6417d")
        );
        assert_eq!(facts.author_uris, vec!["wd:Q892"]);
        assert_eq!(facts.publisher_uri, None);
    }

    #[test]
    fn an_unknown_isbn_is_not_found() {
        let body: Value = serde_json::from_str(NOT_FOUND).unwrap();
        assert!(parse_edition(&body, "isbn:0000000000000").is_none());
    }

    #[test]
    fn labels_fill_in_the_author_name() {
        let body: Value = serde_json::from_str(EDITION_AND_WORK).unwrap();
        let mut candidate = parse_edition(&body, "isbn:9788445000663")
            .unwrap()
            .candidate;
        let labels: Value = serde_json::from_str(LABELS).unwrap();

        apply_labels(&mut candidate, &["wd:Q892".to_owned()], None, &labels);

        assert_eq!(candidate.authors, vec!["J. R. R. Tolkien"]);
    }

    #[test]
    fn reads_a_real_search() {
        let body: Value = serde_json::from_str(SEARCH).unwrap();
        let candidates = parse_search(&body);

        assert_eq!(candidates.len(), 3);
        let dune = &candidates[0];
        assert_eq!(dune.kind, Kind::Book);
        assert_eq!(dune.source, Source::Inventaire);
        assert_eq!(dune.title, "Dune");
        assert_eq!(dune.external_id.as_deref(), Some("wd:Q190192"));
        assert_eq!(
            dune.cover_url.as_deref(),
            Some("https://inventaire.io/img/entities/a936e0978d400fb13e0c0a2527d345a56b799e54")
        );
    }

    #[test]
    fn a_search_with_no_results_is_empty() {
        assert!(parse_search(&serde_json::json!({})).is_empty());
    }
}
