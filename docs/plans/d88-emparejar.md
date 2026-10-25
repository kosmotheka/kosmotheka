# Plan: emparejar con un número de 6 cifras (D88, primera parte)

> 25/10/2026. Decisión en [D88](../DECISIONS.md). Rama `feat/pair-without-qr`, sacada de la última rama de la versión 0.7.0 para no estorbar a sus pull requests. **Es seguridad:** sus casos hostiles entran en la [batería de acceso](../architecture/bateria-de-acceso.md), cada uno con su test escrito antes y visto fallar.

**Estado (25/10/2026):** partes 1 a 3 hechas, con sus tests; falta probarlo entre el portátil y el móvil del maintainer.

**Alcance, recortado con el maintainer el 25/10/2026:** «no tocamos lo que funciona y mejoramos lo que no funciona». El QR se queda como está (la mayoría emparejará así). Lo que se arregla es el **código manual**, que hoy mide unos 350 caracteres y obliga a copiarlo, mandarlo por WhatsApp y pegarlo. La otra parte de D88, que los dispositivos de una biblioteca se encuentren solos sin emparejarse de dos en dos, queda para más adelante (abajo, con lo ya pensado).

## Cómo se ve

- **En el dispositivo que enseña el código** (el ordenador, normalmente): el QR de siempre y debajo, en grande, «¿Sin cámara? Número: **482 913**».
- **En el que se une** (el móvil): junto a «Escanear con la cámara», **«Escribir el número»**. Enseña los dispositivos de la wifi con el emparejamiento abierto, por su nombre; eliges el tuyo, escribes las 6 cifras y sigue como con el QR (de quién es, qué bibliotecas).

Como el «screencast» o una tele: el número prueba que estás viendo esa pantalla.

## Por qué hacen falta dos piezas

El código largo lleva tres cosas: **dónde** está el otro (sus direcciones), **cómo reconocerlo** (la huella de su certificado) y **el permiso de un solo uso**. Con 6 cifras no caben, así que:

1. **Dónde:** el que se une busca en su propia subred quién tiene el emparejamiento abierto. Para encontrarlo rápido, mientras la ventana está abierta el dispositivo escucha también en un **puerto fijo de emparejar**. Es lo que ya se hace para reencontrar a un dispositivo que cambió de dirección (D37), sin mDNS ni permisos nuevos.
2. **Reconocerlo y el permiso:** las 6 cifras no se mandan nunca. Los dos hacen un intercambio **SPAKE2** (un PAKE, *password-authenticated key exchange*): solo quien conoce el número llega a la misma clave, y con ella cada uno confirma al otro, el que enseña el número incluida **la huella del certificado TLS** con el que habla. Así:
   - quien se ponga en medio sin el número no consigue la clave ni puede hacerse pasar por el otro;
   - quien escuche no puede probar números en su casa: cada intento es una conversación con el dispositivo, y **a los tres fallos la ventana se cierra**;
   - acertar al azar es una entre un millón por intento.

Después de esto se fija la huella completa, como con el QR, y todo sigue igual: secreto de cliente, qué se comparte, de quién es.

## Casos hostiles (a la batería, antes que su arreglo)

1. **Número equivocado:** no empareja, y el que enseña el número no guarda nada.
2. **Tres intentos fallidos** cierran la ventana, también el correcto después.
3. **Intermediario:** un servidor con otro certificado (aunque reenvíe todo) no consigue que el que se une acepte, porque la confirmación no cuadra con la huella que ve; y el que se une no le manda nada.
4. **Un solo uso:** con la ventana ya usada, caducada o cerrada, el número no vale.
5. **Antes del número no sale nada:** buscar dispositivos solo dice quién tiene la ventana abierta y su nombre; ni bibliotecas ni ids de biblioteca.
6. El emparejamiento por número **no ocupa el sitio** de un dispositivo ya emparejado, como hoy con el QR (`a_paired_device_cannot_pair_again_until_forgotten`).

## Orden de trabajo

| Parte | Qué | Prueba |
|---|---|---|
| 1 · El intercambio | SPAKE2 con el número, confirmaciones (con la huella), tres intentos por ventana, en el servidor y en el cliente | Casos 1 a 4 y 6 con TLS de verdad, como los de `lan/client.rs` |
| 2 · Encontrarse | El puerto fijo de emparejar mientras la ventana está abierta y la búsqueda en la subred | Caso 5; prueba con el portátil y el móvil |
| 3 · Pantallas | El número bajo el QR; «Escribir el número» con la lista y las 6 cifras | Capturas y prueba en el móvil |
| 4 · Cerrar | Guía, arquitectura (`sync.md`), D13 y D88, novedades | |

**Dependencias nuevas:** `spake2` (RustCrypto, MIT o Apache-2.0) y, si no está ya, `hmac`, de la misma familia que `sha2`.

**Límite, el mismo que con el QR:** en las wifi que aíslan los dispositivos entre sí (algunas de invitados) no se encuentran.

## Para más adelante: reconocerse por biblioteca

La otra parte de D88: los dispositivos de una biblioteca se encuentran solos en casa y se sincronizan sin haberse emparejado entre ellos. Pensado el 25/10/2026 y aparcado con el maintainer (no hace falta para lo de arriba):

- **Quién entra:** un dispositivo de esa biblioteca con su persona asignada, su clave avalada (D96) y sin retirada (D71). Una valla sola (D84) no lo excluye: también se le pone a quien bajan de editor a lector, que sigue recibiendo la biblioteca; lo que corta el acceso (quitarle la persona o echarla, D98) ya lo deja «de nadie».
- **Prueba mutua:** cada lado firma con su clave de dispositivo (D56) el reto del otro y la huella del certificado de la conexión; solo después se habla de bibliotecas, y viajan las que los dos comparten con confianza.
- **Encontrarse:** mDNS con un identificador aleatorio que cambia (decidido: `mdns-sd`), las últimas direcciones como respaldo.
- **Las casillas de «Compartir y dispositivos»** pasan a ser la puerta: dar a un dispositivo una biblioteca que aún no tiene; entre los que ya la tienen, todo es automático (decidido el 25/10/2026).
- **Modelo de amenazas y casos hostiles:** el curioso de una red ajena, el que conoce ids, el retirado que no se ha enterado, el impostor que escucha y quiere que le manden la biblioteca, y el que repite una conexión grabada.
