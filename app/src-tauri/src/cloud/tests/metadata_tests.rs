//! Google Books through the server (7.8, D83). The server is somebody else's: it may be slow, down, lying or
//! hostile, and it learns the ISBN of a person who did not ask for it unless they let it. So: with a key of their own
//! it is never asked, one that does not offer the search never gets the request, and whatever it answers is cut
//! down to the fixed shape before it reaches the form.

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::*;
use crate::metadata::ProviderError;
use crate::metadata::via_server::{GoogleAccess, choose_access, lookup_google, parse_answer};

fn isbn() -> Isbn {
    Isbn::parse("9788408176039").unwrap()
}

fn book(title: &str) -> Metadata {
    Metadata {
        title: Some(title.to_owned()),
        authors: vec!["Dan Brown".to_owned()],
        ..Metadata::default()
    }
}

/// Google asked directly: records with which key, and answers what the test says.
#[derive(Clone)]
struct Direct {
    keys: Arc<Mutex<Vec<Option<String>>>>,
    answer: Arc<dyn Fn() -> std::result::Result<Option<Metadata>, ProviderError> + Send + Sync>,
}

impl Direct {
    fn answering(book: Option<Metadata>) -> Self {
        Self::with(move || Ok(book.clone()))
    }

    fn failing() -> Self {
        Self::with(|| Err(ProviderError::Quota))
    }

    fn with(
        answer: impl Fn() -> std::result::Result<Option<Metadata>, ProviderError>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            keys: Arc::default(),
            answer: Arc::new(answer),
        }
    }

    fn keys(&self) -> Vec<Option<String>> {
        self.keys.lock().unwrap().clone()
    }

    async fn ask(
        &self,
        key: Option<String>,
    ) -> std::result::Result<Option<Metadata>, ProviderError> {
        self.keys.lock().unwrap().push(key);
        (self.answer)()
    }
}

async fn run(
    access: GoogleAccess<'_, FakeApi>,
    direct: &Direct,
) -> std::result::Result<Option<Metadata>, ProviderError> {
    let direct = direct.clone();
    lookup_google(access, &isbn(), move |key| {
        let direct = direct.clone();
        async move { direct.ask(key).await }
    })
    .await
}

// --- Who may be asked ---------------------------------------------------------------------------------------------

fn access_name(access: &GoogleAccess<'_, FakeApi>) -> &'static str {
    match access {
        GoogleAccess::Key(_) => "key",
        GoogleAccess::Server(_) => "server",
        GoogleAccess::Anonymous => "anonymous",
    }
}

#[test]
fn their_key_wins_over_the_server_and_the_server_needs_a_session_and_the_setting() {
    let api = FakeCloud::default().api("ana");
    let cases = [
        // (key, setting on, session, who asks)
        (Some("k"), true, true, "key"),
        (Some("k"), false, false, "key"),
        (None, true, true, "server"),
        (None, false, true, "anonymous"),
        (None, true, false, "anonymous"),
        (None, false, false, "anonymous"),
        (Some("  "), true, true, "server"),
        (Some(""), true, true, "server"),
    ];
    for (key, setting, session, expected) in cases {
        let access = choose_access(key, setting, session.then_some(&api));
        assert_eq!(
            access_name(&access),
            expected,
            "{key:?} {setting} {session}"
        );
    }
}

// --- Who is asked -------------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_key_of_their_own_goes_straight_to_google_and_the_server_is_never_asked() {
    let cloud = FakeCloud::default();
    cloud.with_google_books(vec![Ok(Some(book("From the server")))]);
    let api = cloud.api("ana");
    let direct = Direct::answering(Some(book("From Google")));

    let found = run(GoogleAccess::Key("their-key"), &direct).await.unwrap();

    assert_eq!(found.unwrap().title.as_deref(), Some("From Google"));
    assert_eq!(direct.keys(), vec![Some("their-key".to_owned())]);
    assert!(cloud.book_asks().is_empty());
    assert_eq!(cloud.0.lock().unwrap().offer_checks, 0);
    let _ = api;
}

#[tokio::test]
async fn a_server_that_offers_it_answers_and_google_is_not_asked() {
    let cloud = FakeCloud::default();
    cloud.with_google_books(vec![Ok(Some(book("From the server")))]);
    let api = cloud.api("ana");
    let direct = Direct::answering(Some(book("From Google")));

    let found = run(GoogleAccess::Server(&api), &direct).await.unwrap();

    assert_eq!(found.unwrap().title.as_deref(), Some("From the server"));
    assert_eq!(cloud.book_asks(), vec!["9788408176039".to_owned()]);
    assert!(direct.keys().is_empty());
}

#[tokio::test]
async fn a_server_that_does_not_offer_it_never_gets_the_request() {
    let cloud = FakeCloud::default(); // offers nothing
    let api = cloud.api("ana");
    let direct = Direct::answering(Some(book("From Google")));

    let found = run(GoogleAccess::Server(&api), &direct).await.unwrap();

    assert_eq!(found.unwrap().title.as_deref(), Some("From Google"));
    assert_eq!(direct.keys(), vec![None]);
    assert!(cloud.book_asks().is_empty());
}

#[tokio::test]
async fn a_server_that_offers_other_catalogues_but_not_this_one_never_gets_the_request() {
    let cloud = FakeCloud::default();
    cloud.0.lock().unwrap().metadata = vec!["open_library".to_owned(), "google_books ".to_owned()];
    let api = cloud.api("ana");
    let direct = Direct::answering(None);

    run(GoogleAccess::Server(&api), &direct).await.unwrap();

    assert!(cloud.book_asks().is_empty());
    assert_eq!(direct.keys(), vec![None]);
}

#[tokio::test]
async fn not_being_able_to_read_what_the_server_offers_counts_as_not_offered() {
    let cloud = FakeCloud::default();
    cloud.with_google_books(vec![Ok(Some(book("From the server")))]);
    cloud.0.lock().unwrap().offer_fails = true;
    let api = cloud.api("ana");
    let direct = Direct::answering(Some(book("From Google")));

    let found = run(GoogleAccess::Server(&api), &direct).await.unwrap();

    assert_eq!(found.unwrap().title.as_deref(), Some("From Google"));
    assert!(cloud.book_asks().is_empty());
}

#[tokio::test]
async fn no_key_and_no_account_is_google_without_a_key() {
    let direct = Direct::answering(Some(book("From Google")));

    let found = run(GoogleAccess::Anonymous, &direct).await.unwrap();

    assert_eq!(found.unwrap().title.as_deref(), Some("From Google"));
    assert_eq!(direct.keys(), vec![None]);
}

// --- When the server fails ----------------------------------------------------------------------------------------

#[tokio::test]
async fn whatever_the_server_fails_at_google_without_a_key_answers() {
    for code in [
        "server_unavailable",
        "metadata_quota_exhausted",
        "metadata_failed",
        "metadata_unavailable",
        "too_many_requests",
        "invalid_token",
        "account_blocked",
        "invalid_isbn",
    ] {
        let cloud = FakeCloud::default();
        cloud.with_google_books(vec![Err(code)]);
        let api = cloud.api("ana");
        let direct = Direct::answering(Some(book("From Google")));

        let found = run(GoogleAccess::Server(&api), &direct).await;

        assert_eq!(
            found.unwrap().unwrap().title.as_deref(),
            Some("From Google"),
            "after {code}"
        );
        assert_eq!(direct.keys(), vec![None], "after {code}");
    }
}

#[tokio::test]
async fn if_the_server_and_google_both_fail_the_usual_error_stays() {
    let cloud = FakeCloud::default();
    cloud.with_google_books(vec![Err("metadata_quota_exhausted")]);
    let api = cloud.api("ana");
    let direct = Direct::failing();

    let result = run(GoogleAccess::Server(&api), &direct).await;

    assert!(matches!(result, Err(ProviderError::Quota)));
}

#[tokio::test]
async fn a_book_the_server_does_not_know_is_an_answer_and_not_a_failure() {
    let cloud = FakeCloud::default();
    cloud.with_google_books(vec![Ok(None)]);
    let api = cloud.api("ana");
    let direct = Direct::answering(Some(book("From Google")));

    let found = run(GoogleAccess::Server(&api), &direct).await.unwrap();

    assert_eq!(found, None);
    assert!(direct.keys().is_empty());
}

// --- What the server says -----------------------------------------------------------------------------------------

#[test]
fn the_fixed_shape_is_read() {
    let metadata = parse_answer(&json!({
        "found": true, "title": "Inferno", "subtitle": "A novel", "authors": ["Dan Brown"],
        "publisher": "Planeta", "published_year": 2013, "language": "es",
        "cover_url": "https://books.google.com/books/content?id=abc",
        "external_id": "abc123",
    }))
    .unwrap()
    .unwrap();

    assert_eq!(metadata.title.as_deref(), Some("Inferno"));
    assert_eq!(metadata.subtitle.as_deref(), Some("A novel"));
    assert_eq!(metadata.authors, vec!["Dan Brown"]);
    assert_eq!(metadata.publisher.as_deref(), Some("Planeta"));
    assert_eq!(metadata.published_year, Some(2013));
    assert_eq!(metadata.language.as_deref(), Some("es"));
    assert_eq!(
        metadata.cover_url.as_deref(),
        Some("https://books.google.com/books/content?id=abc")
    );
    assert_eq!(metadata.external_id.as_deref(), Some("abc123"));
}

#[test]
fn not_found_is_none() {
    assert_eq!(parse_answer(&json!({ "found": false })), Ok(None));
}

#[test]
fn an_answer_that_is_not_the_shape_is_a_failure() {
    for body in [
        json!(null),
        json!("text"),
        json!([1, 2]),
        json!({}),
        json!({ "found": "yes" }),
        json!({ "found": 1 }),
        json!({ "title": "no found" }),
    ] {
        assert_eq!(parse_answer(&body), Err(()), "for {body}");
    }
}

#[test]
fn fields_of_the_wrong_type_are_dropped_and_extra_ones_ignored() {
    let metadata = parse_answer(&json!({
        "found": true, "title": "Inferno", "subtitle": ["array"], "authors": "nobody",
        "publisher": 42, "published_year": "2013", "language": {"x": 1},
        "cover_url": 5, "external_id": null, "evil": "<script>", "nested": {"a": [1, 2, 3]},
    }))
    .unwrap()
    .unwrap();

    assert_eq!(metadata.title.as_deref(), Some("Inferno"));
    assert_eq!(metadata.subtitle, None);
    assert!(metadata.authors.is_empty());
    assert_eq!(metadata.publisher, None);
    assert_eq!(metadata.published_year, None);
    assert_eq!(metadata.language, None);
    assert_eq!(metadata.cover_url, None);
    assert_eq!(metadata.external_id, None);
}

#[test]
fn enormous_fields_are_cut() {
    let authors: Vec<String> = (0..100_000).map(|n| format!("Author {n}")).collect();
    let metadata = parse_answer(&json!({
        "found": true, "title": "T".repeat(1_000_000), "subtitle": "S".repeat(1_000_000),
        "authors": authors, "publisher": "P".repeat(1_000_000),
        "language": "l".repeat(1_000_000), "external_id": "i".repeat(1_000_000),
    }))
    .unwrap()
    .unwrap();

    assert_eq!(metadata.title.unwrap().chars().count(), 300);
    assert_eq!(metadata.subtitle.unwrap().chars().count(), 300);
    assert_eq!(metadata.authors.len(), 20);
    assert_eq!(metadata.publisher.unwrap().chars().count(), 200);
    assert_eq!(metadata.language, None);
    assert_eq!(metadata.external_id, None);
}

#[test]
fn a_cover_that_is_not_from_google_books_is_dropped() {
    for cover in [
        "https://evil.example/c.jpg",
        "http://books.google.com/c.jpg",
        "https://books.google.com.evil.example/c.jpg",
        "https://books.google.com@evil.example/c.jpg",
        "https://user@books.google.com/c.jpg",
        "https://books.google.com:8443/c.jpg",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:image/png;base64,AAAA",
        "//books.google.com/c.jpg",
        "https://127.0.0.1/c.jpg",
        "https://books.google.com/c.jpg\nHost: evil",
        "",
    ] {
        let metadata = parse_answer(&json!({ "found": true, "title": "T", "cover_url": cover }))
            .unwrap()
            .unwrap();
        assert_eq!(metadata.cover_url, None, "kept {cover:?}");
    }
}

#[test]
fn text_with_control_characters_is_cleaned() {
    let metadata = parse_answer(
        &json!({ "found": true, "title": "In\u{0}fer\nno\u{7}", "authors": ["  ", "Dan\tBrown"] }),
    )
    .unwrap()
    .unwrap();

    assert_eq!(metadata.title.as_deref(), Some("In fer no"));
    assert_eq!(metadata.authors, vec!["Dan Brown"]);
}

#[test]
fn a_book_with_neither_title_nor_authors_is_not_found() {
    assert_eq!(
        parse_answer(&json!({ "found": true, "publisher": "Planeta" })),
        Ok(None)
    );
}

#[test]
fn a_year_that_cannot_be_one_is_dropped() {
    for year in [
        json!(-5),
        json!(99),
        json!(99_999),
        json!(1e12),
        json!(2013.5),
    ] {
        let metadata =
            parse_answer(&json!({ "found": true, "title": "T", "published_year": year }))
                .unwrap()
                .unwrap();
        assert_eq!(metadata.published_year, None, "kept {year}");
    }
}
