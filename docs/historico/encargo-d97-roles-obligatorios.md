# Encargo · Roles obligatorios desde que se crea la biblioteca (D97)

> Encargo cerrado para la sesión de hitos, escrito el 15/10/2026. Sin commit, sin push, sin cambiar de rama. Toca `spec/`, `crates/domain`, `app/`, `server/` y `docs/`. **Empieza por la fase 0 y espera la respuesta de la sesión coordinadora antes de escribir código.**

## Para qué

Hoy una biblioteca nace sin roles: todos los dispositivos pueden todo hasta que alguien da el primer rol (D73, regla 1 de `spec/README.md`, «Roles and permissions»). Son dos modos y un paso entre ellos, y cada regla lleva su excepción «o cualquiera, mientras no haya roles». El sello (D94) existe sobre todo por ese paso.

Lee [D97](../DECISIONS.md) entera, con lo que precisó el maintainer el 15/10/2026, y D73, D94, D96 y D98:

- **El modo sin roles desaparece del todo**, también de `spec/`. No hay conversión de bibliotecas viejas: son datos de prueba, y el maintainer las borra y las vuelve a crear. «No se mantiene algo que no va a usarse nunca.» Nada de código muerto para lo antiguo.
- **Quien crea una biblioteca es su administrador desde la primera operación.**
- **El nombre de la persona se pide en la bienvenida.** Todo el mundo tiene «Mi biblioteca» desde que entra (`libraries::ensure_first_library`, que hoy se crea al abrir la base, antes de la bienvenida).
- Para dar la biblioteca a otro dispositivo por QR, hay que decir siempre de quién es y con qué rol. Dar «administrador» es responsabilidad de quien lo da.

## Fase 0 · Inventario y propuesta (sin código)

Mándame un informe con:

1. **Reglas (`spec/README.md`).** Qué texto se quita y qué se escribe en su lugar, regla por regla. Sobre todo la primera: cómo es la operación que funda una biblioteca (crea la fila `library`, su primera persona con `role: admin` y la asignación del dispositivo autor, con su `vouched_key`), quién puede hacerla y qué pasa con una biblioteca cuya historia no empieza así (por ejemplo, la de una app modificada).
2. **Casos de conformidad.** Cuántos de `spec/conformance/*.json` dependen de una biblioteca sin roles y cómo los rehaces. No pueden perder lo que cubren.
3. **El sello (D94).** Qué parte sigue haciendo falta sin el paso a los roles y qué se puede quitar. Es una decisión aceptada: propón y no quites nada hasta que te conteste.
4. **App.** Qué desaparece: `people::start_roles`, las ramas de `has_roles` en `people.rs`, `permissions.rs` y `marks.rs`, los textos y pantallas de «sin roles», el interruptor de la nube que se ocultaba en bibliotecas sin roles… Cómo se crea «Mi biblioteca» con su administrador cuando el nombre llega en la bienvenida (por ejemplo, creándola en ese momento o renombrando después a la persona). Qué nombre lleva el creador en las bibliotecas que cree después. Qué pasa al restaurar una copia de seguridad o al recibir una biblioteca por QR o desde la nube.
5. **Servidor.** Los cambios en el juez de PHP, `ScenarioFixtures`, la reconstrucción del catálogo y cualquier otro sitio que pregunte si hay roles.
6. **Tamaño.** Una estimación honrada de cuánto código y cuántos casos se quitan y cuántos se añaden, y si propones partirlo en varias entregas.
7. **Los datos de prueba del maintainer.** Qué tiene que borrar él, en la app y en la nube, y cómo, sin herramientas nuevas si es posible. Nada que dependa de entrar en su servidor.

## Lo que no cambia

Firmas (D56), juzgar cada operación en cada dispositivo, las vallas (D84, D87), la identidad de la nube (D95), la clave avalada (D96) y echar a una persona (D98).

## Reglas

- **Una regla cambia primero en `spec/`, con sus casos**, y después en las dos implementaciones (Rust y PHP), que deben pasar los mismos casos.
- La sincronización debe converger: los cambios en cómo se aplican las operaciones llevan prueba en `sync.rs` con `assert_converged`.
- Quién recibe una biblioteca se prueba primero contra un par hostil (la «Batería de acceso» de `docs/plans/fase-7.md`).
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar (después de la fase 0, cuando te conteste)

Servidor: `make check` después del último cambio. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`, `check:codes`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. La salida real de cada uno.

Documentación: `spec/README.md`, `docs/architecture/` (operaciones, sincronización, permisos), `docs/plans/fase-7.md`, `docs/ROADMAP.md` y las notas de la versión en `docs/publicacion/novedades.md` (hay que borrar los datos de prueba).

## Estado al cortar la primera sesión (16/10/2026) y cómo retomar

La sesión que empezó la entrega 1 se quedó sin límite a medias, sin informe ni commit. Su trabajo está en el árbol de trabajo (unos 41 ficheros). **Quien retome no necesita su conversación:** empieza por `git status` y `git diff`, y por esta lista. Las decisiones de la fase 0 están en D97, D94 (reemplazada) y D46 (precisada) de `docs/DECISIONS.md`.

**Hecho (sin commit):**
- Dominio: `crates/domain/src/founding.rs` (fundación, `library_id` derivado de la clave y `founding_nonce`). El juez de Rust ya no tiene modo «sin roles». Las 81 pruebas del dominio pasan.
- `spec/conformance/permissions.json`: de 75 a 69 casos (13 quitados, 7 de fundación nuevos, el resto reescritos). Los 69 pasan en Rust y en PHP. `spec/sync.json` está en 35.
- Sello retirado en Rust (funciones, tabla `device_seals`, pruebas) y en PHP (`Seal`, el juez, migración `Version20261015214624`).
- Juez de PHP: `Permissions.php` y `Sync/LibraryId.php`.
- App: migración `0035_founding.sql`, `libraries::found`, `found_as` y `derives_here`; quitados `start_roles` y `retire_fixed_library`.
- Copias: guardan `founding_nonce` y, al restaurar, conservan el id solo si este dispositivo lo deriva. La semilla de firma no va en las copias (D58), así que en un dispositivo nuevo ni la copia completa conserva los ids.
- Ayudantes de pruebas en Rust: `share_as`, `assign_as` y `open_in_memory_as`.

**Medido por la sesión coordinadora al retomar (16/10/2026):**
- `cargo test -p kosmotheka-domain`: 81 pasan.
- `cargo test -p kosmotheka`: 563 pasan y 14 fallan (backup ×2, cloud ×4, LAN ×3, people ×4, sync ×1), y hay un aviso de import sin uso (`crate::people::PersonPick`).
- Servidor, `tests/Sync/Permission`, `tests/Sync/LibraryIdTest.php` y `tests/Entity`: 721 pruebas y 7 fallos (`CatalogJudgeTest` ×3, `CatalogLibraryViewHostileTest`, `PermissionConformancePushTest` ×2 y `CatalogSchemaTest`). La suite completa daba, según la sesión anterior, 20 errores y 171 fallos, sobre todo porque `SigningDevice`, `ScenarioFixtures` y `CloudSyncControllerTest` siguen subiendo bibliotecas sin fundación.
- Base de pruebas del servidor migrada; la de desarrollo, no.

**Falta, para cerrar la entrega 1 en verde** (las pruebas tienen que pasar enteras, así que entra parte de lo que el plan ponía en la 3):
1. Rust: adaptar las 14 pruebas que fallan. Las que solo comprobaban el modo viejo se quitan; las que cubren algo que sigue valiendo se rehacen sobre una biblioteca fundada. Después, `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.
2. Servidor: `SigningDevice` con su fundación, `ScenarioFixtures` y las pruebas de la nube que suben bibliotecas (`registration(`, unos 72 usos), con un ayudante común. Después, `make check` completo.
3. `spec/operation.schema.json`: el campo `founding_nonce` en `library`. Todavía no está, y `npm run check:spec` lo debe notar. También `CatalogEntity.php` y `schema.rs`, si no lo tienen.
4. `spec/README.md`: reglas 1, 3, 6, 11 (fuera), 13 y 14, la fundación y un vector de ejemplo de la derivación del id, como el de la regla 12.
5. App TS: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec` y `check:codes`. `claim_person`, el diálogo de activación y el nombre en la bienvenida son de la entrega 2: hasta entonces la persona se llama «Me», como dejó la primera sesión.
6. Documentación de la entrega 1: `spec/README.md` y `docs/architecture/` (operaciones, sincronización, permisos).

Al terminar la entrega 1, informe corto con las salidas reales y **para**: la sesión coordinadora la revisa antes de la 2.
