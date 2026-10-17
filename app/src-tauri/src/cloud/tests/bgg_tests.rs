//! Board games (BoardGameGeek) and role-playing books (RPGGeek) searched through the server (7.8, D62). The text a person
//! searches for goes only where it must: never without a session, never to a server that does not say it offers the
//! search, and there is no key of their own to go around the server. What comes back is somebody else's, so it is cut
//! to eight candidates of the fixed shape, with covers only from BoardGameGeek's image host, and, because its terms
//! forbid changing its data, a value that does not fit is dropped whole, never cut or cleaned.

use std::collections::BTreeSet;

use kosmotheka_domain::ContributorRole;
use serde_json::{Value, json};

use super::*;
use crate::metadata::bgg::{BggScope, parse_bgg_answer, search_bgg};
use crate::metadata::{Candidate, Lookup, ProviderError, Source};

fn game(title: &str) -> Candidate {
    let mut candidate = Candidate::new(Kind::Boardgame, Source::BoardGameGeek, title);
    candidate.external_id = Some("13".to_owned());
    candidate
}

fn book(title: &str) -> Candidate {
    let mut candidate = Candidate::new(Kind::Book, Source::RpgGeek, title);
    candidate.external_id = Some("4000".to_owned());
    candidate
}

// --- Who is asked, and with what ------------------------------------------------------------------------------------

#[tokio::test]
async fn without_a_session_nothing_goes_anywhere() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Ok(vec![game("Catan")])]);

    let found = search_bgg::<FakeApi>(None, BggScope::Boardgame, "catan")
        .await
        .unwrap();

    assert!(found.is_empty());
    assert!(cloud.bgg_asks().is_empty());
    assert_eq!(cloud.0.lock().unwrap().offer_checks, 0);
}

#[tokio::test]
async fn a_server_that_does_not_offer_bgg_never_gets_the_text() {
    for offered in [
        vec![],
        vec!["google_books"],
        vec!["tmdb", "rawg", "comic_vine"],
        vec!["bgg "],
        vec!["BGG"],
        vec!["boardgamegeek"],
    ] {
        let cloud = FakeCloud::default();
        cloud.0.lock().unwrap().metadata = offered.iter().map(|p| (*p).to_owned()).collect();
        cloud.0.lock().unwrap().bgg_answers = vec![Ok(vec![game("Catan")])].into();
        let api = cloud.api("ana");

        for scope in [BggScope::Boardgame, BggScope::Rpg] {
            let found = search_bgg(Some(&api), scope, "catan").await.unwrap();
            assert!(found.is_empty(), "{offered:?}");
        }
        assert!(cloud.bgg_asks().is_empty(), "{offered:?}");
    }
}

#[tokio::test]
async fn not_being_able_to_read_what_the_server_offers_sends_nothing() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Ok(vec![game("Catan")])]);
    cloud.0.lock().unwrap().offer_fails = true;
    let api = cloud.api("ana");

    let found = search_bgg(Some(&api), BggScope::Boardgame, "catan")
        .await
        .unwrap();

    assert!(found.is_empty());
    assert!(cloud.bgg_asks().is_empty());
}

#[tokio::test]
async fn a_server_that_offers_it_gets_the_text_and_the_scope_and_nothing_else_and_answers() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Ok(vec![game("Catan")]), Ok(vec![book("D&D")])]);
    let api = cloud.api("ana");

    let games = search_bgg(Some(&api), BggScope::Boardgame, "  catan ")
        .await
        .unwrap();
    let books = search_bgg(Some(&api), BggScope::Rpg, "dungeons")
        .await
        .unwrap();

    assert_eq!((games.len(), books.len()), (1, 1));
    assert_eq!(
        cloud.bgg_asks(),
        vec![
            (BggScope::Boardgame, "catan".to_owned()),
            (BggScope::Rpg, "dungeons".to_owned())
        ]
    );
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
        cloud.with_bgg(vec![Err(code)]);
        let api = cloud.api("ana");

        assert!(
            search_bgg(Some(&api), BggScope::Boardgame, "catan")
                .await
                .is_err(),
            "{code}"
        );
    }
}

#[tokio::test]
async fn a_quota_refusal_is_told_as_quota() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Err("metadata_quota_exhausted")]);
    let api = cloud.api("ana");

    assert!(matches!(
        search_bgg(Some(&api), BggScope::Boardgame, "catan").await,
        Err(ProviderError::Quota)
    ));
}

/// Every other source is off, so nothing here reaches the network.
fn all_off_but(source: Source) -> BTreeSet<String> {
    Source::ALL
        .iter()
        .filter(|candidate| **candidate != source)
        .map(|candidate| candidate.id().to_owned())
        .collect()
}

#[tokio::test]
async fn a_board_game_search_asks_boardgamegeek_through_the_server() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Ok(vec![game("Catan")])]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();

    let result = lookup
        .search(
            Kind::Boardgame,
            "catan",
            &all_off_but(Source::BoardGameGeek),
            Some("a-key-that-is-never-used"),
            Some(&api),
            "en",
        )
        .await;

    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.candidates[0].source, Source::BoardGameGeek);
    assert_eq!(
        cloud.bgg_asks(),
        vec![(BggScope::Boardgame, "catan".to_owned())]
    );
}

#[tokio::test]
async fn a_book_search_asks_rpggeek_and_is_never_asked_when_it_is_turned_off() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Ok(vec![book("Dungeon Master's Guide")])]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();

    let result = lookup
        .search(
            Kind::Book,
            "dungeon",
            &all_off_but(Source::RpgGeek),
            None,
            Some(&api),
            "en",
        )
        .await;
    assert_eq!(result.candidates.len(), 1);
    assert_eq!(result.candidates[0].source, Source::RpgGeek);
    assert_eq!(result.candidates[0].kind, Kind::Book);
    assert_eq!(
        cloud.bgg_asks(),
        vec![(BggScope::Rpg, "dungeon".to_owned())]
    );

    let off: BTreeSet<String> = Source::ALL.iter().map(|s| s.id().to_owned()).collect();
    let none = lookup
        .search(Kind::Book, "dungeon", &off, None, Some(&api), "en")
        .await;
    assert!(none.candidates.is_empty());
    assert_eq!(cloud.bgg_asks().len(), 1, "turned off: not asked again");
}

#[tokio::test]
async fn other_kinds_never_send_their_text_to_boardgamegeek() {
    let cloud = FakeCloud::default();
    cloud.with_bgg(vec![Ok(vec![game("x")]), Ok(vec![game("x")])]);
    let api = cloud.api("ana");
    let lookup = Lookup::new().unwrap();
    let off: BTreeSet<String> = Source::ALL.iter().map(|s| s.id().to_owned()).collect();

    for kind in [
        Kind::Film,
        Kind::TvSeries,
        Kind::Videogame,
        Kind::Comic,
        Kind::Music,
    ] {
        lookup
            .search(kind, "catan", &off, None, Some(&api), "en")
            .await;
    }

    assert!(cloud.bgg_asks().is_empty());
}

// --- What the server says -------------------------------------------------------------------------------------------

fn answer(candidates: Vec<Value>) -> Value {
    json!({ "candidates": candidates })
}

fn full_game() -> Value {
    json!({
        "external_id": 13, "title": "CATAN", "year": 1995,
        "cover_url": "https://cf.geekdo-images.com/W3Bsga_uLP9kO91gZ7H8yw__original/img/o18bjC1GpdDPSHpdYqmZ2eLuUVU=/0x0/filters:format(jpeg)/pic2419375.jpg",
        "publisher": "KOSMOS", "designers": ["Klaus Teuber"], "artists": ["Volkan Baga", "Michael Menzel"],
        "players_min": 3, "players_max": 4, "duration_minutes": 120, "min_age": 10, "expands": null,
    })
}

fn parse_games(candidates: Vec<Value>) -> Vec<Candidate> {
    parse_bgg_answer(&answer(candidates), BggScope::Boardgame).unwrap()
}

#[test]
fn a_game_is_read_as_a_candidate() {
    let found = parse_games(vec![full_game()]);

    assert_eq!(found.len(), 1);
    let candidate = &found[0];
    assert_eq!(candidate.kind, Kind::Boardgame);
    assert_eq!(candidate.source, Source::BoardGameGeek);
    assert_eq!(candidate.title, "CATAN");
    assert_eq!(candidate.external_id.as_deref(), Some("13"));
    assert_eq!(candidate.published_year, Some(1995));
    assert_eq!(candidate.publisher.as_deref(), Some("KOSMOS"));
    assert_eq!(
        (candidate.players_min, candidate.players_max),
        (Some(3), Some(4))
    );
    assert_eq!(candidate.duration_minutes, Some(120));
    assert_eq!(candidate.min_age, Some(10));
    assert!(candidate.cover_url.is_some());
    assert!(candidate.expands.is_none());
    let people: Vec<_> = candidate
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        people,
        vec![
            ("Klaus Teuber", ContributorRole::Designer),
            ("Volkan Baga", ContributorRole::Illustrator),
            ("Michael Menzel", ContributorRole::Illustrator),
        ]
    );
}

#[test]
fn an_expansion_keeps_the_game_it_needs_and_nothing_creates_the_relation() {
    let mut expansion = full_game();
    expansion["expands"] = json!({ "external_id": 13, "title": "CATAN" });
    let mut half = full_game();
    half["expands"] = json!({ "external_id": 13 });
    let mut bad = full_game();
    bad["expands"] = json!({ "external_id": "13", "title": "CATAN" });
    let mut worse = full_game();
    worse["expands"] = json!("CATAN");

    let found = parse_games(vec![expansion, half, bad, worse]);

    let base = found[0].expands.as_ref().unwrap();
    assert_eq!(
        (base.external_id.as_str(), base.title.as_str()),
        ("13", "CATAN")
    );
    assert!(found[1..].iter().all(|c| c.expands.is_none()));
}

#[test]
fn a_role_playing_book_is_a_book_whose_designers_are_its_authors() {
    let found = parse_bgg_answer(
        &answer(vec![json!({
            "external_id": 4000, "title": "Dungeon Master's Guide", "year": 1979,
            "cover_url": "https://cf.geekdo-images.com/rpg__original/img/abc=/0x0/pic1.png",
            "publisher": "TSR", "designers": ["Gary Gygax"], "artists": ["David Sutherland"],
            "players_min": 3, "players_max": 4, "duration_minutes": 120, "min_age": 10,
            "expands": { "external_id": 1, "title": "x" },
        })]),
        BggScope::Rpg,
    )
    .unwrap();

    let candidate = &found[0];
    assert_eq!(candidate.kind, Kind::Book);
    assert_eq!(candidate.source, Source::RpgGeek);
    assert_eq!(candidate.authors, vec!["Gary Gygax".to_owned()]);
    assert_eq!(candidate.publisher.as_deref(), Some("TSR"));
    assert_eq!(candidate.contributors.len(), 1);
    assert_eq!(candidate.contributors[0].role, ContributorRole::Illustrator);
    // A book has none of the game's fields, whatever the server says.
    assert_eq!(candidate.players_min, None);
    assert_eq!(candidate.duration_minutes, None);
    assert_eq!(candidate.min_age, None);
    assert!(candidate.expands.is_none());
    assert_eq!(candidate.isbn, None);
}

#[test]
fn a_game_without_detail_is_only_a_title_and_a_year() {
    let found = parse_games(vec![json!({
        "external_id": 5, "title": "Plain", "year": 2001, "cover_url": null, "publisher": null,
        "designers": [], "artists": [], "players_min": null, "players_max": null,
        "duration_minutes": null, "min_age": null, "expands": null,
    })]);

    assert_eq!(found[0].title, "Plain");
    assert!(found[0].contributors.is_empty());
    assert_eq!(found[0].players_min, None);
    assert_eq!(found[0].publisher, None);
}

#[test]
fn a_list_of_a_hundred_is_cut_to_eight() {
    let many: Vec<Value> = (1..=100)
        .map(|n| json!({ "external_id": n, "title": format!("G{n}") }))
        .collect();

    let found = parse_games(many);

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
        assert_eq!(
            parse_bgg_answer(&body, BggScope::Boardgame),
            Err(()),
            "for {body}"
        );
    }
}

#[test]
fn candidates_that_are_not_candidates_are_dropped() {
    let found = parse_games(vec![
        json!("text"),
        json!(null),
        json!([1]),
        json!({ "external_id": "13", "title": "string id" }),
        json!({ "external_id": -1, "title": "negative id" }),
        json!({ "external_id": 0, "title": "zero id" }),
        json!({ "external_id": 1.5, "title": "float id" }),
        json!({ "external_id": 99_999_999_999_u64, "title": "huge id" }),
        json!({ "external_id": [1], "title": "array id" }),
        json!({ "title": "no id" }),
        json!({ "external_id": 2 }),
        json!({ "external_id": 3, "title": ["x"] }),
        json!({ "external_id": 4, "title": "   " }),
        json!({ "external_id": 5, "title": "fine" }),
    ]);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "fine");
}

#[test]
fn fields_of_the_wrong_type_are_dropped_and_extra_ones_ignored() {
    let found = parse_games(vec![json!({
        "external_id": 7, "title": "Odd", "year": "1995", "cover_url": 5, "publisher": ["x"],
        "designers": 3, "artists": {"a": 1}, "players_min": "3", "players_max": 4.5,
        "duration_minutes": [1], "min_age": null, "description": "Text from Wikipedia",
        "evil": "<script>", "nested": {"a": [1, 2]},
    })]);

    let candidate = &found[0];
    assert_eq!(candidate.title, "Odd");
    assert_eq!(candidate.published_year, None);
    assert_eq!(candidate.cover_url, None);
    assert_eq!(candidate.publisher, None);
    assert!(candidate.contributors.is_empty());
    assert_eq!(candidate.players_min, None);
    assert_eq!(candidate.players_max, None);
    assert_eq!(candidate.duration_minutes, None);
}

#[test]
fn numbers_that_cannot_be_real_are_dropped() {
    for value in [
        json!(-5),
        json!(0),
        json!(100),
        json!(99_999),
        json!(1e12),
        json!(3.5),
    ] {
        let found = parse_games(vec![json!({
            "external_id": 7, "title": "T", "players_min": value, "players_max": value,
            "min_age": value, "year": value,
        })]);
        assert_eq!(found[0].players_min, None, "{value}");
        assert_eq!(found[0].players_max, None, "{value}");
        assert_eq!(found[0].min_age, None, "{value}");
        assert_eq!(found[0].published_year, None, "{value}");
    }
    let found = parse_games(vec![
        json!({ "external_id": 7, "title": "T", "duration_minutes": 10_000 }),
        json!({ "external_id": 8, "title": "T", "duration_minutes": 10_001 }),
    ]);
    assert_eq!(found[0].duration_minutes, Some(10_000));
    assert_eq!(found[1].duration_minutes, None);
}

// --- Nothing of theirs is cut or cleaned ----------------------------------------------------------------------------

#[test]
fn a_value_that_is_too_long_is_dropped_whole_and_never_cut() {
    let fits = "y".repeat(300);
    let found = parse_games(vec![
        json!({ "external_id": 1, "title": "x".repeat(301) }),
        json!({ "external_id": 2, "title": fits, "publisher": "p".repeat(301),
                "designers": ["d".repeat(301), "Klaus Teuber"] }),
    ]);

    assert_eq!(
        found.len(),
        1,
        "a title that does not fit is not a candidate"
    );
    assert_eq!(found[0].title.chars().count(), 300);
    assert_eq!(found[0].publisher, None);
    let names: Vec<_> = found[0]
        .contributors
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, vec!["Klaus Teuber"]);
}

#[test]
fn enormous_fields_are_dropped_and_enormous_lists_cut() {
    let people: Vec<String> = (0..100_000).map(|n| format!("Person {n}")).collect();
    let found = parse_games(vec![json!({
        "external_id": 7, "title": "T", "publisher": "P".repeat(1_000_000),
        "designers": people, "artists": people,
    })]);

    assert_eq!(found[0].publisher, None);
    // Five designers and five artists: choosing how many is not changing a value.
    assert_eq!(found[0].contributors.len(), 10);
}

#[test]
fn text_with_control_characters_or_hidden_marks_is_dropped_whole_not_cleaned() {
    let found = parse_games(vec![
        json!({ "external_id": 1, "title": "Cat\u{0}an" }),
        json!({ "external_id": 2, "title": "Ca\ntan" }),
        json!({ "external_id": 3, "title": "Catan\u{202E}" }),
        json!({ "external_id": 4, "title": "Ca\u{200B}tan" }),
        json!({ "external_id": 5, "title": "Catan", "publisher": "KOS\tMOS",
                "designers": ["Klaus\u{7}", "  ", "Klaus Teuber"] }),
    ]);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Catan");
    assert_eq!(found[0].publisher, None);
    assert_eq!(found[0].contributors.len(), 1);
    assert_eq!(found[0].contributors[0].name, "Klaus Teuber");
}

#[test]
fn what_fits_comes_out_as_it_was_sent_with_only_the_blanks_around_it_trimmed() {
    let found = parse_games(vec![json!({
        "external_id": 1, "title": "  Catán: 5 & 6 — 君  ", "publisher": "Devir  Iberia",
    })]);

    assert_eq!(found[0].title, "Catán: 5 & 6 — 君");
    assert_eq!(found[0].publisher.as_deref(), Some("Devir  Iberia"));
}

#[test]
fn a_cover_that_is_not_an_image_of_their_host_is_dropped() {
    for cover in [
        "https://evil.example/pic1.jpg",
        "http://cf.geekdo-images.com/pic1.jpg",
        "https://cf.geekdo-images.com.evil.example/pic1.jpg",
        "https://cf.geekdo-images.com@evil.example/pic1.jpg",
        "https://user@cf.geekdo-images.com/pic1.jpg",
        "https://cf.geekdo-images.com:8443/pic1.jpg",
        "https://cf.geekdo-images.com/a/../pic1.jpg",
        "https://cf.geekdo-images.com/%2e%2e/pic1.jpg",
        "https://cf.geekdo-images.com//pic1.jpg",
        "https://cf.geekdo-images.com/https://evil.example/pic1.jpg",
        "https://cf.geekdo-images.com/pic1.svg",
        "https://cf.geekdo-images.com/pic1.html",
        "https://cf.geekdo-images.com/pic1",
        "https://cf.geekdo-images.com/pic1.jpg?evil=1",
        "https://cf.geekdo-images.com/pic1.jpg#frag",
        "https://cf.geekdo-images.com/pic 1.jpg",
        "https://cf.geekdo-images.com/a\\b.jpg",
        "https://127.0.0.1/pic1.jpg",
        "https://media.rawg.io/pic1.jpg",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "//cf.geekdo-images.com/pic1.jpg",
        "https://cf.geekdo-images.com/pic1.jpg\nHost: evil",
        "",
    ] {
        let found = parse_games(vec![
            json!({ "external_id": 7, "title": "T", "cover_url": cover }),
        ]);
        assert_eq!(found[0].cover_url, None, "kept {cover:?}");
    }
}

#[test]
fn the_covers_of_their_image_host_are_kept_in_every_format_they_serve() {
    for cover in [
        "https://cf.geekdo-images.com/pic1.jpg",
        "https://cf.geekdo-images.com/a/b/pic1.png",
        "https://cf.geekdo-images.com/a/pic1.webp",
        "https://cf.geekdo-images.com/a/pic1.gif",
        "https://cf.geekdo-images.com/sZYp_3BTDGjh2unaZfZmuA__original/img/7d-lj5Gd1e8PFnD97LYFah2c45M=/0x0/filters:format(jpeg)/pic2437871.jpg",
    ] {
        let found = parse_games(vec![
            json!({ "external_id": 7, "title": "T", "cover_url": cover }),
        ]);
        assert_eq!(found[0].cover_url.as_deref(), Some(cover));
    }
}
