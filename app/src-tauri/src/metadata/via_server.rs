//! Google Books asked through the account's server (7.8, D83). Three ways, in this order of preference: the person's
//! own key (direct), the server of the account (it puts its key), or Google without a key. The server is only ever
//! asked when it says it offers the search, and whatever it fails at, the person gets the direct answer as before.

use std::future::Future;
use std::time::Duration;

use kosmotheka_domain::Isbn;
use serde_json::Value;
use url::Url;

use super::{Metadata, ProviderError};
use crate::cloud::CloudApi;

/// Who asks Google Books for this lookup.
pub enum GoogleAccess<'a, C> {
    /// The person's own key: straight to Google, as always (D83: it has preference).
    Key(&'a str),
    /// The server of the account, if it offers the search; Google without a key if it fails.
    Server(&'a C),
    /// Google without a key.
    Anonymous,
}

/// Who asks Google Books: their own key, else the server of the account (if there is a session and the setting is on),
/// else nobody in particular. A key that is only blanks is no key.
pub fn choose_access<'a, C>(
    key: Option<&'a str>,
    via_server: bool,
    cloud: Option<&'a C>,
) -> GoogleAccess<'a, C> {
    match (key.map(str::trim).filter(|key| !key.is_empty()), cloud) {
        (Some(key), _) => GoogleAccess::Key(key),
        (None, Some(cloud)) if via_server => GoogleAccess::Server(cloud),
        _ => GoogleAccess::Anonymous,
    }
}

/// The provider's id in `GET /v1/server`'s `metadata` list.
pub const GOOGLE_BOOKS: &str = "google_books";

/// How long the server has to answer before Google is asked directly: the form should not wait for a slow server.
const SERVER_PATIENCE: Duration = Duration::from_secs(8);

const MAX_TEXT: usize = 300;
const MAX_NAME: usize = 200;
const MAX_AUTHORS: usize = 20;
const MAX_URL: usize = 500;
/// The only host the covers of Google Books come from: the app downloads whatever address it is given here.
const COVER_HOST: &str = "books.google.com";

/// `direct` asks Google itself, with the key it is given (none: without a key).
pub async fn lookup_google<C, Direct, Fut>(
    access: GoogleAccess<'_, C>,
    isbn: &Isbn,
    direct: Direct,
) -> Result<Option<Metadata>, ProviderError>
where
    C: CloudApi,
    Direct: Fn(Option<String>) -> Fut,
    Fut: Future<Output = Result<Option<Metadata>, ProviderError>>,
{
    match access {
        GoogleAccess::Key(key) => direct(Some(key.to_owned())).await,
        GoogleAccess::Server(cloud) => {
            // Only a server that says it offers the search is asked, and only the ISBN goes.
            let asked = tokio::time::timeout(SERVER_PATIENCE, async {
                if cloud.offers_metadata(GOOGLE_BOOKS).await? {
                    cloud.google_books_isbn(isbn).await.map(Some)
                } else {
                    Ok(None)
                }
            })
            .await;
            match asked {
                // «Not found» is the server's answer, not a failure.
                Ok(Ok(Some(answer))) => Ok(answer),
                // Not offered, failed or too slow: as if there were no server.
                _ => direct(None).await,
            }
        }
        GoogleAccess::Anonymous => direct(None).await,
    }
}

/// Reads the server's answer, which is only trusted to be the fixed shape: every field is checked and cut, and what
/// does not fit is dropped. `Err` when it is not the shape at all (the caller treats that as a failed server).
pub fn parse_answer(body: &Value) -> Result<Option<Metadata>, ()> {
    let found = body.get("found").and_then(Value::as_bool).ok_or(())?;
    if !found {
        return Ok(None);
    }

    let title = text(body.get("title"), MAX_TEXT);
    let authors: Vec<String> = body
        .get("authors")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .take(10 * MAX_AUTHORS)
                .filter_map(|name| text(Some(name), MAX_NAME))
                .take(MAX_AUTHORS)
                .collect()
        })
        .unwrap_or_default();
    // A book that says nothing a form could use is not a find.
    if title.is_none() && authors.is_empty() {
        return Ok(None);
    }

    Ok(Some(Metadata {
        title,
        subtitle: text(body.get("subtitle"), MAX_TEXT),
        authors,
        publisher: text(body.get("publisher"), MAX_NAME),
        published_year: body
            .get("published_year")
            .and_then(Value::as_i64)
            .filter(|year| (1000..=2100).contains(year))
            .and_then(|year| i32::try_from(year).ok()),
        language: body
            .get("language")
            .and_then(Value::as_str)
            .filter(|language| is_language(language))
            .map(str::to_owned),
        cover_url: body
            .get("cover_url")
            .and_then(Value::as_str)
            .filter(|url| is_google_cover(url))
            .map(str::to_owned),
        external_id: body
            .get("external_id")
            .and_then(Value::as_str)
            .filter(|id| is_external_id(id))
            .map(str::to_owned),
    }))
}

/// A string cleaned of control characters and cut to `max` characters; none when it is not a string or is empty.
fn text(value: Option<&Value>, max: usize) -> Option<String> {
    let raw = value?.as_str()?;
    let cleaned = raw
        .split(char::is_control)
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!cleaned.is_empty()).then(|| cleaned.chars().take(max).collect())
}

/// `es`, `eng`, `pt-BR`.
fn is_language(language: &str) -> bool {
    let (main, region) = match language.split_once('-') {
        Some((main, region)) => (main, Some(region)),
        None => (language, None),
    };
    (2..=3).contains(&main.len())
        && main.bytes().all(|b| b.is_ascii_lowercase())
        && region.is_none_or(|r| {
            (2..=4).contains(&r.len()) && r.bytes().all(|b| b.is_ascii_alphabetic())
        })
}

fn is_external_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `https` on the covers' host, with no user, no port and nothing that `Url` would silently tidy away.
fn is_google_cover(raw: &str) -> bool {
    if raw.len() > MAX_URL || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some(COVER_HOST)
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
    })
}
