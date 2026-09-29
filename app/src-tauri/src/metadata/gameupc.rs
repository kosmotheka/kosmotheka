//! GameUPC (gameupc.com): board games by barcode, resolved against BoardGameGeek.
//! Every endpoint needs a key, sent as the `x-api-key` header (D62): without one this
//! provider is simply never called (see `Lookup::lookup_non_isbn_code`).
//! https://gameupc.com/gameupc-oas.yaml

use kosmotheka_domain::{Format, Kind};
use reqwest::StatusCode;
use serde_json::Value;

use super::{Candidate, ProviderError, Source, text, year_in};

const BASE: &str = "https://api.gameupc.com/v1";

/// A scanned barcode (UPC/EAN). `None` when GameUPC has nothing verified enough
/// to trust (ambiguous or unknown).
pub async fn lookup(
    client: &reqwest::Client,
    barcode: &str,
    api_key: &str,
) -> Result<Option<Candidate>, ProviderError> {
    let response = client
        .get(format!("{BASE}/upc/{barcode}"))
        .header("x-api-key", api_key)
        .send()
        .await?;
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(ProviderError::BadKey),
        StatusCode::TOO_MANY_REQUESTS => return Err(ProviderError::Quota),
        status if !status.is_success() => {
            return Err(ProviderError::Unexpected(format!("HTTP {status}")));
        }
        _ => {}
    }
    let body: Value = response.json().await?;
    Ok(parse_upc_info(&body))
}

/// Only a `verified` match becomes a candidate: anything else means the game
/// is ambiguous and GameUPC itself is asking for a human pick, which this app
/// does not support (the person can still search by title instead).
pub fn parse_upc_info(body: &Value) -> Option<Candidate> {
    if body.get("bgg_info_status").and_then(Value::as_str) != Some("verified") {
        return None;
    }
    let game = body.get("bgg_info")?.get(0)?;
    let title = text(game.get("name").and_then(Value::as_str))?;

    let mut candidate = Candidate::new(Kind::Boardgame, Source::GameUpc, title);
    candidate.published_year = game
        .get("published")
        .and_then(Value::as_str)
        .and_then(year_in);
    candidate.cover_url = text(game.get("image_url").and_then(Value::as_str))
        .or_else(|| text(game.get("thumbnail_url").and_then(Value::as_str)));
    candidate.barcode = text(body.get("upc").and_then(Value::as_str));
    candidate.external_id = game
        .get("id")
        .and_then(Value::as_i64)
        .map(|id| id.to_string());
    candidate.format = Some(Format::Box);
    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real response for upc=019962194719 (Gloomhaven), recorded on 2026-09-29
    // with the public test key from https://gameupc.com; trimmed to bgg_info[0]
    // and stripped of its `versions` (not used here). The key itself is not in
    // this fixture.
    const VERIFIED: &str = include_str!("../../tests/fixtures/gameupc-verified-019962194719.json");
    // Real response for the OpenAPI doc's own "needs user input" example
    // (upc=222222222224, Tiny Towns), same recording; a second bgg_info entry
    // was hand-added (not from the live response) to exercise "more than one
    // candidate, still ambiguous".
    const AMBIGUOUS: &str =
        include_str!("../../tests/fixtures/gameupc-ambiguous-222222222224.json");
    // Hand-built (no barcode returned this from the live test server): an
    // empty `bgg_info` array, the shape the OpenAPI schema allows for "ok" but
    // nothing found.
    const NOT_FOUND: &str = include_str!("../../tests/fixtures/gameupc-not-found.json");

    #[test]
    fn reads_a_verified_match() {
        let candidate = parse_upc_info(&serde_json::from_str(VERIFIED).unwrap()).unwrap();

        assert_eq!(candidate.kind, Kind::Boardgame);
        assert_eq!(candidate.source, Source::GameUpc);
        assert_eq!(candidate.title, "Gloomhaven");
        assert_eq!(candidate.published_year, Some(2017));
        assert_eq!(
            candidate.cover_url.as_deref(),
            Some(
                "https://cf.geekdo-images.com/sZYp_3BTDGjh2unaZfZmuA__original/img/7d-lj5Gd1e8PFnD97LYFah2c45M=/0x0/filters:format(jpeg)/pic2437871.jpg"
            )
        );
        assert_eq!(candidate.barcode.as_deref(), Some("019962194719"));
        assert_eq!(candidate.external_id.as_deref(), Some("174430"));
        assert_eq!(candidate.format, Some(Format::Box));
        assert_eq!(
            candidate.players_min, None,
            "GameUPC does not report player counts"
        );
    }

    #[test]
    fn an_ambiguous_match_is_not_a_candidate() {
        let candidate = parse_upc_info(&serde_json::from_str(AMBIGUOUS).unwrap());
        assert!(candidate.is_none());
    }

    #[test]
    fn no_match_is_not_a_candidate() {
        let candidate = parse_upc_info(&serde_json::from_str(NOT_FOUND).unwrap());
        assert!(candidate.is_none());
    }

    #[test]
    fn a_missing_status_is_not_a_candidate() {
        assert!(parse_upc_info(&serde_json::json!({})).is_none());
    }
}

/// Hits the real service. Not run by default: `GAMEUPC_API_KEY=... cargo test -p kosmotheka
/// -- --ignored --nocapture live_gameupc`
#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "needs network and a key"]
    async fn live_gameupc_lookup() {
        let Ok(api_key) = std::env::var("GAMEUPC_API_KEY") else {
            return;
        };
        let client = super::super::http_client().unwrap();

        let candidate = lookup(&client, "019962194719", &api_key).await.unwrap();

        println!("gameupc: {candidate:#?}");
        assert!(candidate.is_some());
    }
}
