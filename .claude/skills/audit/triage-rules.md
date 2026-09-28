## Instructions

A script found these candidates mechanically in the Kosmotheka code (a local-first household library app: Rust core and SQLite on each device, React UI in a Tauri webview, devices that sync over the LAN, an optional PHP server). Each candidate shows the matching line with a few lines around it. Decide for each one whether it is a real problem, using **only** the rules of its category below and the code shown. You have no tools besides reading this packet. If the excerpt is not enough to decide, put the candidate under "No concluyentes" with what is missing, in a few words.

Output, in Spanish, and nothing else:

```
# Triaje · <categories> · <commit>
## Hallazgos
| Candidatos | Dónde | Severidad | Qué falla (una frase) | Arreglo (una frase) |
|---|---|---|---|---|
One row per root cause; list every candidate number it covers, for example [3, 7].
## No son hallazgos
One line per reason with the candidate numbers, for example: "[2, 5, 9] el valor viene de una constante".
## No concluyentes
[n] what is missing, or —.
```

Every candidate number must appear exactly once in one of the three sections.

## rust.panic
`.unwrap()`, `.expect(`, `panic!` and similar outside tests. Where can the value come from?
- Data from another device, the server or the network (sync payloads, LAN requests, provider responses): `alta`, a buggy or malicious peer can crash the app.
- Local files, user input or the database: `media`.
- Provably infallible (a constant, a value checked just before, a regex literal, a lock that cannot be poisoned in practice): not a finding.

## rust.swallow
An error discarded with `let _ =`, `.ok();`, `unwrap_or_default()`, `Err(_) =>` or `map_err(|_| …)`.
- An error the user or the sync process must know about disappears (a failed backup, write, import or sync step): `alta` if the user can lose data without being told, `media` otherwise.
- Deliberate best effort (cleanup, closing, optional cache, a fallback that is correct by design): not a finding.

## rust.sql-format
SQL built with `format!`.
- A **value** (not a table or column name) is interpolated: `alta` if it can come from user input, a file or synced data; `media` otherwise.
- Only table or column names that come from constants or `schema.rs`: not a finding.

## rust.sql-write-other
A write to a table that is not a catalog table (`schema.rs` lists the catalog tables).
- Local device state (settings, known devices, pairing, sync cursors, search index, caches): not a finding.
- Data that other devices need, or that should be an operation according to AGENTS.md ("every write is an operation"): `alta`.

## rust.sql-write-catalog
A write to a catalog table outside `ops.rs` and `sync.rs`. AGENTS.md: every write goes through `ops::record`; only `sync::merge` may apply received operations directly. Always `alta`, unless the excerpt shows it is part of applying received operations.

## rust.lock-await
An async function that locks a mutex and awaits. AGENTS.md forbids holding the database `MutexGuard` across `.await`.
- The guard is still alive when an `.await` runs (not dropped, not confined to an inner block that ends before it): `alta`.
- The guard is dropped or scoped before every `.await`: not a finding.

## rust.cast
A narrowing integer cast (`as u32`, `as i32`…). A finding (`media`) only if real data can exceed the target type (sizes, counts, timestamps, lengths).

## rust.allow
`#[allow(dead_code)]` or `#[allow(unused…)]`. `baja` if the item really is unused; not a finding if the excerpt shows why it is needed.

## rust.unsafe
Explain what the block does. A finding only if it is unsound or unnecessary.

## rust.non-english
Code and comments must be in English (AGENTS.md). Spanish comment or identifier: `baja`. Text shown to users that is not an i18n key: `media`.

## ts.hardcoded-text · ts.hardcoded-attr
Text in JSX or in `placeholder`, `title`, `aria-label`, `alt`, `label`. AGENTS.md: no user-facing string is hard-coded; use i18n keys. A finding (`media`) if people see it. The brand name "Kosmotheka", CSS classes, keys, test ids and technical values are not findings.

## ts.ipc-outside-api
A Tauri `invoke` or core import outside `app/src/lib/api.ts`. `media`, unless the file is under `app/src/dev/`.

## ts.any
`any`, `as any`, `as unknown as`, `@ts-ignore`. `media` when it hides the type of data returned by a command or coming from outside; `baja` otherwise.

## ts.listen
A Tauri event listener. `media` if the unlisten function is not called when the component unmounts.

## ts.index-key
`key={index}` in a list. `baja` if the list can reorder or delete items; otherwise not a finding.

## ts.console
`console.log` or `console.debug` left in code: `baja`.

## ts.sink
- `dangerouslySetInnerHTML` or `innerHTML` with data that comes from metadata providers, other devices or imported files: `crítica` (in Tauri, script in the webview can call every command).
- `href={…}` or `src={…}` with such data and no check of the URL scheme (a `javascript:` URL): `alta`.
- URLs built by the app (`convertFileSrc`, constants, routes): not a finding.

## ts.role-check
A role or permission decided in TypeScript. AGENTS.md: domain logic lives in Rust. `media` if the UI decides what a person may do from the role itself instead of from a value computed in Rust (the rule can drift). Showing a role label is not a finding.

## ts.mock-ipc-import
The development IPC mock. `alta` if it can load in a production build (not behind `import.meta.env.DEV` or an equivalent flag).

## ts.effect
A `useEffect`. A finding only if it runs asynchronous work without protection against results arriving out of order or after unmount (no cancelled flag, `AbortController` or request id) **and** the person could then see or act on the wrong data (another book, another library): `media`, or `alta` if they could edit the wrong item.

## ts.non-english
Spanish in code or comments: `baja`. Text shown to users outside the i18n files: `media`. Sample data in `app/src/dev/`: not a finding.

## php.enum-from
`Enum::from()` raises an error on an unknown value. `media` if applied to request data (it becomes an HTTP 500 instead of a clear error); not a finding for internal constants.

## php.generic-exception
A generic PHP exception. `baja`, or `media` if the project has an error code for that case (`OperationErrorCode`, `SignatureErrorCode`).

## php.no-strict-types
A PHP file without `declare(strict_types=1)`: `baja`.

## php.empty-catch
An empty `catch`: `media`.

## semgrep
A static-analysis rule matched. Judge whether the rule's concern really applies to this code in this app, with the severity the concern deserves here. Tool configuration, examples and development-only files usually are not findings; say why.
