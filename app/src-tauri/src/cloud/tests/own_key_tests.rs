//! TMDB and RAWG with the person's own key (7.8, D62): the key has preference over the account's server, as for
//! Google Books, and the provider's answer is as little to be trusted as a server's. The key is somebody's secret
//! (RAWG takes it in the address of the call), so no error that reaches the screen may carry it. Answers are
//! recorded by hand in `tests/fixtures/` with the documented shape of each API: there is no real key to record with.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use kosmotheka_domain::ContributorRole;
use serde_json::{Value, json};
use tokio::net::TcpListener;

use super::*;
use crate::metadata::via_server::{SearchAccess, choose_search_access, search_rawg, search_tmdb};
use crate::metadata::{Candidate, ProviderError, Source, rawg, tmdb};

const SEARCH_MOVIE: &str = include_str!("../../../tests/fixtures/tmdb-search-movie.json");
const MOVIE_603: &str = include_str!("../../../tests/fixtures/tmdb-movie-603.json");
const SEARCH_TV: &str = include_str!("../../../tests/fixtures/tmdb-search-tv.json");
const TV_1396: &str = include_str!("../../../tests/fixtures/tmdb-tv-1396.json");
const RAWG_SEARCH: &str = include_str!("../../../tests/fixtures/rawg-search.json");
const RAWG_GAME_3498: &str = include_str!("../../../tests/fixtures/rawg-game-3498.json");

const TOKEN: &str = "SENTINEL-own-token-77";

// --- The provider, in memory ------------------------------------------------------------------------------------------

/// What each request was: the path, its query string and the `Authorization` header.
type Asked = Arc<Mutex<Vec<(String, String, Option<String>)>>>;

#[derive(Clone)]
struct Provider {
    asked: Asked,
    /// What `search` and `detail` answer, as (status, body). A detail with no answer here is a 404.
    search: Arc<Mutex<(u16, String)>>,
    details: Arc<Mutex<Vec<(String, u16, String)>>>,
}

async fn answer(
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
    let (status, body) = if path.ends_with("/search/movie")
        || path.ends_with("/search/tv")
        || path.ends_with("/games")
    {
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
    let app = Router::new().fallback(answer).with_state(stub.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), stub)
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
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

// --- Which way: the own key first -----------------------------------------------------------------------------------

/// The provider asked directly: records the keys it was given and answers what the test says.
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

fn candidate(kind: Kind, source: Source, title: &str) -> Candidate {
    Candidate::new(kind, source, title)
}

async fn tmdb_via(
    access: SearchAccess<'_, FakeApi>,
    direct: &Direct,
) -> std::result::Result<Vec<Candidate>, ProviderError> {
    let direct = direct.clone();
    search_tmdb(access, Kind::Film, "matrix", "es", move |key| {
        let direct = direct.clone();
        async move { direct.ask(key).await }
    })
    .await
}

async fn rawg_via(
    access: SearchAccess<'_, FakeApi>,
    direct: &Direct,
) -> std::result::Result<Vec<Candidate>, ProviderError> {
    let direct = direct.clone();
    search_rawg(access, "gta", move |key| {
        let direct = direct.clone();
        async move { direct.ask(key).await }
    })
    .await
}

#[tokio::test]
async fn with_their_own_key_nothing_goes_to_the_account_server_for_either_provider() {
    for session in [true, false] {
        let cloud = FakeCloud::default();
        cloud.with_tmdb(vec![Ok(vec![candidate(
            Kind::Film,
            Source::Tmdb,
            "From the server",
        )])]);
        cloud.with_rawg(vec![Ok(vec![candidate(
            Kind::Videogame,
            Source::Rawg,
            "From the server",
        )])]);
        let api = cloud.api("ana");
        let direct = Direct::answering(vec![candidate(Kind::Film, Source::Tmdb, "Direct")]);

        let tmdb_found = tmdb_via(
            choose_search_access(Some("k1"), session.then_some(&api)),
            &direct,
        )
        .await
        .unwrap();
        let rawg_found = rawg_via(
            choose_search_access(Some("k2"), session.then_some(&api)),
            &direct,
        )
        .await
        .unwrap();

        assert_eq!(tmdb_found[0].title, "Direct", "session {session}");
        assert_eq!(rawg_found[0].title, "Direct", "session {session}");
        assert_eq!(direct.keys(), vec!["k1".to_owned(), "k2".to_owned()]);
        assert!(
            cloud.tmdb_asks().is_empty() && cloud.rawg_asks().is_empty(),
            "session {session}"
        );
        assert_eq!(cloud.0.lock().unwrap().offer_checks, 0, "session {session}");
    }
}

#[tokio::test]
async fn without_a_key_the_server_of_the_session_answers() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Ok(vec![candidate(
        Kind::Film,
        Source::Tmdb,
        "From the server",
    )])]);
    cloud.with_rawg(vec![Ok(vec![candidate(
        Kind::Videogame,
        Source::Rawg,
        "From the server",
    )])]);
    let api = cloud.api("ana");
    let direct = Direct::answering(vec![]);

    let tmdb_found = tmdb_via(choose_search_access(None, Some(&api)), &direct)
        .await
        .unwrap();
    let rawg_found = rawg_via(choose_search_access(None, Some(&api)), &direct)
        .await
        .unwrap();

    assert_eq!(tmdb_found[0].title, "From the server");
    assert_eq!(rawg_found[0].title, "From the server");
    assert!(direct.keys().is_empty());
}

#[tokio::test]
async fn without_a_key_and_without_a_session_nothing_is_asked_of_anybody() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Ok(vec![candidate(
        Kind::Film,
        Source::Tmdb,
        "From the server",
    )])]);
    cloud.with_rawg(vec![Ok(vec![candidate(
        Kind::Videogame,
        Source::Rawg,
        "From the server",
    )])]);
    let direct = Direct::answering(vec![candidate(Kind::Film, Source::Tmdb, "Direct")]);

    let tmdb_found = tmdb_via(choose_search_access(None, None), &direct)
        .await
        .unwrap();
    let rawg_found = rawg_via(choose_search_access(None, None), &direct)
        .await
        .unwrap();

    assert!(tmdb_found.is_empty() && rawg_found.is_empty());
    assert!(direct.keys().is_empty());
    assert!(cloud.tmdb_asks().is_empty() && cloud.rawg_asks().is_empty());
}

#[tokio::test]
async fn a_blank_key_is_no_key() {
    for blank in ["", " ", "   ", "\t", "\n", " \t \n "] {
        let cloud = FakeCloud::default();
        cloud.with_tmdb(vec![Ok(vec![candidate(
            Kind::Film,
            Source::Tmdb,
            "From the server",
        )])]);
        let api = cloud.api("ana");
        let direct = Direct::answering(vec![candidate(Kind::Film, Source::Tmdb, "Direct")]);

        // With no session: nothing is asked of anybody.
        let alone = tmdb_via(choose_search_access(Some(blank), None), &direct)
            .await
            .unwrap();
        // With a session: the server answers, as if there were no key.
        let with_session = tmdb_via(choose_search_access(Some(blank), Some(&api)), &direct)
            .await
            .unwrap();

        assert!(alone.is_empty(), "{blank:?}");
        assert_eq!(with_session[0].title, "From the server", "{blank:?}");
        assert!(direct.keys().is_empty(), "{blank:?}");
    }
}

#[tokio::test]
async fn the_key_is_cleaned_of_blanks_before_it_is_used() {
    let direct = Direct::answering(vec![]);

    tmdb_via(
        choose_search_access::<FakeApi>(Some("  abc \n"), None),
        &direct,
    )
    .await
    .unwrap();

    assert_eq!(direct.keys(), vec!["abc".to_owned()]);
}

#[tokio::test]
async fn a_key_the_provider_rejects_says_so_and_does_not_fall_back_to_the_server() {
    let cloud = FakeCloud::default();
    cloud.with_tmdb(vec![Ok(vec![candidate(
        Kind::Film,
        Source::Tmdb,
        "From the server",
    )])]);
    cloud.with_rawg(vec![Ok(vec![candidate(
        Kind::Videogame,
        Source::Rawg,
        "From the server",
    )])]);
    let api = cloud.api("ana");
    let direct = Direct::with(|| Err(ProviderError::BadKey));

    let tmdb_result = tmdb_via(choose_search_access(Some("bad"), Some(&api)), &direct).await;
    let rawg_result = rawg_via(choose_search_access(Some("bad"), Some(&api)), &direct).await;

    assert!(matches!(tmdb_result, Err(ProviderError::BadKey)));
    assert!(matches!(rawg_result, Err(ProviderError::BadKey)));
    assert!(cloud.tmdb_asks().is_empty() && cloud.rawg_asks().is_empty());
}

#[tokio::test]
async fn tmdb_with_a_key_asks_nothing_for_a_kind_it_does_not_cover() {
    for kind in [
        Kind::Book,
        Kind::Music,
        Kind::Videogame,
        Kind::Comic,
        Kind::Boardgame,
        Kind::Other,
    ] {
        let direct = Direct::answering(vec![candidate(Kind::Film, Source::Tmdb, "Direct")]);
        let direct_clone = direct.clone();

        let found = search_tmdb(
            choose_search_access::<FakeApi>(Some("k"), None),
            kind,
            "matrix",
            "es",
            move |key| {
                let direct = direct_clone.clone();
                async move { direct.ask(key).await }
            },
        )
        .await
        .unwrap();

        assert!(found.is_empty(), "{kind:?}");
        assert!(direct.keys().is_empty(), "{kind:?}");
    }
}

// --- TMDB asked directly --------------------------------------------------------------------------------------------

#[tokio::test]
async fn tmdb_a_film_is_asked_with_the_token_in_a_header_and_read_with_its_detail() {
    let (api, stub) = provider(
        (200, SEARCH_MOVIE),
        vec![
            ("/movie/603", 200, MOVIE_603),
            (
                "/movie/604",
                200,
                r#"{"runtime":138,"credits":{"crew":[]}}"#,
            ),
        ],
    )
    .await;

    let found = tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix & más", "es-ES")
        .await
        .unwrap();

    assert_eq!(found.len(), 2);
    let matrix = &found[0];
    assert_eq!(matrix.source, Source::Tmdb);
    assert_eq!(matrix.title, "Matrix");
    assert_eq!(matrix.external_id.as_deref(), Some("603"));
    assert_eq!(matrix.published_year, Some(1999));
    assert_eq!(matrix.language.as_deref(), Some("en"));
    assert_eq!(matrix.duration_minutes, Some(136));
    assert_eq!(
        matrix.cover_url.as_deref(),
        Some("https://image.tmdb.org/t/p/w500/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg")
    );
    let directors: Vec<_> = matrix
        .contributors
        .iter()
        .map(|c| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(
        directors,
        vec![
            ("Lilly Wachowski", ContributorRole::Director),
            ("Lana Wachowski", ContributorRole::Director)
        ]
    );
    assert_eq!(found[1].duration_minutes, Some(138));

    let asked = stub.asked.lock().unwrap();
    let (path, query, auth) = &asked[0];
    assert_eq!(path, "/search/movie");
    assert_eq!(auth.as_deref(), Some(format!("Bearer {TOKEN}").as_str()));
    assert!(!query.contains(TOKEN));
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect();
    assert!(pairs.contains(&("query".to_owned(), "matrix & más".to_owned())));
    assert!(pairs.contains(&("include_adult".to_owned(), "false".to_owned())));
    assert!(pairs.contains(&("language".to_owned(), "es-ES".to_owned())));
    assert!(asked.iter().all(|(_, q, _)| !q.contains(TOKEN)));
}

#[tokio::test]
async fn tmdb_a_series_is_read_with_its_creators_and_seasons() {
    let (api, stub) = provider((200, SEARCH_TV), vec![("/tv/1396", 200, TV_1396)]).await;

    let found = tmdb::search(
        &client(),
        &api,
        TOKEN,
        Kind::TvSeries,
        "breaking bad",
        "en-US",
    )
    .await
    .unwrap();

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, Kind::TvSeries);
    assert_eq!(found[0].seasons, Some(5));
    assert_eq!(found[0].contributors[0].name, "Vince Gilligan");
    assert_eq!(
        stub.paths(),
        vec!["/search/tv".to_owned(), "/tv/1396".to_owned()]
    );
}

#[tokio::test]
async fn tmdb_a_detail_that_fails_does_not_cost_the_others_theirs() {
    let (api, _) = provider(
        (200, SEARCH_MOVIE),
        vec![
            ("/movie/603", 500, "oops"),
            (
                "/movie/604",
                200,
                r#"{"runtime":138,"credits":{"crew":[]}}"#,
            ),
        ],
    )
    .await;

    let found = tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
        .await
        .unwrap();

    assert_eq!(found[0].duration_minutes, None);
    assert_eq!(found[1].duration_minutes, Some(138));
}

#[tokio::test]
async fn rawg_a_detail_that_fails_does_not_cost_the_others_theirs() {
    let (api, _) = provider(
        (200, RAWG_SEARCH),
        vec![
            ("/games/3498", 500, "oops"),
            ("/games/3328", 200, RAWG_GAME_3498),
        ],
    )
    .await;

    let found = rawg::search(&client(), &api, TOKEN, "gta").await.unwrap();

    assert!(found[0].contributors.is_empty());
    assert_eq!(found[1].contributors[0].name, "Rockstar North");
}

#[tokio::test]
async fn tmdb_a_detail_that_fails_leaves_that_candidate_without_it() {
    let (api, _) = provider((200, SEARCH_MOVIE), vec![("/movie/603", 500, "oops")]).await;

    let found = tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
        .await
        .unwrap();

    assert_eq!(found.len(), 2);
    assert_eq!(found[0].duration_minutes, None);
    assert!(found[0].contributors.is_empty());
    assert_eq!(found[0].title, "Matrix");
}

fn hostile_tmdb() -> String {
    let mut results: Vec<Value> = (1..=100)
        .map(|n| json!({ "id": n, "title": format!("T{n}"), "poster_path": "/ok.jpg", "release_date": "2001-01-01" }))
        .collect();
    results.insert(0, json!({ "id": 7001, "title": "T".repeat(100_000), "original_language": "<script>",
        "poster_path": "/../../x.jpg", "release_date": "long ago", "overview": "o".repeat(10_000) }));
    results.insert(1, json!({ "id": "abc", "title": "string id" }));
    results.insert(2, json!({ "id": -4, "title": "negative id" }));
    results.insert(
        3,
        json!({ "id": 8002, "title": "evil cover", "poster_path": "https://evil.example/x.jpg" }),
    );
    json!({ "results": results }).to_string()
}

#[tokio::test]
async fn tmdb_rubbish_from_the_provider_gets_the_same_cuts_as_from_a_server() {
    let (api, stub) = provider((200, &hostile_tmdb()), vec![]).await;

    let found = tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
        .await
        .unwrap();

    assert_eq!(found.len(), 8);
    assert_eq!(found[0].external_id.as_deref(), Some("7001"));
    assert_eq!(found[0].title.chars().count(), 300);
    assert_eq!(found[0].language, None);
    assert_eq!(found[0].cover_url, None);
    assert_eq!(found[0].published_year, None);
    assert!(
        found
            .iter()
            .all(|c| c.external_id.as_deref() != Some("8002") || c.cover_url.is_none())
    );
    // Only numbers TMDB itself gave, and positive ones, are ever put in an address.
    for path in stub.paths().iter().skip(1) {
        assert!(
            path.strip_prefix("/movie/")
                .is_some_and(|id| id.parse::<u64>().is_ok_and(|n| n > 0)),
            "{path}"
        );
    }
    assert!(stub.paths().len() <= 1 + 8);
}

#[tokio::test]
async fn tmdb_an_enormous_answer_is_a_failure() {
    let huge = json!({ "results": [{ "id": 1, "title": "T" }], "padding": "x".repeat(6_000_000) })
        .to_string();
    let (api, _) = provider((200, &huge), vec![]).await;

    assert!(
        tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn tmdb_an_answer_that_is_not_json_is_a_failure() {
    let (api, _) = provider((200, "<html>hello</html>"), vec![]).await;

    assert!(
        tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn the_refusals_of_the_provider_are_told_as_what_they_are() {
    for (status, expected) in [
        (401, "bad_key"),
        (403, "bad_key"),
        (429, "quota"),
        (500, "unexpected"),
        (404, "unexpected"),
    ] {
        let (api, _) = provider((status, r#"{"status_message":"no"}"#), vec![]).await;

        let tmdb_error = tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
            .await
            .unwrap_err();
        let rawg_error = rawg::search(&client(), &api, TOKEN, "gta")
            .await
            .unwrap_err();

        for error in [tmdb_error, rawg_error] {
            let kind = match error {
                ProviderError::BadKey => "bad_key",
                ProviderError::Quota => "quota",
                ProviderError::Unexpected(_) => "unexpected",
                ProviderError::Network(_) => "network",
            };
            assert_eq!(kind, expected, "status {status}");
        }
    }
}

// --- RAWG asked directly --------------------------------------------------------------------------------------------

#[tokio::test]
async fn rawg_a_search_is_asked_with_the_key_in_the_address_and_the_detail_of_the_first_three_only()
{
    let many = json!({ "results": (1..=20).map(|n| json!({ "id": n, "name": format!("G{n}") })).collect::<Vec<_>>() }).to_string();
    let (api, stub) = provider((200, &many), vec![("/games/1", 200, RAWG_GAME_3498)]).await;

    let found = rawg::search(&client(), &api, TOKEN, "gta").await.unwrap();

    assert_eq!(found.len(), 8);
    let paths = stub.paths();
    assert_eq!(paths[0], "/games");
    let mut details: Vec<_> = paths[1..].to_vec();
    details.sort();
    assert_eq!(details, vec!["/games/1", "/games/2", "/games/3"]);
    let asked = stub.asked.lock().unwrap();
    let pairs: Vec<(String, String)> = url::form_urlencoded::parse(asked[0].1.as_bytes())
        .into_owned()
        .collect();
    assert!(pairs.contains(&("search".to_owned(), "gta".to_owned())));
    assert!(pairs.contains(&("page_size".to_owned(), "8".to_owned())));
    assert!(pairs.contains(&("key".to_owned(), TOKEN.to_owned())));
    assert!(asked.iter().all(|(_, _, auth)| auth.is_none()));
}

#[tokio::test]
async fn rawg_a_game_is_read_with_its_studio_publisher_and_platform() {
    let (api, _) = provider(
        (200, RAWG_SEARCH),
        vec![("/games/3498", 200, RAWG_GAME_3498)],
    )
    .await;

    let found = rawg::search(&client(), &api, TOKEN, "gta").await.unwrap();

    assert_eq!(found.len(), 2);
    let gta = &found[0];
    assert_eq!(gta.source, Source::Rawg);
    assert_eq!(gta.title, "Grand Theft Auto V");
    assert_eq!(gta.published_year, Some(2013));
    assert_eq!(gta.platform, None, "two platforms: the person chooses");
    assert_eq!(gta.publisher.as_deref(), Some("Rockstar Games"));
    assert_eq!(gta.contributors[0].name, "Rockstar North");
    assert_eq!(gta.contributors[0].role, ContributorRole::Studio);
    assert_eq!(
        found[1].platform.as_deref(),
        Some("PC"),
        "one platform: suggested"
    );
}

#[tokio::test]
async fn rawg_rubbish_from_the_provider_gets_the_same_cuts_as_from_a_server() {
    let mut results: Vec<Value> = (1..=100)
        .map(|n| json!({ "id": n, "name": format!("G{n}") }))
        .collect();
    results.insert(0, json!({ "id": 9001, "name": "T".repeat(100_000), "released": "long ago",
        "background_image": "https://evil.example/media/x.jpg",
        "platforms": (0..5_000).map(|n| json!({ "platform": { "name": format!("P{n}") } })).collect::<Vec<_>>() }));
    results.insert(1, json!({ "id": "x", "name": "string id" }));
    results.insert(1, json!({ "id": 99_999_999_999_u64, "name": "too big id" }));
    results.insert(1, json!({ "id": 0, "name": "zero id" }));
    results.insert(2, json!({ "id": 9002, "name": "ok", "background_image": "https://media.rawg.io/media/../x.jpg" }));
    let (api, stub) = provider((200, &json!({ "results": results }).to_string()), vec![]).await;

    let found = rawg::search(&client(), &api, TOKEN, "gta").await.unwrap();

    assert_eq!(found.len(), 8);
    assert_eq!(found[0].title.chars().count(), 300);
    assert_eq!(found[0].cover_url, None);
    assert_eq!(found[0].published_year, None);
    assert_eq!(found[0].platform, None);
    assert_eq!(found[1].cover_url, None);
    for path in stub.paths().iter().skip(1) {
        assert!(
            path.strip_prefix("/games/")
                .is_some_and(|id| id.parse::<u64>().is_ok_and(|n| n > 0)),
            "{path}"
        );
    }
    assert!(stub.paths().len() <= 1 + 3);
}

#[tokio::test]
async fn rawg_an_enormous_answer_is_a_failure() {
    let huge = json!({ "results": [{ "id": 1, "name": "G" }], "padding": "x".repeat(6_000_000) })
        .to_string();
    let (api, _) = provider((200, &huge), vec![]).await;

    assert!(rawg::search(&client(), &api, TOKEN, "gta").await.is_err());
}

// --- The key never reaches the screen --------------------------------------------------------------------------------

/// Everything an error could say, in every way it can be written.
fn written(error: &ProviderError) -> String {
    format!("{error} {error:?}")
}

#[tokio::test]
async fn no_error_carries_the_key_whatever_goes_wrong() {
    // A provider that echoes the address, the key and everything in its errors.
    for status in [400u16, 401, 403, 404, 429, 500, 502] {
        let echo = format!(r#"{{"detail":"bad key {TOKEN}","url":"/games?key={TOKEN}"}}"#);
        let (api, _) = provider((status, &echo), vec![]).await;

        let errors = [
            tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
                .await
                .unwrap_err(),
            rawg::search(&client(), &api, TOKEN, "gta")
                .await
                .unwrap_err(),
        ];

        for error in &errors {
            assert!(
                !written(error).contains(TOKEN),
                "status {status}: {}",
                written(error)
            );
        }
    }
}

#[tokio::test]
async fn no_error_carries_the_key_when_the_connection_itself_fails() {
    // Nobody listens there: the error of the http client names the address, and RAWG's address holds the key.
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };

    let errors = [
        tmdb::search(&client(), &closed, TOKEN, Kind::Film, "matrix", "es-ES")
            .await
            .unwrap_err(),
        rawg::search(&client(), &closed, TOKEN, "gta")
            .await
            .unwrap_err(),
    ];

    for error in &errors {
        assert!(matches!(error, ProviderError::Network(_)));
        assert!(!written(error).contains(TOKEN), "{}", written(error));
    }
}

#[tokio::test]
async fn no_error_carries_the_key_when_the_answer_cannot_be_read() {
    let (api, _) = provider((200, "not json at all"), vec![]).await;

    let errors = [
        tmdb::search(&client(), &api, TOKEN, Kind::Film, "matrix", "es-ES")
            .await
            .unwrap_err(),
        rawg::search(&client(), &api, TOKEN, "gta")
            .await
            .unwrap_err(),
    ];

    for error in &errors {
        assert!(!written(error).contains(TOKEN), "{}", written(error));
    }
}
