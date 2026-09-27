//! Marks: what one person notes about one work (D68). Status, date, rating and
//! favorite, each person their own.

use uuid::Uuid;

/// Fixed namespace for mark ids. Never change it: ids already written depend on it.
const MARK_NAMESPACE: Uuid = Uuid::from_u128(0x7b2f0c1e_4d8a_4f3e_9c61_2a5d8e0b4f17);

/// The id of `member`'s mark on `work`. It is derived instead of random, so two
/// devices of the same person marking the same work offline write the same row,
/// merged field by field.
pub fn mark_id(member: &str, work: &str) -> String {
    let name = format!("{}:{}", member.to_lowercase(), work.to_lowercase());
    Uuid::new_v5(&MARK_NAMESPACE, name.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mark_id_depends_only_on_its_person_and_work() {
        const HUGO: &str = "01997c7e-0000-7000-8000-000000000013";
        const DUNE: &str = "01997c7e-0000-7000-8000-0000000000d1";
        assert_eq!(mark_id(HUGO, DUNE), mark_id(&HUGO.to_uppercase(), DUNE));
        assert_ne!(mark_id(HUGO, DUNE), mark_id(DUNE, HUGO));
    }
}
