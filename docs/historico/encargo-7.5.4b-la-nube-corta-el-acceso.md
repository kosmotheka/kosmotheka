# Encargo: la nube corta el acceso (hito 7.5.4b)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit de 7.5.4a. **Nunca deshagas un fichero con git** (`git checkout`, `git restore`, `git stash`): se edita.

Lee antes [D98](../DECISIONS.md), [D71](../DECISIONS.md) y [D96](../DECISIONS.md) enteras; en `docs/architecture/sync.md`, «Compartir y retirar el acceso» (los avisos con `proofs`, SEG-011); y en `docs/architecture/cloud.md`, el enlace de la cuenta y los avisos (`events`).

**Qué se construye.** La nube deja de entregar una biblioteca, al momento, a un dispositivo con una retirada y a la cuenta de una persona con el acceso retirado. Al dispositivo afectado le da la prueba firmada, y el dispositivo borra la biblioteca **solo si la comprueba**, como ya hace por la red local.

**Qué no se construye.** Pantallas ni comandos de Tauri (7.5.5). Borrar una biblioteca de la nube. No se toca `spec/`: si crees que hace falta, **para**.

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Glosario de `AGENTS.md`.
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`.
- Prohibido tocar `docs/DECISIONS.md`, `docs/audits/` y `spec/`.
- **El servidor no se fía de ningún id del cliente (D84).** A quién se corta sale del dispositivo autenticado, de `membership` y del catálogo del servidor.
- **Tests hostiles primero**, vistos en rojo; el informe copia la línea del fallo.
- **Si el trabajo crece, algo choca con el código, o un comando es denegado, se para y se cuenta.**
- Doctrine primero; SQL solo donde ya se escribe SQL, con parámetros enlazados.
- Explicaciones de Rust: en el informe, no en comentarios.

## 1. Servidor: quién deja de recibir

Dos cortes, los dos **leídos del estado del catálogo** en cada petición (no de un aviso que pueda perderse):

**A. El dispositivo.** El dispositivo autenticado (id `D`, clave `K`) tiene una retirada (`revocation`) aplicada en esa biblioteca, y la fila `device` de `D` en el catálogo tiene `public_key` = `K` (es el mismo dispositivo, no otro con su id). La cuenta conserva su acceso desde sus otros dispositivos.

**B. La cuenta.** La fila de `membership` de la cuenta apunta a una persona que tiene `access_withdrawn`, o que ya no existe. Entonces:
- `Membership` gana `withdrawnAt` (anulable). Se pone en cuanto el servidor lo ve: al final del `push` que trae la retirada (dentro de su transacción, después de `settle`, para **todas** las filas de esa biblioteca) y, por si acaso, en la propia puerta.
- En ese mismo momento, si la nube tiene el poder, escribe `member.account_id` vacío con `CloudAuthor` (intención `member.unlink_account`), sin bucles, como en `noteAccount`.
- Una fila con `withdrawnAt` **no da acceso**, aunque después alguien quite la marca a la persona: volver es enlazarse de nuevo (abajo).
- La fila de quien subió la biblioteca (sin persona todavía) no se ve afectada por esto: sin persona no hay a quién retirar. Si su persona, ya enlazada, es echada, se le corta como a cualquiera. Apunta en el informe qué pasa entonces con la biblioteca de la nube (sigue existiendo, de quién es); **no lo resuelvas**.

**La puerta** (`CloudLibraryAccess` y lo que use `list`, `pull`, `push`, `link` y los temas de `events`):

| Situación | Respuesta |
|---|---|
| Sin fila de `membership` | `404`, como hoy |
| Corte A o B | `403` `access_withdrawn`, con `proofs` |
| Lo demás | Como hoy |

- **`proofs`:** las operaciones firmadas, tal como están guardadas, que crearon las retiradas aplicadas que nombran a `D` en esa biblioteca. Nada más: ni otras operaciones, ni filas del catálogo. Si ninguna retirada nombra a `D` (un dispositivo de la cuenta echada que no estaba en la biblioteca), la lista va vacía.
- `GET /v1/cloud/libraries` no lista las bibliotecas cortadas, pero la respuesta gana `withdrawn`: sus ids, para que la app pida la prueba. Solo ids de bibliotecas en las que la cuenta **tenía** fila: no se revela nada que no supiera.
- Los temas de `events` salen solo de filas con acceso. Al cortar, se publica en el tema de la **cuenta** afectada, para que sus dispositivos conectados sincronicen ya.
- El `403` no depende de lo que el cliente mande en el cuerpo.

**`AccountLinker`:**
- No enlaza con una persona que tiene `access_withdrawn`, ni vincula dispositivos a ella (la regla del juez ya lo dejaría en petición; aquí ni se intenta).
- **Volver:** una fila con `withdrawnAt` se enlaza de nuevo por el camino de siempre (dispositivo con persona, clave avalada igual a la suya, sin retirada, persona sin la marca): se le quita `withdrawnAt` y se pone esa persona. Para eso `linkTo` tiene que admitir el cambio de persona **solo** en una fila con `withdrawnAt`.

## 2. App

- `CloudApi`: el error `access_withdrawn` con sus `proofs`, y `withdrawn` en el listado.
- Al recibirlo para una biblioteca que este dispositivo tiene: se fusionan las `proofs` y se juzgan **como hace `lan` con `SharingNotices`** (reutiliza ese código; no lo dupliques). Solo si queda una retirada aplicada que nombra a este dispositivo, la biblioteca se borra de aquí (`libraries::remove_revoked`) y el informe de la sesión lo dice. **Sin prueba válida no se borra nada:** la biblioteca se queda, deja de sincronizarse con la nube y el informe lo dice con un resultado propio (por ejemplo `Outcome::AccessWithdrawn`).
- Las `proofs` pueden no encadenar con lo que el dispositivo tiene (le faltan operaciones anteriores de quien administra). Mira cómo lo resuelve `lan`; si por la nube no sale igual, **para y cuéntalo** antes de inventar otra cosa.
- Una biblioteca cortada no vuelve a bajar sola (el guardarraíl de SEG-017 ya lo hace para las borradas aquí: comprueba que cubre este caso, con test).
- `FakeCloud` hace lo mismo en pequeño.
- El texto de los avisos nuevos, con claves i18n en `en` y `es`, solo si el informe de la sesión ya se enseña en algún sitio; si no, apúntalo para 7.5.5.

## 3. Tests

**Hostiles, vistos en rojo** (servidor):

1. Una administradora retira el acceso al móvil de Jhonny (lector, con cuenta enlazada). Ese móvil: `pull` y `push` responden `403` con la prueba, y **no** reciben ni una operación posterior a la retirada ni ninguna otra. Su otro dispositivo sigue entrando.
2. Echan a Jhonny (`member.withdraw`). Con cualquiera de sus dispositivos, y con uno **nuevo** de su cuenta: `403`; el listado no la trae; `claim` no lo enlaza ni lo vincula; el token de `events` no lleva el tema de esa biblioteca.
3. Tras echarlo, la nube ha vaciado `account_id` de Jhonny (con poder), firmado y aplicado. Sin poder, no escribe nada y el corte vale igual.
4. Le quitan la marca a Jhonny **sin** vincularle ningún dispositivo: sigue sin acceso.
5. Jhonny, echado, sube con una app modificada una operación que se quita la marca a sí mismo, o que borra su retirada: `403`, y nada cambia en el catálogo.
6. Un dispositivo con el **id** de un dispositivo retirado y otra clave (otra persona, con acceso): no se le corta por eso.
7. Otra cuenta sin acceso sigue recibiendo `404`, no `403`: el `403` no revela bibliotecas a quien nunca estuvo.
8. Un cuerpo con ids, o con `proofs` propias, no cambia la respuesta.
9. La retirada llega **tarde** (la biblioteca se juzga de nuevo): el corte vale igual después.

**Hostiles en la app (Rust), vistos en rojo:**

10. **Un servidor que miente:** responde `access_withdrawn` sin `proofs`, con `proofs` de otra biblioteca, con una retirada firmada por quien no administra, o con una retirada que nombra a otro dispositivo. En los cuatro casos la biblioteca **sigue** en el dispositivo, entera.
11. Con una prueba válida, la biblioteca se borra, y no vuelve a bajar en la sesión siguiente.

**Los demás:** volver (quitar la marca, vincular de nuevo un dispositivo, `claim` enlaza otra vez y `pull` entrega); el escenario de D76 «alguien sale o lo echan» de punta a punta en Rust con `FakeCloud`; fixtures solo si no crecen.

## 4. Documentación

- `docs/architecture/cloud.md`: los dos cortes, la puerta, `proofs`, volver.
- `docs/architecture/sync.md`: lo que cambie de «Retirar el acceso» con la nube.
- `docs/architecture/security.md`: piezas 4 y 5, y en «Lo que no protege» la línea del dispositivo retirado, si cambia. **Lenguaje llano.**
- `docs/plans/fase-7.md`: una línea de 7.5.4b en «Estado», **sin marcar**; los hostiles en la batería.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run lint && npm run check:spec`, y en `server/` `make check`. Todo en verde. `make migration` no debe proponer nada al terminar.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada test visto en rojo; cómo encadenan las `proofs`; qué pasa con la biblioteca de la nube cuando echan a quien la subió; salidas de las comprobaciones; todo lo que no coincidía y dónde se paró.
