//! Invitations, from the app's side (D43, milestone 7.6.2). The server decides who may invite and who may come
//! in (docs/architecture/cloud.md, «Invitaciones»); the app only avoids offering what will be refused, creates
//! the person a new invitation is for, and carries a link from where it was opened to the account that accepts it.
//!
//! A `link` invitation's address exists once: in the answer that creates it. It is never written to the
//! database nor to a message (`Secret` hides it from `Debug`), and the token of a link somebody opened waits
//! in the device's local state, never in an operation, until the account that accepts it signs in.

use std::time::SystemTime;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

use kosmotheka_domain::{Change, Entity};

use super::{CloudApi, LibraryReport, Outcome, events, link_of, sync_all, sync_library};
use crate::account::{self, OFFICIAL_SERVER};
use crate::db::Database;
use crate::error::{Error, Result};
use crate::ops::{self, read_state, write_state};
use crate::{libraries, people};

/// Where the official server's invitation links live (D79). A self-hosted server names its own.
pub const OFFICIAL_INVITATIONS_HOST: &str = "kosmotheka.app";

/// Where a link somebody opened waits for the account that accepts it.
const PENDING: &str = "pending_invitation";

/// A token is 32 random bytes in base64url, without padding: 43 characters (the server's `InvitationToken`).
const TOKEN_LENGTH: usize = 43;

// --- Types -------------------------------------------------------------------

/// Text that must not reach a log or a message: an invitation link or its token. `Debug` hides it; the
/// serialized form (what the UI gets, what the pending store keeps) carries it whole.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Secret(..)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationKind {
    /// Tied to an email address: whoever has an account with it comes in, and keeps their role.
    Email,
    /// A link to copy: only for somebody new, who comes in as a reader.
    Link,
}

/// What `POST …/invitations` answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedInvitation {
    pub id: String,
    pub kind: InvitationKind,
    pub member_id: String,
    pub expires_at: String,
    /// Only for `link`, and only here: the server does not keep it and neither does the app.
    #[serde(default)]
    pub link: Option<Secret>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InvitationState {
    Pending,
    Accepted,
    Revoked,
    Expired,
    /// A state this app does not know yet: shown as nothing in particular.
    #[serde(other)]
    Unknown,
}

/// One row of the list of a library's invitations. Never has the token nor the email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationItem {
    pub id: String,
    pub kind: InvitationKind,
    /// The person it is for: the screen of People crosses it with its list.
    pub member_id: String,
    pub state: InvitationState,
    pub created_at: String,
    pub expires_at: String,
    #[serde(default)]
    pub accepted_at: Option<String>,
}

/// What whoever holds a link may know without a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationPreview {
    pub library_name: String,
    pub kind: InvitationKind,
    pub expires_at: String,
}

/// A link that was opened: what it says, and whether the account that accepts it is already here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedInvitation {
    pub library_name: String,
    pub kind: InvitationKind,
    pub expires_at: String,
    pub signed_in: bool,
}

/// Why a library cannot invite now. The first that applies, in the order of the variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Blocker {
    NotSignedIn,
    /// Not in the cloud of this account's server.
    NotInCloud,
    NoRoles,
    /// This device's person does not administer the library.
    NotAdmin,
    /// The library does not name the cloud as a device that may link new ones (D76).
    NoCloudPower,
    ServerWithoutInvitations,
}

impl Blocker {
    fn error(self) -> Error {
        match self {
            Blocker::NotSignedIn => Error::Cloud("not_signed_in"),
            Blocker::NotInCloud => Error::Cloud("not_in_cloud"),
            Blocker::NoRoles => Error::Cloud("no_roles"),
            Blocker::NotAdmin => Error::NotAllowed,
            Blocker::NoCloudPower => Error::Cloud("no_cloud_power"),
            Blocker::ServerWithoutInvitations => Error::Cloud("server_without_invitations"),
        }
    }
}

/// «Can this library invite?», for the screen: yes, or the reason why not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteStatus {
    pub can_invite: bool,
    pub blocker: Option<Blocker>,
}

/// Who an invitation is for. A new person always starts as a reader: there is no way to ask for another role,
/// and a request that names one is refused (`deny_unknown_fields`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvitePerson {
    New { name: String },
    Existing { member: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteRequest {
    pub person: InvitePerson,
    pub kind: InvitationKind,
    /// Only for `email`. Sent as typed (trimmed) and never kept.
    #[serde(default)]
    pub email: Option<String>,
}

/// What inviting did. Past the creation of a new person nothing is undone: if what comes after fails, the person
/// stays (and `failed` says why) so the screen can tell it and the invitation can be asked again for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteResult {
    pub member: String,
    pub person_created: bool,
    pub invitation: Option<CreatedInvitation>,
    /// The code of what failed after the person was created, when something did.
    pub failed: Option<String>,
}

/// The cloud library an invitation took this account into, and what the session after it brought down.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Accepted {
    pub cloud_library: String,
    /// The library on this device, if the session brought it down (or it was already here).
    pub library: Option<String>,
    pub library_name: Option<String>,
    pub reports: Vec<LibraryReport>,
    /// The code of why the session after accepting failed. The invitation is already used: the next ordinary
    /// session brings the library.
    pub sync_error: Option<String>,
}

/// An invitation to a library this device knows, accepted by someone: what the app says (the live notice).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedNotice {
    pub library: String,
    pub library_name: String,
}

// --- The link ----------------------------------------------------------------

/// The token of `<base>/i/<token>`, if it has the form of one **and** the link belongs to `server`, the one this
/// device talks to. The app never calls a server because a link names it (D84): the link is only read, and only
/// the account's own server (or the default one, with no account) is asked. What the link names has to match it,
/// so a token made for another server is not handed to this one, which could use it there.
///
/// - the official server (D79): links of `kosmotheka.app`, and no other, whatever it publishes;
/// - a server of one's own that publishes where its links start (`base`, from `GET /v1/server`): the links that
///   start there, and no other. A base that is not a plain address, or that is the official one, is not believed;
/// - a server of one's own that says nothing (an older one): links of its own host or of the domain above it
///   (`api.example.org` and `example.org`). Anything else is refused: a link of `kosmotheka.app` never reaches a
///   server of one's own.
pub fn token_of_link(link: &str, server: &str, base: Option<&str>) -> Result<Secret> {
    let bad = || Error::Cloud("invitation_bad_link");
    let url = Url::parse(link.trim()).map_err(|_| bad())?;
    let server_url = Url::parse(server).map_err(|_| Error::Cloud("invitation_other_server"))?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(bad());
    }
    match url.scheme() {
        "https" => {}
        // Plain HTTP only where the server itself is plain HTTP (a local one, which the account already allows).
        "http" if server_url.scheme() == "http" => {}
        _ => return Err(bad()),
    }
    let segments: Vec<&str> = url.path_segments().ok_or_else(bad)?.collect();
    let Some((&token, front)) = segments.split_last() else {
        return Err(bad());
    };
    let Some((&"i", front)) = front.split_last() else {
        return Err(bad());
    };
    if !is_token_shaped(token) {
        return Err(bad());
    }
    let official = server == OFFICIAL_SERVER;
    let published = if official {
        None
    } else {
        base.and_then(plain_base)
    };
    let ours = match &published {
        Some(base) => starts_at(&url, front, base),
        None if !front.is_empty() => return Err(bad()),
        None => belongs_to(&url, &server_url, official),
    };
    if !ours {
        return Err(Error::Cloud("invitation_other_server"));
    }
    Ok(Secret::new(token))
}

/// The base a server published, if it is a plain address that is not the official one: no credentials, query or
/// fragment, and a host. Its scheme is not judged here: a link has to start with it, and a link is only `https`
/// (or `http` where the server is).
fn plain_base(base: &str) -> Option<Url> {
    let url = Url::parse(base.trim()).ok()?;
    let plain = url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none();
    let official = url.host_str() == Some(OFFICIAL_INVITATIONS_HOST);
    (plain && !official && url.host().is_some()).then_some(url)
}

/// Whether the link starts where the base does: same scheme, host and port, and the path of the base before
/// `/i/<token>`.
fn starts_at(link: &Url, front: &[&str], base: &Url) -> bool {
    let wanted: Vec<&str> = base
        .path_segments()
        .map(|segments| segments.filter(|segment| !segment.is_empty()).collect())
        .unwrap_or_default();
    link.scheme() == base.scheme()
        && link.host() == base.host()
        && link.port_or_known_default() == base.port_or_known_default()
        && front == wanted.as_slice()
}

fn is_token_shaped(token: &str) -> bool {
    token.len() == TOKEN_LENGTH
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn belongs_to(link: &Url, server: &Url, official: bool) -> bool {
    if link.port_or_known_default() != server.port_or_known_default() {
        return false;
    }
    fn above(host: &str) -> Option<&str> {
        host.split_once('.')
            .map(|(_, rest)| rest)
            .filter(|rest| rest.contains('.'))
    }
    match (link.host(), server.host()) {
        _ if official => link.host_str() == Some(OFFICIAL_INVITATIONS_HOST),
        (Some(url::Host::Domain(link_host)), Some(url::Host::Domain(server_host))) => {
            link_host != OFFICIAL_INVITATIONS_HOST
                && (link_host == server_host || above(server_host) == Some(link_host))
        }
        // An address (a local server): only the very same one.
        (Some(link_host), Some(server_host)) => link_host == server_host,
        _ => false,
    }
}

// --- What this device may offer ----------------------------------------------

enum Readiness {
    Ready { cloud: String },
    Blocked(Blocker),
}

/// The checks this device can make alone, in the order a person would hit them. The server makes them all again
/// and has the last word; these only keep the app from offering what it will refuse (and, for a device that does
/// not administer, from writing a person or asking at all).
fn readiness(
    conn: &Connection,
    library: &str,
    server: &str,
    needs_cloud_power: bool,
) -> Result<Readiness> {
    if !libraries::library_exists(conn, library)? {
        return Err(Error::MissingRow {
            entity: "library",
            id: library.to_owned(),
        });
    }
    if !account::is_signed_in(conn)? {
        return Ok(Readiness::Blocked(Blocker::NotSignedIn));
    }
    let cloud = link_of(conn, library)?.and_then(|link| {
        let cloud = link.cloud_library.clone()?;
        link.is(&cloud, server).then_some(cloud)
    });
    let Some(cloud) = cloud else {
        return Ok(Readiness::Blocked(Blocker::NotInCloud));
    };
    let listed = people::list_people(conn, library)?;
    if !listed.has_roles {
        return Ok(Readiness::Blocked(Blocker::NoRoles));
    }
    if listed.my_role.as_deref() != Some("admin") {
        return Ok(Readiness::Blocked(Blocker::NotAdmin));
    }
    if needs_cloud_power && people::cloud_device_of(conn, library)?.is_none() {
        return Ok(Readiness::Blocked(Blocker::NoCloudPower));
    }
    Ok(Readiness::Ready { cloud })
}

fn ready(
    database: &Database,
    library: &str,
    server: &str,
    needs_cloud_power: bool,
) -> Result<String> {
    match readiness(&*database.lock()?, library, server, needs_cloud_power)? {
        Readiness::Ready { cloud } => Ok(cloud),
        Readiness::Blocked(blocker) => Err(blocker.error()),
    }
}

/// Whether the screen of People can offer to invite, and if not, why. Asks the server only what it offers.
pub async fn invite_status<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
) -> Result<InviteStatus> {
    // The guard is dropped before the network is asked.
    let local = readiness(&*database.lock()?, library, api.server(), true)?;
    let blocker = match local {
        Readiness::Blocked(blocker) => Some(blocker),
        Readiness::Ready { .. } if !api.offers_invitations().await? => {
            Some(Blocker::ServerWithoutInvitations)
        }
        Readiness::Ready { .. } => None,
    };
    Ok(InviteStatus {
        can_invite: blocker.is_none(),
        blocker,
    })
}

/// Just the shape: something before an `@`, a domain with a dot after it, no spaces. Whether it is a real mailbox
/// is for the mail that goes there; the server checks its own rules again.
pub fn email_shape_is_valid(email: &str) -> bool {
    let email = email.trim();
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    email.len() <= 180
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// The code of an error, for what a result says after the fact.
fn code_of(error: &Error) -> String {
    match error {
        Error::Cloud(code) => (*code).to_owned(),
        Error::Account { code, .. } => code.clone(),
        Error::AccessWithdrawn(_) => "access_withdrawn".to_owned(),
        Error::NotAllowed => "not_allowed".to_owned(),
        _ => "unknown".to_owned(),
    }
}

// --- Inviting ----------------------------------------------------------------

/// Invites somebody to `library`, which must be the one the cloud knows. For somebody new, the person is created
/// first with a signed operation as a **reader** and uploaded, and only then is the invitation asked of the server
/// with that person's id. Everything that can be refused is checked before anything is written: a device that
/// does not administer writes nothing and calls nothing.
pub async fn invite<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    request: InviteRequest,
) -> Result<InviteResult> {
    let cloud = ready(database, library, api.server(), true)?;
    let email = match (request.kind, request.email.as_deref().map(str::trim)) {
        (InvitationKind::Email, Some(email)) if email_shape_is_valid(email) => Some(email),
        (InvitationKind::Link, None) => None,
        _ => return Err(Error::Cloud("invalid_email")),
    };
    let existing = match &request.person {
        InvitePerson::Existing { member } => {
            if request.kind == InvitationKind::Link {
                return Err(Error::Cloud("link_needs_new_person"));
            }
            let conn = database.lock()?;
            let person = people::list_people(&conn, library)?
                .people
                .into_iter()
                .find(|person| &person.id == member)
                .ok_or_else(|| Error::MissingRow {
                    entity: "member",
                    id: member.clone(),
                })?;
            if person.invitable != people::Invitable::Email {
                return Err(Error::Cloud("person_not_invitable"));
            }
            Some(person.id)
        }
        InvitePerson::New { name } if name.trim().is_empty() => {
            return Err(Error::Cloud("invalid_name"));
        }
        InvitePerson::New { .. } => None,
    };
    if !api.offers_invitations().await? {
        return Err(Blocker::ServerWithoutInvitations.error());
    }
    let (member, created) = match (existing, &request.person) {
        (Some(member), _) => (member, false),
        (None, InvitePerson::New { name }) => {
            (create_reader(database, library, name.trim())?, true)
        }
        (None, InvitePerson::Existing { .. }) => unreachable!("an existing person was found above"),
    };

    // From here the person (if new) exists: what fails is told, not undone.
    let mut result = InviteResult {
        member: member.clone(),
        person_created: created,
        invitation: None,
        failed: None,
    };
    if created {
        // The server compares the id with its own catalog, so the person has to be there first.
        match sync_library(database, api, library).await {
            Ok(report) if report.outcome == Outcome::Synced => {}
            Ok(report) => {
                result.failed = Some(report.error.unwrap_or_else(|| "not_uploaded".to_owned()));
                return Ok(result);
            }
            Err(error) => {
                result.failed = Some(code_of(&error));
                return Ok(result);
            }
        }
    }
    match api
        .create_invitation(&cloud, &member, request.kind, email)
        .await
    {
        Ok(invitation) => result.invitation = Some(invitation),
        Err(error) => result.failed = Some(code_of(&error)),
    }
    Ok(result)
}

/// The signed operation that creates a person, always a reader who cannot share. Checked against the roles first,
/// as any change is.
fn create_reader(database: &Database, library: &str, name: &str) -> Result<String> {
    let member = ops::new_id();
    let change = Change::create(
        Entity::Member,
        &member,
        json!({ "name": name, "role": "reader", "can_share": false }),
    );
    let mut conn = database.lock()?;
    if !ops::would_allow(&conn, library, vec![change.clone()])? {
        return Err(Error::NotAllowed);
    }
    let tx = conn.transaction()?;
    ops::record(&tx, library, "member.invite", vec![change])?;
    tx.commit()?;
    Ok(member)
}

pub async fn list_invitations<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
) -> Result<Vec<InvitationItem>> {
    let cloud = ready(database, library, api.server(), false)?;
    api.list_invitations(&cloud).await
}

pub async fn revoke_invitation<A: CloudApi>(
    database: &Database,
    api: &A,
    library: &str,
    invitation: &str,
) -> Result<()> {
    let cloud = ready(database, library, api.server(), false)?;
    api.revoke_invitation(&cloud, invitation).await
}

// --- Opening a link ----------------------------------------------------------

/// What waits for the account: the token (never in an operation), where it was opened and by whom.
#[derive(Serialize, Deserialize)]
struct Pending {
    token: Secret,
    server: String,
    library_name: String,
    kind: InvitationKind,
    expires_at: String,
    /// The account that was signed in when the link was opened, if any. A link opened with no account may be
    /// accepted by whoever signs in next; one opened by an account is only for that account.
    account: Option<String>,
}

pub fn forget_pending(conn: &Connection) -> Result<()> {
    write_state(conn, PENDING, None)
}

/// The invitation waiting for a session, unless it has expired: that one is forgotten.
fn pending(conn: &Connection) -> Result<Option<Pending>> {
    let Some(raw) = read_state(conn, PENDING)? else {
        return Ok(None);
    };
    let kept = serde_json::from_str::<Pending>(&raw)
        .ok()
        .filter(|pending| events::time_left(&pending.expires_at, SystemTime::now()).is_some());
    if kept.is_none() {
        forget_pending(conn)?;
    }
    Ok(kept)
}

/// For the screen that follows a sign-in: what is waiting, if anything still is.
pub fn pending_invitation(database: &Database) -> Result<Option<OpenedInvitation>> {
    let conn = database.lock()?;
    let signed_in = account::is_signed_in(&conn)?;
    Ok(pending(&conn)?.map(|pending| OpenedInvitation {
        library_name: pending.library_name,
        kind: pending.kind,
        expires_at: pending.expires_at,
        signed_in,
    }))
}

/// «Not this one»: forgets the invitation that was waiting.
pub fn discard_pending_invitation(database: &Database) -> Result<()> {
    forget_pending(&*database.lock()?)
}

/// Opens a link, with no session: reads its token (refusing a link of another server before asking anything),
/// asks the server what it says, and keeps the token until the account that accepts it signs in. The server asked
/// is always `api.server()`, whatever the link says.
pub async fn open<A: CloudApi>(
    database: &Database,
    api: &A,
    link: &str,
) -> Result<OpenedInvitation> {
    let base = api.invitation_base().await?;
    let token = token_of_link(link, api.server(), base.as_deref())?;
    let preview = api.peek_invitation(token.expose()).await?;
    let conn = database.lock()?;
    let account = account::signed_in_user_id(&conn)?;
    let signed_in = account::is_signed_in(&conn)?;
    let pending = Pending {
        token,
        server: api.server().to_owned(),
        library_name: preview.library_name.clone(),
        kind: preview.kind,
        expires_at: preview.expires_at.clone(),
        account,
    };
    write_state(&conn, PENDING, Some(&serde_json::to_string(&pending)?))?;
    Ok(OpenedInvitation {
        library_name: preview.library_name,
        kind: preview.kind,
        expires_at: preview.expires_at,
        signed_in,
    })
}

/// What leaves the invitation waiting after a failed accept: the account could not reach the server, or has not
/// signed in yet. Anything else the server said is an answer, and the invitation is forgotten with it.
pub(super) fn keeps_pending(error: &Error) -> bool {
    match error {
        Error::Cloud("not_signed_in") | Error::Http(_) => true,
        Error::Account { code, .. } => matches!(
            code.as_str(),
            "server_unavailable" | "too_many_requests" | "not_signed_in"
        ),
        _ => false,
    }
}

/// Accepts the invitation that is waiting, with the signed-in account, and then syncs so the library comes down
/// and this device is linked by the way that already exists (`claim`, D76, D96).
pub async fn accept_pending<A: CloudApi>(database: &Database, api: &A) -> Result<Accepted> {
    let token = {
        let conn = database.lock()?;
        let Some(pending) = pending(&conn)? else {
            return Err(Error::Cloud("invitation_no_pending"));
        };
        if !account::is_signed_in(&conn)? {
            return Err(Error::Cloud("not_signed_in"));
        }
        // Another server than the one it was opened for, or an account that was not the one: it is not followed.
        if pending.server != api.server() {
            forget_pending(&conn)?;
            return Err(Error::Cloud("invitation_other_server"));
        }
        let now = account::signed_in_user_id(&conn)?;
        if pending
            .account
            .as_deref()
            .is_some_and(|opened_by| now.as_deref() != Some(opened_by))
        {
            forget_pending(&conn)?;
            return Err(Error::Cloud("invitation_other_account"));
        }
        pending.token
    };
    let cloud = match api.accept_invitation(token.expose()).await {
        Ok(cloud) => cloud,
        Err(error) => {
            if !keeps_pending(&error) {
                forget_pending(&*database.lock()?)?;
            }
            return Err(error);
        }
    };
    forget_pending(&*database.lock()?)?;

    let mut accepted = Accepted {
        cloud_library: cloud.clone(),
        library: None,
        library_name: None,
        reports: Vec::new(),
        sync_error: None,
    };
    match sync_all(database, api).await {
        Ok(reports) => {
            let here = reports.iter().find(|report| {
                report.cloud_library.as_deref() == Some(cloud.as_str())
                    && matches!(report.outcome, Outcome::Downloaded | Outcome::Synced)
            });
            if let Some(report) = here {
                accepted.library = Some(report.library.clone());
                accepted.library_name = library_name(&*database.lock()?, &report.library)?;
            }
            accepted.reports = reports;
        }
        Err(error) => accepted.sync_error = Some(code_of(&error)),
    }
    Ok(accepted)
}

fn library_name(conn: &Connection, library: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT name FROM libraries WHERE id = ?1",
            [library],
            |row| row.get(0),
        )
        .optional()?)
}

// --- The notice to who invited -----------------------------------------------

/// The cloud library of an `invitation_accepted` live notice, if `data` is one. The notice carries no personal
/// data: only that someone accepted an invitation to that library.
pub fn accepted_cloud_library(data: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct Notice {
        event: String,
        library: String,
    }
    let notice: Notice = serde_json::from_str(data).ok()?;
    (notice.event == "invitation_accepted").then_some(notice.library)
}

/// What the app says about it, if the library is one this device has in `server`'s cloud. The server's word only
/// gives a notice; it never changes anything here. Separate from the app's window so that a notification with the
/// app closed can use it as it is.
pub fn accepted_notice(
    conn: &Connection,
    server: &str,
    cloud_library: &str,
) -> Result<Option<AcceptedNotice>> {
    Ok(conn
        .query_row(
            "SELECT id, name FROM libraries WHERE cloud_library_id = ?1 AND cloud_server = ?2",
            [cloud_library, server],
            |row| {
                Ok(AcceptedNotice {
                    library: row.get(0)?,
                    library_name: row.get(1)?,
                })
            },
        )
        .optional()?)
}
