# Hallazgos de auditoría

Registro vivo. Cada identificador es fijo y no se reutiliza. El procedimiento está en `README.md`.

| ID | Estado | Severidad | Título | Dónde | Auditor | Abierto | Cerrado | Informe |
|---|---|---|---|---|---|---|---|---|
| CAL-001 | descartado | baja | Parámetro `only` siempre vacío (falso: `notices` le pasa un filtro) | `requests.rs:91` | coordinador | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CAL-002 | resuelto | baja | Siete traducciones sin uso: `cloud.outcome.*` | `app/src/i18n/locales/*.json` | facts | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
| CON-001 | abierto | media | `library-merge.json` solo lo ejecuta Rust | `spec/conformance/library-merge.json` | facts | 2026-10-03 | | [2026-10-03](2026-10-03.md) |
| CON-002 | resuelto | media | `duplicate_library` registrado y sin pantalla | `lib.rs:149` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CON-003 | resuelto | baja | 177 traducciones sin uso aparente (solo 9 de verdad; el resto, claves dinámicas) | `app/src/i18n/locales` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CON-004 | resuelto | media | La lista de géneros del JSON Schema no es la de Rust ni la de PHP (anterior a la 6.7) | `spec/operation.schema.json` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| CON-005 | resuelto | media | La app y el servidor no rechazan los mismos textos (NUL) | `server/src/Sync/OperationReader.php:44` | library-access | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
| CON-006 | resuelto | baja | Tres códigos de error de la cuenta sin texto propio | `app/src/lib/account-errors.ts` | facts | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
| COS-001 | resuelto | baja | Búsquedas que no descartan respuestas viejas | `book-form.tsx:1347`, `series-work-picker.tsx:51`, `tags-page.tsx:31` | triaje ts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| DAT-001 | resuelto | media | Si la nube rechaza una operación y acepta otra posterior del mismo autor, las copias divergen | `sync.rs:165-186` | library-access | 2026-10-03 | 2026-10-04 | [2026-10-03-2](2026-10-03-2.md) |
| DAT-002 | resuelto | crítica | `adopt_link` salta las operaciones intermedias del autor del enlace | `cloud/mod.rs:418-449` | library-access | 2026-10-03 | 2026-10-03 | [2026-10-03-3](2026-10-03-3.md) |
| DAT-003 | resuelto | crítica | Tras actualizar la app, las operaciones de la versión anterior se rechazan siempre | `sync.rs:264`, `ops.rs:351` | sync-convergence | 2026-10-03 | 2026-10-03 | [2026-10-03-3](2026-10-03-3.md) |
| DAT-004 | resuelto | media | En la app, una operación rechazada deja un hueco que el vector nunca vuelve a pedir | `sync.rs:264-269` | sync-convergence | 2026-10-03 | 2026-10-03 | [2026-10-03-3](2026-10-03-3.md) |
| DAT-005 | resuelto | media | El HLC no limita el desfase de reloj de otro dispositivo | `crates/domain/src/hlc.rs:52` | sync-convergence | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
| DAT-006 | resuelto | media | Un borrado que choca con una clave foránea sin cascada se descarta sin lápida (latente: condición antes de «añadir edición a obra existente», «quitar miembro» o borrar personas o editoriales; ver `sync.md`) | `sync.rs:458-461`, `0009_libraries.sql:83` | sync-convergence | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
| PRU-001 | resuelto | baja | Test ignorado que no se llama `live_*` | `signature.rs` | facts | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-001 | resuelto | media | La copia automática de Android se lleva las claves | `AndroidManifest.xml` | threat-model | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-002 | resuelto | media | Volver a emparejar con un `device_id` conocido suplanta a ese dispositivo | `lan/server.rs:148-153` | threat-model | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-003 | resuelto | media | CI sin `permissions:` y con actions de terceros sin fijar | `.github/workflows/ci.yml` | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-004 | resuelto | media | Falta `SECURITY.md` | raíz | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-005 | resuelto | media | Sin avisos de licencias de terceros | `web/assets/fonts`, `app/package.json` | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-006 | abierto | baja | La CSP deja cargar imágenes de cualquier origen https | `tauri.conf.json` | threat-model | 2026-10-03 | | [2026-10-03](2026-10-03.md) |
| SEG-007 | resuelto | baja | En Windows la clave TLS se escribe sin permisos propios | `lan/identity.rs:109-112` | threat-model | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-008 | abierto | baja | Dependencias con avisos sin parche, fuera del uso de la app | `Cargo.lock`, `package-lock.json` | supply-chain | 2026-10-03 | | [2026-10-03](2026-10-03.md) |
| SEG-009 | resuelto | baja | Dockerfiles de desarrollo como root o con `sudo` | `server/docker/*/Dockerfile` | supply-chain | 2026-10-03 | 2026-10-03 | [2026-10-03](2026-10-03.md) |
| SEG-010 | resuelto | crítica | Una biblioteca borrada se acepta sola y su borrado arrasa la del otro | `lan/mod.rs:454-462` | library-access | 2026-10-03 | 2026-10-03 | [2026-10-03-2](2026-10-03-2.md) |
| SEG-011 | resuelto | alta | Un aviso `revoked` falso hace borrar una biblioteca entera | `lan/mod.rs:430-435` | library-access | 2026-10-03 | 2026-10-03 | [2026-10-03-2](2026-10-03-2.md) |
| SEG-012 | resuelto | alta | El plan de la nube no dice de quién es un id de biblioteca | `docs/plans/fase-7.md` (7.2) | library-access | 2026-10-03 | 2026-10-03 (`a120fa9..6b0853b`) | [2026-10-03-2](2026-10-03-2.md) |
| SEG-013 | abierto | alta | «Puede compartir» al subir a la nube solo lo comprueba la app | `docs/plans/fase-7.md` (7.2) | library-access | 2026-10-03 | | [2026-10-03-2](2026-10-03-2.md) |
| SEG-014 | resuelto | media | Quien perdió un rol puede antedatar operaciones firmadas a antes de perderlo | `sync.rs` (`merge_in`), `permissions.rs` | coordinador (test `a_demoted_editor_cannot_backdate_a_change`) | 2026-10-03 | 2026-10-03 | [2026-10-03-2](2026-10-03-2.md) |
| SEG-015 | resuelto | alta | Lo que firma un dispositivo retirado entra en todos a través de otro que aún no lo sabe | `sync.rs`, `permissions.rs` | coordinador (test `a_revoked_device_s_edit_relayed_by_a_third_device_does_not_apply`) | 2026-10-03 | 2026-10-03 | [2026-10-03-2](2026-10-03-2.md) |
| SEG-016 | resuelto | media | Un intruso que conoce el id de un dispositivo lo ocupa en la nube con otra clave y deja fuera a su dueño | `server/src/Controller/AuthController.php` | coordinador (test `testAnIntruderCannotTakeADeviceIdBeforeItsOwner`) | 2026-10-03 | 2026-10-03 | [2026-10-03-2](2026-10-03-2.md) |
| SEG-017 | resuelto | alta | Una biblioteca que se dejó o se retiró vuelve a bajar sola de la nube | `cloud/mod.rs:375-378`, `libraries.rs:341-351` | library-access | 2026-10-03 | 2026-10-03 | [2026-10-03-3](2026-10-03-3.md) |
| SEG-018 | resuelto | alta | Una operación nueva firmada por debajo de la marca de una valla se juzga con el rol antiguo | `permissions.rs:95-110` | library-access | 2026-10-03 | 2026-10-04 (D87) | [2026-10-03-3](2026-10-03-3.md) |
| SEG-019 | abierto | media | El servidor registra la clave de un dispositivo en `C` con la primera operación que llega (parte 1 hecha el 2026-10-04: una clave registrada no cambia; queda quién registra primero, para 7.5) | `server/src/Sync/CloudPush.php:170-182` | library-access | 2026-10-03 | | [2026-10-03-3](2026-10-03-3.md) |
| SEG-020 | resuelto | media | El contador `u32` del HLC desborda con una operación remota | `crates/domain/src/hlc.rs:45-59` | sync-convergence | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
| SEG-021 | resuelto | baja | `isLink` del servidor no exige que el cambio sea de la fila de la propia biblioteca | `server/src/Sync/CloudPush.php:210-221` | library-access | 2026-10-03 | 2026-10-04 | [2026-10-03-3](2026-10-03-3.md) |
