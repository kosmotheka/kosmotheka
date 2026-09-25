//! A book someone would like to have (D36).

use serde::Deserialize;
use url::Url;

use crate::book::{ErrorCode, FieldError, parse_cover_url};
use crate::isbn::Isbn;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WishInput {
    pub title: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub isbn: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidWish {
    pub title: String,
    pub author: Option<String>,
    pub isbn: Option<Isbn>,
    pub note: Option<String>,
    pub cover_url: Option<Url>,
}

impl WishInput {
    pub fn validate(self) -> Result<ValidWish, Vec<FieldError>> {
        let clean =
            |value: Option<String>| value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
        let mut errors = Vec::new();

        let title = clean(Some(self.title));
        if title.is_none() {
            errors.push(FieldError::new("title", ErrorCode::Required));
        }
        let isbn = match clean(self.isbn) {
            Some(raw) => Isbn::parse(&raw)
                .map_err(|error| errors.push(FieldError::new("isbn", error.into())))
                .ok(),
            None => None,
        };
        let cover_url = match clean(self.cover_url) {
            Some(raw) => {
                let url = parse_cover_url(&raw);
                if url.is_none() {
                    errors.push(FieldError::new("coverUrl", ErrorCode::InvalidCoverUrl));
                }
                url
            }
            None => None,
        };

        match title {
            Some(title) if errors.is_empty() => Ok(ValidWish {
                title,
                author: clean(self.author),
                isbn,
                note: clean(self.note),
                cover_url,
            }),
            _ => Err(errors),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_reports_errors() {
        let wish = WishInput {
            title: " Tehanu ".into(),
            isbn: Some("0-306-40615-2".into()),
            note: Some("  ".into()),
            ..WishInput::default()
        }
        .validate()
        .unwrap();
        assert_eq!(wish.title, "Tehanu");
        assert_eq!(wish.isbn.unwrap().as_str(), "9780306406157");
        assert_eq!(wish.note, None);

        let errors = WishInput {
            isbn: Some("123".into()),
            ..WishInput::default()
        }
        .validate()
        .unwrap_err();
        assert_eq!(
            errors,
            vec![
                FieldError::new("title", ErrorCode::Required),
                FieldError::new("isbn", ErrorCode::IsbnLength),
            ]
        );
    }
}
