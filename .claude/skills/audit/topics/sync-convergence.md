---
agent: audit-reviewer-strong
code:
  - app/src-tauri/src/sync.rs::@prod
  - app/src-tauri/src/ops.rs::@prod
  - crates/domain/src/operation.rs::@prod
  - crates/domain/src/hlc.rs::@prod
  - crates/domain/src/library.rs::@prod
  - app/src-tauri/src/sync.rs::assert_converged*
context:
  - docs/architecture/sync.md
  - docs/architecture/operations.md
  - facts/contracts.md#Conformance cases
decisions: D05 D25 D31 D32 D33 D41 D46 D47 D54 D56 D65 D71
max_lines: 2800
---
Can honest devices lose data or diverge? Reason with concrete timelines: devices A, B, C, the operations each makes, the order they arrive, and the final state of each. A timeline that ends in different states, or a change silently lost, is a finding.

1. **Convergence** of each change type: the same final state whatever the delivery order and with duplicates; deterministic tie-break when two HLCs are equal; field-level merge and "delete wins" (D32); an operation that arrives before the one that created its row; a batch where one change fails (partial application?).
2. **HLC** (`hlc.rs` and its callers): monotonic after a restart (persisted or rebuilt from the clock?); wall clock going backwards; a peer with a clock far in the future: is there a maximum drift, or does one device from 2099 win every conflict forever?; counter overflow; ordering of serialized HLCs (string against numeric).
3. **Deletion**: delete against concurrent edit; delete then re-create; an old operation arriving late resurrecting a row; cascades (a work deleted on A while B adds a copy); orphans after merge; are tombstones ever collected?
4. **Atomicity**: operation log and catalog written in one transaction; received batches applied atomically or with a clear policy.
5. **Libraries** (D46, D54, D65, D71): moving a book between libraries seen by a device that receives only one of them; deleting or leaving a library while others edit it; `library-merge` conformance cases against the code.
6. **Version skew** (D31): a device on an older app version receives operations with fields or entities it does not know: rejected, dropped or stored? If dropped and it later edits the row, does it erase the newer field elsewhere?
7. **Tests**: `assert_converged` exists; for each merge rule, say which scenario (from points 1 to 3) has no test, as a proposed test.

Add a final section `## Propiedades comprobadas`: the guarantees you verified, each with the code that ensures it.
