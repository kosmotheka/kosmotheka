//! Live notices from the cloud (D75, milestone 7.3): one connection to the server's
//! Mercure hub while the app is open. Every event means «something changed in one of
//! your cloud libraries»; the app then syncs by the usual protocol (D39).
//!
//! The loop (`run`) only knows the `EventsApi` trait and an `emit` closure, so the tests
//! drive it without Tauri. The database guard never crosses an `.await` here: the only
//! code that touches the database is `HttpEvents`, and each of its methods reads what it
//! needs in a one-line statement (`let (..) = authed_context(..)?;`, which locks, copies
//! and drops the guard) before the first `.await`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Method;
use serde::{Deserialize, Serialize};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;
use tokio::time::{Instant, sleep, sleep_until, timeout};

use crate::account::client::{self, Auth, send_json};
use crate::account::{self, ResetDeps};
use crate::cloud::invitations;
use crate::commands;
use crate::db::Database;
use crate::error::{Error, Result};

// --- SSE parsing -------------------------------------------------------------

/// One Server-Sent Event: the fields the hub sends. Only «there was one» matters to the
/// app; the fields are kept so the parser is honest and testable.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
}

/// Incremental parser: feed it the chunks as they arrive, in any split. Lines end in
/// `\n` or `\r\n`; lines starting with `:` are comments (heartbeats); an event is
/// complete only at its blank line, so one cut off by the connection is never emitted.
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
    current: SseEvent,
    started: bool,
}

impl SseParser {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.buffer.extend_from_slice(bytes);
        let mut done = Vec::new();
        while let Some(end) = self.buffer.iter().position(|&byte| byte == b'\n') {
            let raw: Vec<u8> = self.buffer.drain(..=end).collect();
            let line = String::from_utf8_lossy(&raw[..end]);
            let line = line.strip_suffix('\r').unwrap_or(&line);
            if line.is_empty() {
                if self.started {
                    done.push(std::mem::take(&mut self.current));
                    self.started = false;
                }
            } else if !line.starts_with(':') {
                self.field(line);
            }
        }
        done
    }

    fn field(&mut self, line: &str) {
        let (name, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match name {
            "data" => {
                if self.started && !self.current.data.is_empty() {
                    self.current.data.push('\n');
                }
                self.current.data.push_str(value);
            }
            "id" => self.current.id = Some(value.to_owned()),
            "event" => self.current.event = Some(value.to_owned()),
            // `retry:` and unknown fields: nothing to do.
            _ => return,
        }
        self.started = true;
    }
}

// --- Dates -------------------------------------------------------------------

/// Days since 1970-01-01 of a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Seconds since the epoch of `YYYY-MM-DDTHH:MM:SS` followed by anything (a fraction,
/// `Z`, `+00:00`): the server sends UTC, so what follows the seconds is ignored.
fn parse_utc(text: &str) -> Option<u64> {
    let head = text.get(..19)?;
    let number = |range: std::ops::Range<usize>| head.get(range)?.parse::<i64>().ok();
    if head.as_bytes()[10] != b'T' && head.as_bytes()[10] != b' ' {
        return None;
    }
    let days = days_from_civil(number(0..4)?, number(5..7)?, number(8..10)?);
    let seconds = days * 86_400 + number(11..13)? * 3600 + number(14..16)? * 60 + number(17..19)?;
    u64::try_from(seconds).ok()
}

/// How long until `expires_at`, or `None` when it is not a date or already past.
pub(super) fn time_left(expires_at: &str, now: SystemTime) -> Option<Duration> {
    let expires = parse_utc(expires_at)?;
    let now = now.duration_since(UNIX_EPOCH).ok()?.as_secs();
    expires.checked_sub(now).map(Duration::from_secs)
}

// --- The network, behind a trait ---------------------------------------------

/// What the server hands out to subscribe: a token for the hub, and the topics it allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub token: String,
    pub topics: Vec<String>,
    pub lifetime: Duration,
}

/// The stream of bytes the hub answers with.
#[allow(async_fn_in_trait)]
pub trait ChunkStream {
    /// `None` when the server closed the stream.
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>>;
}

#[allow(async_fn_in_trait)]
pub trait EventsApi {
    type Stream: ChunkStream;
    /// `GET /v1/server`: does this server run a hub?
    async fn server_has_events(&self) -> Result<bool>;
    /// `GET /v1/cloud/events`.
    async fn ticket(&self) -> Result<Ticket>;
    /// Subscribes to the hub with the ticket.
    async fn open(&self, ticket: &Ticket) -> Result<Self::Stream>;
    /// Lets the app react to an account error (sign-out, device reset). The loop only
    /// decides whether to go on.
    async fn account_error(&self, _error: &Error) {}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TicketWire {
    token: String,
    topics: Vec<String>,
    expires_at: String,
}

/// Used when the server's date cannot be read: renew in a few minutes anyway.
const FALLBACK_LIFETIME: Duration = Duration::from_secs(300);

pub struct HttpEvents {
    database: Database,
    api: reqwest::Client,
    hub: reqwest::Client,
}

impl HttpEvents {
    pub fn new(database: Database) -> Result<Self> {
        Ok(Self {
            database,
            api: client::build_client()?,
            hub: client::build_streaming_client()?,
        })
    }
}

pub struct HubStream(reqwest::Response);

impl ChunkStream for HubStream {
    async fn next_chunk(&mut self) -> Result<Option<Vec<u8>>> {
        self.0
            .chunk()
            .await
            .map(|chunk| chunk.map(|bytes| bytes.to_vec()))
            .map_err(|_| Error::account("server_unavailable"))
    }
}

impl EventsApi for HttpEvents {
    type Stream = HubStream;

    async fn server_has_events(&self) -> Result<bool> {
        let base = account::server(&*self.database.lock()?)?;
        Ok(client::server_info(&self.api, &base).await?.events)
    }

    async fn ticket(&self) -> Result<Ticket> {
        let (base, device_id, token) = account::authed_context(&self.database)?;
        let wire: TicketWire = send_json(
            &self.api,
            Method::GET,
            &format!("{base}/v1/cloud/events"),
            Some(Auth {
                device_id: &device_id,
                token: &token,
            }),
            None,
        )
        .await?;
        Ok(Ticket {
            token: wire.token,
            topics: wire.topics,
            lifetime: time_left(&wire.expires_at, SystemTime::now()).unwrap_or(FALLBACK_LIFETIME),
        })
    }

    async fn open(&self, ticket: &Ticket) -> Result<HubStream> {
        // The hub is always where the app builds it, never where a response says (D84).
        let base = account::server(&*self.database.lock()?)?;
        let topics: Vec<(&str, &str)> = ticket.topics.iter().map(|t| ("topic", &**t)).collect();
        let response = self
            .hub
            .get(format!("{base}/.well-known/mercure"))
            .query(&topics)
            .bearer_auth(&ticket.token)
            .header(reqwest::header::ACCEPT, "text/event-stream")
            .send()
            .await
            .map_err(|_| Error::account("server_unavailable"))?;
        if !response.status().is_success() {
            return Err(Error::account("server_unavailable"));
        }
        Ok(HubStream(response))
    }
}

// --- The loop ----------------------------------------------------------------

/// What the loop tells the outside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// Something may have changed in the cloud: sync.
    Changed,
    /// Somebody accepted an invitation to this cloud library (the id is the cloud's). It comes along with a
    /// `Changed`; it only says what happened, so the app can tell whoever invited (D43).
    InvitationAccepted(String),
    /// The connection came up or went down (it will retry).
    Live(bool),
    /// The loop ended for good (no hub, signed out): nothing more will come.
    Stopped,
}

#[derive(Debug, Clone)]
pub struct Timing {
    /// No bytes (heartbeats count) for this long means the connection is dead.
    pub idle: Duration,
    /// The token is renewed this long before it expires...
    pub renew_margin: Duration,
    /// ...but never sooner than this after connecting.
    pub min_renew: Duration,
    pub backoff_first: Duration,
    pub backoff_max: Duration,
    /// At most one `Changed` per this long.
    pub coalesce: Duration,
}

impl Timing {
    pub const fn production() -> Self {
        Self {
            idle: Duration::from_secs(90),
            renew_margin: Duration::from_secs(60),
            min_renew: Duration::from_secs(10),
            backoff_first: Duration::from_secs(1),
            backoff_max: Duration::from_secs(60),
            coalesce: Duration::from_secs(1),
        }
    }
}

enum End {
    /// Do not try again (until the next sign-in or app start).
    Stop,
    /// The token is about to expire: connect again at once with a new one.
    Renew,
    /// Something failed or the stream ended: wait, then try again.
    Retry,
}

struct Cycle {
    end: End,
    /// The hub accepted the subscription, so the backoff starts over.
    connected: bool,
}

/// Emits `Changed` on the first event and then at most once per `gap`, with the rest
/// of a burst folded into one trailing emit.
struct Coalescer {
    gap: Duration,
    last: Instant,
    pending: bool,
}

impl Coalescer {
    fn emit_now(&mut self, emit: &impl Fn(Signal)) {
        self.last = Instant::now();
        self.pending = false;
        emit(Signal::Changed);
    }

    fn mark(&mut self, emit: &impl Fn(Signal)) {
        self.pending = true;
        self.flush(emit);
    }

    fn flush(&mut self, emit: &impl Fn(Signal)) {
        if self.pending && Instant::now() >= self.last + self.gap {
            self.emit_now(emit);
        }
    }

    fn due_at(&self) -> Option<Instant> {
        self.pending.then(|| self.last + self.gap)
    }
}

/// Keeps one connection to the hub until something definitive ends it. Runs as a
/// background task; `wake` skips a backoff wait (the app came back to the foreground).
pub async fn run<A: EventsApi>(api: &A, timing: &Timing, wake: &Notify, emit: &impl Fn(Signal)) {
    let mut backoff = timing.backoff_first;
    let mut live = false;
    loop {
        let cycle = connect_once(api, timing, emit, &mut live).await;
        if cycle.connected {
            backoff = timing.backoff_first;
        }
        match cycle.end {
            End::Stop => {
                emit(Signal::Stopped);
                return;
            }
            End::Renew => continue,
            End::Retry => {
                if live {
                    live = false;
                    emit(Signal::Live(false));
                }
                tokio::select! {
                    () = sleep(backoff) => {}
                    () = wake.notified() => {}
                }
                backoff = (backoff * 2).min(timing.backoff_max);
            }
        }
    }
}

/// What an error means for the loop. Account errors go to the app first, which resets
/// the device or clears the session as every other cloud call does.
async fn judge<A: EventsApi>(api: &A, error: Error) -> End {
    if let Error::Account { code, .. } = &error {
        api.account_error(&error).await;
        if matches!(
            code.as_str(),
            "events_unavailable"
                | "device_removed"
                | "signed_out_password_changed"
                | "invalid_token"
                | "not_signed_in"
        ) {
            return End::Stop;
        }
    }
    End::Retry
}

fn failed(end: End) -> Cycle {
    Cycle {
        end,
        connected: false,
    }
}

async fn connect_once<A: EventsApi>(
    api: &A,
    timing: &Timing,
    emit: &impl Fn(Signal),
    live: &mut bool,
) -> Cycle {
    match api.server_has_events().await {
        Ok(true) => {}
        Ok(false) => return failed(End::Stop),
        Err(error) => return failed(judge(api, error).await),
    }
    let ticket = match api.ticket().await {
        Ok(ticket) => ticket,
        Err(error) => return failed(judge(api, error).await),
    };
    // A hub that accepts the connection and never answers must not hang the loop.
    let mut stream = match timeout(timing.idle, api.open(&ticket)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(error)) => return failed(judge(api, error).await),
        Err(_) => return failed(End::Retry),
    };
    if !*live {
        *live = true;
        emit(Signal::Live(true));
    }
    // Whatever happened while this device was not listening.
    let mut coalescer = Coalescer {
        gap: timing.coalesce,
        last: Instant::now(),
        pending: false,
    };
    coalescer.emit_now(emit);

    let renew_at = Instant::now()
        + ticket
            .lifetime
            .saturating_sub(timing.renew_margin)
            .max(timing.min_renew);
    let mut idle_at = Instant::now() + timing.idle;
    let mut parser = SseParser::default();
    let end = loop {
        let flush_at = coalescer.due_at().unwrap_or(renew_at);
        // Dropping a pending `next_chunk` when a timer wins loses nothing: it only polls
        // the body, whose state lives in the stream.
        tokio::select! {
            chunk = stream.next_chunk() => match chunk {
                Ok(Some(bytes)) => {
                    idle_at = Instant::now() + timing.idle;
                    let events = parser.feed(&bytes);
                    for event in &events {
                        if let Some(cloud) = invitations::accepted_cloud_library(&event.data) {
                            emit(Signal::InvitationAccepted(cloud));
                        }
                    }
                    if !events.is_empty() {
                        coalescer.mark(emit);
                    }
                }
                Ok(None) | Err(_) => break End::Retry,
            },
            () = sleep_until(flush_at), if coalescer.pending => coalescer.flush(emit),
            () = sleep_until(idle_at) => break End::Retry,
            () = sleep_until(renew_at) => break End::Renew,
        }
    };
    Cycle {
        end,
        connected: true,
    }
}

// --- The service the app manages ---------------------------------------------

/// What the UI shows: connected to the hub or not, and whether the server has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Connection {
    pub live: bool,
    /// `false` once the loop ended for good: no hub, or signed out.
    pub available: bool,
}

struct Inner {
    task: Mutex<Option<JoinHandle<()>>>,
    status: Mutex<Connection>,
    wake: Notify,
}

/// Tauri state: owns the background task and what the UI last heard.
#[derive(Clone)]
pub struct EventsService {
    inner: Arc<Inner>,
    app: AppHandle,
    database: Database,
    reset: ResetDeps,
}

impl EventsService {
    pub fn new(app: AppHandle, database: Database, reset: ResetDeps) -> Self {
        Self {
            inner: Arc::new(Inner {
                task: Mutex::new(None),
                status: Mutex::new(Connection {
                    live: false,
                    available: false,
                }),
                wake: Notify::new(),
            }),
            app,
            database,
            reset,
        }
    }

    /// At startup: only a device signed into an account has anything to listen to.
    pub fn start_if_signed_in(&self) {
        let signed_in = matches!(
            self.database.lock(),
            Ok(conn) if account::is_signed_in(&conn).unwrap_or(false)
        );
        if signed_in {
            self.start();
        }
    }

    /// Starts listening, or starts over (another sign-in, another server).
    pub fn start(&self) {
        self.abort_task();
        let Ok(api) = HttpEvents::new(self.database.clone()) else {
            return;
        };
        let api = AppEvents {
            http: api,
            app: self.app.clone(),
            database: self.database.clone(),
            reset: self.reset.clone(),
        };
        let starting = Connection {
            live: false,
            available: true,
        };
        self.set_status(starting);
        // The UI learns there is something to wait for, even if the hub never answers.
        let _ = self.app.emit("cloud-connection", starting);
        let inner = self.inner.clone();
        let emit = self.emitter();
        let task = tauri::async_runtime::spawn(async move {
            run(&api, &Timing::production(), &inner.wake, &emit).await;
        });
        if let Ok(mut slot) = self.inner.task.lock() {
            *slot = Some(task);
        }
    }

    /// Signing out, a device reset: nothing to listen to any more.
    pub fn stop(&self) {
        self.abort_task();
        self.emitter()(Signal::Stopped);
    }

    /// The app is in the foreground again: a socket Android killed may be waiting to retry.
    pub fn wake(&self) {
        self.inner.wake.notify_one();
    }

    pub fn connection(&self) -> Connection {
        self.inner
            .status
            .lock()
            .map(|status| *status)
            .unwrap_or(Connection {
                live: false,
                available: false,
            })
    }

    fn abort_task(&self) {
        if let Ok(mut slot) = self.inner.task.lock()
            && let Some(task) = slot.take()
        {
            task.abort();
        }
    }

    fn set_status(&self, status: Connection) {
        if let Ok(mut current) = self.inner.status.lock() {
            *current = status;
        }
    }

    /// Turns the loop's signals into what the UI hears: `cloud-changed`, `cloud-connection` and
    /// `invitation-accepted`.
    fn emitter(&self) -> impl Fn(Signal) + Send + 'static {
        let (inner, app, database) = (self.inner.clone(), self.app.clone(), self.database.clone());
        move |signal| {
            let snapshot = |change: &dyn Fn(&mut Connection)| {
                inner.status.lock().ok().map(|mut status| {
                    change(&mut status);
                    *status
                })
            };
            match signal {
                Signal::Changed => {
                    let _ = app.emit("cloud-changed", ());
                }
                Signal::InvitationAccepted(cloud) => {
                    // Only a library this device has in this server's cloud is named; the server's word alone
                    // changes nothing here.
                    let notice = database.lock().ok().and_then(|conn| {
                        let server = account::server(&conn).ok()?;
                        invitations::accepted_notice(&conn, &server, &cloud)
                            .ok()
                            .flatten()
                    });
                    if let Some(notice) = notice {
                        let _ = app.emit("invitation-accepted", notice);
                    }
                }
                Signal::Live(live) => {
                    if let Some(status) = snapshot(&|status| status.live = live) {
                        let _ = app.emit("cloud-connection", status);
                    }
                }
                Signal::Stopped => {
                    let stopped = |status: &mut Connection| {
                        status.live = false;
                        status.available = false;
                    };
                    if let Some(status) = snapshot(&stopped) {
                        let _ = app.emit("cloud-connection", status);
                    }
                }
            }
        }
    }
}

/// The real client plus what an account error needs: the same reaction as every other
/// cloud command (`react_to_account_error`, then the `device-reset` event).
struct AppEvents {
    http: HttpEvents,
    app: AppHandle,
    database: Database,
    reset: ResetDeps,
}

impl EventsApi for AppEvents {
    type Stream = HubStream;

    async fn server_has_events(&self) -> Result<bool> {
        self.http.server_has_events().await
    }

    async fn ticket(&self) -> Result<Ticket> {
        self.http.ticket().await
    }

    async fn open(&self, ticket: &Ticket) -> Result<HubStream> {
        self.http.open(ticket).await
    }

    async fn account_error(&self, error: &Error) {
        if let Error::Account { code, retry_after } = error {
            let result: Result<()> = Err(Error::Account {
                code: code.clone(),
                retry_after: *retry_after,
            });
            let result =
                account::react_to_account_error(&self.database, self.reset.context(), result).await;
            commands::emit_if_reset(&self.app, &result);
        }
    }
}

#[cfg(test)]
mod tests;
