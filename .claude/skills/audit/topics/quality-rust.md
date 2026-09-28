---
agent: audit-reviewer
code:
  - @longest-rust:6
context:
  - AGENTS.md#Code conventions
  - AGENTS.md#Glossary
  - AGENTS.md#Teaching mode for Rust and Tauri
decisions: D02
max_lines: 2000
---
Quality of the longest Rust functions. The maintainer is an experienced PHP and TypeScript developer learning Rust: every finding carries an extra line `- **Por qué (Rust):** …` with 2 to 4 sentences on the Rust concept behind it (ownership, borrowing, lifetimes, `Result` and `?`, traits, async, `Send`/`Sync`), compared with PHP or TypeScript when that helps.

1. Domain rules (validation, permissions, merge) written in the Tauri core instead of `crates/domain`, or duplicated in both.
2. Responsibilities mixed in one function; blocks that should be extracted; nesting deeper than four levels.
3. Errors: swallowed errors, English text sent to the UI instead of codes, lost context.
4. Idioms worth teaching: needless `.clone()`, `&String` or `&Vec<T>` parameters, manual index loops, `match` that should be `?` or `if let`, `is_some()` followed by `unwrap()`. At most five, the most instructive.
5. Names that break the glossary; comments that restate or contradict the code.
