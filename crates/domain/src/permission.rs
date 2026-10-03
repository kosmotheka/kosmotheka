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
    /// Whether `hlc` falls inside a honoured antedating fence of `device` (D84):
    /// after the newest operation of the device the administrator knew, and before
    /// the operation that took its power away. Such an operation cannot be honest.
    /// Also after a revocation of `device` still in force at `hlc` (D71): a revoked
    /// device writes nothing, even through a device that has not heard of it yet.
    fn fenced(&self, device: &str, hlc: &str) -> bool;
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
    let fenced = view.fenced(author, &operation.hlc.to_string());
    if !view.has_roles() && !fenced {
        return true;
    }
    let role = if fenced {
        None
    } else {
        view.role_of_device(author)
    };
    operation
        .changes
        .iter()
        .all(|change| change_allowed(change, author, role, view))
}

fn change_allowed(
    change: &Change,
    author: &str,
    role: Option<Role>,
    view: &impl LibraryView,
) -> bool {
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
            change.action != Action::Delete && !touches("member_id") && !touches("trusted_until")
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
        fn fenced(&self, _: &str, _: &str) -> bool {
            self.fenced
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
            signature: Some("signed".into()),
        }
    }

    fn view(role: Option<Role>) -> View {
        View {
            roles: true,
            role,
            same_person: false,
            fenced: false,
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
            json!({ "trusted_until": "" }),
        )]);
        let other = op(vec![Change::update(
            Entity::Device,
            "01997c7e-0000-7000-8000-0000000000fc",
            json!({ "trusted_until": "" }),
        )]);
        assert!(!is_allowed(&own, &view(Some(Role::Editor))));
        assert!(!is_allowed(&other, &view(Some(Role::Editor))));
        assert!(is_allowed(&other, &view(Some(Role::Admin))));
        assert!(!is_allowed(&own, &view(None)));
    }
}
