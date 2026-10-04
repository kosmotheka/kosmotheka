# Sincronización

> Cómo se ponen de acuerdo varios dispositivos sin servidor (Fase 2) y, más adelante, con él (Fase 7). Decisiones de fondo: D05, D13, D25 y D31–D34. Documento vivo: se completa hito a hito.

## Idea

Cada dispositivo guarda todas las operaciones (las suyas y las recibidas). Sincronizar es **intercambiar las operaciones que le faltan al otro** y aplicarlas con unas reglas que hacen que **todos los dispositivos lleguen al mismo estado**, reciban las operaciones en el orden que las reciban. No hay árbitro: las reglas se encargan de ello.

## Qué operaciones se replican (D31)

- Cada operación guarda la **versión del esquema** con la que se registró. Dos dispositivos con versiones distintas no se sincronizan: la app pide actualizar primero (esa comprobación entre los dos extremos no cambia). Al fusionar, una operación de una versión **mayor** que la local se rechaza; una **menor o igual** se acepta si pasa `validate` y su firma, porque es historia de antes de una actualización (D31, precisado el 03/10/2026; tests `operations_from_an_older_schema_version_are_accepted`, `operations_from_a_newer_schema_version_are_rejected`, `a_device_paired_after_an_app_update_gets_the_old_history`).
- **Una operación rechazada retiene a las siguientes de su autor en el lote.** Si `merge` rechaza antes de registrarla (versión mayor, reloj adelantado, `validate` o firma), las operaciones posteriores del mismo autor y biblioteca en ese lote tampoco se registran, para que el vector del autor quede por debajo de la rechazada y la próxima sincronización la pida de nuevo (DAT-004; test `a_rejected_operation_holds_back_the_later_ones_of_its_author`). Las que se juzgan por permisos y se registran como petición no cuentan.
- **Un reloj muy adelantado espera.** Una operación con un HLC más de una hora por delante del reloj de quien la recibe (`MAX_CLOCK_DRIFT_MILLIS`, `Hlc::is_ahead_of`) se rechaza sin tocar el reloj local, y retiene a las siguientes de su autor. Cuando el reloj de quien la recibe la alcanza, entra en la siguiente sincronización. Sin esto, un móvil con la fecha en 2099 ganaría todo campo que tocara y arrastraría los relojes de los demás para siempre (DAT-005; test `an_operation_from_a_clock_far_ahead_waits_and_does_not_drag_the_clock`).
- **El contador del HLC no da la vuelta.** Si llega a su máximo, el reloj pasa al milisegundo siguiente con el contador a 0, así que nunca retrocede (SEG-020; tests en `crates/domain/src/hlc.rs`).
- Las operaciones anteriores a la sincronización (hitos 1.2 a 1.6) quedan como **historial**: se ven en la ficha del libro, pero **no se replican**. Tienen formatos antiguos (la editorial era texto) y ninguna otra copia depende de ellas.
- La primera vez que arranca una versión con sincronización, cada dispositivo registra una **operación inicial** (`sync.baseline`) que crea todo lo que ya tiene. Así viajan también los libros anteriores al registro de operaciones. En una instalación nueva, sin datos, esa operación no existe.
- Consecuencia: **un dispositivo nuevo no necesita una copia completa aparte**. Recibe todas las operaciones replicables, empezando por las iniciales de cada dispositivo.

## Por biblioteca (Fase 4)

Todo lo que sigue ocurre **biblioteca a biblioteca** (D46, D54):
- Cada operación pertenece a una biblioteca, y una fila se identifica por biblioteca, entidad e id. Versiones de campo y lápidas también son por biblioteca: nada de lo que pase en una toca otra.
- **Qué va a cada dispositivo** lo dice la tabla local `library_peers`. Solo se piden, se envían y se aceptan operaciones de bibliotecas compartidas con ese dispositivo; lo demás se descarta antes de fusionar. Ver «Compartir y retirar el acceso», más abajo.
- **Borrar una biblioteca** (D65) quita todas sus filas y deja una lápida de la biblioteca: cualquier cambio posterior en ella se ignora, tenga la marca que tenga. La operación de borrado se queda en el registro (D66) y sigue viajando a quien comparta esa biblioteca.

## Compartir y retirar el acceso (hito 4.4, D71)

**Cada dispositivo elige lo que envía.** Al emparejar, los dos marcan qué bibliotecas comparten con el otro, y se puede cambiar después en «Compartir y dispositivos». Nada sale sin marcarlo.

**La oferta.** En cada sincronización, las claves de los vectores que manda cada lado son su oferta: las bibliotecas que comparte con el otro. Al recibirla, cada dispositivo concilia (`lan::reconcile`):
- **Una biblioteca que nunca tuvo** la acepta: la añade a `library_peers` y la recibe entera. Si la acepta el cliente, hace una segunda vuelta en la misma sincronización para traerla. Una biblioteca **borrada** cuenta como tenida: conserva su registro y su borrado (D65, D66), y aceptarla sola llevaría ese borrado a quien nunca la compartió con este dispositivo (SEG-010, `libraries::library_known`).
- **Una biblioteca que ya tiene** no la acepta por su cuenta: compartir lo propio es siempre una decisión de ese dispositivo. Esto vale también para un id repetido a propósito (una copia, un par hostil): ningún id está fijo desde D85, así que dos dispositivos solo tienen la misma biblioteca si una llegó a la otra sincronizando, y solo se une si **los dos** la marcan.
- Una biblioteca viaja solo cuando **los dos lados** la comparten.

**Retirar el acceso** (desmarcar en «Qué va a…» o «Dejar de compartir» en los ajustes de la biblioteca):
- Se registra una operación `library.revoke` que crea una fila `revocation` (dispositivo y su nombre) en esa biblioteca. Viaja a todos los que la comparten, y **ninguno** vuelve a compartirla con ese dispositivo: la retirada gana sobre la lista local de cada uno.
- Cada sincronización lleva unos **avisos** (`SharingNotices`): `revoked`, las bibliotecas a las que el otro ya no tiene acceso, y `proofs`, las operaciones firmadas que crearon esas retiradas. Al recibirlo, el afectado fusiona esas operaciones y las juzga como cualquier otra (firma y permisos, D73); solo si queda una retirada que le nombra **borra la biblioteca** (contenido y registro, como al salir) y lo cuenta en el informe (`removed`). Un aviso sin retirada válida no borra nada (SEG-011): un compañero con una app modificada no puede hacer que otro pierda una biblioteca. Además, solo lo acepta de un dispositivo con el que compartía esa biblioteca.
- **Lo que firme después no entra en ningún sitio** (SEG-015, D84): mientras siga en vigor la retirada, toda operación de ese dispositivo con HLC posterior pasa a petición, con roles o sin ellos, también si llega a través de otro dispositivo que aún no sabía de la retirada. Al llegar la retirada tarde, la biblioteca se recalcula desde su registro. Si se le devuelve el acceso, lo que firme a partir de entonces vuelve a contar; lo de entre medias se queda como petición.
- **Devolver el acceso** (`library.grant`) borra la fila de retirada y vuelve a compartir: la biblioteca llega entera otra vez.

**Salir** de una biblioteca (D65) deja un aviso pendiente para cada dispositivo con el que se compartía (tabla local `leave_notices`): en la próxima sincronización se le dice `left` y deja de ofrecerla. El aviso se borra cuando ese dispositivo ya no la ofrece. Así no vuelve a aceptarse sola.

**Límites** (dichos también en D71):
- el afectado se entera cuando vuelve a conectar con cualquiera de los dispositivos que la tienen;
- los dispositivos que la recibieron **a través** del afectado (otros suyos) no se conocen hasta que haya cuentas (Fase 5), y la conservan;
- lo que ya se haya exportado no se puede borrar.

## Permisos al fusionar y recalcular (D73)

- **Cada operación recibida se juzga** con los roles vigentes (`permissions::allowed`). Si está permitida, se aplica como siempre; si no, se guarda como petición.
- **Si llega tarde algo en una biblioteca con roles** (una operación con HLC anterior a la más reciente que ya había), el dispositivo **recalcula la biblioteca desde su registro** (`permissions::rebuild`):
  1. vacía su contenido, pero conserva la fila de la biblioteca, porque la migrada no tiene operación de creación;
  2. vuelve a aplicar todas sus operaciones replicables en orden de HLC, juzgando cada una;
  3. rehace el índice de búsqueda.

  Así, si a Laura le quitaron el rol mientras su móvil estaba sin conexión, lo que hizo después pasa a petición en todos los dispositivos, llegue en el orden que llegue.
- **Cada operación encadena la anterior de su autor (D87):** lleva `previous`, firmado con el resto: el id de la última operación replicable del mismo dispositivo en la misma biblioteca cuando se hizo, o `null` si es la primera. Las peticiones (operaciones que quedan como petición) encadenan como las demás. La clave es obligatoria en la entrada: una operación sin ella no se lee.
- **Regla de espera al recibir (`sync::previous_holds`):** una operación cuya `previous` no está en el registro **espera**: no se guarda, tapa a las siguientes de su autor en el lote (`held_back`) y cuenta como rechazada; el vector del autor se queda por debajo y la siguiente sincronización la reenvía con todas las que faltan. Si `previous` está pero es de otro dispositivo, de otra biblioteca o tiene un HLC no menor que el de la operación, la cadena está mal formada: se rechaza y tapa igual. Una `previous` nula cuando el autor ya tiene operaciones, o dos operaciones con la misma `previous` (una bifurcación), **se aceptan** como antes: las gestionan las vallas, abajo, y el límite de más adelante. Nunca se salta una operación que falta, así que el vector de versiones ya no puede dejar un hueco sin que nadie lo vea (DAT-001 a DAT-004).
- **Vallas contra el antedatado (SEG-014, SEG-018, D84, D87):** el rol se juzga en el HLC de la operación, y ese HLC lo pone quien firma; sin más, quien acaba de perder poder podría firmar un cambio con fecha anterior a la bajada. Por eso toda operación que quita poder a una persona (bajar el rol, quitar «Puede compartir», cambiar de quién es un dispositivo o dejarlo sin persona, retirar el acceso) lleva, en la misma operación, un `trusted_through` por cada dispositivo afectado: el **id** de la última operación de ese dispositivo que tiene quien la registra (`""` si no tiene ninguna). La valla es `(dispositivo, through, until)`, con `until` el HLC de la operación que la pone. **Una operación del dispositivo con HLC menor que `until` pasa a petición, salvo que sea `through` o una de sus anteriores por `previous`**; después de `until` decide el rol, como siempre. Así no importa con qué fecha firme una operación nueva: si no cuelga de la cadena que conocía el administrador, no cuenta. Cada valla cuenta, no solo la última: se guardan en `device_fences`, y la cadena de cada una (`through` y sus anteriores) se calcula una sola vez al anotarla, en `device_fence_chains`; las dos salen siempre del registro. Si `through` aún no está en el registro (sus operaciones esperan), la valla cubre todo lo del dispositivo anterior a `until`, y cuando `through` llega, más antigua que lo ya recibido, la biblioteca se recalcula. Solo valen las vallas puestas por operaciones permitidas (un administrador; un dispositivo nunca fija la suya), así que `rebuild` parte de todas las del registro, descarta las que hayan quedado en petición y repite hasta que no cambie nada; termina siempre y da lo mismo en todos los dispositivos. Al fusionar, si una valla aplicada deja bajo sospecha una operación que el registro ya tenía, la biblioteca se recalcula. La cláusula de retiradas se mantiene: todo lo que un dispositivo retirado firma después de la retirada (SEG-015) queda fuera, y su valla lo deja fuera también por debajo. **Coste aceptado:** una edición honrada hecha sin conexión justo antes de enterarse de la bajada queda como petición; quien administra puede aceptarla y no se pierde nada.
- **Límite que queda (D87):** un editor con una app modificada puede bifurcar su propia cadena (dos operaciones con la misma `previous`) y que solo la reciban los dispositivos que iban por detrás. La cadena lo hace detectable, pero no se resuelve: es un riesgo aceptado del modelo de amenazas, limitado a quien ya podía editar y a lo que ya podía hacer. Con la valla de un editor degradado la bifurcación no sirve de nada: lo que no cuelga de lo que el administrador conocía es petición.
- **Sin roles solo se recalcula si hay marcas o retiradas** (`permissions::judged_by_order`): sin ellas se sigue fusionando campo a campo, como hasta ahora.
- **Red de seguridad:** todos los tests de convergencia comprueban además que recalcular desde el registro deja cada biblioteca exactamente igual (`assert_rebuild_changes_nothing`). Los casos de `spec/conformance/permissions.json` se ejecutan en `sync::tests::permission_conformance_cases`.

## Qué le falta a quién: el vector de versiones

Cada dispositivo sabe, para cada biblioteca y cada autor de operaciones, cuál es la última que tiene: `{biblioteca → {dispositivo → última marca HLC}}`. Se calcula del registro, sin tablas extra.

Una sincronización entre A y B:
1. A envía su vector a B.
2. B responde con las operaciones que tiene y a A le faltan (las de cada dispositivo posteriores a lo que dice el vector de A), ordenadas por HLC, y con su propio vector.
3. A las aplica.
4. A envía a B las que a B le faltan, según el vector de B. B las aplica.

Se reenvían también las operaciones de terceros: si A sincroniza con B y B con C, lo de A llega a C a través de B.

## Reglas al aplicar operaciones recibidas (D32)

Para cada cambio de cada operación, en orden de HLC:

| Situación | Regla |
|---|---|
| Operación ya conocida (mismo id) | Se ignora: sincronizar dos veces no cambia nada |
| La fila tiene **lápida** (fue borrada) | Se ignora el cambio: **el borrado gana** |
| Crear una fila que no existe | Se crea |
| Crear una fila que ya existe | Se trata como una modificación |
| Modificar | **Campo a campo**: cada campo se escribe solo si la marca de la operación es posterior a la última que escribió ese campo |
| Modificar una fila que no existe | Se ignora (fue borrada en cascada) |
| Borrar | Se borra y se deja una **lápida** con la marca |
| Referencia a una fila que ya no existe (un ejemplar de un libro borrado en otro dispositivo) | Se ignora el cambio |

Para aplicar «campo a campo» cada dispositivo guarda, por cada campo de cada fila, la marca HLC que lo escribió por última vez (tabla `field_versions`). Los borrados quedan en `tombstones`.

Al recibir operaciones, el reloj HLC local salta por delante de las marcas recibidas (`HlcClock::receive`), para que lo que hagas después quede siempre por detrás de lo que ya has visto.

Nada de esto rompe una sincronización: un cambio que no se puede aplicar se salta y se cuenta, pero la operación queda registrada y se sigue reenviando a otros dispositivos.

### Límite conocido: borrar un padre que tiene hijos sin cascada (DAT-006)

Algunas claves foráneas no tienen `ON DELETE CASCADE`: `editions.work_id`, y las referencias a `publisher`, `member` (dueño) y `person`. Si un dispositivo borra la fila padre mientras otro le crea un hijo, el borrado choca con la clave foránea, `apply_change` lo deshace sin lápida y el resultado depende del orden de llegada. **Hoy no se alcanza desde la app**: añadir una edición crea siempre su obra, y no hay forma de borrar una persona, una editorial ni un miembro. **Antes de construir** «añadir una edición a una obra existente», «quitar un miembro», o borrar personas o editoriales, hay que arreglarlo (cascada o `SET NULL`, o que el borrado gane con su lápida) con un test de los dos órdenes y `assert_converged` (auditoría 2026-10-03-3).

## Duplicados (D33)

Si en dos dispositivos, sin conexión, se da de alta el mismo autor, la misma editorial, el mismo propietario o el mismo ISBN, al sincronizar **se conservan los dos**. Rechazar uno perdería datos; fusionarlos automáticamente podría fusionar cosas distintas. Por eso:
- Los nombres y el ISBN dejan de ser únicos en la base de datos (siguen siéndolo para lo que tecleas: la app reutiliza lo existente).
- Las listas de sugerencias muestran cada nombre una vez.
- Más adelante habrá una opción para **fusionar duplicados** a mano, que se registrará como una operación más.

## Transporte (D34, hito 2.1)

**Quién sirve y quién se conecta.** El dispositivo que muestra el código de emparejamiento **sirve**; el que lo introduce **se conecta**. Normalmente sirve el escritorio y se conecta el móvil, pero funciona en cualquier sentido (dos escritorios, por ejemplo).

**Identidad.** Cada dispositivo genera la primera vez un certificado propio (ECDSA P-256) y lo guarda en su carpeta de datos (`sync-cert.der` y `sync-key.der`; en Linux, macOS y Android la clave se escribe con permisos `0600`, legible solo por el usuario; en Windows hereda los permisos de la carpeta de datos, que en `%APPDATA%` ya son solo del usuario). Su huella SHA-256 es lo que el otro dispositivo fija.

**Código de emparejamiento** (`KOSMOTHEKA1:` + JSON en base64url, también como QR): identificador y nombre del dispositivo, sus direcciones `ip:puerto` en la red local, la huella del certificado y un **secreto de un solo uso** que caduca a los 10 minutos.

**Emparejar** (`POST /v1/pair`):
1. El que se conecta prueba las direcciones del código hasta que una responde. Solo acepta el certificado cuya huella viene en el código: cualquier otro se rechaza como impostor.
2. Envía el secreto de un solo uso, su identificador y su nombre. Cada lado guarda en `library_peers` las bibliotecas que marcó para el otro; las del servidor van en la ventana de emparejamiento abierta al mostrar el código.
3. El servidor, si el secreto es válido, lo invalida (no sirve para un segundo dispositivo) y devuelve un **secreto permanente**. De ese secreto solo guarda el hash.
   - Si el identificador ya está emparejado, responde `403` (`already_paired`): el identificador lo declara el cliente, y aceptarlo dejaría a otro quitarle el sitio a ese dispositivo y quedarse con lo que se compartía con él. Para volver a emparejar, se olvida antes en el servidor.
4. El cliente guarda al servidor: nombre, direcciones, huella y secreto.

**Sincronizar** (autenticado con `X-Device-Id` y `Authorization: Bearer <secreto>`):
1. `POST /v1/sync/pull` con la versión de esquema, los vectores del cliente (uno por biblioteca que comparte con el servidor) y sus avisos. El servidor concilia, responde con las operaciones que le faltan al cliente de las bibliotecas que ambos comparten, con sus propios vectores de las que él comparte y con sus avisos. Si las versiones de esquema difieren, responde `409` y no se sincroniza.
2. El cliente concilia con la oferta y los avisos del servidor, aplica las operaciones (`merge`) y calcula las que le faltan al servidor.
3. `POST /v1/sync/push` con esas operaciones. El servidor las aplica.
4. El cliente coloca primero la dirección que ha funcionado, para la próxima vez.

**Cuándo escucha el servidor.** Solo cuando hace falta: si hay dispositivos que se sincronizan con este, o mientras se muestra un código de emparejamiento. El puerto se guarda para que los emparejados lo encuentren en el mismo sitio; si está ocupado, se usa otro. En Windows, la primera vez el cortafuegos preguntará si se permite.

**Cuándo se sincroniza** (D75, hito 7.3). En silencio y sin preguntar cada cierto tiempo; si un dispositivo o la nube no responde, no pasa nada. También a mano, con «Sincronizar ahora».
- **Al abrir la app y al volver a primer plano**, con todos los dispositivos emparejados a los que se conecta y con la nube, como mucho una vez por minuto.
- **Tras cada cambio propio:** cada operación que este dispositivo registra despierta un aviso interno (`ops::local_change_burst`); el núcleo espera 2 segundos más para juntar una ráfaga y emite `local-changed`. La interfaz hace entonces la sincronización completa (red local y después nube) sin el límite del minuto; si ya había una en marcha, se repite una vez al terminar. Las operaciones que llegan de otros dispositivos pasan por `sync::merge`, que no registra nada, y nunca lo despiertan.
- **Cuando la nube avisa** (`cloud-changed`): se sincroniza solo la nube, sin límite y con la misma regla de no solaparse (`runCloudSync` lo guarda y repite una vez).
- **Cuando llega algo por la red local** (`library-changed`): un dispositivo emparejado hace `push` a este y trae operaciones nuevas; el servidor local avisa a la interfaz (`ServerState::arrival`, que fija la app al arrancar), que recarga y hace la sincronización completa para **reenviarlo** a los dispositivos a los que se conecta y a la nube. Así un móvil hace de puente entre otro móvil sin cuenta, el portátil y la nube. No hay bucle: solo avisa un `push` que trae algo nuevo.
- **Cada 5 minutos con la app en primer plano**, solo con la red local: un servidor local no puede avisar a sus clientes, así que cada cliente le pregunta (hasta D88). La nube entra en esa vuelta solo si la red local trajo algo nuevo; si no, ya la cubre la conexión en directo. Si hay otra sincronización en marcha, esa vuelta se salta.

**La conexión en directo con la nube** (`cloud/events.rs`, D75). Mientras hay sesión, el núcleo mantiene **una** conexión abierta con el hub Mercure del servidor, en `<servidor>/.well-known/mercure` (la dirección la construye la app, nunca sale de una respuesta, D84):
1. Pide `GET /v1/cloud/events` (token del hub, temas y caducidad) y se suscribe a esos temas con `Authorization: Bearer`.
2. Al conectar emite `cloud-connection` `{ live: true }` y un `cloud-changed` para ponerse al día de lo ocurrido mientras no escuchaba.
3. Cada evento del hub (el contenido no se usa) emite `cloud-changed`, a lo sumo uno por segundo: el primero sale al momento y una ráfaga se junta en uno más.
4. Un minuto antes de que caduque el token, se reconecta con uno nuevo, sin pasar por «sin conexión».
5. Ante un error de red, el cierre del servidor o 90 segundos sin recibir nada (los latidos cuentan), emite `{ live: false }` y reintenta a 1, 2, 4… hasta 60 segundos. La interfaz llama a `cloud_events_wake` al volver a primer plano para saltarse la espera (Android puede haber matado el socket).
6. Se detiene, hasta el siguiente inicio de sesión o arranque de la app, si el servidor no tiene hub (`events: false` en `GET /v1/server`, o `events_unavailable`) o si el servidor cierra la sesión o retira el dispositivo (la misma reacción que las demás llamadas de la nube).

**Sin hub, todo sigue como antes (D18):** la app sincroniza al abrir, al volver a primer plano y tras cada cambio, y el indicador de «en directo» no aparece. La interfaz muestra una marca discreta junto al estado de sincronización, solo con sesión y con hub: «En directo» o «Sin conexión en directo».

**Si el otro ha cambiado de dirección (D37).** Cuando ninguna dirección guardada responde, se recorre la subred propia (hasta 1022 direcciones, 64 a la vez) en los puertos que el otro usaba, pidiendo `GET /v1/hello` con el certificado fijado. Solo se acepta al que presenta ese certificado y el identificador de dispositivo esperado. Si aparece, se guarda la nueva dirección y se sincroniza. No se usa mDNS porque en Android exigiría código nativo para recibir multicast.

**Emparejar desde el móvil.** El escritorio muestra el QR; el móvil lo escanea con la cámara (el mismo plugin que lee los ISBN, D35) y empareja sin teclear nada.

**Probar con dos instancias en un mismo equipo.** En las compilaciones de desarrollo, la variable `KOSMOTHEKA_DATA_DIR` hace que una segunda instancia use otra carpeta de datos (ver `docs/testing/fase-2.md`).

## Excepción a «toda escritura es una operación»

Aplicar operaciones recibidas **no genera operaciones nuevas**: se registran las recibidas, tal cual. Es replicación, no una acción del usuario.

## Dónde está en el código (hito 2.0)

- `app/src-tauri/src/sync.rs`: operación inicial, vector de versiones, `operations_since` (lo que le falta al otro) y `merge` (aplicar lo recibido). Sus tests simulan dispositivos con bases de datos en memoria y comprueban que todos acaban con el mismo estado (`assert_converged`).
- `app/src-tauri/src/ops.rs`: las operaciones locales también registran las versiones de campo y las lápidas, así que local y remoto siguen las mismas reglas.
- `app/src-tauri/src/lan/`: identidad y certificado (`identity.rs`), código de emparejamiento (`pairing.rs`), servidor HTTPS (`server.rs`), cliente con certificado fijado (`client.rs`) y el servicio que los une a la app (`service.rs`). Sus tests emparejan y sincronizan por TLS real, también por la IP de la red local.
- Casos de conformidad de la fusión por biblioteca: `spec/conformance/library-merge.json`, que la app pasa en `sync::tests::merge_conformance_cases`. Pendiente: casos para el resto de reglas (campo a campo, el borrado gana, referencias rotas), para que el servidor PHP (Fase 7) aplique exactamente las mismas.

