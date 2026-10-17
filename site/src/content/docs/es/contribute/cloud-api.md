---
title: "La nube: la API y las reglas de acceso"
description: "Un mapa de las rutas de /v1 agrupadas por tema, de dónde sale el acceso a una biblioteca, cómo se suben y bajan operaciones y cómo funcionan los avisos en directo."
sidebar:
  order: 13
---

Esta página es un **mapa** de la API del servidor: qué rutas hay, qué hace cada una en una línea y qué reglas las gobiernan. **No es una referencia de cada campo**: para eso está el código, y cada sección dice dónde mirar. Para entender qué es una operación, lee antes [El motor de sincronización](../sync-engine/); para las cuentas, [Cuentas e inicio de sesión](../accounts/).

Todas las rutas cuelgan de `/v1`, hablan JSON y usan `snake_case`. Para ver la lista real y al día, ejecuta `make console C="debug:router"` en `server/`. Los errores de negocio responden `{"error": "<código>"}` (los casos de `AccountErrorCode`), con un `429` y `Retry-After` cuando se alcanza un límite de peticiones.

## La regla de oro: el servidor no se fía de ningún id del cliente

Es la regla que decide todo lo demás (decisión D84). **Los ids que llegan en una petición se comparan, nunca se usan para buscar ni para decidir el acceso**:

- el id local de la biblioteca,
- los ids de las operaciones,
- y los ids de los dispositivos.

*Por qué:* esos ids no son secretos (los ven todos los que comparten una biblioteca), así que quien los conoce no debe poder aprovecharlos. El caso con el que se mide todo es el de una cuenta hostil que conoce todos los ids de otra familia: no debe recibir nada de ella, ni escribir en ella, ni estorbarla.

El acceso sale solo de tres cosas, que el servidor ya tiene:

1. **La cuenta y el dispositivo autenticados** (por el token, no por lo que diga la petición).
2. **La tabla de acceso** (`membership`): qué cuentas pueden entrar en qué biblioteca.
3. **El id propio de la biblioteca en la nube** (`C`, `cloud_library_id`), que **crea el servidor**. Es distinto del id local de la biblioteca: el servidor nunca traduce uno en otro.

Dos consecuencias prácticas:

- **Una biblioteca ajena y una que no existe responden lo mismo**: `404 not_found`. Así nadie puede averiguar qué bibliotecas existen.
- **El mismo id local en dos cuentas son dos bibliotecas de la nube distintas**, con sus propias operaciones y claves.

### De dónde sale el acceso a una biblioteca

La puerta es `CloudLibraryAccess` (en `server/src/Sync/`). Una cuenta llega a una biblioteca si:

- tiene una **membresía** en ella, **activa** (no retirada) y, si la biblioteca tiene roles, **ligada a una persona** de la biblioteca que no tenga el acceso retirado;
- y el **dispositivo** con el que llama no está cortado: no hay una retirada aplicada en la biblioteca que nombre a ese dispositivo con su misma clave.

La nube corta el acceso **al momento**, leyendo el catálogo que guarda, en dos casos: cuando se retira a un dispositivo y cuando se echa a una persona. Y a un dispositivo cortado le enseña la prueba firmada de quien administra; el dispositivo solo borra la biblioteca si esa prueba es buena. Más en [Seguridad](../security/).

Una biblioteca de la nube tiene una **dueña** (la cuenta que la subió), pero solo cuenta para el primer enlace y para el borrado: el acceso de todos, la dueña incluida, sale de `membership`. Si la dueña borra su cuenta, la biblioteca pasa a otra cuenta con un sitio activo, y solo se borra si no queda nadie.

## Las rutas

Todas salvo las marcadas como públicas exigen un dispositivo con sesión (las cabeceras `X-Device-Id` y `Authorization: Bearer`).

### El servidor

| Ruta | Qué hace |
|---|---|
| `GET /v1/server` (pública) | Dice **qué ofrece este servidor**: si admite registro y si es restringido, si entra con Google, si tiene segundo factor, avisos en directo, invitaciones (y la base de sus enlaces) y qué servicios de datos busca. La app decide qué mostrar con esto, en vez de suponer qué funciones existen |

### Cuenta y sesión

Explicadas en [Cuentas e inicio de sesión](../accounts/).

| Ruta | Qué hace |
|---|---|
| `POST /v1/account/register` (pública) | Crea una cuenta sin confirmar y manda un código. Siempre `202` |
| `POST /v1/account/confirm` (pública) | Confirma el correo con el código y fija la contraseña |
| `POST /v1/account/code` (pública) | Pide otro código de confirmación |
| `POST /v1/account/code/action` | Manda a la cuenta un código para una acción delicada (cuentas sin contraseña) |
| `POST /v1/account/password/forgot` y `/reset` (públicas) | Recuperar la contraseña con un código del correo |
| `POST /v1/account/password` | Cambiar la contraseña conociendo la actual |
| `GET` · `PATCH` · `DELETE /v1/account` | Ver, cambiar nombre o idioma, y borrar la cuenta |
| `GET /v1/account/devices` | Lista los dispositivos de la cuenta |
| `DELETE /v1/account/devices/{id}` · `POST …/{id}/restore` | Quitar un dispositivo, y deshacerlo mientras la instalación no se haya enterado |
| `POST /v1/auth/challenge` · `/login` (públicas) | Pedir el reto y entrar probando la clave del dispositivo |
| `POST /v1/auth/logout` | Cerrar la sesión de este dispositivo |
| `POST /v1/auth/google/start` · `/connect` · `/callback` · `/finish` (públicas) | Entrar con Google |
| `POST /v1/account/two-factor/setup` · `/enable` · `/backup-codes`; `DELETE /v1/account/two-factor` | El segundo factor |

### La nube: bibliotecas y sincronización

Todas bajo `/v1/cloud/libraries`.

| Ruta | Qué hace |
|---|---|
| `POST /v1/cloud/libraries` | Crea una biblioteca de la nube para la cuenta, con su membresía y la identidad de la nube. Devuelve su `C` |
| `GET /v1/cloud/libraries` | Lista las que **alcanza este dispositivo** y, aparte, las que la cuenta tenía y de las que la han cortado |
| `POST /v1/cloud/libraries/{C}/link` | Enlaza `C` con el id local de la biblioteca, con una operación firmada. Es un paso aparte antes de subir |
| `POST /v1/cloud/libraries/{C}/push` | **Sube operaciones** (hasta 1.000 por llamada) |
| `POST /v1/cloud/libraries/{C}/pull` | **Baja las operaciones que le faltan** al dispositivo, según su vector de versiones |
| `POST /v1/cloud/libraries/claim` | Enlaza la cuenta con su persona en cada biblioteca donde su dispositivo ya es de esa persona y, en un móvil nuevo, vincula el dispositivo. **No recibe ningún id** |
| `GET /v1/cloud/events` | Da el token para los avisos en directo |

### Invitaciones

| Ruta | Qué hace |
|---|---|
| `POST /v1/cloud/libraries/{C}/invitations` | Crea una invitación (solo quien administra) |
| `GET /v1/cloud/libraries/{C}/invitations` | Lista las de la biblioteca (solo quien administra), sin el enlace ni el correo |
| `DELETE /v1/cloud/libraries/{C}/invitations/{id}` | Anula una (el id se compara con las de `C`) |
| `GET /v1/cloud/invitations/{token}` (pública) | Dice lo justo para «Te invitan a…»: el nombre de la biblioteca, la clase y cuándo caduca |
| `POST /v1/cloud/invitations/{token}/accept` | Acepta, con la cuenta que ha entrado |

### Metadatos: buscar datos a través del servidor

El servidor busca por quien tiene cuenta en servicios de datos con su propia clave. Solo se envía **lo que se busca**; la respuesta tiene siempre una forma pequeña y propia, nunca la del servicio.

| Ruta | Qué busca |
|---|---|
| `GET /v1/metadata/google-books/isbn/{isbn}` | Un libro por ISBN (Google Books) |
| `POST /v1/metadata/tmdb/search` | Películas y series por título (TMDB) |
| `POST /v1/metadata/rawg/search` | Videojuegos por título (RAWG) |
| `POST /v1/metadata/comic-vine/search` | Cómics por título (Comic Vine) |
| `POST /v1/metadata/bgg/search` | Juegos de mesa y libros de rol por título (BoardGameGeek) |

Si el servidor no tiene la clave de un servicio, su ruta responde `404 metadata_unavailable` y `GET /v1/server` no lo lista. El servidor guarda la respuesta unos días, **sin guardar quién preguntó ni el texto buscado** (la búsqueda se guarda bajo una huella), y mantiene un tope de llamadas reales al día al servicio y por cuenta.

## Subir y bajar operaciones

El formato de lo que viaja es el de [la especificación](../operations-spec/). Aquí, las reglas de la ruta.

### Subir (`push`)

Cada operación de la subida pasa por estas comprobaciones, en este orden (`App\Sync\CloudPush`):

1. **Bien formada:** la misma validación que en la app, con los mismos casos de prueba.
2. **¿Ya la teníamos?** La misma operación en la misma `C` se cuenta como conocida y no cambia nada.
3. **Es de esta biblioteca:** su `library` tiene que ser el id local enlazado a `C`, y su **firma** tiene que valer con la clave de ese dispositivo **dentro de esta `C`** (la de su fila `device`, o la que la propia operación registra para sí). Una clave distinta para un dispositivo ya registrado se rechaza.
4. **Cadena de cada autor:** si falta la operación anterior de su autor, la operación **espera** (se rechaza, y tapa a las siguientes de ese autor en la llamada); la app la reenvía completa después.
5. **Un HLC, una operación**, y nada de más de una hora en el futuro.
6. **Se juzga**: los permisos con las mismas reglas que los dispositivos. Lo que no está permitido queda como **petición**: se guarda y se reparte, pero no se aplica.

La respuesta cuenta cuántas se **recibieron**, cuántas **ya se conocían** y cuántas se **rechazaron**. Una operación mala **no** hace fallar el lote: se cuenta como rechazada. Todo va en una transacción que bloquea la fila de `C`, así que dos subidas a la vez (dos administradores) no se pisan. La versión del esquema del cliente tiene que ser la del servidor (`spec/sync.json`) o responde `409 schema_version`.

**Enlazar es un paso propio.** La app envía primero a `link` la operación firmada que enlaza la biblioteca (comprobada contra un dispositivo de la cuenta dueña) y después sube el historial. Así, una biblioteca de más de mil operaciones entra en lotes sin problemas.

### Bajar (`pull`)

El cliente manda su **vector de versiones** (la última marca que tiene de cada autor) y el servidor responde con **todas las operaciones de esa biblioteca con marca mayor**, aplicadas o en petición, ordenadas por marca, y con su propio vector. Es el mismo protocolo que usan dos dispositivos por la red local. Cada dispositivo juzga por su cuenta lo que recibe.

### El motor de fusión

Lo que se sube se aplica al catálogo con el mismo motor de fusión que usa la app, escrito otra vez en PHP (`Sync/Merge/`). **Es la única parte que escribe el catálogo con DBAL en vez de con el ORM de Doctrine**, y todo su SQL sale de una sola clase (`CatalogSql`), a partir de los metadatos de las entidades.

## Los límites de peticiones

Por cuenta, salvo donde se dice: subir y bajar, 600 por hora cada una; enlazar y `claim`, 30 por hora; el token de avisos, 120 por hora; invitaciones, 20 por hora para crearlas, 10 cada 15 minutos para aceptar y 30 cada 15 minutos por IP para mirarlas; metadatos, 600 por hora. Los de las cuentas están en [Cuentas e inicio de sesión](../accounts/).

## Los avisos en directo

Con la app abierta, el servidor puede avisar «hay algo nuevo en esta biblioteca» para que se sincronice al momento. Es **opcional**: sin él, la app sincroniza al abrir, al volver a primer plano y tras cada cambio.

- **Mercure.** Un *hub* aparte (Mercure) mantiene las conexiones abiertas, porque PHP no puede. Symfony le publica por la red interna; la app llega a través del servidor web.
- **El aviso no lleva datos**: dice *que* algo cambió, no *qué*. Lo que cada dispositivo puede ver lo sigue decidiendo `pull`, con la tabla de acceso.
- **Los temas** son cadenas, nunca el id local: `urn:kosmotheka:cloud-library:<C>` (todas las cuentas con acceso a esa biblioteca, tras una subida que trajo algo) y `urn:kosmotheka:account:<id>` (una cuenta concreta, cuando cambia su lista de bibliotecas).
- **El token** (`GET /v1/cloud/events`) no lee nada del cliente: los temas salen de las membresías de la cuenta. Es un JWT firmado con una clave de **suscriptor** distinta de la del publicador (un token de suscriptor nunca puede publicar) y **caduca a los 10 minutos**. Cada vez que se pide uno nuevo se vuelve a leer la tabla de acceso, así que quien pierde el acceso deja de recibir el tema en cuanto renueva, y el hub corta su conexión al caducar el token.
- **Sin Mercure**: la ruta del token responde `409 events_unavailable` y `GET /v1/server` dice `events: false`.
- **Un aviso nunca hace fallar una subida**: se publica después de confirmar la transacción, y si el *hub* falla solo se anota en el registro.

## Dónde mirar

- Las rutas: `server/src/Controller/` (`CloudController`, `InvitationController`, `CloudEventsController`, `MetadataController`, `ServerController`).
- Las reglas de acceso y de `push`: `server/src/Sync/` (`CloudLibraryAccess`, `CloudPush`, `CloudPull`, `CloudLink`, `AccessCutter`, `AccountLinker`).
- Los límites: `server/config/packages/rate_limiter.yaml`.
- Para el detalle de cada ruta y sus casos hostiles, las pruebas: `server/tests/Controller/CloudSyncControllerTest.php` y los demás de esa carpeta.
