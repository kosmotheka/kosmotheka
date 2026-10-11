//! Video games searched in RAWG through the server (7.8, D62). As with TMDB, the text a person searches for goes only
//! where it must: never without a session, never for another kind, never to a server that does not say it offers
//! the search; and what comes back is somebody else's, so it is cut to eight candidates of the fixed shape, with
//! covers only from RAWG's image host.

use kosmotheka_domain::ContributorRole;
use serde_json::{Value, json};

use super::*;
use crate::metadata::via_server::{SearchAccess, parse_rawg_answer, search_rawg as search_with};
use crate::metadata::{Candidate, ProviderError, Source};

fn game(title: &str) -> Candidate {
    let mut candidate = Candidate::new(Kind::Videogame, Source::Rawg, title);
    candidate.external_id = Some("3498".to_owned());
    candidate
}

/// The server of the account, or nobody; the own key has its own tests (`own_key_tests`). It must never be used here.
async fn search_rawg<C: CloudApi>(
    cloud: Option<&C>,
    query: &str,
) -> std::result::Result<Vec<Candidate>, ProviderError> {
    let access = cloud.map_or(SearchAccess::Nothing, SearchAccess::Server);
    search_with(access, query, |_key| async {
        panic!("the own key is not in play")
    })
    .await
}

// --- Who is asked, and with what ------------------------------------------------------------------------------------

#[tokio::test]
async fn without_a_session_nothing_goes_anywhere() {
    let cloud = FakeCloud::default();
    cloud.with_rawg(vec![Ok(vec![game("GTA")])]);

    let found = search_rawg::<FakeApi>(None, "gta").await.unwrap();

    assert!(found.is_empty());
    assert!(cloud.rawg_asks().is_empty());
    assert_eq!(cloud.0.lock().unwrap().offer_checks, 0);
}

#[tokio::test]
async fn a_server_that_does_not_offer_rawg_never_gets_the_text() {
    for offered in [
        vec![],
        vec!["google_books"],
        vec!["tmdb"],
        vec!["rawg "],
        vec!["RAWG"],
    ] {
        let cloud = FakeCloud::default();
        cloud.0.lock().unwrap().metadata = offered.iter().map(|p| (*p).to_owned()).collect();
        cloud.0.lock().unwrap().rawg_answers = vec![Ok(vec![game("GTA")])].into();
        let api = cloud.api("ana");

        let found = search_rawg(Some(&api), "gta").await.unwrap();

        assert!(found.is_empty(), "{offered:?}");
        assert!(cloud.rawg_asks().is_empty(), "{offered:?}");
    }
}

#[tokio::test]
async fn not_being_able_to_read_what_the_server_offers_sends_nothing() {
    let cloud = FakeCloud::default();
    cloud.with_rawg(vec![Ok(vec![game("GTA")])]);
    cloud.0.lock().unwrap().offer_fails = true;
    let api = cloud.api("ana");

    let found = search_rawg(Some(&api), "gta").await.unwrap();

    assert!(found.is_empty());
    assert!(cloud.rawg_asks().is_empty());
}

#[tokio::test]
async fn a_server_that_offers_it_gets_the_text_and_nothing_else_and_answers() {
    let cloud = FakeCloud::default();
    cloud.with_rawg(vec![Ok(vec![game("GTA")])]);
    let api = cloud.api("ana");

    let found = search_rawg(Some(&api), "  grand theft auto ")
        .await
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(cloud.rawg_asks(), vec!["grand theft auto".to_owned()]);
}

#[tokio::test]
async fn a_server_that_fails_is_a_warning_and_never_an_empty_success() {
    for code in [
        "server_unavailable",
        "metadata_quota_exhausted",
        "metadata_failed",
        "too_many_requests",
        "invalid_token",
    ] {
        let cloud = FakeCloud::default();
        cloud.with_rawg(vec![Err(code)]);
        let api = cloud.api("ana");

        assert!(search_rawg(Some(&api), "gta").await.is_err(), "{code}");
    }
}

#[tokio::test]
async fn a_quota_refusal_is_told_as_quota() {
    let cloud = FakeCloud::default();
    cloud.with_rawg(vec![Err("metadata_quota_exhausted")]);
    let api = cloud.api("ana");

    assert!(matches!(
        search_rawg(Some(&api), "gta").await,
        Err(ProviderError::Quota)
    ));
}

// --- What the server says -------------------------------------------------------------------------------------------

fn answer(candidates: Vec<Value>) -> Value {
    json!({ "candidates": candidates })
}

fn full_game() -> Value {
    json!({
        "external_id": 3498, "title": "Grand Theft Auto V", "year": 2013,
        "cover_url": "https://media.rawg.io/media/games/456/456dea5e1c7e3cd07060c14e96612001.jpg",
        "platforms": ["PC"], "developers": ["Rockstar North"], "publishers": ["Rockstar Games"],
    })
}

#[test]
fn a_game_is_read_as_a_candidate() {
    let found = parse_rawg_answer(&answer(vec![full_game()])).unwrap();

    assert_eq!(found.len(), 1);
    let candidate = &found[0];
    assert_eq!(candidate.kind, Kind::Videogame);
    assert_eq!(candidate.source, Source::Rawg);
    assert_eq!(candidate.title, "Grand Theft Auto V");
    assert_eq!(candidate.external_id.as_deref(), Some("3498"));
    assert_eq!(candidate.published_year, Some(2013));
    assert_eq!(
        candidate.cover_url.as_deref(),
        Some("https://media.rawg.io/media/games/456/456dea5e1c7e3cd07060c14e96612001.jpg")
    );
    assert_eq!(candidate.platform.as_deref(), Some("PC"));
    assert_eq!(candidate.publisher.as_deref(), Some("Rockstar Games"));
    let studios: Vec<_> = candidate
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(studios, vec![("Rockstar North", ContributorRole::Studio)]);
}

#[test]
fn the_platform_is_only_set_when_there_is_just_one() {
    let mut several = full_game();
    several["platforms"] = json!(["PC", "PlayStation 5"]);
    let mut none = full_game();
    none["platforms"] = json!([]);

    let found = parse_rawg_answer(&answer(vec![full_game(), several, none])).unwrap();

    assert_eq!(found[0].platform.as_deref(), Some("PC"));
    assert_eq!(found[1].platform, None);
    assert_eq!(found[2].platform, None);
}

#[test]
fn a_game_without_detail_has_no_studio_and_no_publisher() {
    let found = parse_rawg_answer(&answer(vec![json!({
        "external_id": 5, "title": "Plain", "platforms": ["PC"], "developers": [], "publishers": [],
    })]))
    .unwrap();

    assert!(found[0].contributors.is_empty());
    assert_eq!(found[0].publisher, None);
}

#[test]
fn a_list_of_a_hundred_is_cut_to_eight() {
    let many: Vec<Value> = (1..=100)
        .map(|n| json!({ "external_id": n, "title": format!("G{n}") }))
        .collect();

    let found = parse_rawg_answer(&answer(many)).unwrap();

    assert_eq!(found.len(), 8);
    assert_eq!(found[0].title, "G1");
    assert_eq!(found[7].title, "G8");
}

#[test]
fn an_answer_that_is_not_the_shape_is_a_failure() {
    for body in [
        json!(null),
        json!("text"),
        json!([1, 2]),
        json!({}),
        json!({ "candidates": "no" }),
        json!({ "candidates": {"a": 1} }),
        json!({ "items": [] }),
    ] {
        assert_eq!(parse_rawg_answer(&body), Err(()), "for {body}");
    }
}

#[test]
fn candidates_that_are_not_candidates_are_dropped() {
    let found = parse_rawg_answer(&answer(vec![
        json!("text"),
        json!(null),
        json!([1]),
        json!({ "external_id": "3498", "title": "string id" }),
        json!({ "external_id": -1, "title": "negative id" }),
        json!({ "external_id": 0, "title": "zero id" }),
        json!({ "external_id": 1.5, "title": "float id" }),
        json!({ "external_id": [1], "title": "array id" }),
        json!({ "title": "no id" }),
        json!({ "external_id": 2 }),
        json!({ "external_id": 3, "title": ["x"] }),
        json!({ "external_id": 4, "title": "   " }),
        json!({ "external_id": 5, "title": "fine" }),
    ]))
    .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "fine");
}

#[test]
fn fields_of_the_wrong_type_are_dropped_and_extra_ones_ignored() {
    let found = parse_rawg_answer(&answer(vec![json!({
        "external_id": 7, "title": "Odd", "year": "2013", "cover_url": 5, "platforms": "pc",
        "developers": 3, "publishers": {"a": 1}, "evil": "<script>", "nested": {"a": [1, 2]},
    })]))
    .unwrap();

    let candidate = &found[0];
    assert_eq!(candidate.title, "Odd");
    assert_eq!(candidate.published_year, None);
    assert_eq!(candidate.cover_url, None);
    assert_eq!(candidate.platform, None);
    assert!(candidate.contributors.is_empty());
    assert_eq!(candidate.publisher, None);
}

#[test]
fn enormous_fields_are_cut() {
    let people: Vec<String> = (0..100_000).map(|n| format!("Studio {n}")).collect();
    let platforms: Vec<String> = (0..100_000).map(|n| format!("Platform {n}")).collect();
    let found = parse_rawg_answer(&answer(vec![json!({
        "external_id": 7, "title": "T".repeat(1_000_000), "developers": people, "publishers": people,
        "platforms": platforms,
    })]))
    .unwrap();

    assert_eq!(found[0].title.chars().count(), 300);
    assert_eq!(found[0].contributors.len(), 5);
    assert_eq!(found[0].platform, None);
    assert!(found[0].publisher.as_ref().unwrap().chars().count() <= 200);
}

#[test]
fn a_platform_too_long_for_a_copy_is_not_suggested() {
    let found = parse_rawg_answer(&answer(vec![
        json!({ "external_id": 7, "title": "T", "platforms": ["P".repeat(5000)] }),
    ]))
    .unwrap();

    assert_eq!(found[0].platform, None);
}

#[test]
fn text_with_control_characters_is_cleaned() {
    let found = parse_rawg_answer(&answer(vec![json!({
        "external_id": 7, "title": "Gr\u{0}and\nTheft\u{7}", "developers": ["  ", "Rock\tstar"], "platforms": ["P\nC"],
    })]))
    .unwrap();

    assert_eq!(found[0].title, "Gr and Theft");
    assert_eq!(found[0].contributors[0].name, "Rock star");
    assert_eq!(found[0].platform.as_deref(), Some("P C"));
}

#[test]
fn a_cover_that_is_not_an_image_of_their_host_is_dropped() {
    for cover in [
        "https://evil.example/media/games/x.jpg",
        "http://media.rawg.io/media/x.jpg",
        "https://media.rawg.io.evil.example/media/x.jpg",
        "https://media.rawg.io@evil.example/media/x.jpg",
        "https://user@media.rawg.io/media/x.jpg",
        "https://media.rawg.io:8443/media/x.jpg",
        "https://media.rawg.io/other/x.jpg",
        "https://media.rawg.io/media/../x.jpg",
        "https://media.rawg.io/media/%2e%2e/x.jpg",
        "https://media.rawg.io/media//x.jpg",
        "https://media.rawg.io/media/x.svg",
        "https://media.rawg.io/media/x.html",
        "https://media.rawg.io/media/x",
        "https://media.rawg.io/media/x.jpg?evil=1",
        "https://media.rawg.io/media/x.jpg#frag",
        "https://media.rawg.io/media/https://evil.example/x.jpg",
        "https://media.rawg.io/media/x y.jpg",
        "https://127.0.0.1/media/x.jpg",
        "https://image.tmdb.org/media/x.jpg",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "//media.rawg.io/media/x.jpg",
        "https://media.rawg.io/media/x.jpg\nHost: evil",
        "",
    ] {
        let found = parse_rawg_answer(&answer(vec![
            json!({ "external_id": 7, "title": "T", "cover_url": cover }),
        ]))
        .unwrap();
        assert_eq!(found[0].cover_url, None, "kept {cover:?}");
    }
}

#[test]
fn the_covers_of_their_image_host_are_kept_in_every_format_they_serve() {
    for cover in [
        "https://media.rawg.io/media/games/456/a.jpg",
        "https://media.rawg.io/media/games/456/a.jpeg",
        "https://media.rawg.io/media/screenshots/a_b-c.png",
        "https://media.rawg.io/media/crop/600/400/games/456/a.webp",
    ] {
        let found = parse_rawg_answer(&answer(vec![
            json!({ "external_id": 7, "title": "T", "cover_url": cover }),
        ]))
        .unwrap();
        assert_eq!(found[0].cover_url.as_deref(), Some(cover));
    }
}

#[test]
fn a_year_that_cannot_be_one_is_dropped() {
    for value in [
        json!(-5),
        json!(0),
        json!(99_999),
        json!(1e12),
        json!(2013.5),
    ] {
        let found = parse_rawg_answer(&answer(vec![
            json!({ "external_id": 7, "title": "T", "year": value }),
        ]))
        .unwrap();
        assert_eq!(found[0].published_year, None, "year {value}");
    }
}
