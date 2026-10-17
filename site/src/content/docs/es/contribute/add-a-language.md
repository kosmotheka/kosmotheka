---
title: Añadir un idioma
description: "Qué hay que traducir y tocar para que la app, el servidor, las novedades y la documentación de Kosmotheka estén en un idioma nuevo."
sidebar:
  order: 16
---

Hoy Kosmotheka está en **español e inglés**. Añadir un tercer idioma es posible, pero hay más sitios de los que parece: la mayoría se traducen sin tocar código, y unos pocos tienen «español o inglés» escrito a mano. Esta página los enumera todos, para que sepas el tamaño del trabajo antes de empezar. En los ejemplos, el idioma nuevo es el francés (`fr`).

Antes de empezar, abre un *issue* (ver [cómo contribuir](../how-to-contribute/)): un idioma hay que **mantenerlo** en cada versión, y es mejor saber quién lo hará.

## La app

### Los textos

Todos los textos de la interfaz están en `app/src/i18n/locales/`, un archivo por idioma:

- `en.json` y `es.json` hoy. Copia `en.json` como `fr.json` y traduce los valores, **nunca las claves**.
- `app/src/i18n/index.ts` es **la lista de idiomas de la interfaz**: `import fr from "./locales/fr.json"`, `fr: { translation: fr }` en `resources`, `"fr"` en `supportedLanguages` y su nombre, **en su propio idioma**, en `languageNames` (TypeScript avisa si falta). De ahí leen los selectores de idioma (Ajustes, la cuenta y el alta), las novedades, el enlace de la privacidad y el idioma de las búsquedas: no hay que tocar nada más en la interfaz, salvo el enlace de la política de privacidad si la tienes en tu idioma (`PRIVACY` en `about-dialog.tsx`; sin ella, se usa la inglesa).

La app elige el idioma del sistema (`i18next-browser-languagedetector`) y, si no tiene el suyo, cae en inglés.

Cosas del texto que se tropiezan:

- **Plurales**: una clave con `count` lleva `_one` y `_other` (`results_one`, `results_other`). Algunos idiomas necesitan más formas; consulta las reglas de i18next.
- **Variables**: `{{title}}` y similares se dejan igual.
- **Etiquetas**: `<link>RAWG</link>` en los textos de créditos se queda; solo se traduce lo de dentro.
- **Sin texto en el código**: si un texto no está en `en.json`, no se puede traducir. Añádelo a los idiomas que ya existen.

### Las listas cerradas

Algunas listas viven en el dominio (Rust) y su texto, en los archivos de arriba:

| Lista | Dónde está | Claves de texto |
|---|---|---|
| Géneros de cada tipo | `crates/domain/src/genre.rs` | `genres.*` |
| Estados de lectura, de juego… por tipo | dominio | `marks.statuses.*`, `marks.statusesByKind.<tipo>.*` |
| Tipos de obra, formatos de ejemplar | dominio | `types.*`, `typesPlural.*`, `formats.*` |
| Roles de las personas y estados de las peticiones | dominio | `people.roles.*`, `requests.status.*` |
| Códigos de error | Rust y servidor | `accountErrors.*`, `cloud.errors.*`, `invitations.errors.*`… |

Un género o un estado **se guarda por su clave** (`science_fiction`), no por su texto: traducirlos no cambia ningún dato ni la sincronización. Pero **cada clave de la lista tiene que existir en tu archivo**; si falta una, la interfaz enseña la clave en bruto.

### Lo que avisa de claves que faltan

`npm run check:i18n` (en `app/`, y en la integración continua) falla si una clave está en un idioma y no en otro. Compara **todos** los archivos de `src/i18n/locales/`, así que el tuyo entra solo. La única excepción son los `genresByKind`: un idioma puede necesitar ajustar un género según el tipo (en español, «Policiaca» para una película) y otro no, porque la app usa entonces el general (`genres.<clave>`).

Además, la auditoría (`.claude/skills/audit/bin/facts.py`) avisa de las claves que el código usa y no existen, y de las listas cerradas (`contracts.json`) a las que les falta texto.

Además, `npm run typecheck` y `npm run lint` tienen que seguir pasando, y conviene abrir la app en el idioma nuevo y recorrer las pantallas.

### Dónde está la lista de cada parte

Cada parte tiene **una sola lista** de idiomas, y lo demás la lee. Al añadir un idioma, tocas la de cada parte y nada más:

| Parte | Dónde está la lista | Qué se añade |
|---|---|---|
| La interfaz | `app/src/i18n/index.ts`: `supportedLanguages`, `languageNames` | El código y el nombre del idioma en sí mismo, más su `locales/<código>.json` |
| La política de privacidad que enlaza «Acerca de» | `PRIVACY` en `app/src/components/about-dialog.tsx` | La dirección de la política en el idioma nuevo. Opcional: sin ella se enseña la inglesa |
| TMDB (películas y series) | `TMDB_TAGS` en `app/src-tauri/src/metadata/mod.rs` y `tmdbTag()` en `server/src/Enum/Locale.php` | Cómo escribe TMDB ese idioma (`fr` → `fr-FR`). Opcional: sin ella se pregunta en `en-US` |
| Las novedades | Los archivos de `app/src/i18n/locales/` | Nada: `check:whats-new` exige texto en cada idioma que tenga archivo |
| El servidor | `server/src/Enum/Locale.php` | Un caso nuevo; los tres formularios lo validan contra él |

Los demás buscadores preguntan en su idioma de siempre: para libros y títulos, el idioma de la edición no depende del de la app.

## Las novedades (`whats-new.json`)

Cada línea de `app/src/whats-new.json` lleva el texto de cada idioma, por su código (`"es": …`, `"en": …`, y el nuevo). El inglés es obligatorio y es el respaldo: si una línea no tiene texto en el idioma de la persona, se enseña el inglés. `npm run check:whats-new` exige texto en **todos los idiomas que tienen archivo** en `src/i18n/locales/`, así que para un idioma nuevo hay que **traducir todas las líneas de todas las versiones** que quieras que se vean. No hay que tocar ni el script ni `whats_new.rs`.

## El servidor

Dos cosas:

- **La lista de idiomas (`locale` de la cuenta)**: `server/src/Enum/Locale.php`, un enum con un caso por idioma. `RegisterAccountRequest`, `UpdateAccountRequest` y `GoogleStartRequest` lo validan con `#[Assert\Choice(callback: [Locale::class, 'values'])]`, así que añadir el caso basta. La app manda su idioma al crear la cuenta y al entrar con Google. Si TMDB lo escribe distinto, añade también su etiqueta en `tmdbTag()`.
- **Los correos**: `server/translations/emails.es.yaml` y `emails.en.yaml`, con el asunto y el cuerpo de cada uno (confirmar la cuenta, recuperar la contraseña, el aviso de que alguien intentó usar tu correo, la contraseña cambiada, el segundo factor, la invitación…). Hace falta un `emails.fr.yaml` con **todas** las claves. Son traducciones, no una lista. El idioma por defecto del servidor es `en` (`config/packages/translation.yaml`) y es el que usa si no hay traducción.

Los textos de error que el servidor devuelve son **códigos** (`invalid_credentials`), no frases: la app los traduce con sus archivos. No hay que tocarlos.

## La documentación y la web

Son aparte y se traducen cuando haya quien los mantenga:

- **La documentación** (`site/`): Starlight tiene los idiomas en `site/astro.config.mjs` (hoy `es` por defecto y `en`). Un idioma nuevo es una carpeta nueva en `site/src/content/docs/<idioma>/` con la **misma estructura de carpetas** (los nombres de carpeta son los mismos en todos los idiomas, porque el selector de idioma enlaza a la misma página). Lo que no esté traducido sale en español con un aviso.
- **La web** (`web/`): una carpeta por idioma (`web/es/`, `web/en/`) con sus páginas, entre ellas la **política de privacidad** y la de borrar la cuenta.
- **La política de privacidad** conviene tenerla en el idioma de quien usa la app: el enlace del diálogo «Acerca de» tiene que apuntar a ella.

## Lista de comprobación

1. Un *issue* abierto, y quién mantendrá el idioma.
2. `locales/<código>.json` completo y registrado en `i18n/index.ts` (`resources`, `supportedLanguages`, `languageNames`).
3. Géneros, estados, tipos, formatos y roles con texto; la app recorrida en el idioma nuevo.
4. Opcional: la política de privacidad en `PRIVACY` y la etiqueta de TMDB (`TMDB_TAGS`, `Locale::tmdbTag()`).
5. `whats-new.json`: texto en el idioma nuevo en cada línea.
6. Servidor: un caso en `Locale` y `emails.<código>.yaml`.
7. Documentación y web, cuando se pueda.
8. `npm run typecheck`, `npm run lint`, `npm run check:i18n`, `npm run check:whats-new`, `cargo test` y `make check` (en `server/`).
