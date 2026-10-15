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
