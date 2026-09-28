---
agent: audit-reviewer
code:
context:
  - facts/docs.md
  - docs/testing/fase-5.md
max_lines: 1200
---
Docs against code. Use at most 8 follow-up commands in total.

1. From the roadmap facts, take the milestones marked done most recently. From the decisions index, pick the accepted decisions those milestones depend on (at most 8).
2. For each, check with one targeted `grep` that the behavior exists as decided. A contradiction between an accepted decision and the code is `alta` if it affects data or permissions, `media` otherwise.
3. Checks in `fase-5.md` whose expected result the code no longer produces.
