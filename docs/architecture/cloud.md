# La nube: bibliotecas, acceso y sincronización

> El lado del servidor del hito 7.2: las bibliotecas que una cuenta sube al servidor y las rutas para sincronizarlas. Decisión de fondo: D84 (la puerta), con D56 (firmas), D57, D73 y D76. Pensado como mapa para revisar el código: dice qué clase hace qué y qué test lo prueba. Documento vivo.

## La regla de oro (D84)

**El servidor no se fía de ningún id del cliente.** El id local de la biblioteca, los ids de operación y los ids de dispositivo que llegan en una petición se **comparan**, nunca se usan para buscar ni para decidir el acceso. El acceso sale solo de la cuenta y el dispositivo autenticados, de la tabla `membership` y del id propio de la biblioteca en la nube (`C`, `cloud_library_id`), que crea el servidor.

El caso con el que se mide todo: **Jhonny** tiene una cuenta, es hostil y conoce los ids de una familia (biblioteca, operaciones, dispositivos). No debe recibir nada de ella, ni escribir en ella, ni estorbarla.

## Tablas

Una migración (`Version20261003165457`). Todo cuelga de `cloud_library`; al borrarla se borra lo demás, y al borrar una cuenta se borran las bibliotecas que posee y sus membresías (`ON DELETE CASCADE`).

| Tabla (entidad) | Contenido | Clave |
|---|---|---|
| `cloud_library` (`CloudLibrary`) | `C` (UUID v7 que genera el servidor), dueña (`owner_id`), `local_library_id` (nulo hasta el enlace, se fija una vez), `cloud_device_id`, `cloud_public_key` y `cloud_secret_key` (la identidad de la nube, 7.5.2a: nulas si el servidor no tiene secreto; la privada va cifrada y no sale nunca), `link_operation_id` (la operación que enlaza: la que `CloudLink` comprobó —firma de un dispositivo de la cuenta dueña— al enlazar, esté ya guardada o no; la primera se queda; D93), fecha | `id` |
| `membership` (`Membership`) | cuenta, `C`, `member_id` (nulo hasta 7.5), fecha. Hasta 7.5 la única fila es la de la dueña, creada con la biblioteca | `id`; único `(user_id, cloud_library_id)` |
| `cloud_operation` (`CloudOperation`) | autor (el dispositivo del HLC), `hlc`, versión de esquema, `body` JSONB (la operación tal como llegó, firma incluida), fecha de recepción, `previous`, `status` (7.4.5; `applied`, `request` o nulo si aún no se juzgó) y quién la subió (7.4.6: `pushed_by_user_id`, foránea a la cuenta con `ON DELETE SET NULL`, y `pushed_by_device_id`, el id del dispositivo de esa cuenta, sin foránea). Quién la firmó y quién la subió no son lo mismo: una operación puede haber llegado antes por la red local. Una operación ya guardada no cambia este dato; las guardadas antes de 7.4.6 lo tienen vacío | **`(cloud_library_id, operation_id)`** |
| `cloud_device_key` (`CloudDeviceKey`) | clave pública de un dispositivo **dentro del registro de esa `C`** | `(cloud_library_id, device_id)` |

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
9. **El id del enlace es solo del enlace (SEG-022, SEG-024).** La operación que acaba de enlazar `C` en este `push` se dio por comprobada con la clave de la cuenta dueña, y solo esa entrada del lote, no cualquiera que lleve su id: las demás pasan por la regla 4. Una operación cuyo id es el anotado en `link_operation_id` solo se guarda si es el enlace de `C` firmado por un dispositivo de la cuenta dueña (`CloudLink::holds()`); si no, se rechaza (`bad_signature`). Sin esto, otra operación con ese id ocupaba el sitio del enlace y la regla 8 miraba su estado. `refuseKeyChanges` se aplica a todas, también al enlace. Una biblioteca enlazada sin enlace anotado (las de antes de 7.4.6) lo anota cuando llega ese enlace por `push` o por la ruta.

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
- **Sellos** (D94, 7.4.7): `catalog_device_seals` (operación que sella y su HLC), derivada del registro como las vallas. Una valla del autor sobre su propio dispositivo hace de la operación un sello (`CatalogJudge::insertFence`); `CatalogLibraryView::fenced` mira, además de las vallas, si hay un sello posterior a la operación que **no nombra** a su autor (no hay fila de `catalog_device_fences` de esa operación para ese dispositivo): entonces la operación cae bajo el sello y se juzga como de nadie. `take()` recalcula la biblioteca si llega un sello en petición (puede ser el que aparta lo que lo dejó así) o si uno aplicado deja bajo sospecha operaciones que el registro ya tenía de dispositivos que no nombra (`CloudOperationRepository::hasOperationUnderSeal`). La migración rellena la tabla con las vallas que ya había y que son del propio autor. Con el sello vuelve a valer la regla de D93 contra quien antedata su enlace: un enlace de un dispositivo que el sello no nombra, fechado antes, queda en petición y el `push` se deshace (`403 link_not_allowed`).
- **El juez** (`CatalogJudge`; no abre transacciones): `take(biblioteca, operación)` anota si llega tarde (su HLC es menor que el mayor de las operaciones ya juzgadas de la biblioteca antes de esta tanda, que se lee una vez), juzga con `Permissions`, si se permite aplica con el motor y vuelca, guarda el estado, y si quedó aplicada apunta sus vallas y mira si alguna deja bajo sospecha una operación que ya estaba. `settle(biblioteca)` rehace la biblioteca si algo llegó tarde **y** la biblioteca se juzga por orden (tiene roles, vallas o retiradas). `rebuild(biblioteca)` vacía el catálogo y aplica el registro entero en orden de HLC, por tramos, juzgando cada operación en su momento; repite mientras alguna valla candidata quede fuera por venir de una operación que no acabó aplicada. No hay avisos de petición ni índice de búsqueda: son solo de la app.
- **Comando:** `app:catalog:rebuild` llama a `CatalogJudge::rebuild()` e imprime operaciones aplicadas y operaciones que quedan en petición. Cada reconstrucción toma el mismo bloqueo de `C` que un `push` (DAT-007), así que se puede ejecutar con el servidor en marcha: un `push` a esa biblioteca espera. `app:catalog:rebuild --all` (7.4.6) reconstruye cada biblioteca de la nube, **de una en una y cada una en su transacción**, e imprime una línea por biblioteca; si una falla lo dice, sigue con las demás y termina con error (`CatalogRebuildCommandTest`). Al desplegar 7.4.6 hay que ejecutarlo una vez, porque las operaciones guardadas antes tienen el estado vacío y las bibliotecas no tienen catálogo. Con una sola biblioteca: `app:catalog:rebuild <id>`.
- **Qué no se porta:** los avisos de petición (`request_notices`), el índice de búsqueda, las vallas locales de `ops` (`fence_changes` y `fence_mark` son de la parte que escribe de la app), y la validación de la cadena y de la firma, que son de `CloudPush`.
- **Tests:** `PermissionConformanceTest` ejecuta los 32 casos de `spec/conformance/permissions.json` en cada orden de entrega (cada operación sola, guardada, juzgada y asentada) y de nuevo tras un `rebuild`; las operaciones `rejected` no llegan al juez y un test fija los dos casos que las traen. `CatalogJudgeTest` comprueba que una operación tardía sin roles, vallas ni retiradas no rehace nada, y la valla que llega antes que su operación (`a_fence_that_arrives_before_its_operation_is_judged_again_when_it_arrives`). `CatalogLibraryViewHostileTest` trae tres casos vistos fallar: dos bibliotecas con los mismos ids de dispositivo y miembro, retirar la petición de otro con un id de operación repetido, y un `Member` anticuado en el `EntityManager`.

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

**Cuando echan a quien subió la biblioteca.** Su fila de `membership` sin persona no se ve afectada (no hay a quién retirar); si ya estaba enlazada con su persona, se le corta como a cualquiera. La biblioteca de la nube **sigue existiendo** (`cloud_library.owner` sigue siendo su cuenta) y los demás miembros siguen entrando. No se resuelve aquí de quién es entonces.

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
- Los errores propios son de tipo `cloud`: `not_found`, `schema_version`, `not_signed_in`, `unsigned_history`, `already_linked`. Textos en `cloud.errors.*` (`en` y `es`).
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
