//! The order of a book search's results: the title that matches what was typed first, then the app's language, bare
//! titles last, and the editions of one work together (propuesta 4 of docs/testing/ux-escanear-y-editar.md).

use super::Candidate;

/// Reorders `candidates` in place. The sort is stable, so provider order still decides among equals.
pub fn rank_books(candidates: &mut Vec<Candidate>, query: &str, language: &str) {
    let query = fold(query);
    candidates.sort_by_key(|candidate| {
        (
            title_rank(candidate, &query),
            language_rank(candidate, language),
            is_bare(candidate),
        )
    });
    group_works(candidates);
}

/// The title as typed, then a title with those words in it, then the rest: some catalogues answer with anything that
/// shares a word, and the language alone would lift those.
fn title_rank(candidate: &Candidate, query: &str) -> u8 {
    let title = fold(&candidate.title);
    if title == query {
        0
    } else if format!(" {title} ").contains(&format!(" {query} ")) {
        1
    } else {
        2
    }
}

/// Unknown language sits between the app's own and any other: many good results do not say theirs.
fn language_rank(candidate: &Candidate, language: &str) -> u8 {
    match candidate.language.as_deref() {
        Some(code) if code == language => 0,
        None => 1,
        Some(_) => 2,
    }
}

/// A title and nothing else: the result that only fills one field of the form.
fn is_bare(candidate: &Candidate) -> bool {
    candidate.authors.is_empty()
        && candidate.contributors.is_empty()
        && candidate.publisher.is_none()
        && candidate.published_year.is_none()
        && candidate.isbn.is_none()
        && candidate.cover_url.is_none()
}

/// Moves each result up next to the first one of its work, which keeps its place.
fn group_works(candidates: &mut Vec<Candidate>) {
    let mut rest = std::mem::take(candidates);
    while !rest.is_empty() {
        let leader = rest.remove(0);
        let (same, other): (Vec<_>, Vec<_>) = rest
            .into_iter()
            .partition(|candidate| same_work(&leader, candidate));
        candidates.push(leader);
        candidates.extend(same);
        rest = other;
    }
}

fn same_work(a: &Candidate, b: &Candidate) -> bool {
    same_work_by_name(&a.title, &a.authors, &b.title, &b.authors)
}

/// Same title, and an author in common when both say who wrote them.
pub(crate) fn same_work_by_name(
    title: &str,
    authors: &[String],
    other_title: &str,
    other_authors: &[String],
) -> bool {
    if fold(title) != fold(other_title) {
        return false;
    }
    if authors.is_empty() || other_authors.is_empty() {
        return true;
    }
    authors
        .iter()
        .any(|x| other_authors.iter().any(|y| same_person(x, y)))
}

/// Providers write «Herbert, Frank» or «Frank Herbert»: the same words in any order.
fn same_person(a: &str, b: &str) -> bool {
    let words = |name: &str| {
        let mut words: Vec<String> = fold(name).split(' ').map(str::to_owned).collect();
        words.sort();
        words
    };
    words(a) == words(b)
}

/// Lower case, no accents, no punctuation, single spaces: «El nombre del viento.» and «el nombre del viento» match.
fn fold(text: &str) -> String {
    let plain: String = text
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::Source;
    use kosmotheka_domain::Kind;

    fn book(source: Source, title: &str, language: Option<&str>, authors: &[&str]) -> Candidate {
        let mut candidate = Candidate::new(Kind::Book, source, title);
        candidate.language = language.map(str::to_owned);
        candidate.authors = authors.iter().map(|&a| a.to_owned()).collect();
        candidate
    }

    fn order(candidates: &[Candidate]) -> Vec<(Source, &str)> {
        candidates
            .iter()
            .map(|c| (c.source, c.title.as_str()))
            .collect()
    }

    #[test]
    fn the_apps_language_goes_first_and_a_bare_title_last() {
        let mut candidates = vec![
            book(
                Source::LibraryOfCongress,
                "The Name of the Wind",
                Some("en"),
                &["Rothfuss, Patrick"],
            ),
            book(Source::Bne, "El temor de un hombre sabio", Some("es"), &[]),
            book(Source::OpenLibrary, "Kvothe", None, &["Patrick Rothfuss"]),
            book(
                Source::Inventaire,
                "El nombre del viento",
                Some("es"),
                &["Patrick Rothfuss"],
            ),
        ];
        // None of them is called that, so the title does not decide.
        rank_books(&mut candidates, "Rothfuss", "es");
        assert_eq!(
            order(&candidates),
            vec![
                (Source::Inventaire, "El nombre del viento"),
                (Source::Bne, "El temor de un hombre sabio"),
                (Source::OpenLibrary, "Kvothe"),
                (Source::LibraryOfCongress, "The Name of the Wind"),
            ]
        );
    }

    #[test]
    fn editions_of_one_work_come_together_after_the_best_of_them() {
        let mut candidates = vec![
            book(Source::Bne, "Dune", Some("es"), &["Herbert, Frank"]),
            book(Source::Bne, "Dune Mesías", Some("es"), &["Herbert, Frank"]),
            book(Source::OpenLibrary, "dune.", Some("en"), &["Frank Herbert"]),
            book(Source::RpgGeek, "Dune", None, &["Gareth Ryder-Hanrahan"]),
            book(Source::Wikidata, "Dune", None, &[]),
        ];
        rank_books(&mut candidates, "dune", "es");
        assert_eq!(
            order(&candidates),
            vec![
                (Source::Bne, "Dune"),
                // Unknown language and no author: it may well be this work.
                (Source::Wikidata, "Dune"),
                (Source::OpenLibrary, "dune."),
                // Same title, someone else's book.
                (Source::RpgGeek, "Dune"),
                (Source::Bne, "Dune Mesías"),
            ]
        );
    }

    #[test]
    fn a_title_that_matches_beats_the_language_of_one_that_does_not() {
        let mut candidates = vec![
            book(Source::Bne, "¿Qué te hice yo, mi amor?", Some("es"), &[]),
            book(
                Source::OpenLibrary,
                "Dune Messiah",
                Some("es"),
                &["Frank Herbert"],
            ),
            book(
                Source::LibraryOfCongress,
                "Dune",
                Some("en"),
                &["Frank Herbert"],
            ),
        ];
        rank_books(&mut candidates, "Dune", "es");
        assert_eq!(
            order(&candidates),
            vec![
                (Source::LibraryOfCongress, "Dune"),
                (Source::OpenLibrary, "Dune Messiah"),
                (Source::Bne, "¿Qué te hice yo, mi amor?"),
            ]
        );
    }
}
