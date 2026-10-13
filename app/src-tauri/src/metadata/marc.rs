//! MARC21 records as national libraries serve them over SRU (MARCXML): what the National Library of Spain and the
//! Library of Congress have in common. Each has its own address and search words; the reading is the same.

use kosmotheka_domain::{Isbn, Kind};
use roxmltree::{Document, Node};

use super::{Candidate, Metadata, ProviderError, Source, year_in};

/// One MARC record, reduced to what the app uses.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct Record {
    pub(super) metadata: Metadata,
    pub(super) isbns: Vec<String>,
    /// E-books are cataloged apart from the printed edition and rarely match what is on a shelf.
    pub(super) electronic: bool,
    /// A printed or electronic book, and not a recording, a map or a photograph: a whole catalogue holds them all.
    pub(super) book: bool,
}

/// The print record whose ISBN is the requested one, else any print record,
/// else an electronic one.
pub(super) fn pick_for_isbn(records: Vec<Record>, isbn: &Isbn) -> Option<Metadata> {
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

pub(super) fn candidates_from(records: Vec<Record>, source: Source) -> Vec<Candidate> {
    let any_print = records.iter().any(|r| !r.electronic);
    records
        .into_iter()
        .filter(|r| !any_print || !r.electronic)
        .filter_map(|record| {
            let metadata = record.metadata;
            let mut candidate = Candidate::new(Kind::Book, source, metadata.title?);
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
pub(super) fn parse_records(xml: &str) -> Result<Vec<Record>, ProviderError> {
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

    let electronic = first("300", "a").is_some_and(|extent| {
        extent.contains("recurso en línea") || extent.contains("online resource")
    }) || first("337", "b").as_deref() == Some("c");
    // Leader positions 6 and 7: language material, published as a monograph.
    let book = record
        .children()
        .find(|n| n.has_tag_name("leader"))
        .and_then(|n| n.text())
        .is_some_and(|leader| leader.get(6..8) == Some("am"));

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
        // The number may come with a note after it: "0691014620 (pbk.) :".
        isbns: fields("020")
            .filter_map(|f| sub(f, "a"))
            .filter_map(|raw| raw.split_whitespace().next().map(str::to_owned))
            .collect(),
        electronic,
        book,
    })
}

/// Strips the ISBD punctuation MARC leaves at the end of a value ("Title /",
/// "Plaza & Janés,", "[2011]." and the like).
pub(super) fn clean(raw: &str) -> Option<String> {
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
pub(super) fn invert_name(name: &str) -> String {
    match name.split_once(',') {
        Some((surname, given)) if !given.trim().is_empty() => {
            format!("{} {}", given.trim(), surname.trim())
        }
        _ => name.trim_end_matches(',').trim().to_owned(),
    }
}
