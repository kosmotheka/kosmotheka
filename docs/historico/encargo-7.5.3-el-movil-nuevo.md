# Encargo: el móvil nuevo (hito 7.5.3)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

El commit `dffb522` (7.5.2 cerrado), con el árbol limpio salvo este encargo y dos ficheros de `docs/` de la sesión principal. **Nunca deshagas un fichero con git** (`git checkout`, `git restore`, `git stash`): se edita.

Lee antes [D76](../DECISIONS.md) (con su enmienda), [D95](../DECISIONS.md) y [D96](../DECISIONS.md) enteras (incluido «La nube también avala»), la regla 12 y la 13 de `spec/README.md`, y en `docs/architecture/cloud.md` la sección del enlace de la cuenta.

**Qué se construye.** Laura estrena móvil y entra con su cuenta, que ya está enlazada con su persona en una biblioteca de la nube. El móvil baja la biblioteca, se registra en ella, y **la nube dice que ese móvil es de Laura**, avalando su clave. Desde entonces tiene su rol en todos los dispositivos de la familia. Vale igual para la administradora única que perdió su único móvil (D58).

**Qué no se construye.** Quitar el acceso a una cuenta o «quitar miembro» (7.5.4), invitaciones (7.6), pantallas (7.5.5): ni la casilla, ni la pregunta del conflicto.

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Glosario de `AGENTS.md`.
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`. Nada fuera de los contenedores.
- Prohibido tocar `docs/DECISIONS.md` y `docs/audits/`.
- **El servidor no se fía de ningún id del cliente (D84).** `claim` sigue sin recibir ids.
- **Casos y tests hostiles primero**, vistos en rojo; el informe copia la línea del fallo.
- **Si el trabajo crece, algo de aquí choca con el código o con otra regla de `spec/`, un caso no da lo esperado, o un comando es denegado, se para y se cuenta.** No relajes ninguna regla de `spec/` que este encargo no nombre.
- Doctrine primero; SQL solo donde ya se escribe SQL, con parámetros enlazados.
- Explicaciones de Rust: en el informe, no en comentarios.

## 1. La regla 12 cambia: la nube avala lo que vincula

En `spec/README.md`, y en los dos jueces:

- El cambio `device` de la nube pasa de «toca **solo** `member_id`» a «toca **exactamente** `member_id` y `vouched_key`, los dos, ninguno vacío». Con `member_id` solo, o `vouched_key` sola, o un campo más: petición.
- Lo demás de la regla 12 no cambia: sobre **otro** dispositivo, que en ese momento tiene `member_id` vacío, apuntando a una persona que en ese momento tiene `account_id`.
- El juez no comprueba de quién es la clave: lo hace la regla 13 (si la clave avalada no es la registrada, el dispositivo es de nadie).
- Quita de la regla 12 la línea que dejaba esto «para 7.5.3».
- `spec/sync.json` **se queda en 33**: no cambia el esquema y nada de esto ha salido.

**Casos de conformidad** (`permissions.json`), vistos en rojo donde toque: los casos de 7.5.1 que vinculan con `member_id` solo pasan a llevar las dos cosas; y casos nuevos:

1. La nube vincula con `member_id` solo: petición. (Hoy aplicado: tiene que verse en rojo.)
2. La nube vincula avalando la clave de verdad: aplicado, y lo que ese dispositivo hace después tiene el rol de su persona.
3. La nube vincula avalando **otra** clave: la operación queda aplicada, pero el dispositivo es de nadie (regla 13): lo que hace después, petición.
4. La nube escribe `vouched_key` sola sobre un dispositivo de nadie, o sobre uno que ya es de alguien: petición.

## 2. Servidor: `claim` vincula el dispositivo

`AccountLinker::claim` gana un segundo paso, después del que ya tiene. Para el dispositivo autenticado (id `D`, clave `K`, cuenta `U`), en cada biblioteca de la nube donde `U` tiene fila de `membership` **con persona** `M`, con el bloqueo de la biblioteca y volviendo a leer dentro, la nube escribe con `CloudAuthor` un cambio `device` `update` de `D` con `member_id` = `M` y `vouched_key` = `K` (intención `device.link_account`) **solo si se cumple todo**:

| Condición | Por qué |
|---|---|
| La nube tiene el poder ahí ahora (como en `noteAccount`) | Sin poder sería una petición |
| `M` existe y su `account_id` es el `accountId` de `U` | Lo pide la regla 12; si aún no lo es, primero se anota y se vincula en esa misma llamada |
| El catálogo tiene la fila `device` de `D` con `public_key` = `K` | El móvil se ha registrado él mismo; con otra clave, alguien ocupó su id: nada |
| Su `member_id` está vacío | La nube no reasigna |
| Su `trusted_through` está vacío y no hay retirada para `D` | A un dispositivo al que alguien quitó su persona o retiró el acceso solo lo devuelve un administrador |
| No hay ya una petición de la nube con ese mismo cambio | Sin bucles, como con `member.link_account` |

Una sola consulta para saber en qué bibliotecas mirar (las de `membership` de `U` con persona), no una por biblioteca del servidor.

**Respuesta:** `claim` gana una tercera lista, `devices`: ids de las bibliotecas de la nube donde este dispositivo acaba de quedar vinculado. Idempotente.

## 3. App

- **El orden en un móvil nuevo:** `claim`, listar, bajar la biblioteca que no tiene (ya existe: `download`), registrarse en ella y subir ese registro, `claim` otra vez, y bajar lo que la nube acaba de escribir. Al terminar **una** sesión (`sync_all`), el móvil tiene su persona y su rol. Mira qué hace hoy `download` con el registro del dispositivo: si el móvil no se registra hasta su primer cambio, haz que se registre al bajar (una operación suya que solo crea su fila `device`, como en el resto de la app), y dime cómo queda.
- El segundo `claim` solo se llama si en la sesión el dispositivo subió algo; si devuelve `devices`, se vuelve a bajar de esas bibliotecas. Su fallo no rompe la sesión.
- `FakeCloud` hace lo mismo en pequeño.
- Nada de pantallas. Si `status` o algún comando necesita exponer algo nuevo para 7.5.5, no lo hagas: apúntalo en el informe.

## 4. Tests

**Hostiles, vistos en rojo** (servidor, en `AccountLinkerTest` o junto a él):

1. **El móvil perdido.** Un administrador quita su persona al móvil de Hugo (con su valla); ese móvil, aún con la sesión de la cuenta de Hugo, llama a `claim`: la nube **no** lo vuelve a vincular.
2. Lo mismo con una **retirada**.
3. Jhonny, con acceso como lector, registra en la biblioteca el id del móvil nuevo de Laura con una clave suya antes que ella; el móvil de Laura llama a `claim`: no se vincula nada, y el dispositivo de Jhonny con ese id no pasa a ser de Laura.
4. Una cuenta **sin** enlace (sin fila, o con fila sin persona) con un dispositivo registrado: no se vincula.
5. La cuenta de un **lector** con un móvil nuevo: queda vinculado como lector, nunca con más rol; lo que sube sigue en petición.
6. La nube **sin poder** (sin nombrar, o quitada con su valla): no escribe nada y el dispositivo sigue siendo de nadie.
7. Un dispositivo que **ya es de otra persona**: la nube no lo toca.
8. Un cuerpo con ids en `claim`: se ignora.
9. Dos llamadas seguidas: una sola operación.

**Los escenarios de D76, de punta a punta:**

- **Móvil nuevo** (servidor y Rust): cuenta enlazada, móvil nuevo; tras una sesión tiene la biblioteca, su persona y su rol; un cambio suyo queda aplicado en la nube, en el móvil viejo y en el de otro miembro; `assert_converged`.
- **Administradora única sin móvil** (D58): su único dispositivo desaparece; con un móvil nuevo y su cuenta vuelve a ser administradora y puede cambiar un rol.
- **Biblioteca que nunca se subió:** nada cambia.
- Rust: el móvil nuevo de una cuenta **no** enlazada no recibe nada.

Fixtures: si encaja sin crecer, un móvil nuevo de la familia vinculado por la nube; si no, déjalo y dilo.

## 5. Documentación

- `spec/README.md`: la regla 12.
- `docs/architecture/cloud.md`: el segundo paso de `claim`, su tabla y el orden de la sesión en un móvil nuevo.
- `docs/architecture/sync.md`: lo que cambie de la regla 12.
- `docs/architecture/security.md`: en la pieza 6, cambia «En construcción: que la nube lo use» por lo que quede construido. **Lenguaje llano**, sin nombres del código fuera de los paréntesis.
- `docs/plans/fase-7.md`: una línea de 7.5.3 en «Estado», **sin marcar**; los hostiles en la batería.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run lint && npm run check:spec`, y en `server/` `make check`. Todo en verde. `make migration` no debe proponer nada al terminar.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada caso y test visto en rojo; qué casos de 7.5.1 cambiaron; cómo queda el registro del dispositivo al bajar; salidas de las comprobaciones; todo lo que no coincidía con este encargo y dónde se paró.
