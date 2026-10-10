# Encargo: el poder de la nube, en el protocolo (hito 7.5.1)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit `198a5fb`. Lee antes [D76](../DECISIONS.md) y [D95](../DECISIONS.md) enteras, la sección «7.5 La cuenta en la biblioteca» de [`fase-7.md`](fase-7.md) y, en `spec/README.md`, «Roles and permissions».

**Qué se construye.** Solo el protocolo: dos campos nuevos y una regla de permisos, en `spec/`, en el juez de Rust y en el de PHP. **El servidor todavía no firma nada ni enlaza cuentas** (eso es 7.5.2): aquí la nube es un dispositivo cualquiera que los tests simulan.

**Qué no se construye.** Claves del servidor, rutas, pantallas, la tabla `membership`. Nada de `server/src/Controller/` ni de `app/src/`.

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Añade al glosario de `AGENTS.md`: **dispositivo de la nube → `cloud_device_id`** («Field of the `library` row: the device that is the cloud in this library, and the only one with the cloud's power (D76, D95)») y **cuenta de una persona → `account_id`** («Field of a `member`: a random id of the account linked to that person; only the cloud writes it (D76)»).
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`.
- Prohibido tocar `docs/DECISIONS.md` y `docs/audits/`.
- **Casos hostiles primero**, vistos en rojo en Rust y en PHP; el informe copia la línea del fallo.
- **Si el trabajo crece, una regla de aquí choca con el código o con otra regla de `spec/`, da un resultado que no es el esperado, o un comando es denegado, se para y se cuenta.**
- Explicaciones de Rust: en el informe, no en comentarios.
- SQL nuevo en el servidor: solo en `CatalogSql`, `CloudOperationRepository` o la migración, con parámetros enlazados.

## 1. Los dos campos

Los dos cambian a la vez, como pide `AGENTS.md`: migración de SQLite (**0032**), `crates/domain/src/schema.rs`, `spec/operation.schema.json`, casos de `spec/conformance/`, `server/src/Enum/CatalogEntity.php`, la entidad de Doctrine con su migración (`make migration`), y `spec/sync.json` (**sube a 32**).

- **`library.cloud_device_id`**: texto anulable, el id de un dispositivo. No es una referencia con clave foránea: el dispositivo de la nube puede registrarse después de que el campo lo nombre. A diferencia de `cloud_library_id` y `cloud_server`, **el servidor sí lo guarda** en `catalog_libraries`, porque su juez lo necesita.
- **`member.account_id`**: texto anulable, un UUID. Sin clave foránea a nada.

Los valores se comparan sin distinguir mayúsculas, como los demás ids.

## 2. La regla, en `spec/README.md`

Una regla nueva en «Roles and permissions», con estas definiciones:

- **El dispositivo de la nube** de una biblioteca es el que nombra `library.cloud_device_id` en el momento de la operación. Si el campo está vacío, no hay ninguno.
- **Quién escribe `library.cloud_device_id`:** quien puede cambiar la fila `library`, como hoy (un administrador, o cualquiera sin roles). No cambia nada de esa regla.
- **El dispositivo de la nube puede**, además de lo que puede un dispositivo de nadie (registrarse y renombrarse):
  1. un cambio `member` `update` que toca **solo** `account_id` (ponerlo o vaciarlo), sobre una persona que existe;
  2. un cambio `device` `update` que toca **solo** `member_id`, sobre **otro** dispositivo que en ese momento **no es de nadie**, apuntando a una persona que en ese momento **tiene `account_id`**.
- **`member.account_id` solo lo escribe el dispositivo de la nube.** Nadie más, tampoco un administrador: en una biblioteca con roles, un cambio que lo toque y no venga del dispositivo de la nube deja la operación en petición. Al crear una persona no puede venir relleno.
- **Lo que no puede:** roles, `can_share`, reasignar un dispositivo que ya es de alguien, quitar a un dispositivo su persona, `trusted_through`, borrar, la fila `library`, las retiradas ni el catálogo. Su propio dispositivo sigue siendo de nadie.
- **Sin roles** no cambia nada: todos pueden todo (regla 1), también esto.
- **Vallas y sellos valen igual para la nube:** una operación suya que cae bajo una valla o un sello se juzga como de nadie, y entonces no tiene este poder.
- **Todo o nada**, como siempre: una operación de la nube con un cambio permitido y otro no queda en petición entera.

## 3. Casos de conformidad, primero y vistos en rojo

En `spec/conformance/permissions.json`, en todos los órdenes de entrega. Biblioteca con roles: una administradora con su dispositivo, un lector con el suyo, un dispositivo de nadie y el dispositivo de la nube, nombrado por la administradora en `library.cloud_device_id`.

1. La nube pone `account_id` a una persona y después vincula a ella el dispositivo de nadie: las dos aplicadas, y el dispositivo tiene desde entonces el rol de esa persona (una operación suya posterior lo demuestra).
2. La nube vincula el dispositivo de nadie a una persona **sin** `account_id`: petición.
3. La nube reasigna un dispositivo que ya es de otra persona: petición.
4. La nube quita a un dispositivo su persona (`member_id` nulo): petición.
5. La nube cambia un rol, o `can_share`: petición.
6. La nube pone `trusted_through` a un dispositivo, sola o junto a un `member_id` permitido: petición.
7. La nube crea un libro: petición.
8. **Un dispositivo que no es el de la nube** (el lector, y aparte el de nadie) hace lo del caso 1: petición.
9. Un lector escribe `account_id`; y una administradora escribe `account_id`: petición las dos.
10. Un lector escribe `library.cloud_device_id` nombrándose a sí mismo y después se vincula a la administradora: las dos, petición.
11. **Poder retirado.** La administradora vacía `library.cloud_device_id` en una operación que lleva la valla del dispositivo de la nube. Una vinculación de la nube fechada después: petición. Una fechada antes y que es la marcada o anterior suya: aplicada. Una fechada antes y fuera de la cadena (antedatada): petición.
12. **Otra nube.** La administradora cambia `library.cloud_device_id` a otro dispositivo: el anterior ya no puede, el nuevo sí, cada uno según la fecha de su operación.
13. Un `account_id` vaciado: la nube ya no puede vincular dispositivos nuevos a esa persona, y los ya vinculados lo siguen estando.
14. Sin roles: cualquiera hace cualquiera de estas cosas y queda aplicado.

Tienen que verse fallar antes de la regla, al menos: 1 (hoy la nube es un dispositivo de nadie y todo queda en petición), 9 en su mitad de la administradora (hoy un administrador puede todo), y los casos de conformidad de validación que nombren los campos nuevos.

## 4. Rust

- `crates/domain/src/permission.rs`: `LibraryView` gana lo que la regla necesita saber (quién es el dispositivo de la nube; si una persona tiene `account_id`); `is_allowed` y `change_allowed` con la regla. Ojo: hoy `Role::Admin` devuelve `true` antes de mirar nada, y `account_id` es la excepción.
- `app/src-tauri/src/permissions.rs`: `SqlView` con las consultas nuevas. La biblioteca se juzga «con los valores de ese momento»: comprueba que el recálculo (`rebuild`, `judged_by_order`) cubre el caso de que `cloud_device_id` o un `account_id` lleguen tarde, con un test en `sync.rs` y `assert_converged`.
- **Quitar el poder lleva valla:** una función en la app que vacía `cloud_device_id` con `fence_changes` del dispositivo de la nube en la misma operación, y otra que lo nombra. Sin pantalla todavía: solo las funciones y sus tests. No se añaden comandos de Tauri.
- Tests unitarios en `permission.rs` para la regla, y en `sync.rs` los de convergencia.

## 5. PHP

- `server/src/Sync/Permission/Permissions.php`, `LibraryView` y `CatalogLibraryView`: la misma regla, con DQL de escalares por la biblioteca del servidor, como las demás consultas.
- El motor guarda `cloud_device_id` en `catalog_libraries` y `account_id` en `catalog_members`; revisa `DbalCatalogMerge::NOT_STORED`.
- `PermissionConformanceTest` y `PermissionConformancePushTest` pasan los casos nuevos sin tocar su código, salvo lo que haga falta para los campos.
- `CatalogSchemaTest` (la paridad con `CatalogEntity`) sigue en verde.

## 6. Documentación

- `docs/architecture/sync.md`: la regla, junto a las de roles y vallas.
- `docs/architecture/cloud.md`: los campos nuevos en «Tablas del catálogo» y que el servidor guarda `cloud_device_id`.
- `docs/plans/fase-7.md`: la línea de 7.5.1 en «Estado», **sin marcar**.
- `spec/README.md`: además de la regla, la lista de ficheros si cambia.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run check:spec`, y en `server/` `make check`. Todo en verde.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada caso visto en rojo; cualquier caso de los catorce cuyo resultado no saliera como dice este encargo y por qué; salidas de las comprobaciones; todo lo que no coincidía y dónde se paró.
