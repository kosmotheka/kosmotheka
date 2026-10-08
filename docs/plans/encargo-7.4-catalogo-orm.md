# Encargo: el catálogo del servidor con Doctrine ORM (hitos 7.4.3 rehecho y 7.4.4)

> Encargo cerrado para la sesión `library-sonnet`. Lo prepara y lo revisa la sesión principal. Nace de [D92](../DECISIONS.md) (08/10/2026). Mientras dure el trabajo vive aquí; al terminar pasa a `docs/historico/`.

Son **dos partes, una por encargo**. La parte B no empieza hasta que la A esté revisada.

## Reglas comunes a las dos partes

- **Solo se toca `server/`** y los ficheros de `docs/` y `spec/` que cada parte nombra. Prohibido tocar `app/`, `crates/`, `docs/DECISIONS.md`, `AGENTS.md`, `server/AGENTS.md`, las migraciones que ya existen y `server/spike/` (se puede leer como referencia; no se copia tal cual).
- **Sin commit.** No se hace `git add`, `git commit`, `git stash` ni se cambia de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**.
- Convenciones de `server/AGENTS.md`: `declare(strict_types=1)`, clases `final`, promoción de propiedades en el constructor, comparaciones Yoda, sin `empty()`, excepciones propias, formas de array en phpdoc. PHPStan va al nivel máximo y **sin baseline**: no se silencia ninguna regla.
- Los comandos se lanzan desde `server/` con el `Makefile` (`make help`). Nada de PHP ni Composer fuera del contenedor.
- **Si una parte crece más de lo previsto, o una regla de aquí choca con lo que hay en el código, se para y se cuenta.** No se improvisa una salida.
- **Tests hostiles primero:** donde se dice «visto en rojo», el test se escribe antes del arreglo, se ejecuta, falla, y el informe copia la línea del fallo.

## Vocabulario

| Término | Qué es |
|---|---|
| `key` | La clave propia del servidor de una fila del catálogo (UUID v7). Clave primaria. Nunca sale del servidor |
| `id` | El id que la app dio a la fila. Texto. Único dentro de su biblioteca, no en toda la tabla |
| `CatalogEntity` | El enum `App\Enum\CatalogEntity`: la lista cerrada de entidades que las operaciones pueden cambiar, con sus campos (`fields()`) y su tabla (`table()`) |
| Referencia | Un campo de tipo `FieldType::Ref`: el id de otra fila. Obligatoria (`required`) u opcional |

---

## Parte A · 7.4.3 rehecho: entidades del catálogo

### Qué se construye

Las tablas `catalog_*` pasan de una migración escrita a mano, ocultas al ORM, a **entidades de Doctrine**, una por tabla, con la clave de D92. Nadie escribe ni lee esas tablas todavía: `push` y `pull` no cambian.

### Referencia que leer antes

- `server/src/Enum/CatalogEntity.php` y `server/src/Enum/FieldType.php`: la fuente de qué entidades y campos hay.
- `server/migrations/Version20261004150000.php`: las tablas actuales (lo que se sustituye).
- `server/tests/Sync/CatalogTablesTest.php`: el test de paridad actual (se rehace).
- `server/spike/v2/o2/Entities.php`: el patrón de mapeo que funcionó en la prueba (ocho entidades). Vale la forma del mapeo; el estilo (propiedades públicas, varias clases por fichero) **no**.
- `server/spike/v2/checks.php`: el disparador `same_library` que funcionó.
- `server/src/Entity/Membership.php`: el estilo de entidad de este proyecto.

### Entidades

En `server/src/Entity/Catalog/`, espacio de nombres `App\Entity\Catalog`. Un fichero por clase.

1. **`CatalogRow`**, `#[ORM\MappedSuperclass]`, abstracta. Lo común a toda fila:
   - `key`: `#[ORM\Id]`, tipo `uuid`, columna `key`, creada en el constructor con `Uuid::v7()`.
   - `cloudLibrary`: `ManyToOne` a `App\Entity\CloudLibrary`, columna `cloud_library_id`, `nullable: false`, `onDelete: 'CASCADE'`.
   - `id`: `Types::TEXT`, columna `id`.
   - Constructor `(CloudLibrary $cloudLibrary, string $id)`.
2. **Una clase por caso de `CatalogEntity`** (23), que extiende `CatalogRow`. Nombre de clase: el valor del caso en PascalCase (`work_contributor` → `WorkContributor`, `library` → `Library`, `device` → `Device`). Tabla: la de `CatalogEntity::table()`, sin cambios.
3. En cada clase, `#[ORM\UniqueConstraint(name: 'uniq_<tabla>_library_id', columns: ['cloud_library_id', 'id'])]`.
4. **Campos**, uno por entrada de `fields()`, con esta correspondencia:

   | `FieldType` | Doctrine | PHP | Columna |
   |---|---|---|---|
   | `Text`, `Date`, `OneOf`, `Isbn13` | `Types::TEXT` | `string` | el nombre del campo |
   | `Integer`, `Count` | `Types::BIGINT` | `int` | el nombre del campo |
   | `Rating` | `Types::INTEGER` | `int` | el nombre del campo |
   | `Boolean` | `Types::BOOLEAN` | `bool` | el nombre del campo |
   | `Ref` | `ManyToOne` a la clase de `references` | la clase | el nombre del campo sin `_id`, más `_key` (`work_id` → `work_key`, `related_work_id` → `related_work_key`) |

   - Campo no obligatorio → `nullable: true` y propiedad `?tipo` con valor `null`.
   - Propiedad en camelCase; la de una referencia, sin el `Id` (`work_id` → `$work`, `related_work_id` → `$relatedWork`).
   - Referencia obligatoria → `nullable: false`, `onDelete: 'CASCADE'`. Opcional → `nullable: true`, `onDelete: 'SET NULL'`. `referencedColumnName: 'key'`. Nombre explícito de la clave foránea, como en `Membership`.
5. **Excepciones:**
   - `Library`: los campos `cloud_library_id` y `cloud_server` **no tienen columna** (D84). Solo `name`.
   - Las cinco de detalles (`book_details`, `comic_details`, `boardgame_details`, `film_details`, `tv_series_details`): además de sus campos, una referencia obligatoria `edition` (`ManyToOne` a `Edition`, columna `edition_key`, `CASCADE`). Su `id` es el id de su edición. **No** se usa herencia de Doctrine: una edición puede tener detalles de dos tipos a la vez.
   - `WorkContributor`: además, `UNIQUE (work_key, person_key, role)`.
   - `Revocation.device_id` es texto, sin clave foránea, como hoy.
6. **Sin `CHECK`** de `role`, `status` ni `rating`: son reglas de `spec/` (D92).
7. **Sin colecciones inversas** (`OneToMany`): se añaden cuando alguien las lea.
8. Propiedades privadas con su *getter* y su *setter* (decidido por el maintainer el 08/10/2026, tras revisar la parte A; `key`, `cloudLibrary` e `id` solo se fijan en el constructor). El motor de la parte B, que es genérico, puede seguir escribiendo con los metadatos de Doctrine.
9. Cada clave foránea lleva el nombre de su tabla, también la de `cloud_library_id`: `fk_<tabla>_cloud_library` (decidido el 08/10/2026).
9. **Tres entidades de contabilidad**, que no extienden `CatalogRow`, con las tablas y columnas de hoy:
   - `FieldVersion` → `catalog_field_versions`: clave `(cloud_library_id, entity, entity_id, field)`; `hlc`.
   - `Tombstone` → `catalog_tombstones`: clave `(cloud_library_id, entity, entity_id)`; `hlc`.
   - `RowAuthor` → `catalog_row_authors`: clave `(cloud_library_id, entity, entity_id)`; `device_id`.
   - En las tres, `cloudLibrary` es `#[ORM\Id]` y `ManyToOne` a `CloudLibrary` con `onDelete: 'CASCADE'`; los demás campos de la clave son `#[ORM\Id]` de texto. Sin clave foránea a la fila: una versión o una lápida sobrevive a su fila.
   - Las columnas `hlc` llevan `options: ['collation' => 'C']`: se comparan byte a byte.
10. En `CatalogEntity`, un método nuevo `entityClass(): string` (`class-string<CatalogRow>`) con un `match` exhaustivo. `table()` se queda.

### Configuración

En `server/config/packages/doctrine.yaml`, quitar `schema_filter` y su comentario: el ORM ya ve estas tablas.

### Migración

Una migración **nueva**; la de 7.4.3 no se toca.

1. Primera sentencia: `DROP TABLE` de las 26 tablas actuales (la lista está en `CATALOG_TABLES` de `Version20261004150000`), en una sola sentencia.
2. Después, el SQL que genera Doctrine para las entidades nuevas. Para obtenerlo: `make console C="doctrine:schema:create --dump-sql"` y copiar solo las sentencias de `catalog_*`. No se escribe ese SQL a mano.
3. Al final, a mano, la red de seguridad (ver abajo).
4. `getDescription()` rellenado y sin comentarios generados. `down()` llama a `$this->throwIrreversibleMigrationException()`, con un comentario de una línea: las tablas anteriores no las usó nadie.
5. Comprobación obligatoria tras `make migrate`: `make migration` debe decir que **no hay cambios**, y `make console C="doctrine:schema:validate"` debe dar el mapeo y la base de datos por buenos. Si proponen algo, el mapeo o la migración están mal: se arregla ahí, no se acepta la migración extra.

### Red de seguridad: una fila no apunta a otra biblioteca

Con claves foráneas de una columna, la base de datos ya no lo impide sola. Se recupera con disparadores.

1. **Primero el test hostil, visto en rojo** (sin los disparadores en la migración): con DBAL, crear dos bibliotecas de la nube, una obra en la segunda y una edición en la primera cuyo `work_key` es la obra de la segunda. Hoy entra. El test espera que la base de datos lo rechace.
2. En la migración: una función `catalog_same_library()` en plpgsql y **un disparador por referencia** (`BEFORE INSERT OR UPDATE`, `FOR EACH ROW`), como en `server/spike/v2/checks.php`. Hay un disparador por cada campo `Ref` de cada entidad y uno por el `edition_key` de cada tabla de detalles. Nombre: `<tabla>_<columna>_same_library`.
3. La función usa `format('%I', …)` para el nombre de tabla y recibe tabla y columna como argumentos del disparador. Lanza el error con `ERRCODE = '23503'`.
4. El test pasa a verde. Se comprueba de nuevo que `make migration` no propone nada.

### Tests

`server/tests/Sync/CatalogTablesTest.php` se sustituye por `server/tests/Entity/Catalog/CatalogSchemaTest.php` (contra la base de datos real, como el actual) y `CatalogRowsTest.php` (comportamiento con el ORM). `KernelTestCase` en los dos.

`CatalogSchemaTest`, con `#[DataProvider]` sobre `CatalogEntity::cases()` donde aplique:

| Test | Qué comprueba |
|---|---|
| Columnas | Cada tabla tiene exactamente `key` (uuid), `cloud_library_id` (uuid), `id` (text), una columna por campo escalar con el tipo de la tabla de arriba y una `<x>_key` (uuid) por referencia; nula justo cuando el campo es opcional. `edition_key` en las de detalles |
| Claves foráneas | `cloud_library_id -> cloud_library(id) cascade`; cada referencia `-> <tabla destino>(key)` con `cascade` si es obligatoria y `set null` si es opcional |
| Único por biblioteca | Cada tabla tiene `UNIQUE (cloud_library_id, id)` |
| Clases | `entityClass()` existe, extiende `CatalogRow` y su tabla en los metadatos del ORM es `table()` |
| Ni una más | Las tablas `catalog_*` son las de las entidades y las tres de contabilidad |
| Contabilidad | Las tres llevan la clave foránea a `cloud_library` con `cascade`; `hlc` con colación `C` |
| Disparadores | Cada referencia de cada entidad, y cada `edition_key`, tiene su disparador (consulta a `pg_trigger`) |
| En sincronía | `Doctrine\ORM\Tools\SchemaValidator`: el mapeo es válido y la base de datos coincide con él |

`CatalogRowsTest`:

| Test | Qué comprueba |
|---|---|
| Hostil: otra biblioteca | El del apartado anterior, para una referencia obligatoria (`edition.work`), una opcional (`copy.owner`) y un detalle (`book_details.edition`) |
| Mismo id, dos bibliotecas | La misma fila con el mismo `id` en dos bibliotecas son dos filas |
| Mismo id, una biblioteca | El mismo `id` dos veces en una biblioteca se rechaza |
| Referencia opcional vacía | Guardar con el ORM una edición sin editorial funciona, y `count(['publisher' => null])` la cuenta |
| Borrado opcional | Borrar una editorial deja la edición con `publisher` a null (tras `refresh`) |
| Borrado obligatorio | Borrar una obra se lleva sus ediciones, los ejemplares de estas y sus detalles |
| Dos detalles | Una edición puede tener a la vez `book_details` y `comic_details` |
| Borrar la biblioteca de la nube | Se lleva todas sus filas del catálogo y su contabilidad, y deja intacta la otra biblioteca |

Las filas se crean con el constructor y los *setters*, directamente o con un ayudante de test en `tests/Support/`.

### Documentación

- `docs/architecture/cloud.md`: reescribir el apartado «Tablas del catálogo (7.4.3)» y su tabla «Diferencias con SQLite» con la forma nueva (entidades, `key`, `UNIQUE`, referencias `_key`, disparadores, sin `CHECK`). Decir que D92 lo decidió.
- `docs/plans/fase-7.md`: en «Estado», una línea nueva para «7.4.3 rehecho» con la fecha y lo hecho. No tocar las demás líneas.

### Comprobaciones antes de entregar

```bash
cd server
make migrate && make db-test
make migration        # debe decir que no hay cambios; si crea un fichero, borrarlo y arreglar el mapeo
make console C="doctrine:schema:validate"
make check            # estilo, Rector, PHPStan y PHPUnit, todo en verde
make fixtures         # las fixtures siguen cargando
```

### Informe de entrega

1. Ficheros creados, cambiados y borrados.
2. La línea del fallo del test hostil visto en rojo.
3. La salida de `make migration` y de `doctrine:schema:validate`.
4. La última línea de `make check`.
5. Cualquier punto donde el encargo no encajó con el código, y qué se hizo.

---

## Parte B · 7.4.4: el motor de fusión con el ORM

> Se lanza cuando la parte A esté revisada. Los nombres de clases y métodos de las entidades son los que hayan quedado en A.

### Qué se construye

1. Un **motor de fusión** que aplica una operación al catálogo de una biblioteca de la nube, con las entidades de la parte A y el `EntityManager`. Sin SQL.
2. Un **comando de consola** que reconstruye el catálogo de una biblioteca desde su registro (`cloud_operation`).
3. Los casos de `spec/conformance/merge.json` y `library-merge.json` pasando en PHP.
4. Una **prueba de carga** con una biblioteca grande inventada.

`CloudPush` y `CloudPull` **no se tocan**: el motor no se enchufa al `push` hasta 7.4.6. El servidor sigue comportándose igual.

### Referencia que leer antes

- `spec/README.md`, apartados «Merging operations from several libraries» y «Merging operations within a library»: **son las reglas**. Si este encargo y `spec/` discrepan, manda `spec/` y se avisa.
- `app/src-tauri/src/sync.rs`, funciones `apply_change` y `without_deleted_references`: la implementación de referencia. Se copian las **reglas**, no la forma: Rust deja que SQLite falle y deshace con un *savepoint*; aquí no se puede (ver «Nunca un `flush` que falle»).
- `app/src-tauri/src/ops.rs`, `delete_library`, `delete_row` y `write_versions`: qué se borra y qué versión o lápida se escribe.
- `server/spike/v2/OrmEngine.php`: el esqueleto que funcionó en la prueba. **Sus reglas son una versión reducida y en dos puntos no coinciden con las reales** (ver la tabla de reglas); vale como idea de estructura.
- `server/src/Dto/OperationPayload.php` y `ChangePayload.php`: la forma de una operación ya validada.

### Diseño

En `server/src/Sync/Merge/`:

- **`CatalogMerge`** (interfaz): aplica una operación ya validada (`OperationPayload`) a una `CloudLibrary` y devuelve, por cada cambio, si escribió algo o se saltó. Más un método para volcar lo pendiente (`flush`). La interfaz existe para poder cambiar la implementación sin tocar a quien la llama (D92).
- **`OrmCatalogMerge`**: la implementación, con `EntityManagerInterface`. Registrada como el servicio de la interfaz.
- El resultado de un cambio es un enum, no una cadena ni un booleano suelto.
- **Guiado por `CatalogEntity`**, sin un `switch` por entidad: la clase sale de `entityClass()`, los campos de `fields()`, y los valores se leen y escriben con `ClassMetadata::getFieldValue` y `setFieldValue`. Nada de propiedades dinámicas (`$row->{$name}`).
- **Ningún texto del cliente llega a un nombre de clase, propiedad o campo.** El nombre de entidad y el de campo solo sirven para buscar en `CatalogEntity`; lo que se usa después es el caso del enum y la clave de `fields()`.
- **El id de la app se resuelve siempre dentro de la biblioteca**: toda búsqueda de una fila lleva `cloudLibrary` y `id`. No existe ninguna búsqueda solo por `id`.

### Reglas del motor

Por cada cambio de la operación, en este orden. `hlc` es el de la operación.

| # | Situación | Qué pasa |
|---|---|---|
| 1 | La biblioteca tiene lápida (entidad `library`) | Se salta, sea cual sea el HLC |
| 2 | La fila tiene lápida | Se salta: el borrado gana |
| 3 | Borrado de una fila | Se borra la fila si existe (la base de datos se lleva lo que cuelga y anula las referencias opcionales, D89), se borran las versiones de campo de esa fila y se escribe su lápida con el `hlc`, **exista o no la fila** |
| 4 | Borrado de la `library` | Se borra todo el catálogo de esa biblioteca de la nube y también sus versiones de campo, sus demás lápidas y sus autores de fila; queda solo la lápida de la biblioteca. **Nunca se borra la fila de `cloud_library`** ni el registro `cloud_operation` |
| 5 | Actualización de una fila que no existe | Se salta (nunca creada, o se fue con su padre) |
| 6 | Referencia opcional a una fila **con lápida** | El campo se aplica como null y su versión se escribe como siempre; el resto del cambio sigue |
| 7 | Referencia (obligatoria u opcional) a una fila que **no existe y no tiene lápida** | Se salta el cambio entero. *El motor de la prueba la ponía a null: es incorrecto* |
| 8 | Referencia obligatoria a una fila con lápida | Se salta el cambio entero |
| 9 | Detalle (`*_details`) cuya edición no existe | Se salta |
| 10 | Creación a la que le falta un campo obligatorio | Se salta |
| 11 | `work_contributor` que repite `(work, person, role)` de otra fila | Se salta |
| 12 | Campo a campo | Un campo se escribe solo si no tiene versión o si `hlc` es **mayor** que la guardada (comparación de texto, byte a byte). Un empate no escribe |
| 13 | Creación de una fila que ya existe | Se trata como una actualización: campo a campo |
| 14 | Fila que existe y ningún campo gana | No se escribe nada |
| 15 | Versiones | Solo se escribe la versión de los campos que ganaron. Un campo que perdió conserva la suya |
| 16 | Un cambio saltado | No escribe nada: ni fila, ni versión, ni lápida. Los demás cambios de la operación se aplican |
| 17 | Campos `cloud_library_id` y `cloud_server` de `library` | No tienen columna; su versión sí se guarda |
| 18 | Columna `entity` de versiones y lápidas | El valor de `CatalogEntity` (`work`), no el nombre de tabla de la app (`works`) que usa Rust |

Las reglas 7 a 11 son lo que en SQLite hace una restricción al fallar. **La tabla no se da por completa:** hay que repasar todas las restricciones de las tablas del catálogo (`NOT NULL`, claves foráneas, `UNIQUE`, disparadores) y comprobar que cada una tiene su regla aquí. El informe lleva esa lista: restricción → dónde se comprueba antes de escribir. Si aparece una que no está en la tabla, se añade y se avisa.

### Nunca un `flush` que falle

Si un `flush` lanza una excepción, Doctrine cierra el `EntityManager` y la petición entera se pierde. Por eso **toda situación de las reglas 5 a 11 se comprueba antes de persistir**. El motor nunca usa la excepción de la base de datos para decidir.

Test que lo vigila: tras aplicar cada caso de `spec/` y cada test hostil, `EntityManager::isOpen()` es `true`.

### Lo que hay en memoria y aún no en la base de datos

- Una operación crea la obra y, en el cambio siguiente, su edición: la obra aún no se ha volcado. El motor recuerda las filas, versiones y lápidas que ha tocado desde el último `flush`, como el `seen` de `server/spike/v2/OrmEngine.php`, y busca ahí antes de ir al repositorio.
- **Tras un borrado**, la base de datos borra en cascada y anula referencias: lo que hay en memoria deja de valer. El motor hace `flush`, **desvincula con `detach()` las filas del catálogo que cargó** y vacía su memoria.
- **Prohibido `EntityManager::clear()` dentro del motor:** en 7.4.6 se ejecutará dentro de un `push`, con la cuenta y la biblioteca de la petición en el mismo `EntityManager`.
- El motor **no abre ni cierra transacciones**: lo hace quien lo llama.

### Comando

`app:catalog:rebuild <id de la biblioteca de la nube>`, en `server/src/Command/`, con `#[AsCommand]`.

1. En una transacción: borra el catálogo y la contabilidad de esa biblioteca, y aplica su registro (`cloud_operation`) en orden de HLC.
2. Hace `flush` cada 100 operaciones (una constante) y entonces sí puede vaciar el `EntityManager`: el proceso es suyo.
3. Lee el registro por tramos, no entero en memoria. Si `CloudOperationRepository` no tiene un método para eso, se le añade.
4. Imprime operaciones aplicadas, cambios saltados y tiempo.
5. Reconstruir dos veces deja exactamente el mismo catálogo.

### Tests

En `server/tests/Sync/Merge/`.

1. **Hostiles, primero y vistos en rojo.** Se escriben contra una primera versión del motor que aún no tenga la defensa:
   - Una operación con una entidad o un campo que no está en la lista (`work"; DROP TABLE catalog_works; --`), entregada directa al motor, sin pasar por la validación: se salta y las tablas siguen ahí.
   - Una edición de la biblioteca A cuyo `work_id` es el id de una obra que solo existe en la biblioteca B: se salta, y ninguna fila de A apunta a B.
   - El mismo `id` en dos bibliotecas: un cambio en A no toca la fila de B.
2. **Conformidad:** `merge.json` y `library-merge.json`, con `App\Tests\Support\Spec`. Por cada caso y **por cada orden** de `deliveries`: bibliotecas de la nube nuevas (una por cada `library` distinta del caso), se entrega cada operación sola al motor en ese orden, y se compara con `expect.rows`:
   - las filas se leen con el ORM y se pasan a la forma de `spec/`: `{library, entity, id, fields}`, con las referencias traducidas al `id` de la app;
   - una fila que no está en `expect.rows` no debe existir, en ninguna entidad;
   - un campo que no está en `fields` debe ser null.
   - Los casos con `expect.rejected` **no vacío** tratan de operaciones rechazadas antes de guardarse, que es cosa de `CloudPush` y no del motor: se saltan con una lista explícita de nombres y su motivo, y un test comprueba que esa lista es exactamente la de los casos con `rejected` no vacío. Se cubren en 7.4.6.
3. **Reconstrucción:** con las fixtures (`ScenarioFixtures`), el comando reconstruye cada biblioteca; el resultado tiene filas, y es idéntico al de aplicar las operaciones una a una y al de reconstruir otra vez.
4. **Carga:**
   - Un generador de una biblioteca inventada en `tests/Support/`, con el número de libros como parámetro: por libro, una operación con obra, edición (una de cada siete sin editorial), ejemplar y detalles; después una edición de título cada diez libros y un borrado de obra cada cincuenta. Como `history()` de `server/spike/v2/volume.php`.
   - En la suite normal, con 300 libros: se aplica con `flush` cada 100 operaciones y de una en una, y los dos catálogos son idénticos; los recuentos de filas son los esperados.
   - Fuera de la suite normal (`#[Group('stress')]`, excluido en `phpunit.dist.xml`), con 20.000 libros: lo mismo, e imprime el tiempo de reconstruir y la mediana de 30 `push` de dos operaciones sobre la biblioteca ya llena. Objetivo `make stress` nuevo en el `Makefile`.
   - **Referencia de la prueba del 08/10/2026:** 188 s para reconstruir y 22 ms por `push`. La carga no falla por tiempo, pero si reconstruir pasa de 300 s o el `push` de 50 ms, se dice en el informe con los números: es el dato con el que se decide si hace falta una implementación con DBAL.

### Documentación

- `docs/architecture/cloud.md`: un apartado «Motor de fusión (7.4.4)»: qué hace, las reglas (enlazando `spec/README.md`, sin repetirlas), por qué se comprueba antes de escribir, el comando y cómo lanzar la carga con sus números.
- `spec/README.md`: en la frase que dice qué ejecuta cada implementación, que PHP ejecuta ya los casos de fusión contra el motor, y que el caso de HLC repetido lo cubrirá al enchufarlo al `push` (7.4.6). Solo esa frase.
- `docs/audits/hallazgos.md`: CON-001 pasa a cerrado, con la fecha.
- `docs/plans/fase-7.md`: en «Estado», la línea de 7.4.4.

### Comprobaciones antes de entregar

```bash
cd server
make db-test
make check
make stress           # copiar los números al informe
make fixtures && make console C="app:catalog:rebuild <id de una biblioteca de las fixtures>"
cd ../app && npm run check:spec
```

### Informe de entrega

1. Ficheros creados y cambiados.
2. Las líneas de los tres tests hostiles vistos en rojo.
3. La lista «restricción → dónde se comprueba antes de escribir».
4. Casos de `spec/` ejecutados, saltados y por qué.
5. Los números de `make stress`.
6. La última línea de `make check`.
7. Cualquier punto donde el encargo o `spec/` no encajaron con el código, y qué se hizo.
