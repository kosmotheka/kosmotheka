//! The live-notices loop against a real HTTP server on 127.0.0.1 that speaks just
//! enough HTTP/1.1 by hand: the server side of a Server-Sent Events stream is a
//! response that never ends, which axum cannot produce without a stream crate this
//! project does not use. No test touches the network beyond the loopback.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use super::*;
use crate::account::{AccountUser, store_session};

// --- The parser --------------------------------------------------------------

fn parse_all(chunks: &[&[u8]]) -> Vec<SseEvent> {
    let mut parser = SseParser::default();
    chunks.iter().flat_map(|chunk| parser.feed(chunk)).collect()
}

fn data(text: &str) -> SseEvent {
    SseEvent {
        data: text.to_owned(),
        ..SseEvent::default()
    }
}

#[test]
fn parses_an_event_with_lf_lines() {
    let events = parse_all(&[b"id: 7\nevent: update\ndata: {}\n\n"]);
    assert_eq!(
        events,
        vec![SseEvent {
            id: Some("7".to_owned()),
            event: Some("update".to_owned()),
            data: "{}".to_owned()
        }]
    );
}

#[test]
fn parses_crlf_lines() {
    assert_eq!(parse_all(&[b"id:1\r\ndata:{}\r\n\r\n"])[0].data, "{}");
}

#[test]
fn an_event_split_across_chunks_arrives_once_whole() {
    let events = parse_all(&[b"id: 1\nda", b"ta: {", b"}\n", b"\r", b"\n"]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, "{}");
}

#[test]
fn a_split_in_the_middle_of_crlf_still_works() {
    let events = parse_all(&[b"data: a\r", b"\n\r", b"\ndata: b\r\n\r\n"]);
    assert_eq!(events, vec![data("a"), data("b")]);
}

#[test]
fn comments_and_heartbeats_are_ignored() {
    assert!(parse_all(&[b": keep-alive\n\n: again\n"]).is_empty());
    let events = parse_all(&[b":hb\n\ndata: x\n:inside\n\n"]);
    assert_eq!(events, vec![data("x")]);
}

#[test]
fn several_events_in_one_chunk() {
    let events = parse_all(&[b"data: 1\n\ndata: 2\n\ndata: 3\n\n"]);
    assert_eq!(events.len(), 3);
}

#[test]
fn a_final_event_without_its_blank_line_is_not_emitted() {
    assert!(parse_all(&[b"data: {}\n"]).is_empty());
    assert!(parse_all(&[b"data: {}"]).is_empty());
}

#[test]
fn multi_line_data_joins_with_newlines_and_retry_is_skipped() {
    let events = parse_all(&[b"retry: 3000\ndata: a\ndata: b\n\n"]);
    assert_eq!(events, vec![data("a\nb")]);
}

#[test]
fn a_blank_line_alone_is_not_an_event() {
    assert!(parse_all(&[b"\n\n\r\n"]).is_empty());
}

// --- Dates -------------------------------------------------------------------

/// The inverse of `parse_utc`, for the test server's `expiresAt`.
fn iso(epoch: u64) -> String {
    let days = i64::try_from(epoch / 86_400).unwrap();
    let secs = epoch % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}+00:00",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

#[test]
fn reads_utc_dates_in_the_usual_spellings() {
    for text in [
        "2023-11-14T22:13:20+00:00",
        "2023-11-14T22:13:20Z",
        "2023-11-14T22:13:20.123456Z",
        "2023-11-14 22:13:20",
    ] {
        assert_eq!(parse_utc(text), Some(1_700_000_000), "{text}");
    }
    assert_eq!(parse_utc("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(parse_utc("2024-02-29T00:00:00Z"), Some(1_709_164_800));
    assert_eq!(parse_utc("soon"), None);
    assert_eq!(parse_utc("2023-13-xxT22:13:20Z"), None);
}

#[test]
fn the_test_formatter_and_the_parser_agree() {
    for epoch in [0, 951_782_400, 1_700_000_000, 1_999_999_999] {
        assert_eq!(parse_utc(&iso(epoch)), Some(epoch));
    }
}

#[test]
fn time_left_counts_down_and_stops_at_the_past() {
    let now = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    assert_eq!(
        time_left("2023-11-14T22:23:20Z", now),
        Some(Duration::from_secs(600))
    );
    assert_eq!(time_left("2023-11-14T22:00:00Z", now), None);
    assert_eq!(time_left("whenever", now), None);
}

// --- A hub on the loopback ---------------------------------------------------

#[derive(Clone)]
enum Step {
    Send(&'static [u8]),
    Wait(u64),
    /// Keeps the connection open, silent, until the client leaves.
    Hold,
}

#[derive(Clone)]
struct Config {
    events: bool,
    /// Status and body the ticket route answers with instead of a ticket.
    ticket_error: Option<(u16, &'static str)>,
    lifetime_secs: u64,
    /// One script per hub connection; the last one serves every later connection.
    hub: Vec<Vec<Step>>,
}

impl Config {
    fn hold() -> Self {
        Self {
            events: true,
            ticket_error: None,
            lifetime_secs: 3600,
            hub: vec![vec![Step::Hold]],
        }
    }

    fn with_hub(hub: Vec<Vec<Step>>) -> Self {
        Self {
            hub,
            ..Self::hold()
        }
    }

    fn failing_ticket(status: u16, body: &'static str) -> Self {
        Self {
            ticket_error: Some((status, body)),
            ..Self::hold()
        }
    }
}

struct Server {
    port: u16,
    ticket_calls: Arc<AtomicUsize>,
    hub_calls: Arc<AtomicUsize>,
    topics_seen: Arc<Mutex<Vec<String>>>,
}

async fn respond_json(stream: &mut TcpStream, status: u16, body: &str) {
    let text = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(text.as_bytes()).await;
}

async fn serve(
    mut stream: TcpStream,
    config: Config,
    server: (Arc<AtomicUsize>, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>),
) {
    // Small writes must leave at once, or Nagle and delayed ACKs spread a "burst" over
    // tens of milliseconds.
    let _ = stream.set_nodelay(true);
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }
    let text = String::from_utf8_lossy(&request).into_owned();
    let target = text.split_whitespace().nth(1).unwrap_or("").to_owned();
    let (ticket_calls, hub_calls, topics_seen) = server;
    if target == "/v1/server" {
        let body = format!(
            r#"{{"name":"t","api_version":1,"registration":true,"sign_in_with_google":false,"two_factor":false,"events":{}}}"#,
            config.events
        );
        respond_json(&mut stream, 200, &body).await;
    } else if target == "/v1/cloud/events" {
        ticket_calls.fetch_add(1, Ordering::SeqCst);
        if let Some((status, body)) = config.ticket_error {
            respond_json(&mut stream, status, body).await;
            return;
        }
        let expires = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + config.lifetime_secs;
        let body = format!(
            r#"{{"token":"jwt","topics":["urn:a","urn:b"],"expiresAt":"{}"}}"#,
            iso(expires)
        );
        respond_json(&mut stream, 200, &body).await;
    } else if target.starts_with("/.well-known/mercure?") {
        let index = hub_calls.fetch_add(1, Ordering::SeqCst);
        let authorized = text.to_lowercase().contains("authorization: bearer jwt");
        if !authorized {
            respond_json(&mut stream, 401, "").await;
            return;
        }
        *topics_seen.lock().unwrap() = target
            .split_once('?')
            .unwrap()
            .1
            .split('&')
            .map(str::to_owned)
            .collect();
        let _ = stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .await;
        for step in &config.hub[index.min(config.hub.len() - 1)] {
            match step {
                Step::Send(bytes) => {
                    if stream.write_all(bytes).await.is_err() || stream.flush().await.is_err() {
                        return;
                    }
                }
                Step::Wait(ms) => tokio::time::sleep(Duration::from_millis(*ms)).await,
                Step::Hold => {
                    // Returns when the client closes its side.
                    let mut sink = [0u8; 16];
                    while matches!(stream.read(&mut sink).await, Ok(read) if read > 0) {}
                    return;
                }
            }
        }
        // End of the script: the connection closes.
    } else {
        respond_json(&mut stream, 404, "{}").await;
    }
}

async fn start_server(config: Config) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let ticket_calls = Arc::new(AtomicUsize::new(0));
    let hub_calls = Arc::new(AtomicUsize::new(0));
    let topics_seen = Arc::new(Mutex::new(Vec::new()));
    let (t, h, s) = (ticket_calls.clone(), hub_calls.clone(), topics_seen.clone());
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve(
                stream,
                config.clone(),
                (t.clone(), h.clone(), s.clone()),
            ));
        }
    });
    Server {
        port,
        ticket_calls,
        hub_calls,
        topics_seen,
    }
}

fn signed_in_database(port: u16) -> Database {
    let database = Database::in_memory();
    account::set_server(&database, Some(format!("http://127.0.0.1:{port}"))).unwrap();
    let user = AccountUser {
        id: "u".to_owned(),
        email: "a@b.c".to_owned(),
        name: "A".to_owned(),
        locale: "en".to_owned(),
    };
    store_session(&database.lock().unwrap(), "device-token", &user, "a@b.c").unwrap();
    database
}

/// `fast()` with an idle timeout that a short test never reaches.
fn patient() -> Timing {
    Timing {
        idle: Duration::from_secs(30),
        ..fast()
    }
}

fn fast() -> Timing {
    Timing {
        idle: Duration::from_millis(300),
        renew_margin: Duration::ZERO,
        min_renew: Duration::ZERO,
        backoff_first: Duration::from_millis(20),
        backoff_max: Duration::from_millis(100),
        coalesce: Duration::from_millis(100),
    }
}

struct Running {
    server: Server,
    database: Database,
    signals: Arc<Mutex<Vec<Signal>>>,
    wake: Arc<Notify>,
    task: JoinHandle<()>,
}

impl Running {
    fn signals(&self) -> Vec<Signal> {
        self.signals.lock().unwrap().clone()
    }

    fn changed(&self) -> usize {
        self.signals()
            .iter()
            .filter(|signal| **signal == Signal::Changed)
            .count()
    }

    fn tickets(&self) -> usize {
        self.server.ticket_calls.load(Ordering::SeqCst)
    }

    fn hubs(&self) -> usize {
        self.server.hub_calls.load(Ordering::SeqCst)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn launch(config: Config, timing: Timing) -> Running {
    let server = start_server(config).await;
    let database = signed_in_database(server.port);
    let signals = Arc::new(Mutex::new(Vec::new()));
    let wake = Arc::new(Notify::new());
    let api = HttpEvents::new(database.clone()).unwrap();
    let task = {
        let (signals, wake) = (signals.clone(), wake.clone());
        tokio::spawn(async move {
            let emit = move |signal| signals.lock().unwrap().push(signal);
            run(&api, &timing, &wake, &emit).await;
        })
    };
    Running {
        server,
        database,
        signals,
        wake,
        task,
    }
}

async fn eventually(what: &str, condition: impl Fn() -> bool) {
    for _ in 0..100 {
        if condition() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("never happened: {what}");
}

async fn settle(millis: u64) {
    tokio::time::sleep(Duration::from_millis(millis)).await;
}

const EVENT: &[u8] = b"id: urn:uuid:1\ndata: {}\n\n";

#[tokio::test(flavor = "multi_thread")]
async fn connects_with_the_topics_and_catches_up() {
    let running = launch(Config::hold(), patient()).await;
    eventually("live", || running.signals().contains(&Signal::Live(true))).await;
    settle(50).await;
    assert_eq!(running.signals(), vec![Signal::Live(true), Signal::Changed]);
    assert_eq!(
        *running.server.topics_seen.lock().unwrap(),
        vec!["topic=urn%3Aa", "topic=urn%3Ab"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_event_gives_one_cloud_changed() {
    let running = launch(
        Config::with_hub(vec![vec![Step::Wait(150), Step::Send(EVENT), Step::Hold]]),
        patient(),
    )
    .await;
    eventually("the event's emit", || running.changed() == 2).await;
    settle(300).await;
    // The catch-up on connecting, and the event itself.
    assert_eq!(running.changed(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_burst_of_events_is_folded_into_one_or_two() {
    let mut script = vec![Step::Wait(150)];
    for _ in 0..5 {
        script.push(Step::Send(EVENT));
        script.push(Step::Wait(5));
    }
    script.push(Step::Hold);
    let running = launch(Config::with_hub(vec![script]), patient()).await;
    settle(700).await;
    let after_catch_up = running.changed() - 1;
    assert!((1..=2).contains(&after_catch_up), "{after_catch_up} emits");
}

#[tokio::test(flavor = "multi_thread")]
async fn heartbeats_alone_never_emit_a_change() {
    let running = launch(
        Config::with_hub(vec![vec![
            Step::Send(b": hb\n\n"),
            Step::Wait(100),
            Step::Send(b": hb\n\n"),
            Step::Hold,
        ]]),
        patient(),
    )
    .await;
    settle(250).await;
    assert_eq!(running.changed(), 1, "only the catch-up");
}

#[tokio::test(flavor = "multi_thread")]
async fn when_the_server_closes_the_stream_it_reconnects_and_catches_up() {
    let running = launch(
        Config::with_hub(vec![vec![Step::Send(b": bye\n\n")], vec![Step::Hold]]),
        fast(),
    )
    .await;
    eventually("second connection", || running.hubs() == 2).await;
    eventually("live again", || {
        running
            .signals()
            .iter()
            .filter(|s| **s == Signal::Live(true))
            .count()
            == 2
    })
    .await;
    settle(50).await;
    assert_eq!(
        running.signals(),
        vec![
            Signal::Live(true),
            Signal::Changed,
            Signal::Live(false),
            Signal::Live(true),
            Signal::Changed,
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn silence_beyond_the_timeout_reconnects() {
    let running = launch(Config::hold(), fast()).await;
    eventually("a second connection", || running.hubs() >= 2).await;
    assert!(running.signals().contains(&Signal::Live(false)));
}

#[tokio::test(flavor = "multi_thread")]
async fn heartbeats_keep_a_quiet_connection_alive() {
    let mut script = Vec::new();
    for _ in 0..8 {
        script.push(Step::Send(b": hb\n\n"));
        script.push(Step::Wait(100));
    }
    script.push(Step::Hold);
    let running = launch(Config::with_hub(vec![script]), fast()).await;
    // 600 ms is twice the idle timeout, but something arrives every 100 ms.
    settle(600).await;
    assert_eq!(running.hubs(), 1);
    assert!(!running.signals().contains(&Signal::Live(false)));
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_hub_on_the_server_it_stops_without_asking_for_a_ticket() {
    let mut config = Config::hold();
    config.events = false;
    let running = launch(config, fast()).await;
    eventually("the loop ends", || running.task.is_finished()).await;
    assert_eq!(running.signals(), vec![Signal::Stopped]);
    assert_eq!((running.tickets(), running.hubs()), (0, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn events_unavailable_stops_the_task_without_retrying() {
    let running = launch(
        Config::failing_ticket(409, r#"{"error":"events_unavailable"}"#),
        fast(),
    )
    .await;
    eventually("the loop ends", || running.task.is_finished()).await;
    settle(200).await;
    assert_eq!(running.signals(), vec![Signal::Stopped]);
    assert_eq!((running.tickets(), running.hubs()), (1, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_account_error_that_ends_the_session_stops_the_task() {
    for code in [
        "device_removed",
        "signed_out_password_changed",
        "invalid_token",
    ] {
        let body: &'static str = Box::leak(format!(r#"{{"error":"{code}"}}"#).into_boxed_str());
        let running = launch(Config::failing_ticket(401, body), fast()).await;
        eventually("the loop ends", || running.task.is_finished()).await;
        assert_eq!(running.signals(), vec![Signal::Stopped], "{code}");
        assert_eq!(running.tickets(), 1, "{code}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_error_is_retried_with_backoff_not_given_up() {
    let running = launch(Config::failing_ticket(503, ""), fast()).await;
    eventually("several tries", || running.tickets() >= 3).await;
    assert!(!running.task.is_finished());
    assert!(running.signals().is_empty(), "never live, never stopped");
}

#[tokio::test(flavor = "multi_thread")]
async fn waking_the_loop_skips_the_backoff_wait() {
    let mut timing = fast();
    timing.backoff_first = Duration::from_secs(30);
    timing.backoff_max = Duration::from_secs(30);
    let running = launch(Config::failing_ticket(503, ""), timing).await;
    eventually("first try", || running.tickets() == 1).await;
    settle(100).await;
    assert_eq!(running.tickets(), 1, "waiting out the backoff");
    running.wake.notify_one();
    eventually("a try right after the wake", || running.tickets() == 2).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_token_near_its_expiry_is_renewed_before_it_runs_out() {
    let mut config = Config::hold();
    config.lifetime_secs = 3;
    let mut timing = fast();
    timing.idle = Duration::from_secs(30);
    // Renews 2 s before the end: after about a second.
    timing.renew_margin = Duration::from_secs(2);
    let running = launch(config, timing).await;
    eventually("a second ticket", || running.tickets() >= 2).await;
    eventually("a second connection", || running.hubs() >= 2).await;
    assert!(
        !running.signals().contains(&Signal::Live(false)),
        "renewing is not going down: {:?}",
        running.signals()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_database_stays_free_while_connected_and_while_waiting() {
    let mut timing = fast();
    timing.idle = Duration::from_secs(30);
    let running = launch(Config::hold(), timing).await;
    eventually("live", || running.signals().contains(&Signal::Live(true))).await;
    // A guard held across an `.await` in the loop would block this lock for as long
    // as the connection lasts.
    for _ in 0..3 {
        let database = running.database.clone();
        let locked = tokio::time::timeout(
            Duration::from_secs(1),
            tokio::task::spawn_blocking(move || drop(database.lock().unwrap())),
        )
        .await;
        assert!(locked.is_ok(), "the database was locked by the live task");
        settle(50).await;
    }
}
