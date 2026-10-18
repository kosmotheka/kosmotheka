# Contributing to Kosmotheka

Thanks for wanting to help. This page is for someone who can program but does not know the project yet. If something here is wrong or unclear, tell us: that is a good first contribution.

By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## How to contribute

1. **Open an issue first** if the change is big or alters how something behaves, so we can agree on it before you spend time. For a small fix, go straight to the pull request.
2. **Fork** the repository and create a branch from `main`.
3. **Make your change** and run the checks below until they pass.
4. **Open a pull request** that explains *why* the change is needed, not only what it does.
5. **Go through the review.** We may ask for changes.

One pull request per topic. Write commit messages in English, in the imperative or descriptive style of the existing history.

## Set up your machine

**The app** (`app/`, `crates/`) needs:

- Rust (stable) and Node 22.
- On Linux, the system packages Tauri needs: `libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev` (the names on Debian and Ubuntu).
- For Android, a JDK 17, the Android SDK and NDK.

The maintainer works inside a Distrobox container, but you do not have to. [`docs/development/setup.md`](docs/development/setup.md) lists every step (in Spanish).

```bash
cd app
npm install
npm run tauri dev
```

**The server** (`server/`) needs only Docker. No PHP on your machine. See [`docs/development/server.md`](docs/development/server.md) (in Spanish).

```bash
cd server
make up       # PostgreSQL, PHP-FPM, nginx, Mailpit and Mercure; the API on http://localhost:8095
make help     # every command
```

## Checks your change must pass

The CI runs these. Run the ones that match what you touched.

**Rust.** `app/dist` must exist first, because Tauri embeds the built UI: run `npm run build` in `app/`.

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check      # licences and advisories of the dependencies
cargo machete         # dependencies declared but unused
```

**The interface**, from `app/`:

```bash
npm run build
npm run typecheck
npm run lint
npm run format:check
npm run knip                 # dead code
npm run check:whats-new
npm run check:licenses       # needs cargo; by package name (exact versions: check:licenses:exact, before a release)
```

**The operation spec**, from `app/`:

```bash
npm run check:spec           # schema against the conformance cases
npm run check:codes
npm run check:i18n           # every key in every language
```

**The server**, from `server/`:

```bash
make check    # code style, Rector, PHPStan and PHPUnit
make fix      # fixes the style problems it can
```

Tests never use the network. Tests against real services are marked `#[ignore]` and named `live_*`; run them by hand with `cargo test -p kosmotheka -- --ignored live_`.

If you change a dependency that ships in the app, `npm run licenses` regenerates the licences list. It needs Docker and the network.

## Improving the documentation

The public guide (docs.kosmotheka.com) is built with [Starlight](https://starlight.astro.build), a documentation theme for Astro, from the Markdown in `site/src/content/docs/`. It is static HTML: nothing of it runs on a server. To see it while you edit:

```bash
cd site
npm ci
npm run dev        # http://localhost:4321, reloads as you save
npm run build && npm run check:links   # what the CI runs
```

Write a page in Spanish first, then in English with the same path. `site/README.md` has the details. `docs/` is not the guide: it is the development diary, in Spanish.

## Rules that matter here

The full list is in [`AGENTS.md`](AGENTS.md). These are the ones most likely to affect your change.

- **Every change to the catalogue is an operation.** Devices exchange operations, not rows, and that is what makes sync converge. Never write to a catalogue table directly.
- **A new field or entity touches several places together:** a SQLite migration with `schemaVersion` raised in `spec/sync.json`, `crates/domain/src/schema.rs`, `spec/operation.schema.json` with its conformance cases, and on the server `CatalogEntity.php`, the Doctrine class in `src/Entity/Catalog/` and a migration. A new entity also goes into three lists of the app (`ORDER`, `CONTENTS`, `LIBRARY_CONTENTS_CHILDREN_FIRST`). Miss one and the Rust and PHP sides stop agreeing. The [contributor guide](https://docs.kosmotheka.com/es/contribute/how-to-contribute/) walks through it.
- **The rules live in `spec/`.** Each rule has conformance cases that both the Rust and the PHP implementation run, so they cannot drift apart.
- **Sync must converge.** A change to how operations apply needs a test that simulates several devices and checks they end up equal.
- **Who receives a library is tested against a hostile peer first.** For anything that decides which device or account gets, keeps or loses data (pairing, sharing, revoking access), write the test of the attack first and see it fail, then write the fix. Think of a modified app, a peer that knows the ids, a forged notice.
- **The server never trusts an id from the client.** Library, operation and device ids in a request are compared, never used to decide access. Access comes from the account and the server's own tables.
- **No text is hard-coded in the interface.** Use i18n keys and add both the English and the Spanish translation.
- **A change people would notice needs a line in `app/src/whats-new.json`**, in Spanish and English, written for people and not for programmers.
- **Never commit secrets:** keys, passwords, tokens.

## Language

Code, comments, commit messages and issues are in English. The documents in `docs/` are in Spanish for now and will be translated. A question in Spanish or English is welcome.

## AI assistants

[`AGENTS.md`](AGENTS.md) has the instructions for a coding assistant working in this repository. Contributions made with the help of an AI assistant are accepted if the person opening the pull request understands the change and answers for it.

## Security

Never report a vulnerability in a public issue. Follow [`SECURITY.md`](SECURITY.md).

## Licence of your contributions

What you contribute is released under the licence of the part it touches: AGPL-3.0-or-later for the app and the server, MIT for `spec/`. There is no contributor licence agreement to sign.
