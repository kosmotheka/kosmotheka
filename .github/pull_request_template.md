<!--
Thank you for contributing! Before opening this pull request, read CONTRIBUTING.md.
For a big change, or one that alters how something behaves, open an issue first.

Title: `type(scope): summary`, e.g. `feat(app): read the ISBN printed on a back cover`,
`fix(server): refuse an invitation accepted twice`, `docs(guide): explain the shelf photo`.
Types: feat, fix, docs, refactor, test, perf, build, ci, chore.
Scopes: app, ui, domain, spec, server, site, web, ci.
Branch from `main`, named like the title: `feat/printed-isbn`.
Code, its tests and its documentation go together in this one pull request.
Open it as a draft while it is not ready. Tick what applies; delete the blocks that do not.
-->

## What and why

<!-- The problem it solves and how. "Closes #123" if there is an issue. -->

## How it was tested

<!-- Commands you ran, devices you tried it on (desktop OS, Android model),
     screenshots or a short video for anything on screen. -->

## Parts touched

- [ ] App, Rust core (`app/src-tauri/`)
- [ ] App, interface (`app/src/`)
- [ ] Domain (`crates/domain/`)
- [ ] Operation spec (`spec/`): the contract both Rust and PHP follow
- [ ] Server (`server/`)
- [ ] A migration (SQLite or server)
- [ ] Documentation site (`site/`) or website (`web/`)
- [ ] CI, build or dependencies

## Checklist

**Always**

- [ ] Each part I touched passes its checks (CONTRIBUTING.md, «Checks your change must pass»)
- [ ] Tests for what changed; a bug fix comes with the test that failed before it
- [ ] No keys, passwords, tokens or personal data, in the code or in the history
- [ ] The documentation it affects is updated (`docs/`, the guide, `DECISIONS.md` for architecture)
- [ ] It does **not** need the app and the server updated at the same time
      (if it does, say so above: the release has to go together)

**App**

- [ ] No hard-coded text: keys in `en.json` and `es.json`
- [ ] People would notice it → a line in `app/src/whats-new.json` under `next`, in Spanish and English
- [ ] It changes what leaves the device → that line has its `switch`, and the privacy page
      and «What leaves your device» say so
- [ ] Domain logic stays in Rust, and every write is an operation (`ops::record`)
- [ ] Sync changed → a convergence test in `sync.rs` (`assert_converged`)
- [ ] Dependencies changed → `npm run licenses` and the licences list committed

**Operations, fields or entities**

- [ ] The rule is in `spec/` first, with conformance cases that Rust and PHP both run
- [ ] A new field or entity touches every place CONTRIBUTING.md lists (migrations,
      `schemaVersion`, `schema.rs`, `operation.schema.json`, `CatalogEntity.php`, the Doctrine class)

**Who receives a library's data** (pairing, sharing, revoking, the cloud's access table)

- [ ] The hostile case has its test, written first and seen failing,
      and is listed in `docs/architecture/bateria-de-acceso.md`
- [ ] The server decides access only from the account and its own tables, never from an id the client sends (D84)

**Server**

- [ ] `make check` passes (style, Rector, PHPStan, PHPUnit)
- [ ] A new endpoint has a functional test, rejections included

## AI assistance

<!-- Welcome, as long as you understand the change and answer for it (CONTRIBUTING.md). -->

- [ ] None
- [ ] An AI assistant helped with: …
