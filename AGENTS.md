# AGENTS.md

Instructions for AI coding agents working in this repository. Human contributors will find the same conventions in `CONTRIBUTING.md` once it exists.

## Project

A shared catalog for a household library (books first; board games, comics, video games, films later): what we own, where it is, who has read it and who borrowed it. The product is called **Kosmotheka** (D70). In the code, `library` is not the product: it is the domain entity, a library that people share (see the glossary). Never rename it along with the product.

- **Local-first**: the app always reads and writes its own SQLite database and works fully without a server or an account.
- **Optional authoritative server**: in connected mode, the client sends *operations* (atomic batches of entity changes) to the server, which checks permissions and re-applies them and is the source of truth.
- **Local sync**: devices pair with a QR code and sync over the LAN.

The full rationale lives in `docs/` (in Spanish):
- `docs/VISION.md` — original vision (historical).
- `docs/DECISIONS.md` — architecture decision log (D01, D02…), each with a status. **Read the relevant decisions before proposing or changing architecture.**
- `docs/ROADMAP.md` — execution phases.
- `docs/research/` — research notes.
- `docs/plans/` — plans for the phase in progress, while they are active.
- `docs/historico/` — superseded plans and debates. Never edited; they explain where the current decisions come from.

## Stack

| Part | Technology |
|---|---|
| Desktop and Android app | Tauri 2, Rust |
| App UI | React, TypeScript, Vite, shadcn/ui, Tailwind, react-i18next |
| Client storage | SQLite via `rusqlite` (`bundled`) and `rusqlite_migration` |
| Server | PHP 8.5, Symfony 8, Doctrine, PostgreSQL 18, Docker |
| Operation spec | JSON Schema plus shared conformance cases |

## Repository layout

```
Cargo.toml      Cargo workspace root
app/            Tauri app: React UI in src/, Rust core in src-tauri/
  src-tauri/migrations/   SQLite migrations, applied on startup
  src-tauri/gen/android/  Generated Android project (committed; edit with care)
crates/domain/  Domain types, validation, HLC and operations; no storage or UI code
spec/           Operation spec: JSON Schema and conformance cases (MIT licence)
server/         Symfony sync server (PHP), with its own Docker setup in server/docker/
docs/           Working documentation (Spanish); see docs/architecture/
```

Each part keeps its own tooling: the app and crates build in the `library-dev` Distrobox container, the server runs in its own Docker Compose project (`server/docker/`). The only thing they share is `spec/`, mounted read-only into the server's PHP container. See `server/AGENTS.md` for server-specific instructions.

## Language

- **Everything in the repository is in English**: code, identifiers, comments, commit messages, README, issues, UI strings in the source language.
- **Exception: `docs/` is in Spanish** for now; it will be translated later.
- **Talk to the maintainer in Spanish.**

## Code conventions

- Clean, readable code. Comments are short and explain *why* when it is not obvious; never restate what the code does.
- **Domain logic lives in Rust** (`crates/domain`), never in the React UI. The UI calls Tauri commands; it never runs SQL.
- Every write is an **operation** (docs/architecture/operations.md): build a list of `Change`s and call `ops::record` inside a transaction. Never `INSERT`/`UPDATE`/`DELETE` catalog tables directly. The only exception is `sync::merge`, which applies operations received from other devices (docs/architecture/sync.md).
- Sync must converge: any change to how operations apply needs a test in `sync.rs` that simulates devices and uses `assert_converged`.
- **Who receives a library is tested against a hostile peer first.** Any rule about which device or account gets, keeps or loses a library's data (pairing, sharing, revocation, the cloud's access table) needs a test of the hostile case, written before the fix and seen failing: a modified app, a peer that knows the ids, a forged notice. The list lives in `docs/plans/fase-7.md` («Batería de acceso»); the audit is the second line, not the first.
- **The server never trusts an id from the client** (D84): library, operation and device ids in a request are compared, never used to look up or decide access. Access comes only from the account, the access table and the server's own `cloud_library_id`.
- Adding a field or entity means changing, together: a migration, `crates/domain/src/schema.rs`, `spec/operation.schema.json`, and conformance cases in `spec/conformance/`.
- Every rule for an operation must be defined in `spec/` and covered by conformance cases that both the Rust and PHP implementations run.
- Adding a field or entity also means updating `server/src/Enum/CatalogEntity.php`, the PHP mirror of `schema.rs`.
- External services (metadata providers, AI) sit behind swappable interfaces and are **always optional**. The app must keep working when they are unavailable.
- Never hold the database `MutexGuard` across an `.await`: read what you need in a block, drop the guard, then await (see `commands::lookup_isbn`).
- Unit tests never touch the network: parse recorded responses in `app/src-tauri/tests/fixtures/`. Tests that need the network are `#[ignore]` and named `live_*`. The only other ignored tests are fixture generators, named `print_*`, run by hand when a format changes.
- **No user-facing string is hard-coded**: use i18n keys and add both `en` and `es` translations.
- **Never commit secrets** (API keys, passwords, tokens). The repository will become public, so its history must be clean.

## Glossary

The docs are in Spanish and the code in English. **Check this table before naming anything in the domain**, in Rust, PHP or TypeScript: a term that becomes `copy` in one file and `item` in another never leaves the repository. If a term is missing, add it here before using it.

| Spanish (docs) | Code | Notes |
|---|---|---|
| obra | `work` | The book in the abstract |
| edición | `edition` | A published edition of a work |
| ejemplar | `copy` | What is on the shelf: format, owner, location |
| persona (autor) | `person` | Someone who contributed to a work |
| editorial | `publisher` | |
| género | `genre` | A key from the app's closed list per kind, not a row |
| etiqueta | `tag` | Free word of one library, linked to works |
| miembro de la familia | `member` | A catalog row that owns copies; **not** an account, though it may be linked to one (D46) |
| marca (de una persona sobre una obra) | `mark` | Status, rating and favorite of one person for one work (D68) |
| serie (agrupación de obras) | `series` | Not the TV kind, which is `tv_series` |
| obra en una serie | `series_entry` | |
| «necesita» | `work_relation` with `requires` | An expansion requires its base game |
| estado · valoración · favorito | `status` · `rating` · `favorite` | Fields of a `mark` |
| lo deseo (marca de una persona) | `wished` | Field of a `mark` (D61) |
| operación · cambio · intención | `operation` · `change` · `intent` | D25 |
| operación anterior (del mismo autor) | `previous` | Field of an operation: the id of its author's previous operation in that library, or null (D87) |
| lápida | `tombstone` | Marks a deleted row so it cannot come back |
| reloj lógico híbrido | `hlc` | |
| dispositivo | `device` | |
| biblioteca | `library` | On the server, the unit that owns data (D38) |
| usuario (cuenta) | `user` | An account on the server |
| membresía | `membership` | On the server, links a user (account) to a `member` of a library (D76). The role is always the member's, never copied here. Keep it distinct from `member` |
| rol: administrador · editor · lector | `admin` · `editor` · `reader` | |
| invitación | `invitation` | |
| retirada de acceso | `revocation` | A device that may no longer receive a library (D71). Synced with the library |
| solicitud | `change_request` / `ChangeRequest` | Not `request`, which collides with the HTTP request (milestone 3.4) |

## Documentation is part of the work

A change is not finished until the docs affected by it are updated:
- An architecture choice → add or update an entry in `docs/DECISIONS.md` (see the `record-decision` skill).
- A phase item done → update `docs/ROADMAP.md`.
- Anything a user or self-hoster will need later (setup steps, configuration, behavior) → write it down now in Markdown, so it can become user and installation guides.

## Teaching mode for Rust and Tauri

Learning Rust and Tauri is a goal of this project. The maintainer is an experienced web developer (Symfony, PHP, TypeScript) and new to Rust.
- When writing Rust or Tauri code, explain the new concepts involved (ownership, borrowing, lifetimes, traits, error handling, async, Tauri commands and plugins), relating them to PHP or TypeScript equivalents when that helps.
- Put explanations in the conversation, **not in code comments**.
- Prefer simple, idiomatic Rust over clever abstractions.
- Do not add these explanations for Symfony or PHP.

## Commands

On the maintainer's machine, every toolchain command runs inside the `library-dev` Distrobox container (see `docs/development/setup.md`):

```bash
distrobox enter library-dev -- bash -ic '<command>'
```

The interactive flag (`-i`) is required: it loads the environment from `~/.bashrc`.

| Task | Command (from the repo root unless noted) |
|---|---|
| Run desktop app | `cd app && npm run tauri dev` |
| Run on Android device | `cd app && npm run tauri android dev` |
| Debug APK | `cd app && npm run tauri android build -- --debug --apk --target aarch64` |
| Rust format / lint / test | `cargo fmt --all` · `cargo clippy --all-targets -- -D warnings` · `cargo test` |
| UI build / typecheck / lint / format / dead code | `cd app && npm run build` · `npm run typecheck` · `npm run lint` · `npm run format` (check: `npm run format:check`) · `npm run knip` |
| Spec schema vs conformance cases | `cd app && npm run check:spec` |
| Rust dependencies (CI job `rust-deps`) | `cargo deny check` (rules in `deny.toml`) · `cargo machete` |
| Live tests against real services (network, not in CI) | `cargo test -p kosmotheka -- --ignored live_` |
| Second instance with its own data (debug builds), to test LAN sync | `KOSMOTHEKA_DATA_DIR=/tmp/kosmotheka-b target/debug/kosmotheka` (with `npm run tauri dev` running) |

Server commands run in Docker through the server's Makefile (`cd server && make help`): `make up`, `make check` (style, Rector, PHPStan, PHPUnit), `make fix`, `make migrate`, `make console C="…"`.

`app/dist` must exist before `cargo clippy` or `cargo test`, because Tauri embeds the built UI at compile time. Run `npm run build` first.
