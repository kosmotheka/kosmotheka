# Sincronización

> Cómo se ponen de acuerdo varios dispositivos sin servidor (Fase 2) y, más adelante, con él (Fase 7). Decisiones de fondo: D05, D13, D25 y D31–D34. Documento vivo: se completa hito a hito.

## Idea

Cada dispositivo guarda todas las operaciones (las suyas y las recibidas). Sincronizar es **intercambiar las operaciones que le faltan al otro** y aplicarlas con unas reglas que hacen que **todos los dispositivos lleguen al mismo estado**, reciban las operaciones en el orden que las reciban. No hay árbitro: las reglas se encargan de ello.

## Qué operaciones se replican (D31)

- Cada operación guarda la **versión del esquema** con la que se registró. Dos dispositivos con versiones distintas no se sincronizan: la app pide actualizar primero. Convertir operaciones antiguas queda para cuando haga falta.
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
- **Una biblioteca que no tiene** la acepta: la añade a `library_peers` y la recibe entera. Si la acepta el cliente, hace una segunda vuelta en la misma sincronización para traerla.
- **Una biblioteca que ya tiene** no la acepta por su cuenta: compartir lo propio es siempre una decisión de ese dispositivo. Por eso «Mi biblioteca» (el mismo id en todos, D46) solo se une si **los dos** la marcan.
- Una biblioteca viaja solo cuando **los dos lados** la comparten.

**Retirar el acceso** (desmarcar en «Qué va a…» o «Dejar de compartir» en los ajustes de la biblioteca):
- Se registra una operación `library.revoke` que crea una fila `revocation` (dispositivo y su nombre) en esa biblioteca. Viaja a todos los que la comparten, y **ninguno** vuelve a compartirla con ese dispositivo: la retirada gana sobre la lista local de cada uno.
- Cada sincronización lleva unos **avisos** (`SharingNotices`): `revoked`, las bibliotecas a las que el otro ya no tiene acceso. Al recibirlo, el afectado **borra la biblioteca** (contenido y registro, como al salir) y lo cuenta en el informe (`removed`). Solo lo acepta de un dispositivo con el que compartía esa biblioteca.
- Sus cambios que no llegaron antes de la retirada se descartan: ya no se comparte con él.
- **Devolver el acceso** (`library.grant`) borra la fila de retirada y vuelve a compartir: la biblioteca llega entera otra vez.

**Salir** de una biblioteca (D65) deja un aviso pendiente para cada dispositivo con el que se compartía (tabla local `leave_notices`): en la próxima sincronización se le dice `left` y deja de ofrecerla. El aviso se borra cuando ese dispositivo ya no la ofrece. Así no vuelve a aceptarse sola.

**Límites** (dichos también en D71):
- el afectado se entera cuando vuelve a conectar con cualquiera de los dispositivos que la tienen;
- los dispositivos que la recibieron **a través** del afectado (otros suyos) no se conocen hasta que haya cuentas (Fase 5), y la conservan;
- lo que ya se haya exportado no se puede borrar.

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

## Duplicados (D33)

Si en dos dispositivos, sin conexión, se da de alta el mismo autor, la misma editorial, el mismo propietario o el mismo ISBN, al sincronizar **se conservan los dos**. Rechazar uno perdería datos; fusionarlos automáticamente podría fusionar cosas distintas. Por eso:
- Los nombres y el ISBN dejan de ser únicos en la base de datos (siguen siéndolo para lo que tecleas: la app reutiliza lo existente).
- Las listas de sugerencias muestran cada nombre una vez.
- Más adelante habrá una opción para **fusionar duplicados** a mano, que se registrará como una operación más.

## Transporte (D34, hito 2.1)

**Quién sirve y quién se conecta.** El dispositivo que muestra el código de emparejamiento **sirve**; el que lo introduce **se conecta**. Normalmente sirve el escritorio y se conecta el móvil, pero funciona en cualquier sentido (dos escritorios, por ejemplo).

**Identidad.** Cada dispositivo genera la primera vez un certificado propio (ECDSA P-256) y lo guarda en su carpeta de datos (`sync-cert.der` y `sync-key.der`, esta última legible solo por el usuario). Su huella SHA-256 es lo que el otro dispositivo fija.

**Código de emparejamiento** (`KOSMOTHEKA1:` + JSON en base64url, también como QR): identificador y nombre del dispositivo, sus direcciones `ip:puerto` en la red local, la huella del certificado y un **secreto de un solo uso** que caduca a los 10 minutos.

**Emparejar** (`POST /v1/pair`):
1. El que se conecta prueba las direcciones del código hasta que una responde. Solo acepta el certificado cuya huella viene en el código: cualquier otro se rechaza como impostor.
2. Envía el secreto de un solo uso, su identificador y su nombre. Cada lado guarda en `library_peers` las bibliotecas que marcó para el otro; las del servidor van en la ventana de emparejamiento abierta al mostrar el código.
3. El servidor, si el secreto es válido, lo invalida (no sirve para un segundo dispositivo) y devuelve un **secreto permanente**. De ese secreto solo guarda el hash.
4. El cliente guarda al servidor: nombre, direcciones, huella y secreto.

**Sincronizar** (autenticado con `X-Device-Id` y `Authorization: Bearer <secreto>`):
1. `POST /v1/sync/pull` con la versión de esquema, los vectores del cliente (uno por biblioteca que comparte con el servidor) y sus avisos. El servidor concilia, responde con las operaciones que le faltan al cliente de las bibliotecas que ambos comparten, con sus propios vectores de las que él comparte y con sus avisos. Si las versiones de esquema difieren, responde `409` y no se sincroniza.
2. El cliente concilia con la oferta y los avisos del servidor, aplica las operaciones (`merge`) y calcula las que le faltan al servidor.
3. `POST /v1/sync/push` con esas operaciones. El servidor las aplica.
4. El cliente coloca primero la dirección que ha funcionado, para la próxima vez.

**Cuándo escucha el servidor.** Solo cuando hace falta: si hay dispositivos que se sincronizan con este, o mientras se muestra un código de emparejamiento. El puerto se guarda para que los emparejados lo encuentren en el mismo sitio; si está ocupado, se usa otro. En Windows, la primera vez el cortafuegos preguntará si se permite.

**Cuándo se sincroniza.** Al abrir la app y cada vez que vuelve a primer plano (como mucho una vez por minuto), con todos los dispositivos emparejados a los que se conecta, en silencio: si alguno no está, no pasa nada. También a mano, con «Sincronizar ahora».

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

