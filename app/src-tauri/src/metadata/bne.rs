//! National Library of Spain (BNE): free, keyless catalogue served over SRU 1.2
//! as MARC21 XML. It is the best source for Spanish editions: authors and
//! publisher are cataloged by librarians. It has no covers.

use kosmotheka_domain::{Isbn, Kind};
use reqwest::StatusCode;
use roxmltree::{Document, Node};

use super::{Candidate, Metadata, ProviderError, Source, year_in};

const ENDPOINT: &str = "https://bne.alma.exlibrisgroup.com/view/sru/34BNE_INST";

/// One MARC record, reduced to what the app uses.
#[derive(Debug, Clone, PartialEq, Default)]
struct Record {
    metadata: Metadata,
    isbns: Vec<String>,
    /// E-books are cataloged apart from the printed edition and rarely match what is on a shelf.
    electronic: bool,
}

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
    Ok(candidates_from(parse_records(&body)?))
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

/// The print record whose ISBN is the requested one, else any print record,
/// else an electronic one.
fn pick_for_isbn(records: Vec<Record>, isbn: &Isbn) -> Option<Metadata> {
    let matches_isbn = |record: &Record| {
        record
            .isbns
            .iter()
            .any(|candidate| Isbn::parse(candidate).is_ok_and(|parsed| parsed == *isbn))
    };
    let best = records
        .iter()
        .find(|r| !r.electronic && matches_isbn(r))
        .or_else(|| records.iter().find(|r| matches_isbn(r)))
        .or_else(|| records.iter().find(|r| !r.electronic))
        .or_else(|| records.first())?;
    Some(best.metadata.clone())
}

fn candidates_from(records: Vec<Record>) -> Vec<Candidate> {
    let any_print = records.iter().any(|r| !r.electronic);
    records
        .into_iter()
        .filter(|r| !any_print || !r.electronic)
        .filter_map(|record| {
            let metadata = record.metadata;
            let mut candidate = Candidate::new(Kind::Book, Source::Bne, metadata.title?);
            candidate.subtitle = metadata.subtitle;
            candidate.authors = metadata.authors;
            candidate.publisher = metadata.publisher;
            candidate.published_year = metadata.published_year;
            candidate.language = metadata.language;
            candidate.external_id = metadata.external_id;
            candidate.isbn = record
                .isbns
                .iter()
                .find_map(|raw| Isbn::parse(raw).ok())
                .map(|isbn| isbn.as_str().to_owned());
            Some(candidate)
        })
        .collect()
}

/// Pure: reads every MARC record out of an SRU response. Records without a title are dropped.
fn parse_records(xml: &str) -> Result<Vec<Record>, ProviderError> {
    let document =
        Document::parse(xml).map_err(|error| ProviderError::Unexpected(error.to_string()))?;
    Ok(document
        .descendants()
        .filter(|n| n.has_tag_name("record") && has_child(*n, "leader"))
        .filter_map(parse_record)
        .collect())
}

fn has_child(node: Node, name: &str) -> bool {
    node.children().any(|c| c.has_tag_name(name))
}

fn parse_record(record: Node) -> Option<Record> {
    let control = |tag: &str| {
        record
            .children()
            .find(|n| n.has_tag_name("controlfield") && n.attribute("tag") == Some(tag))
            .and_then(|n| n.text())
    };
    let fields = |tag: &'static str| {
        record
            .children()
            .filter(move |n| n.has_tag_name("datafield") && n.attribute("tag") == Some(tag))
    };
    let sub = |field: Node, code: &str| -> Option<String> {
        field
            .children()
            .find(|n| n.has_tag_name("subfield") && n.attribute("code") == Some(code))
            .and_then(|n| n.text())
            .and_then(clean)
    };
    let first = |tag: &'static str, code: &str| fields(tag).find_map(|f| sub(f, code));

    let title = first("245", "a")?;
    let imprint = |code: &str| first("264", code).or_else(|| first("260", code));
    // 008 positions 35-37 hold the language.
    let language = control("008")
        .and_then(|c| c.chars().skip(35).take(3).collect::<String>().into())
        .filter(|code: &String| {
            code.chars().count() == 3 && code.chars().all(|c| c.is_alphabetic())
        })
        .map(|code| super::openlibrary::language_code(&code));

    let electronic = first("300", "a").is_some_and(|extent| extent.contains("recurso en línea"))
        || first("337", "b").as_deref() == Some("c");

    Some(Record {
        metadata: Metadata {
            title: Some(title),
            subtitle: first("245", "b"),
            authors: first("100", "a")
                .map(|a| invert_name(&a))
                .into_iter()
                .collect(),
            publisher: imprint("b"),
            published_year: imprint("c").as_deref().and_then(year_in),
            language,
            cover_url: None,
            external_id: control("001").and_then(clean),
        },
        isbns: fields("020").filter_map(|f| sub(f, "a")).collect(),
        electronic,
    })
}

/// Strips the ISBD punctuation MARC leaves at the end of a value ("Title /",
/// "Plaza & Janés,", "[2011]." and the like).
fn clean(raw: &str) -> Option<String> {
    let mut value = raw.trim();
    loop {
        let trimmed = value
            .trim_end_matches(|c: char| {
                c.is_whitespace() || matches!(c, '/' | ':' | ';' | '=' | ',')
            })
            .trim_end_matches('.')
            .trim_end();
        if trimmed.len() == value.len() {
            break;
        }
        value = trimmed;
    }
    (!value.is_empty()).then(|| value.to_owned())
}

/// "Rothfuss, Patrick" becomes "Patrick Rothfuss", as the other providers return it.
fn invert_name(name: &str) -> String {
    match name.split_once(',') {
        Some((surname, given)) if !given.trim().is_empty() => {
            format!("{} {}", given.trim(), surname.trim())
        }
        _ => name.trim_end_matches(',').trim().to_owned(),
    }
}

#[cfg(test)]
mod tests {
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
        let candidates = candidates_from(parse_records(SEARCH).unwrap());

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
