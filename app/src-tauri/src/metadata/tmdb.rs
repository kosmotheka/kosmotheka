//! TMDB asked directly with the person's own «API Read Access Token» (7.8, D62), with no server in between. What it
//! sends is as little to be trusted as what an account's server sends, so it is turned into the fixed shape the
//! server's answer has and goes through the same reader (`via_server::parse_tmdb_answer`): eight candidates, texts
//! cut, covers only from TMDB's image host, numbers that can be. The token travels in a header, and no error carries
//! an address or anything TMDB said.

use std::collections::BTreeMap;

use kosmotheka_domain::Kind;
use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::{Candidate, ProviderError, get_json};

pub const API: &str = "https://api.themoviedb.org/3";

/// The results looked at to find the eight that are well formed, as the server does.
const RESULTS_LOOKED_AT: usize = 40;
const MAX_CANDIDATES: usize = 8;
const MAX_ID: u64 = 1_000_000_000;
/// Where the posters live; the path TMDB gives is put after it and the reader checks the whole address.
const IMAGES: &str = "https://image.tmdb.org/t/p/w500";

pub async fn search(
    client: &reqwest::Client,
    api: &str,
    token: &str,
    kind: Kind,
    query: &str,
    language: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let path = path_of(kind);
    let body = get_json(
        client
            .get(format!("{api}/search/{path}"))
            .bearer_auth(token)
            .query(&[
                ("query", query),
                ("include_adult", "false"),
                ("language", language),
                ("page", "1"),
            ]),
    )
    .await?;
    let results = body
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::Unexpected("tmdb".to_owned()))?;

    // Only numbers TMDB itself gave, and that can be one, are put in an address.
    let picked: Vec<&Value> = results
        .iter()
        .take(RESULTS_LOOKED_AT)
        .filter(|result| id_of(result).is_some())
        .take(MAX_CANDIDATES)
        .collect();

    // All the details at once, so the time is that of the slowest.
    let mut details = JoinSet::new();
    for result in &picked {
        let Some(id) = id_of(result) else { continue };
        let (client, token, language) = (client.clone(), token.to_owned(), language.to_owned());
        let url = format!("{api}/{path}/{id}");
        let credits = kind != Kind::TvSeries;
        details.spawn(async move {
            let mut request = client
                .get(url)
                .bearer_auth(token)
                .query(&[("language", language.as_str())]);
            if credits {
                request = request.query(&[("append_to_response", "credits")]);
            }
            (id, get_json(request).await.ok())
        });
    }
    let mut by_id: BTreeMap<u64, Value> = BTreeMap::new();
    // A detail that fails (or a task that dies) only leaves its candidate without it: the others are still waited for.
    while let Some(joined) = details.join_next().await {
        if let Ok((id, Some(detail))) = joined {
            by_id.insert(id, detail);
        }
    }

    let candidates: Vec<Value> = picked
        .iter()
        .map(|result| candidate(kind, result, id_of(result).and_then(|id| by_id.get(&id))))
        .collect();
    super::via_server::parse_tmdb_answer(&json!({ "candidates": candidates }), kind)
        .map_err(|()| ProviderError::Unexpected("tmdb".to_owned()))
}

fn path_of(kind: Kind) -> &'static str {
    if kind == Kind::TvSeries {
        "tv"
    } else {
        "movie"
    }
}

fn id_of(result: &Value) -> Option<u64> {
    result
        .get("id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))
}

/// TMDB's result and detail in the fixed shape of the server's answer; the reader cuts and checks the rest.
fn candidate(kind: Kind, result: &Value, detail: Option<&Value>) -> Value {
    let series = kind == Kind::TvSeries;
    let (title, original, date) = if series {
        ("name", "original_name", "first_air_date")
    } else {
        ("title", "original_title", "release_date")
    };
    let year = result
        .get(date)
        .and_then(Value::as_str)
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<i64>().ok());
    let names = |people: Option<&Value>, job: Option<&str>| -> Vec<Value> {
        people
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter(|person| {
                        job.is_none_or(|job| person.get("job").and_then(Value::as_str) == Some(job))
                    })
                    .filter_map(|person| person.get("name").cloned())
                    .collect()
            })
            .unwrap_or_default()
    };
    json!({
        "external_id": result.get("id"),
        "title": result.get(title),
        "original_title": result.get(original),
        "year": year,
        "language": result.get("original_language"),
        "cover_url": result.get("poster_path").and_then(Value::as_str).map(|path| format!("{IMAGES}{path}")),
        "directors": names(detail.and_then(|d| d.pointer("/credits/crew")), Some("Director")),
        "creators": names(detail.and_then(|d| d.get("created_by")), None),
        "duration_minutes": detail.and_then(|d| d.get("runtime")),
        "seasons": detail.and_then(|d| d.get("number_of_seasons")),
    })
}
