# Encargo 7.8 (primera parte) · Google Books a través del servidor

> Encargo cerrado para la sesión de hitos, 11/10/2026 ([plan](fase-7.md), «7.8 Proveedores con clave»). Sin commit, sin push, sin cambiar de rama. Toca `server/`, `app/` y `docs/`. Nada en `spec/` ni `crates/`.

## Para qué

Google Books sin clave falla a menudo (cuota compartida agotada, D83), y es la fuente que mejor cubre los libros en español. Quien tiene cuenta debe poder buscar un ISBN en Google Books **a través del servidor**, que pone su clave. Lee D18, D62 y D83 enteras, `docs/research/proveedores.md`, `server/AGENTS.md`, y en la app `metadata/mod.rs`, `metadata/google.rs`, `commands::lookup_isbn` y `lookup_code`, `settings.rs` y `providers-dialog.tsx`.

**Alcance: solo la búsqueda por ISBN en Google Books.** Ni búsqueda por título, ni otros proveedores, ni portadas a través del servidor. La forma debe servir para añadir después otro proveedor sin rehacerla, pero no construyas nada para proveedores que aún no existen.

## Servidor

1. **Ruta** `GET /v1/metadata/google-books/isbn/{isbn}`, con sesión de dispositivo como las demás rutas de la nube (cuenta confirmada y no bloqueada). Responde un JSON **propio y pequeño**, no el de Google: `found`, `title`, `subtitle`, `authors`, `publisher`, `published_year`, `language`, `cover_url`, `external_id`. Los mismos campos que `Metadata` de la app.
2. **Clave** en `GOOGLE_BOOKS_API_KEY` (vacía por defecto en `.env`, nunca un valor real en el repo). Sin clave, la ruta responde «no disponible» con un código propio y `/v1/server` dice `metadata: []`; con clave, `metadata: ["google_books"]`.
3. **Llamada a Google** con `symfony/http-client` (ya está instalado porque lo trae Brevo; pásalo a dependencia directa; ninguna otra dependencia nueva). La clave va en la cabecera `X-Goog-Api-Key`, **no en la dirección**. Dirección fija; del cliente solo entra el ISBN ya validado. Tiempo máximo corto (unos 5 s) y tope de tamaño de la respuesta.
4. **Caché** en una tabla propia (entidad Doctrine, migración con `make migration`): proveedor, código, respuesta ya normalizada, si se encontró y cuándo se pidió. **Sin nada que diga quién preguntó.** Se guarda también el «no encontrado». Plazos en constantes de un solo sitio: 30 días lo encontrado, 1 día lo no encontrado. La purga, en el comando y la planificación que ya existen. Un fallo de Google (cuota, error, tiempo agotado) no se guarda.
5. **Límites**, con `RequestLimiter` y `rate_limiter.yaml` como los demás:
   - por cuenta: 600 búsquedas por hora (cuenta todas, también las que salen de la caché);
   - del servidor entero: 900 llamadas **reales** a Google al día (su cuota gratuita es de 1.000). Agotado, la ruta responde «cuota agotada» con un código propio y sigue sirviendo lo que haya en caché.
6. **Lo que no se registra:** qué ISBN busca cada cuenta no va al registro de actividad ni a ningún log. Dímelo en el informe si alguna pieza lo escribiría (el log de accesos de nginx sí lo verá: apúntalo en la guía de producción del repo como regla `access_log off` para `/v1/metadata/`, igual que las invitaciones).

## App

1. `CloudApi` gana la búsqueda por ISBN (con su `FakeCloud`), y `/v1/server` se lee para saber si el servidor la ofrece.
2. **Cuándo se usa**, en `lookup_isbn` y `lookup_code`:
   - hay **clave propia** en Ajustes → se llama a Google directamente con ella, como hoy (tiene preferencia, D83);
   - si no, hay **sesión**, el servidor lo ofrece y el ajuste está encendido → se pregunta al servidor;
   - si no → Google sin clave, como hoy.
   Si el servidor falla (red, no disponible, cuota, límite), se prueba Google sin clave como hoy; si también falla, el aviso es el de siempre. Un «no encontrado» del servidor es una respuesta, no un fallo.
3. Los demás proveedores no cambian, y si Google Books está desactivado en Proveedores no se pregunta a nadie, tampoco al servidor.
4. **Ajuste nuevo** en Proveedores, **encendido por defecto** y visible solo con sesión en un servidor que lo ofrezca: buscar en Google Books a través de la nube. Con un texto corto que diga qué se envía (el ISBN y nada más) y que se puede apagar (D83). Cadenas en `en` y `es`.
5. El `Source` sigue siendo `GoogleBooks`: para quien usa la app es la misma fuente.
6. Recuerda: nunca el `MutexGuard` de la base a través de un `.await`. Tests sin red, con respuestas grabadas.

## Casos hostiles (test antes de la defensa, visto fallar)

1. Sin sesión, con token inválido, con cuenta sin confirmar o bloqueada: no hay respuesta ni llamada a Google.
2. Un ISBN que no lo es (letras, dígito de control malo, barras, `..`, saltos de línea, muy largo): `422`, **sin llamar a Google y sin fila en la caché**. ISBN-10 y con guiones: decide si el servidor los acepta y normaliza a 13 o solo acepta 13 dígitos; que la caché tenga una sola forma por libro.
3. La clave no aparece nunca: ni en la respuesta, ni en un error, ni en un mensaje de excepción que acabe en un log, ni en la caché. Un test con una clave centinela recorre respuestas y filas.
4. Google responde basura: no es JSON, JSON enorme, campos de otro tipo, un título de un mega, cien mil autores, una `cover_url` que no es `https` o apunta a otro sitio que no sea de Google Books. El servidor recorta, descarta lo que no encaja y no rompe; a la app solo le llega la forma fija.
5. Google responde 429 o 403: «cuota agotada», sin fila en la caché; la segunda petición no machaca a Google sin freno (dime cómo lo frenas).
6. Una cuenta pasa su límite: se le corta a ella, no a las demás. El tope diario del servidor se agota: lo que está en caché se sigue sirviendo.
7. Dos cuentas buscan el mismo ISBN: una sola llamada a Google, y ninguna fila guarda quién fue.
8. En la app: un servidor que responde con campos de más, de otro tipo o enormes no rompe la búsqueda ni rellena el formulario con basura; y un servidor que no ofrece la búsqueda nunca recibe la petición.

## Reglas

- Doctrine primero; consultas con parámetros, nunca texto concatenado.
- El controlador, fino; la llamada a Google detrás de una interfaz para probar con `MockHttpClient`.
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. Servidor con `cd server && make …`; app con `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

Servidor: `make check` **después del último cambio** y `make migration` para ver que no queda nada pendiente. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. Dime la salida real de cada uno.

Documentación: `docs/architecture/cloud.md` (la ruta, los códigos, la caché, los límites), `docs/research/proveedores.md`, `docs/publicacion/servidor-produccion.md` (la variable nueva y la regla de nginx, en genérico), el borrador `docs/publicacion/politica-privacidad-con-servidor.md` (qué se envía y que no se guarda quién lo buscó), `docs/plans/fase-7.md` y `docs/ROADMAP.md`.

Informe: qué has hecho, qué decidiste tú y por qué, qué tests viste fallar antes, qué no has podido comprobar, y cualquier cosa que te haya parecido rara.
