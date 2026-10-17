---
title: "How to contribute: style, tests and commits"
description: The procedure, the rules that affect a change most, and how to add a field or an entity step by step.
sidebar:
  order: 4
---

This page gathers what you need to know so that a change is accepted the first time. The full procedure, in English, is in [`CONTRIBUTING.md`](https://github.com/kosmotheka/kosmotheka/blob/main/CONTRIBUTING.md), and the full rules in [`AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/AGENTS.md) and [`server/AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/server/AGENTS.md). The whole of them is not repeated here: the reason behind the rules that cause the most trouble is explained. If anything here is wrong or unclear, tell us: it would be a good first contribution.

## The procedure

1. **Open an *issue* first** if the change is big or changes how something behaves, so that it is agreed before you spend time on it. For a small fix, go straight to the *pull request*.
2. **Make a *fork*** and a branch from `main`.
3. **Make the change** and run the checks until they pass ([the app](../setup-app/#the-checks), [the server](../setup-server/#the-checks)).
4. **Open the *pull request*** and explain **why** it is needed, not only what it does.
5. **Go through the review.** We may ask for changes.

One *pull request* per topic. Whoever takes part accepts the [code of conduct](https://github.com/kosmotheka/kosmotheka/blob/main/CODE_OF_CONDUCT.md). For a security problem **do not open a public issue**: follow [`SECURITY.md`](https://github.com/kosmotheka/kosmotheka/blob/main/SECURITY.md).

If you work with an AI assistant, the result is accepted if whoever opens the *pull request* understands the change and answers for it. `AGENTS.md` has the instructions for the assistant.

## Language, commits and licence

- **The whole repository is in English:** code, identifiers, comments, commit messages, README, issues and the interface texts in their original language. The exception is `docs/`, which is in Spanish for now. A question in Spanish or in English is welcome.
- **Commits:** in English, in the imperative or descriptive, like the history that already exists. They explain the why when it is not obvious.
- **Comments in the code:** short, and they say **why**, never repeat what the code does.
- **Licence of what you contribute:** the licence of the part you touch. AGPL-3.0 or later for the app and the server, and MIT for `spec/`. No agreement needs to be signed.

## The rules that affect a change most

Each one has its reason. Understanding it helps more than memorising it.

- **Every write to the catalogue is an operation.** Build a list of changes and call `ops::record` inside a transaction; **never** run `INSERT`, `UPDATE` or `DELETE` directly on the catalogue tables. *Why:* devices swap operations, not rows, and that is what makes sync converge. The only exception is `sync::merge`, which applies the ones that arrive from other devices.
- **Domain logic goes in Rust** (`crates/domain`), not in the interface. React calls Tauri commands and does not run SQL.
- **A new field or entity touches several places at once.** It is explained below, step by step. *Why:* there are two implementations (Rust and PHP) of one same format; if you forget one, they stop agreeing.
- **The rules of an operation are defined in `spec/`** and covered by **conformance cases** that both the Rust implementation and the PHP one run. A *conformance case* is an example operation with the result that **every** implementation must give. *Why:* it is what stops the two from drifting apart, and what lets someone write a compatible client or server. A change of rule goes first to `spec/`, with its cases, and then to both implementations.
- **Sync must converge.** A change in how operations are applied needs a test in `sync.rs` that simulates several devices and checks, with `assert_converged`, that they end up equal.
- **Who receives a library is tested first against a hostile peer.** For any rule that decides which device or account gets, keeps or loses a library's data (pairing, sharing, withdrawing access, the cloud's access table), write **before the fix** the test of the attack and **check that it fails**: a modified app, a peer that knows the ids, a forged notice. *Why:* a later review is the second line of defence, not the first.
- **The server never trusts an id from the client.** The library, operation and device ids in a request are **compared**, never used to look something up or to decide access. Access comes only from the account, the access table and the server's own data.
- **External services are optional and sit behind interfaces.** The app has to keep working when they are not there.
- **Tests do not use the network.** They parse recorded responses in `app/src-tauri/tests/fixtures/`. The ones that need the network are `#[ignore]` and named `live_*`.
- **In Rust, do not keep the database access open across an `.await`.** The database `MutexGuard` must not cross a wait: read what you need in a block, drop the guard and then wait (see `commands::lookup_isbn`).
- **No interface text is written by hand.** Use i18n keys and add the translation in English and in Spanish.
- **A change that a person would notice needs its line in `app/src/whats-new.json`.** One line in Spanish and one in English, under `next`, written for people (no internals or refactorings). If the change alters what leaves the device, it also carries a switch tied to the setting that controls it. `npm run check:whats-new` fails if the app's version has no entry.
- **No secrets in the repository:** no keys, passwords, tokens, not even as examples. The repository is public and its history has to stay clean.

### Server rules

In addition to the above, in `server/` (more in `server/AGENTS.md`):

- **If Symfony already has the mechanism, use Symfony's.** Request bodies as DTOs with `#[MapRequestPayload]`, request limits with `#[RateLimit]`, the user with `#[CurrentUser]`, the clock with `ClockInterface`.
- **A method returns its type or throws**: never `Foo|string` or a null value to signal a failure. And its own exceptions, not a bare `\RuntimeException`.
- **Each class goes in a folder that names its role** (`Controller/`, `Entity/`, `Dto/`, `Enum/`…). None at the root of `src/` except `Kernel.php`.
- **`make check` has to pass**, with PHPStan at the maximum level and no baseline file.

## Adding a field or an entity, step by step

It is the change that is made most often and where it is easiest to forget something. A new field (for example, one more piece of data for board games) goes through these places, **together, in the same change**:

1. **A SQLite migration** in `app/src-tauri/migrations/`: a file `NNNN_name.sql` with the next number after the last one, and its line `M::up(include_str!(…))` in `app/src-tauri/src/db.rs`. Bump `schemaVersion` in `spec/sync.json` too: a test checks that it matches the number of migrations, because two devices with different versions do not sync. The app applies migrations on startup, and to the databases of the users who already exist too. *Why:* nobody starts from zero; the schema has to be able to evolve over real data.
2. **`crates/domain/src/schema.rs`**: add the field to its entity's list of fields with `field("name", FieldType::…, required)`. This is where you declare what kind of data it is and whether it is required.
3. **`spec/operation.schema.json`**: the same field, in the JSON Schema. *Why:* it is what any other client or server sees, and what `npm run check:spec` validates.
4. **Conformance cases** in `spec/conformance/`: a valid operation with the new field, and the invalid ones that apply (wrong type, a required one set to `null`…), in `operation-validation.json`. If the field changes how operations are merged or permissions, also add a case in `merge.json` or in `permissions.json`.
5. **`server/src/Enum/CatalogEntity.php`**: the PHP mirror of `schema.rs`. The server's tests run the same conformance cases and report most of the mismatches.
6. **The Doctrine class of that entity**, in `server/src/Entity/Catalog/`, with its column, and **a server migration** (`make migration` and then `make migrate`; remove the comments it generates and fill in `getDescription()`). *Why:* the server stores the catalogue in PostgreSQL with Doctrine, in addition to describing it in `CatalogEntity`.

Then, what uses the field: read and write it in `app/src-tauri/src/catalog.rs` (or wherever it belongs), expose it in `app/src/lib/api.ts` and in the form, and the texts with i18n keys in `es.json` and `en.json`.

For a **new entity** (a new table), everything above plus:

- the new case in the `enum Entity` of `schema.rs` and in the server's `CatalogEntity`;
- its place in the app's entity lists, which have a fixed size in the code: the sync order (`ORDER` in `sync.rs`, parents before children), the backup order (`CONTENTS` in `backup.rs`) and the order for deleting a library (`LIBRARY_CONTENTS_CHILDREN_FIRST` in `ops.rs`, children before parents);
- a test in `sync.rs` that it converges between devices.

Check at the end with `cargo test`, `npm run check:spec` and `make check`.

## Before opening the *pull request*

- Run the checks that match what you touched.
- Go through the list: does it have a test? A what's-new line, if people will notice it? Interface texts with i18n in both languages? Anything in `docs/` that needs updating?
- If an **architecture decision** changes, note it in the [decision log](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md), with its status. A decision that is changed **in silence** is how, three months later, nobody knows what was decided or why.

The names that repeat in the code (work, copy, member…) are in the [glossary](../glossary/): look there before you choose a new name.
