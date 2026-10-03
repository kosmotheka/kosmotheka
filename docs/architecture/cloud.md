# La nube: bibliotecas, acceso y sincronización

> El lado del servidor del hito 7.2: las bibliotecas que una cuenta sube al servidor y las rutas para sincronizarlas. Decisión de fondo: D84 (la puerta), con D56 (firmas), D57, D73 y D76. Pensado como mapa para revisar el código: dice qué clase hace qué y qué test lo prueba. Documento vivo.

## La regla de oro (D84)

**El servidor no se fía de ningún id del cliente.** El id local de la biblioteca, los ids de operación y los ids de dispositivo que llegan en una petición se **comparan**, nunca se usan para buscar ni para decidir el acceso. El acceso sale solo de la cuenta y el dispositivo autenticados, de la tabla `membership` y del id propio de la biblioteca en la nube (`C`, `cloud_library_id`), que crea el servidor.

El caso con el que se mide todo: **Jhonny** tiene una cuenta, es hostil y conoce los ids de una familia (biblioteca, operaciones, dispositivos). No debe recibir nada de ella, ni escribir en ella, ni estorbarla.

## Tablas

Una migración (`Version20261003165457`). Todo cuelga de `cloud_library`; al borrarla se borra lo demás, y al borrar una cuenta se borran las bibliotecas que posee y sus membresías (`ON DELETE CASCADE`).

| Tabla (entidad) | Contenido | Clave |
|---|---|---|
| `cloud_library` (`CloudLibrary`) | `C` (UUID v7 que genera el servidor), dueña (`owner_id`), `local_library_id` (nulo hasta el enlace, se fija una vez), fecha | `id` |
| `membership` (`Membership`) | cuenta, `C`, `member_id` (nulo hasta 7.5), fecha. Hasta 7.5 la única fila es la de la dueña, creada con la biblioteca | `id`; único `(user_id, cloud_library_id)` |
| `cloud_operation` (`CloudOperation`) | autor (el dispositivo del HLC), `hlc`, versión de esquema, `body` JSONB (la operación tal como llegó, firma incluida), fecha de recepción | **`(cloud_library_id, operation_id)`** |
| `cloud_device_key` (`CloudDeviceKey`) | clave pública de un dispositivo **dentro del registro de esa `C`** | `(cloud_library_id, device_id)` |

Detalles que conviene saber al revisar:

- `hlc` usa la colación `C` (byte a byte): el HLC tiene ancho fijo (16 cifras, 8 hexadecimales en minúscula), así que el orden de bytes es el orden del HLC.
- `body` es JSONB, que **no conserva el orden de las claves**. No importa: la forma canónica de la firma (RFC 8785) las ordena. Sí importa que `{}` siga siendo `{}`: por eso `CloudOperation` no se lee con el ORM (el tipo JSON de Doctrine lo convertiría en `[]`) y `CloudOperationRepository` usa SQL.
- Hay índices que repiten el prefijo de una clave (`idx_cloud_operation_cloud_library`, por ejemplo): Doctrine crea el de cada clave foránea aunque la clave primaria ya lo cubra, y quitarlos haría que cada `make migration` los volviera a proponer.

## Rutas

Todas bajo `/v1/cloud/libraries`, cortafuegos `api` (cabeceras `X-Device-Id` y `Authorization: Bearer`, como el resto de `/v1`), JSON. Los errores de negocio son `{"error": "<código>"}`, los de `AccountErrorCode` (el mismo listener que la API de cuentas).

| Ruta | Cuerpo | Respuesta |
|---|---|---|
| `POST /v1/cloud/libraries` | — | `201 {"id": C}`. Crea `C` de la cuenta y su membresía |
| `GET /v1/cloud/libraries` | — | `200 [{"id": C, "localLibraryId": …\|null}]`, las de la cuenta |
| `POST /v1/cloud/libraries/{C}/pull` | `{"schemaVersion": n, "vector": {dispositivo: hlc}}` | `200 {"operations": [{"schemaVersion": n, "operation": {…}}], "vector": {…}}` |
| `POST /v1/cloud/libraries/{C}/push` | `{"schemaVersion": n, "operations": [{"schemaVersion": n, "operation": {…}}]}` (máximo 1000; el de arriba es el de la base de datos del cliente, el de cada operación el de cuando se grabó) | `200 {"received": x, "known": y, "rejected": z}` |

- **`pull`** devuelve las operaciones de `C` con HLC mayor que la entrada del vector para su autor (un autor que no aparece viene entero), ordenadas por HLC, y el vector propio de `C` (el mayor HLC de cada autor; siempre un objeto, aunque esté vacío). Es el protocolo de la red local ([sync.md](sync.md)) para una sola biblioteca.
- **`push`**: `received` son las guardadas ahora, `known` las que `C` ya tenía y `rejected` las que no cumplen las reglas de abajo. Lo rechazado no se guarda ni se vuelve a enviar.
- **Acceso primero en toda ruta con `{C}`:** si la cuenta no tiene membresía en `C`, `404 {"error":"not_found"}`, igual que si `C` no existe o no es un UUID. Nunca se confirma que existe. (Un cuerpo de `pull` mal formado da `422` antes de mirar el acceso; es independiente de `C` y no revela nada.)
- **`409 {"error":"schema_version"}`** en `pull` y `push` si la versión de esquema no es la de `spec/sync.json` (`schemaVersion`). En `push` se compara la `schemaVersion` del cuerpo, la del cliente: un cliente de otra versión no sincroniza (D31). No se guarda nada. La de cada operación no se compara con igualdad (el historial grabado antes de una actualización sigue siendo válido): se rechaza solo esa operación si es **mayor** que la del servidor, y si es menor o igual se juzga como cualquier otra. Falta el campo del cuerpo: `400 invalid_request`.
- `400 {"error":"invalid_request"}` si el cuerpo de `push` no es JSON de esa forma. Una operación mala **no** hace fallar el lote: se cuenta en `rejected`.
- Un dispositivo quitado (D81) o con la sesión cerrada recibe `401` del autenticador, como en el resto de la API.

## Reglas de `push`

Las aplica `App\Sync\CloudPush` (el controlador solo mapea). Todo el `push` va en una transacción que bloquea la fila de `C` (`SELECT … FOR UPDATE`): enlazar y registrar una clave comprueban antes de escribir, y dos subidas a la vez a la misma `C` (dos administradores) no se pisan.

1. **Estructura** (`App\Sync\OperationReader`): la misma validación que cualquier operación (Serializer estricto y `ValidOperation`, con los casos de `spec/conformance/`), más: `fields` de cada cambio es un objeto (no una lista) y ningún texto tiene el carácter NUL (U+0000), que JSONB no admite: es una regla de `spec/` (`wrong_type`), y el lector mira además los valores ya decodificados, de modo que los seis caracteres `\u0000` escritos en un título pasan. Una operación con una `schemaVersion` mayor que la del servidor también se rechaza aquí.
2. **Conocida:** misma `(C, id de operación)` → `known`, no cambia nada. Se mira antes que lo demás.
3. **Enlace** (mientras `C` no tenga `local_library_id`): solo entra la operación de enlace, un cambio de la fila `library` con `cloud_library_id` igual a `C`. Debe estar **firmada por un dispositivo de la cuenta dueña de `C`**: se busca la fila `device` por cuenta e id del HLC (nunca por el id solo) y se verifica con su clave. Si el lote la trae, se procesa la primera; fija `local_library_id` con su campo `library`. Cualquier otra operación antes del enlace se rechaza.
4. **Tras el enlace**, se rechaza la operación cuya `library` no sea la `local_library_id` de `C`, o cuya firma no valga:
   - la clave es la de `cloud_device_key` para el dispositivo del HLC **en esta `C`**;
   - si no hay ninguna, la de un cambio `device` (create) de ese mismo dispositivo dentro de la propia operación (se registra a sí mismo), que entonces queda guardada;
   - una clave distinta para un dispositivo ya registrado se rechaza, aunque la firma valga;
   - una operación sin firma se rechaza (en la nube no hay operaciones de antes de la Fase 5).
5. **Nunca se rechaza por el orden de llegada:** una operación más antigua que otras de su autor se guarda como cualquier otra. Dentro de un lote, el servidor ordena por HLC (con el enlace delante) para que el registro de un dispositivo se procese antes que lo que firma, sea cual sea el orden en que el cliente lo envíe. **Una rechazada tapa a las siguientes de su autor en el lote** (DAT-001, como `held_back` en `merge_in` de la app): una vez rechazada una operación de un autor, las posteriores de ese autor en el mismo `push` también se rechazan, para que el vector de `C` no salte un hueco que nadie volvería a enviar (la app las reenvía en el siguiente `push`, ya completas). Las que no se pueden leer cuentan por el autor y el HLC que declaran, si declaran alguno. Entre `push` distintos no reordena: el cliente envía las operaciones de un autor en orden y completas, y una operación de un autor cuya clave aún no consta en `C` se rechaza.
6. **No se juzgan roles ni permisos** (hito 7.4) ni se leen retiradas (D71) hasta 7.5: hasta entonces solo reciben los dispositivos de la cuenta dueña. Un lector con la app modificada puede subir una copia a su cuenta (SEG-013): pendiente de 7.4.

## Dónde se cumple cada costura de D84

| Costura | Dónde | Test (`tests/Controller/CloudSyncControllerTest.php`) |
|---|---|---|
| Acceso solo por la tabla de acceso; sin acceso es como no existir | `CloudLibraryAccess::require()` → `CloudLibraryRepository::findOneAccessibleBy()` (une con `membership`) | `testNoAccessIsTheSameAsNotExisting` |
| 1 · Ids de operación únicos dentro de cada `C` | clave primaria de `cloud_operation`; `CloudOperationRepository::has()` e `insertIfNew()` (`ON CONFLICT DO NOTHING`) | `testStolenOperationIdsDoNotKeepTheFamilyOut` |
| 2 · Las peticiones dicen `C`, no el id local | rutas `{C}` de `CloudController`; el servidor nunca traduce uno en otro | `testTheSameLocalIdInTwoLibrariesMakesTwoCloudLibraries` |
| 3 · `C` atada al id local de su enlace | `CloudPush::link()` (la fija una vez, `CloudLibrary::link()`) y `CloudPush::accept()` (la compara) | `testAnOperationOfAnotherLocalLibraryIsRejectedInBothAccounts`, `testWithoutALinkNothingEnters` |
| Enlace firmado por un dispositivo de la cuenta dueña | `CloudPush::link()` → `DeviceRepository::findOneOfUser()` | `testWithoutALinkNothingEnters` |
| 4 · Claves de firma dentro de cada `C` | `CloudDeviceKeyRepository::findOneInLibrary()`, `CloudPush::verifyAuthor()` | `testSomebodyElsesKeyForADeviceIdIsRefused` |
| 5 · Avisos, invitaciones y actividad cuelgan de `C` | aún no existen (7.3, 7.6, 7.7) | — |
| 6 · Un dispositivo es su id junto con su clave | `Device` (restricción única `(device_id, public_key)`), `DeviceTokenAuthenticator`; arreglado en 7.1 | `AccountControllerTest`, y `testARemovedDeviceNoLongerPulls` para las rutas de la nube |
| 7 · Rechazar solo por lo que dice la operación | `CloudPush::accept()` y `inOrder()`: ninguna comprobación mira lo ya guardado de su autor | `testAnOldOperationArrivingLateIsStoredNotRejected` |
| 8 · Dos subidas a la vez (`C1` y `C2`) | cliente y 7.5: el servidor ya soporta dos `C` con el mismo id local (costura 1 y 2); la fusión y el borrado de la sobrante no están hechos | — |

Otros tests: `testUploadingALibraryLinksItAndStoresItsHistory`, `testALinkLateInTheBatchIsProcessedFirst`, `testPullingWithAnEmptyVectorReturnsEverythingInHlcOrder`, `testPullingWithAPartialVectorReturnsOnlyWhatIsNewer`, `testPushingTheSameOperationsTwiceChangesNothing`, `testADifferentSchemaVersionIsAConflictOnPullAndPush`, `testAnEmptyObjectOfFieldsSurvivesAndAListOfFieldsIsRejected`, `testDeletingTheAccountDeletesItsCloudLibraries`. Las operaciones firmadas las construye `src/DataFixtures/SigningDevice.php` (la usan también las fixtures) con la misma forma canónica que `OperationSignature`.

## Dónde está en el código

- `src/Controller/CloudController.php`: las cuatro rutas, sin lógica.
- `src/Sync/`: `CloudLibraryAccess` (la puerta), `CloudLibraryCreator`, `CloudPull`, `CloudPush` (las reglas), `OperationReader` (estructura), `SchemaVersion` (lee `spec/sync.json`), `OperationRejectedException`.
- `src/Dto/`: `PullRequest` (con `#[MapRequestPayload]`), `PushRequest` (no: ver su comentario), `PushedOperation`, `PushResult`.
- `src/Entity/` y `src/Repository/`: las cuatro tablas. Los códigos de error nuevos (`not_found`, `schema_version`, `invalid_request`) están en `AccountErrorCode` y `AccountException`.

## En la app

El cliente de la nube está en `app/src-tauri/src/cloud/`. Habla con las rutas de arriba y reutiliza lo de la red local sin cambiarlo: `sync::version_vectors`, `operations_since` y `merge` (que juzga firmas y permisos), `people::ensure_may_share` y `ops::record`.

- `cloud/client.rs`: `HttpCloud`, sobre el cliente de cuentas (`account::client`: mismo `reqwest`, mismas cabeceras del dispositivo). Crea `C`, lista las de la cuenta, `pull` y `push` (lotes de 1000). Un `404 not_found` y un `409 schema_version` pasan a errores de tipo `cloud`; el resto (sesión cerrada, dispositivo quitado, sin conexión) conserva la traducción de la cuenta.
- `cloud/mod.rs`: el trait `CloudApi` (crear, listar, `pull`, `push`) y la lógica: `upload`, `sync_library`, `sync_all` y `status`. Los tests la ejecutan contra una nube en memoria (`cloud/tests.rs`), sin red.
- Comandos de Tauri: `upload_to_cloud(library_id)`, `sync_cloud()` (un informe por biblioteca) y `cloud_status()` (solo local: por biblioteca, si está enlazada, a qué `C`, y si podría subir). Los envoltorios están en `app/src/lib/api.ts`.
- Los errores propios son de tipo `cloud`: `not_found`, `schema_version`, `not_signed_in`, `unsigned_history`, `already_linked`. Textos en `cloud.errors.*` (`en` y `es`).
- El bloqueo de la base de datos nunca se mantiene a través de una llamada de red: se lee lo necesario, se suelta, se espera y se vuelve a bloquear para fusionar.

### Dónde vive el enlace

En la propia fila de la biblioteca: `libraries.cloud_library_id` y `libraries.cloud_server` (migración 0027). Solo los escribe la operación firmada `library.cloud` (D84, punto 3), que viaja con el registro y se aplica con las reglas de siempre. Nunca se rellenan con una tabla local ni con un valor que diga el servidor: lo que cuenta es lo que el registro fusionado aplicó. El cliente solo sincroniza una biblioteca si su fila dice `C` **y** el servidor de la cuenta actual.

### `upload(biblioteca)`

Falla con un error y sin cambiar nada si no se cumple una comprobación, en este orden:

1. Sesión iniciada (`HttpCloud::connect`: `not_signed_in`).
2. Este dispositivo puede compartirla (`people::ensure_may_share`: `CannotShare`).
3. Ninguna operación replicable sin firma (`unsigned_history`; la interfaz propondrá «Duplicar como nueva», D58).
4. No está enlazada a ninguna nube (`already_linked`).
5. El enlace lo permitirían los roles (`ops::would_allow`: con roles, solo administra): se mira antes de crear `C` para no dejar una `C` huérfana.

Después: crea `C` en el servidor, registra con `ops::record` la operación `library.cloud` (un cambio de la fila `library` con `cloud_library_id` y `cloud_server`; se vuelve a comprobar que nadie la enlazó mientras tanto) y llama a `sync_library`, que envía todo el registro, el enlace incluido.

### `sync_library(biblioteca)`

Una sesión, como `lan::client::sync_once`: `pull` con el vector del dispositivo para esa biblioteca; **se queda solo con las operaciones cuya `library` sea esa** (las demás se descartan y cuentan como rechazadas: el servidor no es de fiar para decir de qué biblioteca es una operación); `sync::merge`; y `push` de lo que a `C` le falta según el vector que devolvió (`operations_since`). El informe dice recibidas, enviadas y rechazadas (las descartadas, las que `merge` rechazó y las que rechazó la nube).

### `sync_all()`

Lista las `C` de la cuenta y, para cada una:

| Caso | Qué hace | Resultado |
|---|---|---|
| Una biblioteca local está enlazada a esa `C` en este servidor | `sync_library` | `synced` |
| Ninguna biblioteca local tiene el id local de `C` | La descarga: `pull` con vector vacío, solo las operaciones de ese id local, `merge`. Si la fila de la biblioteca no acaba diciendo `C` y este servidor, borra lo fusionado | `downloaded`, o `not_linked` |
| Hay una biblioteca local con ese id local, pero su registro aún no la enlazó a `C` | Trae de `C` lo que le falta. Solo si entre ello hay una operación de enlace **escrita por un dispositivo ya registrado en esa biblioteca** fusiona **todas** las operaciones de esa biblioteca (no solo el enlace, o el vector del autor saltaría las intermedias: DAT-002), juzgadas como cualquier otra (firma y permisos; `adopt_link`); si no hay enlace así, no fusiona nada. Si queda enlazada a `C` (otro dispositivo de la familia la subió antes de que este se enterase por la red local), sincroniza; un enlace de un lector queda en petición y no enlaza | `synced` (test `a_family_device_that_got_the_library_over_the_lan_before_the_upload_joins_it`) |
| Lo mismo, pero ningún dispositivo de esa biblioteca la enlazó a `C` (el caso de Tanaka y Jhonny: dos instalaciones con el id fijo), o la enlazó un lector | **No fusiona nada más** | `library_conflict` (tests `a_library_with_the_same_local_id_that_was_never_linked_is_not_merged` y `a_link_signed_by_a_reader_of_the_family_is_not_adopted`) |
| Este dispositivo conoce ese id local pero ya no tiene la biblioteca (la borró, la dejó o lo retiraron) | No la vuelve a bajar sola. Dejar o ser retirado apunta el id en `forgotten_libraries` (local, SEG-017); la red local no la mira y sigue pudiendo retomarla. Cómo sale de la cuenta un dispositivo retirado lo cambia el hito 7.5 | `deleted_here` (tests `a_library_left_here_is_not_downloaded_again`, `a_library_removed_by_a_revocation_is_not_downloaded_again`) |
| `C` aún no tiene enlace en el servidor (creada y sin subir) | Nada | (sin informe) |
| Biblioteca local enlazada a una `C` que la cuenta ya no lista | Nada | `missing_in_cloud` |
| Biblioteca local enlazada a una nube de otro servidor | Nada, ni una llamada | `other_server` |

### Qué test cubre cada regla (`cloud/tests.rs`)

| Regla | Test |
|---|---|
| Subir: el enlace está en el registro, la fila enlazada y la nube tiene todo el registro | `uploading_links_the_library_and_sends_its_whole_log` |
| No se sube dos veces | `uploading_twice_is_refused` |
| Historia sin firmar: rechazada, nada creado en la nube, nada registrado | `an_unsigned_history_is_not_uploaded` |
| Un lector no sube; la administradora sí | `a_reader_cannot_upload_but_the_administrator_can` |
| Un segundo dispositivo de la cuenta la descarga y converge, en los dos sentidos | `a_second_device_of_the_account_downloads_the_library_and_converges` |
| Mismo id local sin enlazar: no se fusiona nada | `a_library_with_the_same_local_id_that_was_never_linked_is_not_merged` |
| Operaciones de otra biblioteca que manda la nube: descartadas (al sincronizar y al descargar) | `operations_of_another_library_from_the_cloud_are_dropped` |
| Al adoptar un enlace llega todo lo que el autor escribió antes de subir | `adopting_a_link_brings_what_the_author_wrote_before_it` |
| Una biblioteca que llega sin su enlace se borra otra vez | `a_library_that_arrives_without_its_link_is_removed_again` |
| Nube y red local a la vez (D39), en los dos órdenes: mismo estado, sin duplicados ni pérdidas | `a_week_offline_syncs_with_the_cloud_and_then_a_lan_peer`, `a_week_offline_syncs_with_a_lan_peer_and_then_the_cloud` |
| Otro servidor: se salta sin llamar | `a_library_linked_to_another_server_is_skipped` |
| `C` que la cuenta ya no lista | `a_library_the_account_no_longer_lists_is_skipped_and_reported` |
| Códigos de error del servidor | `cloud/client.rs`: `the_servers_own_codes_become_cloud_errors`, `other_errors_keep_the_account_mapping` |

La nube de los tests (`FakeCloud`) solo hace cumplir lo que el cliente necesita probar: acceso por cuenta, operaciones únicas por `C` y una `C` que solo admite el id local de su enlace. No comprueba firmas: de eso se encarga el `merge` del cliente.
