# Encargo: el enlace de la cuenta con su persona (hito 7.5.2b-2)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

**Del árbol de trabajo tal como está**: el commit `198a5fb` más 7.5.1, 7.5.2a y 7.5.2b-1, revisados y **sin commit**. No deshagas ni reordenes nada de eso. **Nunca deshagas un fichero con git** (`git checkout`, `git restore`, `git stash`): se edita.

Lee antes [D76](../DECISIONS.md) (con su enmienda), [D95](../DECISIONS.md) y [D96](../DECISIONS.md) enteras, y `docs/architecture/cloud.md`.

**Qué se construye.** El servidor enlaza una cuenta con su persona en cada biblioteca de la nube donde el dispositivo con el que inició sesión ya es de esa persona, con la clave avalada. Ese enlace es una fila de `membership`, que es lo que da acceso. Y la nube anota en la biblioteca que esa persona tiene cuenta.

**Qué no se construye.** Que la nube vincule dispositivos nuevos (7.5.3), bajar en un móvil nuevo una biblioteca que no tiene (7.5.3), quitar el acceso (7.5.4), invitaciones (7.6), pantallas (7.5.5). No se toca `spec/`: si crees que hace falta, **para**.

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Glosario de `AGENTS.md`.
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`. Nada fuera de los contenedores.
- Prohibido tocar `docs/DECISIONS.md`, `docs/audits/` y `spec/`.
- **El servidor no se fía de ningún id del cliente (D84).** La ruta nueva no recibe ningún id: todo sale del dispositivo autenticado (la entidad `App\Entity\Device` de la sesión: su `deviceId` y su `publicKey`, probados con el reto firmado) y de las tablas del servidor.
- **Tests hostiles primero**, vistos en rojo; el informe copia la línea del fallo.
- **Si el trabajo crece, algo de aquí choca con el código, o un comando es denegado, se para y se cuenta.**
- Doctrine primero (ORM y DQL); SQL solo donde ya se escribe SQL, con parámetros enlazados.
- Explicaciones de Rust: en el informe, no en comentarios.

## 1. El id público de la cuenta

`User` gana `accountId`: un UUID v4 aleatorio, único, generado al crear la cuenta; migración que lo rellena en las cuentas que ya existen (uno distinto por fila). Es lo único de la cuenta que viaja con una biblioteca (`member.account_id`, D76). No es la clave primaria, ni sale del email.

## 2. La ruta

`POST /v1/cloud/libraries/claim`, sin cuerpo (un cuerpo, si llega, se ignora). Autenticada como las demás, con el dispositivo actual (`CurrentDevice`). Con el mismo límite de peticiones que las rutas de la nube.

Una clase nueva (por ejemplo `App\Sync\AccountLinker`) hace el trabajo. Para el dispositivo autenticado, con id `D` y clave `K`, de la cuenta `U`:

**Candidatas:** las bibliotecas de la nube **enlazadas** (con su operación de enlace) en cuyo catálogo hay una fila `device` con id `D` (sin distinguir mayúsculas), `public_key` igual a `K`, `vouched_key` igual a `K`, y persona. Las tres condiciones: una asignación sin clave avalada **no** enlaza (D96), aunque el juez la siga creyendo para el rol.

**Por cada candidata**, con el bloqueo de la biblioteca que usa el `push` y en una transacción, volviendo a leer dentro del bloqueo. Sea `M` la persona de ese dispositivo:

| Situación | Qué se hace |
|---|---|
| Hay una retirada (`revocation`) para `D` en esa biblioteca | Nada |
| Otra cuenta ya está enlazada con `M` en esa biblioteca | Nada (una persona, una cuenta) |
| `U` no tiene fila de `membership` ahí | Se crea, con `memberId` = `M` |
| `U` tiene fila con `memberId` vacío (quien la subió) | Se rellena con `M` |
| `U` tiene fila con `memberId` = `M` | Nada |
| `U` tiene fila con otra persona | Nada; la biblioteca va en `conflicts` de la respuesta |

`Membership::memberId` deja de ser `readonly` solo para ese relleno (un método que lo pone una vez y falla si ya tiene otro valor). Añade la restricción única (`cloud_library_id`, `member_id`) en la migración.

**Anotar la cuenta en la biblioteca:** cuando la fila queda enlazada con `M` (recién creada, recién rellenada, o ya lo estaba) y `member.account_id` de `M` no es ya el `accountId` de `U`, la nube lo escribe con `CloudAuthor::author` (intención `member.link_account`, un cambio `member` `update` con solo `account_id`). **Solo si la nube tiene el poder en esa biblioteca en ese momento** (el `cloud_device_id` del catálogo es el suyo y está registrada): si no lo tiene, no se escribe nada, para no llenar el registro de peticiones; el acceso se da igual. Si la operación, aun así, acaba en petición, el enlace se mantiene y no se reintenta en bucle (dime cómo lo evitas). Lo escrito se avisa a los demás dispositivos como lo hace un `push`.

**Respuesta:** `200` con `{"linked": [<ids de biblioteca de la nube recién enlazadas>], "conflicts": [<ids>]}`. Solo ids de bibliotecas a las que la cuenta tiene acceso al terminar. Idempotente: la segunda llamada devuelve `linked` vacío y no escribe nada.

**Coste:** una consulta para las candidatas (no una por biblioteca del servidor). Si necesita un índice sobre `catalog_devices`, va en la migración.

## 3. App

- `CloudApi` gana `claim()`; `HttpCloud` y `FakeCloud` lo implementan (el falso, con las mismas reglas en pequeño: basta con lo que los tests necesiten, sin duplicar la lógica del servidor).
- Se llama al iniciar sesión y al principio de cada sesión con la nube, **antes** de listar. Un fallo de `claim` (servidor antiguo sin la ruta, red) no rompe la sesión: se sigue con lo que hubiera.
- Con eso, un dispositivo que ya tiene una biblioteca por la red local y cuya cuenta acaba de quedar enlazada la sincroniza con la nube como cualquier otra. Comprueba que nada lo impide (por ejemplo el guardarraíl de SEG-017 en `cloud/mod.rs`) y, si algo lo impide, **para y cuéntalo**. Bajar una biblioteca que el dispositivo **no** tiene es de 7.5.3: no lo construyas.
- El mock de desarrollo (`app/src/dev/mock-ipc.ts`) calcula `canUpload` con `canShare`: cámbialo a «administra» (pendiente de 7.5.2b-1).

## 4. Tests

**Hostiles, vistos en rojo**, en un test de controlador nuevo o en `CloudSyncControllerTest`. La familia de las fixtures: Laura administra y subió la biblioteca; Jhonny es de fuera salvo donde se diga.

1. **El ataque de D96.** Jhonny, con acceso (`shareWithJhonny`), registra en la nube el id de la tablet nueva de Laura con una clave suya; Laura la asigna a su persona avalando la clave de verdad. La cuenta de Jhonny, con un dispositivo de ese id y su clave, llama a `claim`: no queda enlazado con Laura, su fila de `membership` no cambia, y `account_id` de Laura no es el suyo.
2. Jhonny, **sin acceso**, con un dispositivo cuyo id es el del móvil de Laura y una clave suya: `claim` no le da nada y la biblioteca sigue respondiéndole `404`.
3. Una asignación **sin** clave avalada: `claim` no enlaza.
4. Un dispositivo registrado y **de nadie**: no enlaza.
5. Un lector que se «asigna» a sí mismo a la administradora avalando su propia clave (la operación queda en petición): no enlaza.
6. Un dispositivo con una **retirada**: no enlaza.
7. Una persona que **ya tiene otra cuenta** enlazada: la segunda cuenta no entra.
8. Un cuerpo con ids de biblioteca, de persona o de dispositivo: se ignora; el resultado es el mismo que sin cuerpo.
9. Una biblioteca de la nube **sin enlazar** (creada, sin operación de enlace): no se enlaza nadie.
10. La cuenta enlazada como lector **lee** la biblioteca (`pull`) y lo que sube queda en petición: el enlace da acceso, no rol.

**Los demás:**

- El caso feliz: el dispositivo de un editor, asignado y avalado, con su cuenta: `claim` crea la fila, `pull` le entrega la biblioteca, y `member.account_id` queda escrito por la nube, firmado, aplicado y repartido por `pull`.
- Quien subió: su fila pasa de `memberId` vacío a su persona.
- La nube sin poder (sin nombrar): se enlaza, hay acceso, y no se escribe ninguna operación.
- Conflicto: la cuenta ya es una persona y su dispositivo es de otra: nada cambia y sale en `conflicts`.
- Dos llamadas seguidas, y dos a la vez si el test lo permite: una sola fila, una sola operación.
- `accountId`: distinto por cuenta; nunca es la clave primaria ni aparece el email en ninguna operación.
- Rust (`cloud/tests.rs`): un segundo dispositivo, con otra cuenta, que tiene la biblioteca por la red local y está asignado y avalado, hace una sesión con la nube: queda con acceso, sincroniza y converge con el que la subió; uno sin avalar no recibe nada de la nube.
- Fixtures (`ScenarioFixtures`): si ya hay una segunda cuenta de la familia, que quede enlazada por este camino; documenta en `fixtures.md` lo que cambie.

## 5. Documentación

- `docs/architecture/cloud.md`: la ruta, la tabla de situaciones, qué da el enlace y qué no.
- `docs/architecture/accounts.md`: `accountId`.
- `docs/plans/fase-7.md`: una línea de 7.5.2b-2 en «Estado», **sin marcar**; los hostiles en la batería.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run lint && npm run check:spec`, y en `server/` `make check`. Todo en verde. `make migration` no debe proponer nada al terminar.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada test visto en rojo; la consulta de las candidatas tal como queda; cómo se evita el bucle de la anotación; salidas de las comprobaciones; todo lo que no coincidía con este encargo y dónde se paró.
