# Encargo 7.5.5b · La nube en la biblioteca: qué se sube, el interruptor y los avisos

> Encargo cerrado para la sesión de hitos, 10/10/2026. Segunda de las tres partes de 7.5.5 ([plan](fase-7.md)). Sin commit, sin push, sin cambiar de rama.

## Qué hay y qué falta

Por dentro está hecho (D76 con su enmienda del 10/10/2026, D95, D96, hitos 7.5.1 a 7.5.4): solo sube una biblioteca quien la administra (o cualquiera si no tiene roles); la nube es un dispositivo de la biblioteca (`library.cloud_device_id`) con un solo poder, vincular dispositivos nuevos; `people::name_cloud_device` y `people::remove_cloud_device` dan y quitan ese poder; `cloud::sync_all` reclama (`claim`) y devuelve informes por biblioteca. Falta la pantalla de casi todo eso.

Lee antes: `AGENTS.md`; D76 (y su enmienda) y D95 en `docs/DECISIONS.md`; `docs/architecture/cloud.md` (qué hace `claim`, la tabla de `sync_all`, los códigos de error); `docs/architecture/security.md`, piezas 5 y 6. Del diseño, en `docs/design/screens/cloud-family/`: `CloudPick.dc.html`, `LibSettings.dc.html`, `Notice.dc.html`, `Lib.dc.html`, `Copy.dc.html` y `errors-fase-7b.md`. Las diferencias conocidas con el diseño están al final de `docs/design/screens/README.md`.

## Qué construir

1. **Qué bibliotecas subir** (`components/cloud/cloud-pick-dialog.tsx`):
   - las que no administras salen sin casilla, con «La sube quien la administra»; las que no tienen personas ni roles, con casilla y «Sin personas ni roles»;
   - el texto de cabecera (`cloud.pick.intro`, hoy «solo las que puedes compartir», falso desde 7.5.2b-1) pasa al de Design (`cloud_pick.lead`);
   - el dato sale de Rust (`LibraryCloudStatus` ya calcula `can_upload` con `administers`): si la pantalla necesita distinguir «no administras» de «sin firmar» o «ya subida», añade a ese struct el campo que haga falta, y a su tipo de TypeScript. La UI no decide el permiso.
2. **El interruptor «La nube puede dar acceso a dispositivos nuevos»:**
   - en `CloudPick`, al subir (el que Design dibujó en la primera tanda y se dejó sin mostrar): averigua qué hace hoy la subida con `cloud_device_id` y haz que el interruptor diga la verdad. Encendido por defecto, como el diseño;
   - en los ajustes de cada biblioteca que está en la nube (`LibSettings`): para quien administra, el interruptor con sus dos líneas (`autolink.on` / `autolink.off`); apagarlo pide confirmar (`autolink.off_confirm`), encenderlo no. Para quien no administra, el estado sin poder cambiarlo (`autolink.readonly`). Si la biblioteca no tiene roles, mira qué permite el código y dímelo antes de inventar una regla;
   - comandos de Tauri sobre `name_cloud_device` y `remove_cloud_device` si no existen, y el estado («la nube tiene el poder en esta biblioteca») calculado en Rust.
3. **Avisos tras sincronizar** (`lib/cloud.ts`, `cloudNotices`):
   - «Ya estás en «Casa» en la nube, como Jhonny · lector» (`joined.one`, `joined.many`) cuando `claim` enlaza la cuenta con una persona de una biblioteca que este dispositivo ya tenía. Una vez, no en cada sincronización: decide en Rust cómo saber que es nuevo (por ejemplo, que `claim` lo devuelva en `linked` y antes no estuviera) y dime cómo lo resolviste. Si hace falta guardar algo, es estado local de este dispositivo, no una operación;
   - «Quien administra «Casa» ha quitado este dispositivo…» (`device.removed_by_admin`): comprueba si los avisos `cloud.notice.removed` y `accessWithdrawn` que ya existen cubren los dos casos del diseño (`access_revoked_wiped`, `access_revoked_unverified`) y ajusta los textos a los de Design, con el nombre de la biblioteca si se tiene.
4. **El dispositivo sin persona:** al texto que ya existe («Pide a quien administra que diga de quién es este dispositivo») se le añade «…y entrarás también por la nube» solo si hay sesión iniciada y la biblioteca está en la nube (`device.no_person`).
5. **Errores de la nube con texto propio** (cierra CON-008): `link_rejected`, `link_not_allowed`, `events_unavailable` y `access_withdrawn` en `app/src/lib/account-errors.ts`, y los de `errors-fase-7b.md` que correspondan a un código que el servidor o Rust dan de verdad. **Los códigos de Design son nombres suyos: mandan los del servidor** (`docs/architecture/cloud.md`). Un error de Design sin código real detrás no se inventa: lístalo en el informe.
6. **`docs/design/errors.md`:** pasa a él los textos de `errors-fase-7b.md` que hayas usado, con el código real.
7. **Textos:** todos con claves i18n, en `en` y `es`. Corrige lo que no sea verdad en la app.

## Límites

- Solo `app/` (Rust y UI) y `docs/`. Nada en `server/`, `spec/` ni `crates/`. Ninguna regla nueva de permisos ni de quién recibe una biblioteca: si crees que hace falta, para y dímelo.
- Fuera de este encargo (7.5.5c): la pantalla de bajada del móvil nuevo, «este dispositivo es de otra persona» (`conflicts`), la marca «tiene cuenta», la etiqueta «por la nube» y «Lo vinculó Ana». Tampoco «Si sales de la app, seguirá bajando», que no es verdad.
- La lógica va en Rust. Nunca sostengas el `MutexGuard` de la base de datos a través de un `.await`.
- Tests de Rust para todo dato nuevo que calcule Rust (el estado del interruptor, lo que hace nuevo un «ya estás en…»). Si tocas quién puede dar o quitar el poder de la nube, test del caso hostil antes y visto fallar.
- No deshagas nada con git. Rust en `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`. Actualiza `docs/architecture/cloud.md` y la línea de 7.5.5b en `docs/plans/fase-7.md` (sin marcarla). Informe corto: qué hiciste por punto, qué decidiste por tu cuenta, en qué te apartaste del diseño y por qué, qué tests viste fallar, resultado de las comprobaciones y qué no pudiste comprobar (nada se prueba en pantalla: eso lo hace el maintainer).
