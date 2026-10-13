//! Library of Congress: free, keyless catalogue served over SRU 1.1 as MARC21 XML, the same reading as the National
//! Library of Spain's. It is the best source for editions in English, cataloged by librarians. It has no covers.
//!
//! It asks programs for no more than ten requests a minute: one search here is one request.

use kosmotheka_domain::Isbn;

use super::marc::{Record, candidates_from, parse_records, pick_for_isbn};
use super::{Candidate, Metadata, ProviderError, Source, get_text};

/// The encrypted address. The catalogue also answers on port 210, in the clear: never that one.
const ENDPOINT: &str = "https://lx2.loc.gov/sru/lcdb";

pub async fn lookup(
    client: &reqwest::Client,
    isbn: &Isbn,
) -> Result<Option<Metadata>, ProviderError> {
    let body = fetch(client, &format!("bath.isbn={isbn}"), "5").await?;
    Ok(pick_for_isbn(books(parse_records(&body)?), isbn))
}

/// Title search, for the add-by-search flow.
pub async fn search(
    client: &reqwest::Client,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let body = fetch(client, &title_query(query), "10").await?;
    Ok(candidates_from(
        books(parse_records(&body)?),
        Source::LibraryOfCongress,
    ))
}

/// What a person typed, as one quoted phrase: without the two characters that could end it.
fn title_query(query: &str) -> String {
    let cleaned = query.replace(['"', '\\'], " ");
    format!("dc.title=\"{}\"", cleaned.trim())
}

/// The catalogue holds recordings, maps and photographs too, and a title finds them all.
fn books(records: Vec<Record>) -> Vec<Record> {
    records.into_iter().filter(|record| record.book).collect()
}

async fn fetch(
    client: &reqwest::Client,
    query: &str,
    max_records: &str,
) -> Result<String, ProviderError> {
    get_text(client.get(ENDPOINT).query(&[
        ("version", "1.1"),
        ("operation", "searchRetrieve"),
        ("recordSchema", "marcxml"),
        ("maximumRecords", max_records),
        ("query", query),
    ]))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real responses recorded on 2026-10-13.
    const ISBN: &str = include_str!("../../tests/fixtures/loc-isbn-9780756404734.xml");
    const NOT_FOUND: &str = include_str!("../../tests/fixtures/loc-not-found.xml");
    const SEARCH: &str = include_str!("../../tests/fixtures/loc-search-name-of-the-wind.xml");

    fn isbn() -> Isbn {
        Isbn::parse("9780756404734").unwrap()
    }

    #[test]
    fn an_isbn_gives_the_book_with_its_values_cleaned() {
        let metadata = pick_for_isbn(books(parse_records(ISBN).unwrap()), &isbn()).unwrap();

        assert_eq!(metadata.title.as_deref(), Some("The wise man's fear"));
        assert_eq!(metadata.authors, vec!["Patrick Rothfuss"]);
        assert_eq!(metadata.publisher.as_deref(), Some("DAW Books"));
        assert_eq!(metadata.published_year, Some(2011));
        assert_eq!(metadata.language.as_deref(), Some("en"));
        assert_eq!(metadata.external_id.as_deref(), Some("16982418"));
        assert_eq!(metadata.cover_url, None);
    }

    #[test]
    fn an_unknown_isbn_is_not_found() {
        let records = books(parse_records(NOT_FOUND).unwrap());
        assert!(pick_for_isbn(records, &isbn()).is_none());
    }

    #[test]
    fn a_search_keeps_only_books() {
        let records = parse_records(SEARCH).unwrap();
        // The recorded answer has a sound disc and a photograph among the books.
        assert!(records.iter().any(|r| !r.book));

        let candidates = candidates_from(books(records), Source::LibraryOfCongress);

        assert!(!candidates.is_empty());
        assert!(
            candidates
                .iter()
                .all(|c| c.source == Source::LibraryOfCongress)
        );
        assert!(candidates.iter().all(|c| c.title != "In the name of love"));
        assert!(candidates.iter().all(|c| !c.title.contains("Wind River")));
        assert!(
            candidates
                .iter()
                .all(|c| !c.title.ends_with(['/', ':', ' ']))
        );
    }

    #[test]
    fn a_search_reads_an_isbn_that_comes_with_a_note() {
        // Recorded as "0691067767 :" and "0691014620 (pbk.) :".
        let candidates = candidates_from(
            books(parse_records(SEARCH).unwrap()),
            Source::LibraryOfCongress,
        );
        let noted = candidates
            .iter()
            .find(|c| c.title == "My name on the wind")
            .expect("the book is listed");
        assert_eq!(noted.isbn.as_deref(), Some("9780691067766"));
    }

    #[test]
    fn a_title_cannot_close_its_own_quotes() {
        assert_eq!(
            title_query(" dune\" or dc.creator=\"x\\ "),
            "dc.title=\"dune  or dc.creator= x\""
        );
    }
}
