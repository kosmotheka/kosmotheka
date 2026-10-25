# Seguridad: qué protege Kosmotheka y cómo

> Puesto al día el 17/10/2026. Está escrito para que lo entienda quien no ha seguido el desarrollo. Si una frase no se entiende, el fallo es de la página. Todas las piezas descritas están construidas; lo que falta o queda como límite está en «Lo que no protege». El detalle técnico está en [sync.md](sync.md), [cloud.md](cloud.md), [accounts.md](accounts.md) y en `spec/README.md`.

## La idea en cuatro frases

1. Tus datos viven en tus dispositivos. La app funciona entera sin servidor y sin cuenta.
2. Una biblioteca solo llega a quien alguien de la familia se la ha dado: por un código QR en casa, o por la nube si quien la administra decidió subirla.
3. Dentro de la familia hay roles (quien administra, quien edita, quien solo lee). Cada dispositivo comprueba por su cuenta que los demás los respetan: no hace falta un servidor que vigile.
4. Nada se pierde: lo que alguien intenta sin tener permiso no se borra, queda como una **petición** que quien administra acepta o rechaza.

## De quién protege

Hay dos fronteras, y conviene no mezclarlas.

| Frontera | Ejemplo | Qué está en juego |
|---|---|---|
| **Los de fuera** | Un vecino en la misma wifi, alguien en internet, quien encuentra un móvil | Que lean tu biblioteca: qué tienes, dónde está, de quién es |
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

Y no hay un momento en que una biblioteca pase de no tener roles a tenerlos, que sería fechable hacia atrás: nace con su administrador, y solo vale la fundación de quien puede derivar el id de su clave (D97). *(Fundación, `founding`.)*

*Construidas.*

### 4. Una biblioteca solo llega a quien se le da

**Problema:** que alguien en la misma wifi reciba tu biblioteca.

- **En casa**, dos dispositivos se emparejan con un código QR que se enseña en pantalla. A partir de ahí hablan cifrado y cada uno reconoce al otro por su certificado, no por su dirección en la red.
- **Quién puede dar una biblioteca a otro dispositivo:** quien la administra o quien tiene «Puede compartir».
- **Retirar el acceso:** quien administra puede decir que un dispositivo ya no recibe una biblioteca; los demás dejan de entregársela. Lo que ese dispositivo ya tenía copiado no se le puede quitar.
- **Echar a una persona:** quien administra puede decir que una persona ya no entra en la biblioteca. Sus dispositivos dejan de contar como suyos y se les retira el acceso, de modo que borran la biblioteca al conectar; su nombre y lo suyo (sus libros, sus marcas) se quedan. Si vuelve, quien administra le devuelve el acceso y le vuelve a vincular un dispositivo. No se puede echar al último administrador.
- **Salir de la biblioteca (D102):** cualquier persona puede retirarse el acceso a sí misma (regla 15 de `spec/`): la marca de su propia persona y el corte de sus propios dispositivos, nada más, en una sola operación; si trae algo más, es una petición entera. Nadie puede ponerle la marca a otra persona ni quitársela a sí misma. **Límite conocido, a propósito:** quien se va no pone valla a sus otros dispositivos, solo al que firma (a su operación anterior, así que no se deja de creer nada suyo). Un dispositivo suyo modificado podría firmar después de irse operaciones con fecha anterior a la salida. La valla, que sí lo evitaría, desacreditaría también el trabajo de sus otros dispositivos que el que firma aún no ha visto, y quien lo haya visto y construido encima lo perdería; eso lo puede provocar cualquiera sin mala intención (un móvil que lleva días sin sincronizar con la tableta), y salir es voluntario: quien quiere seguir escribiendo no necesita salir y volver con fechas falsas. Hoy nadie lo cierra: echar a la persona otra vez no encuentra dispositivos que apunten a ella, y retirar un dispositivo (D71) no le pone valla si su retirada ya existe, como es el caso. Cerrarlo pediría una acción nueva de quien administra (vallar un dispositivo ya retirado); no está construida. El último administrador no sale: lo comprueba la app, igual que al echar, no el juez.

*Construida. Nombres: emparejamiento, `can_share`, retirada (`revocation`), acceso retirado (`access_withdrawn`).*

### 5. La nube: la cuenta y la lista de acceso

**Problema:** que una cuenta lea bibliotecas de otra.

- La nube es opcional. **Solo quien administra una biblioteca puede subirla**; registrarse no sube nada.
- El servidor guarda una lista: qué cuentas pueden entrar en qué biblioteca. Es lo único que decide el acceso. El servidor nunca se fía de un identificador que le mande la app: lo compara con lo que él tiene, no lo usa para buscar.
- A una biblioteca ajena, o que no existe, el servidor responde lo mismo, para que nadie averigüe cuáles existen.
- El servidor aplica las mismas reglas de roles que los dispositivos (pieza 2) a todo lo que guarda.
- **En una biblioteca con roles, por la nube solo lee quien es alguien en ella.** Una cuenta que no corresponde a ninguna persona de la biblioteca no recibe nada, aunque fuera quien la subió. Así nadie queda fuera del alcance de «echar a una persona».
- **Si echan a una persona o retiran un dispositivo, la nube deja de entregarles la biblioteca al momento**, aunque entren desde un móvil nuevo; a la cuenta echada se le olvida también el vínculo con su persona. Al dispositivo cortado le enseña la prueba firmada de quien administra, y el dispositivo solo borra la biblioteca si esa prueba es buena: un servidor que mintiera no podría hacer perder una biblioteca. Quien nunca tuvo la biblioteca sigue recibiendo «no existe», no «te la quitaron».

*Construida, también con varias cuentas en la misma biblioteca (cada una es una `membership` ligada a una persona). Nombres: `cloud_library`, `membership`.*

### 6. La nube es un dispositivo más, con un solo poder

**Problema:** te compras un móvil nuevo. Nadie en casa lo conoce, así que no tiene ningún rol hasta que alguien lo empareje a mano. Y si eras la única administradora y perdiste tu único móvil, nadie puede.

Al subir una biblioteca, la nube entra en ella como un dispositivo más, con su propia clave, y quien administra le da **un único poder**: decir «este dispositivo nuevo es de Laura» cuando Laura ha entrado en él con su cuenta. No puede cambiar roles, ni tocar el catálogo, ni quitarle un dispositivo a nadie. Los demás dispositivos lo comprueban como comprueban todo lo demás. Quien no se fíe de su servidor puede quitarle ese poder; entonces cada dispositivo nuevo se empareja a mano.

Cada biblioteca de la nube tiene su propia clave, guardada cifrada. El identificador de «la nube» en una biblioteca sale de su clave, de modo que nadie puede presentarse con ese identificador y otra clave.

*Construido: el poder y la identidad de la nube, y su uso. Cuando Laura entra con su cuenta en un móvil nuevo, el móvil baja la biblioteca, se da de alta en ella, y la nube dice que ese móvil es de Laura **apuntando la clave con la que ha entrado**. Todo en una sesión: al terminar, el móvil tiene su persona y su rol en todos los dispositivos de la familia. Vale igual para la administradora única que perdió su único móvil. La nube no vuelve a vincular un móvil al que un administrador quitó su persona o retiró el acceso, ni toca uno que ya es de otra persona, ni el que alguien ocupó con una clave que no es la suya. Si la nube no tiene el poder en esa biblioteca (no está nombrada, o se le quitó), no escribe nada. Nombres: `cloud_device_id`, `account_id`, `claim` (`device.link_account`).*

### 7. Decir de quién es un dispositivo incluye su clave

**Problema:** el identificador de un dispositivo no es secreto; lo ven todos en la familia. Un familiar con una app modificada podría presentar en la nube el identificador de la tablet nueva de la administradora, con una clave suya, antes de que llegue la de verdad. Si después la administradora dice «esa tablet es mía», la nube creería que lo es el dispositivo del familiar.

Por eso, quien dice «este dispositivo es de Laura» apunta también **la clave que él ha visto** de ese dispositivo. Un dispositivo solo cuenta como de Laura si su clave es esa. Presentarse con el identificador de otro y una clave propia deja de servir para nada. Es la misma idea que «vincular un dispositivo» en las aplicaciones de mensajería.

*Construida, en los dispositivos y en la nube. Nombre: clave avalada, `vouched_key`. Queda un límite, en «Lo que no protege».*

### 8. Invitar a alguien de fuera

**Problema:** quieres que una persona entre en tu biblioteca por la nube, sin estar en tu casa. Un enlace viaja por un chat o un correo, y puede acabar en manos que no son las de esa persona.

- **Solo invita quien administra**, y el servidor lo comprueba en sus propias listas, no por lo que diga la app. Si quien invitó deja de administrar, o lo echan, **antes de que se acepte**, la invitación ya no vale.
- **Una invitación vale una vez y 24 horas**, y quien administra puede anularla antes. El servidor no guarda el enlace, solo una huella suya: ni él mismo podría enseñarlo después. El enlace es un número larguísimo al azar; probarlos a ciegas no lleva a ninguna parte y además el servidor corta a quien lo intenta muchas veces.
- **Hay dos clases.** *Por correo*: el servidor manda el enlace a esa dirección y solo lo abre una cuenta que se registró con ese mismo correo; sirve también para alguien que ya está en la biblioteca, que **conserva su rol**. *Enlace para copiar*: lo abre quien llegue primero, así que **solo da el rol de lectora y solo a una persona nueva**, sin libros, marcas ni dispositivos suyos en la biblioteca. Así un enlace que se filtra no deja que un desconocido ocupe el sitio de alguien de la familia.
- **A quien administra no se le invita nunca**, ni a quien tiene el acceso retirado, ni a quien ya tiene cuenta: con un correo robado se podría echar a los demás.
- **Todo se vuelve a comprobar al aceptar**, no solo al crear: la persona, el rol, quién invitó y si la biblioteca sigue con roles. Y si dos cuentas abren el mismo enlace a la vez, entra una sola. Anular y aceptar a la vez tampoco se pisan: anular toma los mismos bloqueos y en el mismo orden, así que gana el que llegue primero y el otro lo sabe (o se anuló, o ya entró alguien), nunca las dos cosas.
- **Un enlace no se ata a un correo**: si la app manda una dirección con un enlace para copiar, el servidor lo rechaza en vez de ignorarla, para que nadie crea que ese enlace solo lo abre esa persona.
- **La app solo da un enlace por bueno si es de su servidor.** Compara los dominios sin fijarse en mayúsculas ni en un punto final, un servidor propio nunca se queda con el dominio del servidor oficial ni con nada que cuelgue de él, y si un servidor antiguo no dice dónde están sus enlaces, solo valen los de su mismo dominio (no el de encima, que en sitios como `github.io` es de cualquiera).
- **El servidor no sirve para mandar correo a desconocidos**: pocas invitaciones por correo al día y por cuenta, tres por biblioteca a cada dirección y diez en total a una misma dirección aunque vengan de bibliotecas distintas (así otra biblioteca no puede gastarte el cupo para escribir a alguien). Solo cuentan los correos que salen de verdad. El texto del correo es fijo y **no lleva nada que escriba quien invita**, ni el nombre de la biblioteca: sale de nuestro dominio hacia direcciones ajenas, y un nombre como «tu cuenta caduca: entra en tal sitio» sería una frase nuestra. El nombre se ve en la app al abrir el enlace. Quien use alias de una dirección (`ana+1@…`) cuenta como una dirección distinta: no se adivina qué proveedor los trata como el mismo buzón.
- **Quien mira el enlace sin haber entrado** solo ve el nombre de la biblioteca (y, si no vale, por qué: caducado, usado o anulado). No ve quién más está en ella ni quién invitó.

*Construida, en el servidor y en la app. Nombres: invitación (`invitation`), por correo (`email`), enlace para copiar (`link`).*

### 9. Un registro de lo que pasó

**Problema:** alguien dice «yo no borré eso» o «me echaron sin motivo», y la biblioteca ya no está, o la cuenta tampoco. Sin un registro aparte, quien lo hizo y cuándo se va con lo borrado.

- **El servidor apunta**, en una tabla donde solo se añade, qué pasó, a qué cuenta y a qué dispositivo se debe, cuándo y desde qué dirección y qué app. Un cambio hecho en un dispositivo lo puede subir otro (una tablet que lo recibe por la red de casa): la fila apunta **a quien lo firmó** (el dispositivo que la firma prueba, y la cuenta de su persona) **y aparte a quien lo subió**, con la dirección y la app de quien lo subió. Lo apunta el servidor con lo que él sabe (la cuenta que inició sesión, su reloj, la dirección desde la que llegó la petición), **nunca con lo que dijo la app**: una app modificada no puede apuntar otra hora ni otra persona.
- **Apunta**: crear la cuenta, entrar, entrar mal (exista o no la cuenta; si no existe, sin cuenta ni correo), salir, cambiar o restablecer la contraseña, el segundo factor, borrar la cuenta o bloquearla; subir una biblioteca, cambiar un rol o el permiso de compartir, de quién es un dispositivo, retirar y devolver el acceso, y dar o quitar el poder a la nube; crear, anular y aceptar invitaciones; y lo que la nube hace sola al enlazar una cuenta con su persona.
- **Solo lo que de verdad pasó**: un cambio que el servidor rechazó, o que dejó como petición, no queda apuntado como hecho.
- **Sobrevive a lo borrado**: no depende de ninguna cuenta ni biblioteca. De una cuenta borrada se guarda su identificador y una **huella** de su correo, no el correo: sirve para contestar «¿fue esta dirección?», no para sacar la dirección.
- **Qué no guarda nunca**: contraseñas, códigos, enlaces de invitación, el correo de una invitación ni el correo de nadie. De una invitación solo el tipo (por correo o para copiar).
- **Un año, y se borra solo.** La dirección IP es un dato personal: se guarda por seguridad y con ese plazo, y la política de privacidad lo cuenta.
- **Nadie lo ve desde la app**, ni quien administra una biblioteca: no hay ninguna ruta que devuelva sus filas. Se consulta por consola, en el servidor: todavía no hay panel de administración.
- **La dirección es la de verdad** mientras el servidor sepa qué proxys son de fiar: delante de nginx hay que decírselo (ver cloud.md), o todas las filas llevarían la dirección de nginx.

*Construida en el servidor. Nombre: registro de actividad (`activity_log`).*

### 10. Entrar con Google

**Problema:** Google es la puerta de entrada de todas las cuentas que lo usen: si alguien se cuela por ahí, entra en la cuenta de otra persona. Y la vuelta de Google a la app viaja por un enlace que cualquier otra app o página puede lanzar.

- **Lo que viaja por el navegador no sirve solo.** La app empieza con un secreto que nunca sale de ella, y el servidor solo guarda su huella. El servidor devuelve un código de un solo uso (dos minutos), pero ese código **no da el token sin el secreto**: otra app que se haga pasar por la nuestra y robe el enlace de vuelta se queda con algo inútil.
- **La app solo acepta la vuelta de un intento que ella empezó y está esperando.** Si alguien empieza un intento en su móvil, entra con *su* Google y le manda a una víctima el enlace de vuelta, la app de la víctima no lo acepta (no es suyo) y ni siquiera pregunta al servidor.
- **El servidor liga la vuelta de Google al navegador que salió.** Tiene su propio `state`, que vale una vez, y una cookie del mismo navegador; una vuelta repetida, de otro navegador o inventada se rechaza sin llamar a Google.
- **Solo vale un correo que Google confirma.** Si no lo confirma, no se crea ni se enlaza nada.
- **Una cuenta que nadie confirmó pierde su contraseña.** Si alguien registró «ada@…» con una contraseña suya y nunca confirmó el código, y la verdadera Ada entra con Google, esa cuenta pasa a ser de Ada y **la contraseña que puso la otra persona deja de funcionar**.
- **Se reconoce a la persona por el identificador de Google, no por el correo**, que Google deja cambiar: si el correo de otra cuenta de Google coincide con el de una cuenta ya enlazada, no entra en ella.
- **Lo demás sigue igual:** el registro cerrado vale (fuera de la lista no nace ninguna cuenta), una cuenta bloqueada no entra, el segundo factor se pide también con Google, y el dispositivo prueba su clave como en un inicio de sesión normal.
- **Se guarda lo mínimo:** el identificador de Google, el correo y el nombre. Ningún token de Google. Ni el código, ni el secreto, ni nada de Google en el registro de actividad ni en los errores. Las dos direcciones del servidor que llevan un secreto en la dirección (`connect` y `callback`) se excluyen del registro de accesos.

*Construida en el servidor y en la app. Detalle y rutas en [accounts.md](accounts.md#continuar-con-google-71-d77). El código pide el segundo factor también al entrar con Google, mientras que D78 proponía confiar en el de Google y dejaba la duda abierta «a revisar al construirlo»: sigue por confirmar.*

## Lo que no protege

Dicho claro, para que nadie lo dé por supuesto:

- **Quien administra el servidor puede leer las bibliotecas subidas a él.** No hay cifrado de extremo a extremo. Por eso la nube es opcional, se puede instalar un servidor propio, y subir es una decisión de quien administra la biblioteca.
- **Quien puede leer una biblioteca puede copiarla.** Ningún rol lo impide.
- **Quien sepa tu correo puede bloquearte la cuenta un rato.** Si alguien falla tu contraseña cinco veces en un cuarto de hora, la cuenta responde «demasiados intentos» a todo el mundo, también a ti con la contraseña buena, hasta que pase ese cuarto de hora. No entra nadie, pero te molesta: es lo que se paga por que no se pueda adivinar una contraseña desde muchos sitios a la vez. Los intentos fallidos de un correo que no tiene cuenta se cuentan igual, para que la respuesta no diga qué direcciones tienen una. Y una familia en la misma wifi no se estorba: el límite por conexión es de 30 peticiones cada cuarto de hora y los inicios correctos no gastan el de la cuenta.
- **Un enlace para copiar lo usa quien lo abra primero**, sea quien sea. La única excepción es una cuenta a la que echaron de esa biblioteca: esa no entra con un enlace para copiar, solo con una invitación por correo dirigida a ella. Por eso solo da el rol más bajo, solo a una persona nueva, vale una vez y caduca en un día; si hace falta más seguridad, se invita por correo.
- **Un dispositivo retirado conserva lo que ya tenía**, aunque la nube ya no le entregue nada más y su app borre la biblioteca al ver la prueba. Una app modificada puede negarse a borrarla.
- **Un móvil desbloqueado en manos de otro actúa como su dueño**, hasta que alguien de la familia lo quite o se cambie la contraseña de la cuenta.
- **El registro de actividad es tan fiable como el servidor.** Quien administra el servidor puede leerlo y, con acceso a la base de datos, cambiarlo: no está firmado ni cifrado, y sirve para responder a una reclamación, no para una pericia.
- **Un familiar con una app modificada puede estorbar**: por ejemplo, impedir que un dispositivo nuevo entre en la nube. No gana acceso ni rol, y quien administra puede echarlo.
- **Emparejar un dispositivo nuevo a través de alguien que quiere engañar** (pieza 7): si la clave falsa llega al propio dispositivo de quien administra antes que la de verdad, la daría por buena. Se nota, porque el dispositivo de verdad no funciona con nadie. **Aceptado como riesgo conocido** el 25/10/2026 (D96, «Lo que queda»): hace falta alguien de dentro con una app modificada, no gana nada sin que un administrador asigne el dispositivo, y se arregla quitándole el acceso al falso.

## Cómo se comprueba

- **El caso hostil se prueba antes de escribir la defensa**, y se ve fallar: una app modificada, alguien que conoce los identificadores, un aviso falsificado.
- **Las reglas de permisos son casos compartidos**: los mismos ficheros los ejecutan la app (Rust) y el servidor (PHP).
- **Auditorías periódicas** con un método escrito, que cualquiera puede repetir: ver [`docs/audits/README.md`](../audits/README.md).
- **Revisión de seguridad de conjunto** de la nube, ahora que está construida: en curso (hay una primera pasada hecha; faltan las cuentas y el acceso a las bibliotecas completos). El modelo de amenazas, en [`docs/audits/amenazas.md`](../audits/amenazas.md), se revisa cuando cambian las decisiones de seguridad.
