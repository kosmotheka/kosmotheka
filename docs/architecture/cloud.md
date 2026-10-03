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
| `POST /v1/cloud/libraries/{C}/push` | `{"operations": [{"schemaVersion": n, "operation": {…}}]}` (máximo 1000) | `200 {"received": x, "known": y, "rejected": z}` |

- **`pull`** devuelve las operaciones de `C` con HLC mayor que la entrada del vector para su autor (un autor que no aparece viene entero), ordenadas por HLC, y el vector propio de `C` (el mayor HLC de cada autor; siempre un objeto, aunque esté vacío). Es el protocolo de la red local ([sync.md](sync.md)) para una sola biblioteca.
- **`push`**: `received` son las guardadas ahora, `known` las que `C` ya tenía y `rejected` las que no cumplen las reglas de abajo. Lo rechazado no se guarda ni se vuelve a enviar.
- **Acceso primero en toda ruta con `{C}`:** si la cuenta no tiene membresía en `C`, `404 {"error":"not_found"}`, igual que si `C` no existe o no es un UUID. Nunca se confirma que existe. (Un cuerpo de `pull` mal formado da `422` antes de mirar el acceso; es independiente de `C` y no revela nada.)
- **`409 {"error":"schema_version"}`** en `pull` y `push` si la versión de esquema no es la de `spec/sync.json` (`schemaVersion`). En `push` basta con que una operación traiga otra versión: un cliente de otra versión no sincroniza (D31), así que no es un rechazo operación a operación. No se guarda nada.
- `400 {"error":"invalid_request"}` si el cuerpo de `push` no es JSON de esa forma. Una operación mala **no** hace fallar el lote: se cuenta en `rejected`.
- Un dispositivo quitado (D81) o con la sesión cerrada recibe `401` del autenticador, como en el resto de la API.

## Reglas de `push`

Las aplica `App\Sync\CloudPush` (el controlador solo mapea). Todo el `push` va en una transacción que bloquea la fila de `C` (`SELECT … FOR UPDATE`): enlazar y registrar una clave comprueban antes de escribir, y dos subidas a la vez a la misma `C` (dos administradores) no se pisan.

1. **Estructura** (`App\Sync\OperationReader`): la misma validación que cualquier operación (Serializer estricto y `ValidOperation`, con los casos de `spec/conformance/`), más: `fields` de cada cambio es un objeto (no una lista) y no hay el carácter NUL, que JSONB no admite.
2. **Conocida:** misma `(C, id de operación)` → `known`, no cambia nada. Se mira antes que lo demás.
3. **Enlace** (mientras `C` no tenga `local_library_id`): solo entra la operación de enlace, un cambio de la fila `library` con `cloud_library_id` igual a `C`. Debe estar **firmada por un dispositivo de la cuenta dueña de `C`**: se busca la fila `device` por cuenta e id del HLC (nunca por el id solo) y se verifica con su clave. Si el lote la trae, se procesa la primera; fija `local_library_id` con su campo `library`. Cualquier otra operación antes del enlace se rechaza.
4. **Tras el enlace**, se rechaza la operación cuya `library` no sea la `local_library_id` de `C`, o cuya firma no valga:
   - la clave es la de `cloud_device_key` para el dispositivo del HLC **en esta `C`**;
   - si no hay ninguna, la de un cambio `device` (create) de ese mismo dispositivo dentro de la propia operación (se registra a sí mismo), que entonces queda guardada;
   - una clave distinta para un dispositivo ya registrado se rechaza, aunque la firma valga;
   - una operación sin firma se rechaza (en la nube no hay operaciones de antes de la Fase 5).
5. **Nunca se rechaza por el orden de llegada** (DAT-001): una operación más antigua que otras de su autor se guarda como cualquier otra. Dentro de un lote, el servidor ordena por HLC (con el enlace delante) para que el registro de un dispositivo se procese antes que lo que firma, sea cual sea el orden en que el cliente lo envíe. Entre `push` distintos no reordena: el cliente envía las operaciones de un autor en orden y completas, y una operación de un autor cuya clave aún no consta en `C` se rechaza.
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
