# Encargo: el `push` aplica y juzga (hito 7.4.6)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit `bc244b9`. El servidor tiene el motor de fusión (`server/src/Sync/Merge/`) y el juez (`server/src/Sync/Permission/CatalogJudge.php`), y **nada los llama** salvo los tests y `app:catalog:rebuild`. `CloudPush` comprueba estructura, firma, clave y cadena, y guarda la operación en `cloud_operation` con `status` nulo.

**Qué se construye:** `CloudPush` entrega al juez cada operación que guarda, de modo que la nube tiene el catálogo de cada biblioteca y sabe qué operaciones quedaron en petición. Es la primera parte de 7.4 que **cambia lo que hace la nube**.

**Qué no cambia:** lo que la app ve. `pull` sigue repartiendo todas las operaciones del registro, aplicadas o en petición, porque cada dispositivo las juzga por su cuenta (D73). `PushResult` sigue contando `received`, `known` y `rejected`; una operación que queda en petición está guardada y cuenta como `received`.

Referencias: `app/src-tauri/src/sync.rs` (`merge_in`), `spec/README.md` («Permissions»), `docs/architecture/cloud.md` («Permisos en el servidor»), y el encargo anterior en [`docs/historico/encargo-7.4.5-permisos.md`](../historico/encargo-7.4.5-permisos.md).

## Reglas

- **Solo se toca `server/`** y los ficheros de `docs/` que se nombran abajo. Prohibido tocar `app/`, `crates/`, `spec/`, `docs/DECISIONS.md` (D93 ya está escrita), `AGENTS.md` y `server/AGENTS.md`.
- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Nombres del glosario de `AGENTS.md`.
- Convenciones de `server/AGENTS.md`. PHPStan al nivel máximo y sin baseline.
- Comandos desde `server/` con el `Makefile`. Nada fuera del contenedor.
- **Si el trabajo crece, una regla de aquí choca con el código, o un comando es denegado, se para y se cuenta.** No se busca otro camino para hacer lo denegado.
- **Tests hostiles primero:** donde se dice «visto en rojo», el test se escribe antes de la defensa, se ejecuta, falla, y el informe copia la línea del fallo.
- **El servidor no se fía de ningún id del cliente (D84).** Quién sube se saca de la sesión, nunca de la petición.
- **SQL:** ninguno nuevo fuera de `CatalogSql`, de `CloudOperationRepository` (que ya escribe el registro con SQL) y de la migración. Parámetros siempre enlazados.

## 1. Quién subió cada operación

- Migración nueva: dos columnas en `cloud_operation`.
  - `pushed_by_user_id`, UUID anulable, foránea a la cuenta con `ON DELETE SET NULL`: al borrar una cuenta el registro se queda y el dato se vacía.
  - `pushed_by_device_id`, texto anulable: el id del dispositivo de la cuenta que hizo el `push`. Sin foránea, para que quitar el dispositivo no lo borre.
- Las dos quedan nulas en las filas que ya existen. No se rellenan con el autor: quien firma y quien sube son personas distintas cuando la operación llegó antes por la red local.
- `CloudPush::push()` recibe el dispositivo autenticado (el de la sesión, del que sale la cuenta) y `insertIfNew` guarda los dos datos. El controlador lo saca de la sesión.
- Una operación ya guardada (`known`) **no** cambia su dato: cuenta quien la subió primero.
- Test: dos dispositivos de dos cuentas con acceso a la misma biblioteca suben cada uno una operación firmada por un tercero; cada fila guarda quien la subió. Y el caso `known`.

## 2. El `push` entrega al juez

- En `CloudPush::apply()`, cada operación que `accept()` guarda (`received`) se pasa a `CatalogJudge::take()`. El bucle ya recorre el lote en orden de HLC, que es lo que `take` pide. Al final del lote, un `CatalogJudge::settle()`.
- Todo dentro de la transacción que ya abre `push()`, con el bloqueo de la biblioteca que ya existe. El juez no abre transacciones.
- Las operaciones `known` y las `rejected` no llegan al juez.
- El aviso (`SyncNotifier`) no cambia: después del commit, si se guardó algo.
- El test de los lotes (`testALibraryUploadedInBatchesOf1000WithTheLinkLastIsStoredWhole`) ya está escrito y en rojo. Se pone en verde con el punto 2b, llamando antes a la ruta de enlace.

## 2b. Enlazar es un paso propio (D93, añadido el 09/10/2026)

Aceptado por el mantenedor: la primera subida tiene que funcionar a la primera.

- Ruta nueva `POST /v1/cloud/libraries/{id}/link`. Cuerpo: una operación con su versión de esquema, con la misma forma que un elemento de `operations` en `push`.
- Acceso como en `push`: por la cuenta y la tabla de acceso (`CloudLibraryAccess`); sin acceso, `404` como en las demás rutas.
- Comprueba **lo mismo que hoy `CloudPush::link()`**, sin relajar nada: la operación se lee con `OperationReader`, es el enlace de esta biblioteca de la nube (`isLink()`), la firma un dispositivo de la cuenta dueña buscado por la cuenta y el id del HLC, ese dispositivo no está quitado, y la firma vale. Entonces enlaza la biblioteca. **No guarda la operación**: llegará con el registro, en su sitio.
- Repetirla con la biblioteca ya enlazada al mismo id local responde igual que la primera vez y no cambia nada. Con la biblioteca enlazada a otro id local, se rechaza.
- Respuesta y errores por el camino que ya usa `CloudController`. Si no hay un código que encaje, propón uno en el informe; no inventes un formato nuevo.
- El código de enlace se comparte entre la ruta y `CloudPush` (una clase propia o un método público), sin duplicarlo. El `push` **sigue enlazando como hoy** cuando el enlace viene en el lote: las apps ya instaladas no cambian.
- Tests por la ruta, escritos antes que la ruta:
  - Jhonny, con cuenta propia, envía a la biblioteca de la familia un enlace firmado por su dispositivo: `404`, sigue sin enlazar.
  - Alguien con acceso pero que no es de la cuenta dueña firma el enlace: rechazado.
  - Un dispositivo quitado de la cuenta dueña: rechazado.
  - Un enlace que nombra otra biblioteca de la nube, y una operación que no es un enlace: rechazados.
  - El caso bueno, y su repetición.
  - El de los 2.500: enlace por la ruta y después tres lotes: `received` 1000/1000/500 y nada rechazado.
  - El de las dos pasadas (`testTheSameThreeBatchesTwiceReportWhatEachPassDoes`) pasa a tener aserciones y se queda como el test de «una app anterior»: sin la ruta, la primera pasada rechaza todo y la segunda lo guarda.
- La parte de la app (llamar a la ruta en `upload`) **no es de este encargo**.

## 3. Quién puede subir una biblioteca (SEG-013, D93)

Aceptado por el mantenedor el 09/10/2026 ([D93](../DECISIONS.md)).

- El enlace es un cambio de la fila `library`, que con roles solo puede hacer un administrador. El juez ya lo dice: la operación de enlace queda `applied` o `request`.
- **Regla:** si al acabar un `push`, después de `settle`, la operación que enlaza esta biblioteca de la nube está en `request`, el `push` entero se deshace y no se guarda nada de ese `push`. **Nada más se borra:** los lotes anteriores se quedan y la biblioteca no se vacía (D93 explica por qué).
- La operación de enlace es la del registro que cumple `CloudPush::isLink()` para esta biblioteca. Se busca **dentro de la biblioteca de la nube**, nunca por un id que mande el cliente. Si hace falta guardar cuál es, una columna anulable en `cloud_library` con el id de esa operación, escrita por `link()`.
- **Respuesta:** un error de la petición entera, `403` con el código `link_not_allowed`, por el mismo camino que los demás errores de la nube (`AccountException` o el equivalente que ya use `CloudController`). **No** es un caso nuevo de `OperationRejection`: esas razones dependen solo de lo que dice la operación (DAT-001) y esta depende del registro.
- `wrapInTransaction` cierra el `EntityManager` cuando se deshace. Hay que comprobar que la respuesta de error sale bien y que ningún `flush` posterior de la petición falla.
- Se comprueba en **cada** `push`, no solo en el que enlaza: un `push` posterior que trae las operaciones que dan los roles puede hacer que la biblioteca se rehaga y el enlace pase a petición.
- **Tests hostiles, vistos en rojo**, por la ruta:
  - Un lector de la familia sube en un solo `push` la biblioteca entera (con los roles) y su propio enlace: `403`, nada guardado.
  - El mismo lector enlaza por la ruta y sube en lotes: los lotes sin el enlace entran, el que lo trae da `403` y no guarda nada.
  - El mismo lector sube primero solo la parte sin roles y su enlace (entra: es la copia que D93 acepta), y después las operaciones que dan los roles: ese segundo `push` da `403` y no guarda nada.
  - La administradora sube la suya con roles: entra, y su enlace queda `applied`.
  - Una biblioteca sin roles la sube cualquiera.

## 4. Reconstruir lo que ya estaba

- `app:catalog:rebuild` gana `--all`: reconstruye cada biblioteca de la nube, **de una en una y cada una en su transacción**, e imprime una línea por biblioteca. Si una falla, sigue con las demás y termina con error.
- Las operaciones guardadas antes de este hito tienen `status` nulo. Se documenta en `docs/development/server.md` que al desplegar 7.4.6 se ejecuta una vez.
- Test: tres bibliotecas con registro y sin catálogo; tras `--all`, las tres tienen catálogo y ninguna operación con `status` nulo.

## 5. Tests

- `spec/conformance/permissions.json` **por la ruta de `push`**: los 24 casos, en cada orden de entrega, comprobando `applied`, `requests` y también `rejected` (los dos casos de `CASES_WITH_REJECTED` dejan de saltarse aquí).
- `merge.json`: el caso que `MergeConformanceTest` salta («two operations with one HLC…») se cubre por la ruta.
- **Batería de Jhonny** (`CloudSyncControllerTest`), hostiles **vistos en rojo**:
  - Jhonny, con cuenta propia y conociendo los ids de la familia, sube a su biblioteca operaciones con los ids de dispositivo y de miembro de la familia: el catálogo y los estados de la biblioteca de la familia no cambian.
  - Un lector de la familia sube un cambio firmado por él: queda guardado, en `request`, y el catálogo no cambia.
  - Un editor degradado sube un cambio con fecha anterior a la degradación: queda en `request` (SEG-014 por la ruta).
  - Los del punto 2b y los del punto 3.
- `ScenarioFixtures`: después de cargarlas, cada biblioteca tiene su catálogo en las tablas `catalog_*`, para poder mirarlas. Se añade un escenario con un lector cuya operación queda en petición.
- `make stress`: el `push` de la prueba de carga pasa por `CloudPush` de verdad (firma incluida) en vez de llamar al juez. Se mide con 2.000 libros, dos veces, una tras `VACUUM`.

## 6. Documentación

- `docs/architecture/cloud.md`: la ruta de enlace; «Permisos en el servidor» y la descripción de `push` dejan de decir «nada lo llama»; columnas nuevas; `--all`.
- `docs/plans/fase-7.md`: la línea de 7.4.6, **sin marcar**; la batería de la nube con los tests nuevos.
- `docs/development/server.md`: `app:catalog:rebuild --all` y el paso de despliegue.
- `docs/audits/hallazgos.md` no se toca: lo cierra la sesión principal tras la auditoría.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada test visto en rojo; salida de `make check`; medidas de carga; todo lo que no coincidía con este encargo y dónde se paró.
