//! Comic Vine asked directly with the person's own key (7.8, D62), with no server in between. Comic Vine only takes
//! its key as the `api_key` parameter of the address, so here the key is in every address, and no error may carry it:
//! `get_json` strips the address from every failure and never passes on anything Comic Vine said. What it sends is
//! turned into the fixed shape of the server's answer and goes through the same reader
//! (`via_server::parse_comic_vine_answer`): eight candidates, series first, texts cut, covers only from its image
//! site, numbers that can be.

use std::collections::BTreeMap;

use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::{Candidate, ProviderError, get_json};

pub const API: &str = "https://comicvine.gamespot.com/api";

const RESULTS_LOOKED_AT: usize = 40;
const MAX_CANDIDATES: usize = 8;
/// Only the first results get a second call, as on the server.
const DETAILS_OF: usize = 3;
const MAX_ID: u64 = 1_000_000_000;
const SEARCH_FIELDS: &str =
    "id,name,resource_type,start_year,publisher,image,issue_number,cover_date,store_date,volume";
/// What Comic Vine puts before the id in the address of a detail.
const SERIES_PREFIX: &str = "4050";
const ISSUE_PREFIX: &str = "4000";
const WRITER_ROLES: &[&str] = &["writer"];
const ARTIST_ROLES: &[&str] = &[
    "penciler",
    "penciller",
    "inker",
    "artist",
    "painter",
    "illustrator",
];

pub async fn search(
    client: &reqwest::Client,
    api: &str,
    key: &str,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let body = answer(
        get_json(client.get(format!("{api}/search/")).query(&[
            ("api_key", key),
            ("format", "json"),
            ("query", query),
            ("resources", "volume,issue"),
            ("limit", "8"),
            ("field_list", SEARCH_FIELDS),
        ]))
        .await?,
    )?;
    let results = body
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::Unexpected("comic vine".to_owned()))?;

    // The series first, then the issues, each in the order Comic Vine gave them; only what was asked for, only by a
    // number that can be one.
    let (mut series, mut issues) = (Vec::new(), Vec::new());
    for result in results.iter().take(RESULTS_LOOKED_AT) {
        let Some(id) = id_of(result) else { continue };
        match result.get("resource_type").and_then(Value::as_str) {
            Some("volume") => series.push(("volume", id, result)),
            Some("issue") => issues.push(("issue", id, result)),
            _ => {}
        }
    }
    let picked: Vec<(&str, u64, &Value)> = series
        .into_iter()
        .chain(issues)
        .take(MAX_CANDIDATES)
        .collect();

    // All the details at once, so the time is that of the slowest.
    let mut details = JoinSet::new();
    for (resource, id, _) in picked.iter().take(DETAILS_OF) {
        let prefix = if *resource == "volume" {
            SERIES_PREFIX
        } else {
            ISSUE_PREFIX
        };
        let (client, key, resource, id) = (client.clone(), key.to_owned(), *resource, *id);
        let url = format!("{api}/{resource}/{prefix}-{id}/");
        details.spawn(async move {
            let request = client.get(url).query(&[
                ("api_key", key.as_str()),
                ("format", "json"),
                ("field_list", "person_credits"),
            ]);
            let credits = get_json(request)
                .await
                .ok()
                .and_then(|body| answer(body).ok());
            ((resource, id), credits)
        });
    }
    // A detail that fails (or a task that dies) only leaves its candidate without it: the others are still waited for.
    let mut by_id: BTreeMap<(&str, u64), Value> = BTreeMap::new();
    while let Some(joined) = details.join_next().await {
        if let Ok((id, Some(credits))) = joined {
            by_id.insert(id, credits);
        }
    }

    let items: Vec<Value> = picked
        .iter()
        .map(|(resource, id, result)| item(resource, result, by_id.get(&(*resource, *id))))
        .collect();
    super::via_server::parse_comic_vine_answer(&json!({ "candidates": items }))
        .map_err(|()| ProviderError::Unexpected("comic vine".to_owned()))
}

/// The answer, if its own status code says it went well: `100` is a key it does not accept, even with a success status.
fn answer(body: Value) -> Result<Value, ProviderError> {
    match body.get("status_code").and_then(Value::as_i64) {
        Some(1) => Ok(body),
        Some(100) => Err(ProviderError::BadKey),
        _ => Err(ProviderError::Unexpected("comic vine".to_owned())),
    }
}

fn id_of(result: &Value) -> Option<u64> {
    result
        .get("id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))
}

/// What a person did, as Comic Vine writes the roles of a credit (`writer, penciler`), lower case and without blanks.
fn roles(role: &Value) -> Vec<String> {
    role.as_str()
        .filter(|role| role.len() <= 200)
        .map(|role| {
            role.split([',', ';', '&', '/'])
                .map(|part| part.trim().to_lowercase())
                .filter(|part| !part.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The names of those credited with one of these roles.
fn people(credits: Option<&Value>, wanted: &[&str]) -> Vec<Value> {
    credits
        .and_then(|body| body.pointer("/results/person_credits"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .take(400)
                .filter(|person| {
                    person.get("role").is_some_and(|role| {
                        roles(role).iter().any(|r| wanted.contains(&r.as_str()))
                    })
                })
                .filter_map(|person| person.get("name").cloned())
                .collect()
        })
        .unwrap_or_default()
}

/// A year from `1940` or from the front of `1940-04-01`.
fn year(value: Option<&Value>) -> Option<i64> {
    value?.as_str()?.get(..4)?.parse().ok()
}

/// The medium image, or else the small one; never the original, which can be huge.
fn cover(result: &Value) -> Option<&str> {
    ["medium_url", "small_url"].iter().find_map(|size| {
        result
            .pointer(&format!("/image/{size}"))
            .and_then(Value::as_str)
    })
}

/// Comic Vine's result and credits in the fixed shape of the server's answer; the reader cuts and checks the rest.
fn item(resource: &str, result: &Value, credits: Option<&Value>) -> Value {
    json!({
        "resource": resource,
        "external_id": result.get("id"),
        "title": result.get("name"),
        "series": result.pointer("/volume/name"),
        "number": result.get("issue_number"),
        "year": if resource == "volume" {
            year(result.get("start_year"))
        } else {
            year(result.get("cover_date")).or_else(|| year(result.get("store_date")))
        },
        "publisher": result.pointer("/publisher/name"),
        "cover_url": cover(result),
        "writers": people(credits, WRITER_ROLES),
        "artists": people(credits, ARTIST_ROLES),
    })
}
