//! The board game of a barcode, asked of GameUPC through the server (D62), and the game a person chose sent back. The
//! barcode goes only where it must: never without a session, never to a server that does not say it offers GameUPC,
//! never when GameUPC is turned off, and there is no key of their own to go around the server. A choice follows the
//! same rules.

use std::collections::BTreeSet;

use super::*;
use crate::metadata::gameupc::{GameUpcAnswer, GameUpcStatus, lookup, send_choice};
use crate::metadata::via_server::GoogleAccess;
use crate::metadata::{Candidate, Lookup, ProviderError, Source};

fn game(title: &str, id: &str) -> Candidate {
    let mut candidate = Candidate::new(Kind::Boardgame, Source::GameUpc, title);
    candidate.external_id = Some(id.to_owned());
    candidate.barcode = Some("019962194719".to_owned());
    candidate
}

fn verified() -> GameUpcAnswer {
    GameUpcAnswer {
        status: GameUpcStatus::Verified,
        candidates: vec![game("Gloomhaven", "174430")],
    }
}

fn guesses() -> GameUpcAnswer {
    GameUpcAnswer {
        status: GameUpcStatus::Choose,
        candidates: vec![
            game("Tiny Towns", "265736"),
            game("Tiny Towns: Fortune", "287576"),
        ],
    }
}

fn unknown() -> GameUpcAnswer {
    GameUpcAnswer {
        status: GameUpcStatus::Unknown,
        candidates: vec![],
    }
}

/// Every other source is off, so nothing here reaches the network.
fn all_off_but(source: Source) -> BTreeSet<String> {
    Source::ALL
        .iter()
        .filter(|candidate| **candidate != source)
        .map(|candidate| candidate.id().to_owned())
        .collect()
}

// --- Who is asked, and with what ------------------------------------------------------------------------------------

#[tokio::test]
async fn without_a_session_the_barcode_goes_nowhere() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(verified())]);

    let answer = lookup::<FakeApi>(None, "019962194719").await.unwrap();

    assert_eq!(answer, None);
    assert!(cloud.gameupc_asks().is_empty());
    assert_eq!(cloud.0.lock().unwrap().offer_checks, 0);
}

#[tokio::test]
async fn a_server_that_does_not_offer_gameupc_never_gets_the_barcode_or_a_choice() {
    for offered in [
        vec![],
        vec!["bgg"],
        vec!["google_books", "tmdb", "rawg", "comic_vine"],
        vec!["gameupc "],
        vec!["GameUPC"],
        vec!["game_upc"],
    ] {
        let cloud = FakeCloud::default();
        cloud.0.lock().unwrap().metadata = offered.iter().map(|p| (*p).to_owned()).collect();
        cloud.0.lock().unwrap().gameupc_answers = vec![Ok(verified())].into();
        let api = cloud.api("ana");

        assert_eq!(
            lookup(Some(&api), "019962194719").await.unwrap(),
            None,
            "{offered:?}"
        );
        assert!(
            !send_choice(&api, "222222222224", 265736).await,
            "{offered:?}"
        );
        assert!(cloud.gameupc_asks().is_empty(), "{offered:?}");
        assert!(cloud.gameupc_choices().is_empty(), "{offered:?}");
    }
}

#[tokio::test]
async fn not_being_able_to_read_what_the_server_offers_sends_nothing() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(verified())]);
    cloud.0.lock().unwrap().offer_fails = true;
    let api = cloud.api("ana");

    assert_eq!(lookup(Some(&api), "019962194719").await.unwrap(), None);
    assert!(!send_choice(&api, "222222222224", 265736).await);
    assert!(cloud.gameupc_asks().is_empty());
    assert!(cloud.gameupc_choices().is_empty());
}

#[tokio::test]
async fn a_server_that_offers_it_gets_the_barcode_and_the_choice_and_nothing_else() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(guesses())]);
    let api = cloud.api("ana");

    let answer = lookup(Some(&api), "222222222224").await.unwrap().unwrap();
    let sent = send_choice(&api, "222222222224", 265736).await;

    assert_eq!(answer.status, GameUpcStatus::Choose);
    assert!(sent);
    assert_eq!(cloud.gameupc_asks(), vec!["222222222224".to_owned()]);
    assert_eq!(
        cloud.gameupc_choices(),
        vec![("222222222224".to_owned(), 265736)]
    );
}

#[tokio::test]
async fn a_server_that_fails_is_a_warning_and_a_quota_refusal_is_told_as_quota() {
    for code in ["server_unavailable", "metadata_failed", "invalid_token"] {
        let cloud = FakeCloud::default();
        cloud.with_gameupc(vec![Err(code)]);
        let api = cloud.api("ana");

        assert!(
            matches!(
                lookup(Some(&api), "019962194719").await,
                Err(ProviderError::Unexpected(_))
            ),
            "{code}"
        );
    }
    for code in ["metadata_quota_exhausted", "too_many_requests"] {
        let cloud = FakeCloud::default();
        cloud.with_gameupc(vec![Err(code)]);
        let api = cloud.api("ana");

        assert!(
            matches!(
                lookup(Some(&api), "019962194719").await,
                Err(ProviderError::Quota)
            ),
            "{code}"
        );
    }
}

// --- What a scanned code becomes ------------------------------------------------------------------------------------

#[tokio::test]
async fn a_sure_game_makes_the_code_a_board_game() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(verified())]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();

    let result = lookup
        .lookup_code(
            "019962194719",
            GoogleAccess::<FakeApi>::Anonymous,
            Some(&api),
            &all_off_but(Source::GameUpc),
        )
        .await;

    assert_eq!(result.kind, Some(Kind::Boardgame));
    assert_eq!(result.game_upc, Some(GameUpcStatus::Verified));
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.candidates[0].title, "Gloomhaven");
}

#[tokio::test]
async fn guesses_make_the_code_a_board_game_to_choose_from() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(guesses())]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();

    let result = lookup
        .lookup_code(
            "222222222224",
            GoogleAccess::<FakeApi>::Anonymous,
            Some(&api),
            &all_off_but(Source::GameUpc),
        )
        .await;

    assert_eq!(result.kind, Some(Kind::Boardgame));
    assert_eq!(result.game_upc, Some(GameUpcStatus::Choose));
    assert_eq!(result.candidates.len(), 2);
}

#[tokio::test]
async fn nothing_known_leaves_the_kind_open_and_says_gameupc_was_asked() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(unknown())]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();

    let result = lookup
        .lookup_code(
            "333333333331",
            GoogleAccess::<FakeApi>::Anonymous,
            Some(&api),
            &all_off_but(Source::GameUpc),
        )
        .await;

    assert_eq!(result.kind, None);
    assert_eq!(result.game_upc, Some(GameUpcStatus::Unknown));
    assert!(result.candidates.is_empty());
}

#[tokio::test]
async fn turned_off_it_is_never_asked_and_says_nothing() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(verified())]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();
    let off: BTreeSet<String> = Source::ALL.iter().map(|s| s.id().to_owned()).collect();

    let result = lookup
        .lookup_code(
            "019962194719",
            GoogleAccess::<FakeApi>::Anonymous,
            Some(&api),
            &off,
        )
        .await;

    assert_eq!(result.kind, None);
    assert_eq!(result.game_upc, None);
    assert!(cloud.gameupc_asks().is_empty());
    assert_eq!(cloud.0.lock().unwrap().offer_checks, 0);
}

#[tokio::test]
async fn an_isbn_is_never_sent_to_gameupc() {
    let cloud = FakeCloud::default();
    cloud.with_gameupc(vec![Ok(verified())]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();
    let off: BTreeSet<String> = Source::ALL.iter().map(|s| s.id().to_owned()).collect();

    let result = lookup
        .lookup_code(
            "9780140328721",
            GoogleAccess::<FakeApi>::Anonymous,
            Some(&api),
            &off,
        )
        .await;

    assert_eq!(result.game_upc, None);
    assert!(cloud.gameupc_asks().is_empty());
}
