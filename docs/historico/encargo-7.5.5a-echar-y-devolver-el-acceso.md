# Encargo 7.5.5a · Echar a una persona y devolverle el acceso: las pantallas

> Encargo cerrado para la sesión de hitos, 10/10/2026. Primera de las tres partes de 7.5.5 ([plan](fase-7.md)). Sin commit, sin push, sin cambiar de rama.

## Qué hay y qué falta

Por dentro está hecho (D98, hitos 7.5.4a y 7.5.4b, arreglos de la auditoría): `people::withdraw_person` y `people::restore_person` existen y están probados, pero **no tienen comando de Tauri ni pantalla**. `people::Person` ya envía `accessWithdrawn`; el tipo de TypeScript no lo tiene (CON-010). `remove_device`, `set_role` y `withdraw_person` devuelven `Error::LastAdmin` (`permission` / `last_admin`).

Lee antes: `AGENTS.md`, D98 en `docs/DECISIONS.md`, «Echar a una persona» en `docs/architecture/sync.md`, y del diseño `docs/design/screens/cloud-family/` las piezas `People.dc.html`, `Person.dc.html`, `Dialog.dc.html` y, en `Copy.dc.html`, las claves `remove_device`, `remove_device.tip`, `people.no_access`, `people.restore` y las del diálogo de echar a una persona y del último administrador. Las diferencias conocidas con el diseño están al final de `docs/design/screens/README.md`.

## Qué construir

1. **Comandos de Tauri** `withdraw_person(member)` y `restore_person(member)` en `commands.rs`, sobre la biblioteca activa, registrados en `lib.rs`, con su función en `app/src/lib/api.ts`. Quita los `#[cfg_attr(not(test), allow(dead_code))]` que ya no hagan falta.
2. **`Person.accessWithdrawn: boolean`** en `api.ts` (cierra CON-010).
3. **Personas** (`people-page.tsx`, `components/people/`):
   - una persona con el acceso retirado se ve como «sin acceso», con la línea de que sus libros siguen siendo suyos. El diseño pone una fecha («desde el 3 oct»): Rust no la tiene; **sin fecha**;
   - en su ficha, para quien administra: «Devolver el acceso», con la frase de que después hay que vincular de nuevo un dispositivo suyo. No se le puede cambiar el rol ni asignar dispositivos mientras esté sin acceso, salvo que el código ya lo permita sin romper nada: si es así, déjalo y dímelo;
   - en la ficha de quien sí tiene acceso, para quien administra: «Quitar de la biblioteca», con un diálogo de confirmación que diga lo que pasa (deja de poder entrar, con sus dispositivos y con su cuenta; su nombre, sus libros, sus préstamos y lo que marcó se quedan; lo que ya tenía copiado fuera de la app no se le puede quitar). No se ofrece sobre uno mismo.
4. **Quitar un dispositivo** (`revoke-dialog.tsx` o donde esté): el texto nuevo del diseño (`remove_device`), y el consejo `remove_device.tip` **solo si la persona de ese dispositivo tiene cuenta**. Si la app no sabe hoy si una persona tiene cuenta, muestra el consejo siempre y dímelo: no añadas ese dato a Rust en este encargo.
5. **El último administrador:** cuando no se puede (bajar de rol, echar, o quitarle su único dispositivo a la única persona que administra y tiene algún dispositivo), se explica **antes** de confirmar, como pide el diseño, y no con un error después. Para eso `People` puede llevar un dato calculado en Rust (por ejemplo, el id de «el último administrador que puede actuar», o nada); la regla no se repite en TypeScript. El texto actual de `last_admin` («Eres la única persona que administra…») no vale cuando se echa a otra persona o se le quita un dispositivo: ajústalo o añade otro.
6. **Textos:** todos con claves i18n, en `en` y `es`. Parte de los de Design; corrige lo que no sea verdad en la app.

## Límites

- Solo `app/` (Rust y UI) y `docs/`. Nada en `server/`, `spec/` ni `crates/`. Ninguna regla nueva de permisos: si crees que hace falta una, para y dímelo.
- Fuera de este encargo: la marca «tiene cuenta», la etiqueta «por la nube», «Lo vinculó Ana», el interruptor de la nube, `CloudPick`, los avisos «Ya estás en…» y los errores de la nube (son 7.5.5b y 7.5.5c).
- La lógica va en Rust; la UI llama comandos y no decide permisos. Quién ve cada botón sale de `myRole` y de lo que ya da `People`.
- Tests: de Rust para lo nuevo que calcule Rust (el dato del último administrador), y los de UI que el repositorio ya tenga para piezas parecidas. No hay regla nueva de acceso, así que no hace falta test hostil nuevo; si tocas una, sí, y visto fallar.
- No deshagas nada con git. Rust en `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`. Actualiza `docs/architecture/` donde toque y la línea de 7.5.5a en `docs/plans/fase-7.md` (sin marcarla). Informe corto: qué hiciste por punto, qué decidiste por tu cuenta, en qué te apartaste del diseño y por qué, resultado de las comprobaciones y qué no pudiste comprobar (nada se prueba en pantalla: eso lo hace el maintainer).
