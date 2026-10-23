//! GameUPC (gameupc.com): which BoardGameGeek game the barcode of a box is, asked through the server of the account
//! (D62). Only through the server: GameUPC gives keys to whoever makes the app, never to the people using it, so there
//! is no key of their own. Only the barcode leaves the app, and only to a server that says it offers GameUPC.
//!
//! GameUPC answers in three ways, and asks apps to handle all three: it is sure (`Verified`, one game), it has guesses
//! for the person to choose from (`Choose`), or it knows nothing (`Unknown`, the person searches). When the person picks
//! the game in the last two, the app sends that choice back (`send_choice`): that is how GameUPC learns.

use std::time::Duration;

use kosmotheka_domain::{Format, Kind};
use serde::Serialize;
use serde_json::Value;

use super::bgg::{is_cover, whole_text};
use super::via_server::server_error;
use super::{Candidate, ProviderError, Source};
use crate::cloud::CloudApi;

/// The provider's id in `GET /v1/server`'s `metadata` list.
pub const GAMEUPC: &str = "gameupc";

const PATIENCE: Duration = Duration::from_secs(10);
const MAX_CANDIDATES: usize = 8;
const MAX_TEXT: usize = 300;
/// The biggest number a server may give for a game, as the server itself allows it.
const MAX_ID: u64 = 1_000_000_000;

/// How sure GameUPC is of a barcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameUpcStatus {
    /// One game, checked by people.
    Verified,
    /// Its guesses: the person chooses one, or searches for the game.
    Choose,
    /// Nothing: the person searches for the game.
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameUpcAnswer {
    pub status: GameUpcStatus,
    /// One when verified, the guesses when choosing, none when unknown. Each carries the barcode.
    pub candidates: Vec<Candidate>,
}

/// Asks through the server of the account. `Ok(None)` when nothing was asked: no session, or a server that does not
/// say it offers GameUPC. A server that fails or is slow is a warning, as for any provider.
pub async fn lookup<C: CloudApi>(
    cloud: Option<&C>,
    code: &str,
) -> Result<Option<GameUpcAnswer>, ProviderError> {
    let Some(cloud) = cloud else {
        return Ok(None);
    };
    let asked = tokio::time::timeout(PATIENCE, async {
        if !cloud.offers_metadata(GAMEUPC).await.unwrap_or(false) {
            return Ok(None);
        }
        cloud.gameupc_lookup(code).await.map(Some)
    })
    .await;
    match asked {
        Ok(Ok(answer)) => Ok(answer),
        Ok(Err(error)) => Err(server_error(&error)),
        Err(_) => Err(ProviderError::Unexpected("server too slow".to_owned())),
    }
}

/// Sends the game a person chose for a barcode. Best effort: the book is already in the catalogue, and a choice that
/// does not reach GameUPC only means it learns nothing this time.
pub async fn send_choice<C: CloudApi>(cloud: &C, code: &str, external_id: u64) -> bool {
    tokio::time::timeout(PATIENCE, async {
        cloud.offers_metadata(GAMEUPC).await.unwrap_or(false)
            && cloud.gameupc_choice(code, external_id).await.is_ok()
    })
    .await
    .unwrap_or(false)
}

/// Reads the server's answer: a status and at most eight games of the fixed shape, each field checked. `Err` when it
/// is not the shape at all (the caller treats that as a failed server).
pub fn parse_answer(body: &Value, code: &str) -> Result<GameUpcAnswer, ()> {
    let status = match body.get("status").and_then(Value::as_str) {
        Some("verified") => GameUpcStatus::Verified,
        Some("choose") => GameUpcStatus::Choose,
        Some("unknown") => GameUpcStatus::Unknown,
        _ => return Err(()),
    };
    let candidates: Vec<Candidate> = body
        .get("candidates")
        .and_then(Value::as_array)
        .ok_or(())?
        .iter()
        .take(10 * MAX_CANDIDATES)
        .filter_map(|item| candidate(item, code))
        .take(match status {
            GameUpcStatus::Verified => 1,
            GameUpcStatus::Choose => MAX_CANDIDATES,
            GameUpcStatus::Unknown => 0,
        })
        .collect();
    // A status the games do not back is no status: nothing sure, nothing to choose from.
    let status = if candidates.is_empty() {
        GameUpcStatus::Unknown
    } else {
        status
    };
    Ok(GameUpcAnswer { status, candidates })
}

fn candidate(item: &Value, code: &str) -> Option<Candidate> {
    let id = item
        .get("external_id")?
        .as_u64()
        .filter(|id| (1..=MAX_ID).contains(id))?;
    let title = whole_text(item.get("title"), MAX_TEXT)?;

    let mut candidate = Candidate::new(Kind::Boardgame, Source::GameUpc, title);
    candidate.external_id = Some(id.to_string());
    candidate.cover_url = item
        .get("cover_url")
        .and_then(Value::as_str)
        .filter(|url| is_cover(url))
        .map(str::to_owned);
    candidate.barcode = Some(code.to_owned());
    candidate.format = Some(Format::Box);
    Some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const COVER: &str = "https://cf.geekdo-images.com/sZYp_3BTDGjh2unaZfZmuA__original/img/7d-lj5Gd1e8PFnD97LYFah2c45M=/0x0/filters:format(jpeg)/pic2437871.jpg";

    #[test]
    fn reads_a_verified_game_with_its_barcode() {
        let answer = parse_answer(
            &json!({ "status": "verified", "candidates": [{ "external_id": 174430, "title": "Gloomhaven", "cover_url": COVER }] }),
            "019962194719",
        )
        .unwrap();

        assert_eq!(answer.status, GameUpcStatus::Verified);
        let game = &answer.candidates[0];
        assert_eq!(game.kind, Kind::Boardgame);
        assert_eq!(game.source, Source::GameUpc);
        assert_eq!(game.title, "Gloomhaven");
        assert_eq!(game.external_id.as_deref(), Some("174430"));
        assert_eq!(game.cover_url.as_deref(), Some(COVER));
        assert_eq!(game.barcode.as_deref(), Some("019962194719"));
        assert_eq!(game.format, Some(Format::Box));
    }

    #[test]
    fn reads_guesses_to_choose_from() {
        let answer = parse_answer(
            &json!({ "status": "choose", "candidates": [
                { "external_id": 265736, "title": "Tiny Towns", "cover_url": null },
                { "external_id": 287576, "title": "Tiny Towns: Fortune" },
            ] }),
            "222222222224",
        )
        .unwrap();

        assert_eq!(answer.status, GameUpcStatus::Choose);
        assert_eq!(answer.candidates.len(), 2);
        assert!(
            answer
                .candidates
                .iter()
                .all(|c| c.barcode.as_deref() == Some("222222222224"))
        );
    }

    #[test]
    fn nothing_known_is_unknown() {
        let answer = parse_answer(
            &json!({ "status": "unknown", "candidates": [] }),
            "333333333331",
        )
        .unwrap();

        assert_eq!(answer.status, GameUpcStatus::Unknown);
        assert!(answer.candidates.is_empty());
    }

    #[test]
    fn a_status_without_a_usable_game_is_unknown() {
        let answer = parse_answer(
            &json!({ "status": "verified", "candidates": [{ "external_id": "174430", "title": "Text id" }, { "external_id": 1, "title": "" }] }),
            "019962194719",
        )
        .unwrap();

        assert_eq!(answer.status, GameUpcStatus::Unknown);
        assert!(answer.candidates.is_empty());
    }

    #[test]
    fn a_verified_answer_keeps_one_game_and_guesses_keep_eight() {
        let games: Vec<Value> = (1..=20)
            .map(|id| json!({ "external_id": id, "title": format!("Game {id}") }))
            .collect();

        let verified = parse_answer(
            &json!({ "status": "verified", "candidates": games }),
            "96385074",
        )
        .unwrap();
        let guesses = parse_answer(
            &json!({ "status": "choose", "candidates": games }),
            "96385074",
        )
        .unwrap();

        assert_eq!(verified.candidates.len(), 1);
        assert_eq!(guesses.candidates.len(), 8);
    }

    #[test]
    fn a_cover_from_elsewhere_and_ids_that_cannot_be_real_are_dropped() {
        let answer = parse_answer(
            &json!({ "status": "choose", "candidates": [
                { "external_id": 1, "title": "Cover elsewhere", "cover_url": "https://evil.example/pic.jpg" },
                { "external_id": 0, "title": "Id zero" },
                { "external_id": 10_000_000_000_u64, "title": "Id too big" },
                { "external_id": -3, "title": "Negative" },
                { "external_id": 2, "title": "Hidden\u{202E}mark" },
            ] }),
            "96385074",
        )
        .unwrap();

        assert_eq!(answer.candidates.len(), 1);
        assert_eq!(answer.candidates[0].title, "Cover elsewhere");
        assert_eq!(answer.candidates[0].cover_url, None);
    }

    #[test]
    fn an_answer_without_the_shape_is_a_failure() {
        assert!(parse_answer(&json!({ "candidates": [] }), "96385074").is_err());
        assert!(parse_answer(&json!({ "status": "maybe", "candidates": [] }), "96385074").is_err());
        assert!(parse_answer(&json!({ "status": "verified" }), "96385074").is_err());
        assert!(parse_answer(&json!("verified"), "96385074").is_err());
    }
}
