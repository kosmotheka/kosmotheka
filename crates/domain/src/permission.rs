//! Who may do what in a library (D55, D73). Pure rules: the caller answers the
//! questions in `LibraryView` from the library as it stood when the operation was
//! made, so every device and the server reach the same verdict.

use crate::operation::{Action, Change, Operation};
use crate::schema::{Entity, TRASHABLE};

const DAY_MILLIS: u64 = 24 * 60 * 60 * 1000;
/// How long something stays in the trash before an editor may delete it for good (D105, rule 16).
pub const TRASH_DAYS: u64 = 30;
/// How long before the cloud does it, as a safety net when no device has (D105, rule 16).
pub const CLOUD_TRASH_DAYS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Editor,
    Reader,
}

impl Role {
    pub fn parse(text: &str) -> Option<Role> {
        match text {
            "admin" => Some(Role::Admin),
            "editor" => Some(Role::Editor),
            "reader" => Some(Role::Reader),
            _ => None,
        }
    }
}

/// What the rules need to know about the library at the moment of an operation.
pub trait LibraryView {
    /// Whether the library is founded: some member has a role. Until then nobody may do anything but register
    /// their own device, and the only operation that counts is the one that founds it (D97, rule 1).
    fn founded(&self) -> bool;
    /// The role of the person `device` belongs to. `None` when the device belongs
    /// to nobody or its person has no role.
    fn role_of_device(&self, device: &str) -> Option<Role>;
    /// Whether the row was created by a device of the same person as `device` (D45).
    fn created_by_same_person(&self, entity: Entity, id: &str, device: &str) -> bool;
    /// Whether the operation `request` was written by a device of the same person
    /// as `device`: who asked may withdraw what they asked for.
    fn requested_by_same_person(&self, request: &str, device: &str) -> bool;
    /// The person (member id) `device` belongs to, if any. A device whose `vouched_key` is set
    /// and is not the key it is registered with belongs to nobody (D96), and neither does one
    /// of a person whose access was withdrawn (D98).
    fn member_of_device(&self, device: &str) -> Option<String>;
    /// The `member_id` of `device` as its row says, whatever its `vouched_key` or the person's access say; `None`
    /// when the device is not registered or has none. Leaving a library asks this (rule 15): the devices it cuts
    /// off must be the signer's person's in the state of whoever receives it.
    fn raw_member_of_device(&self, device: &str) -> Option<String>;
    /// Whether the `member_id` of `device` is set, whatever its `vouched_key` says. The
    /// cloud's rule asks this, not `member_of_device`: it only links devices of nobody (D76).
    fn device_has_member_id(&self, device: &str) -> bool;
    /// Whose the mark `id` is, if it exists (D68).
    fn mark_member(&self, id: &str) -> Option<String>;
    /// Whether `operation` falls under a honoured fence of its author (D84, D87):
    /// dated before the operation that took the author's power away, and neither
    /// the last one the administrator had nor one of its ancestors through
    /// `previous`. Such an operation cannot be honest.
    /// Also after a revocation of the author still in force at its HLC (D71): a
    /// revoked device writes nothing, even through a device that has not heard of it yet.
    fn fenced(&self, operation: &Operation) -> bool;
    /// The device `library.cloud_device_id` names now (D76, D95), if any.
    fn cloud_device(&self) -> Option<String>;
    /// Whether the person `member` exists.
    fn member_exists(&self, member: &str) -> bool;
    /// Whether the person `member` has an `account_id`.
    fn member_has_account(&self, member: &str) -> bool;
    /// Whether the person `member` has had their access withdrawn (D98).
    fn member_access_withdrawn(&self, member: &str) -> bool;
    /// Since when the row is in the trash (D105), in milliseconds since 1970, counting what it hangs from: the
    /// copies, details, links and work of an edition in the trash (a work only once all its editions are), the
    /// links of a tag and the entries of a series. The oldest when it hangs from several; `None` when nothing is.
    fn trashed_since(&self, entity: Entity, id: &str) -> Option<u64>;
}

/// Whether the author of `operation` may make all of its changes. All or nothing:
/// one change not allowed makes the whole operation a request.
pub fn is_allowed(operation: &Operation, view: &impl LibraryView) -> bool {
    // Every operation is signed (D56): without a signature nobody can tell who wrote it.
    if operation.signature.is_none() {
        return false;
    }
    let author = operation.hlc.device.as_str();
    // A fenced author is judged as nobody's: its date cannot be trusted, or its access was withdrawn (D71, D84).
    let fenced = view.fenced(operation);
    // The operation that founds the library (D97): the only one allowed while nobody has a role. Whoever
    // holds the key the id derives from founds it, whatever date the operation carries.
    if !fenced && !view.founded() && is_founding(operation) {
        return true;
    }
    // A person leaving the library themself (D102, rule 15): judged before the roles, since it is for any role.
    if !fenced && is_exit(operation, author, view) {
        return true;
    }
    let role = if fenced {
        None
    } else {
        view.role_of_device(author)
    };
    // The cloud's power (D76, D95) is the named device's, unless a fence makes it
    // nobody's. Asked only when a change could use it: its fields, or deleting from the trash (D105).
    let cloud = !fenced
        && operation
            .changes
            .iter()
            .any(|change| touches_cloud_field(change) || change.action == Action::Delete)
        && view.cloud_device().is_some_and(|cloud| cloud == author);
    let at = operation.hlc.millis;
    operation
        .changes
        .iter()
        .all(|change| change_allowed(change, author, role, cloud, at, view))
}

/// A person leaving the library themself (D102, rule 15). The operation is signed by a device of that person
/// (`member_of_device`: its vouched key matches and their access is not withdrawn already) and holds exactly:
/// - their own `member` row updated with `access_withdrawn: true` and nothing else;
/// - for devices that are hers now (their `member_id`, as it stands), the take-away of the person and the vouched
///   key, which is the same `device` update that expelling writes (rule 14); the signing device also fences itself
///   at its previous operation (`trusted_through`), the others are not fenced;
/// - a `revocation` of such a device, with its name.
///
/// Anything else, a device of another person included, makes the whole operation a request.
fn is_exit(operation: &Operation, author: &str, view: &impl LibraryView) -> bool {
    let Some(person) = view.member_of_device(author) else {
        return false;
    };
    let is_hers = |device: &str| {
        view.raw_member_of_device(device)
            .is_some_and(|member| member.eq_ignore_ascii_case(&person))
    };
    let exactly = |change: &Change, keys: &[&str]| {
        change.fields.len() == keys.len() && keys.iter().all(|key| change.fields.contains_key(*key))
    };
    let mut marked = false;
    for change in &operation.changes {
        match (change.entity, change.action) {
            (Entity::Member, Action::Update) => {
                if marked
                    || !change.id.eq_ignore_ascii_case(&person)
                    || !exactly(change, &["access_withdrawn"])
                    || change.fields["access_withdrawn"].as_bool() != Some(true)
                {
                    return false;
                }
                marked = true;
            }
            (Entity::Device, Action::Update) => {
                let own = change.id == author;
                let keys: &[&str] = if own {
                    &["member_id", "vouched_key", "trusted_through"]
                } else {
                    &["member_id", "vouched_key"]
                };
                if !exactly(change, keys)
                    || !change.fields["member_id"].is_null()
                    || !change.fields["vouched_key"].is_null()
                    || !is_hers(&change.id)
                {
                    return false;
                }
                if own {
                    // Everything the device did before is an ancestor of its previous operation: nothing of it is disbelieved.
                    let through = change.fields["trusted_through"].as_str();
                    if through != Some(operation.previous.as_deref().unwrap_or("")) {
                        return false;
                    }
                }
            }
            (Entity::Revocation, Action::Create) => {
                let device = change.fields.get("device_id").and_then(|v| v.as_str());
                if !exactly(change, &["device_id", "device_name"])
                    || change.fields["device_name"].as_str().is_none()
                    || !device.is_some_and(is_hers)
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    marked
}

/// Whether `operation` is the founding of its library (D97, rule 1): it creates the `library` row with a
/// `founding_nonce`, and the public key its author gives in its own `device` row of the same operation, with that
/// nonce, derives the id of the library. Nothing else is asked here; whether the library is still unfounded is
/// the caller's.
fn is_founding(operation: &Operation) -> bool {
    let text = |change: &Change, field: &str| {
        change
            .fields
            .get(field)
            .and_then(|value| value.as_str())
            .map(str::to_owned)
    };
    let nonce = operation.changes.iter().find_map(|change| {
        (change.entity == Entity::Library
            && change.action == Action::Create
            && change.id.eq_ignore_ascii_case(&operation.library))
        .then(|| text(change, "founding_nonce"))
        .flatten()
    });
    let key = operation.changes.iter().find_map(|change| {
        (change.entity == Entity::Device
            && change.action != Action::Delete
            && change.id == operation.hlc.device)
            .then(|| text(change, "public_key"))
            .flatten()
    });
    match (key, nonce) {
        (Some(key), Some(nonce)) => crate::library_id(&key, &nonce)
            .is_some_and(|id| id == operation.library.to_ascii_lowercase()),
        _ => false,
    }
}

/// Whether the change writes something only the cloud device may write, or the one
/// thing it may do beyond a device of nobody: `member.account_id`, or `device.member_id`
/// with the `vouched_key` that goes with it (D96).
fn touches_cloud_field(change: &Change) -> bool {
    match change.entity {
        Entity::Member => change.fields.contains_key("account_id"),
        Entity::Device => {
            change.fields.contains_key("member_id") || change.fields.contains_key("vouched_key")
        }
        _ => false,
    }
}

/// The two changes the cloud device may make besides what a device of nobody may do
/// (D76, D96): `account_id` of a person who exists, which is all the change touches; and
/// the link of another device that belongs to nobody to a person with an account, which
/// touches exactly `member_id` and `vouched_key`, both set. The cloud vouches for what it
/// links; whether that key is the one the device is registered with is rule 13's business.
fn cloud_allowed(change: &Change, author: &str, at: u64, view: &impl LibraryView) -> bool {
    // Its one power over the catalog: deleting for good what has been 60 days in the trash (D105).
    if change.action == Action::Delete {
        return in_trash_for(change, at, CLOUD_TRASH_DAYS, view);
    }
    if change.action != Action::Update {
        return false;
    }
    let text = |field: &str| {
        change
            .fields
            .get(field)
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
    };
    match change.entity {
        Entity::Member => {
            change.fields.len() == 1
                && change.fields.contains_key("account_id")
                && view.member_exists(&change.id)
        }
        Entity::Device => {
            change.fields.len() == 2
                && text("vouched_key").is_some()
                && change.id != author
                && !view.device_has_member_id(&change.id)
                && text("member_id").is_some_and(|member| {
                    view.member_has_account(member) && !view.member_access_withdrawn(member)
                })
        }
        _ => false,
    }
}

/// Whether the row, or what it hangs from, has been in the trash at least `days` at the moment `at` (D105).
fn in_trash_for(change: &Change, at: u64, days: u64, view: &impl LibraryView) -> bool {
    view.trashed_since(change.entity, &change.id)
        .is_some_and(|since| at >= since.saturating_add(days * DAY_MILLIS))
}

/// What a change does with the trash fields (D105): `None` when it does not touch them.
enum TrashMove<'a> {
    /// `trashed_at` set, with the person it names in `trashed_by`.
    Send(Option<&'a str>),
    /// Both null.
    Restore,
}

fn trash_move(change: &Change) -> Option<TrashMove<'_>> {
    if !TRASHABLE.contains(&change.entity) {
        return None;
    }
    let at = change.fields.get("trashed_at")?;
    Some(if at.is_null() {
        TrashMove::Restore
    } else {
        TrashMove::Send(change.fields.get("trashed_by").and_then(|v| v.as_str()))
    })
}

fn change_allowed(
    change: &Change,
    author: &str,
    role: Option<Role>,
    cloud: bool,
    at: u64,
    view: &impl LibraryView,
) -> bool {
    // `founding_nonce` is written once, by the founding (judged before this is asked): nobody writes it
    // afterwards, an administrator included (D97).
    if change.entity == Entity::Library && change.fields.contains_key("founding_nonce") {
        return false;
    }
    // `account_id` is written by the cloud device alone: not by an administrator, not
    // when the member is created.
    if change.entity == Entity::Member && change.fields.contains_key("account_id") {
        return cloud && cloud_allowed(change, author, at, view);
    }
    if cloud && cloud_allowed(change, author, at, view) {
        return true;
    }
    // Who sent something to the trash is the signer's person, an administrator's included: the date and the name
    // in the trash are compared, never taken as said (D84, D105).
    if change.action == Action::Update
        && let Some(TrashMove::Send(by)) = trash_move(change)
        && !by.is_some_and(|by| {
            view.member_of_device(author)
                .is_some_and(|me| me.eq_ignore_ascii_case(by))
        })
    {
        return false;
    }
    let someone = role.is_some();
    let role = role.unwrap_or(Role::Reader);
    if role == Role::Admin {
        return true;
    }
    let touches = |field: &str| change.fields.contains_key(field);
    match change.entity {
        // Every device registers and renames itself; saying whose it is (and vouching
        // for its key, D96), or touching another device, or its fence, is for administrators.
        Entity::Device if change.id == author => {
            change.action != Action::Delete
                && !touches("member_id")
                && !touches("vouched_key")
                && !touches("trusted_through")
        }
        // Withdrawing is the one resolution that is not an administrator's.
        Entity::ChangeRequest => {
            change.action == Action::Create
                && change.fields.len() == 1
                && change.fields.get("status").and_then(|v| v.as_str()) == Some("withdrawn")
                && view.requested_by_same_person(&change.id, author)
        }
        Entity::Device | Entity::Library | Entity::Revocation => false,
        // Nobody gives more than they have.
        // Deleting a person is for administrators, even one the editor created herself (SEG-034): a deleted
        // person leaves her devices without a person and her account without a library.
        Entity::Member if change.action == Action::Delete => false,
        // `access_withdrawn` goes with `role` (D98): whoever may not give a role may not echar a person
        // or give the access back, the person themself included.
        Entity::Member
            if touches("role") || touches("can_share") || touches("access_withdrawn") =>
        {
            false
        }
        // Each person keeps their own marks, whatever their role (D68).
        Entity::Mark => {
            let Some(me) = view.member_of_device(author).filter(|_| someone) else {
                return false;
            };
            let owner = match change.action {
                Action::Create => change
                    .fields
                    .get("member_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned),
                Action::Update | Action::Delete => view.mark_member(&change.id),
            };
            owner.is_some_and(|owner| owner.eq_ignore_ascii_case(&me))
        }
        _ => match (role, change.action) {
            (Role::Reader, _) => false,
            // Sending to the trash is deleting as it was before the trash (D105, rule 16): her person's rows only.
            // Restoring is any editor's, whoever sent it.
            (_, Action::Update) => match trash_move(change) {
                Some(TrashMove::Send(_)) => {
                    view.created_by_same_person(change.entity, &change.id, author)
                }
                Some(TrashMove::Restore) | None => true,
            },
            // A row created already in the trash (restoring a backup) keeps whoever had sent it, as long as that
            // person is in the library. An administrator's restore creates the people in the same operation, so
            // it is not asked: she may do everything.
            (_, Action::Create) => match trash_move(change) {
                Some(TrashMove::Send(by)) => by.is_some_and(|by| view.member_exists(by)),
                _ => true,
            },
            // For good: anything that has been 30 days in the trash, or hangs from something that has, whoever
            // created it. The four things of the trash never go straight; the other rows, as before (rule 3).
            (_, Action::Delete) => {
                in_trash_for(change, at, TRASH_DAYS, view)
                    || (!TRASHABLE.contains(&change.entity)
                        && view.created_by_same_person(change.entity, &change.id, author))
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct View {
        founded: bool,
        role: Option<Role>,
        same_person: bool,
        fenced: bool,
        cloud: bool,
    }

    impl LibraryView for View {
        fn founded(&self) -> bool {
            self.founded
        }
        fn role_of_device(&self, _: &str) -> Option<Role> {
            self.role
        }
        fn created_by_same_person(&self, _: Entity, _: &str, _: &str) -> bool {
            self.same_person
        }
        fn requested_by_same_person(&self, _: &str, _: &str) -> bool {
            self.same_person
        }
        fn member_of_device(&self, _: &str) -> Option<String> {
            self.role.map(|_| MY_MEMBER.to_owned())
        }
        fn raw_member_of_device(&self, _: &str) -> Option<String> {
            self.role.map(|_| MY_MEMBER.to_owned())
        }
        fn device_has_member_id(&self, _: &str) -> bool {
            self.role.is_some()
        }
        fn fenced(&self, _: &Operation) -> bool {
            self.fenced
        }
        fn cloud_device(&self) -> Option<String> {
            self.cloud.then(|| ME.to_owned())
        }
        fn member_exists(&self, _: &str) -> bool {
            true
        }
        fn member_has_account(&self, member: &str) -> bool {
            member == MY_MEMBER
        }
        fn member_access_withdrawn(&self, _: &str) -> bool {
            false
        }
        fn trashed_since(&self, _: Entity, _: &str) -> Option<u64> {
            None
        }
        fn mark_member(&self, _: &str) -> Option<String> {
            Some(
                if self.same_person {
                    MY_MEMBER
                } else {
                    OTHER_MEMBER
                }
                .to_owned(),
            )
        }
    }

    const MY_MEMBER: &str = "01997c7e-0000-7000-8000-000000000013";
    const OTHER_MEMBER: &str = "01997c7e-0000-7000-8000-000000000012";

    #[test]
    fn everyone_with_a_person_keeps_their_own_marks_and_only_theirs() {
        const WORK: &str = "01997c7e-0000-7000-8000-0000000000d1";
        let mark = |member: &str| {
            op(vec![Change::create(
                Entity::Mark,
                crate::mark::mark_id(member, WORK),
                json!({ "member_id": member, "work_id": WORK, "status": "finished" }),
            )])
        };
        let update = op(vec![Change::update(
            Entity::Mark,
            crate::mark::mark_id(MY_MEMBER, WORK),
            json!({ "rating": 5 }),
        )]);
        let reader = view(Some(Role::Reader));
        let mine = View {
            same_person: true,
            ..view(Some(Role::Reader))
        };
        assert!(is_allowed(&mark(MY_MEMBER), &reader));
        assert!(!is_allowed(&mark(OTHER_MEMBER), &view(Some(Role::Editor))));
        assert!(is_allowed(&mark(OTHER_MEMBER), &view(Some(Role::Admin))));
        assert!(is_allowed(&update, &mine));
        assert!(!is_allowed(&update, &reader), "someone else's mark");
        assert!(
            !is_allowed(&mark(MY_MEMBER), &view(None)),
            "a device of nobody"
        );
    }

    const ME: &str = "01997c7e-0000-7000-8000-0000000000fb";

    fn op(changes: Vec<Change>) -> Operation {
        Operation {
            id: "01997c7e-0000-7000-8000-000000000001".into(),
            library: "01997c7e-0000-7000-8000-00000000aaaa".into(),
            hlc: format!("0001727222222222-00000000-{ME}").parse().unwrap(),
            intent: "test.op".into(),
            changes,
            previous: None,
            signature: Some("signed".into()),
        }
    }

    fn view(role: Option<Role>) -> View {
        View {
            founded: true,
            role,
            same_person: false,
            fenced: false,
            cloud: false,
        }
    }

    const TAG: &str = "01997c7e-0000-7000-8000-000000000061";

    #[test]
    fn roles_decide_what_each_device_may_do() {
        let delete = op(vec![Change::delete(Entity::Tag, TAG)]);
        assert!(!is_allowed(&delete, &view(Some(Role::Reader))));
        assert!(!is_allowed(&delete, &view(Some(Role::Editor))));
        assert!(is_allowed(&delete, &view(Some(Role::Admin))));
    }

    const FOUNDER_KEY: &str = "iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w";
    const NONCE: &str = "AAECAwQFBgcICQoLDA0ODw";
    const OTHER_KEY: &str = "jojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w";
    const FOUNDED_ID: &str = "2c33de53-4834-8a05-b014-c42dc60bd4e1";

    /// The operation that founds a library (D97): the device creates its own row with its key, a person with the role
    /// of administrator and the library row with the nonce, all in the one operation.
    fn founding(key: &str, nonce: &str, library: &str) -> Operation {
        let mut founding = op(vec![
            Change::create(
                Entity::Device,
                ME,
                json!({ "public_key": key, "name": "laptop", "member_id": MY_MEMBER, "vouched_key": key }),
            ),
            Change::create(
                Entity::Member,
                MY_MEMBER,
                json!({ "name": "Ana", "role": "admin", "can_share": true }),
            ),
            Change::create(
                Entity::Library,
                library,
                json!({ "name": "Casa", "founding_nonce": nonce }),
            ),
        ]);
        founding.library = library.to_owned();
        founding
    }

    fn unfounded() -> View {
        View {
            founded: false,
            ..view(None)
        }
    }

    #[test]
    fn the_key_the_id_derives_from_founds_the_library_and_nobody_else_does() {
        // Hostile first (D97): a device whose key is not the one the id derives from, or a nonce that is not
        // the one, founds nothing: its operation is a request, whatever date it carries.
        assert!(!is_allowed(
            &founding(OTHER_KEY, NONCE, FOUNDED_ID),
            &unfounded()
        ));
        assert!(!is_allowed(
            &founding(FOUNDER_KEY, "AAECAwQFBgcICQoLDA0ODg", FOUNDED_ID),
            &unfounded()
        ));
        assert!(!is_allowed(
            &founding(FOUNDER_KEY, NONCE, "01997c7e-0000-7000-8000-00000000aaaa"),
            &unfounded()
        ));
        assert!(is_allowed(
            &founding(FOUNDER_KEY, NONCE, FOUNDED_ID),
            &unfounded()
        ));
    }

    #[test]
    fn a_library_is_founded_once() {
        // The very same founding, when somebody already has a role, is judged like any operation: its author
        // is a device of nobody's then (or whatever it is), and cannot give itself a role or touch the library.
        assert!(!is_allowed(
            &founding(FOUNDER_KEY, NONCE, FOUNDED_ID),
            &view(None)
        ));
        assert!(!is_allowed(
            &founding(FOUNDER_KEY, NONCE, FOUNDED_ID),
            &view(Some(Role::Editor))
        ));
    }

    #[test]
    fn the_nonce_is_written_by_the_founding_alone_even_for_an_administrator() {
        let rewrite = op(vec![Change::update(
            Entity::Library,
            "01997c7e-0000-7000-8000-00000000aaaa",
            json!({ "founding_nonce": "AAECAwQFBgcICQoLDA0ODg" }),
        )]);
        assert!(!is_allowed(&rewrite, &view(Some(Role::Admin))));
        let rename = op(vec![Change::update(
            Entity::Library,
            "01997c7e-0000-7000-8000-00000000aaaa",
            json!({ "name": "Otra" }),
        )]);
        assert!(is_allowed(&rename, &view(Some(Role::Admin))));
    }

    #[test]
    fn a_fenced_founder_founds_nothing() {
        let fenced = View {
            fenced: true,
            ..unfounded()
        };
        assert!(!is_allowed(
            &founding(FOUNDER_KEY, NONCE, FOUNDED_ID),
            &fenced
        ));
    }

    #[test]
    fn in_an_unfounded_library_nobody_writes_but_registers_its_own_device() {
        let tag = op(vec![Change::create(
            Entity::Tag,
            TAG,
            json!({ "name": "Tehanu" }),
        )]);
        assert!(!is_allowed(&tag, &unfounded()));
        let register = op(vec![Change::create(
            Entity::Device,
            ME,
            json!({ "public_key": FOUNDER_KEY, "name": "laptop" }),
        )]);
        assert!(is_allowed(&register, &unfounded()));
    }

    #[test]
    fn an_editor_deletes_what_her_person_created_but_the_trash_things_only_through_the_trash() {
        let mine = View {
            same_person: true,
            ..view(Some(Role::Editor))
        };
        // A link is deleted straight, as before the trash; a tag goes to the trash first (D105).
        assert!(is_allowed(
            &op(vec![Change::delete(Entity::WorkTag, TAG)]),
            &mine
        ));
        assert!(!is_allowed(
            &op(vec![Change::delete(Entity::Tag, TAG)]),
            &mine
        ));
    }

    #[test]
    fn a_device_registers_itself_but_does_not_say_whose_it_is() {
        let register = op(vec![Change::create(
            Entity::Device,
            ME,
            json!({ "public_key": "k", "name": "Phone" }),
        )]);
        let claim = op(vec![Change::update(
            Entity::Device,
            ME,
            json!({ "member_id": "01997c7e-0000-7000-8000-000000000011" }),
        )]);
        assert!(is_allowed(&register, &view(None)));
        assert!(!is_allowed(&claim, &view(Some(Role::Editor))));
    }

    #[test]
    fn only_an_administrator_vouches_and_never_the_cloud() {
        const OTHER: &str = "01997c7e-0000-7000-8000-0000000000fc";
        let vouch = |device: &str| {
            op(vec![Change::update(
                Entity::Device,
                device,
                json!({ "vouched_key": "iojj3XQJ8ZX9UtstPLpdcspnCb8dlBIb83SIAbQPb1w" }),
            )])
        };
        assert!(
            !is_allowed(&vouch(ME), &view(Some(Role::Editor))),
            "its own"
        );
        assert!(!is_allowed(&vouch(OTHER), &view(Some(Role::Editor))));
        assert!(!is_allowed(&vouch(ME), &view(None)), "a device of nobody");
        assert!(is_allowed(&vouch(OTHER), &view(Some(Role::Admin))));
        let cloud = View {
            cloud: true,
            ..view(None)
        };
        let link_and_vouch = op(vec![Change::update(
            Entity::Device,
            OTHER,
            json!({ "member_id": MY_MEMBER, "vouched_key": "AAAA" }),
        )]);
        assert!(!is_allowed(&vouch(OTHER), &cloud), "alone");
        assert!(
            is_allowed(&link_and_vouch, &cloud),
            "with the link it makes"
        );
    }

    #[test]
    fn an_unsigned_operation_is_never_allowed() {
        let mut add = op(vec![Change::create(
            Entity::Tag,
            TAG,
            json!({ "name": "Tehanu" }),
        )]);
        assert!(is_allowed(&add, &view(Some(Role::Editor))));
        add.signature = None;
        assert!(!is_allowed(&add, &view(Some(Role::Admin))));
        assert!(!is_allowed(&add, &view(None)), "nor a device of nobody");
    }

    #[test]
    fn whoever_asked_withdraws_and_nothing_more() {
        const REQUEST: &str = "01997c7e-0000-7000-8000-000000000077";
        let withdraw = op(vec![Change::create(
            Entity::ChangeRequest,
            REQUEST,
            json!({ "status": "withdrawn" }),
        )]);
        let accept = op(vec![Change::create(
            Entity::ChangeRequest,
            REQUEST,
            json!({ "status": "accepted" }),
        )]);
        let asker = View {
            same_person: true,
            ..view(Some(Role::Reader))
        };
        assert!(is_allowed(&withdraw, &asker));
        assert!(!is_allowed(&accept, &asker));
        assert!(!is_allowed(&withdraw, &view(Some(Role::Editor))));
    }

    #[test]
    fn a_fenced_device_can_only_do_what_a_device_of_nobody_can() {
        let delete = op(vec![Change::delete(Entity::Tag, TAG)]);
        let register = op(vec![Change::update(
            Entity::Device,
            ME,
            json!({ "name": "Phone" }),
        )]);
        let fenced = View {
            fenced: true,
            ..view(Some(Role::Admin))
        };
        assert!(is_allowed(&delete, &view(Some(Role::Admin))));
        assert!(!is_allowed(&delete, &fenced));
        assert!(is_allowed(&register, &fenced), "renaming itself is for all");
    }

    #[test]
    fn a_device_never_sets_its_own_fence() {
        let own = op(vec![Change::update(
            Entity::Device,
            ME,
            json!({ "trusted_through": "" }),
        )]);
        let other = op(vec![Change::update(
            Entity::Device,
            "01997c7e-0000-7000-8000-0000000000fc",
            json!({ "trusted_through": "" }),
        )]);
        assert!(!is_allowed(&own, &view(Some(Role::Editor))));
        assert!(!is_allowed(&other, &view(Some(Role::Editor))));
        assert!(is_allowed(&other, &view(Some(Role::Admin))));
        assert!(!is_allowed(&own, &view(None)));
    }

    const ACCOUNT: &str = "01997c7e-0000-7000-8000-0000000000a1";

    fn cloud() -> View {
        View {
            cloud: true,
            ..view(None)
        }
    }

    #[test]
    fn the_cloud_device_writes_accounts_and_links_devices_of_nobody_and_nothing_else() {
        let other = "01997c7e-0000-7000-8000-0000000000fc";
        let account = |fields: serde_json::Value| {
            op(vec![Change::update(Entity::Member, OTHER_MEMBER, fields)])
        };
        let link = |device: &str, member: Option<&str>, more: serde_json::Value| {
            // The cloud vouches for what it links (D96): `member_id` goes with `vouched_key`.
            let mut fields = json!({ "member_id": member, "vouched_key": member.map(|_| "k") });
            fields
                .as_object_mut()
                .unwrap()
                .extend(more.as_object().unwrap().clone());
            op(vec![Change::update(Entity::Device, device, fields)])
        };
        let set = account(json!({ "account_id": ACCOUNT }));
        let nothing = json!({});

        assert!(is_allowed(&set, &cloud()));
        assert!(is_allowed(
            &link(other, Some(MY_MEMBER), nothing.clone()),
            &cloud()
        ));
        assert!(
            !is_allowed(&link(other, Some(OTHER_MEMBER), nothing.clone()), &cloud()),
            "a person without an account"
        );
        assert!(
            !is_allowed(&link(other, None, nothing.clone()), &cloud()),
            "taking the person away"
        );
        let alone =
            |fields: serde_json::Value| op(vec![Change::update(Entity::Device, other, fields)]);
        assert!(
            !is_allowed(&alone(json!({ "member_id": MY_MEMBER })), &cloud()),
            "member_id alone"
        );
        assert!(
            !is_allowed(&alone(json!({ "vouched_key": "k" })), &cloud()),
            "vouched_key alone"
        );
        assert!(
            !is_allowed(
                &alone(json!({ "member_id": MY_MEMBER, "vouched_key": "" })),
                &cloud()
            ),
            "an empty key vouches for nothing"
        );
        assert!(
            !is_allowed(
                &link(other, Some(MY_MEMBER), json!({ "trusted_through": "" })),
                &cloud()
            ),
            "beside a fence"
        );
        assert!(
            !is_allowed(&link(ME, Some(MY_MEMBER), nothing), &cloud()),
            "its own device"
        );
        assert!(!is_allowed(&account(json!({ "role": "admin" })), &cloud()));
        assert!(
            !is_allowed(
                &account(json!({ "account_id": ACCOUNT, "name": "Laura" })),
                &cloud()
            ),
            "account_id and nothing else"
        );
        assert!(!is_allowed(
            &op(vec![Change::create(
                Entity::Tag,
                TAG,
                json!({ "name": "Tehanu" })
            )]),
            &cloud()
        ));
        // Not the cloud device: a reader, and not even an administrator.
        assert!(!is_allowed(&set, &view(Some(Role::Reader))));
        assert!(!is_allowed(&set, &view(Some(Role::Admin))));
        let born = op(vec![Change::create(
            Entity::Member,
            OTHER_MEMBER,
            json!({ "name": "Laura", "account_id": ACCOUNT }),
        )]);
        assert!(
            !is_allowed(&born, &view(Some(Role::Admin))),
            "nor at creation"
        );
        // Nor in a library nobody founded.
        assert!(!is_allowed(&set, &unfounded()));
    }

    #[test]
    fn a_fenced_cloud_device_has_no_power() {
        let set = op(vec![Change::update(
            Entity::Member,
            OTHER_MEMBER,
            json!({ "account_id": ACCOUNT }),
        )]);
        assert!(is_allowed(&set, &cloud()));
        let fenced = View {
            fenced: true,
            ..cloud()
        };
        assert!(!is_allowed(&set, &fenced));
    }

    #[test]
    fn a_device_changes_only_its_own_row() {
        let rename = |id: &str| {
            op(vec![Change::update(
                Entity::Device,
                id,
                json!({ "name": "Tablet" }),
            )])
        };
        let other = "01997c7e-0000-7000-8000-0000000000fc";
        assert!(is_allowed(&rename(ME), &view(Some(Role::Editor))));
        assert!(!is_allowed(&rename(other), &view(Some(Role::Editor))));
        assert!(!is_allowed(&rename(other), &view(Some(Role::Reader))));
        assert!(is_allowed(&rename(other), &view(Some(Role::Admin))));
    }
}
