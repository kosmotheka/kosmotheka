---
agent: audit-reviewer
code:
  - app/src/lib/api.ts
  - @longest-ts:2
context:
  - AGENTS.md#Code conventions
  - AGENTS.md#Glossary
decisions: D15 D17
max_lines: 2400
---
Quality of the TypeScript bridge to Rust and of the two largest components.

1. `api.ts`: every command has an explicit return type; no `any` or unchecked casts on returned data; argument names match what Rust expects (the contracts facts already compare them mechanically).
2. Domain logic in React (AGENTS.md): permission decisions from the role, validation that exists only in TypeScript, rules that must match Rust.
3. Effects: asynchronous work without protection against out-of-order results or unmount; listeners without cleanup.
4. Structure: components that mix data loading, state and presentation; duplicated blocks; props passed through many levels.
5. i18n misuse: translations joined by concatenation, plurals by hand, dates and numbers formatted without the current language.
