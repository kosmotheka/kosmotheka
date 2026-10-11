# Encargo 7.8 (tercera parte) · Videojuegos con RAWG a través del servidor

> Encargo cerrado para la sesión de hitos, 11/10/2026 ([plan](fase-7.md), «7.8 Proveedores con clave»). Sin commit, sin push, sin cambiar de rama. Toca `server/`, `app/`, `docs/` y **una línea** de `crates/domain` (la lista de sitios de portadas). Nada en `spec/`.

## Para qué

Los videojuegos solo se buscan hoy en Wikidata. Quien tiene cuenta debe poder buscarlos **por título** en RAWG a través del servidor (D62). Es la misma pieza que TMDB con otro proveedor: **sigue lo hecho en la segunda parte** ([encargo](../historico/encargo-7.8-tmdb.md), `server/src/Metadata/Tmdb*`, `ProviderGate`, `TextCleaner`, `app/src-tauri/src/metadata/via_server.rs`, `providers-dialog.tsx`) y comparte lo que de verdad sea común, sin forzar una abstracción para dos casos.

Antes de empezar mira lo que cambió desde tu entrega: el director se busca en todo el equipo (no en los 200 primeros), la ruta de Google Books acepta cualquier cosa tras `/isbn/` y responde `422`, y el diálogo de Proveedores ya no tiene `min-h-0` (la sección de atribución se pintaba encima de la lista) y su aviso de arriba depende de si hay cuenta.

## Condiciones de RAWG (releídas el 11/10/2026; sus términos son del 01/03/2021)

- Gratis hasta **20.000 peticiones al mes**.
- **Atribución:** nombrar a RAWG como fuente y poner un **enlace activo en cada página donde se usen sus datos**.
- No dicen nada de guardar los datos ni de que un servidor busque en nombre de sus cuentas.

## Servidor

1. **Ruta** `POST /v1/metadata/rawg/search`, con sesión. Cuerpo: `query` (2 a 100 caracteres tras limpiar). Nada más: ni tipo ni idioma (RAWG no traduce). `POST` para que el título no vaya en una dirección.
2. **Respuesta propia**, `candidates` con **8 como mucho**: `external_id` (el número de RAWG), `title`, `year`, `cover_url`, `platforms` (nombres, 10 como mucho) y, solo para los que tengan detalle, `developers` y `publishers`.
3. **Clave** en `RAWG_API_KEY`, vacía por defecto. **RAWG solo admite la clave como parámetro `key` de la dirección**: no hay cabecera. Por eso aquí importa más que nunca que ninguna excepción lleve `previous`, que nada escriba la dirección de la llamada saliente en un log ni en una respuesta, y que el test de la clave centinela lo recorra todo (respuestas, filas, log). Si encuentras en su documentación una forma de mandarla por cabecera, úsala y dímelo.
4. **Llamadas** (`https://api.rawg.io/api`): `games?search=…&page_size=8` y, **solo para los tres primeros resultados**, el detalle `games/{id}` (desarrolladores y editores). El número del detalle sale de lo que devolvió RAWG. Mismos cuidados: tiempo máximo, tope de lectura, sin redirecciones, reintento ante `5xx`, freno ante `429`/`401`/`403`.
5. **Caché**, en la misma tabla: la búsqueda **7 días** bajo huella (sha256 del texto normalizado; el texto nunca en claro); el detalle 30 días por número de RAWG. Sin quién preguntó.
6. **Límites:** el de 600 por hora por cuenta, compartido. Tope propio del servidor de **600 llamadas reales a RAWG al día** (20.000 al mes entre 30, con margen); una búsqueda nueva cuesta hasta cuatro. Agotado, se sirve lo que haya en caché y la app sigue con Wikidata.
7. `/v1/server` añade `"rawg"` a `metadata` cuando hay clave.
8. No se registra qué busca cada cuenta.

## App

1. `Source` gana `Rawg`. En `Lookup::search`, para `Videogame`, junto a Wikidata y con las mismas condiciones que TMDB (sesión, oferta, fuente no desactivada); los de RAWG primero. Sin sesión u oferta, ninguna petición.
2. **Plataforma:** el candidato lleva `platform` solo si RAWG da **una sola**; con varias se deja vacío para que la persona elija la de su ejemplar. Desarrolladores y editores: mira los papeles que existen (`ContributorRole`) y el campo `publisher`, usa los que encajen y dime cuáles; no añadas papeles nuevos (eso toca `spec/`).
3. **Portadas:** `https://media.rawg.io/media/…` con extensión `jpg`, `jpeg`, `png` o `webp`; el servidor la valida y la app otra vez. Añade `media.rawg.io` a `COVER_HOSTS` (lo único que se toca en `crates/domain`). Comprueba que la app sabe guardar el formato que RAWG sirve; si no, descarta la portada y dímelo.
4. **Proveedores:** RAWG en la lista de videojuegos, con las mismas reglas que TMDB.
5. **Atribución (más exigente que TMDB, decidido con el maintainer):**
   - en «Fuentes de datos», el nombre RAWG con un **enlace** a `https://rawg.io`;
   - **en la pantalla de resultados de búsqueda**, cuando haya algún resultado de RAWG, una línea discreta «Datos de RAWG» con ese enlace, además de la etiqueta de texto en cada resultado;
   - nada en las fichas guardadas.
   El enlace se abre con el navegador del sistema, como los demás enlaces externos de la app. Sin logo (no lo exigen).
6. Cadenas en `en` y `es`. Tests sin red.

## Casos hostiles (test antes de la defensa, **visto fallar de verdad esta vez también en el servidor**: ejecuta los tests antes de escribir la defensa y dime la salida)

Los mismos diez del encargo de TMDB, adaptados, y además:

11. **La clave va en la dirección saliente:** ningún fallo (tiempo agotado, DNS, respuesta rota, `4xx`, `5xx`) la deja en una respuesta, una fila o una línea del log. Usa `RecordingLogger`.
12. RAWG devuelve una portada de otro sitio, con `..`, con otra dirección dentro o con una extensión que no es de imagen: se descarta.
13. RAWG devuelve cien plataformas o nombres enormes: se recorta.

## Reglas

- Doctrine primero; consultas con parámetros. Sin dependencias nuevas.
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. Servidor con `cd server && make …`; app con `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

Servidor: `make check` **después del último cambio** y `make migration`. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. La salida real de cada uno.

Documentación: `docs/architecture/cloud.md`, `docs/research/proveedores.md`, `docs/publicacion/servidor-produccion.md` (la variable, en genérico), `docs/publicacion/politica-privacidad-con-servidor.md`, `docs/plans/fase-7.md` y `docs/ROADMAP.md`.

Informe: qué has hecho, qué decidiste tú y por qué, qué tests viste fallar antes, qué no has podido comprobar y cualquier cosa rara.
