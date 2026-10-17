---
title: Adding a language
description: "What has to be translated and changed so that the app, the server, the what's-new notes and the documentation of Kosmotheka are in a new language."
sidebar:
  order: 16
---

Today Kosmotheka is in **Spanish and English**. Adding a third language is possible, but there are more places than it seems: most are translated without touching code, and a few have "Spanish or English" written by hand. This page lists them all, so that you know the size of the job before you start. In the examples, the new language is French (`fr`).

Before you start, open an *issue* (see [how to contribute](../how-to-contribute/)): a language has to be **maintained** in every release, and it is better to know who will do it.

## The app

### The texts

All the interface texts are in `app/src/i18n/locales/`, one file per language:

- `en.json` and `es.json` today. Copy `en.json` as `fr.json` and translate the values, **never the keys**.
- `app/src/i18n/index.ts` is **the list of interface languages**: `import fr from "./locales/fr.json"`, `fr: { translation: fr }` in `resources`, `"fr"` in `supportedLanguages` and its name, **in its own language**, in `languageNames` (TypeScript warns you if one is missing). The language selectors (Settings, the account and sign-up), the what's-new notes, the privacy link and the language of the searches all read from there: nothing else in the interface needs touching, except the privacy-policy link if you have one in your language (`PRIVACY` in `about-dialog.tsx`; without it, the English one is used).

The app picks the system language (`i18next-browser-languagedetector`) and, if it does not have that one, falls back to English.

Things in the text that trip people up:

- **Plurals**: a key with `count` has `_one` and `_other` (`results_one`, `results_other`). Some languages need more forms; check the i18next rules.
- **Variables**: `{{title}}` and the like are left as they are.
- **Tags**: `<link>RAWG</link>` in the credit texts stays; only what is inside is translated.
- **No text in the code**: if a text is not in `en.json`, it cannot be translated. Add it to the languages that already exist.

### The closed lists

Some lists live in the domain (Rust) and their text in the files above:

| List | Where it is | Text keys |
|---|---|---|
| Genres of each type | `crates/domain/src/genre.rs` | `genres.*` |
| Reading, playing… statuses by type | domain | `marks.statuses.*`, `marks.statusesByKind.<type>.*` |
| Types of work, copy formats | domain | `types.*`, `typesPlural.*`, `formats.*` |
| Roles of people and request statuses | domain | `people.roles.*`, `requests.status.*` |
| Error codes | Rust and server | `accountErrors.*`, `cloud.errors.*`, `invitations.errors.*`… |

A genre or a status **is stored by its key** (`science_fiction`), not by its text: translating them changes no data and no synchronisation. But **every key in the list has to exist in your file**; if one is missing, the interface shows the raw key.

### What warns about missing keys

`npm run check:i18n` (in `app/`, and in continuous integration) fails when a key is in one language and not in another. It compares **every** file in `src/i18n/locales/`, so yours is included on its own. The only exception is `genresByKind`: one language may need to agree a genre with the kind (in Spanish, «Policiaca» for a film) and another may not, because the app then uses the general one (`genres.<key>`).

Besides, the audit (`.claude/skills/audit/bin/facts.py`) reports keys the code uses that do not exist, and closed lists (`contracts.json`) missing a text.

In addition, `npm run typecheck` and `npm run lint` have to keep passing, and it is worth opening the app in the new language and going through the screens.

### Where the list of each part is

Each part has **one list** of languages, and everything else reads it. To add a language you touch each part's list and nothing else:

| Part | Where the list is | What you add |
|---|---|---|
| The interface | `app/src/i18n/index.ts`: `supportedLanguages`, `languageNames` | The code and the language's name in itself, plus its `locales/<code>.json` |
| The privacy policy linked from «About» | `PRIVACY` in `app/src/components/about-dialog.tsx` | The policy's address in the new language. Optional: without it, the English one is shown |
| TMDB (films and series) | `TMDB_TAGS` in `app/src-tauri/src/metadata/mod.rs` and `tmdbTag()` in `server/src/Enum/Locale.php` | How TMDB writes that language (`fr` → `fr-FR`). Optional: without it, it is asked in `en-US` |
| The what's-new notes | The files in `app/src/i18n/locales/` | Nothing: `check:whats-new` requires text in every language that has a file |
| The server | `server/src/Enum/Locale.php` | A new case; the three forms validate against it |

The other search services ask in their usual language: for books and titles, the language of the edition does not depend on the app's.

## The what's-new notes (`whats-new.json`)

Each line of `app/src/whats-new.json` has the text of each language, by its code (`"es": …`, `"en": …`, and the new one). English is required and is the fallback: if a line has no text in the person's language, English is shown. `npm run check:whats-new` requires text in **every language that has a file** in `src/i18n/locales/`, so for a new language you have to **translate every line of every version** you want to be seen. Neither the script nor `whats_new.rs` needs touching.

## The server

Two things:

- **The list of languages (the account's `locale`)**: `server/src/Enum/Locale.php`, an enum with one case per language. `RegisterAccountRequest`, `UpdateAccountRequest` and `GoogleStartRequest` validate with `#[Assert\Choice(callback: [Locale::class, 'values'])]`, so adding the case is enough. The app sends its language when creating the account and when signing in with Google. If TMDB writes it differently, add its tag in `tmdbTag()` too.
- **The emails**: `server/translations/emails.es.yaml` and `emails.en.yaml`, with the subject and body of each (confirm the account, recover the password, the notice that someone tried to use your email, password changed, the second factor, the invitation…). An `emails.fr.yaml` with **all** the keys is needed. They are translations, not a list. The server's default language is `en` (`config/packages/translation.yaml`) and it is used when there is no translation.

The error texts the server returns are **codes** (`invalid_credentials`), not sentences: the app translates them with its own files. They do not need touching.

## The documentation and the website

These are separate and are translated when someone is there to maintain them:

- **The documentation** (`site/`): Starlight has the languages in `site/astro.config.mjs` (today `es` by default and `en`). A new language is a new folder in `site/src/content/docs/<language>/` with the **same folder structure** (the folder names are the same in every language, because the language selector links to the same page). What is not translated comes out in Spanish with a notice.
- **The website** (`web/`): one folder per language (`web/es/`, `web/en/`) with its pages, among them the **privacy policy** and the one on deleting the account.
- **The privacy policy** is best kept in the language of whoever uses the app: the link in the «About» dialog has to point to it.

## Checklist

1. An *issue* open, and who will maintain the language.
2. `locales/<code>.json` complete and registered in `i18n/index.ts` (`resources`, `supportedLanguages`, `languageNames`).
3. Genres, statuses, types, formats and roles with text; the app gone through in the new language.
4. Optional: the privacy policy in `PRIVACY` and the TMDB tag (`TMDB_TAGS`, `Locale::tmdbTag()`).
5. `whats-new.json`: text in the new language on every line.
6. Server: a case in `Locale` and `emails.<code>.yaml`.
7. Documentation and website, when possible.
8. `npm run typecheck`, `npm run lint`, `npm run check:i18n`, `npm run check:whats-new`, `cargo test` and `make check` (in `server/`).
