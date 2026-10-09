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

Todas bajo `/v1/cloud/libraries` (menos `events`, abajo), cortafuegos `api` (cabeceras `X-Device-Id` y `Authorization: Bearer`, como el resto de `/v1`), JSON. Los errores de negocio son `{"error": "<código>"}`, los de `AccountErrorCode` (el mismo listener que la API de cuentas).

| Ruta | Cuerpo | Respuesta |
|---|---|---|
| `POST /v1/cloud/libraries` | — | `201 {"id": C}`. Crea `C` de la cuenta y su membresía |
| `GET /v1/cloud/libraries` | — | `200 [{"id": C, "localLibraryId": …\|null}]`, las de la cuenta |
| `POST /v1/cloud/libraries/{C}/pull` | `{"schemaVersion": n, "vector": {dispositivo: hlc}}` | `200 {"operations": [{"schemaVersion": n, "operation": {…}}], "vector": {…}}` |
| `GET /v1/cloud/events` | — | `200 {"token": …, "topics": […], "expiresAt": …}`; ver [Avisos al momento](#avisos-al-momento-73). Sin Mercure: `409 {"error":"events_unavailable"}` |
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
5. **Cadena de cada autor (D87):** una operación cuya `previous` no está guardada en esta `C` (ni la aceptó antes el mismo `push`) **espera**: se rechaza y tapa a las siguientes de su autor en el `push`, igual que en la app; el vector de `C` para ese autor se queda por debajo y la app la reenvía completa. Si `previous` es de otro autor o tiene un HLC no menor, está mal formada y se rechaza igual (`CloudPush::requirePrevious`). Una `previous` nula o repetida se acepta. Una operación con un HLC que ya tiene otra de esta `C` se rechaza (`hlc_taken`) y retiene igual; un índice único `(cloud_library_id, hlc)` lo asegura en la base de datos. No es un juicio por el orden de llegada: no dice que la operación sea mala, solo la aplaza hasta que esté lo que firmó su autor antes (la costura 7 sigue igual para autores distintos).
6. **Nunca se rechaza por el orden de llegada** entre autores distintos: una operación más antigua que otras de otro autor se guarda como cualquier otra. Dentro de un lote, el servidor ordena por HLC (la operación de enlace enlaza `C` al empezar, pero se guarda en su sitio, detrás de las que encadena) para que el registro de un dispositivo se procese antes que lo que firma, sea cual sea el orden en que el cliente lo envíe. **Una rechazada tapa a las siguientes de su autor en el lote** (DAT-001, como `held_back` en `merge_in` de la app): una vez rechazada una operación de un autor, las posteriores de ese autor en el mismo `push` también se rechazan, para que el vector de `C` no salte un hueco que nadie volvería a enviar (la app las reenvía en el siguiente `push`, ya completas). Las que no se pueden leer cuentan por el autor y el HLC que declaran, si declaran alguno. Entre `push` distintos no reordena: el cliente envía las operaciones de un autor en orden y completas, y una operación de un autor cuya clave aún no consta en `C` se rechaza.
7. **No se juzgan roles ni permisos** (hito 7.4) ni se leen retiradas (D71) hasta 7.5: hasta entonces solo reciben los dispositivos de la cuenta dueña. Un lector con la app modificada puede subir una copia a su cuenta (SEG-013): pendiente de 7.4.

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

> Aplica una operación al catálogo de una biblioteca de la nube con las entidades de [Tablas del catálogo](#tablas-del-catálogo-743-rehecho-por-d92) y el `EntityManager`, sin SQL. **Hasta 7.4.6 nadie lo llama desde el `push`**: `CloudPush` y `CloudPull` no cambian y la nube se comporta como antes.

- **Dónde:** `server/src/Sync/Merge/`. `CatalogMerge` es la interfaz (`apply`, `flush`, `clear`) y `OrmCatalogMerge` la implementación con el ORM (D92: si la carga masiva lo pidiera, otra implementación con DBAL tendría que pasar los mismos casos). `apply` devuelve un `ChangeOutcome` (`Written` o `Skipped`) por cada cambio.
- **Qué reglas:** las de [`spec/README.md`](../../spec/README.md) (apartados «Merging operations from several libraries» y «Merging operations within a library»), que ejecutan `spec/conformance/merge.json` y `library-merge.json` contra el motor, en cada orden de entrega, volcando al final y tras cada operación (`server/tests/Sync/Merge/MergeConformanceTest.php`). El caso de HLC repetido es del `push` (7.4.6), y un test comprueba que la lista de casos saltados es exactamente la de los que traen `rejected`.
- **Guiado por `CatalogEntity`:** la clase sale de `entityClass()`, los campos de `fields()`, y los valores se leen y escriben con los metadatos de Doctrine. Nada del cliente llega a un nombre de clase, propiedad o campo, y toda búsqueda de una fila lleva su biblioteca de la nube: un id de la app nunca se resuelve fuera de ella. Un campo que no está en la lista salta el cambio entero.
- **Se comprueba antes de escribir.** Si un `flush` falla, Doctrine cierra el `EntityManager` y la petición entera se pierde. Por eso toda restricción de las tablas (`NOT NULL`, claves foráneas, `UNIQUE`, tipo de la columna, disparadores) tiene su regla en el motor, que la juzga antes de persistir; la base de datos es la red de debajo, nunca lo que decide. Como en la app (`apply_change`), esas comprobaciones valen solo para los campos que ganan por HLC: un campo que pierde no se escribe y no puede hacer que se salte el cambio. Los tests comprueban que, tras cada caso de `spec/` y cada test hostil, el `EntityManager` sigue abierto.
- **En memoria:** el motor recuerda las filas, versiones y lápidas que ha tocado desde el último `flush` y las busca ahí antes de ir a la base de datos. Tras un borrado hace `flush` y desvincula con `detach()` lo que cargó (la base de datos ha borrado en cascada o anulado referencias), pero nunca llama a `EntityManager::clear()`: en 7.4.6 compartirá el `EntityManager` con la cuenta y la biblioteca de la petición. No abre ni cierra transacciones. Quien cargue filas del catálogo por su cuenta debe desvincularlas antes del siguiente `flush` del motor.
- **Comando:** `app:catalog:rebuild <id de la biblioteca de la nube>` borra el catálogo y la contabilidad de esa biblioteca y aplica su registro (`cloud_operation`) en orden de HLC, leyéndolo por tramos y volcando cada 100 operaciones (`CatalogRebuildCommand::FLUSH_EVERY`), todo en una transacción. Cada 500 (`ANALYZE_EVERY`) y al terminar ejecuta `ANALYZE` de `catalog_field_versions`, `catalog_tombstones` y `catalog_row_authors`: dentro de una transacción tan larga las tablas recién llenadas no tienen estadísticas (ni `ANALYZE` ni autovacuum ven filas sin confirmar) y el planificador recorre todas las versiones de la biblioteca por el índice de `cloud_library_id` en vez de usar la clave primaria. Es la única SQL a mano; el motor sigue sin ella. Imprime operaciones aplicadas, cambios saltados y tiempo. Reconstruir dos veces, o aplicar el registro de una en una, deja el mismo catálogo (`CatalogRebuildCommandTest`, con las fixtures).
- **Carga:** `make stress` (`CatalogLoadTest`, grupo `stress`, fuera de `make check`) reconstruye una biblioteca inventada de 20.000 libros y mide la mediana de 30 `push` de dos operaciones sobre ella. En la suite normal, una de 300 libros comprueba que aplicar por lotes y de una en una da el mismo catálogo.

### Lo que midió la carga (08/10/2026)

`make stress BOOKS=… LIMIT=…` (`NO_TRIGGERS=1` apaga los disparadores dentro de la transacción del test, `NO_ANALYZE=1` salta todo `ANALYZE`, `FILTER=testWhatOnePushAsks` lista las consultas de un `push`). Todo ocurre dentro de la transacción única de DAMA, sin commit; el `push` son dos operaciones en una transacción (un savepoint). Reconstruir imita el comando: `flush` cada 100 operaciones y `ANALYZE` cada 500.

| Libros | Operaciones | `ANALYZE` | Disparadores | Reconstruir | Primeras / últimas 1.000 operaciones | `push` (mediana / peor) | Memoria |
|---|---|---|---|---|---|---|---|
| 2.000 | 2.241 | sí | sí | 36,3 s | 17,8 / 17,3 s | 26,8 / 31,6 ms | 104 MB |
| 2.000 | 2.241 | sí | no | 32,3 s | 15,2 / 16,0 s | 23,5 / 28,3 ms | 104 MB |
| 2.000 | 2.241 | no | sí | 37,0 s | 16,8 / 16,5 s | 51,4 / 55,0 ms | 104 MB |
| 5.000 | 5.601 | sí | sí | 86,4 s | 17,5 / 16,7 s | 27,0 / 35,0 ms | 190 MB |
| 5.000 | 5.601 | sí | no | 89,5 s | 16,2 / 18,9 s | 27,6 / 33,4 ms | 190 MB |
| 5.000 | 5.601 | no | sí | 195,1 s | 17,4 / 50,6 s | 84,4 / 98,0 ms | 190 MB |
| 20.000 | 22.401 | sí | sí | 454,6 s | 17,5 / 6,3 s | 34,9 / 40,6 ms | 611 MB |
| 20.000 | 22.401 | sí | no | 438,0 s | 16,2 / 6,4 s | 34,0 / 42,1 ms | 611 MB |

(Las últimas 1.000 operaciones de 20.000 son sobre todo borrados de obras, mucho más baratos.) Sin `ANALYZE`, dentro de la transacción larga el planificador elige el índice de `cloud_library_id` de `catalog_field_versions` en vez de la clave primaria y recorre todas las versiones de la biblioteca: el coste por operación crece con el tamaño (17 → 50 s por cada 1.000 operaciones a 5.000 libros). Con `ANALYZE` es casi plano (unos 15 a 20 ms por operación). Los disparadores cuestan poco (unos 4 a 12 % de la reconstrucción).

La memoria es la del test entero (guarda el estado completo para compararlo); el motor solo ocupa unos 20 a 26 MB.

Un `push` sencillo (cambiar el título de una obra) hace 5 consultas, todas por índice (0,26 a 0,64 ms cada una): la lápida, la fila, las versiones de la fila y dos `UPDATE` (versión y fila). Referencia de la prueba del 08/10/2026: 188 s y 22 ms con 20.000 libros.

**Decidido el 09/10/2026:** no basta. La base de datos ejecuta esas mismas sentencias en 19 a 27 s con 5.000 libros; el resto del tiempo es Doctrine. El motor se reescribe con DBAL (SQL generado desde los metadatos de las entidades) y las versiones de los campos pasan a una columna JSON de cada fila; los tests de este apartado se quedan y son la red del cambio.

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

## Tablas del catálogo (7.4.3, rehecho por D92)

> Las tablas donde el motor de fusión de 7.4.4 escribirá el catálogo de cada biblioteca de la nube. **Hasta 7.4.6 nadie las escribe ni las lee**: `push` y `pull` siguen trabajando solo con `cloud_operation`, y la nube se comporta exactamente como antes. [D92](../DECISIONS.md) (08/10/2026) sustituyó la migración a mano de 7.4.3 por **entidades de Doctrine**; la migración es `Version20261008120000`.

- **Una entidad por tabla**, en `server/src/Entity/Catalog/` (`App\Entity\Catalog`). Todas las filas extienden `CatalogRow` y llevan `key` (UUID v7, clave primaria, **nunca sale del servidor**), `cloud_library_id` y `id` (el de la app, `TEXT`). `CatalogEntity::entityClass()` da la clase de cada entidad y `table()` su tabla (prefijo `catalog_`, para que no choquen con las del servidor). Las entidades tienen *getters* y *setters* (salvo `key`, `cloud_library_id` e `id`, que no cambian).
- **`UNIQUE (cloud_library_id, id)`** en cada tabla (D41 precisada por D92): una biblioteca nunca pisa filas de otra, pero el id de la app ya no va en las claves foráneas. `cloud_library_id` apunta a `cloud_library (id)` con `ON DELETE CASCADE` (D84): al borrar la biblioteca de la nube se borra su catálogo.
- **Claves foráneas de una columna**, a la `key` de la fila referenciada (`work_key`, `publisher_key`…). Reglas de borrado (D89): referencia obligatoria, `CASCADE`; opcional, `SET NULL` (`editions.publisher`, `copies.owner`, `devices.member`), que Doctrine genera sin SQL a mano. Cada clave foránea lleva el nombre de su tabla (`fk_catalog_editions_cloud_library`, `fk_catalog_editions_work_key`…); la de `cloud_library_id` se declara una vez en `CatalogRow` y cada entidad le da su nombre con `AssociationOverrides`. Las tablas de detalles (`book_details`…) llevan además `edition_key` obligatoria; su `id` es el de su edición, y no se usa herencia porque una edición puede tener detalles de dos tipos.
- **Red de seguridad: disparadores «misma biblioteca».** Con claves de una columna, la base de datos ya no impide sola que una fila apunte a otra biblioteca. La migración crea la función `catalog_same_library()` y un disparador `BEFORE INSERT OR UPDATE` por cada referencia y por cada `edition_key` (`<tabla>_<columna>_same_library`), que lanza `23503`. Están escritos a mano en un bloque propio de la migración (el diff de Doctrine no los ve) y el maintainer los revisa. El test hostil de `CatalogRowsTest` se vio fallar sin ellos.
- **Sin `CHECK`** de `role`, `status` ni `rating`: son reglas de `spec/`, y una operación que las incumple no pasa la validación.
- **Contabilidad:** `catalog_field_versions` (el último HLC que escribió cada campo), `catalog_tombstones` (las lápidas, D66) y `catalog_row_authors` (qué dispositivo creó cada fila, D45), con las entidades `FieldVersion`, `Tombstone` y `RowAuthor`. Su `entity` es el valor de `CatalogEntity` (`work`), no el nombre de tabla de la app (`works`). Los HLC son `TEXT COLLATE "C"`. Sin claves foráneas a la fila: una versión o una lápida sobrevive a su fila.
- **Fuera:** lo que en la app es local (`local_state`, `known_servers`, `known_clients`, `library_peers`, `leave_notices`, `request_notices`, `pending_owners`, `forgotten_libraries`, `search_index`), el registro (`operations`: aquí es `cloud_operation`) y las vallas (`device_fences`, `device_fence_chains`): se derivan del registro y llegan con los permisos de 7.4.5.
- **Tests:** `tests/Entity/Catalog/CatalogSchemaTest.php` comprueba contra la base de datos real que cada tabla tiene las columnas de los campos de `CatalogEntity` (más `key`, `cloud_library_id` e `id`), las claves foráneas y sus reglas, el `UNIQUE`, el disparador de cada referencia, que no hay más tablas `catalog_*` y que el mapeo y la base de datos coinciden. `CatalogRowsTest.php` prueba el comportamiento con el ORM: los casos hostiles de otra biblioteca, mismo id en dos bibliotecas, referencias opcionales y borrados en cascada.

### Diferencias con SQLite

| Qué | SQLite | PostgreSQL | Por qué |
|---|---|---|---|
| Clave primaria | `(library_id, id)` o `id` | `key` (UUID v7 del servidor) | D92: Doctrine normal; el id de la app solo es único por biblioteca |
| `library_id` | columna sin clave foránea a `libraries` | `cloud_library_id` con clave foránea a `cloud_library` | La biblioteca de la nube existe antes que su catálogo; el servidor nunca usa el id local (D84) |
| Referencias | `work_id` (id de la app) | `work_key` (clave del servidor) | El motor traduce el id de la app a la fila, siempre dentro de la biblioteca |
| Una fila apunta a otra biblioteca | imposible | los disparadores `catalog_same_library` lo rechazan | Las claves foráneas ya no llevan `cloud_library_id` |
| Campos `cloud_library_id` y `cloud_server` de `library` | columnas de `libraries` | sin columna | El vínculo es el `cloud_library.id` del propio servidor; su versión sí consta en `catalog_field_versions` |
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
