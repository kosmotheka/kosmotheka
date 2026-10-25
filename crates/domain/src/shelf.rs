//! The shelf photo, a help for the first load: what the text read on a shelf's spines is worth searching for in the
//! catalogues, and whether a work they return is the one that was read. The catalogues are the filter: only what
//! matches a real work's title is offered, so a misread spine or a publisher's logo never becomes a book.

/// One line read in the photo, with how sure the engine was of it, when it says.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadLine {
    pub text: String,
    pub confidence: Option<f32>,
}

/// Lines the engine grouped together: usually one spine, or the part of it written in one direction.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReadBlock {
    pub lines: Vec<ReadLine>,
}

/// Below this, the engine's own guess is mostly noise.
pub const MIN_CONFIDENCE: f32 = 0.5;
/// One shelf holds 15 to 20 spines: more searches than this would only be noise, and the catalogues' patience.
pub const MAX_QUERIES: usize = 40;
const MIN_LETTERS: usize = 4;
/// Words a line may have beyond the title: the author, the publisher.
const MAX_EXTRA_WORDS: usize = 4;

/// What to search for, in the order it was read: each line worth it, and the first two lines of a block joined,
/// for a title written over two lines. The same text is searched once.
pub fn queries(blocks: &[ReadBlock]) -> Vec<String> {
    let mut queries: Vec<String> = Vec::new();
    let mut seen: Vec<Vec<String>> = Vec::new();
    let mut push = |query: String| {
        let key = words(&query);
        if !seen.contains(&key) {
            seen.push(key);
            queries.push(query);
        }
    };
    for block in blocks {
        let kept: Vec<String> = block.lines.iter().filter_map(worth_searching).collect();
        if let [first, second, ..] = kept.as_slice()
            && words(first) != words(second)
        {
            push(format!("{first} {second}"));
        }
        kept.into_iter().for_each(&mut push);
    }
    queries.truncate(MAX_QUERIES);
    queries
}

/// The line, tidied, if it can be a title: read with enough confidence, with some letters, and more letters than
/// digits (a price, a reference or a year are not titles).
fn worth_searching(line: &ReadLine) -> Option<String> {
    if line.confidence.is_some_and(|c| c < MIN_CONFIDENCE) {
        return None;
    }
    let text = line.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let digits = text.chars().filter(char::is_ascii_digit).count();
    (letters >= MIN_LETTERS && letters > digits).then_some(text)
}

/// The first title that is what was read: every word of the title is in the text (a letter or two may be misread
/// in a long word), and the text has little more than the title.
pub fn first_match<'a>(read: &str, titles: impl IntoIterator<Item = &'a str>) -> Option<usize> {
    let read = words(read);
    titles.into_iter().position(|title| {
        let title = words(title);
        !title.is_empty()
            && title.iter().map(String::len).sum::<usize>() >= MIN_LETTERS
            && read.len() <= title.len() + MAX_EXTRA_WORDS
            && title
                .iter()
                .all(|word| read.iter().any(|seen| alike(word, seen)))
    })
}

fn alike(word: &str, seen: &str) -> bool {
    let allowed = match word.chars().count() {
        0..5 => 0,
        5..8 => 1,
        _ => 2,
    };
    allowed == 0 && word == seen || allowed > 0 && distance(word, seen) <= allowed
}

/// Lowercase words without accents, so «LA SOMBRA DEL VIENTO» reads like «La sombra del viento».
fn words(text: &str) -> Vec<String> {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// How many letters must change to turn one word into the other (Levenshtein).
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(ca != *cb))
                .min(above + 1)
                .min(row[j] + 1);
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(lines: &[(&str, Option<f32>)]) -> ReadBlock {
        ReadBlock {
            lines: lines
                .iter()
                .map(|(text, confidence)| ReadLine {
                    text: (*text).to_owned(),
                    confidence: *confidence,
                })
                .collect(),
        }
    }

    #[test]
    fn a_spine_gives_its_lines_and_its_first_two_joined() {
        let spine = block(&[
            ("La sombra", Some(0.9)),
            ("del  viento", None),
            ("Planeta", Some(0.8)),
        ]);

        assert_eq!(
            queries(&[spine]),
            ["La sombra del viento", "La sombra", "del viento", "Planeta"]
        );
    }

    #[test]
    fn noise_is_not_searched() {
        let shelf = block(&[
            ("Dune", Some(0.95)),
            ("O9AINC I0 YINGKUSIS", Some(0.3)),
            ("8,95 €", None),
            ("ISBN 84-376", None),
            ("Ed.", None),
            ("1984", None),
        ]);

        assert_eq!(queries(&[shelf]), ["Dune"]);
    }

    #[test]
    fn the_same_text_is_searched_once() {
        let shelf = [
            block(&[("DUNE", None)]),
            block(&[("Dune", None)]),
            block(&[("dune", None), ("Dune", None)]),
        ];

        assert_eq!(queries(&shelf), ["DUNE"]);
    }

    #[test]
    fn no_more_than_a_shelf_is_searched() {
        let wall: Vec<ReadBlock> = (0..100)
            .map(|n| block(&[(&format!("Spine number {n}"), None)]))
            .collect();

        assert_eq!(queries(&wall).len(), MAX_QUERIES);
    }

    #[test]
    fn a_title_read_on_its_spine_matches() {
        let titles = ["Ragnarok", "La sombra del viento"];

        assert_eq!(
            first_match("CARLOS RUIZ ZAFÓN LA SOMBRA DEL VIENTO", titles),
            Some(1)
        );
        assert_eq!(first_match("Ragnarok", titles), Some(0));
    }

    #[test]
    fn a_letter_misread_in_a_long_word_still_matches() {
        assert_eq!(first_match("Ragnar0k", ["Ragnarok"]), Some(0));
        assert_eq!(
            first_match("El imperio flnal", ["El imperio final"]),
            Some(0)
        );
    }

    #[test]
    fn another_work_of_the_same_author_does_not_match() {
        // The author's line finds their other books; none of them is what was read.
        let titles = ["El juego del ángel", "Marina", "El prisionero del cielo"];

        assert_eq!(first_match("Carlos Ruiz Zafón", titles), None);
    }

    #[test]
    fn a_short_word_must_be_read_exactly() {
        assert_eq!(first_match("Dune", ["Duna"]), None);
        assert_eq!(first_match("Dune", ["Dune"]), Some(0));
    }

    #[test]
    fn a_title_lost_in_a_long_text_does_not_match() {
        assert_eq!(
            first_match(
                "Lo mejor para el mejor, una guía de viaje para toda la familia",
                ["Lo mejor"]
            ),
            None
        );
    }

    #[test]
    fn a_title_too_short_to_tell_never_matches() {
        assert_eq!(first_match("Un libro de Stephen King: It", ["It"]), None);
    }
}
