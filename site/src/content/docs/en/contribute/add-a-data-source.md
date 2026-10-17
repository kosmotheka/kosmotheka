---
title: Adding a data source
description: "How an external catalogue (books, films, games…) is added to Kosmotheka: the app's pattern, the server's, the tests, and what has to be updated outside the code."
sidebar:
  order: 15
---

A **data source** is an internet service that the app asks to fill in a record: the National Library, TMDB, RAWG… This page explains how to add a new one. Read it all before you start: the hard part is almost never the code.

Two rules govern everything that follows:

- **Sources are optional.** If one fails, the app says so and keeps working; it never blocks the user.
- **Tests never touch the network.** They read a recorded response. The ones that do need the network are ignored and named `live_*`.

## Before writing code: the provider's terms

Read the provider's terms of use **before** anything else. Some of them:

- **do not let you store their data** (not even in a cache), or only for a few days;
- **do not let you modify it** (if you trim it or mix it with other data);
- **require a link or a logo** on every page where their data is used;
- **forbid commercial use**, or limit the calls per hour or per day;
- **ask for a key issued to whoever uses it**, which cannot go in the app for everyone.

That decides how it is done. If the provider wants a key, each person puts in their own (or the server of their account puts it in). If it asks for a link, the app shows it (see `source-credits.tsx`). If it does not allow storing, nothing is stored. If the terms do not allow what you want, **it is not added**.

## In the app

Everything is in `app/src-tauri/src/metadata/`. Note: **there is no provider `trait`**. Each source is a module with plain functions, and `mod.rs` decides which one to call according to the type (`Kind`). That is deliberate: there are few of them, each answers differently, and a trait would force those differences to be hidden.

### 1. One module per source

Use `musicbrainz.rs` (no key, one service) or `rawg.rs` (with a key) as a model. Each module has:

- a function `search(client, …, query) -> Result<Vec<Candidate>, ProviderError>`, and `lookup_*` if the source answers to a code (ISBN, barcode);
- a **pure** function that turns the provider's answer into `Candidate`s (`parse_releases` in MusicBrainz). No network, no state: this is what gets tested;
- the service's address as a constant.

`Candidate` (`candidate.rs`) is the result the person can choose to fill in the form. It is built with `Candidate::new(kind, source, title)`, and the fields the source knows are filled in; the names match those of the domain's `EditionInput`. **A field the provider does not give is left empty**, it is not invented.

Errors are `ProviderError`: network, quota used up (`Quota`), key rejected (`BadKey`) and unexpected answer (`Unexpected`). If the source uses a key, use `get_json` (in `mod.rs`): it reads with a size limit, turns 401 and 403 into `BadKey` and 429 into `Quota`, and **never lets the address out in any error**, because the key can travel in it.

### 2. Registering it

- `Source` in `metadata/mod.rs`: a new variant, in `Source::ALL` and in `id()` (the id in `snake_case`, which is what the settings store and what `whats-new.json` uses).
- In `Lookup::search` (or `run`, or `lookup_code`) of `mod.rs`: add the call to the matching type, next to the others, **with `maybe_vec(disabled.contains(…id()), …)`**: that way a switched-off source is not called. The order is decided in `collect_candidates`: the first source is the one shown at the top.
- In `candidate_from_lookup` (the `match` is exhaustive): the compiler requires it.
- `app/src/lib/api.ts`: the `Source` type (the union of ids).

### 3. Settings › Data providers

The screen is `app/src/components/providers-dialog.tsx`:

- `SOURCES_BY_KIND`: the types in which the source appears.
- If it has its own key: `KEYED_SOURCES`, `GET_KEY_URL` (where to ask for it), `KEYED_PROVIDERS` in `settings.rs` (Rust) and a key check in `testSource` (a search you know returns a result).
- If the account's server can ask for it when there is no key: `ACCOUNT_SOURCES`, and its field in `Offers`.
- If **only** the server can ask for it, with no key of its own (like BoardGameGeek): `ACCOUNT_ONLY_SOURCES`.
- If it asks for a link: `SOURCES_WITH_A_LINK` in `source-credits.tsx`.

The switch appears by itself: the list of switched-off sources (`disabled_sources`) stores the ids.

Texts, in `es.json` and `en.json`, **both**: the name in `lookup.sources`, the description in `providers.sourceDescriptions`, what it contributes to each type, and the notice or link if the provider requires them. No text is written in the code (see [adding a language](../add-a-language/)).

### 4. Tests

- **Recorded responses** in `app/src-tauri/tests/fixtures/`: one hit, one «not found» and one search (`musicbrainz-search-random-access-memories.json`). They are read with `include_str!("../../tests/fixtures/…")` in a `mod tests` inside the module itself. Record the real response once and trim it: do not invent one.
- **`live_*` tests**, with `#[ignore = "needs network"]`: they call the real service. They are run by hand with `cargo test -p kosmotheka -- --ignored live_`. They do not run in continuous integration.
- If you need a fixture generator, it is called `print_*`, is ignored and is run by hand when the format changes.

## If the source goes through the server

A source that needs a key can be asked **through the account's server**: that way each person does not have to supply the key. Comic Vine (`server/src/Metadata/ComicVine*.php`) is the model. Each source has:

| Piece | What it is for |
|---|---|
| `*Source` | An interface: `isConfigured()` (is there a key?) and `search()`. The tests use a fake one and never reach the network |
| `*HttpSource` | The real call. If the key is only accepted at its address, everything is arranged so that it does not leave there: a client without a log (`UnloggedClientFactory`), errors with fixed text and without the original exception, and answers read with a size limit and field by field |
| `*Search` | What the controller uses: it checks the cache, if not there it asks the source, and it stores the result |
| `ProviderGate` | Goes through all the calls: if the provider says «quota used up», the server leaves it alone for 30 minutes; if it says «broken», it retries up to three times |
| `*Hit` and `*Detail` | What comes from the source, already trimmed and checked |

And in the rest of the server:

- **The cache** (`MetadataCacheEntry`) stores the answer and its time limit, **without who asked or the text**: the search is stored under a fingerprint (`fingerprint()`) that does not allow the text to be recovered. A failure is never stored. Give the source its time limit in `SEARCH_TTLS` and, if it has detail by id, its prefix in `DETAIL_PREFIXES`.
- **The limits** (`config/packages/rate_limiter.yaml`): the one for the whole server (a little below what the provider allows) and the one for each account (a tenth of it, so that one account does not use up everyone's).
- **The route**, in `MetadataController`: a `POST` (the searched text never goes in an address), with its `Dto`, the `metadata_account` limit and `Cache-Control: no-store`. If the source is not configured, `metadataUnavailable`.
- **`GET /v1/server`** (`ServerController`) adds the source's name to `metadata` when there is a key. It is what the app checks to know what this server offers.
- **The key**: a new environment variable (`COMIC_VINE_API_KEY`), empty by default, in `.env`, `services.yaml` (the HTTP client, and in the test environment a fake one) and in [the configuration](/en/run-your-server/configuration/#search-services).
- **Never in a log**: neither the key, nor the text, nor who searched for it. The `/v1/metadata/` addresses do not go into nginx's access log either.

In the app, the route is called through the `trait CloudApi` (`cloud/mod.rs`): a new method (`comic_vine_search`), its real implementation in `cloud/client.rs`, the fake one in `cloud/tests.rs`, and a `search_*` function and an answer reader (`parse_*_answer`) in `metadata/via_server.rs`. That reader **does not trust what the server answers**: it cuts and checks every field.

Also copy the tests of the most similar source (`comic_vine_tests.rs` in the app, `ComicVine*Test` on the server): they cover a server that does not offer the source, quota used up, a text that must not leave, and a badly formed answer.

## Outside the code

- **`app/src/whats-new.json`**: one line, in Spanish and in English, under `next`. Because the source changes **what leaves the device** (what you write goes to a service), the line carries `"switch": "source:<id>"`: the app shows the switch when it is updated. See [What's new in each version](/en/using-the-app/whats-new/) and [publishing a release](../release/).
- **The privacy policy** (`web/es/privacidad.html` and `web/en/…`): what is sent, to whom, what the server stores and for how long. If the source goes through the server, also the list of what the cache stores.
- **The guide «Where the data comes from»** (`using-the-app/data-sources.md`, and its Spanish version) and the one on [what leaves your device](/en/using-the-app/what-leaves-your-device/).
- **The provider's terms** that require something (a link, a logo, a sentence): in the app and in those guides.

## Checklist

1. The provider's terms read; what they ask for, done.
2. A module with `search`, a pure reader and tests with recorded responses; ignored `live_*` tests.
3. `Source`, `Source::ALL`, `id()`, `Lookup::search`, the type in `api.ts`, `SOURCES_BY_KIND`.
4. Texts in `es.json` and `en.json`.
5. If it goes through the server: the seven pieces above, the key, the environment variable and the documentation of the configuration.
6. `whats-new.json` with its `switch`, privacy policy and guides.
7. Checks: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run typecheck` and `npm run lint` in `app/`, and `make check` in `server/`.
