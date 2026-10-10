# Encargo 7.5.5c · El móvil nuevo, el dispositivo de otra persona y quién tiene cuenta

> Encargo cerrado para la sesión de hitos, preparado el 10/10/2026 y **sin enviar**. Tercera y última parte de 7.5.5 ([plan](fase-7.md)). Sin commit, sin push, sin cambiar de rama.

## Qué hay y qué falta

Hechas 7.5.5a (echar y devolver el acceso) y 7.5.5b (qué se sube, el interruptor de la nube, los avisos). Por dentro: `cloud::sync_all` baja las bibliotecas de la cuenta y reclama (`claim`), que devuelve `linked` y `conflicts` (bibliotecas donde este dispositivo ya es de otra persona); la app **no guarda ni enseña** `conflicts`. `member.account_id` dice qué persona tiene cuenta enlazada; la pantalla no lo usa. Nada dice a la pantalla quién vinculó cada dispositivo.

Lee antes: `AGENTS.md`; D76, D96 y D98 en `docs/DECISIONS.md`; `docs/architecture/cloud.md` (`claim`, `sync_all`, «La nube en la biblioteca»); `docs/architecture/security.md`, piezas 6 y 7. Del diseño, en `docs/design/screens/cloud-family/`: `Download.dc.html`, `Dialog.dc.html`, `Lib.dc.html`, `People.dc.html`, `Person.dc.html`, `Copy.dc.html` (también «Lo que hemos supuesto») y `errors-fase-7b.md`. Las diferencias conocidas con el diseño están al final de `docs/design/screens/README.md`.

## Qué construir

0. **Arreglo pequeño de 7.5.5b (decidido por el maintainer, 10/10/2026):** el interruptor «La nube puede dar acceso a dispositivos nuevos» **no se muestra en una biblioteca sin personas ni roles**, ni en sus ajustes ni al subirla: sin personas la nube no tiene a quién vincular nada. Al subir una biblioteca sin roles no se nombra a la nube. Si al activar los roles después hiciera falta ofrecer el interruptor, basta con que aparezca en los ajustes. `LibraryCloudStatus.hasRoles` ya existe.
1. **El móvil nuevo** (`Download`): lo que se ve mientras bajan las bibliotecas de la cuenta y al terminar («Han bajado 2 bibliotecas. En «Casa» eres Laura · administradora»; «Las bibliotecas que no subiste siguen en tus otros dispositivos»). **No** se muestra «Si sales de la app, seguirá bajando»: no es verdad. Averigua qué informa hoy `sync_all` durante la bajada; si no hay progreso, una pantalla de espera sin porcentaje basta: no construyas progreso por operación en este encargo.
2. **«Terminando de preparar este móvil…»** (`Lib`, `lib.finishing`): el rato en que la biblioteca ya está pero el dispositivo aún no tiene su persona. Comprueba primero si ese estado se llega a ver (la bajada y el `claim` van en la misma sesión): si no se ve nunca, no lo construyas y dímelo. Si se ve, la biblioteca se enseña sin «Añadir» ni edición, con esa línea. Qué puede hacer el dispositivo lo dice Rust (ya lo dice: un dispositivo sin persona no edita con roles); la pantalla solo lo cuenta.
3. **Este dispositivo es de otra persona** (`Dialog`, claves `other_owner.*`):
   - Rust guarda los `conflicts` de `claim` como estado local de este dispositivo (no una operación) y los da a la pantalla con lo que hace falta: la biblioteca, la persona de la que es el dispositivo y su rol;
   - el diálogo ofrece «Dejarlo como está» y, si quien ha entrado administra esa biblioteca **desde otro dispositivo suyo**, nada más: **en este dispositivo no administra** (es de Hugo), así que «Hacerla mía» no puede firmarse aquí. Comprueba esto en el código antes de dibujar nada y dime qué sale; si «Hacerla mía» no es posible con las reglas de hoy, no se ofrece y lo apuntas;
   - «Pedir a quien administra que lo cambie»: Design supone una petición normal en Peticiones. Comprueba que una asignación de dispositivo firmada por quien no administra llega como petición y se puede aceptar desde ahí; si no, no se ofrece y lo apuntas;
   - no vuelve a preguntar: la respuesta se guarda en este dispositivo para esa cuenta y esa biblioteca, y solo vuelve si cambia la persona del dispositivo o entra otra cuenta.
4. **Quién tiene cuenta** (`People`, `Person`): una marca discreta en la persona con cuenta enlazada (`member.account_id` no vacío), sin email, con su explicación una vez encima de la lista. `Person.hasAccount` lo calcula Rust. Con ese dato, el consejo de «Mis dispositivos» al quitar un dispositivo (7.5.5a) sale **solo si la persona tiene cuenta**.
5. **«por la nube»** en un dispositivo, y «Lo vinculó Ana»: hace falta saber quién firmó la operación que dijo de quién es el dispositivo. Averigua si se puede sacar del registro de operaciones sin tabla ni campo nuevo (la última operación aplicada que fijó `member_id` de ese dispositivo, y si su autor es `library.cloud_device_id`). Si se puede, `PersonDevice.linkedByCloud` (y, si es barato, el nombre de la persona que lo vinculó). Si pide una migración o tocar `spec/`, **no lo hagas**: dímelo.
6. **Los tres errores de Design sin código real** (`device_duplicate`, `person_account_taken`, `cloud_link_disabled`): busca en `docs/architecture/cloud.md` y en el cliente de la nube qué código da hoy el servidor en cada caso (por ejemplo, la clave que no coincide, la persona ya enlazada con otra cuenta, la nube sin poder). Al que tenga código real, su texto. Al que no llegue hoy a la app, lístalo en el informe, sin inventar.
7. **Textos:** todos con claves i18n, en `en` y `es`. `docs/design/errors.md` al día.

## Límites

- Solo `app/` (Rust y UI) y `docs/`. Nada en `server/`, `spec/` ni `crates/`, y ninguna migración. Ninguna regla nueva de permisos ni de quién recibe una biblioteca: si crees que hace falta, para y dímelo.
- Este encargo tiene varias comprobaciones previas (puntos 2, 3, 5 y 6). Si dos o más salen «no se puede con lo de hoy», para tras comprobarlas y manda un informe intermedio antes de construir el resto.
- La lógica va en Rust. Nunca sostengas el `MutexGuard` de la base de datos a través de un `.await`.
- Tests de Rust para todo dato nuevo que calcule Rust. Si tocas quién puede cambiar de quién es un dispositivo, test del caso hostil antes y visto fallar.
- No deshagas nada con git. Rust en `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`. Actualiza `docs/architecture/cloud.md` y la línea de 7.5.5c en `docs/plans/fase-7.md` (sin marcarla). Informe corto: qué hiciste por punto, qué salió de cada comprobación previa, qué decidiste por tu cuenta, en qué te apartaste del diseño y por qué, qué tests viste fallar, resultado de las comprobaciones y qué no pudiste comprobar (nada se prueba en pantalla: eso lo hace el maintainer).
