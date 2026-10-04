//! Libraries: the unit that is shared, synced and permissioned (D54).

use crate::book::{ErrorCode, FieldError};

/// Long enough for "Rol de los jueves en casa de Marcos", short enough for a selector.
pub const MAX_NAME_CHARS: usize = 60;

/// A library name as typed: trimmed, not empty, not too long. Whether another
/// library on the device already uses it is checked against the database.
pub fn validate_library_name(raw: &str) -> Result<String, Vec<FieldError>> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(vec![FieldError::new("name", ErrorCode::Required)]);
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(vec![FieldError::new("name", ErrorCode::OutOfRange)]);
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_checked() {
        assert_eq!(validate_library_name("  Rol ").unwrap(), "Rol");
        assert_eq!(
            validate_library_name("   ").unwrap_err(),
            vec![FieldError::new("name", ErrorCode::Required)]
        );
        assert_eq!(
            validate_library_name(&"x".repeat(MAX_NAME_CHARS + 1)).unwrap_err(),
            vec![FieldError::new("name", ErrorCode::OutOfRange)]
        );
    }

    #[test]
    fn a_name_of_exactly_the_maximum_length_is_valid() {
        let name = "x".repeat(MAX_NAME_CHARS);
        assert_eq!(validate_library_name(&name).unwrap(), name);
    }
}
