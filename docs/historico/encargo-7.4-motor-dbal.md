# Encargo: el motor de fusión con DBAL y las versiones en la fila (hito 7.4.4, segunda vuelta)

> Encargo cerrado para la sesión `library-sonnet`. Lo prepara y lo revisa la sesión principal. Nace de la revisión de [D92](../DECISIONS.md) del 09/10/2026. Mientras dure el trabajo vive aquí; al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit `532f6a8` dejó el motor con el ORM (`OrmCatalogMerge`), el comando `app:catalog:rebuild` y sus tests. La carga mostró que no basta (unos 450 s para 20.000 libros). Una prueba desechable (`server/spike/v4`) con DBAL y las versiones en una columna JSON pasó los 144 casos de conformidad, dio el mismo estado final y tardó 76 s, con un `push` de 5 a 6 ms.

**Qué se construye:** se sustituye `OrmCatalogMerge` por un motor con DBAL detrás de la misma interfaz `CatalogMerge`, y las versiones de los campos pasan de la tabla `catalog_field_versions` a una columna `versions` de cada fila. Los tests que ya existen son la red: tienen que seguir pasando, adaptados solo donde leen las versiones.

## Reglas

- **Solo se toca `server/`** y los ficheros de `docs/` que se nombran abajo. Prohibido tocar `app/`, `crates/`, `spec/`, `docs/DECISIONS.md`, `AGENTS.md`, `server/AGENTS.md` y `server/spike/` (se lee como referencia).
- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**.
- Convenciones de `server/AGENTS.md`. PHPStan al nivel máximo y **sin baseline**: no se silencia ninguna regla.
- Comandos desde `server/` con el `Makefile`. Nada de PHP ni Composer fuera del contenedor.
- **Si el trabajo crece, una regla de aquí choca con el código, o un comando es denegado, se para y se cuenta.** No se busca otro camino para hacer lo denegado.
- **Tests hostiles primero:** donde se dice «visto en rojo», el test se escribe antes de la defensa, se ejecuta, falla, y el informe copia la línea del fallo.

## Referencia que leer antes

- `server/spike/v4/CatalogSql.php` y `server/spike/v4/DbalCatalogMerge.php`: la forma que funcionó. **No se copian tal cual:** no pasan PHPStan, y el motor de la prueba lleva un modo «biblioteca entera en memoria» que **no se construye** (todo lo que cuelga de `$whole`: `referrers`, `contributors`, `release()`, `known()`, `forgetRow()`, `refer()`).
- `server/src/Sync/Merge/OrmCatalogMerge.php`: las reglas, con sus comentarios numerados. Las reglas no cambian.
- `docs/DECISIONS.md`, D92, «Revisión del 09/10/2026».

## 1 · Esquema

1. `CatalogRow` gana la propiedad `versions`: `#[ORM\Column(name: 'versions', type: Types::JSON, options: ['jsonb' => true])]`, `array<string, string>` (nombre del campo → HLC que lo escribió por última vez), valor inicial `[]`, con `getVersions()` y `setVersions()`.
2. Se borra la entidad `FieldVersion` y la tabla `catalog_field_versions`. `Tombstone` y `RowAuthor` no cambian.
3. **Se edita la migración `Version20261008120000`** (no está publicada ni desplegada), no se añade otra: la columna `versions JSONB NOT NULL` en las 23 tablas y fuera la tabla de versiones. De paso, su `down()` debe borrar también la función `catalog_same_library()`, que hoy sobrevive.
4. Para reaplicarla en las bases de datos de desarrollo y de tests hay que deshacerla y volver a aplicarla. Son tablas que nadie usa todavía. **Si el comando es denegado, se para y se avisa:** lo autoriza el maintainer.
5. `make migration` no debe proponer ningún cambio al terminar.

## 2 · `CatalogSql` (`server/src/Sync/Merge/CatalogSql.php`)

La **única clase del motor que contiene SQL**. Construye sentencias desde el `ClassMetadata` de una entidad:

- Recibe siempre una clase de entidad y nombres de **propiedad** (campo o asociación a uno). Tabla, columnas y tipos salen del mapeo. Una propiedad que no está en el mapeo lanza una excepción propia, sin ejecutar nada.
- Los identificadores se entrecomillan con la plataforma (`quoteSingleIdentifier`). Los **valores van siempre como parámetros enlazados**, con el tipo DBAL de su columna. Nunca se concatena un valor en el SQL.
- Operaciones: insertar una fila (todas las columnas de la clase, en orden fijo, `null` en las que no se dan), actualizar por condición, borrar por condición y leer la primera fila que cumple una condición.
- Las sentencias preparadas se reutilizan por texto de SQL.
- No sabe nada de reglas de fusión ni de `CatalogEntity`.

## 3 · `DbalCatalogMerge` (`server/src/Sync/Merge/DbalCatalogMerge.php`)

Implementa `CatalogMerge`; la interfaz no cambia. Se borra `OrmCatalogMerge`.

- **Mismas reglas y mismo orden** que `OrmCatalogMerge`, incluido que las comprobaciones valen solo para los campos que ganan por HLC.
- **Todo se juzga antes de escribir.** En PostgreSQL una sentencia que falla deja la transacción inservible, así que la base de datos sigue siendo la red de debajo, nunca lo que decide.
- **Cola de sentencias:** las reglas deciden y encolan; `flush()` las ejecuta en orden y olvida lo leído. No abre ni cierra transacciones.
- **Lo que recuerda entre dos `flush`:** de cada fila tocada, su `key`, sus `versions`, las claves a las que apunta y, en `work_contributor`, su `role`; y las lápidas consultadas. Es lo que permite decidir sobre filas que están en la cola y aún no en la base de datos.
- **Una fila se lee siempre con su biblioteca** (`cloud_library_id` e `id`): un id de la app nunca se resuelve fuera de ella. Nada del cliente llega a un nombre de clase, propiedad o columna.
- **Escribir:** crear es un `INSERT` con los campos ganadores y `versions`; actualizar es un `UPDATE` de los campos ganadores y de `versions` entera, por `key`. Una sentencia por cambio.
- **Borrar:** se ejecuta la cola, se borra la fila (la base de datos borra en cascada y anula referencias) y se olvida todo lo leído de esa biblioteca. Después, la lápida.
- **Autor repetido (regla 11):** se ejecuta la cola y se pregunta a la base de datos por otra fila con el mismo trío.
- No usa el `EntityManager` para leer ni escribir filas; solo para los metadatos y la conexión.

## 4 · Comando y carga

- `app:catalog:rebuild` deja de ejecutar `ANALYZE` (venía de la tabla de versiones) y ya no necesita vaciar el `EntityManager` cada tramo. `FLUSH_EVERY` se queda, para que la cola no crezca sin límite.
- `make stress` pierde `NO_ANALYZE`. `NO_TRIGGERS` se queda.
- `LibraryHistory` gana autores, para que la regla 11 se mida bajo carga: 100 personas al principio y, en la operación de cada libro, un `work_contributor` (su obra, la persona `i % 100`, `author`, posición 1). Se ajustan los recuentos esperados.

## 5 · Tests

Se adaptan, sin perder ninguna comprobación: `CatalogSchemaTest` (columna `versions` en cada tabla, sin tabla de versiones), `CatalogRowsTest`, `CatalogState` (las versiones salen de las filas), `CatalogMergeHostileTest`, `CatalogMergeReferencesTest`, `CatalogLoadTest`, `CatalogRebuildCommandTest`. `MergeConformanceTest` no debería necesitar cambios.

Tests nuevos:

1. **`CatalogSqlTest`, hostil:** una propiedad que no está en el mapeo (incluida una con SQL dentro) lanza la excepción y no ejecuta nada.
2. **Hostil, por el motor:** un cambio cuyo `id` y cuyo título son `x'); DROP TABLE catalog_works; --` crea una fila con ese id y ese título literales; las tablas siguen ahí y la otra biblioteca del test no cambia.
3. **Visto en rojo:** el test 2 se escribe primero contra una versión de `CatalogSql` que concatene el valor, se ve fallar y se corrige. El informe copia la línea del fallo.
4. Una fila creada y actualizada en el mismo `flush` (sin pasar por la base de datos entre medias) acaba con los valores y versiones correctos.

## 6 · Docs

- `docs/architecture/cloud.md`: «Tablas del catálogo» (columna `versions`, sin tabla de versiones), «Motor de fusión» reescrito para el motor nuevo y «Lo que midió la carga» con las medidas nuevas, dejando una línea con las del motor anterior.
- `docs/plans/fase-7.md`: la línea de 7.4.4.

## Qué se entrega

1. `make check` en verde.
2. `make stress` con 2.000, 5.000 y 20.000 libros, con y sin disparadores: la tabla de resultados.
3. Un informe corto: ficheros tocados, las líneas de fallo de los tests vistos en rojo, cualquier cosa que no encajara con este encargo y cualquier comando denegado.
