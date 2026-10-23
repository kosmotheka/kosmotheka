//! BoardGameGeek and RPGGeek searched through the server of the account (7.8, D62). Only through the server: BoardGameGeek
//! does not want its token in the hands of end users, so there is no key of their own for it. As with the other providers,
//! the text a person searches for goes only where it must (never without a session, never to a server that does not say
//! it offers the search), and what comes back is somebody else's: eight candidates of the fixed shape, covers only from
//! its image host.
//!
//! One thing is different. BoardGameGeek's terms forbid changing its data, so a value that does not fit is **dropped
//! whole**, never cut or cleaned (`whole_text`), unlike the other providers' readers.

use std::time::Duration;

use kosmotheka_domain::{ContributorRole, Kind};
use serde_json::Value;
use url::Url;

use super::via_server::server_error;
use super::{Candidate, CandidateContributor, CandidateExpands, ProviderError, Source};
use crate::cloud::CloudApi;

/// The provider's id in `GET /v1/server`'s `metadata` list: one for both sites, which share a token.
pub const BGG: &str = "bgg";

/// A search asks the details of up to eight results, in one call.
const PATIENCE: Duration = Duration::from_secs(12);
const MAX_CANDIDATES: usize = 8;
const MAX_PEOPLE: usize = 5;
const MAX_TEXT: usize = 300;
const MAX_URL: usize = 500;
/// The biggest number a server may give for a thing, as the server itself allows it.
const MAX_ID: u64 = 1_000_000_000;
/// Where BoardGameGeek keeps its images (GameUPC's covers come from there too): the server validates the address, the
/// app checks it again, and only this is downloaded for a candidate.
const COVER_HOST: &str = "cf.geekdo-images.com";

/// What is searched: board games on BoardGameGeek, or role-playing books on RPGGeek.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BggScope {
    Boardgame,
    Rpg,
}

impl BggScope {
    /// The word the server's route takes.
    pub fn word(self) -> &'static str {
        match self {
            BggScope::Boardgame => "boardgame",
            BggScope::Rpg => "rpg",
        }
    }
}

/// Searches through the server of the account. Without a session (`cloud` is `None`), or on a server that does not say
/// it offers the search, nothing is sent at all. A server that fails or is slow is a warning, as for any provider.
pub async fn search_bgg<C: CloudApi>(
    cloud: Option<&C>,
    scope: BggScope,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let Some(cloud) = cloud else {
        return Ok(Vec::new());
    };
    let asked = tokio::time::timeout(PATIENCE, async {
        if !cloud.offers_metadata(BGG).await.unwrap_or(false) {
            return Ok(Vec::new());
        }
        cloud.bgg_search(scope, query.trim()).await
    })
    .await;
    match asked {
        Ok(Ok(candidates)) => Ok(candidates),
        Ok(Err(error)) => Err(server_error(&error)),
        Err(_) => Err(ProviderError::Unexpected("server too slow".to_owned())),
    }
}

/// Reads the server's answer: at most eight candidates of the fixed shape, each field checked, what does not fit
/// dropped whole. `Err` when it is not the shape at all (the caller treats that as a failed server).
pub fn parse_bgg_answer(body: &Value, scope: BggScope) -> Result<Vec<Candidate>, ()> {
    let list = body.get("candidates").and_then(Value::as_array).ok_or(())?;
    Ok(list
        .iter()
        .take(10 * MAX_CANDIDATES)
        .filter_map(|item| candidate(item, scope))
        .take(MAX_CANDIDATES)
        .collect())
}

fn candidate(item: &Value, scope: BggScope) -> Option<Candidate> {
    let id = item
        .get("external_id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))?;
    let title = whole_text(item.get("title"), MAX_TEXT)?;

    let designers = names(item.get("designers"));
    let artists = names(item.get("artists"));
    let illustrators = artists.into_iter().map(|name| CandidateContributor {
        name,
        role: ContributorRole::Illustrator,
    });

    let mut candidate = match scope {
        BggScope::Boardgame => {
            let mut candidate = Candidate::new(Kind::Boardgame, Source::BoardGameGeek, title);
            candidate.contributors = designers
                .into_iter()
                .map(|name| CandidateContributor {
                    name,
                    role: ContributorRole::Designer,
                })
                .chain(illustrators)
                .collect();
            candidate.players_min = number(item.get("players_min"), 99);
            candidate.players_max = number(item.get("players_max"), 99);
            candidate.duration_minutes = number(item.get("duration_minutes"), 10_000);
            candidate.min_age = number(item.get("min_age"), 99);
            candidate.expands = expands(item.get("expands"));
            candidate
        }
        BggScope::Rpg => {
            // A role-playing book is a book: its designers are its authors.
            let mut candidate = Candidate::new(Kind::Book, Source::RpgGeek, title);
            candidate.authors = designers;
            candidate.contributors = illustrators.collect();
            candidate
        }
    };
    candidate.external_id = Some(id.to_string());
    candidate.published_year = item
        .get("year")
        .and_then(Value::as_i64)
        .filter(|year| (1000..=2100).contains(year))
        .and_then(|year| i32::try_from(year).ok());
    candidate.publisher = whole_text(item.get("publisher"), MAX_TEXT);
    candidate.cover_url = item
        .get("cover_url")
        .and_then(Value::as_str)
        .filter(|url| is_cover(url))
        .map(str::to_owned);
    Some(candidate)
}

/// The game an expansion needs: its number and its name, both or nothing.
fn expands(value: Option<&Value>) -> Option<CandidateExpands> {
    let value = value?;
    let id = value
        .get("external_id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))?;
    Some(CandidateExpands {
        external_id: id.to_string(),
        title: whole_text(value.get("title"), MAX_TEXT)?,
    })
}

fn number(value: Option<&Value>, max: i64) -> Option<i64> {
    value?.as_i64().filter(|n| (1..=max).contains(n))
}

fn names(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|people| {
            people
                .iter()
                .take(10 * MAX_PEOPLE)
                .filter_map(|person| whole_text(Some(person), MAX_TEXT))
                .take(MAX_PEOPLE)
                .collect()
        })
        .unwrap_or_default()
}

/// A value as it was sent, with only the blanks around it trimmed, or nothing: one that is too long, or holds control
/// characters or marks that hide or reverse text, is dropped whole and never cut or cleaned.
pub(super) fn whole_text(value: Option<&Value>, max: usize) -> Option<String> {
    let text = value?.as_str()?.trim();
    let hidden = |c: char| {
        c.is_control()
            || matches!(c, '\u{200B}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
    };
    (!text.is_empty() && text.chars().count() <= max && !text.chars().any(hidden))
        .then(|| text.to_owned())
}

/// `https://cf.geekdo-images.com/…` made only of the characters its addresses use and ending in an image extension:
/// nothing else is ever downloaded for a candidate.
pub(super) fn is_cover(raw: &str) -> bool {
    if raw.len() > MAX_URL
        || raw.contains("..")
        || raw.contains(['%', '\\'])
        || raw.chars().any(|c| c.is_control() || c.is_whitespace())
        || raw
            .get("https://".len()..)
            .is_none_or(|rest| rest.contains("//"))
    {
        return false;
    }
    Url::parse(raw).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str() == Some(COVER_HOST)
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url
                .path()
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-./=:()".contains(&b))
            && url.path().rsplit_once('.').is_some_and(|(_, extension)| {
                matches!(extension, "jpg" | "jpeg" | "png" | "webp" | "gif")
            })
    })
}
