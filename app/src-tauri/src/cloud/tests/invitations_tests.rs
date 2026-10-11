//! Invitations in the app (D43, 7.6.2) against the in-memory cloud. The hostile cases first: a link of another
//! server, a malformed token, a new person with any role but reader, a device that does not administer, and a
//! token that outlives the account it was meant for. The server's own rules are tested in its repository.

use serde_json::json;

use super::*;
use crate::account::OFFICIAL_SERVER;
use crate::cloud::invitations::{
    Blocker, InvitationKind, InvitationState, InvitePerson, InviteRequest, PendingInvitation,
    accept_pending, accepted_cloud_library, accepted_notice, discard_pending_invitation,
    email_shape_is_valid, invite, invite_status, keeps_pending, list_invitations, open,
    pending_invitation, revoke_invitation, token_of_link,
};
use crate::people::{self, Invitable};

const TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

// --- The world ---------------------------------------------------------------

/// The laptop (Tersoal, administrator, signed in) has uploaded the family library naming the cloud as a device
/// that may link new ones; the phone (Hugo, a reader, signed in as his own account) has it over the LAN.
async fn family() -> (FakeCloud, Database, Database, String) {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    sign_in_as(&laptop, "tersoal");
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    lan_sync(&laptop, &phone, &lib);
    sign_in_as(&phone, "hugo");
    (cloud, laptop, phone, lib)
}

fn new_person(name: &str, kind: InvitationKind, email: Option<&str>) -> InviteRequest {
    InviteRequest {
        person: InvitePerson::New { name: name.into() },
        kind,
        email: email.map(str::to_owned),
    }
}

fn link_for(name: &str) -> InviteRequest {
    new_person(name, InvitationKind::Link, None)
}

fn person_named(device: &Database, library: &str, name: &str) -> people::Person {
    people::list_people(&device.lock().unwrap(), library)
        .unwrap()
        .people
        .into_iter()
        .find(|person| person.name == name)
        .unwrap_or_else(|| panic!("no person called {name}"))
}

/// Role and `can_share` of a member row, if it is there.
fn member_row(device: &Database, library: &str, id: &str) -> Option<(Option<String>, bool)> {
    device
        .lock()
        .unwrap()
        .query_row(
            "SELECT role, coalesce(can_share, 0) FROM members WHERE library_id = ?1 AND id = ?2",
            [library, id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()
        .unwrap()
}

fn roles(device: &Database, library: &str) -> Vec<Option<String>> {
    let mut roles: Vec<_> = people::list_people(&device.lock().unwrap(), library)
        .unwrap()
        .people
        .into_iter()
        .map(|person| person.role)
        .collect();
    roles.sort();
    roles
}

/// Whether `needle` is written anywhere in the database: any column of any table.
fn database_contains(device: &Database, needle: &str) -> bool {
    let conn = device.lock().unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    for table in tables {
        let columns: Vec<String> = conn
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        for column in columns {
            let found: i64 = conn
                .query_row(
                    &format!(
                        "SELECT count(*) FROM \"{table}\" WHERE cast(\"{column}\" as text) LIKE ?1"
                    ),
                    [format!("%{needle}%")],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if found > 0 {
                return true;
            }
        }
    }
    false
}

/// The token inside a link.
fn token_in(link: &invitations::Secret) -> String {
    link.expose().rsplit('/').next().unwrap().to_owned()
}

/// Nora, a new person invited by link, opens it on her own phone, signs in and accepts: the account is hers.
/// Returns her phone and her person's id; the administrator has not heard of it yet.
async fn nora_joins(cloud: &FakeCloud, laptop: &Database, lib: &str) -> (Database, String) {
    let tersoal = cloud.api("tersoal").on(laptop);
    let made = invite(laptop, &tersoal, lib, link_for("Nora"))
        .await
        .unwrap();
    let link = made.invitation.unwrap().link.unwrap();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);
    open(&nora, &api, link.expose()).await.unwrap();
    sign_in_as(&nora, "nora");
    accept_pending(&nora, &api).await.unwrap();
    (nora, made.member)
}

fn waiting(device: &Database) -> bool {
    matches!(
        pending_invitation(device).unwrap(),
        PendingInvitation::Waiting(_)
    )
}

fn code(error: &Error) -> String {
    match error {
        Error::Cloud(code) => (*code).to_owned(),
        Error::Account { code, .. } => code.clone(),
        Error::NotAllowed => "not_allowed".to_owned(),
        other => panic!("unexpected error {other:?}"),
    }
}

// --- Inviting ----------------------------------------------------------------

#[tokio::test]
async fn a_new_person_is_created_as_a_reader_uploaded_and_only_then_invited() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);

    let result = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();

    assert!(result.person_created);
    assert_eq!(result.failed, None);
    assert_eq!(
        member_row(&laptop, &lib, &result.member),
        Some((Some("reader".to_owned()), false)),
        "a reader who cannot share"
    );
    let invitation = result.invitation.unwrap();
    assert_eq!(invitation.member_id, result.member);
    assert!(invitation.link.unwrap().expose().contains("/i/"));
    // The server knew the person before it was asked about them: it compares the id with its catalog (D84).
    let c = cloud.only_library();
    assert!(
        cloud.0.lock().unwrap().libraries[&c].knows_member(&result.member),
        "the person went up first"
    );
    let log = sync::library_log(&laptop.lock().unwrap(), &lib).unwrap();
    assert_eq!(
        log.iter().filter(|op| op.intent == "member.invite").count(),
        1,
        "one signed operation made the person"
    );
}

#[tokio::test]
async fn the_link_of_an_invitation_is_not_written_anywhere_on_the_device() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);

    let result = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();

    let link = result.invitation.clone().unwrap().link.unwrap();
    let token = link.expose().rsplit('/').next().unwrap().to_owned();
    assert!(
        !database_contains(&laptop, &token),
        "no table holds the token"
    );
    assert!(!database_contains(&laptop, link.expose()));
    let printed = format!("{result:?}");
    assert!(
        !printed.contains(&token),
        "a debug print of the result hides it: {printed}"
    );
}

#[tokio::test]
async fn a_person_already_in_the_library_is_invited_by_email_and_their_row_is_not_touched() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    let hugo = person_named(&laptop, &lib, "Hugo");
    let before = log_ids(&laptop, &lib);

    let result = invite(
        &laptop,
        &api,
        &lib,
        InviteRequest {
            person: InvitePerson::Existing {
                member: hugo.id.clone(),
            },
            kind: InvitationKind::Email,
            email: Some(" hugo@example.com ".into()),
        },
    )
    .await
    .unwrap();

    assert!(!result.person_created);
    assert_eq!(result.member, hugo.id);
    assert_eq!(log_ids(&laptop, &lib), before, "nothing written");
    let invitation = result.invitation.unwrap();
    assert_eq!(invitation.kind, InvitationKind::Email);
    assert!(
        invitation.link.is_none(),
        "an email invitation has no link to show"
    );
}

#[tokio::test]
async fn an_email_for_a_new_person_goes_as_typed_and_is_not_kept() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);

    let result = invite(
        &laptop,
        &api,
        &lib,
        new_person("Eva", InvitationKind::Email, Some("eva@example.com")),
    )
    .await
    .unwrap();

    assert!(result.invitation.is_some());
    assert!(!database_contains(&laptop, "eva@example.com"));
}

// --- Hostile: who may invite, and whom ---------------------------------------

/// Hostile: a device whose person does not administer (Hugo's phone, a modified app calling the command) writes
/// no person and does not call the server.
#[tokio::test]
async fn a_device_that_does_not_administer_writes_no_person_and_asks_the_server_nothing() {
    let (cloud, _laptop, phone, lib) = family().await;
    let hugo = cloud.api("hugo").on(&phone);
    let before = log_ids(&phone, &lib);
    let calls = cloud.invitation_calls();

    let new = invite(&phone, &hugo, &lib, link_for("Mallory")).await;
    let list = list_invitations(&phone, &hugo, &lib).await;
    let revoke = revoke_invitation(&phone, &hugo, &lib, "any").await;

    for result in [new.map(|_| ()), list.map(|_| ()), revoke] {
        assert!(matches!(result, Err(Error::NotAllowed)), "{result:?}");
    }
    assert_eq!(log_ids(&phone, &lib), before, "no person written");
    assert_eq!(cloud.invitation_calls(), calls, "the server was not asked");
    assert_eq!(
        invite_status(&phone, &hugo, &lib).await.unwrap().blocker,
        Some(Blocker::NotAdmin)
    );
}

/// Hostile: whatever the request says, a new person starts as a reader. The request has no role to give, and one
/// that names a role is refused whole, before anything is written.
#[tokio::test]
async fn a_new_person_is_never_anything_but_a_reader_even_if_the_command_asks() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    let before = (log_ids(&laptop, &lib), roles(&laptop, &lib));

    for body in [
        json!({ "person": { "type": "new", "name": "Eva", "role": "admin" }, "kind": "link" }),
        json!({ "person": { "type": "new", "name": "Eva" }, "kind": "link", "role": "admin" }),
        json!({ "person": { "type": "new", "name": "Eva", "can_share": true }, "kind": "link" }),
    ] {
        assert!(
            serde_json::from_value::<InviteRequest>(body.clone()).is_err(),
            "{body}"
        );
    }
    assert_eq!((log_ids(&laptop, &lib), roles(&laptop, &lib)), before);

    let honest: InviteRequest = serde_json::from_value(
        json!({ "person": { "type": "new", "name": "Eva" }, "kind": "link" }),
    )
    .unwrap();
    let result = invite(&laptop, &api, &lib, honest).await.unwrap();
    assert_eq!(
        member_row(&laptop, &lib, &result.member),
        Some((Some("reader".to_owned()), false))
    );
    assert_eq!(roles(&laptop, &lib).len(), before.1.len() + 1);
    assert_eq!(
        roles(&laptop, &lib)
            .iter()
            .filter(|role| role.as_deref() == Some("reader"))
            .count(),
        2,
        "Hugo and Eva; no new administrator or editor"
    );
}

/// Hostile: a link is for somebody new. Somebody who is already in the library is invited by email or not at all,
/// and nothing is asked of the server.
#[tokio::test]
async fn a_link_for_a_person_who_is_already_in_the_library_is_refused_before_asking() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    let hugo = person_named(&laptop, &lib, "Hugo");
    let calls = cloud.invitation_calls();

    let refused = invite(
        &laptop,
        &api,
        &lib,
        InviteRequest {
            person: InvitePerson::Existing { member: hugo.id },
            kind: InvitationKind::Link,
            email: None,
        },
    )
    .await;

    assert_eq!(code(&refused.unwrap_err()), "link_needs_new_person");
    assert_eq!(cloud.invitation_calls(), calls);
}

/// Hostile: the administrator, a withdrawn person and one with an account are not offered, and a modified app
/// that asks anyway is stopped here, before the server.
#[tokio::test]
async fn nobody_is_invited_who_administers_has_the_access_withdrawn_or_has_an_account() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    let (_nora_phone, nora) = nora_joins(&cloud, &laptop, &lib).await;
    sync_library(&laptop, &api, &lib).await.unwrap();
    let tersoal = person_named(&laptop, &lib, "Tersoal");
    let hugo = person_named(&laptop, &lib, "Hugo");
    assert_eq!(tersoal.invitable, Invitable::None, "an administrator");
    assert_eq!(hugo.invitable, Invitable::Email);
    assert_eq!(
        person_named(&laptop, &lib, "Nora").invitable,
        Invitable::None,
        "already has an account"
    );
    people::withdraw_person(&mut laptop.lock().unwrap(), &lib, &hugo.id).unwrap();
    assert_eq!(
        person_named(&laptop, &lib, "Hugo").invitable,
        Invitable::None,
        "access withdrawn"
    );
    let calls = cloud.invitation_calls();

    for member in [tersoal.id, hugo.id, nora] {
        let refused = invite(
            &laptop,
            &api,
            &lib,
            InviteRequest {
                person: InvitePerson::Existing { member },
                kind: InvitationKind::Email,
                email: Some("someone@example.com".into()),
            },
        )
        .await;
        assert_eq!(code(&refused.unwrap_err()), "person_not_invitable");
    }
    assert_eq!(cloud.invitation_calls(), calls);
}

#[tokio::test]
async fn a_person_that_is_not_in_this_library_is_not_invited() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);

    let refused = invite(
        &laptop,
        &api,
        &lib,
        InviteRequest {
            person: InvitePerson::Existing {
                member: ops::new_id(),
            },
            kind: InvitationKind::Email,
            email: Some("x@example.com".into()),
        },
    )
    .await;

    assert!(matches!(refused, Err(Error::MissingRow { .. })));
    assert_eq!(cloud.invitation_calls(), 0);
}

// --- What the library can do -------------------------------------------------

async fn blocker(device: &Database, api: &FakeApi, library: &str) -> (bool, Option<Blocker>) {
    let status = invite_status(device, api, library).await.unwrap();
    (status.can_invite, status.blocker)
}

#[tokio::test]
async fn the_status_says_why_a_library_cannot_invite() {
    let cloud = FakeCloud::default();
    let (laptop, _phone, lib) = laptop_admin_phone_reader();
    let api = cloud.api("tersoal").on(&laptop);

    assert_eq!(
        blocker(&laptop, &api, &lib).await,
        (false, Some(Blocker::NotSignedIn))
    );
    sign_in_as(&laptop, "tersoal");
    assert_eq!(
        blocker(&laptop, &api, &lib).await,
        (false, Some(Blocker::NotInCloud))
    );

    // In the cloud, but the upload did not name it as a device that may link new ones.
    upload_with(&laptop, &api, &lib, false).await.unwrap();
    assert_eq!(
        blocker(&laptop, &api, &lib).await,
        (false, Some(Blocker::NoCloudPower))
    );
    set_cloud_power(&laptop, &api, &lib, true).await.unwrap();
    assert_eq!(blocker(&laptop, &api, &lib).await, (true, None));

    // A library with no people has no roles, and so nobody to link an account to.
    let plain = Database::in_memory();
    let own = first_library(&plain);
    sign_in_as(&plain, "ana");
    let plain_api = cloud.api("ana").on(&plain);
    upload(&plain, &plain_api, &own).await.unwrap();
    assert_eq!(
        blocker(&plain, &plain_api, &own).await,
        (false, Some(Blocker::NoRoles))
    );
}

#[tokio::test]
async fn a_server_that_makes_no_invitations_is_known_before_a_person_is_written() {
    let (cloud, laptop, _phone, lib) = family().await;
    let quiet = cloud.clone().without_invitations();
    let api = quiet.api("tersoal").on(&laptop);
    let before = log_ids(&laptop, &lib);

    let refused = invite(&laptop, &api, &lib, link_for("Nora")).await;

    assert_eq!(code(&refused.unwrap_err()), "server_without_invitations");
    assert_eq!(log_ids(&laptop, &lib), before, "no person was created");
    assert_eq!(
        invite_status(&laptop, &api, &lib).await.unwrap().blocker,
        Some(Blocker::ServerWithoutInvitations)
    );
}

#[tokio::test]
async fn nothing_is_written_when_the_email_is_not_one_or_does_not_fit_the_kind() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    let before = log_ids(&laptop, &lib);

    for (kind, email) in [
        (InvitationKind::Email, None),
        (InvitationKind::Email, Some("no-at-sign")),
        (InvitationKind::Email, Some("a@b")),
        (InvitationKind::Email, Some("two words@example.com")),
        (InvitationKind::Email, Some("a@@example.com")),
        (InvitationKind::Link, Some("eva@example.com")),
    ] {
        let refused = invite(&laptop, &api, &lib, new_person("Eva", kind, email)).await;
        assert_eq!(
            code(&refused.unwrap_err()),
            "invalid_email",
            "{kind:?} {email:?}"
        );
    }
    let nameless = invite(&laptop, &api, &lib, link_for("   ")).await;
    assert_eq!(code(&nameless.unwrap_err()), "invalid_name");
    assert_eq!(log_ids(&laptop, &lib), before);
    assert_eq!(cloud.invitation_calls(), 0);
}

#[test]
fn the_shape_of_an_email_is_checked_and_no_more() {
    for good in ["a@b.co", "  eva@example.com ", "e.v+a@sub.example.org"] {
        assert!(email_shape_is_valid(good), "{good}");
    }
    for bad in [
        "", "@", "a@", "@b.co", "a@b", "a b@c.de", "a@b..", "a@.b.co", "a@b@c.de", "a\n@b.co",
    ] {
        assert!(!email_shape_is_valid(bad), "{bad:?}");
    }
    assert!(!email_shape_is_valid(&format!(
        "{}@example.com",
        "a".repeat(200)
    )));
}

// --- After the person exists --------------------------------------------------

#[tokio::test]
async fn if_the_server_refuses_the_invitation_the_new_person_stays_and_the_result_says_so() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    cloud.refuse_invitations("too_many_requests");

    let result = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();

    assert!(result.person_created);
    assert_eq!(result.failed.as_deref(), Some("too_many_requests"));
    assert!(result.invitation.is_none());
    assert!(
        member_row(&laptop, &lib, &result.member).is_some(),
        "the person is not deleted behind the screen's back"
    );
    // The person is somebody in the library now: the invitation is asked again for them, by email.
    cloud.0.lock().unwrap().refuse_invitations = None;
    let again = invite(
        &laptop,
        &api,
        &lib,
        InviteRequest {
            person: InvitePerson::Existing {
                member: result.member,
            },
            kind: InvitationKind::Email,
            email: Some("nora@example.com".into()),
        },
    )
    .await
    .unwrap();
    assert!(again.invitation.is_some() && !again.person_created);
}

#[tokio::test]
async fn if_the_upload_of_the_new_person_fails_the_server_is_not_asked_for_an_invitation() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    cloud.fail_pulls_of(&cloud.only_library(), "not_found");

    let result = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();

    assert!(result.person_created);
    assert_eq!(result.failed.as_deref(), Some("not_found"));
    assert_eq!(cloud.invitation_calls(), 0);
    assert!(member_row(&laptop, &lib, &result.member).is_some());
}

#[tokio::test]
async fn the_pending_invitations_are_listed_and_revoked_by_who_administers() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    let made = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();

    let listed = list_invitations(&laptop, &api, &lib).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].member_id.as_str(), listed[0].state),
        (made.member.as_str(), InvitationState::Pending)
    );

    revoke_invitation(&laptop, &api, &lib, &listed[0].id)
        .await
        .unwrap();
    assert_eq!(
        list_invitations(&laptop, &api, &lib).await.unwrap()[0].state,
        InvitationState::Revoked
    );
}

#[test]
fn what_the_server_sends_is_read_as_it_is_written() {
    let created: invitations::CreatedInvitation = serde_json::from_str(
        r#"{"id":"i1","kind":"link","memberId":"m1","expiresAt":"2026-10-11T10:00:00+00:00","link":"https://kosmotheka.app/i/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"}"#,
    )
    .unwrap();
    assert_eq!(created.kind, InvitationKind::Link);
    assert!(created.link.is_some());
    let by_email: invitations::CreatedInvitation = serde_json::from_str(
        r#"{"id":"i1","kind":"email","memberId":"m1","expiresAt":"2026-10-11T10:00:00+00:00","link":null}"#,
    )
    .unwrap();
    assert!(by_email.link.is_none());
    let item: invitations::InvitationItem = serde_json::from_str(
        r#"{"id":"i1","kind":"email","memberId":"m1","state":"accepted","createdAt":"2026-10-10T10:00:00+00:00","expiresAt":"2026-10-11T10:00:00+00:00","acceptedAt":"2026-10-10T11:00:00+00:00"}"#,
    )
    .unwrap();
    assert_eq!(item.state, InvitationState::Accepted);
    let odd: invitations::InvitationItem = serde_json::from_str(
        r#"{"id":"i1","kind":"link","memberId":"m1","state":"somethingnew","createdAt":"x","expiresAt":"y","acceptedAt":null}"#,
    )
    .unwrap();
    assert_eq!(
        odd.state,
        InvitationState::Unknown,
        "a state of a newer server is not an error"
    );
    let preview: invitations::InvitationPreview = serde_json::from_str(
        r#"{"libraryName":"Casa","kind":"link","expiresAt":"2026-10-11T10:00:00+00:00"}"#,
    )
    .unwrap();
    assert_eq!(preview.library_name, "Casa");
}

// --- The link ----------------------------------------------------------------

fn official_link(token: &str) -> String {
    format!("https://kosmotheka.app/i/{token}")
}

#[test]
fn the_token_of_a_link_is_read_when_the_link_is_the_servers() {
    let token = token_of_link(&official_link(TOKEN), OFFICIAL_SERVER, None).unwrap();
    assert_eq!(token.expose(), TOKEN);
    // A little slack for what a chat adds around a pasted link.
    assert!(
        token_of_link(
            &format!("  {}\n", official_link(TOKEN)),
            OFFICIAL_SERVER,
            None
        )
        .is_ok()
    );
    // A server of one's own that publishes nothing: its own host, written however a person writes it.
    for link in [
        format!("https://api.example.org/i/{TOKEN}"),
        format!("https://API.Example.org/i/{TOKEN}"),
        format!("https://api.example.org./i/{TOKEN}"),
    ] {
        assert!(
            token_of_link(&link, "https://api.example.org", None).is_ok(),
            "{link}"
        );
    }
    assert!(
        token_of_link(
            &format!("https://api.example.org/i/{TOKEN}"),
            "https://api.example.org.",
            None
        )
        .is_ok()
    );
    assert!(
        token_of_link(
            &format!("http://localhost:8000/i/{TOKEN}"),
            "http://localhost:8000",
            None
        )
        .is_ok()
    );
}

/// SEG-048, hostile: the hosts are compared as hosts (no final dot, no capitals), the official one is never a server
/// of one's own, nor anything under it, and the fallback for an old server is its own host and nothing above it.
#[test]
fn hosts_are_compared_as_hosts_and_a_server_of_ones_own_never_has_the_official_domain_nor_the_one_above_it()
 {
    // The official host written with a final dot or in capitals is still the official one.
    for link in [
        "https://kosmotheka.app./i/",
        "https://KOSMOTHEKA.app/i/",
        "https://Kosmotheka.App./i/",
    ] {
        assert!(
            token_of_link(&format!("{link}{TOKEN}"), OFFICIAL_SERVER, None).is_ok(),
            "{link}"
        );
    }
    // A server of one's own, with or without a published base, does not take the official domain or a subdomain.
    for server in [
        "https://api.example.org",
        "https://eu.kosmotheka.app",
        "https://kosmotheka.app.",
    ] {
        for link in [
            "https://kosmotheka.app./i/",
            "https://KOSMOTHEKA.APP/i/",
            "https://www.kosmotheka.app/i/",
            "https://www.kosmotheka.app./i/",
        ] {
            for base in [
                None,
                Some("https://kosmotheka.app."),
                Some("https://www.kosmotheka.app"),
            ] {
                let refused = token_of_link(&format!("{link}{TOKEN}"), server, base)
                    .expect_err(&format!("{server} {link} {base:?}"));
                assert_eq!(code(&refused), "invitation_other_server");
            }
        }
    }
    // A base that is the official host in disguise is not believed: the old rule still applies, same host only.
    assert!(
        token_of_link(
            &format!("https://api.example.org/i/{TOKEN}"),
            "https://api.example.org",
            Some("https://kosmotheka.app.")
        )
        .is_ok()
    );
    // The fallback is the same host: not the domain above, and not a public suffix such as github.io.
    for (server, link) in [
        ("https://api.example.org", "https://example.org/i/"),
        ("https://alice.github.io", "https://github.io/i/"),
        ("https://alice.github.io", "https://bob.github.io/i/"),
        ("https://alice.github.io", "https://io/i/"),
    ] {
        let refused = token_of_link(&format!("{link}{TOKEN}"), server, None)
            .expect_err(&format!("{server} {link}"));
        assert_eq!(code(&refused), "invitation_other_server");
    }
    // The hosts of a published base are compared the same way.
    assert!(
        token_of_link(
            &format!("https://Invitaciones.example.org./i/{TOKEN}"),
            "https://api.example.org",
            Some("https://invitaciones.example.org")
        )
        .is_ok()
    );
}

/// SEG-049: what goes into the path of a route is the shape of an id, nothing else, and a refusal asks nothing.
#[tokio::test]
async fn only_an_id_shaped_like_one_is_put_in_the_path_of_revoking() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);

    for bad in [
        "",
        "../../v1/account",
        "a/b",
        "01234567-89ab-cdef-0123-456789abcdeg",
        "01234567-89AB-CDEF-0123-456789ABCDEF/../x",
        "01234567-89ab-cdef-0123-456789abcdef?x=1",
        "%2e%2e",
    ] {
        let refused = revoke_invitation(&laptop, &api, &lib, bad)
            .await
            .expect_err(bad);
        assert_eq!(code(&refused), "invitation_not_found", "{bad:?}");
    }
    assert_eq!(cloud.invitation_calls(), 0, "the server was not asked");
    // An id that is one goes through (the server then says whether it knows it).
    let made = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();
    let listed = list_invitations(&laptop, &api, &lib).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(made.member, listed[0].member_id);
    revoke_invitation(&laptop, &api, &lib, &listed[0].id)
        .await
        .unwrap();
}

/// COS-002: asking again for a person who is already in the library syncs first, like a new one, so a person
/// whose upload failed is on the server by the time they are invited; if it cannot, the reason is the sync's.
#[tokio::test]
async fn asking_again_for_a_person_whose_upload_failed_syncs_first() {
    let (cloud, laptop, _phone, lib) = family().await;
    let api = cloud.api("tersoal").on(&laptop);
    cloud.fail_pulls_of(&cloud.only_library(), "server_unavailable");
    let first = invite(&laptop, &api, &lib, link_for("Nora")).await.unwrap();
    assert_eq!(first.failed.as_deref(), Some("server_unavailable"));
    let again = |request_member: String| InviteRequest {
        person: InvitePerson::Existing {
            member: request_member,
        },
        kind: InvitationKind::Email,
        email: Some("nora@example.com".into()),
    };

    // Still no connection: the answer is that, and the server is not asked for an invitation it would refuse.
    let calls = cloud.invitation_calls();
    let offline = invite(&laptop, &api, &lib, again(first.member.clone()))
        .await
        .unwrap();
    assert_eq!(offline.failed.as_deref(), Some("server_unavailable"));
    assert!(offline.invitation.is_none() && !offline.person_created);
    assert_eq!(cloud.invitation_calls(), calls);

    // Connection back: the person goes up and the invitation is made.
    cloud.0.lock().unwrap().failing_pulls.clear();
    let online = invite(&laptop, &api, &lib, again(first.member))
        .await
        .unwrap();
    assert_eq!(online.failed, None);
    assert!(online.invitation.is_some());
}

/// Hostile: a link that names another server is not followed, and not even read for its token: the token of
/// `kosmotheka.app` never reaches a server of one's own, nor the other way round. The server asked is always the
/// account's, and a refusal asks nothing.
#[tokio::test]
async fn a_link_of_another_server_is_refused_before_anything_is_asked() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let official = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);
    let own = cloud.api_on("nora", "https://api.example.org").on(&nora);
    // A server of one's own that sits on the official invitations host still does not get the official token.
    let on_the_web_host = cloud.api_on("nora", "https://kosmotheka.app").on(&nora);

    let cases = [
        (&official, "https://evil.example/i/".to_owned() + TOKEN),
        (
            &official,
            format!("https://kosmotheka.app.evil.example/i/{TOKEN}"),
        ),
        (&official, format!("https://evilkosmotheka.app/i/{TOKEN}")),
        (&official, format!("https://api.kosmotheka.app/i/{TOKEN}")),
        (&official, format!("https://kosmotheka.app:8443/i/{TOKEN}")),
        (&own, official_link(TOKEN)),
        (&on_the_web_host, official_link(TOKEN)),
        (&own, format!("https://example.com/i/{TOKEN}")),
        (&own, format!("https://evil-example.org/i/{TOKEN}")),
        (&own, format!("https://org/i/{TOKEN}")),
    ];
    for (api, link) in cases {
        let refused = open(&nora, api, &link).await;
        let refused = refused.expect_err(&link);
        assert_eq!(code(&refused), "invitation_other_server", "{link}");
    }
    assert!(
        cloud.peeks().is_empty(),
        "no server was asked: {:?}",
        cloud.peeks()
    );
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

/// A server that says where its links start (`invitation_base`) decides which links are its own, and only that:
/// the link is compared, never called.
#[tokio::test]
async fn a_server_that_says_where_its_links_start_has_exactly_those_links() {
    let server = "https://api.example.org";
    let base = Some("https://invitaciones.example.org");
    assert!(
        token_of_link(
            &format!("https://invitaciones.example.org/i/{TOKEN}"),
            server,
            base
        )
        .is_ok()
    );
    // With a base, the old guess (same host, domain above) no longer lets anything in.
    for link in [
        format!("https://api.example.org/i/{TOKEN}"),
        format!("https://example.org/i/{TOKEN}"),
        format!("https://invitaciones.example.org:8443/i/{TOKEN}"),
        format!("https://invitaciones.example.org/x/i/{TOKEN}"),
    ] {
        let refused = token_of_link(&link, server, base).expect_err(&link);
        assert_eq!(code(&refused), "invitation_other_server", "{link}");
    }
    let plain = token_of_link(
        &format!("http://invitaciones.example.org/i/{TOKEN}"),
        server,
        base,
    );
    assert_eq!(code(&plain.unwrap_err()), "invitation_bad_link");
    // The scheme is part of where the links start: a local server's `http` base is not the same as `https`.
    let local = "http://localhost:8000";
    assert!(token_of_link(&format!("{local}/i/{TOKEN}"), local, Some(local)).is_ok());
    let secure = token_of_link(
        &format!("https://localhost:8000/i/{TOKEN}"),
        local,
        Some(local),
    );
    assert_eq!(code(&secure.unwrap_err()), "invitation_other_server");
    // A base with a path.
    let under_path = Some("https://example.org/invitaciones/");
    assert!(
        token_of_link(
            &format!("https://example.org/invitaciones/i/{TOKEN}"),
            server,
            under_path
        )
        .is_ok()
    );
    assert!(
        token_of_link(
            &format!("https://example.org/i/{TOKEN}"),
            server,
            under_path
        )
        .is_err()
    );
}

/// Hostile: a link made for another server is still refused when this one publishes its base, and a server that
/// publishes someone else's address does not get the app to call it, nor the tokens of the official server.
#[tokio::test]
async fn a_published_base_never_makes_the_app_call_it_nor_hand_over_an_official_link() {
    let own = "https://api.example.org";
    // The link of another server.
    let refused = token_of_link(
        &format!("https://other.example/i/{TOKEN}"),
        own,
        Some("https://invitaciones.example.org"),
    );
    assert_eq!(code(&refused.unwrap_err()), "invitation_other_server");

    // A server that names a foreign address as its own base: the link is "its", but the call goes to the account's server.
    let cloud = FakeCloud::default().publishing_base("https://evil.test");
    let nora = Database::in_memory();
    let api = cloud.api_on("nora", own).on(&nora);
    // The fake knows no such invitation, so the peek fails; what matters is where it was asked.
    let _ = open(&nora, &api, &format!("https://evil.test/i/{TOKEN}")).await;
    assert_eq!(cloud.peeks(), vec![(own.to_owned(), TOKEN.to_owned())]);

    // A server that names the official address as its base does not get the official tokens.
    for base in ["https://kosmotheka.app", "https://kosmotheka.app/"] {
        let refused = token_of_link(&official_link(TOKEN), own, Some(base));
        assert_eq!(
            code(&refused.unwrap_err()),
            "invitation_other_server",
            "{base}"
        );
    }
    // The official server does not let a base move its links.
    let refused = token_of_link(
        &format!("https://evil.test/i/{TOKEN}"),
        OFFICIAL_SERVER,
        Some("https://evil.test"),
    );
    assert_eq!(code(&refused.unwrap_err()), "invitation_other_server");
    assert!(
        token_of_link(
            &official_link(TOKEN),
            OFFICIAL_SERVER,
            Some("https://evil.test")
        )
        .is_ok()
    );

    // A base that is not a plain address is not believed: the guess of before applies instead.
    for base in [
        "https://user@evil.test",
        "https://:secret@evil.test",
        "https://evil.test/#frag",
        "https://evil.test/?x=1",
        "http://evil.test",
        "ftp://evil.test",
        "not a url",
        "",
    ] {
        let refused = token_of_link(&format!("https://evil.test/i/{TOKEN}"), own, Some(base));
        assert_eq!(
            code(&refused.unwrap_err()),
            "invitation_other_server",
            "{base}"
        );
    }
    assert!(
        token_of_link(
            &format!("https://api.example.org/i/{TOKEN}"),
            own,
            Some("not a url")
        )
        .is_ok()
    );
}

/// Hostile: a token with the wrong form is refused without asking, and without being written.
#[tokio::test]
async fn a_malformed_token_is_refused_before_anything_is_asked() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);
    let long = "A".repeat(44);
    let short = "A".repeat(42);
    let cases = [
        official_link(&short),
        official_link(&long),
        official_link(""),
        official_link("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA!"),
        official_link("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA/"),
        official_link("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA%2e"),
        format!("https://kosmotheka.app/i/{TOKEN}/accept"),
        format!("https://kosmotheka.app/x/{TOKEN}"),
        format!("https://kosmotheka.app/i/../{TOKEN}"),
        format!("https://kosmotheka.app//i/{TOKEN}"),
        format!("https://kosmotheka.app/i/{TOKEN}?next=https://evil.example"),
        format!("https://kosmotheka.app/i/{TOKEN}#more"),
        format!("http://kosmotheka.app/i/{TOKEN}"),
        format!("ftp://kosmotheka.app/i/{TOKEN}"),
        format!("kosmotheka://i/{TOKEN}"),
        format!("https://kosmotheka.app@evil.example/i/{TOKEN}"),
        format!("https://user:pass@kosmotheka.app/i/{TOKEN}"),
        TOKEN.to_owned(),
        "not a link".to_owned(),
        String::new(),
    ];
    for link in cases {
        let refused = open(&nora, &api, &link).await.expect_err(&link);
        assert_eq!(code(&refused), "invitation_bad_link", "{link}");
    }
    assert!(cloud.peeks().is_empty());
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
    assert!(!database_contains(&nora, TOKEN));
}

// --- Opening and accepting ---------------------------------------------------

/// The whole way: the administrator invites Nora with a link, Nora opens it before she has an account, signs in
/// and accepts, and the library comes down and her phone is hers.
#[tokio::test]
async fn a_link_opened_before_signing_in_waits_and_is_accepted_after_it() {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let made = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap();
    let link = made.invitation.unwrap().link.unwrap();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);

    let opened = open(&nora, &api, link.expose()).await.unwrap();

    assert_eq!(
        (opened.library_name.as_str(), opened.kind, opened.signed_in),
        ("Family", InvitationKind::Link, false)
    );
    assert!(waiting(&nora));
    sign_in_as(&nora, "nora");
    let accepted = accept_pending(&nora, &api).await.unwrap();

    assert_eq!(accepted.library.as_deref(), Some(lib.as_str()));
    assert_eq!(accepted.sync_error, None);
    assert_eq!(accepted.cloud_library, cloud.only_library());
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing,
        "used: not kept"
    );
    let me = this_device_as(&nora.lock().unwrap(), &lib)
        .unwrap()
        .unwrap();
    assert_eq!(
        (me.person.as_str(), me.role.as_deref()),
        ("Nora", Some("reader")),
        "the device is linked by the way that already exists"
    );
    assert!(
        !database_contains(&nora, &token_in(&link)),
        "the token is not left behind"
    );
    // The administrator hears of it with the next session, and Nora is not offered an invitation again.
    sync_library(&laptop, &tersoal, &lib).await.unwrap();
    assert!(person_named(&laptop, &lib, "Nora").has_account);
    assert_eq!(
        person_named(&laptop, &lib, "Nora").invitable,
        Invitable::None
    );
}

#[tokio::test]
async fn a_link_opened_with_a_session_is_accepted_by_that_account() {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let made = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap();
    let link = made.invitation.unwrap().link.unwrap();
    let nora = Database::in_memory();
    sign_in_as(&nora, "nora");
    let api = cloud.api("nora").on(&nora);

    let opened = open(&nora, &api, link.expose()).await.unwrap();
    assert!(opened.signed_in);
    let accepted = accept_pending(&nora, &api).await.unwrap();

    assert_eq!(accepted.library.as_deref(), Some(lib.as_str()));
}

#[tokio::test]
async fn accepting_with_no_session_waits_and_with_nothing_pending_says_so() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);

    let none = accept_pending(&nora, &api).await.unwrap_err();
    assert_eq!(code(&none), "invitation_no_pending");

    open(&nora, &api, &seed_link(&cloud).await).await.unwrap();
    let signed_out = accept_pending(&nora, &api).await.unwrap_err();
    assert_eq!(code(&signed_out), "not_signed_in");
    assert!(waiting(&nora), "still waiting for the session");
}

/// The link of an invitation for the tests that only need one: Tersoal's, for a new person. A second call in the
/// same cloud gives the same link.
async fn seed_link(cloud: &FakeCloud) -> String {
    let existing = cloud
        .0
        .lock()
        .unwrap()
        .invitations
        .first()
        .map(|found| found.token.clone());
    if let Some(token) = existing {
        return format!("https://cloud.example/i/{token}");
    }
    let (laptop, _phone, lib) = laptop_admin_phone_reader();
    sign_in_as(&laptop, "tersoal");
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    let made = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap();
    made.invitation.unwrap().link.unwrap().expose().to_owned()
}

/// Hostile: the token that waits does not outlive the account it was meant for.
#[tokio::test]
async fn the_pending_token_does_not_survive_another_account_signing_in() {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let link = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap()
        .invitation
        .unwrap()
        .link
        .unwrap();
    let phone = Database::in_memory();
    sign_in_as(&phone, "nora");
    let api = cloud.api("nora").on(&phone);
    open(&phone, &api, link.expose()).await.unwrap();
    let asked = cloud.invitation_calls();

    // Somebody else's session replaces hers (as a sign-in on the same device would).
    sign_in_as(&phone, "mallory");
    let mallory = cloud.api("mallory").on(&phone);
    let refused = accept_pending(&phone, &mallory).await.unwrap_err();

    assert_eq!(code(&refused), "invitation_other_account");
    assert_eq!(cloud.invitation_calls(), asked, "the server was not asked");
    assert_eq!(
        pending_invitation(&phone).unwrap(),
        PendingInvitation::Nothing
    );
    assert!(!database_contains(&phone, &token_in(&link)));
}

#[tokio::test]
async fn signing_out_forgets_the_pending_token() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    sign_in_as(&nora, "nora");
    let api = cloud.api("nora").on(&nora);
    open(&nora, &api, &seed_link(&cloud).await).await.unwrap();
    assert!(waiting(&nora));
    assert!(
        database_contains(&nora, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1"),
        "waiting, here and nowhere else"
    );

    crate::account::clear_session(&nora.lock().unwrap()).unwrap();

    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
    assert!(!database_contains(
        &nora,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1"
    ));
}

/// Hostile: «leave this device to another person» (D81) wipes the database, and the token with it.
#[tokio::test]
async fn leaving_the_device_to_another_person_takes_the_pending_token_with_it() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);
    open(&nora, &api, &seed_link(&cloud).await).await.unwrap();
    assert!(waiting(&nora));

    nora.reset().unwrap();

    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
    assert!(!database_contains(
        &nora,
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA1"
    ));
}

#[tokio::test]
async fn a_pending_token_expires_with_its_invitation() {
    let nora = Database::in_memory();
    let stale = json!({
        "token": TOKEN,
        "server": "https://cloud.example",
        "library_name": "Family",
        "kind": "link",
        "expires_at": "2020-01-01T00:00:00+00:00",
        "account": null
    });
    ops::write_state(
        &nora.lock().unwrap(),
        "pending_invitation",
        Some(&stale.to_string()),
    )
    .unwrap();

    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Expired {
            library_name: "Family".into()
        },
        "it is told that it expired, not that there was nothing"
    );
    assert!(
        !database_contains(&nora, TOKEN),
        "an expired token is removed, not just ignored"
    );
    // The note stays (the screen that comes after is another call) until it is dismissed.
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Expired {
            library_name: "Family".into()
        }
    );
    discard_pending_invitation(&nora).unwrap();
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

/// COS-003: accepting after the invitation expired while the person signed in says so, instead of «nothing pending».
#[tokio::test]
async fn accepting_an_invitation_that_expired_while_waiting_says_it_expired() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    sign_in_as(&nora, "nora");
    let stale = json!({
        "token": TOKEN,
        "server": "https://cloud.example",
        "library_name": "Family",
        "kind": "link",
        "expires_at": "2020-01-01T00:00:00+00:00",
        "account": null
    });
    ops::write_state(
        &nora.lock().unwrap(),
        "pending_invitation",
        Some(&stale.to_string()),
    )
    .unwrap();
    let api = cloud.api("nora").on(&nora);

    let refused = accept_pending(&nora, &api).await.unwrap_err();

    assert_eq!(code(&refused), "invitation_expired");
    assert_eq!(cloud.invitation_calls(), 0);
}

#[tokio::test]
async fn a_pending_token_is_not_taken_to_another_server() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);
    open(&nora, &api, &seed_link(&cloud).await).await.unwrap();
    sign_in_as(&nora, "nora");
    let elsewhere = cloud.api_on("nora", "https://other.example").on(&nora);
    let asked = cloud.invitation_calls();

    let refused = accept_pending(&nora, &elsewhere).await.unwrap_err();

    assert_eq!(code(&refused), "invitation_other_server");
    assert_eq!(cloud.invitation_calls(), asked);
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

#[tokio::test]
async fn discarding_forgets_what_was_waiting() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);
    open(&nora, &api, &seed_link(&cloud).await).await.unwrap();

    discard_pending_invitation(&nora).unwrap();

    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

#[tokio::test]
async fn an_invitation_that_is_not_valid_says_why_and_is_not_kept() {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let link = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap()
        .invitation
        .unwrap()
        .link
        .unwrap();
    let id = list_invitations(&laptop, &tersoal, &lib).await.unwrap()[0]
        .id
        .clone();
    revoke_invitation(&laptop, &tersoal, &lib, &id)
        .await
        .unwrap();
    let nora = Database::in_memory();
    let api = cloud.api("nora").on(&nora);

    let refused = open(&nora, &api, link.expose()).await.unwrap_err();

    assert_eq!(code(&refused), "invitation_revoked");
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
    let nowhere = open(&nora, &api, &format!("https://cloud.example/i/{TOKEN}"))
        .await
        .unwrap_err();
    assert_eq!(code(&nowhere), "invitation_not_found");
}

#[tokio::test]
async fn an_invitation_used_twice_gives_the_second_account_nothing_and_is_forgotten() {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let link = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap()
        .invitation
        .unwrap()
        .link
        .unwrap();
    let (nora, late) = (Database::in_memory(), Database::in_memory());
    let (first, second) = (cloud.api("nora").on(&nora), cloud.api("late").on(&late));
    open(&nora, &first, link.expose()).await.unwrap();
    open(&late, &second, link.expose()).await.unwrap();
    sign_in_as(&nora, "nora");
    sign_in_as(&late, "late");
    accept_pending(&nora, &first).await.unwrap();

    let refused = accept_pending(&late, &second).await.unwrap_err();

    assert_eq!(code(&refused), "invitation_used");
    assert_eq!(
        pending_invitation(&late).unwrap(),
        PendingInvitation::Nothing
    );
    assert!(!library_exists_here(&late, &lib));
}

#[tokio::test]
async fn an_email_invitation_opened_by_the_wrong_account_gets_nothing() {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let eva = invite(
        &laptop,
        &tersoal,
        &lib,
        new_person("Eva", InvitationKind::Email, Some("eva@example.com")),
    )
    .await
    .unwrap();
    assert!(eva.invitation.is_some());
    // The address came by mail: here the test takes the token from the fake server.
    let token = cloud
        .0
        .lock()
        .unwrap()
        .invitations
        .last()
        .unwrap()
        .token
        .clone();
    let stranger = Database::in_memory();
    sign_in_as(&stranger, "stranger");
    let api = cloud.api("stranger").on(&stranger);
    open(&stranger, &api, &format!("https://cloud.example/i/{token}"))
        .await
        .unwrap();

    let refused = accept_pending(&stranger, &api).await.unwrap_err();

    assert_eq!(code(&refused), "invitation_not_for_this_account");
    assert!(!library_exists_here(&stranger, &lib));
    assert_eq!(
        pending_invitation(&stranger).unwrap(),
        PendingInvitation::Nothing
    );
}

#[tokio::test]
async fn if_the_session_after_accepting_fails_the_invitation_is_already_used_and_the_result_says_so()
 {
    let (cloud, laptop, _phone, lib) = family().await;
    let tersoal = cloud.api("tersoal").on(&laptop);
    let link = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap()
        .invitation
        .unwrap()
        .link
        .unwrap();
    let nora = Database::in_memory();
    sign_in_as(&nora, "nora");
    let api = cloud.api("nora").on(&nora);
    open(&nora, &api, link.expose()).await.unwrap();
    cloud.fail_pulls_of(&cloud.only_library(), "schema_version");

    let accepted = accept_pending(&nora, &api).await.unwrap();

    assert_eq!(accepted.sync_error.as_deref(), Some("schema_version"));
    assert_eq!(accepted.library, None);
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

#[test]
fn only_what_a_retry_can_fix_leaves_the_invitation_waiting() {
    assert!(keeps_pending(&Error::account("server_unavailable")));
    assert!(keeps_pending(&Error::account("too_many_requests")));
    assert!(keeps_pending(&Error::Cloud("not_signed_in")));
    for answered in [
        "invitation_used",
        "invitation_expired",
        "invitation_revoked",
        "invitation_not_for_this_account",
    ] {
        assert!(!keeps_pending(&Error::account(answered)), "{answered}");
    }
    assert!(!keeps_pending(&Error::Cloud("invitation_not_found")));
}

#[test]
fn a_link_and_a_token_do_not_print() {
    let secret = invitations::Secret::new(TOKEN);
    assert!(!format!("{secret:?}").contains(TOKEN));
    let created = invitations::CreatedInvitation {
        id: "i".into(),
        kind: InvitationKind::Link,
        member_id: "m".into(),
        expires_at: "x".into(),
        link: Some(secret),
    };
    assert!(!format!("{created:?}").contains(TOKEN));
    for error in [
        Error::Cloud("invitation_bad_link"),
        Error::Cloud("invitation_other_server"),
    ] {
        assert!(!error.to_string().contains(TOKEN));
    }
}

// --- The notice to who invited -----------------------------------------------

#[test]
fn the_live_notice_of_an_accepted_invitation_is_read_and_anything_else_is_not() {
    assert_eq!(
        accepted_cloud_library(r#"{"event":"invitation_accepted","library":"C1"}"#).as_deref(),
        Some("C1")
    );
    for other in [
        "{}",
        "",
        "not json",
        r#"{"event":"library_changed","library":"C1"}"#,
        r#"{"event":"invitation_accepted"}"#,
        r#"{"library":"C1"}"#,
    ] {
        assert_eq!(accepted_cloud_library(other), None, "{other}");
    }
}

#[tokio::test]
async fn the_notice_names_the_library_only_if_this_device_has_it_in_that_cloud() {
    let (cloud, laptop, _phone, lib) = family().await;
    let c = cloud.only_library();
    let conn = laptop.lock().unwrap();

    let notice = accepted_notice(&conn, "https://cloud.example", &c)
        .unwrap()
        .unwrap();
    assert_eq!(notice.library, lib);
    // What a server invents, or another server's library, says nothing here.
    assert_eq!(
        accepted_notice(&conn, "https://cloud.example", "invented").unwrap(),
        None
    );
    assert_eq!(
        accepted_notice(&conn, "https://other.example", &c).unwrap(),
        None
    );
}

/// The official server: its links are `kosmotheka.app/i/…` (D79) and the account's server is the official one.
#[tokio::test]
async fn on_the_official_server_a_link_of_kosmotheka_app_is_followed() {
    let cloud = FakeCloud::default().with_link_base("https://kosmotheka.app");
    let (laptop, _phone, lib) = laptop_admin_phone_reader();
    sign_in_as(&laptop, "tersoal");
    let tersoal = cloud.api_on("tersoal", OFFICIAL_SERVER).on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    let link = invite(&laptop, &tersoal, &lib, link_for("Nora"))
        .await
        .unwrap()
        .invitation
        .unwrap()
        .link
        .unwrap();
    assert!(link.expose().starts_with("https://kosmotheka.app/i/"));
    let nora = Database::in_memory();
    let api = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);

    open(&nora, &api, link.expose()).await.unwrap();

    assert_eq!(
        cloud.peeks(),
        vec![(OFFICIAL_SERVER.to_owned(), token_in(&link))],
        "asked of the account's server, not of the host of the link"
    );
}
