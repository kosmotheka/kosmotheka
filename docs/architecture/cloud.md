# La nube: bibliotecas, acceso y sincronización

> El lado del servidor del hito 7.2: las bibliotecas que una cuenta sube al servidor y las rutas para sincronizarlas. Decisión de fondo: D84 (la puerta), con D56 (firmas), D57, D73 y D76. Pensado como mapa para revisar el código: dice qué clase hace qué y qué test lo prueba. Documento vivo.

## La regla de oro (D84)

**El servidor no se fía de ningún id del cliente.** El id local de la biblioteca, los ids de operación y los ids de dispositivo que llegan en una petición se **comparan**, nunca se usan para buscar ni para decidir el acceso. El acceso sale solo de la cuenta y el dispositivo autenticados, de la tabla `membership` y del id propio de la biblioteca en la nube (`C`, `cloud_library_id`), que crea el servidor.

El caso con el que se mide todo: **Jhonny** tiene una cuenta, es hostil y conoce los ids de una familia (biblioteca, operaciones, dispositivos). No debe recibir nada de ella, ni escribir en ella, ni estorbarla.

## Tablas

Una migración (`Version20261003165457`). Todo cuelga de `cloud_library`; al borrarla se borra lo demás, y al borrar una cuenta se borran las bibliotecas que posee y sus membresías (`ON DELETE CASCADE`).

| Tabla (entidad) | Contenido | Clave |
|---|---|---|
| `cloud_library` (`CloudLibrary`) | `C` (UUID v7 que genera el servidor), dueña (`owner_id`), `local_library_id` (nulo hasta el enlace, se fija una vez), `cloud_device_id`, `cloud_public_key` y `cloud_secret_key` (la identidad de la nube, 7.5.2a: nulas si el servidor no tiene secreto; la privada va cifrada y no sale nunca), `link_operation_id` (la operación que enlaza: la que `CloudLink` comprobó —firma de un dispositivo de la cuenta dueña— al enlazar, esté ya guardada o no; la primera se queda; D93), `link_signer_key` (la clave pública del dispositivo que firmó ese enlace, escrita con el id, D99; nula en las bibliotecas enlazadas antes), fecha | `id` |
| `membership` (`Membership`) | cuenta, `C`, `member_id` (nulo hasta 7.5), fecha. Hasta 7.5 la única fila es la de la dueña, creada con la biblioteca | `id`; único `(user_id, cloud_library_id)` |
| `cloud_operation` (`CloudOperation`) | autor (el dispositivo del HLC), `hlc`, versión de esquema, `body` JSONB (la operación tal como llegó, firma incluida), fecha de recepción, `previous`, `status` (7.4.5; `applied`, `request` o nulo si aún no se juzgó) y quién la subió (7.4.6: `pushed_by_user_id`, foránea a la cuenta con `ON DELETE SET NULL`, y `pushed_by_device_id`, el id del dispositivo de esa cuenta, sin foránea). Quién la firmó y quién la subió no son lo mismo: una operación puede haber llegado antes por la red local. Una operación ya guardada no cambia este dato; las guardadas antes de 7.4.6 lo tienen vacío | **`(cloud_library_id, operation_id)`** |
| `cloud_device_key` (`CloudDeviceKey`) | clave pública de un dispositivo **dentro del registro de esa `C`** | `(cloud_library_id, device_id)` |
| `invitation` (`Invitation`) | 7.6.1: invitaciones a la biblioteca (ver [Invitaciones](#invitaciones-761-d43)) | `id`; único `token_hash` |

Detalles que conviene saber al revisar:

- `hlc` usa la colación `C` (byte a byte): el HLC tiene ancho fijo (16 cifras, 8 hexadecimales en minúscula), así que el orden de bytes es el orden del HLC.
- `body` es JSONB, que **no conserva el orden de las claves**. No importa: la forma canónica de la firma (RFC 8785) las ordena. Sí importa que `{}` siga siendo `{}`: por eso `CloudOperation` no se lee con el ORM (el tipo JSON de Doctrine lo convertiría en `[]`) y `CloudOperationRepository` usa SQL.
- Hay índices que repiten el prefijo de una clave (`idx_cloud_operation_cloud_library`, por ejemplo): Doctrine crea el de cada clave foránea aunque la clave primaria ya lo cubra, y quitarlos haría que cada `make migration` los volviera a proponer.

## Rutas

Todas bajo `/v1/cloud/libraries` (menos `events`, abajo), cortafuegos `api` (cabeceras `X-Device-Id` y `Authorization: Bearer`, como el resto de `/v1`), JSON. Los errores de negocio son `{"error": "<código>"}`, los de `AccountErrorCode` (el mismo listener que la API de cuentas).

| Ruta | Cuerpo | Respuesta |
|---|---|---|
| `POST /v1/cloud/libraries` | — | `201 {"id": C}`. Crea `C` de la cuenta y su membresía |
| `GET /v1/cloud/libraries` | — | `200 [{"id": C, "localLibraryId": …\|null}]`, las de la cuenta |
| `POST /v1/cloud/libraries/{C}/pull` | `{"schemaVersion": n, "vector": {dispositivo: hlc}}` | `200 {"operations": [{"schemaVersion": n, "operation": {…}}], "vector": {…}}` |
| `GET /v1/cloud/events` | — | `200 {"token": …, "topics": […], "expiresAt": …}`; ver [Avisos al momento](#avisos-al-momento-73). Sin Mercure: `409 {"error":"events_unavailable"}` |
| `POST /v1/cloud/libraries/{C}/link` | `{"schemaVersion": n, "operation": {…}}` (un elemento de `operations` de `push`; el `schemaVersion` es el de la operación) | `200 {"localLibraryId": …}`. `422 {"error":"link_rejected"}` si no vale (ver «Enlazar») |
| `POST /v1/cloud/libraries/{C}/push` | `{"schemaVersion": n, "operations": [{"schemaVersion": n, "operation": {…}}]}` (máximo 1000; el de arriba es el de la base de datos del cliente, el de cada operación el de cuando se grabó) | `200 {"received": x, "known": y, "rejected": z}`; `403 {"error":"link_not_allowed"}` si el `push` deja el enlace en petición (D93) |

- **Enlazar es un paso propio (D93).** La app envía primero el enlace firmado a `link`; el servidor hace las mismas comprobaciones que `push` hacía con el enlace (`App\Sync\CloudLink`, compartido por las dos rutas): la operación se lee como cualquier otra, es el enlace de esta `C`, la firma un dispositivo no quitado de la cuenta dueña (por cuenta e id del HLC) y la firma vale. Entonces enlaza `C` **sin guardar la operación**, que llegará con el registro en su sitio. Repetirla con `C` ya enlazada al mismo id local responde igual y no cambia nada; con otro id local, `link_rejected`. Una operación más nueva que el servidor, que no es el enlace, de otra `C` o con mala firma da el mismo `link_rejected`; no se dice cuál de las causas fue. Hasta 7.4.6 el enlace solo entraba dentro de un lote de `push` y, como la app envía el registro en orden de HLC y en lotes de 1.000 con el enlace como lo más nuevo, una biblioteca de más de 1.000 operaciones no entraba a la primera (rechazaba todo por `not_linked` y quedaba enlazada); las versiones de la app que no llaman a la ruta siguen funcionando así, y la segunda sincronización lo guarda todo (`CloudSyncControllerTest::testWithoutTheLinkRouteTheFirstPass…`).
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
5. **Cadena de cada autor (D87):** una operación cuya `previous` no está guardada en esta `C` (ni la aceptó antes el mismo `push`) **espera**: se rechaza y tapa a las siguientes de su autor en el `push`, igual que en la app; el vector de `C` para ese autor se queda por debajo y la app la reenvía completa. Si `previous` es de otro autor o tiene un HLC no menor, está mal formada y se rechaza igual (`CloudPush::requirePrevious`). Una `previous` nula o repetida se acepta. Una operación con un HLC que ya tiene otra de esta `C` se rechaza (`hlc_taken`) y retiene igual; un índice único `(cloud_library_id, hlc)` lo asegura en la base de datos. No es un juicio por el orden de llegada: no dice que la operación sea mala, solo la aplaza hasta que esté lo que firmó su autor antes (la costura 7 sigue igual para autores distintos).
6. **Nunca se rechaza por el orden de llegada** entre autores distintos: una operación más antigua que otras de otro autor se guarda como cualquier otra. Dentro de un lote, el servidor ordena por HLC (la operación de enlace enlaza `C` al empezar, pero se guarda en su sitio, detrás de las que encadena) para que el registro de un dispositivo se procese antes que lo que firma, sea cual sea el orden en que el cliente lo envíe. **Una rechazada tapa a las siguientes de su autor en el lote** (DAT-001, como `held_back` en `merge_in` de la app): una vez rechazada una operación de un autor, las posteriores de ese autor en el mismo `push` también se rechazan, para que el vector de `C` no salte un hueco que nadie volvería a enviar (la app las reenvía en el siguiente `push`, ya completas). Las que no se pueden leer cuentan por el autor y el HLC que declaran, si declaran alguno. Entre `push` distintos no reordena: el cliente envía las operaciones de un autor en orden y completas, y una operación de un autor cuya clave aún no consta en `C` se rechaza.
7. **Lo guardado se juzga (7.4.6).** Cada operación que `accept()` guarda pasa a `CatalogJudge::take()`, en el orden de HLC del lote, y al final del lote se llama a `settle()`; todo en la transacción del `push`, con el bloqueo de `C`. Las `known` y las rechazadas no llegan al juez. Una operación que el juez deja en `request` está guardada y cuenta como `received`; `pull` sigue repartiendo todo el registro, aplicado o en petición, porque cada dispositivo juzga por su cuenta (D73). Las retiradas (D71) se leen en 7.5.
8. **El enlace no puede contradecir al registro (D93, SEG-013).** Después de `settle`, y en **cada** `push` (uno posterior con los roles puede convertir en petición un enlace que estaba aplicado), si la operación que enlaza `C` (`cloud_library.link_operation_id`, anotada por `CloudLink` al enlazar, y no la primera guardada que se le parezca: un lector con acceso podría guardar antes una operación con la forma de un enlace y bloquear la biblioteca; mientras la operación no esté guardada no tiene estado y no bloquea) está en `request`, el `push` se deshace entero con `403 link_not_allowed`: no se guarda nada de él. No se borra nada de lo que entró antes. No es un motivo de rechazo de una operación (`OperationRejection`), porque esos dependen solo de lo que dice la operación (DAT-001) y este depende del registro. Quien sube con la app modificada solo la parte sin roles obtiene una copia privada suya, sin la operación de enlace, y se acepta (D93).
9a. **Nada del futuro (SEG-028).** Una operación con un HLC más de una hora por delante del reloj del servidor (la deriva máxima de la app, `CloudPush::MAX_CLOCK_DRIFT_MILLIS`) se rechaza (`too_far_ahead`, no se guarda) y retiene a las siguientes de su autor como los demás rechazos. Sin esto, una sola operación fechada en `9999999999999999-ffffffff-…` subía el suelo de todos los HLC que la biblioteca escribiera después, la nube incluida: el contador de `CloudAuthor::hlc` pasa de ocho cifras y la nube no vuelve a firmar nada. (El tratamiento del desbordamiento en `CloudAuthor` es de la sesión principal.)

9. **El id del enlace es solo del enlace (SEG-022, SEG-024).** La operación que acaba de enlazar `C` en este `push` se dio por comprobada con la clave de la cuenta dueña, y solo esa entrada del lote, no cualquiera que lleve su id: las demás pasan por la regla 4. Una operación cuyo id es el anotado en `link_operation_id` solo se guarda si es el enlace de `C` con una firma buena de la clave que firmó el enlace, anotada con él en `link_signer_key` (`CloudLink::holds()`); si no, se rechaza (`bad_signature`). La clave y no «un dispositivo de la cuenta dueña», porque la dueña puede cambiar (D99, abajo): la ruta de enlace no guarda la operación, y la que sube después el registro puede ser otra cuenta, con la operación serializada de otra forma (la firma se comprueba sobre la forma canónica). Esa clave no es un dispositivo de la biblioteca ni entra en `cloud_device_key`: solo sirve para reconocer esa operación. Una biblioteca enlazada antes de que se anotara la clave (columna nula) se juzga como el primer enlace, por un dispositivo de su dueña de ahora, y anota la clave si vale. El primer enlace sigue exigiendo un dispositivo de la dueña de ese momento: D93 no cambia. Sin esto, otra operación con ese id ocupaba el sitio del enlace y la regla 8 miraba su estado. `refuseKeyChanges` se aplica a todas, también al enlace. Una biblioteca enlazada sin enlace anotado (las de antes de 7.4.6) lo anota cuando llega ese enlace por `push` o por la ruta.

## Dónde se cumple cada costura de D84

| Costura | Dónde | Test (`tests/Controller/CloudSyncControllerTest.php`) |
|---|---|---|
| Acceso solo por la tabla de acceso; sin acceso es como no existir | `CloudLibraryAccess::require()` → `CloudLibraryRepository::findOneAccessibleBy()` (une con `membership`) | `testNoAccessIsTheSameAsNotExisting` |
| 1 · Ids de operación únicos dentro de cada `C` | clave primaria de `cloud_operation`; `CloudOperationRepository::has()` e `insertIfNew()` (`ON CONFLICT DO NOTHING`) | `testStolenOperationIdsDoNotKeepTheFamilyOut` |
| 2 · Las peticiones dicen `C`, no el id local | rutas `{C}` de `CloudController`; el servidor nunca traduce uno en otro | `testTheSameLocalIdInTwoLibrariesMakesTwoCloudLibraries` |
| 3 · `C` atada al id local de su enlace | `CloudPush::link()` (la fija una vez, `CloudLibrary::link()`) y `CloudPush::accept()` (la compara) | `testAnOperationOfAnotherLocalLibraryIsRejectedInBothAccounts`, `testWithoutALinkNothingEnters` |
| Enlace firmado por un dispositivo de la cuenta dueña | `CloudPush::link()` → `DeviceRepository::findOneOfUser()` | `testWithoutALinkNothingEnters` |
| 4 · Claves de firma dentro de cada `C` | `CloudDeviceKeyRepository::findOneInLibrary()`, `CloudPush::verifyAuthor()` | `testSomebodyElsesKeyForADeviceIdIsRefused` |
| 5 · Avisos, invitaciones y actividad cuelgan de `C` | avisos (7.3) e invitaciones (7.6.1: `invitation.cloud_library_id`) sí; la actividad aún no existe (7.7) | `CloudEventsControllerTest`, `InvitationControllerTest` |
| 6 · Un dispositivo es su id junto con su clave | `Device` (restricción única `(device_id, public_key)`), `DeviceTokenAuthenticator`; arreglado en 7.1 | `AccountControllerTest`, y `testARemovedDeviceNoLongerPulls` para las rutas de la nube |
| 7 · Rechazar solo por lo que dice la operación | `CloudPush::accept()` y `inOrder()`: ninguna comprobación mira lo ya guardado de su autor | `testAnOldOperationArrivingLateIsStoredNotRejected` |
| 8 · Dos subidas a la vez (`C1` y `C2`) | cliente y 7.5: el servidor ya soporta dos `C` con el mismo id local (costura 1 y 2); la fusión y el borrado de la sobrante no están hechos | — |

Otros tests: `testUploadingALibraryLinksItAndStoresItsHistory`, `testALinkLateInTheBatchIsProcessedFirst`, `testPullingWithAnEmptyVectorReturnsEverythingInHlcOrder`, `testPullingWithAPartialVectorReturnsOnlyWhatIsNewer`, `testPushingTheSameOperationsTwiceChangesNothing`, `testADifferentSchemaVersionIsAConflictOnPullAndPush`, `testAnEmptyObjectOfFieldsSurvivesAndAListOfFieldsIsRejected`, `testDeletingTheAccountDeletesItsCloudLibraries`. Las operaciones firmadas las construye `src/DataFixtures/SigningDevice.php` (la usan también las fixtures) con la misma forma canónica que `OperationSignature`.

## Avisos al momento (7.3)

Decidido en D75: con la app abierta, el servidor avisa «hay algo nuevo en esta biblioteca» y la app sincroniza por el protocolo de siempre. **El aviso no lleva datos**; lo que el dispositivo puede ver lo decide siempre `pull`, con la tabla de acceso.

### El hub

- **Mercure 1.0** (`dunglas/mercure:v1.0.3`, fijada) en su propio contenedor, `mercure_kosmotheka` (172.194.0.14), porque PHP-FPM no mantiene conexiones abiertas. **No publica ningún puerto**: Symfony le publica por la red interna (`http://mercure_kosmotheka/.well-known/mercure`) y la app llega por nginx, en `<servidor>/.well-known/mercure` (`location` con `proxy_buffering off`, HTTP/1.1 y lectura de 24 h, para SSE). La ruta del token no envía la URL del hub: la app la construye.
- Suscriptores anónimos **apagados**, API de suscripciones apagada y sin CORS (el cliente es Rust, no un navegador): son los valores por defecto del hub y no se tocan. **Latido (heartbeat): 40 s** (el hub manda una línea `:` cada 40 s para mantener viva la conexión; comprobado a mano).
- **Dos claves HS256 distintas**: la del publicador (solo Symfony) y la del suscriptor (con la que se firman los tokens que se dan a los dispositivos). Un token de suscriptor nunca puede publicar. El hub confía en un emisor (`iss` = `urn:kosmotheka:server`) y exige la audiencia (`aud`) `http://mercure_kosmotheka/.well-known/mercure`, fijada en el hub con `resource_identifier` para que no dependa del `Host` de la petición; ese valor es también `MERCURE_URL`.
- Los tokens son los de **Mercure 1.0** (RFC 9068): la autorización va en `authorization_details`, no en el antiguo `mercure.subscribe`. El hub rechaza los antiguos.

### Temas

Cadenas, nunca el id local de la biblioteca (D84):

| Tema | Quién lo recibe | Cuándo se publica |
|---|---|---|
| `urn:kosmotheka:cloud-library:<C>` | Las cuentas con membresía en `C` | Tras un `push` cuya transacción se confirmó con `received > 0` |
| `urn:kosmotheka:account:<id de usuario>` | Esa cuenta («tu lista de bibliotecas cambió») | Al crear una biblioteca de la nube para esa cuenta |

Un `push` que solo trae operaciones conocidas o rechazadas no publica nada. Hoy no hay ruta para borrar una biblioteca de la nube (solo desaparece con la cuenta de su dueña, y entonces no queda a quién avisar), así que borrar no publica. Cuando 7.5 añada miembros, entrar o salir de una biblioteca avisará en el tema de la cuenta afectada.

### `GET /v1/cloud/events`

Mismo cortafuegos y cabeceras que el resto. **No lee nada del cliente**: ni cabeceras, ni consulta, ni cuerpo. Los temas salen de las filas de `membership` de la cuenta autenticada (la tabla de acceso, D84) y del id de la propia cuenta.

`200 {"token": "<JWT>", "topics": [...], "expiresAt": "2026-10-04T18:30:00Z"}`. El JWT:

- está firmado solo con la clave de suscriptor;
- lleva `iss`, `aud`, `sub` (id de usuario), `client_id`, `iat`, `jti` y `exp` (1 hora);
- su `authorization_details` tiene una sola entrada con `actions: ["subscribe"]` y un `{"match": <tema>}` por cada tema de `topics`; nada de `publish`.

Sin Mercure: `409 {"error":"events_unavailable"}` (el mismo estado que `two_factor_unavailable`), y `GET /v1/server` dice `events: false`.

### Lo que se publica

Siempre **actualizaciones privadas** con datos `{}`, **después del commit** y nunca dentro de la transacción. Si el hub falla (caído, clave mala), se anota un aviso en el registro y la petición responde como siempre: **un aviso nunca hace fallar una sincronización**.

### Sin Mercure (D18)

`MERCURE_URL` vacío en `.env.local` (así viene en `.env`): el servidor no publica nada, la ruta del token responde `events_unavailable` y `GET /v1/server` anuncia `events: false`. La app sincroniza al abrirse, al volver a primer plano y tras cada cambio. Con la URL puesta y las claves vacías el servidor sí se considera con avisos y fallará al emitir el token: las dos cosas van juntas.

### Caducidad del token y retirar el acceso

El token dura **1 hora** y la app pide uno nuevo antes de que caduque. Si una cuenta pierde el acceso a una biblioteca (o un dispositivo se quita), hasta que el token caduque ese dispositivo todavía puede enterarse de **que** la biblioteca cambió, nunca de **qué** cambió: el aviso es `{}`, y el acceso a los datos lo sigue decidiendo `pull`, que ya no responde (`404`/`401`). El siguiente token ya no lleva el tema. (Un dispositivo quitado ni siquiera puede pedir otro: `401`.)

### Motor de fusión (7.4.4)

> Aplica una operación al catálogo de una biblioteca de la nube con DBAL, sobre las tablas de [Tablas del catálogo](#tablas-del-catálogo-743-rehecho-por-d92). Desde 7.4.6 lo llama el `push` a través del juez (ver «Permisos en el servidor»); `CloudPull` no cambia. Es la segunda versión del motor: la primera usaba el ORM y la carga mostró que no bastaba (D92, «Revisión del 09/10/2026»).

- **Dónde:** `server/src/Sync/Merge/`. `CatalogMerge` es la interfaz (`apply`, `flush`, `clear`); `DbalCatalogMerge` tiene las reglas y `CatalogSql` es **la única clase con SQL**. `apply` devuelve un `ChangeOutcome` (`Written` o `Skipped`) por cada cambio.
- **Qué reglas:** las de [`spec/README.md`](../../spec/README.md) (apartados «Merging operations from several libraries» y «Merging operations within a library»), que ejecutan `spec/conformance/merge.json` y `library-merge.json` contra el motor, en cada orden de entrega, volcando al final y tras cada operación (`server/tests/Sync/Merge/MergeConformanceTest.php`). El caso de HLC repetido es del `push` y se cubre en `PermissionConformancePushTest`, y un test comprueba que la lista de casos saltados es exactamente la de los que traen `rejected`. Las reglas y su numeración son las del motor con ORM.
- **`CatalogSql`:** construye `INSERT`, `UPDATE`, `DELETE` y «la primera fila que cumple» desde el `ClassMetadata` de una entidad. Recibe una clase y nombres de **propiedad**; tabla, columnas y tipos salen del mapeo (con `quoteSingleIdentifier`), y una propiedad que no está en el mapeo lanza `UnknownPropertyException` sin ejecutar nada. Los valores van siempre como parámetros enlazados con el tipo DBAL de su columna, nunca dentro del texto SQL. Las sentencias preparadas se reutilizan por texto. No sabe de reglas de fusión. Hostil, visto en rojo: un id y un título `x'); DROP TABLE catalog_works; --` se guardan como texto (`CatalogMergeHostileTest`, `CatalogSqlTest`).
- **Nada del cliente llega a un nombre:** el motor recorre las claves de `CatalogEntity::fields()`, no las del cambio, y compara; un campo que no está en la lista salta el cambio entero. Toda búsqueda de una fila lleva su biblioteca de la nube: un id de la app nunca se resuelve fuera de ella.
- **Se juzga antes de escribir.** En PostgreSQL una sentencia que falla deja la transacción inservible, así que toda restricción de las tablas (`NOT NULL`, claves foráneas, `UNIQUE`, tipo de la columna, disparadores) tiene su regla en el motor y la base de datos es la red de debajo, nunca lo que decide. Como en la app (`apply_change`), esas comprobaciones valen solo para los campos que ganan por HLC.
- **Cola y memoria:** las reglas deciden y encolan sentencias; `flush()` las ejecuta en orden y olvida lo leído. Entre dos `flush` el motor recuerda de cada fila tocada su `key`, sus `versions`, las claves a las que apunta y, en `work_contributor`, su `role`, más las lápidas consultadas: lo necesario para decidir sobre filas que están en la cola y aún no en la base de datos. Crear es un `INSERT` con los campos ganadores y `versions`; actualizar, un `UPDATE` de los campos ganadores y de `versions` entera, por `key`. Borrar ejecuta la cola, borra la fila (la base de datos borra en cascada y anula referencias, D89), olvida todo lo leído de esa biblioteca y encola la lápida. La regla 11 (autor repetido) ejecuta la cola y pregunta a la base de datos por otra fila con el mismo trío. No usa el `EntityManager` para leer ni escribir filas (solo para los metadatos y la conexión) y no abre ni cierra transacciones.
- **Comando:** `app:catalog:rebuild <id de la biblioteca de la nube>` (o `--all`, ver «Permisos en el servidor») borra el catálogo y la contabilidad de esa biblioteca y aplica su registro (`cloud_operation`) en orden de HLC, leyéndolo por tramos y volcando cada 100 operaciones (`CatalogRebuildCommand::FLUSH_EVERY`, para que la cola no crezca sin límite), todo en una transacción. Imprime operaciones aplicadas, cambios saltados y tiempo. Reconstruir dos veces, o aplicar el registro de una en una, deja el mismo catálogo (`CatalogRebuildCommandTest`, con las fixtures).
- **Carga:** `make stress` (`CatalogLoadTest`, grupo `stress`, fuera de `make check`) reconstruye una biblioteca inventada y mide la mediana de 30 `push` de dos operaciones sobre ella. En la suite normal, una de 300 libros comprueba que aplicar por lotes y de una en una da el mismo catálogo.

### Lo que midió la carga (09/10/2026, motor con DBAL)

`make stress BOOKS=… LIMIT=…` (`NO_TRIGGERS=1` apaga los disparadores dentro de la transacción del test, `FILTER=testWhatOnePushAsks` lista las consultas de un `push`). Todo ocurre dentro de la transacción única de DAMA, sin commit; el `push` son dos operaciones en una transacción (un savepoint). Reconstruir imita el comando: `flush` cada 100 operaciones, sin `ANALYZE`.

| Libros | Operaciones | Disparadores | Reconstruir | Primeras / últimas 1.000 operaciones | `push` (mediana / peor) |
|---|---|---|---|---|---|
| 2.000 | 2.241 | sí | 14,1 s | 6,5 / 7,2 s | 12,2 / 14,5 ms |
| 2.000 | 2.241 | no | 12,6 s | 6,2 / 6,1 s | 8,2 / 14,3 ms |
| 5.000 | 5.601 | sí | 36,8 s | 7,3 / 7,2 s | 9,1 / 17,6 ms |
| 5.000 | 5.601 | no | 31,1 s | 6,3 / 5,9 s | 11,4 / 12,3 ms |
| 20.000 | 22.401 | sí | 145,6 s y 367,3 s | 7,1 / 1,2 y 7,8 / 3,7 s | 11,9 / 16,7 y 38,1 / 43,9 ms |
| 20.000 | 22.401 | no | 126,4 s, 362,9 s y 365,9 s | 6,4 / 1,1; 6,1 / 5,0 y 6,2 / 5,0 s | 9,1 / 16,3; 38,5 / 58,9 y 35,5 / 48,2 ms |

Con 2.000 y 5.000 libros el coste por operación es plano (unos 6 a 7 ms por operación). Con 20.000 hay **dos resultados**, repetidos con y sin disparadores: unos 126 a 160 s o unos 363 a 430 s. El `push` posterior, que corre en la misma transacción de prueba, sale igual de partido (9 a 12 ms o 34 a 40 ms). Aun en la tanda lenta el motor tarda menos que el del ORM con `ANALYZE` (454,6 s).

**La causa de la tanda lenta no está identificada** (09/10/2026). Lo que se sabe:

- En una tanda lenta lo que se alarga son los `INSERT` de filas con padre (`catalog_editions`, `catalog_copies`, `catalog_book_details`, `catalog_work_contributors`): de unos 0,6 ms a más de 2 ms, creciendo con la tabla. Las consultas del motor usan los índices correctos (`catalog_tombstones_pkey`, `uniq_catalog_works_library_id`).
- No son los disparadores: ocurre igual con ellos apagados.
- Se probó `SET LOCAL plan_cache_mode = force_custom_plan` al empezar la reconstrucción, por si PostgreSQL conservaba el plan de las comprobaciones de clave foránea hecho con las tablas vacías. **No lo evita:** con esa línea, cinco pasadas dieron 160,4, 155,4, 373,2, 430,3 y 387,9 s. Se retiró.
- Un script de diagnóstico con el mismo motor fuera de PHPUnit dio cuatro pasadas rápidas seguidas (207 a 224 s con el registro de sentencias encendido) y una lenta el día anterior (351 s); no se consiguió capturar una lenta con `auto_explain` activado.
- Todas las medidas son de una biblioteca en tablas por lo demás **vacías**, dentro de una transacción sin confirmar, en una base de datos de pruebas con muchas ejecuciones deshechas detrás. Ninguna dice cómo se comporta una reconstrucción cuando las tablas ya guardan las filas de otras muchas bibliotecas.

### Revisión periódica de la carga

Queda como tarea **dos veces al año** (la primera, antes de abrir la nube a más gente en 7.9; después, en abril y en octubre), y siempre que cambie el motor, el esquema del catálogo o la versión mayor de PostgreSQL:

1. `make stress` con 2.000, 5.000 y 20.000 libros, tres pasadas de cada, y comparar con la tabla de arriba.
2. **Con las tablas llenas**, que es lo que hoy no se mide: reconstruir una biblioteca de 2.000 libros cuando las tablas `catalog_*` ya tienen del orden de un millón de filas de otras bibliotecas, confirmadas y con estadísticas. Es el caso que importa si la nube crece: que el coste de una biblioteca normal no dependa del tamaño total de las tablas. Hace falta un modo nuevo de `make stress` que llene antes las tablas; no existe todavía.
3. En producción, apuntar lo que tarda `app:catalog:rebuild` (lo imprime al acabar) cada vez que se use, junto con el número de filas de `catalog_works`.
4. Comparar una pasada tras `VACUUM` con una sin él (la pista del 09/10/2026: tras `VACUUM` sale siempre la tanda rápida), y repetir la medida de reconstruir una biblioteca que ya tiene catálogo (`FILTER=testRebuildingALibraryThatHasACatalog`), que es la que deja filas muertas delante. La primera medida está en «Permisos en el servidor».
5. Si aparece la tanda lenta, activar `auto_explain` con `log_nested_statements` en esa sesión para ver el plan de la sentencia que crece; es el dato que falta.

Un `push` sencillo (cambiar el título de una obra) hace 4 sentencias, de 0,4 a 0,8 ms cada una: la lápida de la biblioteca, la lápida de la fila, la fila con sus versiones y un `UPDATE` de los campos y `versions`.

Motor anterior (ORM, 08/10/2026, con `ANALYZE` cada 500 operaciones): 36,3 s con 2.000 libros, 86,4 s con 5.000 y 454,6 s con 20.000; `push` de 27 a 35 ms.

## Dónde está en el código

- `src/Sync/`: `HubSettings` (URL, clave de suscriptor, emisor), `NoticeTopic` (los temas), `EventsToken` (el token), `SyncNotifier` (publica; no lanza nunca). `CloudPush` y `CloudLibraryCreator` lo llaman tras confirmar.
- `src/Controller/CloudEventsController.php` y el campo `events` de `ServerController`.
- `config/packages/mercure.yaml` (el hub de publicación, protocolo 1.0) y `docker/docker-compose.yaml` y `docker/nginx/kosmotheka.local.conf` (el contenedor y el proxy).
- Tests: `tests/Controller/CloudEventsControllerTest.php`, con `tests/Support/RecordingHub.php` en lugar del hub (entorno de test). Los casos hostiles: `testJhonnysTokenHoldsOnlyHisOwnTopicsWhateverIdsHeSends` (Jhonny conoce los ids de la familia y no recibe ninguno de sus temas, mande lo que mande), `testTheTokenIsSignedWithTheSubscriberKeyAndCannotPublish`, `testARemovedDeviceGetsNoToken`, `testASignedOutDeviceGetsNoToken`.

### Comprobación manual del hub real

Con `make up` y `make fixtures` (Ana tiene dos dispositivos; Jhonny, hostil, conoce los ids de la familia). Los tokens de dispositivo de las fixtures están en `server/var/fixtures/cast.json`. Por nginx, en el puerto 8095:

1. **Anónimo:** `curl -i "http://localhost:8095/.well-known/mercure?match=urn:kosmotheka:cloud-library:<C>"` responde `401`.
2. **Token de Ana:** `GET /v1/cloud/events` con las cabeceras de su dispositivo; `topics` trae su cuenta y la biblioteca García. Con ese token, `curl -N -H "Authorization: Bearer <token>" "http://localhost:8095/.well-known/mercure?match=<tema>"` mantiene la conexión abierta y muestra `:` cada 40 s.
3. **Un aviso llega:** con esa conexión abierta, haz `push` de una operación nueva desde el otro dispositivo de Ana (hace falta firmarla; en los tests lo hace `SigningDevice`), o crea otra biblioteca con `POST /v1/cloud/libraries` y llega el aviso del tema de su cuenta. Se ve `id: urn:uuid:…` y `data: {}`.
4. **Jhonny no oye a la familia:** su token (`GET /v1/cloud/events` como Jhonny) solo trae su cuenta y sus bibliotecas. Si se suscribe al tema de la familia, el hub responde `200` (así lo define el protocolo) pero la conexión no recibe nada cuando la familia hace `push`.

Resultado de la primera comprobación (04/10/2026): anónimo `401`; Ana recibió dos avisos `data: {}` (el `push` de su otro dispositivo, `received: 1`, y la biblioteca nueva); la conexión de Jhonny a `cloud-library:<García>` quedó abierta (`200`) con solo el latido; el latido llegó cada 40 s.

## Permisos en el servidor (7.4.5)

> Las reglas de la app (`crates/domain/src/permission.rs`, `app/src-tauri/src/permissions.rs` y la parte de `merge_in` que juzga) escritas en PHP, y la pieza que juzga cada operación, la aplica o la deja en petición, y vuelve a juzgar la biblioteca cuando algo llega tarde. Desde 7.4.6 lo llama `CloudPush` (reglas de `push`, 7 y 8). Es un port: manda la referencia de Rust.

- **Las reglas** (`server/src/Sync/Permission/Permissions.php`, enum `Role`): `isAllowed(operación, vista)` línea a línea como `is_allowed` y `change_allowed`, sin base de datos. `LibraryView` es la interfaz de las siete preguntas (`hasRoles`, `roleOfDevice`, `createdBySamePerson`, `requestedBySamePerson`, `memberOfDevice`, `markMember`, `fenced`). `PermissionsTest` porta los tests del módulo `tests` de `permission.rs` (nueve, no ocho), con una vista de mentira.
- **La vista contra la base de datos** (`CatalogLibraryView`, para **una** biblioteca de la nube): toda consulta lleva la biblioteca como condición (D84), de modo que un id de dispositivo, miembro, marca u operación repetido en otra biblioteca no se confunde con el suyo. **Nunca carga entidades del catálogo**: el motor escribe con DBAL por debajo del ORM, y una entidad en memoria puede estar anticuada (un `Member` cargado antes de una degradación seguiría diciendo `editor`). Todo es DQL que devuelve escalares. Una consulta escalar devuelve las columnas sin convertir (la columna JSON `versions`, la `uuid` y el `status` llegan como texto), y se decodifica en PHP. Las retiradas leen `versions` de cada `revocation` del dispositivo y comparan en PHP `versions['device_id'] < hlc`. Quien llama deja la cola del motor vacía (`CatalogMerge::flush()`) antes de preguntar.
- **Quién creó cada fila** (`row_authors`): `DbalCatalogMerge::apply()` apunta el dispositivo del HLC cuando un `create` acaba escrito y no hay autor ya (el primero cuenta). Se pregunta con `CatalogSql::first()` y se recuerda hasta el siguiente `flush`, como las lápidas. Su `entity` es el valor de `CatalogEntity` (`work`), como en `catalog_tombstones` (la app guarda el nombre de tabla).
- **Estado de cada operación** (`cloud_operation.status`, enum `OperationStatus`): `applied`, `request`, o `null` («guardada y aún sin juzgar»: las de antes de 7.4.6 hasta que se ejecuta `app:catalog:rebuild --all`). Se escribe con DQL. `cloud_operation.previous` (UUID anulable) guarda el `previous` de la operación para recorrer las cadenas de una valla sin leer el cuerpo; la migración lo rellena de las filas que ya existían.
- **Vallas** (D84, D87): `catalog_device_fences` (operación que la pone, dispositivo, `through`, HLC de la operación) y `catalog_device_fence_chains` (las operaciones que la valla aún cuenta: `through` y las que encadena por `previous`). Al apuntar una valla se calcula su cadena una vez, para que juzgar cada operación de un registro largo no la recorra. Si `through` no está en el registro todavía, la cadena queda vacía y la valla cubre todo lo anterior del dispositivo: se vuelve a juzgar cuando llega.
- **Fundación** (D97): `Permissions::isFounding` y `Sync/LibraryId` (puerto de `founding.rs`) deciden si una operación funda su biblioteca; `LibraryView::founded()` pregunta si alguien tiene rol. La tabla `catalog_device_seals` ya no existe (migración `Version20261015214624`); el sello de D94 se retiró. Un enlace de quien no es administrador sigue quedando en petición y el `push` se deshace (`403 link_not_allowed`, D93).
- **El juez** (`CatalogJudge`; no abre transacciones): `take(biblioteca, operación)` anota si llega tarde (su HLC es menor que el mayor de las operaciones ya juzgadas de la biblioteca antes de esta tanda, que se lee una vez), juzga con `Permissions`, si se permite aplica con el motor y vuelca, guarda el estado, y si quedó aplicada apunta sus vallas y mira si alguna deja bajo sospecha una operación que ya estaba. `settle(biblioteca)` rehace la biblioteca si algo llegó tarde (siempre: toda biblioteca tiene roles, D97). `rebuild(biblioteca)` vacía el catálogo y aplica el registro entero en orden de HLC, por tramos, juzgando cada operación en su momento; repite mientras alguna valla candidata quede fuera por venir de una operación que no acabó aplicada. No hay avisos de petición ni índice de búsqueda: son solo de la app.
- **Comando:** `app:catalog:rebuild` llama a `CatalogJudge::rebuild()` e imprime operaciones aplicadas y operaciones que quedan en petición. Cada reconstrucción toma el mismo bloqueo de `C` que un `push` (DAT-007), así que se puede ejecutar con el servidor en marcha: un `push` a esa biblioteca espera. `app:catalog:rebuild --all` (7.4.6) reconstruye cada biblioteca de la nube, **de una en una y cada una en su transacción**, e imprime una línea por biblioteca; si una falla lo dice, sigue con las demás y termina con error (`CatalogRebuildCommandTest`). Al desplegar 7.4.6 hay que ejecutarlo una vez, porque las operaciones guardadas antes tienen el estado vacío y las bibliotecas no tienen catálogo. Con una sola biblioteca: `app:catalog:rebuild <id>`.
- **Qué no se porta:** los avisos de petición (`request_notices`), el índice de búsqueda, las vallas locales de `ops` (`fence_changes` y `fence_mark` son de la parte que escribe de la app), y la validación de la cadena y de la firma, que son de `CloudPush`.
- **Tests:** `PermissionConformanceTest` ejecuta los 69 casos de `spec/conformance/permissions.json` en cada orden de entrega (cada operación sola, guardada, juzgada y asentada) y de nuevo tras un `rebuild`; las operaciones `rejected` no llegan al juez y un test fija los dos casos que las traen. `CatalogJudgeTest` comprueba que una operación tardía rehace la biblioteca, y la valla que llega antes que su operación (`a_fence_that_arrives_before_its_operation_is_judged_again_when_it_arrives`). `CatalogLibraryViewHostileTest` trae tres casos vistos fallar: dos bibliotecas con los mismos ids de dispositivo y miembro, retirar la petición de otro con un id de operación repetido, y un `Member` anticuado en el `EntityManager`.

### Lo que midió la carga con el juez (09/10/2026)

`make stress BOOKS=… ROLES=1` (`ROLES=1`: un administrador y un editor que escribe los libros impares y lo que borra). Cada operación del historial se guarda en el registro (sin contar el tiempo) y se juzga con `take`; `settle` cada 100. El `push` son dos operaciones guardadas y juzgadas en una transacción. Todo dentro de la transacción única de DAMA, sin `ANALYZE`.

| Libros | Operaciones | Roles | Juzgar | Primeras / últimas 1.000 operaciones | `push` (mediana / peor) |
|---|---|---|---|---|---|
| 2.000 | 2.241 | no | 47 a 50 s (tres ejecuciones) | 19 / 26 s | 44 / 52 ms |
| 2.000 | 2.241 | sí | 30,4 s | 14,2 / 13,8 s | 26,2 / 34,6 ms |
| 5.000 | 5.601 | no | 163,1 s | 21,9 / 40,2 s | 68,4 / 77,1 ms |
| 5.000 | 5.601 | sí | 163,7 s | 17,1 / 40,8 s | 70,4 / 80,5 ms |

(El motor solo, sin juez, tardó 14,1 s con 2.000 libros y 36,8 s con 5.000.) Con 5.000 libros el coste por operación crece (las últimas 1.000 operaciones tardan el doble que las primeras). `make stress BOOKS=5000 ROLES=1 FILTER=testWhatOnePushAsks` lista 12 sentencias para un `push` de una operación; dos son las caras: `SELECT MAX(hlc) … status IS NOT NULL` (3,7 ms) y el `UPDATE cloud_operation SET status` (3,3 ms) recorren el índice de `cloud_library_id` y filtran, porque dentro de la transacción de prueba las tablas no tienen estadísticas (el mismo asunto que con la reconstrucción del motor). Ninguna depende del número de libros de forma que una transacción corta con estadísticas no resolvería, pero no se ha medido fuera de la transacción de prueba. El umbral no cambia.

**Con el sello (09/10/2026, 7.4.7, 2.000 libros, `ROLES=1`, el historial inventado empieza con un sello del administrador).** La regla añade una consulta a cada operación que se juzga (¿hay un sello posterior a ella que no nombre a su autor?). Cuatro pasadas en la misma máquina: sin la consulta, 54,5 s y `push` 56,5 ms; con la consulta y sin sello en el historial, 54,4 s y 54,1 ms; con el historial sellado, 33,8 s y 36,8 ms y, repetida la misma, 49,0 s y 59,9 ms. La dispersión entre dos pasadas iguales (las dos tandas de abajo) es mucho mayor que lo que añade la regla: no se ve coste.

**Medidas de la revisión (09/10/2026, 2.000 libros, sin roles), que corrigen lo anterior:**

- Un `ANALYZE cloud_operation` después de guardar el registro y antes de juzgar no cambia nada (28,4 y 44,6 s con él; 31,1 y 46,1 s sin él): la explicación de las estadísticas del registro **no se sostiene**.
- Vuelven a salir dos tandas, como con el motor solo: unos 28 a 32 s (plano: 14 s las primeras 1.000 operaciones y 14 s las últimas) o unos 43 a 48 s (17 a 19 s las primeras y 26 s las últimas). El `push` las sigue: 19 a 27 ms o 43 a 46 ms.
- Con un `VACUUM` de la base de datos de pruebas justo antes de la pasada, tres de tres salieron rápidas (30,4, 30,4 y 32,4 s). Sin él, siete de ocho salieron lentas. Las pasadas anteriores, deshechas por DAMA, dejan decenas de miles de filas muertas, y el autovacuum trabaja sobre ellas mientras corre la siguiente (8 a 10 tablas limpiadas durante cada pasada lenta, aunque una pasada lenta no tuvo ninguna).
- Es una pista, no una causa demostrada. Importa para producción: `app:catalog:rebuild` borra el catálogo de la biblioteca y lo vuelve a escribir en la misma transacción, así que también trabaja con sus propias filas muertas delante.

**Reconstruir una biblioteca que ya tiene catálogo** (09/10/2026, `make stress BOOKS=2000 FILTER=testRebuildingALibraryThatHasACatalog`, sin roles). El test juzga la biblioteca como lo harían los `push` y después la reconstruye tres veces seguidas con `CatalogJudge::rebuild`, todo en la transacción de la prueba:

| Antes de la pasada | Juzgar como `push` | 1.ª reconstrucción | 2.ª | 3.ª |
|---|---|---|---|---|
| `VACUUM` | 30,0 s | 45,9 s | 49,2 s | 51,5 s |
| nada | 28,5 s | 54,9 s | 86,1 s | 97,7 s |
| `VACUUM` | 29,3 s | 47,0 s | 49,3 s | 52,4 s |
| nada | 29,6 s | 73,4 s | 86,3 s | 98,4 s |

- Reconstruir sobre un catálogo ya escrito cuesta entre 1,5 y 2,5 veces lo que costó escribirlo, y cada reconstrucción en la misma transacción cuesta más que la anterior. Con la base de datos recién limpiada la subida es pequeña (unos 3 s por vuelta); sin limpiar es grande.
- Esto apoya la pista de las filas muertas: lo que frena es el trabajo delante de filas borradas que nadie ha limpiado todavía. Sigue sin estar separado cuánto de la primera reconstrucción es eso y cuánto es lo que la reconstrucción hace de más (lee el registro por tramos, valida cada operación al leerla y recorre el registro una vez más para buscar las vallas).
- En producción cada reconstrucción va en su propia transacción y el autovacuum limpia entre una y otra, así que el caso real es la primera columna: de 46 a 73 s para 2.000 libros. Es una orden de mantenimiento, no algo que espere una persona, pero en 7.4.6 hay que reconstruir cada biblioteca una vez al enchufar el juez: se hace de una en una.
- La variante con roles (`ROLES=1`) no se puede reconstruir todavía: el lector de operaciones rechaza la biblioteca inventada (comprobado). La causa más probable, sin comprobar, es que crea los dos dispositivos en una sola operación y solo el propio dispositivo puede crearse. Hay que rehacerla antes de medirla.

### Lo que midió el `push` de verdad (09/10/2026, 7.4.6)

`make stress BOOKS=2000 FILTER=testALargeLibrary`, sin roles: el historial se juzga como antes, pero los 30 `push` de dos operaciones pasan ahora por `CloudPush` (firma, clave y cadena incluidas) en vez de llamar al juez. Dos pasadas seguidas, y una con `ANALYZE` de las tablas tras cargar (`make stress ANALYZE=1`; `VACUUM` no se puede ejecutar dentro de la transacción de la prueba):

| Pasada | Juzgar el historial | `push` de 2 operaciones (mediana · peor) |
|---|---|---|
| 1 | 31,0 s | 33,1 ms · 49,7 ms |
| 2 | 33,2 s | 56,4 ms · 74,8 ms |
| con `ANALYZE` | 50,0 s | 44,5 ms · 66,8 ms |

Las pasadas iguales siguen saliendo en dos tandas, como en las medidas anteriores, sin causa identificada (ver «Revisión periódica de la carga»); `ANALYZE` no mejora nada medible. Antes de enchufar el juez, el `push` de la prueba (2 operaciones) tardaba 44–47 ms sin roles.

## La identidad de la nube (7.5.2a, D76, D95)

Cada biblioteca de la nube tiene un dispositivo propio, «la nube»: un id, un par de claves Ed25519 y la capacidad de firmar operaciones en esa biblioteca. En 7.5.2a el servidor aprende a tenerlo y a firmar; **todavía nadie le pide que firme** (eso es 7.5.2b y 7.5.3: escribir `member.account_id` y vincular dispositivos).

- **El id sale de la clave.** `cloud_device_id` no es un UUID al azar: es `SHA-256(clave pública de 32 bytes)`, los 16 primeros bytes con el nibble de versión a 8 y los bits de variante a `10`, en minúsculas (UUID versión 8; `App\Sync\CloudDeviceId`, `kosmotheka_domain::cloud_device_id`; la fórmula exacta y un vector están en la regla 12 de `spec/README.md`). Por qué: la fila `device` de la nube llega a los dispositivos por el registro, y una fila `device` solo la puede crear ese mismo dispositivo (`other_device`). Si el id fuera libre, un miembro con la app modificada registraría por la red local el id de la nube con una clave suya, y los dispositivos que lo recibieran antes que el registro de verdad le darían el poder de la nube. Con el id derivado, ese registro falso no cumple «el id sale de la clave» y no tiene poder en ningún dispositivo. Lo comprueban los dos jueces (`cloud_device()` solo devuelve el id nombrado si la clave registrada lo da).
- **Secreto del servidor:** `CLOUD_KEY_SECRET` (32 bytes en base64, ver `docs/development/server.md`). `App\Security\CloudKeyVault` cifra con `sodium_crypto_secretbox` (nonce aleatorio por valor, delante del texto cifrado); abrir con otro secreto o con un valor dañado lanza `CloudKeyVaultException`, nunca bytes que parezcan una clave. Vacío, la nube es opcional (D18): las bibliotecas no tienen identidad, los campos de la API salen nulos y nada falla.
- **Cuándo se genera** (`App\Sync\CloudIdentityIssuer`): al crear la biblioteca (`CloudLibraryCreator`) y, para las que ya existían, al listarlas o enlazarlas, dentro de una transacción con el bloqueo de la biblioteca. Una vez generada no cambia nunca. A la vez se guarda su fila `cloud_device_key`: **su clave queda registrada en la biblioteca desde el primer momento**, así que la regla «un dispositivo registrado no cambia de clave» impide que nadie ocupe su id o firme en su nombre. (`CloudLink::check` aplica también esa regla a la operación de enlace: un enlace que nombra a la nube con otra clave no enlaza la biblioteca.)
- **API:** `POST /v1/cloud/libraries` devuelve `id`, `cloudDeviceId` y `cloudPublicKey`; `GET`, además `localLibraryId`. La clave privada no sale, ni cifrada.
- **La nube se registra a sí misma** (`CloudAuthor::registerIfNamed`, llamada por `CloudPush::push` al final, dentro de su transacción y su bloqueo, después de `settle` y de `requireLinkAllowed`): si el catálogo de la biblioteca nombra en `cloud_device_id` el id de su nube y esta aún no tiene ninguna operación en el registro, escribe un `device.register` (create de su fila con su clave y como nombre el host del servidor) con su id en el HLC. Así, en una subida por lotes se escribe tras el lote que trae el enlace, y su HLC es el mayor del registro (no llega «tarde»). No está en `linkFrom`, que no guarda operaciones. Si la administradora quita a la nube y la vuelve a nombrar, no se registra otra vez. No cuenta en la respuesta del `push`.
- **`App\Sync\CloudAuthor::author(biblioteca, intención, cambios)`** firma una operación suya y la entrega a `CloudPush::applyLocked`, **el mismo camino que un `push`** (lectura con `OperationReader`, firma contra la clave registrada, cadena, HLC único, `take` y `settle` del juez): ningún atajo, si una regla del `push` cambia vale para lo que firma la nube. Exige una transacción abierta (toma el bloqueo dentro) y una biblioteca enlazada con identidad; si no, `CloudAuthorException`. `previous` es la operación anterior de la nube en esa biblioteca y el HLC es la hora del servidor, nunca menor que la más nueva del registro (con el reloj parado o hacia atrás sube el contador). `pushed_by_*` queda vacío. Lo que **puede** hacer lo decide el juez, no esta clase: sin ser la nombrada, lo que firma queda en petición. La firma es `OperationSigner`, que también usan las fixtures y los tests; un test comprueba que firma byte a byte lo que `spec/conformance/signatures.json` guarda (Ed25519 es determinista), que es lo que la app verifica.
- **App:** `upload` comprueba que el id que da la API sale de la clave que da la API; si es así lo escribe en el cambio de la fila `library` de la operación de enlace (`cloud_device_id`, la casilla de D76 marcada por defecto); si no, o si el servidor no da identidad, sube como siempre. No crea ninguna fila `device` (solo ese dispositivo crea la suya). Quitar o cambiar la nube (`people::remove_cloud_device`, `name_cloud_device`) pone la valla del dispositivo de la nube **siempre**, también si aún no está registrado en esa biblioteca (marca vacía): si su registro y algo firmado llegan después, fechados antes de la retirada, quedan en petición.

## El enlace de la cuenta con su persona (7.5.2b-2, D76, D96)

`POST /v1/cloud/libraries/claim`, autenticada con el dispositivo actual, **sin cuerpo** (si llega alguno se ignora: ningún id del cliente decide nada, D84). Sin límite de peticiones propio, como el resto de las rutas de la nube (no tienen ninguno; el `RequestLimiter` es de las cuentas). La hace `App\Sync\AccountLinker`. La app la llama al principio de cada `sync_all` (que es también lo que corre al iniciar sesión), antes de listar; si falla, la sesión sigue con lo que haya.

**Candidatas**, en una consulta DQL: las bibliotecas **enlazadas** con una fila `device` de id `D` (sin distinguir mayúsculas), `public_key` y `vouched_key` iguales a la clave `K` del dispositivo autenticado, y persona (índice `idx_catalog_devices_vouched_key`). Una asignación sin clave avalada **no** enlaza, aunque el juez la siga creyendo para el rol (D96).

```
SELECT l FROM CloudLibrary l
 WHERE l.localLibraryId IS NOT NULL
   AND EXISTS (SELECT 1 FROM Catalog\Device d
                WHERE d.cloudLibrary = l AND d.vouchedKey = :key AND d.publicKey = :key
                  AND LOWER(d.id) = :device AND d.member IS NOT NULL)
 ORDER BY l.createdAt
```

**Por cada candidata**, con el bloqueo de la biblioteca (el de `push`) y releyendo dentro de él, siendo `M` la persona del dispositivo:

| Situación | Qué se hace |
|---|---|
| Hay una retirada (`revocation`) para `D` | Nada |
| Otra cuenta ya está enlazada con `M` | Nada (una persona, una cuenta; `UNIQUE (cloud_library_id, member_id)`) |
| La cuenta no tiene fila de `membership` | Se crea, con `member_id` = `M` |
| Tiene fila con `member_id` vacío (quien subió la biblioteca) | Se rellena (`Membership::linkTo`, una vez) |
| Tiene fila con `M` | Nada |
| Tiene fila con otra persona | Nada; la biblioteca sale en `conflicts` |

Respuesta `200 {"linked": [...], "conflicts": [...]}`: solo ids de bibliotecas a las que la cuenta tiene acceso al terminar; idempotente.

**Qué da el enlace y qué no.** Da acceso (`pull`, `push`) a la biblioteca; **no da rol**: el rol es el de la persona en el catálogo, y lo que sube un lector sigue siendo una petición. No vincula dispositivos nuevos (7.5.3), no baja una biblioteca que el dispositivo no tiene (7.5.3), no quita el acceso (7.5.4) ni invita (7.6).

**Anotar la cuenta en la biblioteca.** Con la fila enlazada con `M` y `member.account_id` de `M` distinto del `account_id` de la cuenta, la nube escribe `member.link_account` (un `update` de `member` con solo `account_id`) con `CloudAuthor::author`, y avisa a los demás dispositivos como un `push`. **Solo si tiene el poder** en esa biblioteca en ese momento (la biblioteca la nombra en `cloud_device_id` y está registrada con la clave de la que sale su id: `CatalogLibraryView::cloudDevice()`); si no, el acceso se da y no se escribe nada. Para no llenar el registro de peticiones, si el registro ya tiene una petición de la nube con ese mismo `member.link_account` (misma persona, misma cuenta: `CloudOperationRepository::hasRefusedAccountLink`, contención `@>` sobre el `body` jsonb) no se vuelve a escribir.

## El móvil nuevo: el segundo paso de `claim` (7.5.3, D76, D96)

`claim` hace ahora dos cosas, una detrás de otra, y sigue sin recibir ningún id (D84): el dispositivo es el autenticado (id `D`, clave `K`, cuenta `U`).

1. **Enlazar la cuenta con su persona** (7.5.2b-2, arriba).
2. **Vincular el dispositivo.** En cada biblioteca donde `U` tiene `membership` **con persona** `M` (una consulta, no una por biblioteca del servidor), con el bloqueo de la biblioteca y releyendo dentro, la nube escribe con `CloudAuthor` un `update` de la fila `device` de `D` con `member_id` = `M` y `vouched_key` = `K` (intención `device.link_account`) **solo si**:

| Condición | Por qué |
|---|---|
| La nube tiene el poder ahí ahora (`cloudWithPower`: nombrada y registrada con la clave de la que sale su id) | Sin poder sería una petición |
| `M` existe y su `account_id` es el `accountId` de `U` | Lo pide la regla 12; si aún no lo es, se anota y se vincula en la misma llamada |
| El catálogo tiene la fila `device` de `D` con `public_key` = `K` | El móvil se ha registrado él mismo; con otra clave, alguien ocupó su id: nada |
| Su `member_id` está vacío | La nube no reasigna |
| Su `trusted_through` es **nulo** y no hay retirada para `D` | A un dispositivo al que se le quitó su persona o el acceso solo lo devuelve un administrador. Se pide nulo y no «vacío»: un `trusted_through` de texto vacío es una valla con marca vacía, no ausencia de valla |
| No hay ya una petición de la nube con ese mismo cambio (`hasRefusedDeviceLink`) | Sin bucles |

La respuesta gana `devices`: ids de las bibliotecas de la nube donde el dispositivo acaba de quedar vinculado. Idempotente.

**El orden de una sesión en un móvil nuevo** (`cloud::sync_all`): `claim`; listar; bajar la biblioteca que no tiene (`download`), que ahora **registra al dispositivo** en ella (una operación suya que solo crea su fila `device`, `ops::ensure_registered`) y la sube con el resto; `claim` otra vez, solo si en la sesión se subió algo; si devuelve `devices`, se vuelve a sincronizar esa biblioteca para traer lo que la nube acaba de escribir. Un fallo de cualquiera de los dos `claim` no rompe la sesión. Al terminar **una** sesión el móvil tiene su persona y su rol.

## La nube corta el acceso (7.5.4b, D98, D71)

La nube deja de entregar una biblioteca, **al momento**, en dos casos, y los dos se leen del catálogo que el servidor guarda en cada petición, no de un aviso que pudiera perderse. A quién se corta sale siempre del dispositivo autenticado, de `membership` y del catálogo; nunca de un id del cliente (D84). Lo hace `App\Sync\AccessCutter`, y la puerta es `CloudLibraryAccess`.

- **Corte A, el dispositivo.** Hay una retirada (`revocation`) aplicada en esa biblioteca que nombra al dispositivo autenticado (id `D`), y la fila `device` de `D` en el catálogo tiene su misma clave. Un dispositivo con el *id* de uno retirado y otra clave es otro dispositivo y no se corta. La cuenta conserva su acceso desde los demás dispositivos.
- **Corte B, la cuenta.** Su `membership` apunta a una persona que tiene `access_withdrawn` o que ya no existe. La fila queda marcada (`withdrawnAt`, y sin persona) y **ya no da acceso**, aunque luego alguien quite la marca a la persona: volver es enlazarse otra vez (abajo). Se marca al final del `push` que trae la retirada (dentro de su transacción, después de `settle`, así que también cuando la retirada llega tarde y la biblioteca se juzga de nuevo) y, por si acaso, en la propia puerta. Si la nube tiene el poder (`CloudAuthor::power`), vacía además `member.account_id` de esa persona (`member.unlink_account`, firmado y aplicado; una sola vez, sin bucles: no repite si ya hay una petición suya igual en el registro). Sin poder, no escribe nada y el corte vale igual. Se publica un aviso en el tema de la **cuenta** afectada para que sus dispositivos conectados sincronicen ya.

**La puerta.**

| Situación | Respuesta |
|---|---|
| Sin fila de `membership` | `404`, como siempre: no revela qué bibliotecas existen |
| Corte A o B | `403 access_withdrawn` con `proofs` |
| Lo demás | Como siempre |

`proofs` son las operaciones firmadas, tal como están guardadas, que crearon las retiradas **aplicadas** que nombran al dispositivo en esa biblioteca; nada más. Si ninguna lo nombra (un dispositivo de la cuenta echada que no estaba en la biblioteca) va vacía. El `403` no depende de nada que el cliente mande. `GET /v1/cloud/libraries` responde ahora `{"libraries": [...], "withdrawn": [ids]}`: no lista las cortadas, pero da sus ids (solo de bibliotecas en las que la cuenta *tenía* fila) para que la app pida la prueba; la app acepta también la lista sola de un servidor antiguo. Los temas de `GET /v1/cloud/events` salen solo de bibliotecas con acceso.

**Volver (D98).** Quitar la marca a la persona no da nada. Quien administra vincula de nuevo un dispositivo suyo con la clave avalada; `claim` encuentra la fila marcada y la enlaza otra vez por el camino de siempre (dispositivo con persona, clave avalada igual a la suya, sin retirada, persona sin la marca), con lo que desaparece `withdrawnAt`. `AccountLinker` no enlaza con una persona que tiene `access_withdrawn` ni le vincula dispositivos.

**En la app.** `CloudApi::list_libraries` devuelve `Listing` (bibliotecas y cortadas) y el cliente conserva las `proofs` del `403` (`Error::AccessWithdrawn`). Al recibir el corte de una biblioteca que tiene, la app **solo la borra si comprueba la prueba ella misma**, con el mismo código que la red local (`lan::remove_if_proved`, el de `SharingNotices`, SEG-011): se fusionan las operaciones que son de esa biblioteca y crean una retirada de este dispositivo, se juzgan como cualquiera, y solo si queda una retirada aplicada se borra (`Outcome::Removed`). Sin prueba válida la biblioteca se queda entera, deja de sincronizarse con la nube y el informe de la sesión lo dice (`Outcome::AccessWithdrawn`): un servidor que miente no puede hacer perder una biblioteca. Una borrada así no vuelve a bajar sola (`forgotten_libraries`, SEG-017). Una borrada así no vuelve a bajar sola (`forgotten_libraries`, SEG-017).

**Cómo encadenan las pruebas.** La retirada espera a su `previous` como cualquier operación (D87), y lo normal es que quien administra haya seguido trabajando sin que el dispositivo cortado lo recibiera. Por eso, en un `pull` (que ya trae el vector del cliente), el `403` lleva, además de cada operación que crea la retirada de ese dispositivo, las operaciones **anteriores del mismo autor** que al cliente le faltan según su vector (las de ese autor con HLC mayor que el del vector y menor que el de la retirada), en orden de HLC: solo de ese autor, solo hasta la retirada, nada fechado después, nada de otros autores. El vector es del cliente, así que solo decide cuántas se envían, nunca el acceso ni de qué biblioteca: un vector falso recibe, como mucho, la cadena entera de ese autor hasta una retirada que es anterior al corte. En `push`, `link` y donde no hay vector, las pruebas son solo las retiradas; la app, cuando recibe el `403` por otra ruta, pide la prueba con un `pull` con su vector. La app no cambia de criterio: `lan::remove_if_proved` fusiona lo de esa biblioteca que crea una retirada de este dispositivo y lo anterior de quien la firmó (nada de otra biblioteca ni de otros autores), lo juzga como cualquier operación (firma y permisos) y solo borra si queda una retirada aplicada que nombra a este dispositivo. **Fallo seguro:** si el rol de quien retira depende de una operación de *otro* autor que al dispositivo le falta, la retirada no se aplica y la biblioteca se queda (`Outcome::AccessWithdrawn`). Por la red local no cambia lo que se envía.

**Cuando echan a quien subió la biblioteca.** Su fila de `membership` sin persona no se ve afectada (no hay a quién retirar); si ya estaba enlazada con su persona, se le corta como a cualquiera. La biblioteca de la nube **sigue existiendo** (`cloud_library.owner` sigue siendo su cuenta) y los demás miembros siguen entrando. Qué pasa si esa cuenta se borra: ver «Quién es la dueña» (D99).

### Arreglos tras la auditoría del 10/10/2026

- **Las pruebas son para el dispositivo que la retirada nombra, por id y clave (SEG-031).** `AccessCutter::proofs` no devuelve nada si `deviceCut` no es cierto. Una cuenta cortada que entra con el id de un dispositivo ajeno retirado después, y una clave suya, recibe el `403` sin pruebas: antes recibía la retirada con todo lo que su autor escribió tras cortarla. Los demás dispositivos de una cuenta echada tampoco reciben pruebas; su biblioteca se queda y deja de sincronizarse.
- **El servidor da su persona a la fila que no la tiene, sin esperar a `claim` (SEG-030).** Quien sube nace sin persona. `AccessCutter::fill`, al asentar cada `push` y en la puerta, se la pone desde los dispositivos de esa cuenta (la tabla `device` del servidor, id y clave) que el catálogo tiene con esa misma clave y una persona con acceso; si hay clave avalada tiene que ser esa. Una persona o ninguna: dos dispositivos de dos personas no rellenan nada. Así, echar a quien subió la biblioteca corta también su cuenta aunque su app nunca reclamara.
- **En una biblioteca con roles, una cuenta que no es nadie no lee (SEG-030).** Una fila sin persona, tras intentar rellenarla, recibe `403` `access_withdrawn` sin pruebas en `pull`, no se lista (va en `withdrawn`) y no tiene tema en `events`. No es un corte: no lleva `withdrawnAt` y vuelve a entrar en cuanto un dispositivo suyo tiene persona. **Sí puede subir** (`link` y `push` pasan `toWrite: true`): lo que sube se juzga como de nadie, y una subida en varias tandas tiene roles en el catálogo antes de la tanda que dice de quién es el dispositivo que sube. Sin roles, nada cambia.
- **Una cuenta cortada no sube nada (D98, SEG-032 aceptado).** Echar es definitivo para el echado, también entre dos administradores: gana quien llega antes a la nube.
- **La puerta avisa cuando corta (SEG-038)**, como el `push`: el tema de la biblioteca y el de cada cuenta cortada.
- **`CloudAuthor` (SEG-028, SEG-039):** con el contador del HLC lleno pasa al milisegundo siguiente; la clave se borra de memoria también si firmar falla; una clave sellada con otro secreto es «sin identidad», no un `500`.
- **El inicio de sesión exige el id del dispositivo en minúsculas (SEG-029)**, como el HLC de las operaciones.

## Quién es la dueña de una biblioteca de la nube (D99)

`cloud_library.owner` es la cuenta que la subió. **Solo cuenta para el primer enlace** (D93) **y para su borrado**; el acceso de todos, el de la dueña incluida, sale de `membership`. Antes la base la borraba en cascada con la cuenta: si quien la subió borraba su cuenta, la biblioteca desaparecía de la nube para todos (SEG-061).

Ahora `owner_id` es `ON DELETE RESTRICT` (migración `Version20261015193814`): ninguna biblioteca se borra por cascada sin pasar por la regla. Al borrar una cuenta (`DELETE /v1/account`, `App\Sync\CloudOwnership`, dentro de la misma transacción y bajo el cerrojo de cada biblioteca, como las demás escrituras que cambian quién la tiene), cada biblioteca de la que es dueña:

1. pasa a otra cuenta con una **membresía activa** (`withdrawnAt` nulo y su persona sin `access_withdrawn`, D98): primero una cuya persona tiene el rol `admin`; si no hay, cualquier otra; entre varias del mismo grupo, la de membresía más antigua (y su id), para que el resultado no dependa del orden de la base;
2. si no queda nadie con un sitio, se borra con la cuenta, explícitamente (y la base borra en cascada lo suyo, como antes).

La biblioteca queda igual: no se toca el registro ni el catálogo, así que `app:catalog:rebuild` da lo mismo. El cambio se anota en el registro de actividad como `library_owner_changed` (la biblioteca, la cuenta que se va y la que recibe, por su id; D66). La app avisa al borrar la cuenta de que las bibliotecas compartidas siguen para los demás y las que solo eran suyas se borran de la nube. Tests: `CloudOwnerTransferTest`, y los de enlace de `CloudSyncControllerTest` (`ALinkMadeByTheRoute…`, `TheSameLinkWithItsKeysInAnotherOrder…`).

## Invitaciones (7.6.1, D43)

> El lado del servidor de «invitar con un enlace». Especificación: D43 con su enmienda del 10/10/2026. La app (7.6.2 y 7.6.3) aún no llama a estas rutas. Código en `src/Sync/Invitation/`; tests en `tests/Controller/InvitationControllerTest.php`.

### Qué es una invitación

Una fila de `invitation` que cuelga de la biblioteca de la nube `C` (D84) y nombra a **una persona** (`member`) de esa biblioteca. Dice a quién se invita y por qué vía; **no lleva rol**: quien entra es esa persona, con el rol que ya tiene en el catálogo. El servidor **no crea personas**: la persona nueva la ha creado antes, como lectora, una operación firmada por el dispositivo de quien administra.

| Campo | Qué guarda |
|---|---|
| `cloud_library_id` | La biblioteca de la nube. Al borrarla se borran sus invitaciones |
| `member_id` | La persona (id del cliente, que el servidor solo **compara** con el catálogo de esa biblioteca) |
| `kind` | `email` o `link` |
| `token_hash` | SHA-256 del token. El token son 256 bits aleatorios (43 caracteres en base64 seguro para URL) y no se guarda; al ser aleatorio no necesita sal |
| `email_hash` | Solo en `email`: HMAC-SHA-256 del correo normalizado (minúsculas y sin espacios), con una clave derivada de `APP_SECRET` solo para esto. Un correo se adivina; un hash sin clave se probaría contra correos conocidos. Nunca el correo |
| `created_by_id` | La cuenta que invita. Si se borra la cuenta, se borran sus invitaciones |
| `created_at`, `expires_at` | Caduca a las **24 horas** |
| `accepted_at`, `accepted_by_id` | Cuándo y qué cuenta (`SET NULL` si se borra la cuenta) |
| `revoked_at` | Si quien administra la anuló |

«Un solo uso» y «24 horas» son reglas del código (`InvitationManager::LIFETIME`, `InvitationAcceptor`), no de la tabla: una invitación de varios usos o sin caducidad, que D43 deja como idea, no pide cambiar el esquema. No se construye.

### Los dos tipos

| | `email` | `link` |
|---|---|---|
| Quién la abre | Solo una cuenta cuyo correo (verificado al registrarse, D77) es ese | Quien la abra primero |
| Para qué persona | Nueva o ya en la biblioteca (editora o lectora): **conserva su rol** | Solo una persona **nueva y lectora** |
| Cómo llega | El servidor manda el correo con el enlace; la respuesta de crear **no** lo trae | La respuesta de crear lo trae, una vez; el servidor no manda nada |

**Nunca** se invita a una persona que administra (con un correo robado se echaría a los demás, D98), que tiene el acceso retirado, que ya tiene cuenta (`member.account_id` o una fila de `membership` con ella) o que no existe en esa biblioteca.

**«Persona nueva»** (decisión del hito, a confirmar): una lectora sin nada suyo en la biblioteca: **ningún dispositivo** asignado, **ningún ejemplar** suyo y **ninguna marca**. Es lo que D43 pide con «un enlace filtrado no deja a un extraño ocupar el sitio de alguien que ya tiene libros y préstamos». Si se añaden cosas que pertenecen a una persona (préstamos, por ejemplo), hay que añadirlas a `InvitationRules::isNew`.

### Quién invita

Solo quien **administra** la biblioteca, y se lee del servidor, no del cliente (`InvitationRules::administers`): la cuenta tiene una fila de `membership` con persona y sin retirar, y esa persona tiene en el catálogo el rol `admin` y no tiene el acceso retirado. La misma pregunta se hace **otra vez al aceptar**, sobre la cuenta que invitó: si ya no administra, o la echaron, o está bloqueada, la invitación no vale.

Para crear una invitación hace falta además que la biblioteca esté **lista** (`InvitationRules::isReady`): enlazada, **con personas y roles** y con la **nube con poder para vincular dispositivos** (`CloudAuthor::power`: la biblioteca la nombra en `cloud_device_id` y está registrada con la clave de la que sale su id). Se comprueba también al aceptar.

### Rutas

Todas bajo `/v1/cloud`. Los errores son `{"error": "<código>"}` como el resto.

| Ruta | Sesión | Cuerpo | Respuesta |
|---|---|---|---|
| `POST /v1/cloud/libraries/{C}/invitations` | sí | `{"memberId": "<uuid>", "kind": "email"\|"link", "email": "…"}` (`email` solo y siempre con `kind: email`; nada más: un `email` con `kind: link` da `400 invalid_request`, no se ignora, SEG-047) | `201 {"id", "kind", "memberId", "expiresAt", "link"}`; `link` es `null` en `email` |
| `GET /v1/cloud/libraries/{C}/invitations` | sí, quien administra | — | `200 {"invitations": [{"id","kind","memberId","state","createdAt","expiresAt","acceptedAt"}]}`. `state`: `pending`, `accepted`, `revoked`, `expired`. Nunca el token ni el correo. Las caducadas se ven una semana (la purga) |
| `DELETE /v1/cloud/libraries/{C}/invitations/{id}` | sí, quien administra | — | `204`. El id se **compara** con las invitaciones de `C`; una de otra biblioteca da `404 not_found`. Anular una ya anulada o caducada no es un error; una aceptada da `410 invitation_used`. Toma los mismos bloqueos que aceptar, en el mismo orden (la biblioteca y luego la fila), y mira si ya está aceptada **después** de bloquear (SEG-043): si alguien acepta a la vez, o gana la aceptación y esto dice `invitation_used`, o gana la anulación y quien acepta lee «anulada» |
| `GET /v1/cloud/invitations/{token}` | **no** | — | `200 {"libraryName", "kind", "expiresAt"}` |
| `POST /v1/cloud/invitations/{token}/accept` | sí | — | `200 {"id": C}` |

- **Las rutas con `{C}` pasan primero por la puerta de siempre** (`CloudLibraryAccess::require`): quien no tiene fila en esa biblioteca recibe `404 not_found`, exactamente lo mismo que si no existiera. Quien tiene acceso pero no administra recibe `403 invitation_forbidden` (ya sabe que la biblioteca existe).
- **Mirar una invitación sin sesión** enseña lo justo para «Te invitan a «Casa»» o para decir por qué no vale. **No** enseña personas, roles ni **quién invita**: ni su nombre ni su correo, porque un enlace que acaba en un chat los filtraría. (Decisión del hito, a confirmar.) Un token que no existe o no tiene la forma del nuestro da `404 not_found`, igual en las dos rutas.
- **El enlace** es `<INVITATION_BASE_URL>/i/<token>` (D79). **El token va en la ruta, no en el fragmento (`#`).** D79 no dice nada del fragmento: un fragmento no llega al servidor web y no queda en sus registros, pero la página de `kosmotheka.app/i/` tendría que leerlo con JavaScript para pasárselo a la app, y los enlaces verificados de Android lo conservan. Si se quiere, es un cambio pequeño de la construcción del enlace (`InvitationSettings::linkFor`) y de la web; queda para decidir.
- **Si el servidor no tiene dirección para los enlaces** (`INVITATION_BASE_URL` vacía) no hace invitaciones: `409 invitations_unavailable`, y `GET /v1/server` dice `invitations: false` e `invitation_base: null`. Con dirección, `invitation_base` es esa dirección sin la barra final (`https://kosmotheka.app`): lo que van a empezar todos sus enlaces (`<base>/i/<token>`). La app solo la **compara** con un enlace; nunca la llama (7.6.3).

### Aceptar

Todo dentro de una transacción, con el **bloqueo de la biblioteca** (el mismo de `push` y `claim`) y después el de la fila de la invitación, que se **vuelve a leer** ya bloqueada: dos cuentas que abren el mismo enlace a la vez hacen cola y la segunda lee «usada». Se comprueba, en este orden:

1. La invitación: no usada (`invitation_used`), no anulada (`invitation_revoked`), no caducada (`invitation_expired`).
2. Quien invitó **sigue administrando**; la biblioteca **sigue lista**; la persona **sigue pudiendo ser invitada** por ese tipo (existe en esa biblioteca, no administra, sin acceso retirado, sin cuenta, y en un `link`, lectora y nueva). Si algo de esto falla: `invitation_revoked`, la app pide otra.
3. En `email`: el correo de la cuenta, huella con huella en tiempo constante (`hash_equals`), es el de la invitación (`invitation_not_for_this_account`, `403`).
4. La cuenta **no es ya otra persona** en esa biblioteca (`invitation_account_already_in_library`, `409`). Una fila de acceso sin persona (la de quien subió la biblioteca, antes de que el servidor se la dé) o ya cortada se enlaza con la persona; una cuenta **echada** de esa biblioteca (fila cortada) solo vuelve por una invitación **por correo**, que es la que alguien le dirigió a propósito; con un enlace para copiar recibe `invitation_not_for_this_account` (`403`) y el enlace no se gasta (D43, D98; test `testAnAccountThatWasThrownOutDoesNotComeBackByALinkToCopyOnlyByAnEmailMeantForIt`).

Después: la fila `membership` queda enlazada con la persona (da acceso), la invitación queda aceptada, y si la nube tiene el poder escribe `member.account_id` con `AccountLinker::noteAccount`, como hace `claim`. **El dispositivo de quien acepta se vincula después por el camino de siempre** (`claim`, `device.link_account`).

**Avisos** (después de confirmar la transacción, nunca dentro): a `libraryChanged` si la nube escribió `account_id`; al tema de la **cuenta de quien invitó**, solo ese, con `{"event":"invitation_accepted","library":"<C>"}` (sin datos de personas; la app pregunta lo demás); y al tema de la **cuenta que entró**, «tu lista de bibliotecas cambió». Es la primera vez que un aviso lleva datos, y la app actual los ignora (`SseEvent`).

### Códigos de error

| Código | HTTP | Cuándo |
|---|---|---|
| `invitations_unavailable` | 409 | El servidor no tiene `INVITATION_BASE_URL` |
| `invitation_forbidden` | 403 | La cuenta está en la biblioteca pero no la administra |
| `invitation_library_not_ready` | 409 | Sin personas y roles, o la nube sin poder para vincular dispositivos |
| `invitation_person_unsuitable` | 422 | La persona no existe en esa biblioteca, administra, tiene el acceso retirado, ya tiene cuenta, o (`link`) no es una lectora nueva |
| `invitation_expired` | 410 | Caducada |
| `invitation_used` | 410 | Ya aceptada |
| `invitation_revoked` | 410 | Anulada, o ya no vale (quien invitó dejó de administrar, la persona cambió, la biblioteca perdió los roles o la nube su poder) |
| `invitation_not_for_this_account` | 403 | Invitación por correo abierta por una cuenta con otro correo |
| `invitation_account_already_in_library` | 409 | La cuenta ya es otra persona en esa biblioteca |
| `not_found` | 404 | Sin acceso a la biblioteca, id de invitación de otra biblioteca, o un token que no existe |
| `too_many_requests` | 429 | Algún límite de abajo |

### Límites de peticiones

Con los limitadores de siempre (`RequestLimiter`, `config/packages/rate_limiter.yaml`):

| Limitador | Límite | Clave |
|---|---|---|
| `invitation_create` | 20 por hora | cuenta |
| `invitation_email` | **10 al día** | cuenta que invita |
| `invitation_recipient_library` | **3 al día** | pareja (biblioteca, huella del correo de destino): otra biblioteca no puede gastar lo que esta puede mandar a una dirección (SEG-044) |
| `invitation_recipient` | **10 al día** | huella del correo de destino, en total, para que no se pueda inundar una dirección desde muchas bibliotecas |
| `invitation_lookup` | 30 cada 15 min | IP (mirar y aceptar) |
| `invitation_accept` | 10 cada 15 min | cuenta |
| `cloud_sync` | 600 por hora para subir y otros 600 para bajar | cuenta (`push` y `pull`, aparte) |
| `cloud_link` | 30 por hora | cuenta (reclamar la persona y enlazar una biblioteca, juntos) |
| `cloud_events` | 60 por hora | cuenta (el token de los avisos) |

Las cifras de las rutas de la nube las decidió el maintainer el 15/10/2026: son anchas para que no frenen la primera subida de una biblioteca grande (lotes de 1.000 operaciones), y se gastan antes de mirar nada. Crear y listar bibliotecas no tienen límite propio. Las cifras de correo las decidió el maintainer el 11/10/2026 (SEG-044). Los tres límites de correo (cuenta, pareja y dirección) se **miran todos antes de gastar ninguno** (`RequestLimiter::requireInvitationEmailOpen`) y se **gastan solo cuando el correo ya se ha enviado** (`spendInvitationEmail`): una validación, una regla o un envío que falla no cuestan cuota, y un límite agotado no gasta los otros. **Límite conocido:** la huella es la del correo tal como se escribe, normalizado solo a minúsculas y sin espacios. `laura+1@…` y `laura@…` cuentan como direcciones distintas, a propósito: cada proveedor trata los alias a su manera y no se adivina cuál es el mismo buzón. El tope por cuenta (10 al día) sigue acotando lo que una cuenta puede mandar.

### El correo

`AccountMailer::sendInvitation`: texto plano, asunto y cuerpo fijos (`translations/emails.{en,es}.yaml`, clave `invitation`) en el idioma de la cuenta que invita (la de quien recibe se desconoce). El correo dice «Te han invitado a una biblioteca de Kosmotheka» y **no lleva nada escrito por quien invita**: ni el nombre de la biblioteca, ni su nombre, ni su correo (SEG-042, 11/10/2026). Se manda desde el dominio oficial a direcciones ajenas, y un nombre de biblioteca como «Tu cuenta caduca: entra en ejemplo.com/login» pasaba el filtro anterior, que solo quitaba `://` y `www.`; se quitó el filtro y el nombre. Quien abre el enlace ve el nombre de la biblioteca en la app. El correo se manda dentro de la transacción: si falla, la invitación no queda.

### Purga

`AccountPurger` (la purga horaria, D49) borra las invitaciones **una semana después de caducar**, aceptadas, anuladas o no. Hasta entonces, la lista de quien administra enseña qué fue de cada una.

### Lo que necesita quien aloja su servidor

- `INVITATION_BASE_URL` en `.env.local` (`.env` la trae vacía): la dirección con la que empiezan los enlaces, sin barra al final, p. ej. `https://kosmotheka.app` en el servidor oficial. **Vacía = sin invitaciones.** Esa dirección tiene que servir `/i/<token>` (D79: la página que abre la app, o deja copiar la invitación) y, para que los enlaces abran la app en Android, estar verificada en ella; en un servidor propio ese enlace no abre la app sola (D79, «Límite»).
- Correo configurado (`MAILER_DSN`, ver `docs/development/server.md`) si quiere invitar por correo. Sin él, el tipo `link` funciona igual.
- Ninguna clave nueva: la huella del correo se deriva de `APP_SECRET`. Si se cambia, las invitaciones por correo pendientes dejan de valer (duran 24 horas).

### Qué test cubre cada caso hostil

`tests/Controller/InvitationControllerTest.php` (el reparto y qué se vio fallar, en el informe del hito). Los diez casos de [`docs/plans/fase-7.md`](../plans/fase-7.md), sección 7.6:

| Caso | Tests |
|---|---|
| 1 · usado, caducado, anulado; dos a la vez | `testAUsedInvitationGivesNothingToASecondAccount`, `testAnExpiredInvitationGivesNothing`, `testARevokedInvitationGivesNothing`, `testWhoLosesTheRaceForAnInvitationReadsItAgainUnderTheLockAndGetsNothing` |
| 2 · correo de otra cuenta | `testAnEmailInvitationOpenedByAnAccountWithAnotherEmailGivesNothing`, `testTheServerKeepsAFingerprintOfTheEmailAndOfTheTokenNeverThemselves` |
| 3 · quien no administra | `testWhoDoesNotAdministerCannotInvite`, `testWhoIsNotInTheLibraryAtAllGetsTheSameNotFoundAsALibraryThatDoesNotExist` |
| 4 · persona de otra biblioteca | `testAnAdministratorOfAnotherLibraryCannotInviteForAPersonOfThisOne`, `testAnInvitationOfALibraryOnlyGivesAccessToThatLibraryWhateverTheOtherOneHoldsWithTheSameIds`, `testAnInvitationIsRevokedOnlyInTheLibraryItBelongsTo` |
| 5 · enlace para quien no es nueva o no es lectora, al crear y al aceptar | `testALinkForAPersonWhoIsNotANewReaderIsRefusedWhenMade`, `testALinkIsRefusedWhenAcceptedIfThePersonStoppedBeingANewReaderMeanwhile`, `testALinkIsRefusedWhenAcceptedIfThePersonGotADeviceMeanwhile` |
| 6 · administra, acceso retirado, ya tiene cuenta | `testAnInvitationForAPersonWhoAdministersHasTheAccessWithdrawnOrAlreadyHasAnAccountIsRefusedWhenMade` y los cuatro `…IsRefusedWhenAcceptedIfThePerson…Meanwhile` |
| 7 · quien invitó deja de administrar o lo echan | `testAnInvitationDiesWithTheAdministratorWhoMadeItWhenTheyAreDemoted`, `…WhenTheyAreThrownOut`, `testAnInvitationDiesIfTheLibraryLosesItsRolesOrTheCloudLosesThePowerToLinkDevices` |
| 8 · una cuenta que ya es otra persona | `testAnAccountThatIsAlreadyAnotherPersonInTheLibraryCannotAcceptAnInvitation`, `testTheAdministratorWhoInvitedCannotAcceptHerOwnInvitationAsAnotherPerson` |
| 9 · adivinar enlaces | `testTokensAreLongRandomAndAnUnknownOneIsAPlainNotFound`, `testGuessingFromOneAddressIsStoppedByTheRateLimit`, `testAcceptingIsLimitedPerAccount` |
| 10 · mandar correo a ajenos | `testEmailInvitationsAreLimitedPerAccountAndDay`, `testOneAddressCannotBeFloodedEvenFromSeveralAccounts`, `testCreatingInvitationsIsLimitedPerAccount`, `testTheEmailSaysOnlyWhatIsFixedAndNothingOfWhatWhoeverInvitesWrote`, `testAStrangerFromAnotherLibraryCannotUseUpWhatTheFamilyMayMailAnAddress`, `testNobodyMailsOneAddressMoreThanTenTimesADayInAllAndRefusingSpendsNothing`, `testAnEmailThatIsNotSentSpendsNoQuota`, `testAnAddressAlreadyAtItsLimitDoesNotUseUpTheAccountsOwn` |

## El registro de actividad (7.7, D66)

Una tabla de solo añadir, `activity_log`, para poder decir **quién hizo qué, cuándo y desde dónde** ante una reclamación, aunque lo afectado ya no exista. Escribe una sola pieza (`App\Security\ActivityLog`), llamada desde donde ocurre cada cosa: nada escucha todas las peticiones.

**La fila.** Sin claves foráneas: los ids se copian como valores, así que la fila sobrevive a la cuenta, la biblioteca o el dispositivo borrados. Campos: `kind` (el enum cerrado `ActivityKind`), `occurred_at` (reloj del servidor, UTC), `channel` (`http` o `console`: una orden de consola no tiene IP), `account_id` (el id de `user`), `account_email_fingerprint`, `device_id`, `uploaded_by_account_id` y `uploaded_by_device_id` (solo en las filas que salen de una operación: quien la subió, ver «Autor y quien subió»), `library_id` (la biblioteca **de la nube**) y `library_name` (el que el servidor tenía entonces), `subject_id` (la persona, dispositivo o invitación afectada), `operation_id`, `detail` (un valor corto y cerrado: un rol, `true`, el tipo de invitación o el id de una persona; nunca texto libre), `ip` y `user_agent` (cortado a 255 y sin caracteres de control). La entidad `ActivityEntry` no tiene modificadores, y lo único que borra de la tabla es la purga por antigüedad (`ActivityEntryRepository::deleteBefore`).

**Quién, cuándo y desde dónde salen del servidor (D84).** La cuenta y el dispositivo son los de la sesión (en una operación, los del **autor** que la firma verifica, y aparte quien la subió); la biblioteca, la de la tabla de acceso; el nombre, el que el catálogo del servidor guarda; la hora, la del reloj del servidor, nunca la HLC de la operación; la IP, `Request::getClientIp()`, que solo hace caso de `X-Forwarded-For` si quien conecta es uno de los `TRUSTED_PROXIES`. Lo que diga el cuerpo de una petición solo decide **de qué tipo** es el suceso de una operación, y solo cuando **se ha aplicado**.

**Qué se escribe:**

| Tipo | Dónde se escribe |
|---|---|
| `account_registered` | `POST /v1/account/confirm`: el registro **confirmado** |
| `login_succeeded` · `login_failed` | `POST /v1/auth/login`. El fallo, exista o no la cuenta (contraseña o código de segundo factor equivocados; contra una dirección sin cuenta la fila va sin cuenta ni huella, para que fallar cueste lo mismo en los dos casos y no delate qué direcciones tienen cuenta), sin dispositivo: la prueba del reto solo demuestra que quien llama tiene su clave, no que el dispositivo sea de la cuenta |
| `device_signed_out` | `POST /v1/auth/logout` |
| `password_changed` · `password_reset` | `POST /v1/account/password` · `/password/reset` |
| `two_factor_enabled` · `two_factor_disabled` | `TwoFactorController`, y la orden `app:user:two-factor-disable` |
| `account_deleted` | `DELETE /v1/account`, antes de borrar |
| `account_blocked` · `account_unblocked` | `app:user:block` |
| `library_uploaded` · `cloud_power_on` · `cloud_power_off` | una operación aplicada que cambia `library.cloud_library_id` o `library.cloud_device_id` |
| `role_changed` · `can_share_changed` | una operación aplicada que cambia `member.role` o `member.can_share` (`detail`: el valor nuevo) |
| `access_withdrawn` · `access_restored` | una operación aplicada que cambia `member.access_withdrawn` (D98) |
| `device_assigned` | una operación aplicada de un dispositivo que cambia `device.member_id` («de quién es», `detail`: la persona) |
| `device_access_revoked` | una operación aplicada que crea una `revocation` (D71) |
| `invitation_created` · `invitation_revoked` · `invitation_accepted` | `InvitationManager` y `InvitationAcceptor`; `detail` es `email` o `link`, nunca el correo ni el enlace |
| `account_linked` · `device_linked` | lo que **la nube hace sola** (D76): la operación `member.account_id` o `device.member_id` que firma ella misma. La cuenta es **para quien** se hizo (la que es esa persona en la tabla de acceso) y el dispositivo es el de la nube, autora de la operación |

**Autor y quien subió (SEG-041, 11/10/2026).** Un dispositivo sube con normalidad operaciones de otros (una tablet recibe por la red local lo que firmó otro móvil y lo sube primero), así que en las filas que salen de una operación `device_id` es el **autor**: el dispositivo que la firmó (el del HLC, que la firma verifica con la clave registrada; el cuerpo no puede elegirlo) y `account_id` la cuenta de **su persona** en esa biblioteca (la tabla de acceso del servidor, con la regla de la clave avalada de D96); nula si esa persona no tiene cuenta. `uploaded_by_account_id` y `uploaded_by_device_id` son los de la **sesión que la subió**, y `ip` y `user_agent` son siempre los de la petición, es decir, en estas filas los de quien subió. Lo que firma la nube tiene a la nube como autor (`device_id` es su dispositivo), `account_id` sigue siendo la cuenta **para quien** se hizo y no hay quien subió. En el resto de filas (iniciar sesión, invitaciones…) `account_id` y `device_id` son quien lo hizo y las dos columnas nuevas van vacías. Las filas anteriores a este cambio llevan en `account_id`/`device_id` a quien subió. `app:activity:list --account` busca por cualquiera de las dos cuentas y enseña las dos.

Solo se leen **cambios** (`update`, y la `revocation` que es una creación): la primera subida de una biblioteca trae toda su historia, cuyas creaciones no son «algo que cambió». Aun así, esas operaciones llegan al servidor entonces y la hora es la de llegada: una subida de una biblioteca con años de historia deja las filas de sus cambios de rol de entonces con la hora de la subida. `operation_id` permite ir a la operación firmada.

**No existe todavía:** `LibraryDeleted` está en el enum y en la lectura de operaciones (un `library` con `delete`), pero **hoy no hay ninguna ruta ni operación que borre una biblioteca de la nube** (solo se borra al borrar su cuenta, que deja `account_deleted`), así que no se escribe. Tampoco se escribe «persona creada» (no estaba en la lista), ni el cierre de sesión por quitar un dispositivo (D81), ni los accesos del superadministrador por consola (`app:user:super-admin`), que D52 menciona.

**La cuenta borrada.** Cada fila con cuenta lleva la **huella con clave de su correo** (la de `EmailFingerprint`: HMAC del correo normalizado con un secreto derivado de `APP_SECRET`), no el correo. Se calcula al escribir la fila, no al borrar la cuenta, para no modificar filas. Permite contestar «¿fue esta dirección?» sin guardarla. **Si cambia `APP_SECRET`, las huellas viejas ya no coinciden** con ninguna dirección (las invitaciones pendientes también se anulan, pero duran 24 horas y el registro, un año): hay que tenerlo en cuenta al rotarlo.

**Dentro de la transacción, o con rastro.** `record` solo **persiste**: la fila la escribe el mismo `flush` que escribe lo que registra, y en la misma transacción donde la hay (una operación del `push`, una invitación, un cambio de contraseña, el borrado de una cuenta), de modo que no existe una sin la otra. En el `push` se escribe tras `settle` y `requireLinkAllowed`, y solo las operaciones que **siguen** aplicadas: si el push se deshace, se deshacen también sus filas. Donde no hay nada más que escribir (un inicio de sesión fallido) `recordNow` hace su propio `flush` y, si falla, **lo dice en el log de la aplicación** (`error`) y deja seguir la respuesta: el inicio fallido no da acceso a nadie, y convertirlo en un error distinto diría a un atacante qué cuentas existen. Un inicio correcto escribe su fila en el mismo `flush` que el token: si falla, no hay inicio. Activar el segundo factor es la excepción: `enable` ya hace su propio `flush`, y la fila va en el siguiente; si ese falla, queda un 500 en el log de la aplicación (no se envolvió en una transacción porque un error esperado dentro de `wrapInTransaction` cierra el `EntityManager`).

**Purga.** `AccountPurger` borra al año (`AccountPurger::ACTIVITY_RETENTION`, el único sitio donde está el plazo): la misma orden (`app:accounts:purge`) y la misma planificación horaria que el resto.

**Consultarlo.** `make console C="app:activity:list --account=<id> --library=<id> --kind=<tipo> --since=2026-10-01 --until=2026-11-01 --limit=100"`. Sin ruta HTTP. Tope de 1000 filas (50 por defecto); lo que imprime va sin caracteres de control ni marcas de consola.

**IP y proxys.** En `.env`, `TRUSTED_PROXIES=127.0.0.1`: solo un proxy en la propia máquina es de confianza. Con nginx delante en el mismo host (7.9) basta con que nginx pase `X-Forwarded-For` con `$proxy_add_x_forwarded_for`: Symfony se queda con la dirección que nginx vio y no con lo que el cliente haya escrito a la izquierda. Si nginx corriera en otra máquina o en otra red de Docker, hay que poner **su** dirección o su rango en `TRUSTED_PROXIES`; con un valor de más, un cliente podría elegir su IP, y con uno de menos todas las filas llevarían la IP del proxy. Con un CDN delante, añadir sus rangos.

**Qué test cubre cada caso hostil** (`tests/Controller/ActivityLogTest.php` e `InvitationControllerTest.php`; escritos antes de la defensa y vistos fallar contra una primera versión sin ella):

| Caso | Test |
|---|---|
| Una operación rechazada, o que acaba como petición, no deja «cambió un rol» | `testAnOperationThatIsOnlyARequestOrIsRefusedLeavesNoLineOfAChange` |
| El autor, la biblioteca, el nombre y la hora son los del servidor | `testAnAppliedRoleChangeIsWrittenWithWhatTheServerKnowsNotWhatTheBodySays`, `testTheAccountsOwnEventsAreWrittenWithTheServersAuthorTimeAndPlace` |
| Un agente de megas, con saltos de línea y caracteres de control | `testAHugeUserAgentWithControlCharactersIsCutAndCleaned` |
| Un `X-Forwarded-For` falso | `testAForgedForwardedForDoesNotChooseTheAddress` |
| Borrar una cuenta o una biblioteca: las filas siguen y se listan | `testTheLinesOfADeletedAccountStayWithTheFingerprintAndWithoutTheEmail`, `testTheLinesOfALibraryThatIsDeletedStay` |
| Ninguna ruta devuelve filas | `testNoRouteReturnsALineOfTheLog` |
| La purga borra lo de más de un año y nada más reciente | `testThePurgeDeletesWhatIsOlderThanAYearAndNothingNewer` |
| Ni contraseñas, ni códigos, ni tokens, ni correos | `testNoLineHoldsAPasswordACodeATokenOrAnEmail`, `testTheLibraryTheInvitationsAndWhatTheCloudDoesAloneLeaveTheirLines` |

## Buscar libros a través del servidor (7.8, D83)

Google Books sin clave falla a menudo (la cuota anónima es de todo el mundo); con clave del servidor, no. Quien tiene cuenta puede pedir al servidor que busque un ISBN en Google Books. **Solo el ISBN.** Es la primera parte de 7.8: ni búsqueda por título, ni otros proveedores, ni portadas a través del servidor.

**La ruta.** `GET /v1/metadata/google-books/isbn/{isbn}`, con sesión de dispositivo como el resto de la nube (cuenta confirmada y no bloqueada; sin sesión, `401`, y no se llama a Google). Responde siempre la misma forma, pequeña y propia, nunca la de Google:

```json
{ "found": true, "title": "Inferno", "subtitle": null, "authors": ["Dan Brown"], "publisher": "Planeta",
  "published_year": 2013, "language": "es", "cover_url": "https://books.google.com/…", "external_id": "abc123" }
```

Si no se encuentra: `found: false` y el resto a `null` (y `authors: []`). Un «no encontrado» es una respuesta, no un error.

**Qué ofrece el servidor.** `GET /v1/server` lleva `metadata`: la lista de catálogos que busca para sus cuentas (`["google_books"]`, o `[]` si no tiene `GOOGLE_BOOKS_API_KEY`). Un servidor antiguo no la trae. La app solo pregunta a un servidor que lo dice.

**El ISBN.** El servidor acepta ISBN-13 e ISBN-10, con guiones o espacios, y guarda **solo la forma de 13 dígitos**: la caché tiene una fila por libro. Lo que no es un ISBN (letras, dígito de control malo, más de 40 caracteres, saltos de línea, `..`) responde `422 invalid_isbn` sin llamar a Google ni tocar la caché. Con una barra dentro, la ruta ni existe (`404`).

**La llamada.** `GoogleBooksHttpProvider`, detrás de la interfaz `BookProvider` (para probar con un cliente simulado; la forma sirve para añadir otro proveedor sin rehacerla). Dirección fija (`https://www.googleapis.com/books/v1/volumes`), de la que solo cambia el ISBN ya validado; **la clave va en la cabecera `X-Goog-Api-Key`, nunca en la dirección**; tiempo máximo de 5 s, sin seguir redirecciones, y la respuesta se lee con un tope de 512 KB. Lo que Google envía se corta y se comprueba campo a campo: título y subtítulo a 300 caracteres, editorial y autores a 200, como mucho 20 autores, año entre 1000 y 2100, idioma tipo `es` o `pt-BR`, id de Google con letras, cifras, `-` y `_`, y **la portada solo si es de `books.google.com`** (el `http://` se sube a `https://`, y se quita el efecto `edge=curl`). Si no hay ni título ni autores, es «no encontrado». Una respuesta que no es JSON, que pasa del tope o que viene con otro código se trata como fallo.

**Los códigos.**

| Código | Estado | Cuándo |
|---|---|---|
| `metadata_unavailable` | `404` | El servidor no tiene clave |
| `invalid_isbn` | `422` | No es un ISBN |
| `too_many_requests` | `429` | La cuenta pasó su límite (con `Retry-After`) |
| `metadata_quota_exhausted` | `503` | Google dijo que no hay cuota (`429` o `403`), o el servidor ya gastó su parte del día; con `Retry-After` |
| `metadata_failed` | `502` | Google no respondió, tardó, o contestó algo inservible |

**La caché.** Tabla `metadata_cache`: proveedor, ISBN-13, si se encontró, la respuesta ya normalizada y cuándo se pidió. **Ninguna columna dice quién preguntó** (un test lo comprueba por el esquema). Se guarda también el «no encontrado». Los plazos están en `MetadataCacheEntry` y en ningún otro sitio: **30 días** lo encontrado, **1 día** lo no encontrado. Un fallo de Google (cuota, error, tiempo agotado, basura) **no se guarda**. La purga horaria que ya existe (`AccountPurger`, `app:accounts:purge`) borra lo caducado.

**Los límites** (`RequestLimiter`, `rate_limiter.yaml`):
- **Por cuenta:** 600 búsquedas por hora, contando todas, también las que salen de la caché. Se corta a la cuenta que pasa, no a las demás.
- **Del servidor entero:** 900 llamadas **reales** a Google al día (su cuota gratuita es de 1.000). Agotado, la ruta sigue sirviendo lo que haya en caché y responde `metadata_quota_exhausted` para lo demás.
- **De cada cuenta, en llamadas reales** (SEG-052, decidido el 15/10/2026): más o menos la décima parte del tope del servidor, para que ninguna cuenta gaste el de todas. Google Books 90 al día, TMDB 500 al día, RAWG 60 al día y Comic Vine 15 por hora y recurso. Lo que sale de la caché no lo gasta. Agotado, la cuenta recibe `too_many_requests` con `Retry-After`, y sigue recibiendo lo que haya en caché.
- **El freno ante Google:** si Google responde `429` o `403`, el servidor **no vuelve a llamarle durante 30 minutos** (un elemento de la caché `cache.metadata`), responda quien responda. Sin él, cada petición de cada cuenta machacaría una cuota ya agotada. Un `403` también puede ser una clave mal puesta: se trata igual, porque insistir tampoco ayuda.
- **Google roto por un momento:** si Google responde un `5xx`, el servidor **vuelve a preguntar, hasta tres veces** en total, con 0,2 s entre intentos. Visto el 11/10/2026 desde producción: la misma búsqueda daba `503` («Service temporarily unavailable») cinco veces de seis, en 0,2 s cada una. Cada intento es una llamada real y gasta del tope diario. No se reintenta un Google lento (la persona esperaría el triple) ni un rechazo de la petición (un `400` de clave mal puesta daría lo mismo). Si fallan los tres, `metadata_failed` y la app sigue con sus otras fuentes.

**Lo que no se escribe.** Qué ISBN busca cada cuenta no va al registro de actividad ni a ningún log del servidor: la ruta no llama a `ActivityLog` y los errores de Google son un texto fijo, sin la excepción original (puede llevar la dirección). Lo único que lo vería es el **registro de accesos de nginx**: hay que desactivarlo para `/v1/metadata/` (ver [development/server.md](../development/server.md) y [publicacion/servidor-produccion.md](../publicacion/servidor-produccion.md)).

**En la app** (`metadata/via_server.rs`, `CloudApi::google_books_isbn`). Para Google Books, en `lookup_isbn` y `lookup_code`, en este orden:
1. **Clave propia** en Ajustes → directo a Google con ella, como siempre (tiene preferencia, D83). El servidor ni se entera.
2. Si no, con **sesión**, un servidor que dice ofrecerlo (`metadata` de `/v1/server`) y el ajuste **«Buscar en Google Books a través de la nube»** encendido (lo está por defecto) → se pregunta al servidor, con el ISBN y nada más.
3. Si no → Google sin clave, como hoy.

Si el servidor falla (red, no disponible, cuota, límite, más de 8 s, una respuesta que no es la forma fija o pasa de 64 KB), se prueba Google sin clave y, si también falla, el aviso es el de siempre. Lo que el servidor envía se vuelve a cortar y comprobar en la app, con las mismas reglas, porque un servidor es de otro: **la portada solo se acepta de `books.google.com`**, ya que la app la descargará. Si Google Books está desactivado en Proveedores, no se pregunta a nadie. El `Source` sigue siendo `GoogleBooks`. El ajuste solo se muestra con sesión en un servidor que lo ofrece.

**Qué test cubre cada caso hostil** (`tests/Controller/GoogleBooksLookupTest.php`, `tests/Metadata/`, y en la app `cloud/tests/metadata_tests.rs` y `cloud/client.rs`):

| Caso | Tests |
|---|---|
| 1. Sin sesión, token inválido, cuenta sin confirmar o bloqueada | `testWithoutASessionThereIsNoAnswerAndNoCall`, `testAnInvalidToken…`, `testAnAccountThatIsNoLongerConfirmed…`, `testABlockedAccount…` |
| 2. Lo que no es un ISBN | `testSomethingThatIsNotAnIsbnIsRefused…` (12 formas), `testASlashIsNotAnIsbnEither…`, `testAnIsbn10AndHyphensGoToTheSameThirteenDigitsAndOneRow`, `IsbnTest` |
| 3. La clave no aparece | `testTheKeyIsNowhereInWhatTheServerAnswersOrStores`, `testTheKeyGoesInAHeaderAndNeverInTheAddress`, `testFailuresSayOnlyWhetherItWasTheQuota…` |
| 4. Google envía basura | `testRubbishFromGoogleReachesTheAppOnlyAsTheFixedShape`, `testAGoogleThatAnswersNonsense…`, `testAnEnormousAnswerFromGoogle…`, `testOnlyACoverFromGoogleBooksSurvives` |
| 5. Google dice 429 o 403 | `testGoogleSayingNoOnQuota…`, `testAfterAQuotaAnswerTheServerStopsAskingGoogleForAWhile`, `testAGoogleThatFailsOrIsSlow…` |
| 6. Límites | `testAnAccountPastItsLimitIsCutAndTheOthersAreNot`, `testWhenTheServersDayIsSpentTheCacheKeepsServing` |
| 7. Dos cuentas, un ISBN | `testTwoAccountsAskingTheSameIsbnCostOneCallAndNoRowSaysWho` |
| 8. En la app | `a_key_of_their_own_goes_straight…`, `a_server_that_does_not_offer_it_never_gets_the_request`, `whatever_the_server_fails_at…`, `fields_of_the_wrong_type_are_dropped…`, `enormous_fields_are_cut`, `a_cover_that_is_not_from_google_books_is_dropped`, `an_enormous_answer_is_a_failure…`, `a_list_of_the_wrong_type_offers_nothing…` |

## Buscar películas y series por título en TMDB (7.8, segunda parte, D62)

Hoy películas y series se buscan solo en Wikidata, con poco detalle. Quien tiene cuenta puede buscarlas **por título** en TMDB a través del servidor, que pone su token. Es el primer proveedor que busca por título por el servidor y el primero que exige atribución.

**Condiciones de TMDB** (releídas el 11/10/2026; sus términos son del 20/10/2023): uso no comercial (la app gratuita encaja); **no guardar sus datos más de 6 meses** (la caché del servidor guarda como mucho 30 días); **atribución obligatoria**: su logo, menos destacado que el de la app, y el aviso «This application uses TMDB and the TMDB APIs but is not endorsed, certified, or otherwise approved by TMDB.». No dicen nada de que un servidor busque en nombre de sus cuentas.

**La ruta.** `POST /v1/metadata/tmdb/search`, con sesión como el resto. **Es un `POST` a propósito: el título que busca una persona no va nunca en una dirección** (las direcciones se escriben en registros). Cuerpo, estricto (un campo de más es `422 invalid_request`):

```json
{ "kind": "film", "query": "matrix", "language": "es" }
```

- `kind`: `film` o `tv_series`; otro valor, `422`.
- `query`: de 2 a 100 caracteres una vez limpiado (se quitan los blancos de los lados y los repetidos, también los de no separación). Con saltos de línea, tabuladores u otros caracteres de control, `422`. El servidor pregunta a TMDB por la forma limpia y en minúsculas.
- `language`: `es` o `en`; cualquier otro valor se toma por `en` (no es un error).

Responde `{"candidates": [...]}` con **8 como mucho**. Cada candidato tiene siempre la misma forma: `external_id` (el número de TMDB), `title`, `original_title`, `year`, `language` (el original), `cover_url`, `directors` (película), `creators` (serie), `duration_minutes` (película) y `seasons` (serie); lo que no aplica va vacío o `null`. Si el detalle de un resultado falla, ese candidato va sin esos campos: no se cae la búsqueda.

**La llamada** (`TmdbHttpSource`, detrás de `TmdbSource`; no es un `BookProvider`, que es de libros por ISBN). API v3 de TMDB: `search/movie` o `search/tv` con `include_adult=false`, y después el detalle de cada resultado (`/movie/{id}` con `append_to_response=credits`, o `/tv/{id}`), **todos a la vez**. Direcciones fijas; del cliente solo entran el tipo y el idioma (listas cerradas) y el texto como **un solo parámetro codificado**; el número del detalle es uno que **devolvió TMDB**, nunca uno del cliente. El token (el «API Read Access Token») va en `Authorization: Bearer`, nunca en una dirección. Tiempo máximo de 5 s, sin redirecciones, lectura con tope de 512 KB, excepciones sin `previous`, y reintento ante un `5xx` (hasta tres veces, como con Google Books: lo comparte `ProviderGate`). Se miran como mucho 40 resultados para quedarse con 8 bien formados; título y subtítulo a 300 caracteres, nombres a 200, como mucho 5 personas, año entre 1800 y 2200, id un entero positivo, y **el cartel solo si el `poster_path` es `/<letras y números>.<jpg|png>`**: la dirección la construye el servidor (`https://image.tmdb.org/t/p/w500/<fichero>`).

**La caché.** La misma tabla `metadata_cache` (su columna `code` pasa de 13 a 64 caracteres, migración `Version20261011183746`, para guardar una huella). Dos clases de fila, sin nada que diga quién preguntó:
- **La búsqueda**, 1 día, con `provider = tmdb_search` y como código la **huella (sha256) del tipo, el idioma y el texto normalizado**: la tabla no guarda en claro qué títulos se buscan. Guarda los resultados sin detalle.
- **El detalle**, 30 días, con `provider = tmdb_movie_<idioma>` o `tmdb_tv_<idioma>` y como código el número de TMDB.
Un fallo de TMDB (cuota, error, tiempo agotado, basura) **no se guarda**; un detalle que falla deja ese candidato incompleto y se vuelve a pedir la próxima vez. **Excepción (SEG-053, decidido el 15/10/2026):** si el proveedor contesta que no tiene ese detalle, o algo que no sirve (un `404`, un cuerpo que no es lo que dice mandar, el `status_code` 101 de Comic Vine), se guarda como «no encontrado» **una hora**, para que repetir la búsqueda no lo pida cada vez. Vale igual para RAWG y Comic Vine. Los fallos de red, de cuota o del propio proveedor (un `5xx`) siguen sin guardarse. La purga horaria existente borra lo caducado (`MetadataCacheRepository::deleteStaleBefore`).

**Los límites.** El mismo de **600 por hora por cuenta**, **compartido con Google Books** (una búsqueda cuenta una, aunque dentro haga varias llamadas), y un tope propio del servidor de **5.000 llamadas reales a TMDB al día** (la búsqueda y cada detalle cuentan una; `tmdb_calls`). Agotado el día, la caché sigue sirviendo y lo demás responde `metadata_quota_exhausted`; si se agota a mitad de los detalles, los que faltan van incompletos. Si TMDB responde `429`, `401` o `403`, el servidor **no le vuelve a llamar durante 30 minutos** (el mismo freno que con Google, con su propia clave de pausa); un `401` suele ser un token mal puesto y se trata igual, porque insistir tampoco ayuda. `/v1/server` lista `tmdb` en `metadata` si hay `TMDB_API_TOKEN`; sin él la ruta responde `404 metadata_unavailable`.

**Lo que no se escribe.** El texto buscado no va al registro de actividad, ni a ninguna fila, ni a ningún log: viaja en el cuerpo de un `POST`, y los errores de TMDB son un texto fijo. Un test recorre el log de la aplicación con un texto centinela y varias formas de error (cuerpo roto, campo de más, tipo equivocado, TMDB que falla). Ver también «Lo que se escribe en el log» más abajo.

**En la app** (`metadata/via_server.rs`, `CloudApi::tmdb_search`, `Lookup::search`). `Source` gana `Tmdb`. Para `Film` y `TvSeries`, si hay **sesión**, el servidor **dice ofrecerlo** (`metadata` de `/v1/server`) y la fuente no está desactivada en Proveedores, se pregunta al servidor **a la vez** que a Wikidata, y los candidatos de TMDB van primero. Lo que se manda: el texto escrito (sin blancos en los lados), el tipo y el idioma de la app, nada más. Sin sesión, sin oferta o para otro tipo, **no sale ninguna petición**. Si el servidor falla o tarda más de 12 s, quedan los de Wikidata y el aviso de siempre. La respuesta se lee con tope de 128 KB y se vuelve a cortar y comprobar con las mismas reglas (8 candidatos, textos cortados, año y duración posibles, y **el cartel solo de `https://image.tmdb.org/t/p/w500/<fichero>`**, porque la app lo descargará; `image.tmdb.org` se añade a `COVER_HOSTS` de `crates/domain`). De una película, los directores van como colaboradores con el papel `director`; de una serie, los creadores con el papel `writer` (no hay un papel «creador»).

**Proveedores y atribución.** TMDB aparece primero en la lista de fuentes de películas y series, con la etiqueta «Pide una cuenta»; solo se puede encender con sesión en un servidor que lo ofrezca (si no, un texto explica que llega con la cuenta, D62) y se apaga como las demás. Un texto corto dice qué se envía. Una sección **«Fuentes de datos»** dentro de Proveedores lleva el logo de TMDB (el oficial, de su página de marca: `app/src/assets/tmdb-logo.svg`, empaquetado con la app, nada se carga de fuera) y el aviso, en inglés y su traducción en español. En los resultados, la etiqueta de texto «TMDB» como las demás fuentes; **ningún logo en las fichas** (decisión del maintainer, opción A).

**Pendiente:** la clave propia de TMDB en Ajustes (D62 la prevé para todo proveedor con clave) queda fuera de esta pieza.

**Qué test cubre cada caso hostil** (`tests/Controller/TmdbSearchTest.php` y, en la app, `cloud/tests/tmdb_tests.rs` y `cloud/client.rs`):

| Caso | Tests |
|---|---|
| 1. Sin sesión, token inválido, cuenta sin confirmar o bloqueada | `testWithoutASessionThereIsNoAnswerAndNoCall`, `testAnInvalidToken…`, `testAnAccountNoLongerConfirmed…`, `testABlockedAccount…` |
| 2. Cuerpo que no es lo pedido | `testABodyThatIsNotTheOneAsked…` (17 formas), `testAQueryOfOneOverTheLimitAndOfAMegabyteAreRefused`, `testAnOddLanguageMeansEnglish…` |
| 3. Símbolos en el título | `testATitleWithSymbolsTravelsAsOneEncodedParameter`, `testSurroundingBlanksAndRepeatedSpaces…` |
| 4. El token no aparece | `testTheTokenIsNowhereInWhatTheServerAnswersStoresOrLogs` |
| 5. TMDB envía basura | `testAThousandResultsReachTheAppAsEight…`, `testFieldsOfTheWrongTypeAndHugeOnes…`, `testAPosterThatIsNotAPlainFileIsDropped` (12 formas), `testAnswersThatAreNotUsable…`, `testAnEnormousAnswer…` |
| 6. TMDB dice 429 o 401; 5xx | `testTmdbRefusingIsReported…`, `testAfterARefusalTheServerStopsAskingTmdb…`, `testAServerErrorIsTriedAgain…`, `testAServerErrorThatKeepsComing…`, `testASlowTmdbIsNotAskedAgain` |
| 7. Un detalle falla | `testADetailThatFails…`, `testADetailRefusedOnQuotaPauses…`, `testADetailThatFailedIsNotRemembered…` |
| 8. Dos cuentas, mismo título | `testTwoAccountsLookingForTheSame…`, `testTheSearchedTitleReachesNoLog…` |
| 9. Límites | `testAnAccountPastItsSharedLimit…`, `testWhenTheServersDayIsSpent…`, `testTheDayRunningOutWhileTheDetails…` |
| 10. En la app | `without_a_session_nothing_goes_anywhere`, `only_films_and_series_are_ever_sent`, `a_server_that_does_not_offer_tmdb_never_gets_the_text`, `a_list_of_a_hundred_is_cut_to_eight`, `fields_of_the_wrong_type…`, `enormous_fields_are_cut`, `a_cover_that_is_not_a_poster…`, `the_text_goes_in_the_body_of_a_post…`, `an_enormous_search_answer_is_a_failure` |

### Lo que se escribe en el log (revisado el 11/10/2026)

El servidor no usa Monolog: lo que registra es el registrador por defecto de Symfony (a `stderr`, de ahí al log del contenedor), que en producción escribe de `warning` para arriba. Visto con un registrador en memoria en los tests:
- Una excepción de la aplicación (`AccountException`: ISBN inválido, cuota, límite…) se escribe con su **código como único texto**, sin la dirección.
- **Una ruta que no existe escribe la dirección completa** (`No route found for "GET http://…/v1/metadata/google-books/isbn/978/8408176039"`, a nivel `error`), y con ella el ISBN que alguien buscó con una barra de más, o con un segmento de más después del ISBN. La ruta de TMDB no tiene este problema: el título va en el cuerpo. Un ISBN bien formado, o mal formado pero de un solo segmento, no escribe nada.
- A nivel `info` (solo en desarrollo) `Matched route` escribe `request_uri` en cada petición.
- Un error de validación del cuerpo de la ruta de TMDB escribe solo la regla incumplida, no el valor.

**Arreglado el 11/10/2026** (decidido por el maintainer): `{isbn}` acepta cualquier cosa, también barras (`requirements: ['isbn' => '.+']`), así que todo lo que cuelga de la ruta llega al controlador y responde `422 invalid_isbn` sin escribir la dirección (`testAnAddressWithMoreThanAnIsbnIsRefusedWithoutWritingItToTheLog`). El primer punto de esta lista ya no ocurre para esa ruta; una ruta nueva que lleve un dato de la persona en la dirección necesita el mismo cuidado.

## Buscar videojuegos por título en RAWG (7.8, tercera parte, D62)

Los videojuegos solo se buscaban en Wikidata. Quien tiene cuenta puede buscarlos **por título** en RAWG a través del servidor, que pone su clave. Es la misma pieza que TMDB con otro proveedor: comparte `ProviderGate`, `TextCleaner`, la caché y los límites, y lo demás (la fuente, la búsqueda, los candidatos) es propio, sin una abstracción forzada para dos casos.

**Condiciones de RAWG** (releídas el 11/10/2026; términos del 01/03/2021): gratis hasta **20.000 peticiones al mes**; **atribución**: nombrar a RAWG como fuente y poner un **enlace activo en cada página donde se usen sus datos**. No dicen nada de guardar los datos ni de que un servidor busque en nombre de sus cuentas.

**La ruta.** `POST /v1/metadata/rawg/search`, con sesión. Cuerpo estricto con un solo campo, `query`: de 2 a 100 caracteres una vez limpiado (como en TMDB; con caracteres de control, `422`; un campo de más, `422 invalid_request`). Sin tipo ni idioma: RAWG no traduce. Es un `POST` para que el título no vaya en una dirección. Responde `{"candidates": [...]}` con **8 como mucho**, cada uno con `external_id` (el número de RAWG), `title`, `year`, `cover_url`, `platforms` (10 como mucho) y, solo los **tres primeros** (los que tienen detalle), `developers` y `publishers` (5 como mucho cada uno); en los demás van vacíos. Sin `RAWG_API_KEY`, `404 metadata_unavailable`; con ella, `/v1/server` añade `rawg` a `metadata`.

**La clave: el punto delicado.** **RAWG solo admite la clave como parámetro `key` de la dirección**: no tiene cabecera para ella (no he podido ver su página de documentación, que se pinta con JavaScript; su ayuda y sus ejemplos solo usan `?key=`). Así que en cada llamada saliente la clave está en una dirección, y las direcciones se escriben en sitios: en el mensaje de las excepciones del cliente HTTP, en el log del cliente, en una respuesta de error. Lo que se hace para que no salga de ahí:
- Toda excepción del cliente se captura y se vuelve a lanzar como **texto fijo y sin `previous`** (`MetadataProviderException`); en ningún sitio se escribe la dirección de la llamada, ni al responder ni al log. Se cubre la petición, la lectura de la cabecera y la lectura del cuerpo, también cuando la conexión se corta a la mitad.
- El cliente HTTP con el que se pregunta a RAWG es **uno aparte** (`RawgClientFactory`), **sin registrador**: el cliente del framework escribe la dirección de cada petición en el log (a nivel `info`, que un servidor puesto en depuración sí escribiría), y aquí esa dirección lleva la clave. Un test comprueba que no tiene registrador.
- Nada de la respuesta de RAWG se pasa tal cual, y **un texto de RAWG que contenga la clave se descarta** (un RAWG que devolviera la dirección en un campo no la haría viajar a la app ni a la caché).
- Ni la respuesta de la ruta, ni una fila, ni una línea del log contienen la clave: un test con una clave centinela provoca todos los fallos posibles (`401`, `403`, `429`, `404`, `400`, `500`, `502`, tiempo agotado, DNS, JSON roto, una conexión que se corta al leer, un detalle que falla) y recorre las respuestas, las filas de la caché y el log de la aplicación.

**Las llamadas** (`RawgHttpSource`, detrás de `RawgSource`): `GET https://api.rawg.io/api/games?search=…&page_size=8` y, solo para los tres primeros resultados, `GET /games/{id}` (desarrolladores y editores), a la vez. El número del detalle sale de lo que devolvió RAWG. Mismos cuidados que con TMDB: 5 s, sin redirecciones, lectura con tope de 512 KB, reintento ante un `5xx` (hasta tres veces), freno de 30 minutos ante `429`, `401` o `403`. Textos recortados (título 300, plataformas 100, empresas 200), 10 plataformas, 5 empresas, año entre 1950 y 2200. **La portada solo si es `https://media.rawg.io/media/…`**, de nombres simples y acabada en `jpg`, `jpeg`, `png` o `webp`; sin `..`, usuario, puerto, consulta ni fragmento. La dirección es la que dio RAWG, validada, no una construida.

**La caché.** La misma tabla. La **búsqueda, 7 días**, con `provider = rawg_search` y como código el sha256 del texto normalizado (el texto nunca en claro); el **detalle, 30 días**, con `provider = rawg_game` y el número de RAWG. Sin quién preguntó. Un fallo no se guarda. La purga horaria existente borra lo caducado, con el plazo de cada tipo (`MetadataCacheEntry::SEARCH_TTLS`).

**Los límites.** El de 600 por hora por cuenta, compartido con Google Books y TMDB. Un tope propio del servidor de **600 llamadas reales a RAWG al día** (20.000 al mes entre 30, con margen; una búsqueda nueva cuesta hasta cuatro: la búsqueda y tres detalles). Agotado, se sirve lo que haya en caché y la app sigue con Wikidata. Los que faltan de los detalles van incompletos.

**En la app** (`metadata/via_server.rs`, `CloudApi::rawg_search`, `Lookup::search`). `Source` gana `Rawg`. Para `Videogame`, junto a Wikidata y con las mismas condiciones que TMDB (sesión, oferta en `/v1/server`, fuente no desactivada), y los de RAWG primero; sin sesión u oferta no sale ninguna petición. Lo que se manda: el texto, nada más. La respuesta se lee con tope de 128 KB y se vuelve a cortar y comprobar, también la portada (la app la descargará; `media.rawg.io` en `COVER_HOSTS`; la app ya guarda `jpeg`, `png` y `webp`). Del candidato:
- **Plataforma:** solo si RAWG da **una**; con varias se deja vacía para que la persona elija la de su ejemplar. Una de más de 60 caracteres (el máximo de un ejemplar) no se sugiere.
- **Desarrolladores** → colaboradores con el papel `studio` (el mismo que usa Wikidata para los videojuegos).
- **Editores** → el campo `publisher` (varios, separados por coma, hasta 200 caracteres).
No se añade ningún papel nuevo.

**Atribución** (más exigente que la de TMDB; decidida con el maintainer): en Proveedores, «Fuentes de datos» nombra a RAWG con un enlace a `https://rawg.io`; **en la pantalla de resultados de búsqueda**, cuando hay algún resultado de RAWG, una línea «Datos de RAWG» con el mismo enlace, además de la etiqueta de texto «RAWG» en cada resultado; nada en las fichas guardadas. El enlace se abre con el navegador del sistema (`@tauri-apps/plugin-opener`, en `components/external-link.tsx`). Sin logo.

**Qué test cubre cada caso hostil** (`tests/Controller/RawgSearchTest.php`, `tests/Metadata/RawgClientFactoryTest.php` y, en la app, `cloud/tests/rawg_tests.rs` y `cloud/client.rs`): los mismos diez de TMDB, adaptados, y además: 11. la clave en la dirección saliente (`testTheKeyIsNowhereInWhatTheServerAnswersStoresOrLogsWhateverGoesWrong`, `testAnAnswerThatBreaksWhileItIsReadLeaksNothingEither`, `testTheClientRawgIsAskedWithHasNoLogger`); 12. portadas de otro sitio, con `..`, con otra dirección dentro o con otra extensión (`testACoverThatIsNotAPlainImageOfTheirHostIsDropped`, 21 formas, y en la app `a_cover_that_is_not_an_image_of_their_host_is_dropped`); 13. cien plataformas o nombres enormes (`testFieldsOfTheWrongTypeAndHugeOnesAreCutOrDropped`, `testNamesOfHugeSizeAreCut`, `enormous_fields_are_cut`).

## Clave propia para TMDB y RAWG (7.8, cuarta parte, D62)

D62: todo proveedor que pide clave admite la **clave propia en Ajustes**, sin nube, como Google Books (D83). TMDB y RAWG solo funcionaban con cuenta; ahora funcionan también con la clave de cada persona.

**El orden** (el mismo que en Google Books, `via_server::choose_search_access`):
1. **Clave propia** (no en blanco: una clave de solo espacios no es clave; con espacios alrededor se usa sin ellos) → la app llama **directamente** al proveedor y **nada sale hacia el servidor de la cuenta**, ni siquiera `GET /v1/server`, haya o no sesión. Si el proveedor la rechaza (`401`/`403`), el aviso es el de clave incorrecta (`WarningKind::BadKey`), **no** se prueba el servidor por detrás: el fallo de una clave propia se enseña, no se esconde; cuota, el aviso de cuota.
2. Sin clave, con sesión, un servidor que lo ofrece y la fuente encendida → se pregunta al servidor (lo descrito arriba).
3. Si no → no se pregunta a nadie: **no hay modo anónimo** en estos dos.

**La llamada directa** (`metadata/tmdb.rs`, `metadata/rawg.rs`). TMDB: `GET /search/movie|tv` con `include_adult=false`, la clave («API Read Access Token») en `Authorization: Bearer`, y el detalle de cada resultado (con los créditos para películas) a la vez. RAWG: `GET /games?search=…&page_size=8&key=…` y el detalle de los tres primeros, con la clave en la dirección (no admite otra cosa). En ambas, el número del detalle es uno que devolvió el proveedor y que puede ser un id (entero positivo, hasta mil millones).

**Las mismas comprobaciones.** Lo que contesta el proveedor es tan poco de fiar como lo que contesta un servidor. Se convierte a **la forma fija** de la respuesta del servidor y pasa por **el mismo lector** (`parse_tmdb_answer`, `parse_rawg_answer`): 8 candidatos como mucho, textos cortados y sin caracteres de control, año, duración y temporadas posibles, plataforma solo si es una, y portadas solo de `image.tmdb.org` (la ruta `/t/p/w500/<fichero>`) o de `media.rawg.io`. Los recortes se comparten porque el lector es uno; lo que cambia es solo cómo se pasa del formato del proveedor a la forma fija. La respuesta del proveedor se lee con un tope de 512 KB.

**La clave no sale en un error.** RAWG la lleva en la dirección, y el error del cliente HTTP escribe la dirección. Toda llamada pasa por `metadata::get_json`, que quita la dirección de cualquier fallo (`reqwest::Error::without_url`) y no pasa nada de lo que dijo el proveedor: solo el tipo de fallo (clave rechazada, cuota, red, inesperado). Los avisos que llegan a la pantalla son solo ese tipo. La app no escribe log alguno. Los tests lo comprueban con una clave centinela contra un proveedor en memoria que repite la clave en sus errores, con una conexión que no se puede hacer (el caso que dejaba la dirección con la clave) y con una respuesta que no se entiende.

**Tests** (`cloud/tests/own_key_tests.rs`, sin red; las respuestas son ficheros pequeños en `tests/fixtures/` —`tmdb-*.json`, `rawg-*.json`— **escritos a mano con la forma documentada de cada API**, porque no había una clave real para grabarlas): la clave primero y el servidor nunca (con y sin sesión), nada sin clave y sin sesión, clave en blanco, clave rechazada sin servidor por detrás, un tipo que TMDB no cubre no envía nada, la forma de cada petición (cabecera o parámetro, y que la clave no aparezca donde no debe), cien resultados, textos enormes, portadas ajenas, ids imposibles, respuesta enorme y que no es JSON, los estados `401`/`403`/`429`/`5xx` y que ningún error lleve la clave.

**En la pantalla de Proveedores.** TMDB y RAWG ganan el campo de clave, el botón de probar y «Cómo conseguir una clave» (`themoviedb.org/settings/api` y `rawg.io/apidocs`). Con clave propia la fuente cuenta como disponible aunque no haya sesión y deja de decir que llega con la cuenta («Usa tu propia clave: nada pasa por el servidor»). La lista de tipos empieza por **«Todas»**, seleccionada al abrir en escritorio y primera de la lista en el móvil: a la derecha, todas las fuentes **una sola vez** con los tipos que cubren («Libros · Películas…»); al pulsar un tipo se filtra. Hay **un interruptor por fuente** (`disabled_sources` no distingue tipos; el modelo no cambia). El contador de «Todas» es el de fuentes usables entre el total de fuentes distintas. La sección de abajo se llama **«Créditos»**, en tamaño y color de pie de página, con el logo y el aviso de TMDB y el enlace a RAWG, y se ve con cualquier selección.

## Buscar cómics por título en Comic Vine (7.8, quinta parte, D62)

Los cómics solo se buscaban en Wikidata. Comic Vine tiene series y números. Sale **completo de una vez**: por el servidor para quien tiene cuenta y con **clave propia** en la app desde el principio (el orden y las comprobaciones son los de [RAWG](#buscar-videojuegos-por-título-en-rawg-78-tercera-parte-d62) y de [la clave propia](#clave-propia-para-tmdb-y-rawg-78-cuarta-parte-d62)).

**Condiciones de Comic Vine** (releídas el 12/10/2026 en `comicvine.gamespot.com/api/`): uso **estrictamente no comercial** (si no, revocan la clave); **200 peticiones por recurso y hora**, con detección de ráfagas, y recomiendan guardar las respuestas; **un enlace de vuelta en cada página donde se usen sus datos**; no admiten un producto que compita con ellos ni redistribuir sus datos alterados. Su página de introducción añade que no quieren «una aplicación de colecciones copiada»: esta guarda un catálogo personal, no ofrece sus datos como producto, y solo los usa para rellenar una ficha; queda dicho aquí por si el maintainer quiere pedirles su parecer antes de publicar.

**Qué se leyó de su documentación y qué se supuso.** Leído en `comicvine.gamespot.com/api/documentation`: `search` (`/search`) admite `api_key`, `format`, `query`, `resources` (`volume`, `issue`, `character`, `publisher`, `person`…), `limit` (**máximo 10** en la búsqueda) y `field_list`; la respuesta trae `status_code` (`1` bien, `100` clave no válida, `101` no encontrado, `102` error de URL…), `error`, `number_of_total_results`, `results`, y cada resultado de la búsqueda un `resource_type`; `volume` tiene `name`, `start_year`, `publisher`, `image`, `person_credits`; `issue` tiene `name`, `issue_number`, `cover_date`, `store_date`, `volume`, `image`, `person_credits`. Una petición real con una clave falsa contesta `HTTP 401` y `{"status_code":100,"error":"Invalid API Key",…}`, también con un agente de usuario genérico. **No dice** (y se supuso, sin clave para verlo): que el detalle de una serie es `/volume/4050-<id>/` y el de un número `/issue/4000-<id>/`; que `image` trae `small_url` y `medium_url` alojadas en `comicvine.gamespot.com/a/uploads/…`; que `person_credits` son `{name, role}` con un `role` como `writer, penciler`; que el límite se rechaza con `420` o `429`. Tampoco dice si exige un `User-Agent` propio: se manda uno (`Kosmotheka (+https://kosmotheka.app)`) y sin él no hubo queja. Todo esto es lo primero que hay que mirar con una clave real.

**Qué se busca.** Una sola llamada a `search` con `resources=volume,issue` y `limit=8`. Se quedan solo series e números (todo lo demás que Comic Vine encuentre —personajes, editoriales, personas, equipos— se descarta); **primero las series y luego los números**, cada grupo en el orden de Comic Vine, **hasta 8** en total. De los tres primeros del resultado final se pide el detalle (cada uno por su recurso).

**La respuesta** (`POST /v1/metadata/comic-vine/search`, cuerpo `{query}` de 2 a 100 caracteres, con sesión): `candidates` con, por cada uno, `resource` (`volume` o `issue`), `external_id`, `title`, `series` (el nombre de la serie, en un número), `number`, `year`, `publisher` (solo en series), `cover_url`, `writers` y `artists`. En la app una serie es el título tal cual, con editorial y año de inicio; un número es **«Serie #número: título»** (con lo que haya: «Serie #1», «Serie: título», «título #1»…), con su `number` y el formato de número suelto (`SingleIssue`). **Ojo:** Wikidata no pone ese formato a ningún cómic hoy (solo lo documenta el comentario del candidato); aquí sí lo pone.

**Autores.** Solo del detalle de los tres primeros. Comic Vine los da con un `role` (`writer`, `penciler`, `inker`…, a veces varios en uno: `writer, penciler`). Se usan los papeles que existen en `ContributorRole`: **`writer` → guion (`Writer`)**; **`penciler`, `penciller`, `inker`, `artist`, `painter`, `illustrator` → dibujo (`Illustrator`)**. Lo que no encaja (`colorist`, `letterer`, `editor`, `cover`) se omite, y una persona sin `role` también. Hasta 5 de cada. Ningún papel nuevo. Para las series, que según lo que se sabe no traen `role` en `person_credits`, saldrán sin autores hasta que se vea con una clave real.

**La clave: como RAWG.** Comic Vine solo admite la clave como parámetro `api_key` de la dirección. Por tanto, todo lo de RAWG: cliente HTTP aparte **sin registrador** (`UnloggedClientFactory`, que ahora sirve a los dos), excepciones relanzadas con texto fijo y **sin `previous`**, texto de Comic Vine que contenga la clave **descartado**, y el test de la clave centinela sobre respuestas, filas de la caché y el log, con todos los fallos (`401`, `403`, `420`, `429`, `404`, `400`, `5xx`, tiempo agotado, DNS, JSON roto, conexión cortada a mitad de lectura, y **un `status_code` 100 dicho con un estado de éxito**). Tiempo máximo de 5 s, sin redirecciones, lectura con tope de 512 KB, reintento ante `5xx`, freno de 30 minutos ante `401`, `403`, `420`, `429` o un `status_code` 100 (`ProviderGate`). Portada: solo `https://comicvine.gamespot.com/a/uploads/<nombres>/<fichero>` con `jpg`, `jpeg`, `png`, `gif` o `webp`; se toma la imagen **mediana** (o la pequeña); **nunca la original**, que puede ser enorme; `comicvine.gamespot.com` entra en `COVER_HOSTS`.

**Los límites, por recurso y hora.** Comic Vine cuenta por recurso, así que el servidor lleva tres: **150 llamadas reales por hora a la búsqueda, 150 al detalle de series y 150 al de números** (por debajo de sus 200). El de 600 por hora por cuenta, compartido con los demás proveedores. **Agotado el de un recurso, los demás siguen**: sin detalles de números, la búsqueda y los detalles de series funcionan (y viceversa); sin búsquedas, se sirve la caché.

**La caché.** La misma tabla: la búsqueda 7 días con `provider = comic_vine_search` (el código es el sha256 del texto normalizado, nunca el texto) y el detalle 30 días con `comic_vine_volume` o `comic_vine_issue` (por el número de Comic Vine). Sin quién preguntó. `/v1/server` añade `comic_vine` a `metadata` si hay `COMIC_VINE_API_KEY`.

**En la app.** `Source::ComicVine`, en `Lookup::search` para `Comic`, junto a Wikidata y primero. **Clave propia** en Ajustes (`KEYED_PROVIDERS`), con su campo, «Probar» (busca «batman») y «Cómo conseguir una clave» (`comicvine.gamespot.com/api/`): clave propia → llamada directa (`metadata/comic_vine.rs`) y nada hacia el servidor → servidor (sesión, oferta, fuente encendida) → nadie. La respuesta directa pasa por **el mismo lector** que la del servidor (`parse_comic_vine_answer`), que ahora exige además que todo id sea un entero entre 1 y mil millones. **Atribución:** en «Créditos», Comic Vine con enlace a `https://comicvine.gamespot.com`; en los resultados, cuando hay alguno suyo, una línea «Datos de Comic Vine» con el enlace. Una sola pieza (`components/source-credits.tsx`) sirve para RAWG y Comic Vine.

**Qué test cubre cada caso hostil** (`tests/Controller/ComicVineSearchTest.php`, 101 tests, y en la app `cloud/tests/comic_vine_tests.rs`): los trece de RAWG, adaptados, los cinco de la clave propia, y 19 (un id que no es un entero positivo razonable nunca llega a una dirección: `testAnIdThatIsNotAReasonablePositiveInteger…`, `direct_rubbish_from_the_provider…`), 20 (un resultado de un recurso que no se pidió se descarta: `testResultsOfAResourceThatWasNotAskedForAreDropped`) y 21 (agotado el tope de un recurso, los demás siguen: `testTheLimitOfOneResourceDoesNotTakeTheOthersDown`, `testTheLimitOfSeriesDetails…`). Las respuestas de prueba de la app son **ficheros escritos a mano** (`comicvine-*.json`) con la forma que dice su documentación: no hay clave real para grabarlas.

## Tablas del catálogo (7.4.3, rehecho por D92)

> Las tablas donde el motor de fusión de 7.4.4 escribirá el catálogo de cada biblioteca de la nube. Desde 7.4.6 las escribe el `push` (a través del motor y el juez); `pull` sigue trabajando solo con `cloud_operation`. [D92](../DECISIONS.md) (08/10/2026) sustituyó la migración a mano de 7.4.3 por **entidades de Doctrine**; la migración es `Version20261008120000`.

- **Una entidad por tabla**, en `server/src/Entity/Catalog/` (`App\Entity\Catalog`). Todas las filas extienden `CatalogRow` y llevan `key` (UUID v7, clave primaria, **nunca sale del servidor**), `cloud_library_id` y `id` (el de la app, `TEXT`). `CatalogEntity::entityClass()` da la clase de cada entidad y `table()` su tabla (prefijo `catalog_`, para que no choquen con las del servidor). Las entidades tienen *getters* y *setters* (salvo `key`, `cloud_library_id` e `id`, que no cambian).
- **`UNIQUE (cloud_library_id, id)`** en cada tabla (D41 precisada por D92): una biblioteca nunca pisa filas de otra, pero el id de la app ya no va en las claves foráneas. `cloud_library_id` apunta a `cloud_library (id)` con `ON DELETE CASCADE` (D84): al borrar la biblioteca de la nube se borra su catálogo.
- **Claves foráneas de una columna**, a la `key` de la fila referenciada (`work_key`, `publisher_key`…). Reglas de borrado (D89): referencia obligatoria, `CASCADE`; opcional, `SET NULL` (`editions.publisher`, `copies.owner`, `devices.member`), que Doctrine genera sin SQL a mano. Cada clave foránea lleva el nombre de su tabla (`fk_catalog_editions_cloud_library`, `fk_catalog_editions_work_key`…); la de `cloud_library_id` se declara una vez en `CatalogRow` y cada entidad le da su nombre con `AssociationOverrides`. Las tablas de detalles (`book_details`…) llevan además `edition_key` obligatoria; su `id` es el de su edición, y no se usa herencia porque una edición puede tener detalles de dos tipos.
- **Red de seguridad: disparadores «misma biblioteca».** Con claves de una columna, la base de datos ya no impide sola que una fila apunte a otra biblioteca. La migración crea la función `catalog_same_library()` y un disparador `BEFORE INSERT OR UPDATE` por cada referencia y por cada `edition_key` (`<tabla>_<columna>_same_library`), que lanza `23503`. Están escritos a mano en un bloque propio de la migración (el diff de Doctrine no los ve). El maintainer decidió mantenerlos el 09/10/2026, con su coste medido (entre un 12 y un 18 % de una reconstrucción). El test hostil de `CatalogRowsTest` se vio fallar sin ellos.
- **`catalog_members.access_withdrawn`** (7.5.4a, D98; migración `Version20261010101344`): booleano anulable (null se lee como falso, como `can_share`), «la persona ya no llega a la biblioteca». `CatalogLibraryView::roleOfDevice` y `memberOfDevice` no dan la persona de un dispositivo cuya persona lo tenga a verdadero; `memberAccessWithdrawn` lo usa la regla de la nube, que no vincula dispositivos a esa persona. El servidor solo aprende el campo y esa regla del juez: cortar a la cuenta (`membership`, `AccountLinker`) es 7.5.4b.
- **`catalog_devices.vouched_key`** (7.5.2b-1, D96; migración `Version20261010071307`): texto anulable, la clave que quien asignó el dispositivo a su persona tenía registrada para él. `CatalogLibraryView::roleOfDevice` y `memberOfDevice` solo dan la persona del dispositivo si está vacía o es igual a `public_key` (un dispositivo con otra clave avalada es de nadie, en la nube también); `deviceHasMemberId` mira `member_id` a secas, para la regla de la nube (que solo vincula dispositivos sin persona). El ataque de D96 sin enlace automático lo cubre `CloudSyncControllerTest::testAnAdministratorVouchingForTheRealKeyOfAnIdJhonnyHadRegisteredLeavesHimNothing`.
- **Sin `CHECK`** de `role`, `status` ni `rating`: son reglas de `spec/`, y una operación que las incumple no pasa la validación.
- **Versiones en la fila:** cada tabla lleva `versions JSONB NOT NULL` (`CatalogRow::$versions`): nombre del campo → HLC que lo escribió por última vez. Así leer una fila y sus versiones, y escribir ambas, es una sola sentencia; no hay tabla de versiones (D92, «Revisión del 09/10/2026»). La comparación es de bytes, igual que antes.
- **Contabilidad:** `catalog_tombstones` (las lápidas, D66) y `catalog_row_authors` (qué dispositivo creó cada fila, D45), con las entidades `Tombstone` y `RowAuthor`. Su `entity` es el valor de `CatalogEntity` (`work`), no el nombre de tabla de la app (`works`). Los HLC son `TEXT COLLATE "C"`. Sin claves foráneas a la fila: una lápida sobrevive a su fila.
- **Fuera:** lo que en la app es local (`local_state`, `known_servers`, `known_clients`, `library_peers`, `leave_notices`, `request_notices`, `pending_owners`, `forgotten_libraries`, `search_index`), el registro (`operations`: aquí es `cloud_operation`) y las vallas (`device_fences`, `device_fence_chains`): se derivan del registro y llegan con los permisos de 7.4.5.
- **Tests:** `tests/Entity/Catalog/CatalogSchemaTest.php` comprueba contra la base de datos real que cada tabla tiene las columnas de los campos de `CatalogEntity` (más `key`, `cloud_library_id` e `id`), las claves foráneas y sus reglas, el `UNIQUE`, el disparador de cada referencia, que no hay más tablas `catalog_*` y que el mapeo y la base de datos coinciden. `CatalogRowsTest.php` prueba el comportamiento con el ORM: los casos hostiles de otra biblioteca, mismo id en dos bibliotecas, referencias opcionales y borrados en cascada.

### Diferencias con SQLite

| Qué | SQLite | PostgreSQL | Por qué |
|---|---|---|---|
| Clave primaria | `(library_id, id)` o `id` | `key` (UUID v7 del servidor) | D92: Doctrine normal; el id de la app solo es único por biblioteca |
| `library_id` | columna sin clave foránea a `libraries` | `cloud_library_id` con clave foránea a `cloud_library` | La biblioteca de la nube existe antes que su catálogo; el servidor nunca usa el id local (D84) |
| Referencias | `work_id` (id de la app) | `work_key` (clave del servidor) | El motor traduce el id de la app a la fila, siempre dentro de la biblioteca |
| Una fila apunta a otra biblioteca | imposible | los disparadores `catalog_same_library` lo rechazan | Las claves foráneas ya no llevan `cloud_library_id` |
| Campos `cloud_library_id` y `cloud_server` de `library` | columnas de `libraries` | sin columna | El vínculo es el `cloud_library.id` del propio servidor; su versión sí consta en `versions` de la fila |
| Campo `cloud_device_id` de `library` y `account_id` de `member` (7.5.1, D76, D95) | columnas de `libraries` y `members` | columnas de `catalog_libraries` y `catalog_members`, **sí guardadas** | A diferencia de `cloud_library_id` y `cloud_server`, el juez del servidor las necesita: quién es la nube de la biblioteca y qué personas tienen cuenta (migración `Version20261009224124`) |
| `created_at` | columna con valor por defecto | no existe | Es de la app; la nube tiene `received_at` en `cloud_operation` |
| Detalles por tipo (`book_details`…) | clave `edition_id` | `id` (el de su edición) y `edition_key` | En `schema.rs` el id ya es el de la edición |
| Booleanos y enteros | `INTEGER` | `BOOLEAN` y `BIGINT` | La operación lleva `true`/`false` y enteros de 64 bits |
| Fechas | `TEXT` | `TEXT` | La operación lleva `AAAA-MM-DD` |
| `CHECK` de `isbn13`, `quantity`, `role`, `status`, `rating` | sí | no | Son reglas de `spec/` |
| Índices por nombre sin distinguir mayúsculas, `barcode` | sí | no | Son para la búsqueda de la app |
| Índices de las claves foráneas | sí | sí | Para que los borrados en cascada no recorran la tabla |

## Dónde está en el código

- `src/Controller/CloudController.php`: las cuatro rutas, sin lógica.
- `src/Sync/`: `CloudLibraryAccess` (la puerta), `CloudLibraryCreator`, `CloudPull`, `CloudPush` (las reglas), `OperationReader` (estructura), `SchemaVersion` (lee `spec/sync.json`), `OperationRejectedException`.
- `src/Dto/`: `PullRequest` (con `#[MapRequestPayload]`), `PushRequest` (no: ver su comentario), `PushedOperation`, `PushResult`.
- `src/Entity/` y `src/Repository/`: las cuatro tablas. Los códigos de error nuevos (`not_found`, `schema_version`, `invalid_request`) están en `AccountErrorCode` y `AccountException`.

## En la app

El cliente de la nube está en `app/src-tauri/src/cloud/`. Habla con las rutas de arriba y reutiliza lo de la red local sin cambiarlo: `sync::version_vectors`, `operations_since` y `merge` (que juzga firmas y permisos), `people::ensure_may_share` y `ops::record`.

- `cloud/client.rs`: `HttpCloud`, sobre el cliente de cuentas (`account::client`: mismo `reqwest`, mismas cabeceras del dispositivo). Crea `C`, lista las de la cuenta, `pull` y `push` (lotes de 1000). Un `404 not_found` y un `409 schema_version` pasan a errores de tipo `cloud`; el resto (sesión cerrada, dispositivo quitado, sin conexión) conserva la traducción de la cuenta.
- `cloud/mod.rs`: el trait `CloudApi` (crear, listar, enlazar, `pull`, `push`) y la lógica: `upload`, `sync_library`, `sync_all` y `status`. Los tests la ejecutan contra una nube en memoria (`cloud/tests.rs`), sin red.
- Comandos de Tauri: `upload_to_cloud(library_id)`, `sync_cloud()` (un informe por biblioteca) y `cloud_status()` (solo local: por biblioteca, si está enlazada, a qué `C`, y si podría subir). Los envoltorios están en `app/src/lib/api.ts`.
- Los errores propios son de tipo `cloud`: `not_found`, `schema_version`, `not_signed_in`, `unsigned_history`, `already_linked`. Textos en `cloud.errors.*` (`en` y `es`). Las invitaciones (`cloud/invitations.rs`, 7.6.2) se cuentan más abajo.
- El bloqueo de la base de datos nunca se mantiene a través de una llamada de red: se lee lo necesario, se suelta, se espera y se vuelve a bloquear para fusionar.

### Dónde vive el enlace

En la propia fila de la biblioteca: `libraries.cloud_library_id` y `libraries.cloud_server` (migración 0027). Solo los escribe la operación firmada `library.cloud` (D84, punto 3), que viaja con el registro y se aplica con las reglas de siempre. Nunca se rellenan con una tabla local ni con un valor que diga el servidor: lo que cuenta es lo que el registro fusionado aplicó. El cliente solo sincroniza una biblioteca si su fila dice `C` **y** el servidor de la cuenta actual.

### `upload(biblioteca)`

Falla con un error y sin cambiar nada si no se cumple una comprobación, en este orden:

1. Sesión iniciada (`HttpCloud::connect`: `not_signed_in`).
2. Este dispositivo **administra** la biblioteca, o esta no tiene roles (`people::administers`: `NotAllowed`). Enmienda de D76 (CON-009, 10/10/2026): subir a la nube nombra a un dispositivo con poder y es de quien administra; «Puede compartir» sigue valiendo solo para la red local (`people::may_share`). `status().can_upload` usa la misma regla, y el servidor no llega a enterarse de nada si no se cumple.
3. Ninguna operación replicable sin firma (`unsigned_history`; la interfaz propondrá «Duplicar como nueva», D58).
4. No está enlazada a ninguna nube (`already_linked`).
5. El enlace lo permitirían los roles (`ops::would_allow`: con roles, solo administra): se mira antes de crear `C` para no dejar una `C` huérfana.

Después: crea `C` en el servidor, registra con `ops::record` la operación `library.cloud` (un cambio de la fila `library` con `cloud_library_id`, `cloud_server` y, si el servidor da una nube cuyo id sale de su clave, `cloud_device_id`; se vuelve a comprobar que nadie la enlazó mientras tanto) y llama a `sync_library`, que envía todo el registro, el enlace incluido. Antes de subir, `push_missing` mira si entre lo que falta por enviar está el enlace de esta `C`; si está, lo envía primero a la ruta `link` (D93) y después sube los lotes. Va ahí y no en `upload` para que un reintento tras un corte de red también enlace.

### `sync_library(biblioteca)`

Una sesión, como `lan::client::sync_once`: `pull` con el vector del dispositivo para esa biblioteca; **se queda solo con las operaciones cuya `library` sea esa** (las demás se descartan y cuentan como rechazadas: el servidor no es de fiar para decir de qué biblioteca es una operación); `sync::merge`; y `push` de lo que a `C` le falta según el vector que devolvió (`operations_since`). El informe dice recibidas, enviadas y rechazadas (las descartadas, las que `merge` rechazó y las que rechazó la nube).

### La nube en la biblioteca: interruptor y avisos (7.5.5b)

- **Qué hace la subida con la nube (D76, D95).** `upload_with(…, autolink)` escribe `cloud_device_id` en la operación de enlace solo si `autolink` y el id que da la API sale de la clave que da la API; `upload` es `upload_with(…, true)`. Sin la casilla la biblioteca sube sin nombrar a la nube, que no se registra ni vincula nada.
- **El interruptor de los ajustes** (`cloud::set_cloud_power`, comando `set_cloud_power`). Solo quien administra (`people::administers`); otra persona recibe `NotAllowed` sin escribir nada (test `a_reader_cannot_give_or_take_the_cloud_s_power`). Encender pide el id de la nube al servidor (`list_libraries`, creído solo si sale de su clave) y llama a `people::name_cloud_device`; apagar, a `people::remove_cloud_device` (con su valla). Después sincroniza esa biblioteca, para que la nube se registre o deje de creerse. Quitar o poner el poder no toca los dispositivos que ya están. `LibraryCloudStatus.cloud_power` dice si la biblioteca nombra a una nube.
- **Qué muestra la pantalla de subir** viene de `LibraryCloudStatus`: `administers` (las que no administras, sin casilla). La interfaz no decide ningún permiso.
- **«Ya estás en … en la nube» (`LibraryReport.joined`).** Lo decide Rust en `sync_all`: la biblioteca está entre las que `claim` devuelve en `linked` (la primera llamada o la de después de subir), la sesión no la ha bajado de nuevo y es **la primera vez** que este dispositivo lo oye. «La primera vez» es una marca local (`local_state`, clave `cloud_joined.<biblioteca>`; nunca una operación) y no se avisa si este mismo dispositivo escribió el enlace con la nube (quien subió no «entra»). El informe lleva la persona y su rol en esa biblioteca. Un dispositivo ya enlazado antes de esta versión lo oirá una vez.
- **Una biblioteca retirada** (`removed`) lleva ahora su nombre en el informe (`library_name`), porque ya no existe cuando la pantalla lo cuenta.

### El móvil nuevo, el dispositivo de otra persona y quién tiene cuenta (7.5.5c)

- **La bajada.** `sync_all` no informa de nada mientras baja: hay un solo informe al terminar. La pantalla de espera (`DownloadWait`) no tiene porcentaje y sale solo si una sincronización, sin ninguna biblioteca enlazada aquí, dura más de 2,5 s. Un informe `downloaded` lleva ahora `me` (`LibraryReport.me`): quién es este dispositivo en esa biblioteca al terminar la sesión, ya con el segundo `claim`. Con eso se escribe un solo resumen («Han bajado 2 bibliotecas. En «Casa» eres Laura · administradora…»). No existe «Si sales de la app, seguirá bajando»: la sincronización solo corre con la app abierta.
- **«Terminando de preparar este móvil…» no existe.** `claim`, la bajada, el registro del dispositivo y el segundo `claim` van en una sola llamada de `sync_all`: la pantalla nunca ve la biblioteca sin que el segundo `claim` haya pasado ya. Si el móvil sigue sin persona al terminar, no es un rato: es el estado de `people.noPerson*`.
- **Quién tiene cuenta.** `Person.has_account` (`member.account_id` no vacío; nunca dice qué cuenta).
- **Quién vinculó cada dispositivo, sin tabla ni campo nuevo.** `PersonDevice.linked_by_cloud` sale de la versión del campo `member_id` en `field_versions`: el dispositivo de su HLC es el autor de la operación que dijo de quién es. Si es el `cloud_device_id` de confianza de la biblioteca (regla 12: registrado con la clave de la que sale su id), el dispositivo está vinculado «por la nube»; si no, `linked_by` es el nombre de la persona del autor (salvo que el autor sea el propio dispositivo). Es lo que **hoy** vale: si se quita el poder a la nube (`cloud_device_id` vacío), los dispositivos que vinculó dejan de salir «por la nube».
- **`claim.conflicts`.** `sync_all` los junta (primera y segunda llamada) y, por cada biblioteca, `cloud::other_owner` guarda en `local_state` la clave `cloud_other_owner.<biblioteca>` con la cuenta y la persona del dispositivo en ese momento, y si ya se respondió. Una primera `claim` que funciona sin la biblioteca en `conflicts` borra la clave. Los comandos `cloud_other_owners` y `answer_other_owner` dan las preguntas abiertas y guardan la respuesta; la pregunta vuelve si cambia la cuenta o la persona del dispositivo.
- **Lo que el diálogo no ofrece.** «Hacerla mía» no se puede: cambiar de quién es un dispositivo es de quien administra (regla 3), y este dispositivo es de otra persona. «Pedir a quien administra que lo cambie» tampoco, hoy: la petición tendría que nombrar a la persona de la cuenta, y la app no la conoce (el `accountId` que está en `member.account_id` no llega nunca al cliente; solo `user.id`). Haría falta que el servidor lo diera, o que `claim` dijera la persona de cada conflicto: se deja fuera de `app/`. El diálogo explica y ofrece «Dejarlo como está».

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
| El paso de **una** biblioteca falla por algo de esa biblioteca (`sync_library`, `adopt_link` o `download`) | Las demás siguen. El informe lleva `error` con el código si lo hay (DAT-009) | `failed` |

**Qué corta la sesión entera y qué no (DAT-009, `cloud::stops_the_session`).** Cortan la sesión: no haber iniciado sesión (`not_signed_in`), una versión del esquema que el servidor no admite (`schema_version`), cualquier error de la cuenta (`Error::Account`: servidor no disponible, límite de peticiones, dispositivo quitado), y los fallos de este dispositivo (`Http`, `Io`, `Sqlite`, base de datos bloqueada). Es de una biblioteca todo lo demás (`not_found`, `unsigned_history`, `invalid_request`, un registro que no se puede fusionar…). `download` trata un `access_withdrawn` como `sync_library`: si el corte llega al bajar, y la biblioteca nunca estuvo aquí, no hay nada que borrar (`access_withdrawn`); si llega al subir lo que se acaba de bajar, se pide la prueba y se comprueba como siempre.

### Invitaciones en la app (7.6.2, D43)

El servidor decide (sección «Invitaciones (7.6.1, D43)» de arriba). La app, en `app/src-tauri/src/cloud/invitations.rs`, solo evita ofrecer lo que el servidor va a rechazar, crea a la persona nueva, y lleva un enlace desde donde se abrió hasta la cuenta que lo acepta. Aquí están los comandos, los tipos de `api.ts` y el mock; las pantallas son de 7.6.3 (más abajo).

**Cliente.** El trait `CloudApi` gana seis métodos: `offers_invitations` (`GET /v1/server`, campo `invitations`), `create_invitation`, `list_invitations`, `revoke_invitation`, `peek_invitation` y `accept_invitation`; los implementan `HttpCloud` y `FakeCloud`. `HttpCloud::anonymous` es el cliente sin sesión (nunca envía el token, aunque lo haya) y lo usan las dos llamadas que no la necesitan: mirar un enlace y preguntar qué ofrece el servidor. Un `404` de un token pasa a `invitation_not_found`; los códigos del servidor (`invitation_expired`, `invitation_used`…) llegan tal cual como errores de cuenta.

**Comandos** (todos reciben `library_id`, como `upload_to_cloud`; la pantalla pasa la biblioteca activa):

| Comando | Qué hace |
|---|---|
| `invite_status(library_id)` | `{canInvite, blocker}`: sin sesión, no está en la nube de este servidor, este dispositivo no administra, la nube sin poder de vincular, o el servidor no hace invitaciones. Local salvo preguntar al servidor qué ofrece; no necesita sesión |
| `invite_to_library(library_id, request)` | `request = {person: {type: "new", name} \| {type: "existing", member}, kind: "email" \| "link", email?}`. Devuelve `{member, personCreated, invitation, failed}`. Siempre sincroniza la biblioteca antes de pedir la invitación, también para una persona que ya estaba (COS-002: si su subida falló la vez anterior, el servidor aún no la conoce); si no puede, `failed` trae el motivo de la sincronización (`server_unavailable`…) y no se pregunta al servidor |
| `list_invitations(library_id)` · `revoke_invitation(library_id, invitation_id)` | Las del servidor (pendientes, aceptadas, anuladas y caducadas; el `memberId` permite cruzarlas con Personas). Solo quien administra. El id de `revoke_invitation` solo entra en la ruta si tiene forma de UUID (SEG-049); si no, `invitation_not_found` sin preguntar |
| `open_invitation(link)` | Sin sesión. Saca el token del enlace, pregunta al servidor qué es y lo deja esperando. Devuelve `{libraryName, kind, expiresAt, signedIn}` |
| `pending_invitation()` · `discard_pending_invitation()` | `{state: "nothing"}`, `{state: "waiting", …}` (lo que espera, si sigue vigente) o `{state: "expired", libraryName}` (caducó mientras se iniciaba sesión); y «no era esta» (que también da por dicha la nota de caducada) |
| `accept_invitation()` | Con sesión: acepta lo que espera, hace `sync_all` y dice en qué biblioteca se ha entrado (`library`, `libraryName`, los informes y `syncError` si la sincronización falló: la invitación ya está usada y la siguiente sincronización normal trae la biblioteca) |

**Invitar.** Antes de escribir nada ni llamar al servidor, la app comprueba en Rust lo que el servidor comprueba (`readiness`): sesión, biblioteca enlazada a la nube **de este servidor**, roles, que la persona de este dispositivo administre, y que la biblioteca nombre a la nube como dispositivo que vincula (D76). Un dispositivo que no administra recibe `not_allowed` sin escribir a la persona y sin llamar. El servidor tiene la última palabra: la app no decide el permiso.
- **Persona nueva:** una operación firmada (`member.invite`) crea la fila `member` con `role: reader` y `can_share: false` (lo único que existe: la petición no tiene campo de rol y `deny_unknown_fields` rechaza entera la que lo traiga), se sube con `sync_library` y solo entonces se pide la invitación con su id. Se admiten `email` y `link`.
- **Persona que ya está:** solo `email`, y no se toca su fila. `Person.invitable` (`"email"` o `"none"`) lo calcula Rust para Personas: solo lectores y editores, sin el acceso retirado y sin cuenta.
- **Si algo falla después de crear a la persona**, nada se deshace: el resultado trae `failed` con el código y `personCreated: true`, y la pantalla lo cuenta. La persona está en la biblioteca y se puede volver a pedir la invitación por correo.
- El correo se valida solo en su forma (algo, `@`, un dominio con punto, sin espacios) y viaja tal cual; no se guarda en ninguna parte.
- **El enlace completo** de una invitación `link` solo existe en la respuesta de crearla. No se escribe en la base de datos ni en el registro, ni en mensajes de error: el tipo `Secret` lo oculta de `Debug`, y un test busca el token en todas las columnas de todas las tablas.

**Abrir un enlace, y a qué servidor se habla.** Hoy la app, sin sesión, habla con el servidor guardado en `local_state` (`account_server`, el de «Otro servidor», D77) o con el oficial `https://api.kosmotheka.app`; solo se cambia con la sesión cerrada. **Un enlace nunca elige a qué servidor se llama**: solo se lee, y el servidor al que se pregunta es siempre el de la cuenta. Pero tiene que ser **de ese servidor**: si no, el token de uno se le entregaría a otro, que podría usarlo allí. La regla (`token_of_link`):
- servidor oficial: solo enlaces de `kosmotheka.app` (D79), por `https` y con el puerto por defecto;
- servidor propio que **publica la base de sus enlaces** (`invitation_base` en `GET /v1/server`, 7.6.3): solo los enlaces que empiezan por esa base (mismo esquema, host y puerto, y la ruta de la base antes de `/i/<token>`). Un enlace de otro sitio se rechaza aunque sea del mismo host que la API. La base **no se cree** si no es una dirección sencilla (con usuario, contraseña, parámetros o fragmento; no `https`, salvo `http` en un servidor `http`) o si es la oficial (`kosmotheka.app`, con o sin punto final) o cuelga de ella: un servidor propio que se dijera dueño de `kosmotheka.app` recibiría los tokens oficiales. Y la oficial **ignora** lo que publique: sus enlaces son los de `kosmotheka.app`;
- servidor propio que no dice nada (uno antiguo): **su mismo host**, y no el dominio de encima (SEG-048: en `github.io` o `co.uk` ese dominio es de cualquiera); **nunca** `kosmotheka.app` ni un subdominio suyo. Es la regla provisional de 7.6.2, que queda de reserva. En todas las reglas los hosts se comparan normalizados: sin mayúsculas y sin el punto final (`kosmotheka.app.` es el oficial, y `api.ejemplo.org.` el mismo `api.ejemplo.org`);
- **la base solo dice qué enlaces son de ese servidor.** Un servidor que publique una dirección ajena (`https://malo.test`) consigue, como mucho, que la app lea un enlace de `malo.test`: la llamada sigue yendo al servidor de la cuenta, y nunca a `malo.test`;
- la forma: `https` (o `http` solo si el propio servidor es `http`, p. ej. local), sin usuario, contraseña, parámetros ni fragmento, ruta exactamente `/i/<token>` y token de 43 caracteres `A-Za-z0-9_-`.
Cualquier otra cosa da `invitation_bad_link` o `invitation_other_server` **sin preguntar nada** (salvo `GET /v1/server`, que sí se pregunta antes para saber la base).

**El token que espera.** Si no hay sesión, el token queda en `local_state` (`pending_invitation`: token, servidor, nombre de la biblioteca, tipo, caducidad y la cuenta que tenía abierta la sesión al abrirlo, si había una). Nunca en una operación, ni en las copias de seguridad. Caduca con la invitación (y entonces `pending_invitation` lo dice: `{state: "expired", libraryName}`, que sustituye al token por una nota con solo el nombre hasta que la pantalla la da por dicha con `discard_pending_invitation`, y `accept_invitation` responde `invitation_expired`; `{state: "nothing"}` si no había nada), se borra al cerrar la sesión (`clear_session`) y con «Dejar el dispositivo a otra persona» (que borra toda la base). Al aceptar se rechaza (y se olvida) si el servidor ya no es el mismo o si lo abrió una cuenta y ahora hay otra. Tras un fallo del servidor solo queda esperando lo que un reintento puede arreglar (sin conexión, límite de peticiones, sin sesión); cualquier respuesta del servidor lo borra.

**Que un enlace abra la app (7.6.4, D79).** Hecho con `tauri-plugin-deep-link` y, en escritorio, `tauri-plugin-single-instance` (con su característica `deep-link`: un segundo arranque con un enlace no abre otra ventana, lo pasa al plugin y trae la existente al frente). Un enlace entra por **un solo sitio**, `links.rs`:

- `route(enlace)` dice **qué es por su forma y sin preguntar a nadie**. Hoy solo hay un destino, `Destination::Invitation`: `https://kosmotheka.app/i/…` (esquema `https`, el host oficial y el primer tramo del camino `i`). Todo lo demás (otro dominio, otra ruta de `kosmotheka.app`, `http`, cualquier esquema, entre ellos `kosmotheka://…`, texto que no es un enlace, más de 2048 caracteres) es `None`: se ignora sin error visible.
- `Arrivals` guarda **un** enlace sin mirar: el más nuevo. Lo que la persona ya está leyendo no está ahí (ya se sacó), así que uno nuevo nunca lo sustituye. El evento `link-arrived` no lleva nada: el enlace no sale de Rust hasta el comando.
- `open_arrived_link` → `links::attend` saca el enlace y, **antes de preguntar a ningún servidor**, comprueba su forma contra el servidor oficial (`token_of_link`: sin usuario, parámetros ni fragmento, el código con su forma); después va por el mismo camino que el pegado, `invitations::open`: el servidor al que se pregunta es siempre el de la cuenta, nunca el que nombre el enlace. Devuelve solo qué dice la invitación; el enlace no llega a la interfaz, así que no está en su estado, su historial ni un mensaje.
- Para añadir un destino (volver de Google, D77) se añade una variante a `Destination`, una línea en `route` y un brazo en `attend`.
- La interfaz (`useArrivedLink`) lo recoge al arrancar, al llegar el evento y mientras haya uno esperando; **solo lo abre cuando no hay delante ningún diálogo** (cualquier `role="dialog"` o `alertdialog`: confirmaciones de borrar, dejar el dispositivo, la bienvenida, otra invitación). Mientras tanto espera en Rust. Al abrir enseña «Tengo una invitación» con la biblioteca a la vista; si el enlace era malo, el mismo diálogo lo dice con los textos de `invitations.errors.*`. **Un enlace nunca hace nada por sí solo**: como mucho abre ese diálogo; entrar sigue pidiendo pulsar «Entrar».
- **Esquema propio.** `kosmotheka` está registrado en escritorio (Linux: `.desktop` con `x-scheme-handler/kosmotheka`; Windows: el registro; macOS: `Info.plist`; en `tauri.conf.json`), pero `route` lo ignora: `kosmotheka://i/<token>` sigue rechazado (un enlace sin dominio no dice de qué servidor es, y está por decidir). Lo usará el encargo de Google. En Android también (7.1): un `intent-filter` con `scheme=kosmotheka`, `host=auth`, `pathPrefix=/google`, sin verificación; solo `kosmotheka://auth/google?…` es una vuelta de Google (`links.rs`, `Destination::GoogleReturn`) y se atiende por su propio camino ([accounts.md](accounts.md#continuar-con-google-71-d77)). `kosmotheka://i/<token>` sigue ignorado.
- **Android.** `tauri.conf.json` declara `https://kosmotheka.app` con `pathPrefix /i/` y `appLink: true`; el plugin escribe el `intent-filter` con `android:autoVerify="true"` en el manifiesto al compilar (detalle en [firma-android.md](../publicacion/firma-android.md#4-enlaces-verificados-las-huellas-de-assetlinksjson-764)). Sin el `assetlinks.json` servido y con la huella buena, Android no verifica y el toque abre el navegador (que enseña la página de `invitacion.html`). Los servidores propios no tienen enlaces verificados (D79).
- **Registros.** Ni `route`, ni `Arrivals`, ni `attend` escriben nada: `Secret` oculta el enlace en `Debug`. En una compilación de depuración el plugin `deep-link` puede escribir (en `tracing`) un argumento de arranque único que parece un enlace de un esquema que no es el configurado; las compilaciones de *release* no.
- **Pruebas** (`cloud/tests/links_tests.rs`, con `FakeCloud`): qué enlaces son de la app y cuáles no (otro dominio, otra ruta, otro esquema, basura, descomunales), que lo ajeno no pregunta a ningún servidor, que uno de los nuestros mal formado se dice mal sin preguntar, que el enlace no elige servidor, que un enlace solo mira (no acepta, no entra, no cambia la sesión), que cien seguidos son una mirada, y que el enlace no sale por `Debug`.

**El aviso a quien invitó.** `invitation_accepted` llega por el canal de avisos (`{"event":"invitation_accepted","library":"<C>"}`, sin datos personales). `events::run` lo lee y emite `Signal::InvitationAccepted(C)` además del `Changed` de siempre; el servicio de avisos lo convierte en el evento `invitation-accepted` de Tauri con `{library, libraryName}`, **solo si** esta instalación tiene esa `C` en el servidor de la cuenta (`invitations::accepted_notice`, que no toca la ventana: una notificación con la app cerrada puede usarla tal cual). La app pone «Alguien ha aceptado tu invitación a «Casa»» y sincroniza por el `cloud-changed` que viene a la vez. No dice quién: el evento no lo trae, y la lista de invitaciones (`state: accepted` + `memberId`) deja a Personas decirlo en 7.6.3.

**Errores.** Los de invitación tienen texto propio en `invitations.errors.*` (`en` y `es`) y en [design/errors.md](../design/errors.md). Códigos del servidor: los nueve de «Invitaciones (7.6.1)». Códigos de la app (tipo `cloud`): `not_in_cloud`, `no_roles`, `no_cloud_power`, `server_without_invitations`, `invalid_email`, `invalid_name`, `link_needs_new_person`, `person_not_invitable`, `invitation_bad_link`, `invitation_other_server`, `invitation_not_found`, `invitation_no_pending`, `invitation_other_account`. Quien no administra recibe `not_allowed` (tipo `permission`).

### Las pantallas de las invitaciones (7.6.3, D43)

Solo muestran lo que Rust dice (`inviteStatus`, `Person.invitable`); el servidor tiene la última palabra. Mapa:

| Pieza | Qué hace |
|---|---|
| `components/people/invite-dialog.tsx` | **Invitar.** Antes de ofrecer nada llama a `inviteStatus`: con un bloqueo no enseña el formulario sino qué falta y un botón que lleva a arreglarlo (iniciar sesión, subir la biblioteca, activar personas y roles, los ajustes de la biblioteca; `not_admin` y `server_without_invitations` solo se explican). Dos formas sin selector de rol: «Por correo» (nombre y correo) y «Copiar enlace» (nombre); para una persona que ya está, solo el correo y conserva su rol. El enlace vive solo en el estado del componente: ni estado global, ni `localStorage`, ni `console`; al cerrar se pierde y antes se avisa |
| `components/people/invitation-note.tsx` | «Tiene una invitación sin usar · caduca …» con «Anular» (confirmación corta), o «Aceptó la invitación». En el móvil la fila es un botón, así que allí solo cuenta y «Anular» está en la ficha |
| `pages/people-page.tsx` | Botón «Invitar»; «Invitar por correo» en el menú y la ficha de quien tiene `invitable: "email"` y ninguna pendiente. La lista (`listInvitations`) se pide al abrir y tras `invitation-accepted` y `cloud-changed`; si falla, Personas sigue igual y no dice nada de invitaciones |
| `components/cloud/invitation-dialog.tsx` | **Abrir un enlace.** Un campo para pegarlo; `openInvitation` y solo el nombre de la biblioteca y la caducidad. Sin sesión lleva a iniciar sesión o crear cuenta; `App` vuelve a este diálogo al entrar (una vez por inicio de sesión, cuando no hay otra cosa abierta). Con sesión, `acceptInvitation` y se cambia a esa biblioteca. Entradas: la pantalla de la cuenta, los ajustes (sin sesión) y la bienvenida |
| `App.tsx` | `invitation-accepted` pone el aviso con «Ver en Personas», que cambia a esa biblioteca y abre Personas |

**Dos invitaciones a la misma persona.** El servidor **no lo impide**: `POST …/invitations` no mira si ya hay una pendiente, y cada una vale por separado. La pantalla no ofrece invitar a quien tiene una sin usar (se ofrece «Anular»); es una regla de la pantalla, no de permisos.

**Qué test cubre la base de los enlaces (7.6.3).** `cloud/tests/invitations_tests.rs`: `a_server_that_says_where_its_links_start_has_exactly_those_links` y `a_published_base_never_makes_the_app_call_it_nor_hand_over_an_official_link` (una base ajena no consigue que la app llame a esa dirección; la de `kosmotheka.app` no se cree; la oficial no se mueve; una base con usuario, contraseña, fragmento o sin forma se ignora; una `http` no casa con enlaces `https`). Escritos antes de la defensa y vistos fallar contra una versión que ignoraba la base. En PHP, `ServerControllerTest::testShowReturnsWhatTheServerOffers`.

### Qué test cubre cada regla (`cloud/tests.rs`)

| Regla | Test |
|---|---|
| Subir: el enlace está en el registro, la fila enlazada y la nube tiene todo el registro | `uploading_links_the_library_and_sends_its_whole_log` |
| Una biblioteca que no cabe en un `push` sube entera a la primera (D93): la app enlaza antes por la ruta `link` | `a_library_bigger_than_one_push_goes_up_whole_the_first_time` (visto fallar: 304 enviadas, 304 rechazadas) |
| Una sincronización normal no vuelve a enlazar | `a_library_already_in_the_cloud_is_not_linked_again` |
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

### Qué test cubre cada caso hostil de las invitaciones en la app (7.6.2)

Todos en `cloud/tests/invitations_tests.rs` (contra `FakeCloud`) salvo el del aviso, en `cloud/events/tests.rs`. «Visto fallar»: se quitó cada defensa por separado y el test se puso rojo (barrido de mutaciones de 39 cambios; el único que sobrevivió, quitar la comprobación expresa de que un servidor propio no recibe enlaces de `kosmotheka.app`, obligó a añadir el caso del servidor propio alojado en `kosmotheka.app`).

| Caso | Test |
|---|---|
| Un enlace de otro servidor no se sigue, ni se lee, ni pregunta nada | `a_link_of_another_server_is_refused_before_anything_is_asked` |
| Un token mal formado, un esquema o ruta raros, usuario o fragmento: nada se pregunta ni se guarda | `a_malformed_token_is_refused_before_anything_is_asked` |
| El servidor al que se pregunta es el de la cuenta, no el del enlace | `on_the_official_server_a_link_of_kosmotheka_app_is_followed` |
| La persona nueva nunca es otra cosa que lectora, aunque el comando lo pida | `a_new_person_is_never_anything_but_a_reader_even_if_the_command_asks`, `a_new_person_is_created_as_a_reader_uploaded_and_only_then_invited` |
| Un dispositivo que no administra no crea a la persona ni llama al servidor | `a_device_that_does_not_administer_writes_no_person_and_asks_the_server_nothing` |
| Un enlace solo para una persona nueva | `a_link_for_a_person_who_is_already_in_the_library_is_refused_before_asking` |
| Nadie que administre, tenga el acceso retirado o tenga cuenta | `nobody_is_invited_who_administers_has_the_access_withdrawn_or_has_an_account` |
| El token que espera no sobrevive a otra cuenta, a cerrar la sesión, a «dejar el dispositivo», a otro servidor ni a caducar | `the_pending_token_does_not_survive_another_account_signing_in`, `signing_out_forgets_the_pending_token`, `leaving_the_device_to_another_person_takes_the_pending_token_with_it`, `a_pending_token_is_not_taken_to_another_server`, `a_pending_token_expires_with_its_invitation` |
| El enlace y el token no se escriben en ninguna tabla ni se imprimen | `the_link_of_an_invitation_is_not_written_anywhere_on_the_device`, `a_link_and_a_token_do_not_print` |
| Si algo falla tras crear a la persona, no se deshace y se dice | `if_the_server_refuses_the_invitation_the_new_person_stays_and_the_result_says_so`, `if_the_upload_of_the_new_person_fails_the_server_is_not_asked_for_an_invitation` |
| El aviso `invitation_accepted`: solo ese evento, y solo con una biblioteca de esta instalación | `an_accepted_invitation_is_told_once_and_still_gives_a_cloud_changed`, `other_events_do_not_tell_of_an_invitation`, `the_notice_names_the_library_only_if_this_device_has_it_in_that_cloud` |

No se ha podido probar con un servidor real (ni con el de `server/`): la nube de los tests es una versión pequeña de sus reglas.
