# Encargo: echar a una persona, en el protocolo (hito 7.5.4a)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit de 7.5.3. **Nunca deshagas un fichero con git** (`git checkout`, `git restore`, `git stash`): se edita.

Lee antes [D98](../DECISIONS.md) y [D71](../DECISIONS.md) enteras, en `docs/architecture/sync.md` «Compartir y retirar el acceso» y las vallas, y las reglas 2, 3, 12 y 13 de `spec/README.md`.

**Qué se construye.** Solo protocolo y app: el campo `member.access_withdrawn`, sus reglas, y la función de la app que echa a una persona y la que le devuelve el acceso. **El servidor solo aprende el campo y la regla del juez**; cortar a la cuenta es 7.5.4b.

**Qué no se construye.** Nada de `membership`, rutas ni `AccountLinker` (7.5.4b). Pantallas y comandos de Tauri (7.5.5). Borrar a una persona del catálogo (no se construye, D98).

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Añade al glosario de `AGENTS.md`: **acceso retirado (de una persona) → `access_withdrawn`** («Field of a `member`: the person may no longer reach the library, from any device or through its account; the person stays in the catalog (D98). Not a `revocation`, which is about one device»).
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`.
- Prohibido tocar `docs/DECISIONS.md` y `docs/audits/`.
- **Casos hostiles primero**, vistos en rojo en Rust y en PHP; el informe copia la línea del fallo.
- **Si el trabajo crece, una regla de aquí choca con el código o con otra regla de `spec/`, un caso no da lo esperado, o un comando es denegado, se para y se cuenta.** No relajes ninguna regla de `spec/`.
- Explicaciones de Rust: en el informe, no en comentarios.

## 1. El campo

Cambia a la vez: migración de SQLite (**0034**), `schema.rs`, `spec/operation.schema.json`, casos de `operation-validation.json`, `CatalogEntity.php`, la entidad `Member` con su migración (`make migration`, `make migrate`), y `spec/sync.json` (**sube a 34**).

- **`member.access_withdrawn`**: booleano, por defecto falso (decide tú entre anulable o no, como `can_share`, y dilo).

## 2. Las reglas, en `spec/README.md`

Una regla nueva (14), y los ajustes que pida en las demás:

- **Quién lo escribe:** quien puede escribir `role` (regla 3: un administrador, o cualquiera sin roles). De cualquier otro, petición. Al crear una persona no puede venir a verdadero de quien no administra.
- **Un dispositivo de una persona con el acceso retirado es de nadie**, en todo lo que el juez pregunta por la persona o el rol de un dispositivo, igual que con la clave avalada (regla 13). «En ese momento», como siempre. Mientras no hay roles, la regla 1 va primero.
- **La nube no vincula a una persona con el acceso retirado** (regla 12): ese cambio es petición. Tampoco escribe `access_withdrawn`.
- **Una persona con el acceso retirado no cuenta como administradora** para «tiene roles la biblioteca» solo si eso no cambia nada de lo que ya hay: `has_roles` sigue mirando si alguien tiene rol, sin más. Si ves que una biblioteca cuyo único rol es de alguien con el acceso retirado queda en un estado raro, **para y cuéntalo**.

## 3. Casos de conformidad, primero y vistos en rojo

En `permissions.json`, en todos los órdenes de entrega:

1. Una administradora retira el acceso a un editor en una operación que además deja sin persona a su dispositivo, con su valla, y crea su retirada: lo que ese dispositivo firma después, petición.
2. **La marca sola** (sin tocar el dispositivo): el dispositivo sigue apuntando a la persona, pero lo que firma después es petición. (Hoy aplicado: tiene que verse en rojo.)
3. Una operación del echado **fechada antes** de la retirada y fuera de lo que la administradora conocía (antedatada): petición. La marcada en la valla y sus anteriores: aplicadas.
4. Un editor, o un lector, escribe `access_withdrawn` (sobre otro, y sobre sí mismo para quitárselo): petición.
5. La nube vincula un dispositivo nuevo a una persona con el acceso retirado: petición. A una que no lo tiene: aplicado.
6. La administradora devuelve el acceso (marca a falso) y vincula de nuevo el dispositivo avalando su clave: lo de después, aplicado; lo de entre medias, petición.
7. Echar a una administradora (hay otra): la echada ya no puede cambiar roles.
8. Sin roles: cualquiera lo escribe y queda aplicado; y un dispositivo de una persona con la marca sigue pudiendo todo (regla 1).

## 4. Rust

- `crates/domain/src/permission.rs` y `app/src-tauri/src/permissions.rs`: la condición entra donde está la de la clave avalada (`person_of`, `person_of_device`), para que todos los sitios que ya pasan por ahí la hereden. Dime si queda alguno que no pase.
- Test de convergencia en `sync.rs` con `assert_converged`: la retirada y lo que el echado firma llegan en órdenes distintos.
- **`people::withdraw_person(conn, library, member)`**: **una** operación (`member.withdraw`) con: la marca; por cada dispositivo de esa persona, su fila sin persona, sin clave avalada y con su valla (`fence_mark`, como `assign_device`); y la retirada de cada uno (como `libraries::revoke_access`, reutilizando su código). Se niega si es la última administradora (`is_last_admin`, que no debe contar a quien tiene el acceso retirado) y si la persona es la de este dispositivo.
- **`people::restore_person(conn, library, member)`**: solo quita la marca. No devuelve dispositivos ni levanta retiradas: eso lo hace quien administra vinculando de nuevo (D98).
- `list_people` expone la marca (un campo más en lo que devuelve), sin tocar `app/src/`. Si TypeScript necesita el campo para compilar, añádelo al tipo y nada más.
- Sin comandos de Tauri nuevos.
- Tests: echar a alguien deja a sus dispositivos sin persona, vallados y con retirada, y el afectado borra la biblioteca al sincronizar por la red local (el camino de D71 que ya existe); la última administradora no se puede echar; uno mismo tampoco; devolver el acceso no devuelve nada por sí solo.

## 5. PHP

- `Permissions.php`, `LibraryView.php`, `CatalogLibraryView.php`, `FakeView.php`: la misma condición. El servidor guarda la marca en `catalog_members`.
- `PermissionConformanceTest` y `PermissionConformancePushTest` pasan los casos nuevos; `CatalogSchemaTest` en verde.
- **Nada más en el servidor.** En particular, `AccountLinker` no se toca: que hoy siga enlazando o vinculando a una persona con el acceso retirado es de 7.5.4b. Si un test existente lo roza, dímelo.

## 6. Documentación

- `spec/README.md`: la regla.
- `docs/architecture/sync.md`: echar a una persona, junto a retirar el acceso a un dispositivo.
- `docs/architecture/cloud.md`: el campo en «Tablas del catálogo».
- `docs/architecture/security.md`: en la pieza 4, una frase sobre echar a una persona. **Lenguaje llano.**
- `docs/plans/fase-7.md`: una línea de 7.5.4a en «Estado», **sin marcar**; los hostiles en la batería.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run lint && npm run check:spec`, y en `server/` `make check`. Todo en verde. `make migration` no debe proponer nada al terminar.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada caso visto en rojo; los sitios que leen la persona de un dispositivo y si pasan por la condición; salidas de las comprobaciones; todo lo que no coincidía y dónde se paró.
