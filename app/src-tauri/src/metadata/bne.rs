//! National Library of Spain (BNE): free, keyless catalogue served over SRU 1.2
//! as MARC21 XML. It is the best source for Spanish editions: authors and
//! publisher are cataloged by librarians. It has no covers.

use kosmotheka_domain::Isbn;
use reqwest::StatusCode;

use super::marc::{candidates_from, parse_records, pick_for_isbn};
use super::{Candidate, Metadata, ProviderError, Source};

const ENDPOINT: &str = "https://bne.alma.exlibrisgroup.com/view/sru/34BNE_INST";

pub async fn lookup(
    client: &reqwest::Client,
    isbn: &Isbn,
) -> Result<Option<Metadata>, ProviderError> {
    let body = fetch(client, &format!("alma.isbn={isbn}"), "5").await?;
    Ok(pick_for_isbn(parse_records(&body)?, isbn))
}

/// Title search, for the add-by-search flow.
pub async fn search(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let cleaned = query.replace('"', " ");
    let body = fetch(client, &format!("alma.title=\"{}\"", cleaned.trim()), "10").await?;
    Ok(candidates_from(parse_records(&body)?, Source::Bne))
}

async fn fetch(
    client: &reqwest::Client,
    query: &str,
    max_records: &str,
) -> Result<String, ProviderError> {
    let response = client
        .get(ENDPOINT)
        .query(&[
            ("version", "1.2"),
            ("operation", "searchRetrieve"),
            ("recordSchema", "marcxml"),
            ("maximumRecords", max_records),
            ("query", query),
        ])
        .send()
        .await?;
    match response.status() {
        StatusCode::TOO_MANY_REQUESTS => Err(ProviderError::Quota),
        status if !status.is_success() => Err(ProviderError::Unexpected(format!("HTTP {status}"))),
        _ => Ok(response.text().await?),
    }
}

#[cfg(test)]
mod tests {
    use super::super::marc::{clean, invert_name};
    use super::*;

    // Real responses recorded on 2026-10-05.
    const ISBN: &str = include_str!("../../tests/fixtures/bne-isbn-9788401339639.xml");
    const NOT_FOUND: &str = include_str!("../../tests/fixtures/bne-not-found.xml");
    const SEARCH: &str = include_str!("../../tests/fixtures/bne-search-temor.xml");

    fn isbn() -> Isbn {
        Isbn::parse("9788401339639").unwrap()
    }

    #[test]
    fn picks_the_print_record_for_the_requested_isbn() {
        let records = parse_records(ISBN).unwrap();
        assert_eq!(records.len(), 2);
        assert!(records.iter().any(|r| r.electronic));

        let metadata = pick_for_isbn(records, &isbn()).unwrap();

        assert_eq!(
            metadata.title.as_deref(),
            Some("El temor de un hombre sabio")
        );
        assert_eq!(metadata.authors, vec!["Patrick Rothfuss"]);
        assert_eq!(metadata.publisher.as_deref(), Some("Plaza & Janés"));
        assert_eq!(metadata.published_year, Some(2011));
        assert_eq!(metadata.language.as_deref(), Some("es"));
        assert!(metadata.external_id.is_some());
    }

    #[test]
    fn an_unknown_isbn_is_not_found() {
        let records = parse_records(NOT_FOUND).unwrap();
        assert!(records.is_empty());
        assert!(pick_for_isbn(records, &isbn()).is_none());
    }

    #[test]
    fn a_search_skips_electronic_records_and_cleans_values() {
        let candidates = candidates_from(parse_records(SEARCH).unwrap(), Source::Bne);

        assert!(!candidates.is_empty());
        assert!(candidates.iter().all(|c| c.source == Source::Bne));
        assert!(
            candidates
                .iter()
                .all(|c| !c.title.ends_with(['/', ':', ' ']))
        );
        let print = candidates
            .iter()
            .find(|c| c.isbn.as_deref() == Some("9788401339639"))
            .expect("the print edition is listed");
        assert_eq!(print.title, "El temor de un hombre sabio");
    }

    #[test]
    fn garbage_is_an_unexpected_response() {
        assert!(matches!(
            parse_records("not xml"),
            Err(ProviderError::Unexpected(_))
        ));
    }

    #[test]
    fn isbd_punctuation_is_stripped() {
        assert_eq!(
            clean("El temor de un hombre sabio /").as_deref(),
            Some("El temor de un hombre sabio")
        );
        assert_eq!(
            clean("Rothfuss, Patrick,").as_deref(),
            Some("Rothfuss, Patrick")
        );
        assert_eq!(clean("[2011].").as_deref(), Some("[2011]"));
        assert_eq!(clean(" : ").as_deref(), None);
        assert_eq!(invert_name("Rothfuss, Patrick"), "Patrick Rothfuss");
        assert_eq!(invert_name("Homero"), "Homero");
    }
}
