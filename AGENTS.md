# AGENTS.md

Instructions for AI coding agents working in this repository. Human contributors will find the same conventions in `CONTRIBUTING.md` once it exists.

## Project

A shared catalog for a household library (books first; board games, comics, video games, films later): what we own, where it is, who has read it and who borrowed it. Working name: **library**.

- **Local-first**: the app always reads and writes its own SQLite database and works fully without a server or an account.
- **Optional authoritative server**: in connected mode, the client sends named *operations* to the server, which re-applies them with its own rules (permissions) and is the source of truth.
- **Local sync**: devices pair with a QR code and sync over the LAN.

The full rationale lives in `docs/` (in Spanish):
- `docs/VISION.md` — original vision (historical).
- `docs/DECISIONS.md` — architecture decision log (D01, D02…), each with a status. **Read the relevant decisions before proposing or changing architecture.**
- `docs/ROADMAP.md` — execution phases.
- `docs/research/` — research notes.

## Stack

| Part | Technology |
|---|---|
| Desktop and Android app | Tauri 2, Rust |
| App UI | React, TypeScript, Vite, shadcn/ui, Tailwind, react-i18next |
| Client storage | SQLite via `rusqlite` (`bundled`) and `rusqlite_migration` |
| Server | Symfony, PostgreSQL, Docker |
| Operation spec | JSON Schema plus shared conformance cases |

## Repository layout

```
Cargo.toml      Cargo workspace root
app/            Tauri app: React UI in src/, Rust core in src-tauri/
  src-tauri/migrations/   SQLite migrations, applied on startup
  src-tauri/gen/android/  Generated Android project (committed; edit with care)
docs/           Working documentation (Spanish)
```

Planned: `crates/domain/` (domain model and operations, independent of Tauri), `server/` (Symfony), `spec/` (operation spec and conformance cases, MIT).

Update this section when the layout changes.

## Language

- **Everything in the repository is in English**: code, identifiers, comments, commit messages, README, issues, UI strings in the source language.
- **Exception: `docs/` is in Spanish** for now; it will be translated later.
- **Talk to the maintainer in Spanish.**

## Code conventions

- Clean, readable code. Comments are short and explain *why* when it is not obvious; never restate what the code does.
- **Domain logic lives in Rust** (`crates/domain`), never in the React UI. The UI calls Tauri commands; it never runs SQL.
- Every write is an **operation**: UUID v7 id, device id and a hybrid logical clock (HLC) timestamp. There are no direct row updates that bypass the operation log.
- Every rule for an operation must be defined in `spec/` and covered by conformance cases that both the Rust and PHP implementations run.
- External services (metadata providers, AI) sit behind swappable interfaces and are **always optional**. The app must keep working when they are unavailable.
- **No user-facing string is hard-coded**: use i18n keys and add both `en` and `es` translations.
- **Never commit secrets** (API keys, passwords, tokens). The repository will become public, so its history must be clean.

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
| UI build / typecheck / lint | `cd app && npm run build` · `npm run typecheck` · `npm run lint` |

`app/dist` must exist before `cargo clippy` or `cargo test`, because Tauri embeds the built UI at compile time. Run `npm run build` first.
