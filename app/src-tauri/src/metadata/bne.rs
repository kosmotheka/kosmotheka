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

/// Title search, for the add-by-search flow. The exact title comes first, as it finds the editions of a title
/// written as it is without the noise of every record that shares its words. Only when it finds nothing, the same
/// words in any order: a title typed with its author («inferno dan brown») is no title, but the BNE's title index
/// also holds the statement of responsibility, so its words still find the book. The BNE answers in catalogue
/// order, not by relevance, and lists e-books first, so this second search asks for more records: the print
/// editions come after them, and only those are kept.
pub async fn search(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let body = fetch(client, &exact_title(query), "10").await?;
    let candidates = candidates_from(parse_records(&body)?, Source::Bne);
    if !candidates.is_empty() {
        return Ok(candidates);
    }
    let body = fetch(client, &all_title_words(query), "25").await?;
    Ok(candidates_from(parse_records(&body)?, Source::Bne))
}

/// The words of a query, without the quotes that would end the CQL string early.
fn words(query: &str) -> String {
    query
        .replace('"', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn exact_title(query: &str) -> String {
    format!("alma.title=\"{}\"", words(query))
}

fn all_title_words(query: &str) -> String {
    format!("alma.title all \"{}\"", words(query))
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
    const ALL_WORDS: &str = include_str!("../../tests/fixtures/bne-search-inferno-dan-brown.xml");

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
    fn the_exact_title_and_the_words_in_any_order_are_two_queries() {
        assert_eq!(
            exact_title("  el nombre  del viento "),
            "alma.title=\"el nombre del viento\""
        );
        assert_eq!(
            all_title_words("inferno dan brown"),
            "alma.title all \"inferno dan brown\""
        );
    }

    #[test]
    fn a_quote_cannot_end_the_query_early() {
        assert_eq!(exact_title("el \"temor\""), "alma.title=\"el temor\"");
        assert_eq!(all_title_words("\" or 1=1 \""), "alma.title all \"or 1=1\"");
    }

    #[test]
    fn a_title_written_with_its_author_finds_the_book_by_its_words() {
        let candidates = candidates_from(parse_records(ALL_WORDS).unwrap(), Source::Bne);

        let booket = candidates
            .iter()
            .find(|c| c.isbn.as_deref() == Some("9788408175735"))
            .expect("the print edition of Inferno, listed after the e-books, is kept");
        assert_eq!(booket.title, "Inferno");
        assert_eq!(booket.authors, ["Dan Brown"]);
        assert!(
            candidates
                .iter()
                .all(|c| c.isbn.as_deref() != Some("9788408118077")),
            "the e-book of the same edition is left out"
        );
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
