# Encargo 7.8 (segunda parte) · Películas y series con TMDB a través del servidor

> Encargo cerrado para la sesión de hitos, 11/10/2026 ([plan](fase-7.md), «7.8 Proveedores con clave»). Sin commit, sin push, sin cambiar de rama. Toca `server/`, `app/`, `docs/` y **una línea** de `crates/domain` (la lista de sitios de portadas). Nada en `spec/`.

## Para qué

Hoy películas y series solo se buscan en Wikidata, con poco detalle. Quien tiene cuenta debe poder buscarlas **por título** en TMDB a través del servidor, que pone su clave (D62). Es el primer proveedor que busca por título por el servidor y el primero que exige atribución: lo que se haga aquí lo heredarán los siguientes.

Lee D18, D62, D83, `docs/research/proveedores.md`, la sección «Buscar libros a través del servidor» de `docs/architecture/cloud.md` y el código de la primera parte (`server/src/Metadata/`, `app/src-tauri/src/metadata/via_server.rs`, `Lookup::search`, `metadata/wikidata.rs`, `candidate.rs`, `providers-dialog.tsx`).

## Condiciones de TMDB (releídas el 11/10/2026; sus términos son del 20/10/2023)

- Uso no comercial: la app gratuita encaja.
- **No guardar sus datos más de 6 meses**: la caché del servidor queda muy por debajo.
- **Atribución obligatoria**: su logo, menos destacado que el de la app, y este aviso, tal cual (en inglés; añade su traducción en `es`): «This application uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.»
- No dicen nada de que un servidor busque en nombre de sus cuentas.

## Servidor

1. **Ruta de búsqueda** `POST /v1/metadata/tmdb/search`, con sesión como la de Google Books. Cuerpo JSON: `kind` (`film` o `tv_series`), `query` (2 a 100 caracteres tras limpiar) e `language` (`es` o `en`; cualquier otro, `en`). **Es `POST` a propósito: el título que busca una persona no va nunca en una dirección.**
2. **Respuesta propia**, una lista `candidates` de **8 como mucho**, cada uno con: `external_id` (el número de TMDB), `title`, `original_title`, `year`, `language` (el original), `cover_url`, y lo que venga del detalle: `directors` (película) o `creators` (serie), `duration_minutes` (película) y `seasons` (serie). Si un detalle falla, el candidato va sin esos campos; no se cae la búsqueda.
3. **Clave** en `TMDB_API_TOKEN` (el «API Read Access Token» de TMDB), vacía por defecto, enviada en la cabecera `Authorization: Bearer`, nunca en la dirección. Con clave, `/v1/server` añade `"tmdb"` a `metadata`; sin ella, la ruta responde `metadata_unavailable`.
4. **Llamadas a TMDB** (API v3): `search/movie` o `search/tv` con `include_adult=false`, y el detalle de cada resultado con sus créditos. Direcciones fijas; del cliente solo entran el tipo (de una lista cerrada), el idioma (de una lista cerrada), el texto de búsqueda como parámetro codificado y, para el detalle, un número que **devolvió TMDB**, no uno que mande el cliente. Mismos cuidados que con Google: tiempo máximo, tope de tamaño, sin redirecciones, excepciones sin `previous`, reintento ante un `5xx`.
5. **Caché**, en la misma tabla si encaja (dime si has tenido que cambiarla y por qué):
   - el detalle, por número de TMDB, tipo e idioma: 30 días;
   - la búsqueda, 1 día, con la clave de caché como **huella** (sha256) del tipo, el idioma y el texto normalizado: la tabla no debe guardar en claro qué títulos se buscan. Sin nada que diga quién preguntó.
6. **Límites:** el mismo de 600 por hora por cuenta, compartido con Google Books (una búsqueda cuenta una, aunque dentro haga varias llamadas). Y un tope propio del servidor de **5.000 llamadas reales a TMDB al día**, para que nadie nos haga machacarles. TMDB responde `429` si se pasa su ritmo: mismo freno que con Google.
7. **No se registra** qué busca cada cuenta, ni en el registro de actividad ni en ningún log. **Compruébame una cosa de la primera parte:** si una excepción de estas rutas (`AccountException`, ruta no encontrada, error de validación) acaba escrita en el log de la aplicación **con la dirección de la petición**; en la ruta de Google Books esa dirección lleva el ISBN. Dime qué has visto y, si es así, propón el arreglo sin hacerlo.

La interfaz `BookProvider` es de libros por ISBN: no la fuerces. Haz la pieza de TMDB con la forma que le toque y comparte solo lo que de verdad sea común (el freno, el reintento, la caché, el recorte de textos).

## App

1. `Source` gana `Tmdb`. En `Lookup::search`, para `Film` y `TvSeries`: si hay sesión, el servidor lo ofrece y la fuente no está desactivada en Proveedores, se pregunta al servidor **a la vez** que a Wikidata; los candidatos de TMDB van primero. Si el servidor falla o tarda, quedan los de Wikidata y el aviso de siempre. Sin sesión o sin oferta, todo como hoy y **no sale ninguna petición**.
2. Lo que se manda: el texto escrito, el tipo y el idioma de la app. Nada más.
3. **Portadas:** `https://image.tmdb.org/t/p/w500/<fichero>`. El servidor construye la dirección a partir del `poster_path` solo si tiene la forma `/<letras y números>.<jpg|png>`; la app la comprueba otra vez. Añade `image.tmdb.org` a `COVER_HOSTS` en `crates/domain/src/book.rs` (es lo único que se toca ahí).
4. **Proveedores:** TMDB aparece en la lista de fuentes de películas y series, se puede desactivar como las demás, y solo se ofrece con sesión en un servidor que lo tenga; si no, se explica que llega con la cuenta (D62). Un texto corto dice qué se envía.
5. **Atribución (decidido por el maintainer, opción A):** una sección «Fuentes de datos» dentro de Proveedores con el logo de TMDB y el aviso de arriba; en los resultados de búsqueda, la etiqueta de texto «TMDB», como las demás fuentes. **Ningún logo en las fichas.** El logo es el oficial de su página de marca, guardado en el repo y empaquetado con la app; nada se carga de fuera al mostrarlo. Si no puedes obtener el fichero oficial, no lo dibujes tú: deja el hueco y dímelo.
6. **Clave propia de TMDB en Ajustes** (D62 la prevé para todo proveedor con clave): **fuera de este encargo.** Apúntalo como pendiente en el plan.
7. Cadenas en `en` y `es`. Tests sin red. Nunca el `MutexGuard` a través de un `.await`.

## Casos hostiles (test antes de la defensa, visto fallar)

1. Sin sesión, token inválido, cuenta sin confirmar o bloqueada: ni respuesta ni llamada a TMDB.
2. Cuerpo que no es lo pedido: `kind` fuera de la lista, `query` vacía, de un mega, con saltos de línea o caracteres de control, `language` rara, campos de más, JSON roto: `422` (o el idioma por defecto), sin llamar a TMDB y sin fila.
3. Un texto de búsqueda con `&`, `?`, `#`, `/`, `..` o `%00`: viaja como un solo parámetro codificado; no cambia la dirección ni añade parámetros.
4. La clave no aparece en ninguna respuesta, error, excepción ni fila (clave centinela, como en la primera parte).
5. TMDB responde basura: no es JSON, enorme, mil resultados, títulos de un mega, un `id` que no es un número, un `poster_path` con `..`, con otra dirección dentro o con otro sitio: se recorta y se descarta; a la app solo le llega la forma fija y nunca más de 8.
6. TMDB responde `429` o `401`: freno, sin fila. `5xx`: reintento, como con Google.
7. Un detalle falla o tarda: la búsqueda responde igual, con ese candidato incompleto.
8. Dos cuentas buscan lo mismo: una sola tanda de llamadas, ninguna fila dice quién, y ninguna fila guarda el texto buscado en claro.
9. Límite por cuenta y tope diario, como en la primera parte.
10. En la app: un servidor que responde de más, de otro tipo, con cien candidatos o con portadas de otro sitio no rompe la búsqueda ni mete basura; uno que no ofrece TMDB no recibe la petición; y el texto no sale hacia el servidor para tipos que no sean película o serie.

## Reglas

- Doctrine primero; consultas con parámetros.
- Sin dependencias nuevas.
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. Servidor con `cd server && make …`; app con `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

Servidor: `make check` **después del último cambio** y `make migration`. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. La salida real de cada uno.

Documentación: `docs/architecture/cloud.md`, `docs/research/proveedores.md`, `docs/publicacion/servidor-produccion.md` (la variable nueva, en genérico), `docs/publicacion/politica-privacidad-con-servidor.md` (qué se envía a TMDB a través del servidor), `docs/plans/fase-7.md` y `docs/ROADMAP.md`.

Informe: qué has hecho, qué decidiste tú y por qué, qué tests viste fallar antes, qué no has podido comprobar, lo del log del punto 7, y cualquier cosa rara.
