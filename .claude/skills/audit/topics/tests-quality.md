---
agent: audit-reviewer
code:
  - crates/domain/src/permission.rs::@tests
  - crates/domain/src/signature.rs::@tests
  - app/src-tauri/src/backup.rs::@tests
  - app/src-tauri/src/sync.rs::assert_converged*
context:
  - facts/tests.md
  - docs/testing/fase-5.md
  - facts/docs.md#Roadmap status
max_lines: 2400
---
QA: is what matters tested, and would the tests catch it breaking? Not coverage percentages.

1. From the test inventory (facts), which of these have no automatic test: convergence of each merge rule (concurrent edits, edit against delete, duplicates, different orders, three devices, clock ahead); each permission with an allowed **and** a denied case; retirada de acceso; signatures (valid, tampered field, wrong key); migrations on a database **with data**; backup and restore; CSV round trip; change requests; metadata parsing with empty or broken responses.
2. For the tests shown: can they fail? (assertions on a value the test just set, no assertion); only the happy path; hidden dependencies on real time, randomness without a seed, shared state or execution order.
3. AGENTS.md: tests that need the network are `#[ignore]` and named `live_*` (facts list the exceptions).
4. The UI has no automatic tests (facts): which UI logic would deserve them, if any.
5. Manual checklists: does `fase-5.md` cover the user-visible behavior of the milestones marked done, with concrete steps and expected results?

Every gap becomes a finding whose "Arreglo propuesto" is a concrete test: its name, the file where it goes, and Given / When / Then in one line each. If there are no property-based tests for sync, propose one (`proptest` is an option; do not add it).
