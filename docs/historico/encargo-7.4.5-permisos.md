# Encargo: los permisos en el servidor (hito 7.4.5)

> Encargo cerrado para la sesión `library-sonnet`. Lo prepara y lo revisa la sesión principal. Mientras dure el trabajo vive aquí; al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit `f5d6fb9` dejó el motor de fusión (`server/src/Sync/Merge/`): aplica operaciones al catálogo, sin juzgar a nadie. `CloudPush` guarda las operaciones en `cloud_operation` tras comprobar estructura, firma y cadena; tampoco juzga.

**Qué se construye:** las reglas de permisos de la app, en PHP, y la pieza que juzga cada operación, la aplica o la deja en petición, y vuelve a juzgar la biblioteca cuando algo llega tarde. **Nada lo usa todavía** salvo los tests y `app:catalog:rebuild`: `CloudPush` no se toca (eso es 7.4.6).

Es un **port**, no un diseño nuevo. La referencia manda:

- `crates/domain/src/permission.rs`: las reglas puras (`is_allowed`, `change_allowed`, `LibraryView`).
- `app/src-tauri/src/permissions.rs`: las preguntas contra la base de datos, las vallas y `rebuild`.
- `app/src-tauri/src/sync.rs`, `merge_in`: cuándo se juzga, cuándo algo «llega tarde» y cuándo se rehace.
- `spec/README.md`, «Permissions», y `spec/conformance/permissions.json` (24 casos).

Donde este encargo y la referencia no coincidan, **se para y se pregunta**; no se elige.

## Reglas

- **Solo se toca `server/`** y los ficheros de `docs/` que se nombran abajo. Prohibido tocar `app/`, `crates/`, `spec/` (salvo la frase de `spec/README.md` que se nombra), `docs/DECISIONS.md`, `AGENTS.md` y `server/AGENTS.md`.
- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Nombres del glosario de `AGENTS.md` (`fence`, `applied`, `request`, `member`, `mark`, `revocation`).
- Convenciones de `server/AGENTS.md`. PHPStan al nivel máximo y **sin baseline**.
- Comandos desde `server/` con el `Makefile`. Nada de PHP ni Composer fuera del contenedor.
- **Si el trabajo crece, una regla de aquí choca con el código, o un comando es denegado, se para y se cuenta.** No se busca otro camino para hacer lo denegado.
- **Tests hostiles primero:** donde se dice «visto en rojo», el test se escribe antes de la defensa, se ejecuta, falla, y el informe copia la línea del fallo.
- **SQL:** ninguno nuevo fuera de `CatalogSql` y de la migración. Lo que este encargo lee, lo lee con DQL (ver 3).

## 1 · Esquema

Una migración nueva (no se edita ninguna anterior), generada con `make migration` y revisada:

1. **`cloud_operation.status`**: enum nuevo `App\Enum\OperationStatus` (`Applied = 'applied'`, `Request = 'request'`), columna de texto **anulable**. `null` significa «guardada y aún sin juzgar», que es como quedan todas hasta 7.4.6.
2. **`cloud_operation.previous`**: UUID anulable, la `previous` de la operación. La migración la rellena desde el cuerpo de las filas que ya existen (`body->>'previous'`), y `CloudOperationRepository::insertIfNew()` la guarda. Es el único cambio permitido en lo que hace `push`, y no cambia su resultado.
3. **Dos entidades de contabilidad** en `src/Entity/Catalog/`, como `Tombstone` (readonly, clave compuesta, `cloud_library_id` con `ON DELETE CASCADE`):
   - `Fence` → `catalog_device_fences`: `cloudLibrary`, `operationId` (la operación que pone la valla), `deviceId`, `through`, `untilHlc` (texto con colación `C`). Clave: `(cloud_library_id, operation_id, device_id)`.
   - `FenceChain` → `catalog_device_fence_chains`: `cloudLibrary`, `fenceOperationId`, `deviceId`, `operationId`. Clave: las cuatro.
4. `make migration` no debe proponer nada al terminar. `CatalogSchemaTest` aprende las dos tablas nuevas.

## 2 · Las reglas puras (`server/src/Sync/Permission/`)

- `App\Enum\Role` (`Admin`, `Editor`, `Reader`; `tryFrom` con los textos de `CatalogEntity::ROLES`).
- `LibraryView`: interfaz con las mismas siete preguntas que el trait de Rust, con los mismos nombres en camelCase (`hasRoles`, `roleOfDevice`, `createdBySamePerson`, `requestedBySamePerson`, `memberOfDevice`, `markMember`, `fenced`).
- `Permissions::isAllowed(OperationPayload, LibraryView): bool`: port **línea a línea** de `is_allowed` y `change_allowed`, con los mismos comentarios. Sin base de datos. Incluye que una operación sin firma nunca se permite y que el dueño de una marca se compara sin distinguir mayúsculas.
- Test unitario `PermissionsTest`, sin kernel, con una vista de mentira: los mismos ocho tests del módulo `tests` de `permission.rs`, con los mismos nombres.

## 3 · La vista contra la base de datos (`CatalogLibraryView`)

Responde las siete preguntas para **una** `CloudLibrary`, como `SqlView` en la app.

- **Toda consulta lleva la biblioteca de la nube** como condición. Un id de dispositivo, de miembro, de marca o de operación nunca se busca sin ella (D84).
- **Nunca se cargan entidades del catálogo.** El motor escribe con DBAL por debajo del ORM, así que una entidad en memoria puede estar anticuada. Se lee solo con **DQL que devuelve escalares** (`getSingleScalarResult`, `getScalarResult`, `getOneOrNullResult` con campos sueltos), en un repositorio o en la propia vista.
- `fenced`: las dos mitades de la consulta de Rust. La de vallas, con `Fence` y `NOT EXISTS` sobre `FenceChain`. La de retiradas: las `Revocation` de la biblioteca con ese `deviceId`, leyendo su columna `versions` y comparando en PHP `versions['device_id'] < hlc` (nada de operadores JSON en la consulta).
- `samePerson(a, b)`: el mismo dispositivo, o dos con el mismo miembro no nulo. Mismo test que en Rust (`the_same_person_is_the_same_device_or_two_devices_of_one_member`).
- Quien llama garantiza que el motor no tiene nada en cola (`CatalogMerge::flush()`) antes de preguntar.

## 4 · Quién creó cada fila

`DbalCatalogMerge::apply()` apunta el autor: cuando un cambio `create` acaba en `Written`, encola un `RowAuthor` (biblioteca, tabla de la entidad, id, dispositivo del HLC) **si no existe ya** uno para esa fila; el primero cuenta (`note_author`, `INSERT OR IGNORE`). El «si no existe» se decide como las lápidas: preguntando con `CatalogSql::first()` y recordándolo hasta el siguiente `flush`. Así solo el motor escribe en las tablas `catalog_*` de filas, lápidas y autores.

Test: dos `create` de la misma fila por dos dispositivos dejan como autor al del primero que se aplicó, como en la app, y un `create` que acaba en `Skipped` no apunta nada. `MergeConformanceTest` sigue pasando sin tocarlo.

## 5 · El juez (`server/src/Sync/Permission/CatalogJudge.php`)

Port de la parte de `merge_in` que juzga y de `permissions::rebuild`. No abre ni cierra transacciones.

```php
/** Judges an operation already stored in the log, applies it or leaves it as a request, and records which. */
public function take(CloudLibrary $library, OperationPayload $operation): OperationStatus;

/** Judges the library again from its log if something taken since the last call arrived late where order decides. */
public function settle(CloudLibrary $library): void;

/** Empties the catalog of the library and applies its whole log again, judging each operation at its moment. */
public function rebuild(CloudLibrary $library): void;
```

- **`take`**, en este orden, como `merge_in`: anota si llega tarde (su HLC es menor que el mayor que tenía la biblioteca antes de esta tanda; el mayor se lee una vez por biblioteca y tanda, sin contar la propia operación); juzga con `Permissions` y la vista; si se permite, `CatalogMerge::apply()` y `flush()`; guarda el `status` en su fila de `cloud_operation`; si quedó aplicada, apunta sus vallas (`note_fences`) y, si alguna deja bajo sospecha una operación que ya estaba en el registro, la biblioteca también «llegó tarde».
- **`settle`**: si la biblioteca llegó tarde **y** `judgedByOrder` (tiene roles, vallas o retiradas), `rebuild`. Olvida lo anotado de esa biblioteca.
- **`rebuild`**: el bucle de Rust tal cual. Vallas candidatas de todo el registro; se repite «vaciar, insertar las vallas, recorrer el registro juzgando y aplicando» hasta que ninguna valla candidata quede fuera por venir de una operación que no acabó aplicada. El registro se recorre por páginas (`findPage`), no entero en memoria. No hay avisos de petición ni índice de búsqueda: eso es solo de la app.
- **Vallas:** `fencesOf(operation)` como en Rust (cambios de `device` que no son borrado y traen `trusted_through` de texto). Al insertar una valla se calcula su cadena: las operaciones de ese dispositivo en esa biblioteca desde `through` hacia atrás por `previous`. Se leen `(operationId, previous)` de ese autor con una consulta DQL y se recorre en PHP. Si `through` no está en el registro, la cadena queda vacía.
- `app:catalog:rebuild` pasa a llamar a `CatalogJudge::rebuild()` dentro de su transacción e imprime operaciones aplicadas y peticiones. `FLUSH_EVERY` desaparece si deja de usarse.

## 6 · Tests

1. **`PermissionConformanceTest`**: los 24 casos de `permissions.json`, cada orden de entrega. Por cada operación: se firma con la clave de prueba como hace `MergeConformanceTest`, se guarda en `cloud_operation` (`insertIfNew`), `take` y `settle` (una tanda por operación, como el runner de Rust). Al final, los ids con `status` aplicado y en petición son los de `expect`. Después, `rebuild` y las listas no cambian (`assert_rebuild_changes_nothing`).
   - Las operaciones de `expect.rejected` **no se entregan** al juez: rechazarlas es cosa de `CloudPush` (firma y clave), y se comprobará con el `push` entero en 7.4.6. Un test aparte comprueba que los casos con `rejected` son exactamente los que se esperan (dos hoy), para que uno nuevo no pase sin que nadie lo mire.
2. **Hostiles, vistos en rojo** (cada uno contra una versión de la vista sin la defensa; el informe copia el fallo):
   - **Dos bibliotecas, los mismos ids.** En la biblioteca de Jhonny su dispositivo es de un miembro administrador. En la de la familia existen un dispositivo y un miembro con esos mismos ids de texto pero el miembro es lector. Su operación en la de la familia acaba en petición. Rojo: quitar la biblioteca de la consulta del rol.
   - **Retirar la petición de otro con un id repetido.** En la biblioteca de Jhonny hay una operación con id `X` escrita por su dispositivo; en la de la familia, `X` es una petición de Laura. Jhonny (lector en la de la familia) crea `change_request` `X` con `withdrawn`: acaba en petición. Rojo: quitar la biblioteca de `requestedBySamePerson`.
   - **El ORM no decide con datos viejos.** Con la entidad `Member` de Laura cargada en el `EntityManager` como editora, se entrega su degradación y después una edición suya: la edición acaba en petición. Rojo: una vista que lea el rol con `find()` o `findOneBy()`.
   - **Marca ajena por mayúsculas** no hace falta: lo cubre `PermissionsTest`.
3. **`CatalogJudgeTest`**: una operación que llega tarde en una biblioteca sin roles, vallas ni retiradas **no** provoca `rebuild` (se comprueba contando: el catálogo no se vacía); una valla que llega antes que la operación a la que apunta se vuelve a juzgar cuando esta llega (`a_fence_that_arrives_before_its_operation_is_judged_again_when_it_arrives` de `sync.rs`).
4. `CatalogRebuildCommandTest`, adaptado: las fixtures tienen roles, así que el resultado distingue aplicadas y peticiones.
5. **Carga:** `make stress` con 2.000 y 5.000 libros pasando por el juez (`take` por operación, `settle` cada 100), en una biblioteca **sin roles** y en otra con un administrador y un editor que escribe la mitad. Solo medir y apuntar; el umbral no cambia.

## 7 · Docs

- `docs/architecture/cloud.md`: sección nueva «Permisos en el servidor» (las piezas, qué lee la vista y por qué con DQL de escalares, las dos tablas de vallas, `status` y su `null`, qué no se porta de la app) y las medidas de carga.
- `docs/plans/fase-7.md`: la línea de estado de 7.4.5, sin marcar.
- `spec/README.md`: solo la frase que dice que PHP no ejecuta `permissions.json`; pasa a decir que lo ejecuta contra el juez desde 7.4.5 y que las operaciones `rejected` se comprobarán con el `push` en 7.4.6.

## Qué se entrega

1. `make check` en verde.
2. La tabla de `make stress`.
3. Un informe corto: ficheros tocados, las líneas de fallo de los tres tests vistos en rojo, cualquier diferencia encontrada entre la referencia de Rust y este encargo, y cualquier comando denegado.
