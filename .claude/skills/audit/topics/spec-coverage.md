---
agent: audit-reviewer
code:
  - spec/operation.schema.json
  - server/src/Validator/ValidOperationValidator.php
context:
  - facts/contracts.md#Conformance cases
  - facts/contracts.md#Error codes
decisions: D25 D31 D32 D73
max_lines: 2000
---
AGENTS.md: every rule of an operation is defined in `spec/` and covered by conformance cases that both the Rust and the PHP implementations run.

1. List every rule of `operation.schema.json` (`required`, `enum`, `pattern`, `maxLength`, `minimum`, conditional `if/then`, `oneOf`). For each, is there a conformance case that must pass and one that must fail (see the case index in the facts)? A rule without a failing case is `media`.
2. Rules the PHP validator enforces that are not in the spec, or spec rules it does not enforce.
3. Conformance files not run by both Rust and PHP (facts): `alta`.
4. Error codes: codes a side produces that another side cannot map (facts).
