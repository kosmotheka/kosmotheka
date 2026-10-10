# Seguridad: qué protege Kosmotheka y cómo

> **Borrador (10/10/2026).** Está escrito para que lo entienda quien no ha seguido el desarrollo. Si una frase no se entiende, el fallo es de la página. La Fase 7 (la nube) está a medias: cada pieza dice si está construida. El detalle técnico está en [sync.md](sync.md), [cloud.md](cloud.md), [accounts.md](accounts.md) y en `spec/README.md`.

## La idea en cuatro frases

1. Tus datos viven en tus dispositivos. La app funciona entera sin servidor y sin cuenta.
2. Una biblioteca solo llega a quien alguien de la familia se la ha dado: por un código QR en casa, o por la nube si quien la administra decidió subirla.
3. Dentro de la familia hay roles (quien administra, quien edita, quien solo lee). Cada dispositivo comprueba por su cuenta que los demás los respetan: no hace falta un servidor que vigile.
4. Nada se pierde: lo que alguien intenta sin tener permiso no se borra, queda como una **petición** que quien administra acepta o rechaza.

## De quién protege

Hay dos fronteras, y conviene no mezclarlas.

| Frontera | Ejemplo | Qué está en juego |
|---|---|---|
| **Los de fuera** | Un vecino en la misma wifi, alguien en internet, quien encuentra un móvil | Que lean tu biblioteca: qué tienes, dónde está, a quién se lo prestaste |
| **Los de dentro** | Un familiar que solo lee e instala una versión modificada de la app para saltarse su rol | Que cambie o borre el catálogo, o se haga administrador. Leer, ya podía |

La primera es la importante. La segunda es la que más mecanismos necesita, porque quien ataca ya tiene una copia de la biblioteca y un dispositivo que los demás conocen.

## Las piezas

Cada pieza dice qué problema resuelve, con un ejemplo. Entre paréntesis, el nombre que tiene en el código y en el resto de la documentación.

### 1. Todo cambio va firmado

**Problema:** que alguien haga un cambio haciéndose pasar por el móvil de otra persona.

Cada dispositivo tiene una clave secreta que nunca sale de él, y firma con ella cada cambio que hace. Los demás conocen su clave pública y comprueban la firma. Un cambio sin firma, o con una firma que no cuadra, se descarta. La clave de un dispositivo no cambia nunca: si llega otra distinta para el mismo dispositivo, se rechaza.

*Construida. Nombres: operación firmada, `public_key`.*

### 2. Cada dispositivo comprueba los permisos

**Problema:** que un lector edite el catálogo con una app modificada.

Cuando llega un cambio, cada dispositivo mira quién lo firmó, de qué persona es ese dispositivo y qué rol tenía esa persona **en ese momento**. Si no podía hacerlo, el cambio no se aplica: queda guardado como petición. Todos los dispositivos, y también la nube, siguen las mismas reglas y llegan al mismo resultado, reciban los cambios en el orden que sea. Esas reglas están escritas una sola vez, con casos de prueba que pasan igual la app y el servidor.

*Construida. Nombres: roles (`admin`, `editor`, `reader`), petición (`request`), casos de conformidad.*

### 3. No se puede colar un cambio «con fecha de ayer»

**Problema:** a Jhonny le quitan el rol de editor hoy. Con una app modificada, escribe un cambio y le pone fecha de la semana pasada, cuando aún era editor.

Dos defensas:

- **Cada dispositivo numera lo suyo.** Cada cambio dice cuál fue el anterior de ese mismo dispositivo, así que no se puede intercalar uno nuevo en medio de los antiguos sin que se note. *(`previous`, la cadena de cada autor.)*
- **Al quitar poder se anota hasta dónde se le cree.** Quien quita el rol a un dispositivo apunta, en ese mismo cambio, cuál es el último cambio de ese dispositivo que ha visto. Lo que aparezca después con fecha anterior no se cree: queda como petición. *(Valla, `fence`, `trusted_through`.)*

Lo mismo se hace una vez para todos al activar los roles en una biblioteca: desde ese momento, nada fechado antes y aparecido después se da por bueno sin más. *(Sello, `seal`.)*

*Construidas.*

### 4. Una biblioteca solo llega a quien se le da

**Problema:** que alguien en la misma wifi reciba tu biblioteca.

- **En casa**, dos dispositivos se emparejan con un código QR que se enseña en pantalla. A partir de ahí hablan cifrado y cada uno reconoce al otro por su certificado, no por su dirección en la red.
- **Quién puede dar una biblioteca a otro dispositivo:** quien la administra o quien tiene «Puede compartir».
- **Retirar el acceso:** quien administra puede decir que un dispositivo ya no recibe una biblioteca; los demás dejan de entregársela. Lo que ese dispositivo ya tenía copiado no se le puede quitar.
- **Echar a una persona:** quien administra puede decir que una persona ya no entra en la biblioteca. Sus dispositivos dejan de contar como suyos y se les retira el acceso, de modo que borran la biblioteca al conectar; su nombre y lo suyo (sus libros, sus préstamos) se quedan. Si vuelve, quien administra le devuelve el acceso y le vuelve a vincular un dispositivo. No se puede echar al último administrador.

*Construida. Nombres: emparejamiento, `can_share`, retirada (`revocation`).*

### 5. La nube: la cuenta y la lista de acceso

**Problema:** que una cuenta lea bibliotecas de otra.

- La nube es opcional. **Solo quien administra una biblioteca puede subirla**; registrarse no sube nada.
- El servidor guarda una lista: qué cuentas pueden entrar en qué biblioteca. Es lo único que decide el acceso. El servidor nunca se fía de un identificador que le mande la app: lo compara con lo que él tiene, no lo usa para buscar.
- A una biblioteca ajena, o que no existe, el servidor responde lo mismo, para que nadie averigüe cuáles existen.
- El servidor aplica las mismas reglas de roles que los dispositivos (pieza 2) a todo lo que guarda.

*Construida para una cuenta por biblioteca. Varias cuentas en la misma biblioteca: en construcción (hito 7.5).*

### 6. La nube es un dispositivo más, con un solo poder

**Problema:** te compras un móvil nuevo. Nadie en casa lo conoce, así que no tiene ningún rol hasta que alguien lo empareje a mano. Y si eras la única administradora y perdiste tu único móvil, nadie puede.

Al subir una biblioteca, la nube entra en ella como un dispositivo más, con su propia clave, y quien administra le da **un único poder**: decir «este dispositivo nuevo es de Laura» cuando Laura ha entrado en él con su cuenta. No puede cambiar roles, ni tocar el catálogo, ni quitarle un dispositivo a nadie. Los demás dispositivos lo comprueban como comprueban todo lo demás. Quien no se fíe de su servidor puede quitarle ese poder; entonces cada dispositivo nuevo se empareja a mano.

Cada biblioteca de la nube tiene su propia clave, guardada cifrada. El identificador de «la nube» en una biblioteca sale de su clave, de modo que nadie puede presentarse con ese identificador y otra clave.

*Construido: el poder y la identidad de la nube, y su uso. Cuando Laura entra con su cuenta en un móvil nuevo, el móvil baja la biblioteca, se da de alta en ella, y la nube dice que ese móvil es de Laura **apuntando la clave con la que ha entrado**. Todo en una sesión: al terminar, el móvil tiene su persona y su rol en todos los dispositivos de la familia. Vale igual para la administradora única que perdió su único móvil. La nube no vuelve a vincular un móvil al que un administrador quitó su persona o retiró el acceso, ni toca uno que ya es de otra persona, ni el que alguien ocupó con una clave que no es la suya. Si la nube no tiene el poder en esa biblioteca (no está nombrada, o se le quitó), no escribe nada. Nombres: `cloud_device_id`, `account_id`, `claim` (`device.link_account`).*

### 7. Decir de quién es un dispositivo incluye su clave

**Problema:** el identificador de un dispositivo no es secreto; lo ven todos en la familia. Un familiar con una app modificada podría presentar en la nube el identificador de la tablet nueva de la administradora, con una clave suya, antes de que llegue la de verdad. Si después la administradora dice «esa tablet es mía», la nube creería que lo es el dispositivo del familiar.

Por eso, quien dice «este dispositivo es de Laura» apunta también **la clave que él ha visto** de ese dispositivo. Un dispositivo solo cuenta como de Laura si su clave es esa. Presentarse con el identificador de otro y una clave propia deja de servir para nada. Es la misma idea que «vincular un dispositivo» en las aplicaciones de mensajería.

*En construcción (hito 7.5.2). Nombre: clave avalada, `vouched_key`.*

## Lo que no protege

Dicho claro, para que nadie lo dé por supuesto:

- **Quien administra el servidor puede leer las bibliotecas subidas a él.** No hay cifrado de extremo a extremo. Por eso la nube es opcional, se puede instalar un servidor propio, y subir es una decisión de quien administra la biblioteca.
- **Quien puede leer una biblioteca puede copiarla.** Ningún rol lo impide.
- **Un dispositivo retirado conserva lo que ya tenía.**
- **Un móvil desbloqueado en manos de otro actúa como su dueño**, hasta que alguien de la familia lo quite o se cambie la contraseña de la cuenta.
- **Un familiar con una app modificada puede estorbar**: por ejemplo, impedir que un dispositivo nuevo entre en la nube. No gana acceso ni rol, y quien administra puede echarlo.
- **Emparejar un dispositivo nuevo a través de alguien que quiere engañar** (pieza 7): si la clave falsa llega al propio dispositivo de quien administra antes que la de verdad, la daría por buena. Se nota, porque el dispositivo de verdad no funciona con nadie. Pendiente de mejorar.

## Cómo se comprueba

- **El caso hostil se prueba antes de escribir la defensa**, y se ve fallar: una app modificada, alguien que conoce los identificadores, un aviso falsificado.
- **Las reglas de permisos son casos compartidos**: los mismos ficheros los ejecutan la app (Rust) y el servidor (PHP).
- **Auditorías periódicas** con un método escrito, que cualquiera puede repetir: ver [`docs/audits/README.md`](../audits/README.md).
- **Al terminar el lado del servidor de la Fase 7** se hará una revisión de seguridad de conjunto, con todo construido.
