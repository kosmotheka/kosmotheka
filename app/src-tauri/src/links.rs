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
}

/// What a link is, by its form alone. Nothing here asks a server, and a link that is not of a known kind is
/// `None`: ignored, with no error anybody sees.
pub fn route(link: &str) -> Option<Destination> {
    if link.len() > MAX_LINK {
        return None;
    }
    let url = Url::parse(link.trim()).ok()?;
    let ours = url.scheme() == "https"
        && matches!(url.host(), Some(url::Host::Domain(host))
            if host.trim_end_matches('.') == OFFICIAL_INVITATIONS_HOST);
    let invitation = url.path_segments().and_then(|mut path| path.next()) == Some("i");
    (ours && invitation).then(|| Destination::Invitation(Secret::new(link.trim())))
}

/// The links that arrived and have not been looked at yet. Holds one: the newest. A link the person already
/// looked at is not here any more, so a newer one never takes the place of what they are reading.
#[derive(Default)]
pub struct Arrivals {
    waiting: Mutex<Option<Destination>>,
}

impl Arrivals {
    /// Reads what arrived. `true` if any of it is for the app, and so the screen should be told.
    pub fn arrive<S: AsRef<str>>(&self, links: impl IntoIterator<Item = S>) -> bool {
        let mut any = false;
        for link in links {
            if let Some(destination) = route(link.as_ref()) {
                *self.waiting.lock().unwrap() = Some(destination);
                any = true;
            }
        }
        any
    }

    /// The link waiting, if any, handing it over for good.
    pub fn take(&self) -> Option<Destination> {
        self.waiting.lock().unwrap().take()
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
        None => Ok(None),
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
