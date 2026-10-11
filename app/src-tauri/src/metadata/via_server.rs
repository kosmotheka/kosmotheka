//! Google Books asked through the account's server (7.8, D83). Three ways, in this order of preference: the person's
//! own key (direct), the server of the account (it puts its key), or Google without a key. The server is only ever
//! asked when it says it offers the search, and whatever it fails at, the person gets the direct answer as before.

use std::future::Future;
use std::time::Duration;

use kosmotheka_domain::{ContributorRole, Format, Isbn, Kind};
use serde_json::Value;
use url::Url;

use super::{Candidate, CandidateContributor, Metadata, ProviderError, Source};
use crate::cloud::CloudApi;
use crate::error::Error;

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

/// The provider's id for TMDB in `GET /v1/server`'s `metadata` list.
pub const TMDB: &str = "tmdb";

/// More than Google Books: a search asks the details of up to eight results, together.
const TMDB_PATIENCE: Duration = Duration::from_secs(12);
const MAX_CANDIDATES: usize = 8;
const MAX_PEOPLE: usize = 5;
/// Where the posters of TMDB live: the server builds the address, the app checks it again, and only this is downloaded.
const TMDB_COVER_HOST: &str = "image.tmdb.org";
const TMDB_COVER_DIR: &str = "/t/p/w500/";

/// Who asks TMDB or RAWG for a search (7.8, D62): the person's own key, the server of the account, or nobody.
pub enum SearchAccess<'a, C> {
    /// The person's own key: straight to the provider, and the server is never told.
    Key(&'a str),
    /// The server of the account, if it offers the search.
    Server(&'a C),
    /// No key and no session: nothing is asked.
    Nothing,
}

/// Their own key first (as for Google Books, D83), else the server of the session, else nobody. A key that is only
/// blanks is no key, and one with blanks around it is used without them.
pub fn choose_search_access<'a, C>(
    key: Option<&'a str>,
    cloud: Option<&'a C>,
) -> SearchAccess<'a, C> {
    match (key.map(str::trim).filter(|key| !key.is_empty()), cloud) {
        (Some(key), _) => SearchAccess::Key(key),
        (None, Some(cloud)) => SearchAccess::Server(cloud),
        (None, None) => SearchAccess::Nothing,
    }
}

/// Searches films or series by title in TMDB through the server of the account. Without a session (`cloud` is
/// `None`), for a kind TMDB does not cover, or on a server that does not say it offers the search, nothing is sent at
/// all and there is nothing to add. A server that fails or is slow is a warning, as for any other provider.
pub async fn search_tmdb<C, D, Fut>(
    access: SearchAccess<'_, C>,
    kind: Kind,
    query: &str,
    language: &str,
    direct: D,
) -> Result<Vec<Candidate>, ProviderError>
where
    C: CloudApi,
    D: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<Candidate>, ProviderError>>,
{
    if !matches!(kind, Kind::Film | Kind::TvSeries) {
        return Ok(Vec::new());
    }
    let cloud = match access {
        SearchAccess::Key(key) => return direct(key.to_owned()).await,
        SearchAccess::Server(cloud) => cloud,
        SearchAccess::Nothing => return Ok(Vec::new()),
    };

    let asked = tokio::time::timeout(TMDB_PATIENCE, async {
        if !cloud.offers_metadata(TMDB).await.unwrap_or(false) {
            return Ok(Vec::new());
        }
        cloud.tmdb_search(kind, query.trim(), language).await
    })
    .await;
    match asked {
        Ok(Ok(candidates)) => Ok(candidates),
        Ok(Err(error)) => Err(server_error(&error)),
        Err(_) => Err(ProviderError::Unexpected("server too slow".to_owned())),
    }
}

/// Reads the server's answer to a title search: at most eight candidates of the fixed shape, each field checked and
/// cut, what does not fit dropped. `Err` when it is not the shape at all (the caller treats that as a failed server).
pub fn parse_tmdb_answer(body: &Value, kind: Kind) -> Result<Vec<Candidate>, ()> {
    let list = body.get("candidates").and_then(Value::as_array).ok_or(())?;
    Ok(list
        .iter()
        .take(10 * MAX_CANDIDATES)
        .filter_map(|item| tmdb_candidate(item, kind))
        .take(MAX_CANDIDATES)
        .collect())
}

fn tmdb_candidate(item: &Value, kind: Kind) -> Option<Candidate> {
    let id = item
        .get("external_id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))?;
    let original = text(item.get("original_title"), MAX_TEXT);
    let title = text(item.get("title"), MAX_TEXT).or_else(|| original.clone())?;

    let mut candidate = Candidate::new(kind, Source::Tmdb, title);
    candidate.external_id = Some(id.to_string());
    candidate.published_year = item
        .get("year")
        .and_then(Value::as_i64)
        .filter(|year| (1800..=2200).contains(year))
        .and_then(|year| i32::try_from(year).ok());
    candidate.language = item
        .get("language")
        .and_then(Value::as_str)
        .filter(|language| is_language(language))
        .map(str::to_owned);
    candidate.cover_url = item
        .get("cover_url")
        .and_then(Value::as_str)
        .filter(|url| is_tmdb_cover(url))
        .map(str::to_owned);

    let (people, role) = match kind {
        Kind::TvSeries => ("creators", ContributorRole::Writer),
        _ => ("directors", ContributorRole::Director),
    };
    candidate.contributors = names(item.get(people))
        .into_iter()
        .map(|name| CandidateContributor { name, role })
        .collect();
    match kind {
        Kind::TvSeries => {
            candidate.seasons = item
                .get("seasons")
                .and_then(Value::as_i64)
                .filter(|seasons| (1..=500).contains(seasons));
        }
        _ => {
            candidate.duration_minutes = item
                .get("duration_minutes")
                .and_then(Value::as_i64)
                .filter(|minutes| (1..=1000).contains(minutes));
        }
    }
    Some(candidate)
}

fn names(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|people| {
            people
                .iter()
                .take(10 * MAX_PEOPLE)
                .filter_map(|person| text(Some(person), MAX_NAME))
                .take(MAX_PEOPLE)
                .collect()
        })
        .unwrap_or_default()
}

/// `https://image.tmdb.org/t/p/w500/<letters and numbers>.jpg`: nothing else is ever downloaded for a candidate.
fn is_tmdb_cover(raw: &str) -> bool {
    if raw.len() > MAX_URL || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    Url::parse(raw).is_ok_and(|url| {
        let file = url.path().strip_prefix(TMDB_COVER_DIR);
        url.scheme() == "https"
            && url.host_str() == Some(TMDB_COVER_HOST)
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && file.is_some_and(is_poster_file)
    })
}

/// `abc123.jpg` or `.png`.
fn is_poster_file(file: &str) -> bool {
    file.rsplit_once('.').is_some_and(|(name, extension)| {
        (1..=100).contains(&name.len())
            && name.bytes().all(|b| b.is_ascii_alphanumeric())
            && matches!(extension, "jpg" | "png")
    })
}

/// The provider's id for RAWG in `GET /v1/server`'s `metadata` list.
pub const RAWG: &str = "rawg";

/// The longest platform name a copy can hold (`MAX_COPY_TEXT_LENGTH`): a longer one is not suggested.
const MAX_PLATFORM: usize = 60;
const MAX_PLATFORMS: usize = 10;
const MAX_PUBLISHERS: usize = 200;
/// Where the images of RAWG live: the server validates the address, the app checks it again, and only this is downloaded.
const RAWG_COVER_HOST: &str = "media.rawg.io";

/// Searches video games by title in RAWG through the server of the account, with the same care as TMDB's search:
/// without a session, or on a server that does not say it offers it, nothing is sent at all.
pub async fn search_rawg<C, D, Fut>(
    access: SearchAccess<'_, C>,
    query: &str,
    direct: D,
) -> Result<Vec<Candidate>, ProviderError>
where
    C: CloudApi,
    D: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<Candidate>, ProviderError>>,
{
    let cloud = match access {
        SearchAccess::Key(key) => return direct(key.to_owned()).await,
        SearchAccess::Server(cloud) => cloud,
        SearchAccess::Nothing => return Ok(Vec::new()),
    };

    let asked = tokio::time::timeout(TMDB_PATIENCE, async {
        if !cloud.offers_metadata(RAWG).await.unwrap_or(false) {
            return Ok(Vec::new());
        }
        cloud.rawg_search(query.trim()).await
    })
    .await;
    match asked {
        Ok(Ok(candidates)) => Ok(candidates),
        Ok(Err(error)) => Err(server_error(&error)),
        Err(_) => Err(ProviderError::Unexpected("server too slow".to_owned())),
    }
}

/// How a failure of the server reads as a warning: its quota is a quota, anything else is unexpected.
fn server_error(error: &Error) -> ProviderError {
    match error {
        Error::Account { code, .. }
            if code == "metadata_quota_exhausted" || code == "too_many_requests" =>
        {
            ProviderError::Quota
        }
        _ => ProviderError::Unexpected("server".to_owned()),
    }
}

/// Reads the server's answer to a video game search: at most eight candidates of the fixed shape, each field checked
/// and cut, what does not fit dropped. `Err` when it is not the shape at all.
pub fn parse_rawg_answer(body: &Value) -> Result<Vec<Candidate>, ()> {
    let list = body.get("candidates").and_then(Value::as_array).ok_or(())?;
    Ok(list
        .iter()
        .take(10 * MAX_CANDIDATES)
        .filter_map(rawg_candidate)
        .take(MAX_CANDIDATES)
        .collect())
}

fn rawg_candidate(item: &Value) -> Option<Candidate> {
    let id = item
        .get("external_id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))?;
    let title = text(item.get("title"), MAX_TEXT)?;

    let mut candidate = Candidate::new(Kind::Videogame, Source::Rawg, title);
    candidate.external_id = Some(id.to_string());
    candidate.published_year = item
        .get("year")
        .and_then(Value::as_i64)
        .filter(|year| (1950..=2200).contains(year))
        .and_then(|year| i32::try_from(year).ok());
    candidate.cover_url = item
        .get("cover_url")
        .and_then(Value::as_str)
        .filter(|url| is_rawg_cover(url))
        .map(str::to_owned);

    // RAWG lists every platform a game came out on; the copy is of one, so the person chooses it, unless it is the only.
    let platforms: Vec<String> = item
        .get("platforms")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .take(10 * MAX_PLATFORMS)
                .filter_map(|platform| text(Some(platform), MAX_NAME))
                .take(MAX_PLATFORMS)
                .collect()
        })
        .unwrap_or_default();
    if let [only] = platforms.as_slice() {
        candidate.platform = Some(only.clone()).filter(|name| name.chars().count() <= MAX_PLATFORM);
    }

    // The studio is who made it; the publisher goes in its own field (the same as Wikidata's video games).
    candidate.contributors = names(item.get("developers"))
        .into_iter()
        .map(|name| CandidateContributor {
            name,
            role: ContributorRole::Studio,
        })
        .collect();
    let publishers = names(item.get("publishers")).join(", ");
    candidate.publisher =
        (!publishers.is_empty()).then(|| publishers.chars().take(MAX_PUBLISHERS).collect());
    Some(candidate)
}

/// `https://media.rawg.io/media/…` ending in an image extension, made only of plain names.
fn is_rawg_cover(raw: &str) -> bool {
    if raw.len() > MAX_URL || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    Url::parse(raw).is_ok_and(|url| {
        let path = url.path().strip_prefix("/media/");
        url.scheme() == "https"
            && url.host_str() == Some(RAWG_COVER_HOST)
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && path.is_some_and(is_rawg_image_path)
    })
}

/// `games/456/abc.jpg`, as RAWG names its images.
fn is_rawg_image_path(path: &str) -> bool {
    is_plain_image_path(path, &["jpg", "jpeg", "png", "webp"], 0)
}

/// The provider's id for Comic Vine in `GET /v1/server`'s `metadata` list.
pub const COMIC_VINE: &str = "comic_vine";

const MAX_NUMBER: usize = 20;
/// The biggest number a provider or a server may give for a thing, as the server itself allows it.
const MAX_ID: u64 = 1_000_000_000;
/// Where Comic Vine keeps its images: the server validates the address, the app checks it again, and only this is
/// downloaded for a candidate.
const COMIC_VINE_COVER_HOST: &str = "comicvine.gamespot.com";

/// Searches comics by title in Comic Vine: with the person's own key straight to it, else through the server of the
/// account if it offers it, else nothing is asked. As for RAWG, a server that fails or is slow is a warning.
pub async fn search_comic_vine<C, D, Fut>(
    access: SearchAccess<'_, C>,
    query: &str,
    direct: D,
) -> Result<Vec<Candidate>, ProviderError>
where
    C: CloudApi,
    D: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<Candidate>, ProviderError>>,
{
    let cloud = match access {
        SearchAccess::Key(key) => return direct(key.to_owned()).await,
        SearchAccess::Server(cloud) => cloud,
        SearchAccess::Nothing => return Ok(Vec::new()),
    };

    let asked = tokio::time::timeout(TMDB_PATIENCE, async {
        if !cloud.offers_metadata(COMIC_VINE).await.unwrap_or(false) {
            return Ok(Vec::new());
        }
        cloud.comic_vine_search(query.trim()).await
    })
    .await;
    match asked {
        Ok(Ok(candidates)) => Ok(candidates),
        Ok(Err(error)) => Err(server_error(&error)),
        Err(_) => Err(ProviderError::Unexpected("server too slow".to_owned())),
    }
}

/// Reads the answer to a comic search, from the server or made from a provider's own: at most eight candidates, series
/// and issues, each field checked and cut, what does not fit dropped. `Err` when it is not the shape at all.
pub fn parse_comic_vine_answer(body: &Value) -> Result<Vec<Candidate>, ()> {
    let list = body.get("candidates").and_then(Value::as_array).ok_or(())?;
    Ok(list
        .iter()
        .take(10 * MAX_CANDIDATES)
        .filter_map(comic_vine_candidate)
        .take(MAX_CANDIDATES)
        .collect())
}

fn comic_vine_candidate(item: &Value) -> Option<Candidate> {
    let resource = item.get("resource")?.as_str()?;
    if !matches!(resource, "volume" | "issue") {
        return None;
    }
    let id = item
        .get("external_id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))?;
    let title = text(item.get("title"), MAX_TEXT);

    let mut candidate = if resource == "volume" {
        let mut candidate = Candidate::new(Kind::Comic, Source::ComicVine, title?);
        candidate.publisher = text(item.get("publisher"), MAX_NAME);
        candidate
    } else {
        let series = text(item.get("series"), MAX_TEXT);
        let number = text(item.get("number"), MAX_NUMBER);
        let name = issue_title(series.as_deref(), number.as_deref(), title.as_deref())?;
        let mut candidate = Candidate::new(Kind::Comic, Source::ComicVine, name);
        candidate.number = number;
        // A single issue, as a person holds it.
        candidate.format = Some(Format::SingleIssue);
        candidate
    };
    candidate.external_id = Some(format!("{resource}-{id}"));
    candidate.published_year = item
        .get("year")
        .and_then(Value::as_i64)
        .filter(|year| (1800..=2200).contains(year))
        .and_then(|year| i32::try_from(year).ok());
    candidate.cover_url = item
        .get("cover_url")
        .and_then(Value::as_str)
        .filter(|url| is_comic_vine_cover(url))
        .map(str::to_owned);
    candidate.contributors = names(item.get("writers"))
        .into_iter()
        .map(|name| CandidateContributor {
            name,
            role: ContributorRole::Writer,
        })
        .chain(
            names(item.get("artists"))
                .into_iter()
                .map(|name| CandidateContributor {
                    name,
                    role: ContributorRole::Illustrator,
                }),
        )
        .collect();
    Some(candidate)
}

/// `Batman #1: The Joker`, or as much of it as there is; nothing when there is nothing to call it.
fn issue_title(series: Option<&str>, number: Option<&str>, title: Option<&str>) -> Option<String> {
    let numbered = |name: &str| match number {
        Some(number) => format!("{name} #{number}"),
        None => name.to_owned(),
    };
    let composed = match (series, title) {
        (Some(series), Some(title)) if title != series => format!("{}: {title}", numbered(series)),
        (Some(series), _) => numbered(series),
        (None, Some(title)) => numbered(title),
        (None, None) => format!("#{}", number?),
    };
    Some(composed.chars().take(MAX_TEXT).collect())
}

/// `https://comicvine.gamespot.com/a/uploads/<names>/<file>` ending in an image extension the app can store.
fn is_comic_vine_cover(raw: &str) -> bool {
    if raw.len() > MAX_URL || raw.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some(COMIC_VINE_COVER_HOST)
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path().strip_prefix("/a/uploads/").is_some_and(|path| {
                is_plain_image_path(path, &["jpg", "jpeg", "png", "gif", "webp"], 1)
            })
    })
}

/// `games/456/abc.jpg`: plain names between slashes (at least `min_dirs` of them), the last with an image extension.
fn is_plain_image_path(path: &str, extensions: &[&str], min_dirs: usize) -> bool {
    let plain = |name: &str| {
        !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    let mut parts: Vec<&str> = path.split('/').collect();
    let Some(file) = parts.pop() else {
        return false;
    };
    parts.len() >= min_dirs
        && file
            .rsplit_once('.')
            .is_some_and(|(name, extension)| plain(name) && extensions.contains(&extension))
        && parts.iter().all(|part| plain(part))
}
