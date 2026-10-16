# Estado · los cuatro hallazgos antes de abrir el repositorio

> Escrito el 16/10/2026 al parar por el límite de uso. Encargo: [encargo-hallazgos-antes-de-abrir.md](encargo-hallazgos-antes-de-abrir.md). Sin commit. Todo lo de abajo está en el árbol de trabajo.

## Qué está hecho, probado y qué falta

### CON-007 · hecho
- **Lo que era:** el orden de llegada no influye (dos tests que pasan lo documentan). La única referencia que la app aplicaba a una fila que nunca existió era `devices.member_id`, que en SQLite no tiene clave foránea (migración 0012); el servidor la salta.
- **Arreglo:** `sync::references_exist` (nuevo, `pub(crate)`), llamada desde `sync::apply_change` (lo que llega) y desde `ops::apply` (lo que escribe el propio dispositivo): si una referencia no nula apunta a una fila que no existe, se salta el cambio entero (o, en local, `MissingRow` y no se registra nada).
- **Tests:** `a_device_assigned_to_a_person_that_never_existed_stays_as_it_was` (visto rojo, ahora verde), `this_device_cannot_assign_a_device_to_a_person_that_does_not_exist_either` (verde; no lo vi rojo antes del cambio), y los dos que documentan que el orden no importa. Caso de conformidad nuevo en `spec/conformance/merge.json` (no en `permissions.json`: allí solo se ven los estados de las operaciones, no filas): «a device assigned to a person nobody created is skipped, and the device stays as it was». Visto rojo en Rust sin el arreglo (se desactivó la comprobación a mano un momento) y verde con él; en PHP pasa (`make test ARGS="--filter MergeConformanceTest"`, 149 tests). El runner de Rust ahora compara también las filas de registro que el caso cambia (antes las saltaba todas).
- **Docs:** `spec/README.md` (la regla de la referencia rota), `docs/architecture/sync.md` (fila nueva), `docs/audits/hallazgos.md` (resuelto 2026-10-16).

### SEG-040 · aceptado (D101)
- Sin arreglo posible sin cambiar el protocolo: `previous` apunta hacia atrás y nada dice al dispositivo que existe una operación posterior que deshizo la retirada.
- El test rojo ya es `an_authentic_withdrawal_is_obeyed_even_if_it_was_undone_later_and_a_hostile_peer_hides_that` y documenta lo aceptado (afirma que se borra). `hallazgos.md` y D101 no los he tocado.

### SEG-037 · hecho
- `EventsToken::LIFETIME_SECONDS = 600`; `cloud_events` a 120 por hora y cuenta (`config/packages/rate_limiter.yaml`).
- **Tests:** `CloudEventsControllerTest` (caduca a 10 min; tras el corte la renovación no lleva el tema y el token que ya se tiene caduca a 10 min; ambos vistos rojos antes del cambio), `CloudLimitsTest` a 120.
- **Docs:** `docs/architecture/cloud.md` (JWT a 10 minutos y el párrafo «Por qué diez minutos», con la prueba del hub), `docs/plans/fase-7.md` (línea del límite), hallazgos (resuelto).

### SEG-074 · hecho
- `CloudKeyVault`: formato `v1.` + XChaCha20-Poly1305, clave derivada con HMAC de un propósito, id de la biblioteca como dato asociado; lee también el formato viejo (`isLegacy`). Cambia la API: `seal($libraryId, $plain)` y `open($libraryId, $sealed)`; actualizados `CloudIdentityIssuer` y `CloudAuthor`.
- Migración `server/migrations/Version20261016120000.php` (irreversible, `abortIf` si falta `CLOUD_KEY_SECRET` o una clave no abre, todo o nada). `phpstan.neon` la añade a `scanFiles` para poder nombrarla en su test.
- **Tests:** `CloudAuthorTest::testAKeySealedForAnotherLibraryIsNoIdentityHere` (visto rojo: «The log refused the operation…» en vez de «no cloud device»; ahora verde), `CloudKeyVaultTest` reescrito (10 tests), `tests/Migration/CloudKeysMigrationTest.php` (6 tests: sella todas, lo nuevo no se toca, una que no abre aborta sin cambiar nada, sin secreto aborta, sin nada que sellar no pide secreto, irreversible).
- **Docs:** `docs/architecture/cloud.md` (formato), `docs/development/server.md` («Sellar de nuevo las claves de la nube al desplegar»: copia antes, pasos, qué pasa si se queda a medias), `docs/plans/fase-7.md` (línea en 7.9), hallazgos (resuelto).

## Tests en rojo
Ninguno. (Los dos de la fase 0 ya no lo están: CON-007 está arreglado y SEG-040 documenta lo aceptado.)

## Comprobaciones pasadas
- App: `cargo fmt --all --check` ok; `cargo clippy --all-targets -- -D warnings` ok; `cargo test`: 599 pasan, 0 fallan, 17 ignorados (+81 del otro binario); `npm run check:spec`: «spec schema agrees with 121 conformance cases and 674 merge and permission operations».
- Servidor, sueltas y verdes: `MergeConformanceTest` (149), `CloudEventsControllerTest` (19), `CloudLimitsTest` (5), `CloudAuthorTest` (10), `CloudKeyVaultTest` (10), `CloudKeysMigrationTest` (6). PHPStan: se quedó con 2 errores de tipos en el test de la migración, que arreglé (anotación `@var`); falta volver a pasarlo entero.
- **Falta:** `make check` completo, una sola vez. `make fix` (que corre php-cs-fixer, Rector, PHPStan y `make test`) terminó con **1 error en la suite completa (1849 tests)** que no he identificado: los seis ficheros de arriba pasan sueltos. Hay que volver a lanzar `make test` entero (unos minutos), ver qué test es y, si es de lo mío, arreglarlo; luego `make check`.

## El hub de Mercure (SEG-037)
Probado contra el hub de desarrollo (Mercure 1.0.3, el contenedor `mercure_kosmotheka`), desde dentro del contenedor PHP, con un script que firma un token con la clave de suscriptor **sin imprimirla** (el script está en el scratchpad de la sesión, no en el repositorio): tokens de 10, 20, 30 y 90 s y un suscriptor abierto leyendo el flujo.
- **El hub corta la conexión abierta cuando caduca su JWT**, a la caducidad o un poco antes: con 10 s, a los 2,7 y 4,4 s; con 20 s, a los 10,7 y 13,0 s; con 30 s, a los 24,6 s. Con 90 s la conexión seguía abierta a los 50 s (control).
- Por tanto acortar la vida del token protege también a quien no renueva (app modificada): lo que oye alguien a quien se le corta el acceso acaba como mucho a los 10 minutos. Está escrito en `docs/architecture/cloud.md`.
- Dato aparte: este hub (1.0.3) pide `?match=<tema>` en la suscripción, no `?topic=`; con `topic` responde 400 («unknown topic matcher query parameter»). La app ya usa lo suyo; no hay nada que cambiar.
- La clave de suscripción se leyó solo dentro del proceso PHP del contenedor de desarrollo; no está en ningún fichero ni mensaje.

## Para el maintainer (sin hacer todavía)
- Pasos de despliegue de SEG-074 (copia de la base y del `.env.local` antes): en `docs/development/server.md`.
- Una línea en `whats-new.json`: ninguno de los cuatro se nota; no hay.
