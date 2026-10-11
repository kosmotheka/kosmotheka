//! Films and series searched in TMDB through the server (7.8, D62). The text a person searches for is the most private
//! thing the app sends anywhere, so it goes only where it must: never without a session, never for a kind TMDB does
//! not cover, never to a server that does not say it offers the search. And what comes back is somebody else's: it is
//! cut to eight candidates of the fixed shape, with covers only from TMDB's image host.

use kosmotheka_domain::ContributorRole;
use serde_json::{Value, json};

use super::*;
use crate::metadata::via_server::{parse_tmdb_answer, search_tmdb};
use crate::metadata::{Candidate, ProviderError, Source};

fn film(title: &str) -> Candidate {
    let mut candidate = Candidate::new(Kind::Film, Source::Tmdb, title);
    candidate.external_id = Some("603".to_owned());
    candidate
}

// --- Who is asked, and with what ------------------------------------------------------------------------------------

#[tokio::test]
async fn without_a_session_nothing_goes_anywhere() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Ok(vec![film("Matrix")])]);

    let found = search_tmdb::<FakeApi>(None, Kind::Film, "matrix", "es")
        .await
        .unwrap();

    assert!(found.is_empty());
    assert!(cloud.tmdb_asks().is_empty());
    assert_eq!(cloud.0.lock().unwrap().offer_checks, 0);
}

#[tokio::test]
async fn only_films_and_series_are_ever_sent() {
    for kind in [
        Kind::Book,
        Kind::Music,
        Kind::Videogame,
        Kind::Comic,
        Kind::Boardgame,
        Kind::Other,
    ] {
        let cloud = FakeCloud::default();
        cloud.with_tmdb(vec![Ok(vec![film("Matrix")])]);
        let api = cloud.api("ana");

        let found = search_tmdb(Some(&api), kind, "matrix", "es").await.unwrap();

        assert!(found.is_empty(), "{kind:?}");
        assert!(cloud.tmdb_asks().is_empty(), "{kind:?}");
        assert_eq!(cloud.0.lock().unwrap().offer_checks, 0, "{kind:?}");
    }
}

#[tokio::test]
async fn a_server_that_does_not_offer_tmdb_never_gets_the_text() {
    for offered in [vec![], vec!["google_books"], vec!["tmdb "], vec!["TMDB"]] {
        let cloud = FakeCloud::default();
        cloud.0.lock().unwrap().metadata = offered.iter().map(|p| (*p).to_owned()).collect();
        cloud.0.lock().unwrap().tmdb_answers = vec![Ok(vec![film("Matrix")])].into();
        let api = cloud.api("ana");

        let found = search_tmdb(Some(&api), Kind::Film, "matrix", "es")
            .await
            .unwrap();

        assert!(found.is_empty(), "{offered:?}");
        assert!(cloud.tmdb_asks().is_empty(), "{offered:?}");
    }
}

#[tokio::test]
async fn not_being_able_to_read_what_the_server_offers_sends_nothing() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Ok(vec![film("Matrix")])]);
    cloud.0.lock().unwrap().offer_fails = true;
    let api = cloud.api("ana");

    let found = search_tmdb(Some(&api), Kind::Film, "matrix", "es")
        .await
        .unwrap();

    assert!(found.is_empty());
    assert!(cloud.tmdb_asks().is_empty());
}

#[tokio::test]
async fn a_server_that_offers_it_gets_the_kind_the_text_and_the_language_and_answers() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Ok(vec![film("Matrix")])]);
    let api = cloud.api("ana");

    let found = search_tmdb(Some(&api), Kind::TvSeries, "  breaking bad ", "es")
        .await
        .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(
        cloud.tmdb_asks(),
        vec![(Kind::TvSeries, "breaking bad".to_owned(), "es".to_owned())]
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
        cloud.with_tmdb(vec![Err(code)]);
        let api = cloud.api("ana");

        let result = search_tmdb(Some(&api), Kind::Film, "matrix", "es").await;

        assert!(result.is_err(), "{code}");
    }
}

#[tokio::test]
async fn a_quota_refusal_is_told_as_quota() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Err("metadata_quota_exhausted")]);
    let api = cloud.api("ana");

    let result = search_tmdb(Some(&api), Kind::Film, "matrix", "es").await;

    assert!(matches!(result, Err(ProviderError::Quota)));
}

// --- What the server says -------------------------------------------------------------------------------------------

fn answer(candidates: Vec<Value>) -> Value {
    json!({ "candidates": candidates })
}

fn full_film() -> Value {
    json!({
        "external_id": 603, "title": "Matrix", "original_title": "The Matrix", "year": 1999, "language": "en",
        "cover_url": "https://image.tmdb.org/t/p/w500/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg",
        "directors": ["Lana Wachowski", "Lilly Wachowski"], "creators": [], "duration_minutes": 136, "seasons": null,
    })
}

#[test]
fn a_film_is_read_as_a_candidate() {
    let found = parse_tmdb_answer(&answer(vec![full_film()]), Kind::Film).unwrap();

    assert_eq!(found.len(), 1);
    let candidate = &found[0];
    assert_eq!(candidate.kind, Kind::Film);
    assert_eq!(candidate.source, Source::Tmdb);
    assert_eq!(candidate.title, "Matrix");
    assert_eq!(candidate.external_id.as_deref(), Some("603"));
    assert_eq!(candidate.published_year, Some(1999));
    assert_eq!(candidate.language.as_deref(), Some("en"));
    assert_eq!(candidate.duration_minutes, Some(136));
    assert_eq!(candidate.seasons, None);
    assert_eq!(
        candidate.cover_url.as_deref(),
        Some("https://image.tmdb.org/t/p/w500/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg")
    );
    let directors: Vec<_> = candidate
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        directors,
        vec![
            ("Lana Wachowski", ContributorRole::Director),
            ("Lilly Wachowski", ContributorRole::Director)
        ]
    );
}

#[test]
fn a_series_is_read_with_its_creators_and_seasons() {
    let found = parse_tmdb_answer(
        &answer(vec![json!({
            "external_id": 1396, "title": "Breaking Bad", "original_title": "Breaking Bad", "year": 2008,
            "language": "en", "cover_url": null, "directors": [], "creators": ["Vince Gilligan"],
            "duration_minutes": null, "seasons": 5,
        })]),
        Kind::TvSeries,
    )
    .unwrap();

    assert_eq!(found[0].kind, Kind::TvSeries);
    assert_eq!(found[0].seasons, Some(5));
    assert_eq!(found[0].duration_minutes, None);
    assert_eq!(found[0].contributors.len(), 1);
    assert_eq!(found[0].contributors[0].name, "Vince Gilligan");
}

#[test]
fn a_list_of_a_hundred_is_cut_to_eight() {
    let many: Vec<Value> = (1..=100)
        .map(|n| json!({ "external_id": n, "title": format!("T{n}") }))
        .collect();

    let found = parse_tmdb_answer(&answer(many), Kind::Film).unwrap();

    assert_eq!(found.len(), 8);
    assert_eq!(found[0].title, "T1");
    assert_eq!(found[7].title, "T8");
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
        assert_eq!(parse_tmdb_answer(&body, Kind::Film), Err(()), "for {body}");
    }
}

#[test]
fn candidates_that_are_not_candidates_are_dropped() {
    let found = parse_tmdb_answer(
        &answer(vec![
            json!("text"),
            json!(null),
            json!([1]),
            json!({ "external_id": "603", "title": "string id" }),
            json!({ "external_id": -1, "title": "negative id" }),
            json!({ "external_id": 0, "title": "zero id" }),
            json!({ "external_id": 1.5, "title": "float id" }),
            json!({ "external_id": [1], "title": "array id" }),
            json!({ "title": "no id" }),
            json!({ "external_id": 2 }),
            json!({ "external_id": 3, "title": ["x"] }),
            json!({ "external_id": 4, "title": "   " }),
            json!({ "external_id": 5, "title": "fine" }),
        ]),
        Kind::Film,
    )
    .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "fine");
}

#[test]
fn fields_of_the_wrong_type_are_dropped_and_extra_ones_ignored() {
    let found = parse_tmdb_answer(
        &answer(vec![json!({
            "external_id": 7, "title": "Odd", "original_title": ["x"], "year": "1999", "language": {"a": 1},
            "cover_url": 5, "directors": "nobody", "creators": 3, "duration_minutes": "long", "seasons": [1],
            "evil": "<script>", "nested": {"a": [1, 2]},
        })]),
        Kind::Film,
    )
    .unwrap();

    let candidate = &found[0];
    assert_eq!(candidate.title, "Odd");
    assert_eq!(candidate.published_year, None);
    assert_eq!(candidate.language, None);
    assert_eq!(candidate.cover_url, None);
    assert!(candidate.contributors.is_empty());
    assert_eq!(candidate.duration_minutes, None);
    assert_eq!(candidate.seasons, None);
}

#[test]
fn enormous_fields_are_cut() {
    let people: Vec<String> = (0..100_000).map(|n| format!("Person {n}")).collect();
    let found = parse_tmdb_answer(
        &answer(vec![json!({
            "external_id": 7, "title": "T".repeat(1_000_000), "directors": people,
            "language": "l".repeat(1_000_000),
        })]),
        Kind::Film,
    )
    .unwrap();

    assert_eq!(found[0].title.chars().count(), 300);
    assert_eq!(found[0].contributors.len(), 5);
    assert_eq!(found[0].language, None);
}

#[test]
fn text_with_control_characters_is_cleaned() {
    let found = parse_tmdb_answer(
        &answer(vec![json!({ "external_id": 7, "title": "Ma\u{0}tr\nix\u{7}", "directors": ["  ", "Lana\tWachowski"] })]),
        Kind::Film,
    )
    .unwrap();

    assert_eq!(found[0].title, "Ma tr ix");
    assert_eq!(found[0].contributors[0].name, "Lana Wachowski");
}

#[test]
fn a_cover_that_is_not_a_poster_of_their_image_host_is_dropped() {
    for cover in [
        "https://evil.example/t/p/w500/x.jpg",
        "http://image.tmdb.org/t/p/w500/x.jpg",
        "https://image.tmdb.org.evil.example/t/p/w500/x.jpg",
        "https://image.tmdb.org@evil.example/t/p/w500/x.jpg",
        "https://user@image.tmdb.org/t/p/w500/x.jpg",
        "https://image.tmdb.org:8443/t/p/w500/x.jpg",
        "https://image.tmdb.org/t/p/original/x.jpg",
        "https://image.tmdb.org/t/p/w500/../../x.jpg",
        "https://image.tmdb.org/t/p/w500/a/b.jpg",
        "https://image.tmdb.org/t/p/w500/x.svg",
        "https://image.tmdb.org/t/p/w500/x.jpg?evil=1",
        "https://image.tmdb.org/t/p/w500/x.jpg#frag",
        "https://127.0.0.1/t/p/w500/x.jpg",
        "https://books.google.com/t/p/w500/x.jpg",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "//image.tmdb.org/t/p/w500/x.jpg",
        "https://image.tmdb.org/t/p/w500/x.jpg\nHost: evil",
        "",
    ] {
        let found = parse_tmdb_answer(
            &answer(vec![
                json!({ "external_id": 7, "title": "T", "cover_url": cover }),
            ]),
            Kind::Film,
        )
        .unwrap();
        assert_eq!(found[0].cover_url, None, "kept {cover:?}");
    }
}

#[test]
fn a_year_or_a_length_that_cannot_be_one_is_dropped() {
    for value in [
        json!(-5),
        json!(0),
        json!(99_999),
        json!(1e12),
        json!(2013.5),
    ] {
        let found = parse_tmdb_answer(
            &answer(vec![json!({
                "external_id": 7, "title": "T", "year": value, "duration_minutes": value, "seasons": value,
            })]),
            Kind::Film,
        )
        .unwrap();
        assert_eq!(found[0].published_year, None, "year {value}");
        assert_eq!(found[0].duration_minutes, None, "length {value}");
        assert_eq!(found[0].seasons, None, "seasons {value}");
    }
}
