//! Where every link the system hands to the app comes in (milestone 7.6.4, D79). Anybody can fire a link at the
//! app: a web page, another app, a message. So a link is only read here. It never accepts, uploads, deletes,
//! switches library nor signs anybody in or out: at most it brings up the «I have an invitation» screen, and
//! entering a library always takes the person pressing a button.
//!
//! One door, one list of destinations: `route` says what a link is by its form, `Arrivals` keeps it until the
//! screen can look at it, `attend` hands it to whoever it is for. A new kind of link (coming back from Google,
//! D77) is a new `Destination` and a new arm in `attend`; nothing else changes.

use std::sync::Mutex;

use url::Url;

use crate::account::OFFICIAL_SERVER;
use crate::cloud::CloudApi;
use crate::cloud::invitations::{self, OFFICIAL_INVITATIONS_HOST, OpenedInvitation, Secret};
use crate::db::Database;
use crate::error::Result;
use tauri::{AppHandle, Emitter, Manager};

/// More than any real link has: what is longer is not parsed.
const MAX_LINK: usize = 2048;

/// What a link is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// An invitation link of the official server (`https://kosmotheka.app/i/<token>`).
    Invitation(Secret),
    /// The way back from signing in with Google (`kosmotheka://auth/google?attempt=…&code=…`, D77): it only counts
    /// for the attempt this app started and is waiting for.
    GoogleReturn(GoogleReturn),
}

/// What the server's page handed back to the app after Google.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleReturn {
    pub attempt: String,
    pub outcome: GoogleOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleOutcome {
    /// The one-use code, worth nothing without the secret the app kept. `Secret` hides it from `Debug`.
    Code(Secret),
    /// Why it did not work: a code of the server's (`google_cancelled`, `registration_closed`…).
    Error(String),
}

/// What a link is, by its form alone. Nothing here asks a server, and a link that is not of a known kind is
/// `None`: ignored, with no error anybody sees.
pub fn route(link: &str) -> Option<Destination> {
    if link.len() > MAX_LINK {
        return None;
    }
    let url = Url::parse(link.trim()).ok()?;
    if url.scheme() == "kosmotheka" {
        return google_return(&url);
    }
    let ours = url.scheme() == "https"
        && matches!(url.host(), Some(url::Host::Domain(host))
            if host.trim_end_matches('.') == OFFICIAL_INVITATIONS_HOST);
    let invitation = url.path_segments().and_then(|mut path| path.next()) == Some("i");
    (ours && invitation).then(|| Destination::Invitation(Secret::new(link.trim())))
}

/// `kosmotheka://auth/google?attempt=…&(code=…|error=…)`, and nothing else: no other host, path, parameter, repeat or
/// shape. (`kosmotheka://i/<token>` is not a link the app follows: it names no server.)
fn google_return(url: &Url) -> Option<Destination> {
    if !matches!(url.host(), Some(url::Host::Domain("auth")))
        || url.path() != "/google"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    let (mut attempt, mut code, mut error) = (None, None, None);
    for (key, value) in url.query_pairs() {
        let slot = match key.as_ref() {
            "attempt" => &mut attempt,
            "code" => &mut code,
            "error" => &mut error,
            _ => return None,
        };
        if slot.replace(value.into_owned()).is_some() {
            return None;
        }
    }
    let shaped = |text: &str, longest: usize| {
        !text.is_empty()
            && text.len() <= longest
            && text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    };
    let attempt = attempt.filter(|attempt| shaped(attempt, 100))?;
    let outcome = match (code, error) {
        (Some(code), None) if shaped(&code, 200) => GoogleOutcome::Code(Secret::new(code)),
        (None, Some(error)) if shaped(&error, 60) => GoogleOutcome::Error(error),
        _ => return None,
    };
    Some(Destination::GoogleReturn(GoogleReturn { attempt, outcome }))
}

/// The links that arrived and have not been looked at yet. Holds one of each kind: the newest. A link the person already
/// looked at is not here any more, so a newer one never takes the place of what they are reading.
#[derive(Default)]
pub struct Arrivals {
    invitation: Mutex<Option<Destination>>,
    google: Mutex<Option<GoogleReturn>>,
}

impl Arrivals {
    /// Reads what arrived. `true` if any of it is for the app, and so the screen should be told.
    pub fn arrive<S: AsRef<str>>(&self, links: impl IntoIterator<Item = S>) -> bool {
        let mut any = false;
        for link in links {
            match route(link.as_ref()) {
                Some(destination @ Destination::Invitation(_)) => {
                    *self.invitation.lock().unwrap() = Some(destination);
                    any = true;
                }
                Some(Destination::GoogleReturn(back)) => {
                    *self.google.lock().unwrap() = Some(back);
                    any = true;
                }
                None => {}
            }
        }
        any
    }

    /// The invitation link waiting, if any, handing it over for good.
    pub fn take(&self) -> Option<Destination> {
        self.invitation.lock().unwrap().take()
    }

    /// The way back from Google, if one is waiting.
    pub fn take_google(&self) -> Option<GoogleReturn> {
        self.google.lock().unwrap().take()
    }
}

/// Looks at the link that was waiting: the same road as a link pasted by hand (`invitations::open`). `None` if
/// nothing was waiting.
pub async fn attend<A: CloudApi>(
    arrivals: &Arrivals,
    database: &Database,
    api: &A,
) -> Result<Option<OpenedInvitation>> {
    match arrivals.take() {
        None | Some(Destination::GoogleReturn(_)) => Ok(None),
        Some(Destination::Invitation(link)) => {
            // The form is judged before any server is asked, as for the official one: a link with credentials,
            // a query or a fragment says so on the screen and is not sent anywhere.
            invitations::token_of_link(link.expose(), OFFICIAL_SERVER, None)?;
            invitations::open(database, api, link.expose())
                .await
                .map(Some)
        }
    }
}

/// The one place the plugins call: reads what arrived and, if any of it is for the app, brings the window up and
/// tells the screen to take it. The event carries nothing; the link stays here until the screen asks.
pub fn receive<S: AsRef<str>>(app: &AppHandle, links: impl IntoIterator<Item = S>) {
    if app.state::<Arrivals>().arrive(links) {
        show_window(app);
        let _ = app.emit("link-arrived", ());
    }
}

/// Brings the main window forward (a link opened with the app already running, or a second launch).
pub fn show_window(app: &AppHandle) {
    #[cfg(desktop)]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    #[cfg(not(desktop))]
    let _ = app;
}
