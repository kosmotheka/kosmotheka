//! Who may do what in a library (D55, D73). Pure rules: the caller answers the
//! questions in `LibraryView` from the library as it stood when the operation was
//! made, so every device and the server reach the same verdict.

use crate::operation::{Action, Change, Operation};
use crate::schema::Entity;

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
    /// Whether any member has a role. Until then, everyone may do everything.
    fn has_roles(&self) -> bool;
    /// The role of the person `device` belongs to. `None` when the device belongs
    /// to nobody or its person has no role.
    fn role_of_device(&self, device: &str) -> Option<Role>;
    /// Whether the row was created by a device of the same person as `device` (D45).
    fn created_by_same_person(&self, entity: Entity, id: &str, device: &str) -> bool;
    /// Whether the operation `request` was written by a device of the same person
    /// as `device`: who asked may withdraw what they asked for.
    fn requested_by_same_person(&self, request: &str, device: &str) -> bool;
    /// The person (member id) `device` belongs to, if any.
    fn member_of_device(&self, device: &str) -> Option<String>;
    /// Whose the mark `id` is, if it exists (D68).
    fn mark_member(&self, id: &str) -> Option<String>;
    /// Whether `operation` falls under a honoured fence of its author (D84, D87):
    /// dated before the operation that took the author's power away, and neither
    /// the last one the administrator had nor one of its ancestors through
    /// `previous`. Such an operation cannot be honest.
    /// Also when it is dated before an applied seal (D94) that does not name its author:
    /// a seal fences the device of its own author, so it fences every device, and the
    /// ones it does not name have no operation it believes before it.
    /// Also after a revocation of the author still in force at its HLC (D71): a
    /// revoked device writes nothing, even through a device that has not heard of it yet.
    fn fenced(&self, operation: &Operation) -> bool;
    /// The device `library.cloud_device_id` names now (D76, D95), if any.
    fn cloud_device(&self) -> Option<String>;
    /// Whether the person `member` exists.
    fn member_exists(&self, member: &str) -> bool;
    /// Whether the person `member` has an `account_id`.
    fn member_has_account(&self, member: &str) -> bool;
}

/// Whether the author of `operation` may make all of its changes. All or nothing:
/// one change not allowed makes the whole operation a request.
pub fn is_allowed(operation: &Operation, view: &impl LibraryView) -> bool {
    // Every operation is signed (D56): without a signature nobody can tell who wrote it.
    if operation.signature.is_none() {
        return false;
    }
    let author = operation.hlc.device.as_str();
    // A fenced author is judged as nobody's, with or without roles: its date cannot
    // be trusted, or its access was withdrawn (D71, D84).
    let fenced = view.fenced(operation);
    if !view.has_roles() && !fenced {
        return true;
    }
    let role = if fenced {
        None
    } else {
        view.role_of_device(author)
    };
    // The cloud's power (D76, D95) is the named device's, unless a fence or a seal
    // makes it nobody's. Asked only when a change could use it.
    let cloud = !fenced
        && operation.changes.iter().any(touches_cloud_field)
        && view
            .cloud_device()
            .is_some_and(|cloud| cloud.eq_ignore_ascii_case(author));
    operation
        .changes
        .iter()
        .all(|change| change_allowed(change, author, role, cloud, view))
}

/// Whether the change writes something only the cloud device may write, or the one
/// thing it may do beyond a device of nobody: `member.account_id` or `device.member_id`.
fn touches_cloud_field(change: &Change) -> bool {
    match change.entity {
        Entity::Member => change.fields.contains_key("account_id"),
        Entity::Device => change.fields.contains_key("member_id"),
        _ => false,
    }
}

/// The two changes the cloud device may make besides what a device of nobody may do
/// (D76): `account_id` of a person who exists, and the person of another device that
/// belongs to nobody, pointing at a person with an account. Each touches that field only.
fn cloud_allowed(change: &Change, author: &str, view: &impl LibraryView) -> bool {
    if change.action != Action::Update || change.fields.len() != 1 {
        return false;
    }
    match change.entity {
        Entity::Member => {
            change.fields.contains_key("account_id") && view.member_exists(&change.id)
        }
        Entity::Device => {
            !change.id.eq_ignore_ascii_case(author)
                && view.member_of_device(&change.id).is_none()
                && change
                    .fields
                    .get("member_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|member| view.member_has_account(member))
        }
        _ => false,
    }
}

fn change_allowed(
    change: &Change,
    author: &str,
    role: Option<Role>,
    cloud: bool,
    view: &impl LibraryView,
) -> bool {
    // `account_id` is written by the cloud device alone: not by an administrator, not
    // when the member is created.
    if change.entity == Entity::Member && change.fields.contains_key("account_id") {
        return cloud && cloud_allowed(change, author, view);
    }
    if cloud && cloud_allowed(change, author, view) {
        return true;
    }
    let someone = role.is_some();
    let role = role.unwrap_or(Role::Reader);
    if role == Role::Admin {
        return true;
    }
    let touches = |field: &str| change.fields.contains_key(field);
    match change.entity {
        // Every device registers and renames itself; saying whose it is, or
        // touching another device, or its fence, is for administrators.
        Entity::Device if change.id == author => {
            change.action != Action::Delete && !touches("member_id") && !touches("trusted_through")
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
        Entity::Member if touches("role") || touches("can_share") => false,
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
            (_, Action::Create | Action::Update) => true,
            (_, Action::Delete) => view.created_by_same_person(change.entity, &change.id, author),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct View {
        roles: bool,
        role: Option<Role>,
        same_person: bool,
        fenced: bool,
        cloud: bool,
    }

    impl LibraryView for View {
        fn has_roles(&self) -> bool {
            self.roles
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
            roles: true,
            role,
            same_person: false,
            fenced: false,
            cloud: false,
        }
    }

    const TAG: &str = "01997c7e-0000-7000-8000-000000000061";

    #[test]
    fn roles_decide_only_once_the_library_has_them() {
        let delete = op(vec![Change::delete(Entity::Tag, TAG)]);
        let none = View {
            roles: false,
            role: None,
            same_person: false,
            fenced: false,
            cloud: false,
        };
        assert!(is_allowed(&delete, &none));
        assert!(!is_allowed(&delete, &view(Some(Role::Reader))));
        assert!(!is_allowed(&delete, &view(Some(Role::Editor))));
        assert!(is_allowed(&delete, &view(Some(Role::Admin))));
    }

    #[test]
    fn an_editor_deletes_what_her_person_created() {
        let delete = op(vec![Change::delete(Entity::Tag, TAG)]);
        let mine = View {
            same_person: true,
            ..view(Some(Role::Editor))
        };
        assert!(is_allowed(&delete, &mine));
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
    fn an_unsigned_operation_is_never_allowed() {
        let mut add = op(vec![Change::create(
            Entity::Tag,
            TAG,
            json!({ "name": "Tehanu" }),
        )]);
        assert!(is_allowed(&add, &view(Some(Role::Editor))));
        add.signature = None;
        assert!(!is_allowed(&add, &view(Some(Role::Admin))));
        assert!(!is_allowed(&add, &view(None)), "not even without roles");
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
            let mut fields = json!({ "member_id": member });
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
        // Without roles everyone may.
        let none = View {
            roles: false,
            ..view(None)
        };
        assert!(is_allowed(&set, &none));
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
