//! RAWG asked directly with the person's own key (7.8, D62), with no server in between. RAWG only takes its key as
//! the `key` parameter of the address, so here the key is in every address, and no error may carry it: `get_json`
//! strips the address from every failure and never passes on anything RAWG said. What RAWG sends is turned into the
//! fixed shape of the server's answer and goes through the same reader (`via_server::parse_rawg_answer`).

use std::collections::BTreeMap;

use serde_json::{Value, json};
use tokio::task::JoinSet;

use super::{Candidate, ProviderError, get_json};

pub const API: &str = "https://api.rawg.io/api";

const RESULTS_LOOKED_AT: usize = 40;
const MAX_CANDIDATES: usize = 8;
/// Only the first results get a second call, as on the server.
const DETAILS_OF: usize = 3;
const MAX_ID: u64 = 1_000_000_000;

pub async fn search(
    client: &reqwest::Client,
    api: &str,
    key: &str,
    query: &str,
) -> Result<Vec<Candidate>, ProviderError> {
    let body = get_json(client.get(format!("{api}/games")).query(&[
        ("search", query),
        ("page_size", "8"),
        ("key", key),
    ]))
    .await?;
    let results = body
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| ProviderError::Unexpected("rawg".to_owned()))?;

    let picked: Vec<&Value> = results
        .iter()
        .take(RESULTS_LOOKED_AT)
        .filter(|result| id_of(result).is_some())
        .take(MAX_CANDIDATES)
        .collect();

    let mut details = JoinSet::new();
    for result in picked.iter().take(DETAILS_OF) {
        let Some(id) = id_of(result) else { continue };
        let (client, key) = (client.clone(), key.to_owned());
        let url = format!("{api}/games/{id}");
        details.spawn(async move {
            (
                id,
                get_json(client.get(url).query(&[("key", key.as_str())]))
                    .await
                    .ok(),
            )
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
        .map(|result| candidate(result, id_of(result).and_then(|id| by_id.get(&id))))
        .collect();
    super::via_server::parse_rawg_answer(&json!({ "candidates": candidates }))
        .map_err(|()| ProviderError::Unexpected("rawg".to_owned()))
}

fn id_of(result: &Value) -> Option<u64> {
    result
        .get("id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))
}

/// RAWG's result and detail in the fixed shape of the server's answer; the reader cuts and checks the rest.
fn candidate(result: &Value, detail: Option<&Value>) -> Value {
    let year = result
        .get("released")
        .and_then(Value::as_str)
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<i64>().ok());
    let names = |list: Option<&Value>, field: &str| -> Vec<Value> {
        list.and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get(field).cloned())
                    .collect()
            })
            .unwrap_or_default()
    };
    let platforms: Vec<Value> = result
        .get("platforms")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|entry| entry.pointer("/platform/name").cloned())
                .collect()
        })
        .unwrap_or_default();
    json!({
        "external_id": result.get("id"),
        "title": result.get("name"),
        "year": year,
        "cover_url": result.get("background_image"),
        "platforms": platforms,
        "developers": names(detail.and_then(|d| d.get("developers")), "name"),
        "publishers": names(detail.and_then(|d| d.get("publishers")), "name"),
    })
}
