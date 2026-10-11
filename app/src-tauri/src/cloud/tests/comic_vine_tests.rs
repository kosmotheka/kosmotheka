//! Comics searched in Comic Vine (7.8, D62), through the server of the account or with the person's own key. As for
//! RAWG: nothing is sent without a session or a key, only to a server that says it offers it; what comes back is
//! somebody else's, so it is cut to eight candidates of the fixed shape (series first, then issues), with covers only
//! from Comic Vine's image site, ids that can be, and nothing of the resources that were not asked for. Comic Vine
//! takes its key only in the address, so no error may carry it. The recorded answers are written by hand
//! (`tests/fixtures/comicvine-*.json`) from what its documentation says: there is no real key to record with.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use kosmotheka_domain::{ContributorRole, Format};
use serde_json::{Value, json};
use tokio::net::TcpListener;

use super::*;
use crate::metadata::via_server::{
    SearchAccess, choose_search_access, parse_comic_vine_answer, search_comic_vine,
};
use crate::metadata::{Candidate, ProviderError, Source, comic_vine};

const SEARCH: &str = include_str!("../../../tests/fixtures/comicvine-search.json");
const VOLUME_796: &str = include_str!("../../../tests/fixtures/comicvine-volume-796.json");
const ISSUE_6: &str = include_str!("../../../tests/fixtures/comicvine-issue-6.json");

const KEY: &str = "SENTINEL-own-comicvine-key-31";

fn comic(title: &str) -> Candidate {
    Candidate::new(Kind::Comic, Source::ComicVine, title)
}

// --- Who is asked, and with what ------------------------------------------------------------------------------------

#[derive(Clone)]
struct Direct {
    keys: Arc<Mutex<Vec<String>>>,
    answer: Arc<dyn Fn() -> std::result::Result<Vec<Candidate>, ProviderError> + Send + Sync>,
}

impl Direct {
    fn answering(candidates: Vec<Candidate>) -> Self {
        Self::with(move || Ok(candidates.clone()))
    }

    fn with(
        answer: impl Fn() -> std::result::Result<Vec<Candidate>, ProviderError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            keys: Arc::default(),
            answer: Arc::new(answer),
        }
    }

    fn keys(&self) -> Vec<String> {
        self.keys.lock().unwrap().clone()
    }

    async fn ask(&self, key: String) -> std::result::Result<Vec<Candidate>, ProviderError> {
        self.keys.lock().unwrap().push(key);
        (self.answer)()
    }
}

async fn via(
    access: SearchAccess<'_, FakeApi>,
    direct: &Direct,
) -> std::result::Result<Vec<Candidate>, ProviderError> {
    let direct = direct.clone();
    search_comic_vine(access, "batman", move |key| {
        let direct = direct.clone();
        async move { direct.ask(key).await }
    })
    .await
}

#[tokio::test]
async fn with_their_own_key_nothing_goes_to_the_account_server() {
    for session in [true, false] {
        let cloud = FakeCloud::default();
        cloud.with_comic_vine(vec![Ok(vec![comic("From the server")])]);
        let api = cloud.api("ana");
        let direct = Direct::answering(vec![comic("Direct")]);

        let found = via(
            choose_search_access(Some("k"), session.then_some(&api)),
            &direct,
        )
        .await
        .unwrap();

        assert_eq!(found[0].title, "Direct", "session {session}");
        assert_eq!(direct.keys(), vec!["k".to_owned()]);
        assert!(cloud.comic_vine_asks().is_empty(), "session {session}");
        assert_eq!(cloud.0.lock().unwrap().offer_checks, 0, "session {session}");
    }
}

#[tokio::test]
async fn without_a_key_the_server_of_the_session_answers() {
    let cloud = FakeCloud::default();
    cloud.with_comic_vine(vec![Ok(vec![comic("From the server")])]);
    let api = cloud.api("ana");
    let direct = Direct::answering(vec![]);

    let found = via(choose_search_access(None, Some(&api)), &direct)
        .await
        .unwrap();

    assert_eq!(found[0].title, "From the server");
    assert!(direct.keys().is_empty());
    assert_eq!(cloud.comic_vine_asks(), vec!["batman".to_owned()]);
}

#[tokio::test]
async fn without_a_key_and_without_a_session_nothing_is_asked_of_anybody() {
    let cloud = FakeCloud::default();
    cloud.with_comic_vine(vec![Ok(vec![comic("From the server")])]);
    let direct = Direct::answering(vec![comic("Direct")]);

    let found = via(choose_search_access(None, None), &direct)
        .await
        .unwrap();

    assert!(found.is_empty());
    assert!(direct.keys().is_empty());
    assert!(cloud.comic_vine_asks().is_empty());
}

#[tokio::test]
async fn a_blank_key_is_no_key() {
    for blank in ["", " ", "\t", "\n", " \t \n "] {
        let cloud = FakeCloud::default();
        cloud.with_comic_vine(vec![Ok(vec![comic("From the server")])]);
        let api = cloud.api("ana");
        let direct = Direct::answering(vec![comic("Direct")]);

        let alone = via(choose_search_access(Some(blank), None), &direct)
            .await
            .unwrap();
        let with_session = via(choose_search_access(Some(blank), Some(&api)), &direct)
            .await
            .unwrap();

        assert!(alone.is_empty(), "{blank:?}");
        assert_eq!(with_session[0].title, "From the server", "{blank:?}");
        assert!(direct.keys().is_empty(), "{blank:?}");
    }
}

#[tokio::test]
async fn a_key_the_provider_rejects_says_so_and_does_not_fall_back_to_the_server() {
    let cloud = FakeCloud::default();
    cloud.with_comic_vine(vec![Ok(vec![comic("From the server")])]);
    let api = cloud.api("ana");
    let direct = Direct::with(|| Err(ProviderError::BadKey));

    let result = via(choose_search_access(Some("bad"), Some(&api)), &direct).await;

    assert!(matches!(result, Err(ProviderError::BadKey)));
    assert!(cloud.comic_vine_asks().is_empty());
}

#[tokio::test]
async fn a_server_that_does_not_offer_comic_vine_never_gets_the_text() {
    for offered in [
        vec![],
        vec!["rawg"],
        vec!["tmdb"],
        vec!["comic_vine "],
        vec!["comicvine"],
        vec!["COMIC_VINE"],
    ] {
        let cloud = FakeCloud::default();
        cloud.0.lock().unwrap().metadata = offered.iter().map(|p| (*p).to_owned()).collect();
        cloud.0.lock().unwrap().comic_vine_answers =
            vec![Ok(vec![comic("From the server")])].into();
        let api = cloud.api("ana");
        let direct = Direct::answering(vec![]);

        let found = via(choose_search_access(None, Some(&api)), &direct)
            .await
            .unwrap();

        assert!(found.is_empty(), "{offered:?}");
        assert!(cloud.comic_vine_asks().is_empty(), "{offered:?}");
    }
}

#[tokio::test]
async fn a_server_that_fails_is_a_warning_and_a_quota_is_told_as_quota() {
    for code in [
        "server_unavailable",
        "metadata_failed",
        "invalid_token",
        "too_many_requests",
    ] {
        let cloud = FakeCloud::default();
        cloud.with_comic_vine(vec![Err(code)]);
        let api = cloud.api("ana");
        let direct = Direct::answering(vec![]);

        assert!(
            via(choose_search_access(None, Some(&api)), &direct)
                .await
                .is_err(),
            "{code}"
        );
    }
    let cloud = FakeCloud::default();
    cloud.with_comic_vine(vec![Err("metadata_quota_exhausted")]);
    let api = cloud.api("ana");
    assert!(matches!(
        via(
            choose_search_access(None, Some(&api)),
            &Direct::answering(vec![])
        )
        .await,
        Err(ProviderError::Quota)
    ));
}

// --- What the server says -------------------------------------------------------------------------------------------

fn volume_item() -> Value {
    json!({
        "resource": "volume", "external_id": 796, "title": "Batman", "series": null, "number": null,
        "year": 1940, "publisher": "DC Comics",
        "cover_url": "https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg",
        "writers": ["Bill Finger"], "artists": ["Bob Kane"],
    })
}

fn issue_item() -> Value {
    json!({
        "resource": "issue", "external_id": 6, "title": "The Joker", "series": "Batman", "number": "1",
        "year": 1940, "publisher": null,
        "cover_url": "https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg",
        "writers": ["Bill Finger"], "artists": [],
    })
}

fn answer(items: Vec<Value>) -> Value {
    json!({ "candidates": items })
}

#[test]
fn a_series_is_read_as_a_candidate() {
    let found = parse_comic_vine_answer(&answer(vec![volume_item()])).unwrap();

    let series = &found[0];
    assert_eq!(series.kind, Kind::Comic);
    assert_eq!(series.source, Source::ComicVine);
    assert_eq!(series.title, "Batman");
    assert_eq!(series.external_id.as_deref(), Some("volume-796"));
    assert_eq!(series.published_year, Some(1940));
    assert_eq!(series.publisher.as_deref(), Some("DC Comics"));
    assert_eq!(series.number, None);
    assert_eq!(series.format, None);
    assert_eq!(
        series.cover_url.as_deref(),
        Some(
            "https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg"
        )
    );
    let people: Vec<_> = series
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        people,
        vec![
            ("Bill Finger", ContributorRole::Writer),
            ("Bob Kane", ContributorRole::Illustrator)
        ]
    );
}

#[test]
fn an_issue_is_read_with_its_series_its_number_and_the_single_issue_format() {
    let found = parse_comic_vine_answer(&answer(vec![issue_item()])).unwrap();

    let issue = &found[0];
    assert_eq!(issue.title, "Batman #1: The Joker");
    assert_eq!(issue.external_id.as_deref(), Some("issue-6"));
    assert_eq!(issue.number.as_deref(), Some("1"));
    assert_eq!(issue.format, Some(Format::SingleIssue));
    assert_eq!(issue.published_year, Some(1940));
    assert_eq!(issue.publisher, None);
}

#[test]
fn the_title_of_an_issue_is_made_of_what_there_is() {
    let cases = [
        (
            json!({"series": "Batman", "number": "1", "title": "The Joker"}),
            "Batman #1: The Joker",
        ),
        (
            json!({"series": "Batman", "number": "1", "title": null}),
            "Batman #1",
        ),
        (
            json!({"series": "Batman", "number": null, "title": "The Joker"}),
            "Batman: The Joker",
        ),
        (
            json!({"series": "Batman", "number": "1", "title": "Batman"}),
            "Batman #1",
        ),
        (
            json!({"series": null, "number": "1", "title": "The Joker"}),
            "The Joker #1",
        ),
        (
            json!({"series": null, "number": null, "title": "The Joker"}),
            "The Joker",
        ),
        (
            json!({"series": "Batman", "number": null, "title": null}),
            "Batman",
        ),
    ];
    for (parts, expected) in cases {
        let mut item = issue_item();
        for (key, value) in parts.as_object().unwrap() {
            item[key] = value.clone();
        }
        let found = parse_comic_vine_answer(&answer(vec![item])).unwrap();
        assert_eq!(found[0].title, expected, "{parts}");
    }
}

#[test]
fn what_cannot_be_named_is_dropped() {
    let mut nothing = issue_item();
    nothing["series"] = Value::Null;
    nothing["number"] = Value::Null;
    nothing["title"] = Value::Null;
    let mut nameless = volume_item();
    nameless["title"] = Value::Null;

    let found = parse_comic_vine_answer(&answer(vec![nothing, nameless, volume_item()])).unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].title, "Batman");
}

#[test]
fn a_list_of_a_hundred_is_cut_to_eight_and_other_resources_are_dropped() {
    let mut many: Vec<Value> = (1..=100)
        .map(|n| json!({ "resource": "volume", "external_id": n, "title": format!("V{n}") }))
        .collect();
    many.insert(
        0,
        json!({ "resource": "character", "external_id": 1, "title": "Batman" }),
    );
    many.insert(
        1,
        json!({ "resource": "publisher", "external_id": 2, "title": "DC" }),
    );
    many.insert(
        2,
        json!({ "resource": ["volume"], "external_id": 3, "title": "Odd" }),
    );
    many.insert(3, json!({ "external_id": 4, "title": "No resource" }));

    let found = parse_comic_vine_answer(&answer(many)).unwrap();

    assert_eq!(found.len(), 8);
    assert_eq!(found[0].title, "V1");
    assert_eq!(found[7].title, "V8");
}

#[test]
fn an_answer_that_is_not_the_shape_is_a_failure() {
    for body in [
        json!(null),
        json!("text"),
        json!([1, 2]),
        json!({}),
        json!({ "candidates": "no" }),
        json!({ "items": [] }),
    ] {
        assert_eq!(parse_comic_vine_answer(&body), Err(()), "for {body}");
    }
}

#[test]
fn ids_and_fields_that_are_not_what_they_should_be_are_dropped_or_cut() {
    let mut bad_ids: Vec<Value> = [
        json!("abc"),
        json!(-4),
        json!(0),
        json!(1.5),
        json!([1]),
        json!(99_999_999_999_u64),
        Value::Null,
    ]
    .into_iter()
    .map(|id| json!({ "resource": "volume", "external_id": id, "title": "Bad" }))
    .collect();
    bad_ids.push(json!("text"));
    bad_ids.push(json!(null));
    bad_ids.push(json!({
        "resource": "issue", "external_id": 9, "title": "T".repeat(100_000), "series": "S".repeat(100_000),
        "number": "9".repeat(5000), "year": "1940", "publisher": ["x"], "cover_url": 5,
        "writers": (0..100_000).map(|n| format!("W{n}")).collect::<Vec<_>>(), "artists": "nobody",
    }));

    let found = parse_comic_vine_answer(&answer(bad_ids)).unwrap();

    assert_eq!(found.len(), 1);
    let issue = &found[0];
    assert!(issue.title.chars().count() <= 300);
    assert!(issue.number.as_ref().unwrap().chars().count() <= 20);
    assert_eq!(issue.published_year, None);
    assert_eq!(issue.cover_url, None);
    assert_eq!(issue.contributors.len(), 5);
    assert_eq!(issue.publisher, None);
}

#[test]
fn text_with_control_characters_is_cleaned() {
    let mut item = volume_item();
    item["title"] = json!("Bat\u{0}man\n");
    item["writers"] = json!(["  ", "Bill\tFinger"]);

    let found = parse_comic_vine_answer(&answer(vec![item])).unwrap();

    assert_eq!(found[0].title, "Bat man");
    assert_eq!(found[0].contributors[0].name, "Bill Finger");
}

#[test]
fn a_cover_that_is_not_an_image_of_their_site_is_dropped() {
    for cover in [
        "https://evil.example/a/uploads/scale_medium/x.jpg",
        "http://comicvine.gamespot.com/a/uploads/scale_medium/x.jpg",
        "https://comicvine.gamespot.com.evil.example/a/uploads/scale_medium/x.jpg",
        "https://comicvine.gamespot.com@evil.example/a/uploads/scale_medium/x.jpg",
        "https://user@comicvine.gamespot.com/a/uploads/scale_medium/x.jpg",
        "https://comicvine.gamespot.com:8443/a/uploads/scale_medium/x.jpg",
        "https://comicvine.gamespot.com/other/scale_medium/x.jpg",
        "https://comicvine.gamespot.com/a/uploads/../x.jpg",
        "https://comicvine.gamespot.com/a/uploads/%2e%2e/x.jpg",
        "https://comicvine.gamespot.com/a/uploads//x.jpg",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x.svg",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x.html",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x.jpg?evil=1",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x.jpg#frag",
        "https://comicvine.gamespot.com/a/uploads/https://evil.example/x.jpg",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x y.jpg",
        "https://comicvine.gamespot.com/api/search/?api_key=x.jpg",
        "https://127.0.0.1/a/uploads/scale_medium/x.jpg",
        "https://media.rawg.io/a/uploads/scale_medium/x.jpg",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "//comicvine.gamespot.com/a/uploads/scale_medium/x.jpg",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/x.jpg\nHost: evil",
        "",
    ] {
        let mut item = volume_item();
        item["cover_url"] = json!(cover);
        let found = parse_comic_vine_answer(&answer(vec![item])).unwrap();
        assert_eq!(found[0].cover_url, None, "kept {cover:?}");
    }
}

#[test]
fn the_covers_of_their_image_site_are_kept_in_every_format_the_app_can_store() {
    for cover in [
        "https://comicvine.gamespot.com/a/uploads/scale_medium/6/66037/1334617-batman_1_cover.jpg",
        "https://comicvine.gamespot.com/a/uploads/scale_small/0/40/a.jpeg",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/0/40/a_b-c.png",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/0/40/a.webp",
        "https://comicvine.gamespot.com/a/uploads/scale_medium/0/40/a.gif",
    ] {
        let mut item = volume_item();
        item["cover_url"] = json!(cover);
        let found = parse_comic_vine_answer(&answer(vec![item])).unwrap();
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
        json!(1940.5),
        json!("1940"),
    ] {
        let mut item = volume_item();
        item["year"] = value.clone();
        let found = parse_comic_vine_answer(&answer(vec![item])).unwrap();
        assert_eq!(found[0].published_year, None, "year {value}");
    }
}

// --- Comic Vine asked directly, with their own key ----------------------------------------------------------------------

/// What each request was: the path, its query string and any `Authorization` header (there must be none).
type Asked = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

#[derive(Clone)]
struct Provider {
    asked: Asked,
    search: Arc<Mutex<(u16, String)>>,
    details: Arc<Mutex<Vec<(String, u16, String)>>>,
}

async fn handle(
    State(stub): State<Provider>,
    uri: Uri,
    headers: HeaderMap,
) -> axum::response::Response {
    let path = uri.path().to_owned();
    stub.asked.lock().unwrap().push((
        path.clone(),
        uri.query().unwrap_or("").to_owned(),
        headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
    ));
    let (status, body) = if path.ends_with("/search/") {
        stub.search.lock().unwrap().clone()
    } else {
        stub.details
            .lock()
            .unwrap()
            .iter()
            .find(|(p, _, _)| *p == path)
            .map_or((404, String::new()), |(_, s, b)| (*s, b.clone()))
    };
    (
        StatusCode::from_u16(status).unwrap(),
        [("content-type", "application/json")],
        body,
    )
        .into_response()
}

async fn provider(search: (u16, &str), details: Vec<(&str, u16, &str)>) -> (String, Provider) {
    let stub = Provider {
        asked: Arc::default(),
        search: Arc::new(Mutex::new((search.0, search.1.to_owned()))),
        details: Arc::new(Mutex::new(
            details
                .into_iter()
                .map(|(p, s, b)| (p.to_owned(), s, b.to_owned()))
                .collect(),
        )),
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = Router::new().fallback(handle).with_state(stub.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), stub)
}

impl Provider {
    fn paths(&self) -> Vec<String> {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .map(|(p, _, _)| p.clone())
            .collect()
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

#[tokio::test]
async fn direct_a_search_is_asked_with_the_key_in_the_address_and_read_with_the_people_of_the_first_three()
 {
    let (api, stub) = provider(
        (200, SEARCH),
        vec![
            ("/volume/4050-796/", 200, VOLUME_796),
            ("/issue/4000-6/", 200, ISSUE_6),
        ],
    )
    .await;

    let found = comic_vine::search(&client(), &api, KEY, "batman & robin")
        .await
        .unwrap();

    // The series first, then the issue; the characters and publishers it also found are not comics.
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].title, "Batman");
    assert_eq!(found[0].external_id.as_deref(), Some("volume-796"));
    assert_eq!(found[0].publisher.as_deref(), Some("DC Comics"));
    assert_eq!(found[0].published_year, Some(1940));
    let people: Vec<_> = found[0]
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        people,
        vec![
            ("Bill Finger", ContributorRole::Writer),
            ("Bob Kane", ContributorRole::Illustrator)
        ]
    );
    assert_eq!(found[1].title, "Batman #1: The Joker");
    assert_eq!(found[1].format, Some(Format::SingleIssue));
    // The writer, and the penciler; the cover credit and the colorist are not roles the app has.
    let issue_people: Vec<_> = found[1]
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        issue_people,
        vec![
            ("Bill Finger", ContributorRole::Writer),
            ("Bob Kane", ContributorRole::Illustrator)
        ]
    );

    let asked = stub.asked.lock().unwrap();
    let (path, query, auth) = &asked[0];
    assert_eq!(path, "/search/");
    assert!(auth.is_none());
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    assert!(pairs.contains(&("api_key".to_owned(), KEY.to_owned())));
    assert!(pairs.contains(&("format".to_owned(), "json".to_owned())));
    assert!(pairs.contains(&("query".to_owned(), "batman & robin".to_owned())));
    assert!(pairs.contains(&("resources".to_owned(), "volume,issue".to_owned())));
    assert!(pairs.contains(&("limit".to_owned(), "8".to_owned())));
    let mut details: Vec<_> = asked[1..].iter().map(|(p, _, _)| p.clone()).collect();
    details.sort();
    assert_eq!(details, vec!["/issue/4000-6/", "/volume/4050-796/"]);
}

fn hostile() -> String {
    let mut results: Vec<Value> = (1..=100)
        .map(|n| json!({ "id": n, "name": format!("V{n}"), "resource_type": "volume" }))
        .collect();
    results.insert(0, json!({ "id": 7001, "name": "T".repeat(100_000), "resource_type": "volume",
        "start_year": "long ago", "publisher": "DC", "image": { "medium_url": "https://evil.example/a/uploads/x/y.jpg", "small_url": "https://comicvine.gamespot.com/a/uploads/../x.jpg" } }));
    for bad in [
        json!("abc"),
        json!(-4),
        json!(0),
        json!(99_999_999_999_u64),
        json!(1.5),
        json!([1]),
    ] {
        results.insert(
            1,
            json!({ "id": bad, "name": "bad id", "resource_type": "volume" }),
        );
    }
    results.insert(
        1,
        json!({ "id": 9100, "name": "A hero", "resource_type": "character" }),
    );
    results.insert(
        1,
        json!({ "id": 9101, "name": "A publisher", "resource_type": "publisher" }),
    );
    results.insert(
        1,
        json!({ "id": 9102, "name": "A person", "resource_type": "person" }),
    );
    json!({ "status_code": 1, "results": results }).to_string()
}

#[tokio::test]
async fn direct_rubbish_from_the_provider_gets_the_same_cuts_as_from_a_server() {
    let (api, stub) = provider((200, &hostile()), vec![]).await;

    let found = comic_vine::search(&client(), &api, KEY, "batman")
        .await
        .unwrap();

    assert_eq!(found.len(), 8);
    assert_eq!(found[0].external_id.as_deref(), Some("volume-7001"));
    assert_eq!(found[0].title.chars().count(), 300);
    assert_eq!(found[0].cover_url, None);
    assert_eq!(found[0].published_year, None);
    // Only the first three results get a detail call, only by a number that can be one, and only for what was asked.
    let paths = stub.paths();
    assert!(paths.len() <= 1 + 3, "{paths:?}");
    for path in paths.iter().skip(1) {
        assert!(
            path.strip_prefix("/volume/4050-")
                .and_then(|rest| rest.strip_suffix('/'))
                .is_some_and(|id| id.parse::<u64>().is_ok_and(|n| n > 0 && n <= 1_000_000_000)),
            "{path}"
        );
    }
    assert!(
        !paths
            .iter()
            .any(|p| p.contains("9100") || p.contains("9101") || p.contains("9102"))
    );
}

#[tokio::test]
async fn direct_a_detail_that_fails_does_not_cost_the_others_theirs() {
    let (api, _) = provider(
        (200, SEARCH),
        vec![
            ("/volume/4050-796/", 500, "oops"),
            ("/issue/4000-6/", 200, ISSUE_6),
        ],
    )
    .await;

    let found = comic_vine::search(&client(), &api, KEY, "batman")
        .await
        .unwrap();

    assert!(found[0].contributors.is_empty());
    assert_eq!(found[1].contributors[0].name, "Bill Finger");
}

#[tokio::test]
async fn direct_only_the_roles_that_fit_count_however_they_are_written() {
    let search = json!({ "status_code": 1, "results": [
        { "id": 796, "name": "Batman", "resource_type": "volume",
          "image": { "original_url": "https://comicvine.gamespot.com/a/uploads/original/0/40/huge.jpg" } },
    ] })
    .to_string();
    let credits = json!({ "status_code": 1, "results": { "person_credits": [
        { "name": "Writer Capitals", "role": "  Writer ,  PENCILER " },
        { "name": "Only Colorist", "role": "colorist" },
        { "name": "Letterer Editor Cover", "role": "letterer, editor, cover" },
        { "name": "Plural Role", "role": "writers" },
        { "name": "No Role" },
        { "name": "Array Role", "role": ["writer"] },
    ] } })
    .to_string();
    let (api, _) = provider((200, &search), vec![("/volume/4050-796/", 200, &credits)]).await;

    let found = comic_vine::search(&client(), &api, KEY, "batman")
        .await
        .unwrap();

    let people: Vec<_> = found[0]
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        people,
        vec![
            ("Writer Capitals", ContributorRole::Writer),
            ("Writer Capitals", ContributorRole::Illustrator)
        ]
    );
    // Only the original image was given, and that one is never the cover: it can be huge.
    assert_eq!(found[0].cover_url, None);
}

#[tokio::test]
async fn direct_an_enormous_answer_and_one_that_is_not_json_are_failures() {
    let huge =
        json!({ "status_code": 1, "results": [], "padding": "x".repeat(6_000_000) }).to_string();
    for body in [huge.as_str(), "<html>hello</html>"] {
        let (api, _) = provider((200, body), vec![]).await;

        assert!(
            comic_vine::search(&client(), &api, KEY, "batman")
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn direct_the_refusals_of_the_provider_are_told_as_what_they_are() {
    let rejected = r#"{"status_code":100,"error":"Invalid API Key","results":[]}"#;
    for (status, body, expected) in [
        (401, rejected, "bad_key"),
        (403, "{}", "bad_key"),
        (200, rejected, "bad_key"),
        (420, "{}", "quota"),
        (429, "{}", "quota"),
        (500, "{}", "unexpected"),
        (404, "{}", "unexpected"),
        (200, r#"{"status_code":104,"results":[]}"#, "unexpected"),
        (200, "{}", "unexpected"),
    ] {
        let (api, _) = provider((status, body), vec![]).await;

        let error = comic_vine::search(&client(), &api, KEY, "batman")
            .await
            .unwrap_err();

        let kind = match error {
            ProviderError::BadKey => "bad_key",
            ProviderError::Quota => "quota",
            ProviderError::Unexpected(_) => "unexpected",
            ProviderError::Network(_) => "network",
        };
        assert_eq!(kind, expected, "status {status} {body}");
    }
}

fn written(error: &ProviderError) -> String {
    format!("{error} {error:?}")
}

#[tokio::test]
async fn direct_no_error_carries_the_key_whatever_goes_wrong() {
    // A provider that echoes the address, the key and everything in its errors.
    for status in [400u16, 401, 403, 404, 420, 429, 500, 502] {
        let echo = format!(
            r#"{{"error":"bad {KEY}","url":"/search/?api_key={KEY}","status_code":100,"results":[]}}"#
        );
        let (api, _) = provider((status, &echo), vec![]).await;

        let error = comic_vine::search(&client(), &api, KEY, "batman")
            .await
            .unwrap_err();

        assert!(
            !written(&error).contains(KEY),
            "status {status}: {}",
            written(&error)
        );
    }
}

#[tokio::test]
async fn direct_no_error_carries_the_key_when_the_connection_itself_fails() {
    // Nobody listens there: the error of the http client names the address, and this one holds the key.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };

    let error = comic_vine::search(&client(), &closed, KEY, "batman")
        .await
        .unwrap_err();

    assert!(matches!(error, ProviderError::Network(_)));
    assert!(!written(&error).contains(KEY), "{}", written(&error));
}

#[tokio::test]
async fn direct_no_error_carries_the_key_when_the_answer_cannot_be_read() {
    let (api, _) = provider((200, "not json at all"), vec![]).await;

    let error = comic_vine::search(&client(), &api, KEY, "batman")
        .await
        .unwrap_err();

    assert!(!written(&error).contains(KEY), "{}", written(&error));
}
