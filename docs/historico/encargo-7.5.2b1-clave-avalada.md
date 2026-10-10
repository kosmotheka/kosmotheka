# Encargo: la clave avalada, en el protocolo (hito 7.5.2b-1)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

**Del árbol de trabajo tal como está**: el commit `198a5fb` más 7.5.1 y 7.5.2a, revisados y **sin commit**. No deshagas ni reordenes nada de eso. **Nunca deshagas un fichero con git** (`git checkout`, `git restore`, `git stash`): se edita.

Lee antes [D96](../DECISIONS.md) entera, la enmienda del 10/10/2026 de [D76](../DECISIONS.md), y en `spec/README.md` «Signatures» (el párrafo de la clave de cada dispositivo) y «Roles and permissions».

**Qué se construye.** Solo protocolo y app: un campo nuevo, `device.vouched_key`, una condición nueva en la regla 2, la app que lo escribe, y quién puede subir. **El servidor no enlaza cuentas todavía** (eso es 7.5.2b-2).

**Qué no se construye.** La tabla `membership`, `CloudAuthor` escribiendo nada, rutas, pantallas. Nada de `server/src/Controller/` ni de `app/src/` salvo lo que el punto 5 obligue.

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Añade al glosario de `AGENTS.md`: **clave avalada → `vouched_key`** («Field of a `device` row: the public key that whoever said whose the device is had registered for it; the device is its person's only if it matches its own key (D96)»).
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`. Nada fuera de los contenedores.
- Prohibido tocar `docs/DECISIONS.md` y `docs/audits/`.
- **Casos hostiles primero**, vistos en rojo en Rust y en PHP; el informe copia la línea del fallo.
- **Si el trabajo crece, una regla de aquí choca con el código o con otra regla de `spec/`, un caso da un resultado distinto del esperado, o un comando es denegado, se para y se cuenta.** No relajes ninguna regla de `spec/` para que algo encaje.
- Explicaciones de Rust: en el informe, no en comentarios.
- SQL nuevo en el servidor: solo en los repositorios que ya escriben SQL o en la migración, con parámetros enlazados.

## 1. El campo

Cambia a la vez, como pide `AGENTS.md`: migración de SQLite (**0033**), `crates/domain/src/schema.rs`, `spec/operation.schema.json`, casos de `spec/conformance/operation-validation.json`, `server/src/Enum/CatalogEntity.php`, la entidad de Doctrine con su migración (`make migration`, y `make migrate` después), y `spec/sync.json` (**sube a 33**).

- **`device.vouched_key`**: texto anulable, con la forma de `public_key` (32 bytes, base64url sin relleno; valida igual). Se compara byte a byte, como `public_key`.
- **No es `public_key`** y no toca su regla: `public_key` sigue siendo lo que el dispositivo registra de sí mismo, y un cambio de `public_key` sigue rechazándose. `vouched_key` es lo que **otro** dice haber visto.

## 2. La regla, en `spec/README.md`

- **Quién la escribe:** quien puede escribir `device.member_id` (regla 3: un administrador, o cualquiera sin roles). Un cambio que toque `vouched_key` y no venga de ahí deja la operación en petición, como `member_id`. Un dispositivo nunca escribe la suya propia con roles (igual que no dice de quién es, regla 4).
- **Regla 2, condición nueva:** la **persona** de un dispositivo es la de su `member_id` solo si su `vouched_key` está vacía (una asignación anterior a D96) o es igual a la `public_key` con la que ese dispositivo está registrado en la biblioteca. Si hay `vouched_key` y no coincide, o el dispositivo no está registrado, **el dispositivo es de nadie** (cuenta como lector, no guarda marcas, etc.), en todo lo que el juez pregunta por la persona o el rol de un dispositivo.
- **«En ese momento»**, como todo: vale la `vouched_key` y el `member_id` vigentes en el HLC de la operación juzgada.
- La nube (regla 12) **no** puede escribir `vouched_key`: su cambio sobre `device` sigue siendo solo `member_id`. Añade a la regla 12 que el dispositivo que vincula queda con `vouched_key` vacía, y deja una línea diciendo que eso se revisa en 7.5.3 (no lo resuelvas aquí).
- El cliente de la regla 12 «un dispositivo que en ese momento no es de nadie»: decide con `member_id` vacío, como hoy, no con la condición nueva. Si ves que eso abre algo, **para y cuéntalo**.

## 3. Casos de conformidad, primero y vistos en rojo

En `spec/conformance/permissions.json`, en todos los órdenes de entrega. Solo hay una clave de test, así que «otra clave» tiene que ser una `vouched_key` distinta de la de test (cualquier otra de 32 bytes válida).

1. Una administradora asigna el dispositivo registrado de un editor avalando **su clave de verdad**: lo que el editor crea después queda aplicado.
2. Lo mismo avalando **otra clave**: lo que ese dispositivo hace después queda en petición (es de nadie), y una marca suya también.
3. Lo mismo y el dispositivo así «asignado» a una administradora intenta cambiar un rol: petición. **Es el caso del ataque.**
4. Asignación sin `vouched_key` (como las de antes): sigue valiendo, aplicado.
5. Un lector escribe `vouched_key` sobre su propio dispositivo o sobre otro: petición.
6. La administradora corrige: una segunda asignación con la clave buena tras una con la mala. Lo fechado entre las dos, petición; lo de después, aplicado.
7. La asignación con clave llega **antes** que el registro del dispositivo (otro orden de entrega): el mismo resultado que en 1 y en 2.
8. Sin roles: cualquiera escribe `vouched_key` y queda aplicado; y un dispositivo con `vouched_key` que no coincide sigue pudiendo todo mientras no hay roles (regla 1).
9. La nube intenta escribir `vouched_key` junto a un `member_id` permitido: petición.

Tienen que verse fallar antes de la regla, al menos el 2 y el 3.

## 4. Rust

- `crates/domain/src/permission.rs` y `app/src-tauri/src/permissions.rs` (`SqlView::role_of_device`, `member_of_device` y lo que pregunte por la persona de un dispositivo): la condición nueva. Revisa **todos** los sitios que leen `devices.member_id` para decidir algo (`created_by_same_person`, `requested_by_same_person`, las marcas, `people::may_share`, `pairing_owners`, el sello): que no quede ninguno que crea a un dispositivo con clave avalada distinta. Dime en el informe la lista de sitios y qué hiciste en cada uno.
- Test de convergencia en `sync.rs` con `assert_converged`: la asignación con clave y el registro llegan en órdenes distintos a dos dispositivos.
- **La app la escribe siempre que dice de quién es un dispositivo:** `people::assign_device`, `start_roles`, `give_access` / `apply_pending_owners`, y cualquier otro sitio que escriba `member_id` no nulo. La clave es la `public_key` que este dispositivo tiene en su fila `devices` para ese id. Quitar la persona (`member_id` nulo) vacía `vouched_key` en el mismo cambio. **Si el dispositivo aún no tiene clave registrada aquí, no se asigna** (error claro o se deja pendiente como ya hace `apply_pending_owners`); si algún flujo de hoy asigna antes de conocer la clave y esto lo rompe, **para y cuéntalo**.
- **Avalar una vez las asignaciones anteriores:** una función `permissions::vouch_if_missing`, llamada donde hoy se llama `seal_if_missing` y después de él (los tres sitios), que en cada biblioteca con roles donde este dispositivo es de un administrador escribe, en **una** operación, `vouched_key` para cada dispositivo registrado que tiene `member_id` y no tiene `vouched_key`, con la clave que este dispositivo tiene de él. Solo `vouched_key` en cada cambio, sin tocar `member_id`. Idempotente: la segunda vez no escribe nada. Tests: avala lo que falta, no repite, un no administrador no escribe nada, y dos administradores que lo hacen a la vez convergen.

## 5. Solo sube quien administra (enmienda de D76, CON-009)

- `cloud::upload` y lo que calcula `can_upload` dejan de usar `people::may_share`: sube quien **administra** la biblioteca (rol `admin` de la persona de este dispositivo, con la condición nueva), o cualquiera si no tiene roles. `may_share` se queda como está para la red local.
- Si la UI muestra un texto que promete subir a quien «puede compartir», corrígelo en `en` y `es`; si no hay ninguno, no toques `app/src/`.
- Test: un editor con «Puede compartir» no puede subir (y el servidor no llega a recibir nada); una administradora sí; sin roles, cualquiera.

## 6. PHP

- `Permissions.php`, `LibraryView.php`, `CatalogLibraryView.php`, `FakeView.php`: la misma condición. El servidor guarda `vouched_key` en `catalog_devices`.
- `PermissionConformanceTest` y `PermissionConformancePushTest` pasan los casos nuevos.
- **Test hostil en `CloudSyncControllerTest`, visto en rojo** (es el ataque de D96, sin enlace automático): Jhonny, con acceso (`shareWithJhonny`), registra en la nube el id de un dispositivo nuevo con una clave suya; la administradora asigna ese id a su persona avalando la clave de verdad; Jhonny sube, firmada con su clave como ese dispositivo, una operación que cambia un rol: queda **en petición** y el rol no cambia en `catalog_members`. Sin la regla nueva quedaría aplicada: cópiame esa línea.
- `CatalogSchemaTest` sigue en verde.

## 7. Documentación

- `spec/README.md`: la regla.
- `docs/architecture/sync.md`: la clave avalada junto a roles y vallas, y `vouch_if_missing` junto al sellado.
- `docs/architecture/cloud.md`: el campo en «Tablas del catálogo»; quién puede subir.
- `docs/plans/fase-7.md`: una línea de 7.5.2b-1 en «Estado», **sin marcar**; el caso hostil en la batería.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run check:spec`, y en `server/` `make check`. Todo en verde.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada caso visto en rojo; la lista de sitios que leen o escriben `member_id` y qué hiciste en cada uno; cualquier caso cuyo resultado no saliera como dice este encargo; salidas de las comprobaciones; todo lo que no coincidía y dónde se paró.
