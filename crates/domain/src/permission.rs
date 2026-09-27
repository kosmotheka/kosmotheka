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
}

/// Whether the author of `operation` may make all of its changes. All or nothing:
/// one change not allowed makes the whole operation a request.
pub fn is_allowed(operation: &Operation, view: &impl LibraryView) -> bool {
    if !view.has_roles() {
        return true;
    }
    let author = operation.hlc.device.as_str();
    // Without a signature nobody can tell who wrote it: it counts as a reader's.
    let role = match operation.signature {
        Some(_) => view.role_of_device(author).unwrap_or(Role::Reader),
        None => Role::Reader,
    };
    operation
        .changes
        .iter()
        .all(|change| change_allowed(change, author, role, view))
}

fn change_allowed(change: &Change, author: &str, role: Role, view: &impl LibraryView) -> bool {
    if role == Role::Admin {
        return true;
    }
    let touches = |field: &str| change.fields.contains_key(field);
    match change.entity {
        // Every device registers and renames itself; saying whose it is, or
        // touching another device, is for administrators.
        Entity::Device if change.id == author => {
            change.action != Action::Delete && !touches("member_id")
        }
        Entity::Device | Entity::Library | Entity::Revocation => false,
        // Nobody gives more than they have.
        Entity::Member if touches("role") || touches("can_share") => false,
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
        }
    }

    const WISH: &str = "01997c7e-0000-7000-8000-000000000061";

    #[test]
    fn roles_decide_only_once_the_library_has_them() {
        let delete = op(vec![Change::delete(Entity::Wish, WISH)]);
        let none = View {
            roles: false,
            role: None,
            same_person: false,
        };
        assert!(is_allowed(&delete, &none));
        assert!(!is_allowed(&delete, &view(Some(Role::Reader))));
        assert!(!is_allowed(&delete, &view(Some(Role::Editor))));
        assert!(is_allowed(&delete, &view(Some(Role::Admin))));
    }

    #[test]
    fn an_editor_deletes_what_her_person_created() {
        let delete = op(vec![Change::delete(Entity::Wish, WISH)]);
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
    fn an_unsigned_operation_counts_as_a_readers() {
        let mut add = op(vec![Change::create(
            Entity::Wish,
            WISH,
            json!({ "title": "Tehanu" }),
        )]);
        assert!(is_allowed(&add, &view(Some(Role::Editor))));
        add.signature = None;
        assert!(!is_allowed(&add, &view(Some(Role::Editor))));
    }
}
