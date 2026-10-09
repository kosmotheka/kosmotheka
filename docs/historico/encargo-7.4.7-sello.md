# Encargo: el sello (hito 7.4.7, D94)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit `4ec226c`. Lee antes [D94](../DECISIONS.md) entera, y de D84 y D87 lo que dicen de las vallas. `spec/README.md`, «Roles and permissions».

**El fallo (SEG-025).** La fecha de una operación la pone su autor, y antes de que haya roles todos pueden todo. Un dispositivo emparejado con la app modificada fecha antes de la activación una operación que le hace administrador, o su enlace a la nube, y se le cree. Las vallas ya resuelven esto al degradar a alguien, pero solo para sus dispositivos.

**Qué se construye.** Una regla nueva del protocolo, en `spec/`, en Rust y en PHP: el **sello**. Y la app lo escribe al activar los roles y, una vez, en las bibliotecas que ya los tenían.

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Nombres del glosario de `AGENTS.md`; el término nuevo es **sello → `seal`** (añádelo al glosario: «An applied operation that fences its own author's device; it fences every device, named or not (D94)»).
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`. Nada fuera de los contenedores.
- Prohibido tocar `docs/DECISIONS.md` (D94 ya está escrita) y `docs/audits/`.
- **Tests hostiles primero:** los casos de conformidad y los tests marcados «visto en rojo» se escriben antes que la regla, se ejecutan, fallan, y el informe copia la línea del fallo.
- **Si el trabajo crece, una regla de aquí choca con el código, la regla de D94 no da el resultado esperado en algún caso, o un comando es denegado, se para y se cuenta.** No se busca otro camino.
- Explicaciones de Rust: en el informe, no en comentarios del código.

## 1. La regla, en `spec/`

En `spec/README.md`, «Roles and permissions», una regla nueva junto a las de las vallas, con estas definiciones exactas:

- Un **sello** es una operación **aplicada** que lleva un cambio `device` con `trusted_through` sobre **el dispositivo de su propio autor**.
- Un sello vale como valla de **todos** los dispositivos. Para los que nombra (cada cambio `device` con `trusted_through`), con la marca que da, como cualquier valla. Para los que **no** nombra, con la marca vacía: ninguna operación suya fechada antes del sello se cree.
- Una operación que cae bajo un sello se juzga como hoy la que cae bajo una valla: como de nadie, haya roles o no.
- Solo cuentan los sellos aplicados, igual que las vallas: un sello que queda en petición no vale, y se recalcula hasta que el resultado no cambia (lo que ya hacen `permissions::rebuild` y `CatalogJudge::rebuild`).
- Lo que no cambia: quién puede escribir `trusted_through` (un administrador, o cualquiera sin roles; nunca un no administrador sobre sí mismo), y todo lo demás de las vallas.

## 2. Casos de conformidad, primero y vistos en rojo

En `spec/conformance/permissions.json`, con la forma de los casos de vallas que ya hay, en todos los órdenes de entrega:

1. **Toma de control.** Un dispositivo se registra, y con fecha anterior a la activación firma una operación que crea un miembro administrador y se asigna a él. La activación de verdad lleva sello. Esperado: la operación antedatada, petición; la activación, aplicada; una operación posterior de ese dispositivo que degrada a la administradora, petición.
2. **Enlace antedatado.** Un dispositivo que la activación deja de lector firma, con fecha anterior al sello y sin ser la marcada ni una anterior suya, un cambio de la fila `library`. Esperado: petición.
3. **Lo sabido se respeta.** Una operación de un editor anterior al sello y que es la marcada, o una anterior suya por `previous`: aplicada, llegue cuando llegue.
4. **Lo no sincronizado queda en petición.** Una operación honrada de un dispositivo nombrado, fechada antes del sello y posterior a su marca: petición.
5. **Dispositivo no nombrado.** Registrado después del sello; firma una operación fechada antes: petición. Su registro y lo fechado después se juzgan como siempre.
6. **Sello falso antedatado.** Un dispositivo firma un «sello» propio fechado antes del de verdad, que pondría bajo valla al historial de los demás. Esperado: ese sello, petición; el historial de los demás, aplicado.
7. **Biblioteca con roles sellada después.** Activación sin sello (como las de hoy), más tarde un sello de la administradora. Una operación antedatada antes de la activación: petición. El historial anterior que el sello marca: aplicado.
8. **Sin roles no hay sello que valga de un no autor:** un cambio `trusted_through` sobre otro dispositivo, sin el del autor, sigue siendo una valla normal y no un sello.

Los casos 1, 2, 5, 6 y 7 tienen que verse fallar antes de la regla, en Rust (`sync.rs`) y en PHP (`PermissionConformanceTest`, `PermissionConformancePushTest`).

## 3. Rust

- `crates/domain/src/permission.rs` y `app/src-tauri/src/permissions.rs` (`fenced`, `candidate_fences`, `fences_of`, `note_fences`, `rebuild`, `judged_by_order`): el sello como valla de todos. Cómo se guarde (una fila más en `device_fences`, una tabla propia) lo eliges tú; **si hace falta migración, es la 0031** y `crates/domain/src/schema.rs` si toca.
- **`spec/sync.json` sube `schemaVersion`** en cualquier caso: una app sin la regla juzgaría distinto y no debe sincronizar. Lo comprueba el test `the_spec_has_the_same_schema_version_as_the_migrations` en `db.rs`; si no hay nada que migrar, la migración 0031 existe igual y lo dice en su comentario.
- **Quién sella:** la operación que da el primer rol de una biblioteca (busca dónde se escribe; `people.rs` y lo que cree la biblioteca con roles) lleva, en la misma operación, `fence_changes` de **todos** los dispositivos registrados, y el del propio autor con su operación anterior (`previous`) como marca.
- **Bibliotecas que ya tienen roles:** al arrancar, como `retire_fixed_library`, por cada biblioteca con roles cuyo registro no tenga ningún sello aplicado y en la que este dispositivo sea de un administrador, se escribe un sello (una operación con las vallas de todos, intención propia). Una sola vez: si ya hay sello, nada. En un dispositivo que no es de administrador, nada.
- Tests en `sync.rs` con `assert_converged`: dos administradores que sellan a la vez sin haberse visto convergen; el sellado al arrancar no se repite; un dispositivo no administrador no sella.

## 4. PHP

- `server/src/Sync/Permission/` (`CatalogJudge::fencesOf`, `noteFences`, `rebuild`, `replay`; `CatalogLibraryView::fenced`): la misma regla. Si hace falta guardar el sello, entidad Doctrine y migración con `make migration`; SQL nuevo solo en `CatalogSql`, `CloudOperationRepository` o la migración, con parámetros enlazados.
- `SchemaVersion` lee `spec/sync.json`: comprueba que el `push` de una app con la versión anterior sigue dando el conflicto de versión.
- **Tests hostiles por la ruta, vistos en rojo**, en `CloudSyncControllerTest`:
  - el de la toma de control (el caso 1, subido por `push`);
  - un lector sube el registro entero con roles **y sello**, y su enlace fechado antes del sello: `403 link_not_allowed`, nada guardado.
- Las fixtures (`ScenarioFixtures`) activan los roles con sello, como la app nueva.
- `make stress` con 2.000 libros y `ROLES=1` si el historial inventado se puede juzgar; si no, dilo. Mide una vez: el sello añade una comprobación a cada operación fechada antes de él.

## 5. Documentación

- `docs/architecture/sync.md` y `docs/architecture/cloud.md` («Permisos en el servidor»): el sello, quién lo escribe y el sellado al arrancar.
- `AGENTS.md`: solo la fila del glosario.
- `docs/plans/fase-7.md`: la línea de 7.4.7, **sin marcar**; la batería de acceso con los casos nuevos.
- Una nota para el usuario en la documentación que ya hable de roles: lo editado sin conexión antes de activar los roles puede quedar en petición.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run check:spec`, y en `server/` `make check`. Todo en verde.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada caso y test visto en rojo; cómo se guarda el sello y por qué; salidas de las comprobaciones; la medida de carga; todo lo que no coincidía con este encargo y dónde se paró.
