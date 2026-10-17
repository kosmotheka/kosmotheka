---
title: Añadir una fuente de datos
description: "Cómo se añade a Kosmotheka un catálogo externo (libros, películas, juegos…): el patrón de la app, el del servidor, las pruebas y lo que hay que actualizar fuera del código."
sidebar:
  order: 15
---

Una **fuente de datos** es un servicio de internet al que la app pregunta para rellenar una ficha: la Biblioteca Nacional, TMDB, RAWG… Esta página explica cómo añadir una nueva. Léela entera antes de empezar: la parte difícil casi nunca es el código.

Dos reglas mandan en todo lo que sigue:

- **Las fuentes son opcionales.** Si una falla, la app avisa y sigue funcionando; nunca bloquea al usuario.
- **Unas pruebas nunca tocan la red.** Se lee una respuesta grabada. Las que sí necesitan red están ignoradas y se llaman `live_*`.

## Antes de escribir código: las condiciones del proveedor

Lee las condiciones de uso del proveedor **antes** de nada. Algunos:

- **no dejan guardar sus datos** (ni en una caché), o solo unos días;
- **no dejan modificarlos** (si los recortas o los mezclas con otros);
- **exigen un enlace o un logo** en cada página donde se usan sus datos;
- **prohíben el uso comercial**, o limitan las llamadas por hora o por día;
- **piden una clave a nombre de quien la usa**, que no puede ir en la app para todos.

Eso decide cómo se hace. Si el proveedor quiere una clave, cada persona pone la suya (o la pone el servidor de su cuenta). Si pide un enlace, la app lo enseña (ver `source-credits.tsx`). Si no deja guardar, no se guarda. Si las condiciones no permiten lo que quieres, **no se añade**.

## En la app

Todo está en `app/src-tauri/src/metadata/`. Ojo: **no hay un `trait` de proveedor**. Cada fuente es un módulo con funciones sueltas, y `mod.rs` decide a cuál llamar según el tipo (`Kind`). Es deliberado: son pocas, cada una responde distinto, y un trait obligaría a esconder esas diferencias.

### 1. Un módulo por fuente

Toma como modelo `musicbrainz.rs` (sin clave, un solo servicio) o `rawg.rs` (con clave). Cada módulo tiene:

- una función `search(client, …, query) -> Result<Vec<Candidate>, ProviderError>`, y `lookup_*` si la fuente responde a un código (ISBN, código de barras);
- una función **pura** que convierte lo que contesta el proveedor en `Candidate` (`parse_releases` en MusicBrainz). Sin red, sin estado: es lo que se prueba;
- la dirección del servicio como una constante.

`Candidate` (`candidate.rs`) es el resultado que la persona puede elegir para rellenar el formulario. Se construye con `Candidate::new(kind, source, título)` y se rellenan los campos que la fuente conoce; los nombres coinciden con los de `EditionInput` del dominio. **Un campo que el proveedor no da se deja vacío**, no se inventa.

Los errores son `ProviderError`: red, cuota agotada (`Quota`), clave rechazada (`BadKey`) y respuesta inesperada (`Unexpected`). Si la fuente lleva clave, usa `get_json` (en `mod.rs`): lee con un tope de tamaño, traduce 401 y 403 a `BadKey` y 429 a `Quota`, y **no deja salir la dirección en ningún error**, porque ahí puede ir la clave.

### 2. Registrarla

- `Source` en `metadata/mod.rs`: una variante nueva, en `Source::ALL` y en `id()` (el id en `snake_case`, que es el que guardan los ajustes y el que usa `whats-new.json`).
- En `Lookup::search` (o `run`, o `lookup_code`) de `mod.rs`: añade la llamada al tipo que corresponda, junto a las demás, **con `maybe_vec(disabled.contains(…id()), …)`**: así un interruptor apagado no la llama. Decide el orden en `collect_candidates`: la primera fuente es la que sale arriba.
- En `candidate_from_lookup` (el `match` es exhaustivo): el compilador te lo exige.
- `app/src/lib/api.ts`: el tipo `Source` (la unión de ids).

### 3. Ajustes › Proveedores de datos

La pantalla es `app/src/components/providers-dialog.tsx`:

- `SOURCES_BY_KIND`: en qué tipos aparece la fuente.
- Si lleva clave propia: `KEYED_SOURCES`, `GET_KEY_URL` (dónde se pide), `KEYED_PROVIDERS` en `settings.rs` (Rust) y una prueba de la clave en `testSource` (una búsqueda que sabes que da resultado).
- Si el servidor de la cuenta puede preguntarla cuando no hay clave: `ACCOUNT_SOURCES`, y su campo en `Offers`.
- Si **solo** la pregunta el servidor, sin clave propia (como BoardGameGeek): `ACCOUNT_ONLY_SOURCES`.
- Si pide un enlace: `SOURCES_WITH_A_LINK` en `source-credits.tsx`.

El interruptor sale solo: la lista de fuentes apagadas (`disabled_sources`) guarda los ids.

Textos, en `es.json` y `en.json`, **los dos**: el nombre en `lookup.sources`, la descripción en `providers.sourceDescriptions`, lo que aporta a cada tipo, y el aviso o el enlace si el proveedor los pide. No se escribe ningún texto en el código (ver [añadir un idioma](../add-a-language/)).

### 4. Pruebas

- **Respuestas grabadas** en `app/src-tauri/tests/fixtures/`: un acierto, un «no encontrado» y una búsqueda (`musicbrainz-search-random-access-memories.json`). Se leen con `include_str!("../../tests/fixtures/…")` en un `mod tests` dentro del propio módulo. Graba la respuesta real una vez y recórtala: no inventes una.
- **Pruebas `live_*`**, con `#[ignore = "needs network"]`: llaman al servicio de verdad. Se lanzan a mano con `cargo test -p kosmotheka -- --ignored live_`. No corren en la integración continua.
- Si necesitas un generador de fixtures, se llama `print_*`, está ignorado y se lanza a mano cuando cambia el formato.

## Si la fuente pasa por el servidor

Una fuente con clave puede preguntarse **a través del servidor de la cuenta**: así la clave no la tiene que pedir cada persona. Comic Vine (`server/src/Metadata/ComicVine*.php`) es el modelo. Cada fuente tiene:

| Pieza | Para qué |
|---|---|
| `*Source` | Una interfaz: `isConfigured()` (¿hay clave?) y `search()`. Las pruebas usan una falsa y nunca llegan a la red |
| `*HttpSource` | La llamada real. Si la clave solo se admite en la dirección, todo se arregla para que no salga de ahí: un cliente sin registro (`UnloggedClientFactory`), errores con texto fijo y sin la excepción original, y respuestas leídas con tope de tamaño y campo a campo |
| `*Search` | Lo que usa el controlador: mira la caché, si no la pregunta a la fuente, y guarda el resultado |
| `ProviderGate` | Pasa todas las llamadas: si el proveedor dice «cuota agotada», el servidor lo deja en paz 30 minutos; si dice «roto», reintenta hasta tres veces |
| `*Hit` y `*Detail` | Lo que sale de la fuente, ya recortado y comprobado |

Y en el resto del servidor:

- **La caché** (`MetadataCacheEntry`) guarda la respuesta y su plazo, **sin quién preguntó ni el texto**: la búsqueda se guarda bajo una huella (`fingerprint()`) que no permite recuperar el texto. Un fallo nunca se guarda. Dale a la fuente su plazo en `SEARCH_TTLS` y, si tiene detalle por id, su prefijo en `DETAIL_PREFIXES`.
- **Los límites** (`config/packages/rate_limiter.yaml`): el del servidor entero (un poco por debajo de lo que admite el proveedor) y el de cada cuenta (una décima parte, para que una cuenta no gaste el de todas).
- **La ruta**, en `MetadataController`: un `POST` (el texto buscado nunca va en una dirección), con su `Dto`, el límite `metadata_account` y `Cache-Control: no-store`. Si la fuente no está configurada, `metadataUnavailable`.
- **`GET /v1/server`** (`ServerController`) añade el nombre de la fuente a `metadata` cuando hay clave. Es lo que mira la app para saber qué ofrece este servidor.
- **La clave**: una variable de entorno nueva (`COMIC_VINE_API_KEY`), vacía por defecto, en `.env`, `services.yaml` (el cliente HTTP, y en el entorno de pruebas uno falso) y en [la configuración](/es/run-your-server/configuration/#buscadores-de-datos).
- **Nunca en un registro**: ni la clave, ni el texto, ni quién lo buscó. Las direcciones `/v1/metadata/` tampoco van al registro de accesos de nginx.

En la app, la ruta se llama a través del `trait CloudApi` (`cloud/mod.rs`): un método nuevo (`comic_vine_search`), su implementación real en `cloud/client.rs`, la falsa de `cloud/tests.rs`, y una función `search_*` y un lector de la respuesta (`parse_*_answer`) en `metadata/via_server.rs`. Ese lector **no se fía de lo que contesta el servidor**: corta y comprueba cada campo.

Copia también las pruebas de la fuente más parecida (`comic_vine_tests.rs` en la app, `ComicVine*Test` en el servidor): cubren a un servidor que no ofrece la fuente, la cuota agotada, un texto que no debe salir, y una respuesta mal formada.

## Fuera del código

- **`app/src/whats-new.json`**: una línea, en español e inglés, bajo `next`. Como la fuente cambia **lo que sale del dispositivo** (lo que escribes va a un servicio), la línea lleva `"switch": "source:<id>"`: la app enseña el interruptor al actualizarse. Ver [Novedades de cada versión](/es/using-the-app/whats-new/) y [publicar una versión](../release/).
- **La política de privacidad** (`web/es/privacidad.html` y `web/en/…`): qué se envía, a quién, qué guarda el servidor y cuánto tiempo. Si la fuente pasa por el servidor, también la lista de lo que la caché guarda.
- **La guía «De dónde salen los datos»** (`using-the-app/data-sources.md`, y su versión en inglés) y la de [qué sale de tu dispositivo](/es/using-the-app/what-leaves-your-device/).
- **Las condiciones del proveedor** que obliguen a algo (un enlace, un logo, una frase): en la app y en esas guías.

## Lista de comprobación

1. Leídas las condiciones del proveedor; lo que piden, hecho.
2. Módulo con `search`, un lector puro y pruebas con respuestas grabadas; pruebas `live_*` ignoradas.
3. `Source`, `Source::ALL`, `id()`, `Lookup::search`, tipo de `api.ts`, `SOURCES_BY_KIND`.
4. Textos en `es.json` y `en.json`.
5. Si pasa por el servidor: las siete piezas de arriba, la clave, la variable de entorno y la documentación de la configuración.
6. `whats-new.json` con su `switch`, política de privacidad y guías.
7. Comprobaciones: `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run typecheck` y `npm run lint` en `app/`, y `make check` en `server/`.
