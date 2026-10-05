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
