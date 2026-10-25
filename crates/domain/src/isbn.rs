use std::fmt;

use serde::{Serialize, Serializer};

/// A valid ISBN, always stored in its 13-digit form without separators.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Isbn(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IsbnError {
    #[error("an ISBN must have 10 or 13 digits")]
    InvalidLength,
    #[error("an ISBN may only contain digits, hyphens, spaces and a final X")]
    InvalidCharacter,
    #[error("the ISBN check digit does not match")]
    InvalidChecksum,
    /// A valid EAN-13 outside the 978 and 979 ranges, such as a shop's price label.
    #[error("an ISBN-13 starts with 978 or 979")]
    NotBookland,
}

impl Isbn {
    /// Accepts ISBN-10 or ISBN-13, with or without hyphens and spaces.
    pub fn parse(input: &str) -> Result<Self, IsbnError> {
        let cleaned: String = input
            .chars()
            .filter(|c| !matches!(c, '-' | ' '))
            .map(|c| c.to_ascii_uppercase())
            .collect();

        match cleaned.len() {
            13 => parse_isbn13(&cleaned),
            10 => parse_isbn10(&cleaned),
            _ => Err(IsbnError::InvalidLength),
        }
    }

    /// The ISBNs printed in `text`, such as the text read from a photo of a back cover, in the order they appear.
    /// Recognition adds noise between the digits (a stray quote, a dot, spaces), so within a line everything that
    /// is not a letter is skipped; a letter other than a final X ends the number. Only what the check digit
    /// confirms is kept: thirteen digits starting with 978 or 979 anywhere in a run of digits, or a run of exactly
    /// ten (a shorter window of a longer run would match one time in eleven by chance).
    pub fn find_in_text(text: &str) -> Vec<Self> {
        let mut found: Vec<Self> = Vec::new();
        for line in text.lines() {
            for run in digit_runs(line) {
                let mut candidates: Vec<Self> = run
                    .as_bytes()
                    .windows(13)
                    .filter(|window| window.starts_with(b"978") || window.starts_with(b"979"))
                    .filter_map(|window| std::str::from_utf8(window).ok())
                    .filter_map(|window| Self::parse(window).ok())
                    .collect();
                if candidates.is_empty() && run.len() == 10 {
                    candidates.extend(Self::parse(&run).ok());
                }
                for isbn in candidates {
                    if !found.contains(&isbn) {
                        found.push(isbn);
                    }
                }
            }
        }
        found
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Only ISBNs with the 978 prefix have an ISBN-10 equivalent.
    pub fn to_isbn10(&self) -> Option<String> {
        let body = self.0.strip_prefix("978")?;
        let first_nine: Vec<u32> = body[..9].chars().filter_map(|c| c.to_digit(10)).collect();
        let check = match isbn10_check_value(&first_nine) {
            10 => 'X',
            n => char::from_digit(n, 10)?,
        };
        Some(format!("{}{check}", &body[..9]))
    }
}

/// The runs of digits in a line, skipping anything that is not a letter between them; an X right after the digits
/// stays (it is an ISBN-10's check value) and ends the run, as any other letter does.
fn digit_runs(line: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut run = String::new();
    for c in line.chars() {
        if c.is_ascii_digit() {
            run.push(c);
        } else if c.is_alphabetic() {
            if matches!(c, 'X' | 'x') && !run.is_empty() {
                run.push('X');
            }
            if !run.is_empty() {
                runs.push(std::mem::take(&mut run));
            }
        }
    }
    if !run.is_empty() {
        runs.push(run);
    }
    runs
}

fn parse_isbn13(cleaned: &str) -> Result<Isbn, IsbnError> {
    let digits = to_digits(cleaned)?;
    if isbn13_check_digit(&digits[..12]) != digits[12] {
        return Err(IsbnError::InvalidChecksum);
    }
    if !(cleaned.starts_with("978") || cleaned.starts_with("979")) {
        return Err(IsbnError::NotBookland);
    }
    Ok(Isbn(cleaned.to_owned()))
}

fn parse_isbn10(cleaned: &str) -> Result<Isbn, IsbnError> {
    let (body, last) = cleaned.split_at(9);
    let first_nine = to_digits(body)?;
    let check = match last {
        "X" => 10,
        _ => to_digits(last)?[0],
    };
    if isbn10_check_value(&first_nine) != check {
        return Err(IsbnError::InvalidChecksum);
    }

    let mut isbn13_digits = vec![9, 7, 8];
    isbn13_digits.extend_from_slice(&first_nine);
    let check13 = isbn13_check_digit(&isbn13_digits);
    Ok(Isbn(format!("978{body}{check13}")))
}

fn to_digits(s: &str) -> Result<Vec<u32>, IsbnError> {
    s.chars()
        .map(|c| c.to_digit(10).ok_or(IsbnError::InvalidCharacter))
        .collect()
}

/// Weights alternate 1 and 3; the check digit brings the sum to a multiple of 10.
fn isbn13_check_digit(first_twelve: &[u32]) -> u32 {
    let sum: u32 = first_twelve
        .iter()
        .enumerate()
        .map(|(i, d)| if i % 2 == 0 { *d } else { d * 3 })
        .sum();
    (10 - sum % 10) % 10
}

/// Weights go from 10 down to 2; the check value (10 is written X) makes the sum divisible by 11.
fn isbn10_check_value(first_nine: &[u32]) -> u32 {
    let sum: u32 = first_nine
        .iter()
        .zip((2..=10).rev())
        .map(|(d, weight)| d * weight)
        .sum();
    (11 - sum % 11) % 11
}

impl fmt::Display for Isbn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Isbn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<String> {
        Isbn::find_in_text(text)
            .into_iter()
            .map(|isbn| isbn.as_str().to_owned())
            .collect()
    }

    #[test]
    fn finds_an_isbn_with_noise_from_recognition() {
        // The maintainer's back cover (25/10/2026): a quote right after the first digit.
        assert_eq!(found("ISBN 9\"78-84-376-0494-7"), ["9788437604947"]);
        assert_eq!(found("I.S.B.N.: 978·84·376·0494·7."), ["9788437604947"]);
        assert_eq!(found("9 788437 604947"), ["9788437604947"]);
    }

    #[test]
    fn finds_the_isbn_of_a_whole_back_cover() {
        // Ragnarok, read with the text recognition plugin on a moto g84 (25/10/2026): 121 lines; the only number is
        // the one under the barcode, with quotes between its groups.
        let text = "Ragnarok\nJuego de Rol de Horror Contemporáne0\nHace 65 millones de años, un enorme meteorito\n\
                    chocó contra la Tierra, provocando un cataclismo\nLo que supondrá el fin del mundo\n\
                    tal como lo conocemos.\nCOPIA DIGITAL INCLUIDA\n9\" 788469 \" 767016\nEDICIONES\nt&t\n\
                    MKULTRA\nLOS ÚLTIMOS\nDÍAS\nHAN LLEGADO\nDagbladet\nSKANDIA\nTHULE";
        assert_eq!(found(text), ["9788469767016"]);
    }

    #[test]
    fn finds_an_isbn_among_other_numbers_of_the_line() {
        assert_eq!(
            found("Depósito legal: M-12345-2020 ISBN 978-84-376-0494-7"),
            ["9788437604947"]
        );
        assert_eq!(found("12,90 € 9788437604947 (rústica)"), ["9788437604947"]);
    }

    #[test]
    fn a_misread_digit_gives_nothing_rather_than_another_book() {
        assert!(found("ISBN 978-84-376-0494-8").is_empty());
        assert!(
            found("ISBN 978-84-376-O494-7").is_empty(),
            "a letter breaks the number"
        );
    }

    #[test]
    fn finds_an_isbn_10_only_when_it_stands_alone() {
        assert_eq!(found("ISBN 84-376-0494-X"), ["9788437604947"]);
        assert_eq!(found("ISBN 0-8044-2957-X"), ["9780804429573"]);
        // Ten digits inside a longer number are not taken for one.
        assert!(found("Tel. 34 84 376 0494 7 1").is_empty());
    }

    #[test]
    fn finds_each_isbn_once_and_in_order() {
        let text =
            "Tapa dura ISBN 978-84-376-0494-7\nRústica ISBN 978-0-306-40615-7\n9788437604947";
        assert_eq!(found(text), ["9788437604947", "9780306406157"]);
    }

    #[test]
    fn accepts_isbn13_with_hyphens() {
        let isbn = Isbn::parse("978-0-306-40615-7").unwrap();
        assert_eq!(isbn.as_str(), "9780306406157");
    }

    #[test]
    fn converts_isbn10_to_isbn13() {
        assert_eq!(
            Isbn::parse("0-306-40615-2").unwrap().as_str(),
            "9780306406157"
        );
    }

    #[test]
    fn accepts_isbn10_ending_in_x() {
        let isbn = Isbn::parse("0-8044-2957-x").unwrap();
        assert_eq!(isbn.as_str(), "9780804429573");
        assert_eq!(isbn.to_isbn10().as_deref(), Some("080442957X"));
    }

    #[test]
    fn rejects_wrong_check_digit() {
        assert_eq!(
            Isbn::parse("9780306406158"),
            Err(IsbnError::InvalidChecksum)
        );
        assert_eq!(Isbn::parse("0306406153"), Err(IsbnError::InvalidChecksum));
    }

    #[test]
    fn rejects_a_shop_label() {
        assert_eq!(Isbn::parse("2401114882038"), Err(IsbnError::NotBookland));
    }

    #[test]
    fn rejects_wrong_length_and_characters() {
        assert_eq!(Isbn::parse("12345"), Err(IsbnError::InvalidLength));
        assert_eq!(
            Isbn::parse("97803064061X7"),
            Err(IsbnError::InvalidCharacter)
        );
        assert_eq!(Isbn::parse("X306406152"), Err(IsbnError::InvalidCharacter));
    }

    #[test]
    fn isbn13_without_978_prefix_has_no_isbn10() {
        let isbn = Isbn::parse("979-10-90636-07-1").unwrap();
        assert_eq!(isbn.to_isbn10(), None);
    }
}
