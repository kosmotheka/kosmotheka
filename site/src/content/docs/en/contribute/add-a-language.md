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
- `app/src/i18n/index.ts` registers them: `import fr from "./locales/fr.json"`, `fr: { translation: fr }` in `resources`, and `"fr"` in `supportedLanguages`.

`supportedLanguages` is what the language selectors use (Settings, the account and sign-up), and each one needs the name of the language **in its own language**: `LANGUAGE_NAMES` in `settings-dialog.tsx` (TypeScript warns you if one is missing) and the selector in `account-dialog.tsx` and `auth-dialog.tsx`.

The app picks the system language (`i18next-browser-languagedetector`) and, if it does not have that one, falls back to English.

Things in the text that trip people up:

- **Plurals**: a key with `count` has `_one` and `_other` (`results_one`, `results_other`). Some languages need more forms; check the i18next rules.
- **Variables**: `{{title}}` and the like are left as they are.
- **Tags**: `<link>RAWG</link>` in the credit texts stays; only what is inside is translated.
- **No text in the code**: if a text is not in `en.json`, it cannot be translated. Add it to the two languages that already exist.

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

There is no test that fails in continuous integration. What exists is an audit check: `.claude/skills/audit/bin/facts.py` compares `en.json` and `es.json` and lists the keys that are in one and not in the other, the keys used in code that do not exist, and the closed lists (`contracts.json`) that lack text. Today it only compares **those two files**: for the new one, adjust the `section_i18n` function or compare by hand:

```bash
cd app
node -e 'const f=(o,p="")=>Object.entries(o).flatMap(([k,v])=>typeof v==="object"?f(v,p+k+"."):[p+k]);const a=f(require("./src/i18n/locales/en.json")),b=new Set(f(require("./src/i18n/locales/fr.json")));console.log(a.filter(k=>!b.has(k)).join("\n"))'
```

In addition, `npm run typecheck` and `npm run lint` have to keep passing, and it is worth opening the app in the new language and going through the screens.

### The places with "Spanish or English" written by hand

These do **not** come out of `supportedLanguages` by themselves. With a new language they fall back to English until they are changed:

| Where | What it does |
|---|---|
| `components/whats-new-dialog.tsx` | Chooses `es` if the language starts with «es»; otherwise `en` |
| `components/about-dialog.tsx` | The address of the privacy policy: `es` or `en` |
| `components/add-search.tsx` | Tells the searches the app's language, `es` or `en` (TMDB uses it) |
| `app/src-tauri/src/metadata/mod.rs` (`tmdb_tag`) | Turns the language into the TMDB tag: `es-ES` or `en-US` |
| The rest of the search services | Ask in their usual language; for books and titles, the language of the edition does not depend on the app's |

If your language has to appear in the what's-new notes or in the privacy policy, those functions have to look at the exact language.

## The what's-new notes (`whats-new.json`)

Each line of `app/src/whats-new.json` has `es` and `en`. Three things require it: `app/scripts/check-whats-new.mjs` (`["es", "en"]`), the `Line` structure in `app/src-tauri/src/whats_new.rs` (fields `es` and `en`) and the dialog. For a new language, add the field to all three and **translate every line of every version** you want to be seen. Otherwise the person will see the what's-new notes in English.

## The server

Two things:

- **The account language (`locale`)**. Today it only accepts `en` and `es`: `server/src/Dto/RegisterAccountRequest.php`, `UpdateAccountRequest.php` and `GoogleStartRequest.php` validate it with `#[Assert\Choice(choices: ['en', 'es'])]`. The app sends it when the account is created and when signing in with Google (`i18n.resolvedLanguage`). The code has to be added to those three.
- **The emails**: `server/translations/emails.es.yaml` and `emails.en.yaml`, with the subject and body of each (confirm the account, recover the password, the warning that someone tried to use your email, the password changed, the second factor, the invitation…). A `emails.fr.yaml` is needed with **all** the keys. The server's default language is `en` (`config/packages/translation.yaml`), and it is the one used when there is no translation.

The error texts the server returns are **codes** (`invalid_credentials`), not sentences: the app translates them with its own files. They do not need touching.

## The documentation and the website

These are separate and are translated when someone is there to maintain them:

- **The documentation** (`site/`): Starlight has the languages in `site/astro.config.mjs` (today `es` by default and `en`). A new language is a new folder in `site/src/content/docs/<language>/` with the **same folder structure** (the folder names are the same in every language, because the language selector links to the same page). What is not translated comes out in Spanish with a notice.
- **The website** (`web/`): one folder per language (`web/es/`, `web/en/`) with its pages, among them the **privacy policy** and the one on deleting the account.
- **The privacy policy** is best kept in the language of whoever uses the app: the link in the «About» dialog has to point to it.

## Checklist

1. An *issue* open, and who will maintain the language.
2. `locales/<code>.json` complete and registered in `i18n/index.ts`, with its name in the selectors.
3. Genres, statuses, types, formats and roles with text; the app gone through in the new language.
4. The places with «es» or «en» written by hand, checked.
5. `whats-new.json`, the script that checks it and `whats_new.rs`, with the new field.
6. Server: the three `Assert\Choice` and `emails.<code>.yaml`.
7. Documentation and website, when possible.
8. `npm run typecheck`, `npm run lint`, `cargo test` and `make check` (in `server/`).
