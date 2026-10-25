---
title: "Seguridad: el modelo de amenazas"
description: De quién protege Kosmotheka, qué protege cada pieza y, sobre todo, lo que no protege.
sidebar:
  order: 8
---

Esta página describe **el modelo**: de quién nos protegemos, con qué piezas y con qué límites. No lista fallos concretos; para avisar de uno, mira [Avisar de un fallo de seguridad](../report-a-vulnerability/). Para entender las piezas conviene haber leído antes [El motor de sincronización](../sync-engine/). Los nombres que usa el código están en el [glosario](../glossary/).

## La idea, en cuatro frases

1. Tus datos viven en tus dispositivos. La app funciona entera sin servidor y sin cuenta.
2. Una biblioteca solo llega a quien alguien de la familia se la ha dado: con un código QR en casa, o por la nube si quien la administra decidió subirla.
3. Dentro de una biblioteca hay roles (quien administra, quien edita, quien solo lee). **Cada dispositivo comprueba por su cuenta** que los demás los respetan; no hace falta un servidor que vigile.
4. Nada se pierde: lo que alguien intenta sin tener permiso no se borra, queda como una **petición** que quien administra acepta o rechaza.

## De quién nos protegemos

Hay dos fronteras, y conviene no mezclarlas.

| Frontera | Ejemplo | Qué está en juego |
|---|---|---|
| **Los de fuera** | Un vecino en la misma wifi, alguien en internet, quien encuentra un móvil | Que lean tu biblioteca: qué tienes, dónde está, de quién es |
| **Los de dentro** | Un familiar que solo lee e instala una versión modificada de la app para saltarse su rol | Que cambie o borre el catálogo, o se haga administrador. Leer, ya podía |

La primera es la más importante. La segunda es la que más mecanismos necesita, porque quien ataca **ya tiene una copia de la biblioteca** y un dispositivo que los demás conocen.

Con más detalle, estos son los atacantes que consideramos:

- **Un lector** con su dispositivo y una copia completa: quiere cambiar o borrar el catálogo, tocar lo de otros o compartir sin permiso.
- **Un editor:** quiere hacerse administrador, borrar lo que no creó o vincular dispositivos.
- **Un antiguo miembro, o un dispositivo al que se retiró el acceso,** con su copia y su clave: quiere seguir leyendo o escribiendo.
- **Un equipo en la misma red,** nunca emparejado: quiere leer el tráfico, suplantar a alguien o emparejarse con un QR visto de lejos.
- **Un móvil robado,** bloqueado o desbloqueado.
- **Alguien en internet** contra un servidor: tomar cuentas, leer bibliotecas ajenas o abusar del correo.
- **Un servicio de datos hostil** (el que nos devuelve los datos de un libro): inyectar contenido o rastrear con imágenes.
- **Una dependencia maliciosa** (un paquete que usamos).
- **Quien administra el servidor:** puede leer y editar lo que se sube a él.

Y las fronteras de confianza donde se aplican las defensas: entre la interfaz y el núcleo de la app, entre dos dispositivos, entre un dispositivo y el servidor, entre la app y los servicios de datos, entre el servidor y su base de datos, y entre el repositorio y la integración continua.

## Lo que protege cada pieza

Cada pieza dice qué problema resuelve, con un ejemplo.

### 1. Todo cambio va firmado

**Problema:** que alguien haga un cambio haciéndose pasar por el móvil de otra persona.

Cada dispositivo tiene una clave secreta que **nunca sale de él**, y firma con ella cada operación. Los demás conocen su clave pública y comprueban la firma. Una operación sin firma, o con una firma que no cuadra, se descarta. La clave de un dispositivo no cambia nunca: si llega otra distinta para el mismo dispositivo, se rechaza.

### 2. Cada dispositivo comprueba los permisos

**Problema:** que un lector edite el catálogo con una app modificada.

Cuando llega una operación, cada dispositivo mira quién la firmó, de qué persona es ese dispositivo y qué rol tenía esa persona **en ese momento**. Si no podía hacerlo, no se aplica: queda como petición. Todos los dispositivos, y también el servidor, siguen las mismas reglas y llegan al mismo resultado, reciban los cambios en el orden que sea. Las reglas están escritas una sola vez, con [casos de prueba](../operations-spec/) que pasan igual la app y el servidor.

### 3. No se puede colar un cambio «con fecha de ayer»

**Problema:** a Hugo le quitan hoy el rol de editor. Con una app modificada, escribe un cambio y le pone fecha de la semana pasada, cuando aún era editor.

Tres defensas:

- **Cada dispositivo numera lo suyo.** Cada operación dice cuál fue la anterior de ese mismo dispositivo (`previous`), así que no se puede intercalar una nueva en medio de las antiguas sin que se note.
- **Al quitar poder se anota hasta dónde se le cree.** Quien quita el rol a un dispositivo apunta, en esa misma operación, cuál es la última operación de ese dispositivo que ha visto. Lo que aparezca después con una fecha anterior y no cuelgue de ella no se cree: queda como petición. Esa marca se llama **valla**.
- **Una biblioteca nace con su administrador.** No hay un momento en que pase de no tener roles a tenerlos, que se podría fechar hacia atrás: la primera operación es la **fundación**, y solo vale si el id de la biblioteca sale de la clave de quien la firma.

### 4. Una biblioteca solo llega a quien se le da

**Problema:** que alguien en la misma red reciba tu biblioteca.

- **En casa**, dos dispositivos se emparejan con un código QR que se enseña en pantalla. A partir de ahí hablan cifrado y cada uno reconoce al otro por su certificado, no por su dirección en la red. El código es de un solo uso y caduca a los diez minutos.
- **Solo da una biblioteca a otro dispositivo** quien la administra o quien tiene el permiso «Puede compartir».
- **Cada dispositivo elige lo que envía:** una biblioteca viaja solo si los dos lados la comparten.
- **Retirar el acceso:** quien administra puede decir que un dispositivo ya no recibe una biblioteca; los demás dejan de entregársela y no aceptan lo que firme después.
- **Echar a una persona:** quien administra puede decir que una persona ya no entra en la biblioteca. Sus dispositivos dejan de contar como suyos y se les retira el acceso, de modo que borran la biblioteca al conectar. La persona sigue en el catálogo, con su nombre y lo que apuntó. No se puede echar al último administrador.

### 5. La nube: la cuenta y la lista de acceso

**Problema:** que una cuenta lea bibliotecas de otra.

- La nube es opcional. **Solo quien administra una biblioteca puede subirla**; registrarse no sube nada.
- El servidor guarda una lista: qué cuentas pueden entrar en qué biblioteca. **Es lo único que decide el acceso.** Nunca se fía de un identificador que le mande la app: lo compara con lo que él sabe, no lo usa para buscar.
- A una biblioteca ajena, o que no existe, responde lo mismo, para que nadie averigüe cuáles existen.
- Aplica a todo lo que guarda las mismas reglas de roles que los dispositivos.
- Si echan a una persona o retiran un dispositivo, la nube **deja de entregarles la biblioteca al momento**, aunque entren desde un móvil nuevo. Al dispositivo cortado le enseña la prueba firmada de quien administra, y el dispositivo solo borra la biblioteca si esa prueba es buena: un servidor que mintiera no podría hacer perder una biblioteca.

### 6. La nube es un dispositivo más, con un solo poder

**Problema:** te compras un móvil nuevo. Nadie en casa lo conoce, así que no tiene ningún rol hasta que alguien lo empareje a mano. Y si eras la única administradora y perdiste tu único móvil, nadie puede.

Al subir una biblioteca, la nube entra en ella como un dispositivo más, con su propia clave guardada cifrada. Quien administra le da **un único poder**: decir «este dispositivo nuevo es de Laura» cuando Laura ha entrado en él con su cuenta. No puede cambiar roles, ni tocar el catálogo, ni quitarle un dispositivo a nadie. El identificador de la nube en una biblioteca **sale de su clave**, de modo que nadie puede presentarse con ese identificador y otra clave. Quien no se fíe de su servidor puede quitarle ese poder; entonces cada dispositivo nuevo se empareja a mano.

### 7. Decir de quién es un dispositivo incluye su clave

**Problema:** el identificador de un dispositivo no es secreto; lo ven todos en la biblioteca. Alguien con una app modificada podría presentar el identificador de la tablet nueva de una administradora, con una clave suya, antes de que llegue la de verdad. Si después la administradora dice «esa tablet es mía», se daría por buena la del impostor.

Por eso quien dice «este dispositivo es de Laura» apunta también **la clave que ha visto** de ese dispositivo (la **clave avalada**). Un dispositivo solo cuenta como de Laura si su clave es esa. Es la misma idea que «vincular un dispositivo» en las aplicaciones de mensajería.

### 8. Invitar a alguien de fuera

**Problema:** un enlace de invitación viaja por un chat o un correo y puede acabar en manos que no son las de esa persona.

- **Solo invita quien administra**, y el servidor lo comprueba en sus propias listas. Si quien invitó deja de administrar antes de que se acepte, la invitación ya no vale.
- **Una invitación vale una vez y 24 horas.** El servidor no guarda el enlace, solo una huella suya. El enlace es un número larguísimo al azar, y el servidor corta a quien lo intenta muchas veces.
- **Hay dos clases.** *Por correo:* solo lo abre una cuenta registrada con ese mismo correo. *Enlace para copiar:* lo abre quien llegue primero, así que **solo da el rol de lector y solo a una persona nueva**, para que un enlace filtrado no deje a un desconocido ocupar el sitio de alguien de la familia.
- **Todo se vuelve a comprobar al aceptar**, no solo al crear.
- **El servidor no sirve para mandar correo a desconocidos:** hay límites por cuenta y por dirección, y el texto del correo es fijo, sin nada que escriba quien invita.

### 9. Un registro de lo que pasó

**Problema:** alguien dice «yo no borré eso» o «me echaron sin motivo», y la biblioteca o la cuenta ya no están.

El servidor apunta, en una tabla donde solo se añade, qué pasó, a qué cuenta y dispositivo se debe, cuándo y desde qué dirección. Lo apunta **con lo que él sabe**, nunca con lo que dijo la app. No guarda contraseñas, códigos, enlaces ni correos (de una cuenta borrada, solo una huella). Se conserva un año y después se borra solo.

### 10. Entrar con Google

**Problema:** Google es la puerta de entrada de todas las cuentas que lo usen, y la vuelta a la app viaja por un enlace que cualquier otra app o página puede lanzar.

- Lo que viaja por el navegador **no sirve solo**: la app empieza con un secreto que no sale de ella, y el código que vuelve no da acceso sin ese secreto.
- **La app solo acepta la vuelta de un intento que ella empezó** y está esperando.
- El servidor liga la vuelta al navegador que salió, con un valor de un solo uso.
- Solo vale un correo que Google confirma, y se reconoce a la persona por su identificador de Google, no por el correo, que se puede cambiar.
- No se guarda ningún token de Google.

### Otras protecciones

- **Copias de seguridad:** van cifradas con contraseña por defecto, y **ninguna lleva la clave de firma, el token de la cuenta ni la identidad de la red local**.
- **Servicios de datos:** la app trata sus respuestas como no fiables. Las portadas solo se descargan por `https` de servidores conocidos y se guardan por su huella; el texto se muestra sin interpretarlo como código.
- **Límites de peticiones** en el servidor, por conexión y por cuenta, y bloqueo temporal de una cuenta tras varios intentos fallidos.

## Lo que no protege

Dicho claro, para que nadie lo dé por supuesto:

- **Quien administra el servidor puede leer las bibliotecas subidas a él.** No hay cifrado de extremo a extremo: las operaciones viajan cifradas, pero en el servidor se guardan para que pueda comprobar permisos. Por eso la nube es opcional, se puede instalar un servidor propio, y subir una biblioteca es una decisión de quien la administra. **La privacidad completa es usar la app sin cuenta.**
- **Quien puede leer una biblioteca puede copiarla.** Ningún rol lo impide.
- **Un dispositivo retirado conserva lo que ya tenía** copiado. La nube deja de entregarle nada más, y su app borra la biblioteca al ver la prueba, pero una app modificada puede negarse a borrarla.
- **Un móvil desbloqueado en manos de otro actúa como su dueño,** hasta que alguien de la familia lo quite o se cambie la contraseña de la cuenta.
- **Un familiar con una app modificada puede estorbar:** por ejemplo, intentar que un dispositivo nuevo no entre. No gana acceso ni rol, y quien administra puede echarlo.
- **Un editor con una app modificada puede bifurcar su propia cadena** (dos operaciones con la misma anterior) y que solo la reciban los dispositivos que iban por detrás. La cadena lo hace detectable, pero no se resuelve: está limitado a quien ya podía editar y a lo que ya podía hacer.
- **Emparejar a través de alguien que quiere engañar.** Al dar acceso a un dispositivo nuevo, si alguien en medio consigue que su clave llegue al dispositivo de quien administra antes que la de verdad, ese dispositivo la daría por buena. Se nota, porque el dispositivo de verdad no consigue sincronizar con nadie, y entonces se le quita el acceso al falso. Es un riesgo aceptado ([D96](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md)).
- **Un reloj muy mal puesto fecha mal sus cambios,** aunque nunca más de una hora por delante del de quien los recibe.
- **Un enlace para copiar lo usa quien lo abra primero.** Por eso solo da el rol más bajo, solo a una persona nueva y caduca en un día; si hace falta más seguridad, se invita por correo.
- **Quien sepa tu correo puede bloquearte la cuenta un rato:** cinco contraseñas fallidas en un cuarto de hora bloquean los intentos de entrada a esa cuenta, también el tuyo, hasta que pase ese tiempo. Es el precio de que nadie pueda adivinar una contraseña desde muchos sitios a la vez.
- **El registro de actividad es tan fiable como el servidor:** quien lo administra puede leerlo y, con acceso a la base de datos, cambiarlo. Sirve para responder a una reclamación, no para una pericia.
- **Un administrador único que pierde su único dispositivo pierde el control** de esa biblioteca, y nadie puede cambiar permisos. El último recurso es duplicar la biblioteca como una nueva.

## Cómo se comprueba

- **El caso hostil se prueba antes de escribir la defensa**, y se ve fallar: una app modificada, alguien que conoce los identificadores, un aviso falsificado.
- **Las reglas de permisos son casos compartidos:** los mismos archivos los ejecutan la app (Rust) y el servidor (PHP).
- **Auditorías periódicas** con un método escrito que cualquiera puede repetir: [Cómo auditamos](../audit-method/).

El modelo se revisa cuando cambian las decisiones de seguridad. Se publican el método y los hallazgos ya cerrados; los abiertos, cuando se cierran.
