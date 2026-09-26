//! Every write is an operation: an atomic batch of entity changes with an intent
//! label (D25). See docs/architecture/operations.md.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::hlc::Hlc;
use crate::schema::{Entity, FieldSpec, FieldType};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub id: String,
    /// The library every change belongs to (D54).
    pub library: String,
    pub hlc: Hlc,
    /// What the user meant, e.g. `book.add`. Informative only: permissions and
    /// merging look at the changes, never at the label.
    pub intent: String,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub entity: Entity,
    pub id: String,
    pub action: Action,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub fields: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationErrorCode {
    InvalidId,
    InvalidLibrary,
    InvalidIntent,
    NoChanges,
    OtherLibrary,
    UnknownField,
    MissingField,
    NullRequired,
    WrongType,
    EmptyUpdate,
    FieldsOnDelete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("invalid operation: {code:?} (change {change:?}, field {field:?})")]
pub struct OperationError {
    pub code: OperationErrorCode,
    /// Index of the offending change, if the problem is inside one.
    pub change: Option<usize>,
    pub field: Option<String>,
}

impl OperationError {
    fn new(code: OperationErrorCode) -> Self {
        Self {
            code,
            change: None,
            field: None,
        }
    }
}

impl Change {
    pub fn create(entity: Entity, id: impl Into<String>, fields: Value) -> Self {
        Self::with_fields(entity, id, Action::Create, fields)
    }

    pub fn update(entity: Entity, id: impl Into<String>, fields: Value) -> Self {
        Self::with_fields(entity, id, Action::Update, fields)
    }

    pub fn delete(entity: Entity, id: impl Into<String>) -> Self {
        Self {
            entity,
            id: id.into(),
            action: Action::Delete,
            fields: Map::new(),
        }
    }

    /// `fields` must be a JSON object; anything else becomes an empty set of fields.
    fn with_fields(entity: Entity, id: impl Into<String>, action: Action, fields: Value) -> Self {
        let fields = match fields {
            Value::Object(map) => map,
            _ => Map::new(),
        };
        Self {
            entity,
            id: id.into(),
            action,
            fields,
        }
    }

    /// `library` is the operation's own: the only library a change may touch.
    fn validate(&self, library: &str) -> Result<(), OperationError> {
        use OperationErrorCode::*;
        let fail = |code, field: Option<&str>| OperationError {
            code,
            change: None,
            field: field.map(str::to_owned),
        };

        if Uuid::parse_str(&self.id).is_err() {
            return Err(fail(InvalidId, None));
        }
        if self.entity == Entity::Library && self.id != library {
            return Err(fail(OtherLibrary, None));
        }

        match self.action {
            Action::Delete if !self.fields.is_empty() => return Err(fail(FieldsOnDelete, None)),
            Action::Update if self.fields.is_empty() => return Err(fail(EmptyUpdate, None)),
            _ => {}
        }

        for (name, value) in &self.fields {
            let spec = self
                .entity
                .field(name)
                .ok_or_else(|| fail(UnknownField, Some(name)))?;
            if value.is_null() {
                if spec.required {
                    return Err(fail(NullRequired, Some(name)));
                }
            } else if !matches_type(spec, value) {
                return Err(fail(WrongType, Some(name)));
            }
        }

        if self.action == Action::Create
            && let Some(missing) = self
                .entity
                .fields()
                .iter()
                .find(|spec| spec.required && !self.fields.contains_key(spec.name))
        {
            return Err(fail(MissingField, Some(missing.name)));
        }
        Ok(())
    }
}

fn matches_type(spec: &FieldSpec, value: &Value) -> bool {
    match spec.field_type {
        FieldType::Text => value.is_string(),
        FieldType::Integer => value.is_i64(),
        FieldType::Date => value
            .as_str()
            .is_some_and(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()),
        FieldType::OneOf(allowed) => value.as_str().is_some_and(|s| allowed.contains(&s)),
        FieldType::Ref(_) => value.as_str().is_some_and(|s| Uuid::parse_str(s).is_ok()),
    }
}

impl Operation {
    /// Structural checks shared by every client and the server. Whether referenced
    /// rows exist, or whether the author may make these changes, is checked when applying.
    pub fn validate(&self) -> Result<(), OperationError> {
        if Uuid::parse_str(&self.id).is_err() {
            return Err(OperationError::new(OperationErrorCode::InvalidId));
        }
        if Uuid::parse_str(&self.library).is_err() {
            return Err(OperationError::new(OperationErrorCode::InvalidLibrary));
        }
        if !is_valid_intent(&self.intent) {
            return Err(OperationError::new(OperationErrorCode::InvalidIntent));
        }
        if self.changes.is_empty() {
            return Err(OperationError::new(OperationErrorCode::NoChanges));
        }
        for (index, change) in self.changes.iter().enumerate() {
            change
                .validate(&self.library)
                .map_err(|error| OperationError {
                    change: Some(index),
                    ..error
                })?;
        }
        Ok(())
    }
}

/// Dot-separated lowercase words, such as `book.add` or `copy.lend`.
fn is_valid_intent(intent: &str) -> bool {
    let mut parts = intent.split('.');
    parts.clone().count() >= 2
        && parts.all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[derive(Deserialize)]
    struct Case {
        name: String,
        operation: Value,
        expect: Expect,
    }

    #[derive(Deserialize)]
    #[serde(tag = "result", rename_all = "snake_case")]
    enum Expect {
        Valid,
        Invalid {
            code: OperationErrorCode,
            change: Option<usize>,
            field: Option<String>,
        },
    }

    /// Shared with every other implementation of the spec (the PHP server included).
    const CASES: &str = include_str!("../../../spec/conformance/operation-validation.json");

    #[test]
    fn conformance_cases() {
        let cases: Vec<Case> = serde_json::from_str(CASES).expect("conformance file must parse");
        assert!(!cases.is_empty());

        for case in cases {
            let result = serde_json::from_value::<Operation>(case.operation)
                .map_err(|error| format!("does not deserialize: {error}"))
                .and_then(|op| op.validate().map_err(|error| format!("{error:?}")));

            match (case.expect, result) {
                (Expect::Valid, Ok(())) => {}
                (
                    Expect::Invalid {
                        code,
                        change,
                        field,
                    },
                    Err(actual),
                ) => {
                    let expected = format!(
                        "{:?}",
                        OperationError {
                            code,
                            change,
                            field
                        }
                    );
                    assert_eq!(actual, expected, "case: {}", case.name);
                }
                (expected, actual) => panic!(
                    "case '{}': expected {}, got {actual:?}",
                    case.name,
                    match expected {
                        Expect::Valid => "valid",
                        Expect::Invalid { .. } => "invalid",
                    }
                ),
            }
        }
    }

    #[test]
    fn serializes_to_the_documented_shape() {
        let op = Operation {
            id: "01997c7e-0000-7000-8000-000000000001".into(),
            library: "01997c7e-0000-7000-8000-00000000aaaa".into(),
            hlc: "0001727222222222-00000000-dev".parse().unwrap(),
            intent: "member.add".into(),
            changes: vec![Change::create(
                Entity::Member,
                "01997c7e-0000-7000-8000-000000000002",
                json!({ "name": "Tersoal" }),
            )],
        };

        assert_eq!(
            serde_json::to_value(&op).unwrap(),
            json!({
                "id": "01997c7e-0000-7000-8000-000000000001",
                "library": "01997c7e-0000-7000-8000-00000000aaaa",
                "hlc": "0001727222222222-00000000-dev",
                "intent": "member.add",
                "changes": [{
                    "entity": "member",
                    "id": "01997c7e-0000-7000-8000-000000000002",
                    "action": "create",
                    "fields": { "name": "Tersoal" }
                }]
            })
        );
    }
}
