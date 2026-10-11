//! Links the system hands to the app (7.6.4) against the in-memory cloud. Anybody can fire a link at the app, so
//! the hostile cases come first: whatever arrives asks no server unless it is an invitation link of the official
//! domain, never does more than look, and a flood of them is one look at a time.

use super::*;
use crate::account::OFFICIAL_SERVER;
use crate::cloud::invitations::{
    InvitationKind, InvitePerson, InviteRequest, PendingInvitation, invite, pending_invitation,
};
use crate::links::{Arrivals, Destination, attend, route};

const TOKEN: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn official(token: &str) -> String {
    format!("https://kosmotheka.app/i/{token}")
}

/// Everything that is not an invitation link of the official domain, or is one that cannot be right.
fn links_that_are_not_for_the_app() -> Vec<String> {
    vec![
        // Another domain, or something that only looks like ours.
        format!("https://evil.example/i/{TOKEN}"),
        format!("https://kosmotheka.app.evil.example/i/{TOKEN}"),
        format!("https://evilkosmotheka.app/i/{TOKEN}"),
        format!("https://api.kosmotheka.app/i/{TOKEN}"),
        format!("https://kosmotheka.com/i/{TOKEN}"),
        format!("http://kosmotheka.app/i/{TOKEN}"),
        // Another path of our own domain.
        format!("https://kosmotheka.app/{TOKEN}"),
        format!("https://kosmotheka.app/privacy/{TOKEN}"),
        format!("https://kosmotheka.app/x/i/{TOKEN}"),
        "https://kosmotheka.app/".to_owned(),
        "https://kosmotheka.app".to_owned(),
        // Schemes the app registers or might be given.
        format!("kosmotheka://i/{TOKEN}"),
        format!("kosmotheka://kosmotheka.app/i/{TOKEN}"),
        "kosmotheka://anything".to_owned(),
        format!("javascript:alert('{TOKEN}')"),
        format!("file:///etc/passwd#{TOKEN}"),
        format!("ftp://kosmotheka.app/i/{TOKEN}"),
        // Not a link at all.
        String::new(),
        "   ".to_owned(),
        TOKEN.to_owned(),
        "https://".to_owned(),
        "\u{0}\u{1}".to_owned(),
        "x".repeat(100_000),
    ]
}

/// Links of the right place whose form is wrong: they reach the screen (which says so) but ask nobody.
fn links_of_ours_that_are_wrong() -> Vec<String> {
    vec![
        format!("https://user@kosmotheka.app/i/{TOKEN}"),
        format!("https://user:pass@kosmotheka.app/i/{TOKEN}"),
        format!("https://kosmotheka.app/i/{TOKEN}?next=https://evil.example"),
        format!("https://kosmotheka.app/i/{TOKEN}#x"),
        format!("https://kosmotheka.app:8443/i/{TOKEN}"),
        "https://kosmotheka.app/i/short".to_owned(),
        format!("https://kosmotheka.app/i/{TOKEN}/more"),
        format!("https://kosmotheka.app/i/{TOKEN}x"),
        "https://kosmotheka.app/i/".to_owned(),
        "https://kosmotheka.app/i".to_owned(),
    ]
}

async fn invitation_link(
    cloud: &FakeCloud,
    laptop: &Database,
    lib: &str,
    name: &str,
) -> invitations::Secret {
    let tersoal = cloud.api("tersoal").on(laptop);
    let request = InviteRequest {
        person: InvitePerson::New { name: name.into() },
        kind: InvitationKind::Link,
        email: None,
    };
    let made = invite(laptop, &tersoal, lib, request).await.unwrap();
    made.invitation.unwrap().link.unwrap()
}

async fn family_with_a_link() -> (FakeCloud, invitations::Secret) {
    let cloud = FakeCloud::default();
    let (laptop, phone, lib) = laptop_admin_phone_reader();
    sign_in_as(&laptop, "tersoal");
    let tersoal = cloud.api("tersoal").on(&laptop);
    upload_settled(&laptop, &tersoal, &lib).await;
    lan_sync(&laptop, &phone, &lib);
    let link = invitation_link(&cloud, &laptop, &lib, "Nora").await;
    (cloud, link)
}

#[test]
fn only_an_invitation_link_of_the_official_domain_is_for_the_app() {
    for link in links_that_are_not_for_the_app() {
        assert_eq!(route(&link), None, "ignored: {:.60}", link);
    }
    for link in links_of_ours_that_are_wrong() {
        assert!(
            matches!(route(&link), Some(Destination::Invitation(_))),
            "{link}"
        );
    }
    assert!(matches!(
        route(&official(TOKEN)),
        Some(Destination::Invitation(_))
    ));
    // Capitals in the host and a final dot are the same host.
    assert!(route(&format!("HTTPS://KOSMOTHEKA.APP/i/{TOKEN}")).is_some());
}

#[tokio::test]
async fn a_link_that_is_not_for_the_app_asks_no_server_and_shows_nothing() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);
    let arrivals = Arrivals::default();

    for link in links_that_are_not_for_the_app() {
        assert!(!arrivals.arrive([&link]), "nothing to show: {:.60}", link);
        assert_eq!(attend(&arrivals, &nora, &api).await.unwrap(), None);
    }

    assert_eq!(cloud.invitation_calls(), 0, "no server was asked");
    assert!(cloud.peeks().is_empty());
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

#[tokio::test]
async fn a_link_of_ours_with_a_wrong_form_is_said_wrong_without_asking_any_server() {
    let cloud = FakeCloud::default();
    let nora = Database::in_memory();
    let api = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);
    let arrivals = Arrivals::default();

    for link in links_of_ours_that_are_wrong() {
        assert!(arrivals.arrive([&link]));
        let refused = attend(&arrivals, &nora, &api).await;
        assert!(refused.is_err(), "{link}");
    }

    assert_eq!(cloud.invitation_calls(), 0, "no server was asked");
    assert!(cloud.peeks().is_empty());
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        PendingInvitation::Nothing
    );
}

/// The server asked is the account's, whatever the link says: a link of the official domain on a device whose
/// account is on a server of its own is refused without that server being sent the token.
#[tokio::test]
async fn the_link_does_not_choose_the_server_that_is_asked() {
    let (cloud, link) = family_with_a_link().await;
    let nora = Database::in_memory();
    let own = cloud.api_on("nora", "https://api.example.org").on(&nora);
    let arrivals = Arrivals::default();
    // The invitation is real, but it was made on `https://cloud.example` and the link names the official domain.
    assert!(arrivals.arrive([official(&token_of(&link))]));

    let refused = attend(&arrivals, &nora, &own).await;

    assert!(refused.is_err());
    assert!(cloud.peeks().is_empty(), "the token never reached it");
}

fn token_of(link: &invitations::Secret) -> String {
    link.expose().rsplit('/').next().unwrap().to_owned()
}

/// A link looks and, at most, keeps its token waiting for the account that accepts it: nothing is accepted, the
/// library is not entered, nothing is uploaded or deleted, and the account stays as it was.
#[tokio::test]
async fn a_link_only_looks_it_accepts_nothing_and_changes_nothing() {
    let (cloud, link) = family_with_a_link().await;
    let nora = Database::in_memory();
    sign_in_as(&nora, "nora");
    let api = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);
    let libraries_before =
        nora.lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM library", [], |row| {
                row.get::<_, i64>(0)
            });
    let arrivals = Arrivals::default();
    assert!(arrivals.arrive([official(&token_of(&link))]));

    let looked = attend(&arrivals, &nora, &api).await;

    assert!(looked.unwrap().is_some(), "it was only looked at");
    assert_eq!(cloud.peeks().len(), 1, "and that was the only thing asked");
    assert_eq!(cloud.accepts(), 0, "nothing was accepted");
    assert_eq!(
        nora.lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM library", [], |row| row
                .get::<_, i64>(0)),
        libraries_before,
        "no library was entered"
    );
    assert!(
        crate::account::is_signed_in(&nora.lock().unwrap()).unwrap(),
        "the session is as it was"
    );
}

#[tokio::test]
async fn a_hundred_links_are_one_look_at_a_time_and_the_one_being_read_stays() {
    let (cloud, link) = family_with_a_link().await;
    let nora = Database::in_memory();
    let api = cloud.api_on("nora", OFFICIAL_SERVER).on(&nora);
    let arrivals = Arrivals::default();

    for n in 0..100 {
        arrivals.arrive([format!("https://kosmotheka.app/i/{n:0>43}")]);
    }
    arrivals.arrive([official(&token_of(&link))]);
    let first = attend(&arrivals, &nora, &api).await.unwrap();
    assert!(first.is_some(), "the newest one is looked at");
    assert_eq!(cloud.peeks().len(), 1, "one look, not a hundred");
    assert_eq!(attend(&arrivals, &nora, &api).await.unwrap(), None);
    assert_eq!(cloud.peeks().len(), 1);

    // The one the person is reading is kept (in Rust) while another arrives: a newer link takes its place
    // only after the person has looked at it, never in silence over what is on screen.
    let reading = pending_invitation(&nora).unwrap();
    assert!(matches!(reading, PendingInvitation::Waiting(_)));
    arrivals.arrive(["https://kosmotheka.app/i/BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"]);
    assert_eq!(
        pending_invitation(&nora).unwrap(),
        reading,
        "arriving does not touch what is waiting"
    );
}

#[test]
fn a_link_is_in_no_message_and_no_debug_output() {
    let arrivals = Arrivals::default();
    arrivals.arrive([official(TOKEN)]);
    let taken = arrivals.take().unwrap();
    assert!(!format!("{taken:?}").contains(TOKEN));
}
