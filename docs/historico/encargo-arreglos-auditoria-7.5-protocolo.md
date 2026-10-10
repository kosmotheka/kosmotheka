# Encargo: arreglos de la auditoría del 10/10/2026, parte de protocolo

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`. Los hallazgos están en [`docs/audits/2026-10-10.md`](../audits/2026-10-10.md): léelo antes (solo leer).

## De dónde se parte y con quién se comparte el árbol

El commit `3f3087c`. **La sesión principal trabaja a la vez en el servidor**, en estos ficheros, que **no debes tocar**: `server/src/Sync/AccessCutter.php`, `CloudLibraryAccess.php`, `AccountLinker.php`, `CloudAuthor.php`, `server/src/Entity/Membership.php`, `server/src/Repository/MembershipRepository.php`, `server/src/Repository/CloudOperationRepository.php`, y los tests `AccountLinkerTest.php` y `CloudAuthorTest.php`. Si `make check` falla **en esos ficheros**, no es tuyo: no lo arregles, dímelo en el informe. Si necesitas cambiar uno, **para y pídemelo**.

Tus ficheros: `spec/`, `crates/domain`, `app/src-tauri`, y del servidor `Permissions.php`, `CatalogLibraryView.php`, `LibraryView.php`, `FakeView.php`, el validador y el lector de operaciones (`ValidOperationValidator`, `OperationReader`, `OperationPayload`), `CloudPush.php`, `CloudLink.php` y sus tests.

## Reglas

- **Sin commit.** Ni `git add`, ni `git stash`, ni cambio de rama. **Nunca deshagas un fichero con git**; para reescribir un fichero con un script, escribe a un temporal y renombra.
- Código en **inglés**, `docs/` en **español**. Contenedores para todo.
- Prohibido tocar `docs/DECISIONS.md` y `docs/audits/`.
- **El test del ataque primero, visto en rojo**; el informe copia la línea del fallo.
- **Si el trabajo crece (por ejemplo, más de una veintena de tests ajenos que cambiar), algo choca con `spec/` o un comando es denegado, se para y se cuenta.**
- Explicaciones de Rust: en el informe.

## A. SEG-029: el id de un dispositivo es un UUID en minúsculas, y se compara exacto

**Primero el test hostil** (`CloudSyncControllerTest`): Jhonny, con acceso como lector, sube un registro de dispositivo cuyo autor es **el id de la nube en mayúsculas**, con una clave suya; después, firmando como ese autor, un `member.account_id` sobre la persona de una administradora y un `device.link_account` de un dispositivo suyo de nadie hacia ella (con su clave avalada). Dime **exactamente qué pasa hoy** en cada paso (aplicada, petición o rechazada, y qué devuelve `cloudDevice()` después): de eso depende la severidad. Lo mismo en Rust, en `sync.rs`, por la red local.

**El arreglo:**
- **Regla en `spec/README.md`:** el dispositivo de un HLC es un UUID en minúsculas (`8-4-4-4-12`, hexadecimal en minúsculas). Una operación cuyo HLC no lo cumple no se lee, como cualquier HLC mal formado. Lo mismo para `library.cloud_device_id`, `revocation.device_id` y el `id` de un cambio `device`: en minúsculas o el cambio no es válido (el código de error que ya corresponda a un id mal formado; no inventes uno si hay). Con sus casos en `operation-validation.json` y uno en `permissions.json` (el del ataque, con resultado `rejected`).
- **En los dos lectores** (`Hlc::from_str` y lo que valide en Rust; `OperationPayload::HLC_PATTERN` y el validador en PHP).
- **Comparaciones exactas en el juez:** el autor contra el dispositivo de la nube (`permission.rs` y `Permissions.php`), `CatalogLibraryView::cloudDevice` (sin `LOWER` ni «la primera fila»), `fenced` y las retiradas. Haz una lista de **todos** los sitios, en Rust y en PHP, que comparan ids de dispositivo sin distinguir mayúsculas (`eq_ignore_ascii_case`, `strcasecmp`, `LOWER(`, `strtolower`) y dime en el informe qué hiciste con cada uno. Los de los ficheros de la sesión principal, solo lístalos.
- Si los tests existentes usan dispositivos que no son UUID en minúsculas y son muchos, **para y dímelo** con el número.

## B. SEG-028: una operación fechada en el futuro no entra en la nube

- **Primero el test hostil:** un lector sube una operación con HLC `9999999999999999-ffffffff-<su dispositivo>`; hoy se guarda (en petición) y lo siguiente que la nube intenta firmar falla. Reprodúcelo en un test de `CloudPush` (que se guarda) y deja el de «la nube ya no firma» descrito en el informe: el arreglo de `CloudAuthor` es de la sesión principal.
- **El arreglo, en `CloudPush`:** se rechaza (no se guarda, y retiene a las siguientes de su autor, como los demás rechazos) toda operación cuyo HLC va por delante del reloj del servidor más de lo que la app admite (`crates/domain/src/hlc.rs` ya tiene esa deriva máxima: usa el mismo valor y dilo en `spec/` si no está). Código de rechazo propio, en la lista de `spec/` y con texto en la app si los rechazos lo tienen.
- Comprueba que las fixtures y los tests con fechas fijas siguen entrando (son del pasado).

## C. SEG-034: borrar a una persona es de quien administra

- **Primero el caso** (`permissions.json`, visto en rojo): un editor crea una persona; una administradora le da rol y un dispositivo; el editor la borra. Hoy aplicado; debe quedar en petición. Y que un administrador sí puede.
- **Regla 3 de `spec/`:** un cambio `member` `delete` es solo de administradores (o de cualquiera sin roles), aunque la persona la creara quien la borra. En `permission.rs` y `Permissions.php`.
- Mira si la app tiene alguna pantalla o función que borre personas como editor y dímelo.

## Documentación

`spec/README.md`; `docs/architecture/sync.md` y `cloud.md` donde cambie; una línea en «Estado» de `docs/plans/fase-7.md`, sin marcar.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run lint && npm run check:spec`, y en `server/` `make check` (con la salvedad de los ficheros de la sesión principal).

## Informe

Qué pasa hoy en el ataque de A, paso a paso; las líneas en rojo; la lista de comparaciones de ids; salidas de las comprobaciones; lo que no coincidía y dónde se paró.
