# Encargo: arreglos de la auditoría del 10/10/2026, parte de la app

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`. Los hallazgos están en [`docs/audits/2026-10-10.md`](../audits/2026-10-10.md) (solo leer).

## De dónde se parte

El commit que cierra la parte de protocolo. Solo `app/src-tauri` y `docs/`; nada de `server/` ni de `spec/`. Si un arreglo pide tocar `spec/`, **para y cuéntalo**.

## Reglas

Las de siempre: **sin commit**, sin `git add`, nunca deshacer con git, scripts que reescriben a un temporal y renombran; inglés en el código, español en `docs/`; **el test del caso primero, visto en rojo**, con la línea del fallo en el informe; si crece o choca, se para; explicaciones de Rust en el informe.

## A. SEG-035: echar a una persona alcanza también a sus dispositivos aún sin registrar

`people::withdraw_person` lee `devices`. Un dispositivo recién emparejado al que se dio esa persona está en `pending_owners` y en `library_peers`, sin fila en `devices`. Para cada uno: su retirada (`revocation_change`), `forget_peer`, y borrar su fila de `pending_owners`, en la misma operación y transacción. Test en `lan/client.rs`, por TLS como los demás: se empareja un móvil, se le da una persona antes de que se registre, se echa a esa persona, y el móvil **no recibe la biblioteca** en la siguiente sincronización ni queda asignado cuando se registra.

## B. SEG-033: completar una retirada con los dispositivos que no se conocían

La retirada desasigna solo los dispositivos que conoce quien la firma. Uno que llegó después (por ejemplo, vinculado por la nube justo antes) sigue apuntando a la persona echada.

- **`permissions::complete_withdrawals`**, llamada donde `vouch_if_missing` y después de él (los tres sitios): en cada biblioteca con roles donde este dispositivo es de un administrador, **una** operación (`member.withdraw`) que, para cada dispositivo cuyo `member_id` crudo apunta a una persona con `access_withdrawn`, lo deja sin persona ni clave avalada, con su valla y su retirada. Idempotente; un no administrador no escribe nada; dos administradores a la vez convergen.
- **`people::restore_person`** hace eso mismo para esa persona **antes** de quitar la marca, en la misma operación.
- Tests en `sync.rs` con `assert_converged`: un dispositivo asignado a la persona echada llega tarde; tras una sesión del administrador queda sin persona, vallado y retirado; devolver el acceso no deja ningún dispositivo apuntando a ella.

## C. SEG-036: el último administrador que puede actuar

`people::is_last_admin` cuenta personas. Que cuente **administradores con acceso que tienen al menos un dispositivo que es suyo** (con `person_of`). Afecta a `set_role`, `withdraw_person` y `remove_device` (quitar el último dispositivo del último administrador): comprueba los tres y dime cuáles cambian de comportamiento. Tests: con dos administradores y uno sin dispositivos, no se puede echar ni degradar al otro, ni quitarle su último dispositivo.

## D. DAT-009: el fallo de una biblioteca no corta la sesión

En `cloud::sync_all`, un error de `sync_library`, `adopt_link` o `download` de **una** biblioteca no aborta las demás: su informe lleva un resultado de fallo (`Outcome::Failed`, con el código del error si lo hay) y la sesión sigue. `download` trata `AccessWithdrawn` como `sync_library`. Los errores que sí cortan todo: no hay sesión, la versión del esquema no coincide, o la red no responde (decide tú cuáles son de la sesión y cuáles de una biblioteca, y dilo). Texto i18n del resultado nuevo en `en` y `es`, y el valor en el tipo de TypeScript. Tests con `FakeCloud`: una biblioteca falla al bajar y las otras dos se sincronizan.

## Documentación

`docs/architecture/sync.md` y `cloud.md` donde cambie; una línea en «Estado» de `docs/plans/fase-7.md`, sin marcar.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run lint && npm run check:spec`.

## Informe

Punto por punto, las líneas en rojo, qué cambia de comportamiento en C, qué errores cortan la sesión en D, y las comprobaciones.
