# Plan: emparejar una vez y el código corto (D88)

> 25/10/2026. Decisión en [D88](../DECISIONS.md) (aceptada en la noche del 24 al 25/10/2026). Rama `feat/pair-without-qr`, sacada de la última rama de la versión 0.7.0 para no estorbar a sus pull requests. **Es seguridad pura:** cada regla nueva sobre quién recibe una biblioteca entra en la [batería de acceso](../architecture/bateria-de-acceso.md) con su test, escrito antes y visto fallar.

**Estado:** plan escrito; falta que el maintainer lo apruebe y resuelva las tres preguntas del final.

## Lo que pasa hoy

- **Emparejar es de dos en dos (D13, D34).** Un dispositivo abre una ventana de 10 minutos y enseña un código de unos 350 caracteres (QR o texto) con su id, sus direcciones, la huella de su certificado TLS y un token de un solo uso. El otro se conecta con ese certificado fijado, gasta el token y recibe un secreto de cliente. Desde entonces uno hace de servidor y el otro de cliente.
- **Qué viaja lo marca cada pareja** (`share_library`, «Marca qué va a cada uno»).
- **Si el otro cambia de dirección**, se le busca en la propia subred, en los puertos que usó (D37), y solo vale quien presenta el certificado fijado y el mismo id.
- **Cada dispositivo tiene su clave** (D56), registrada en cada biblioteca, y la biblioteca sabe de quién es (D96), si está retirado (D71) y si tiene valla (D84, D87). Es lo que decide si sus operaciones cuentan.

Los dos dolores: el código no se puede teclear en un móvil, y con cuatro dispositivos hay que emparejar varias veces o dejar una cadena que se corta cuando el que hace de puente se va de casa.

## Lo decidido (resumen de D88)

1. **Quién entra:** solo un dispositivo de esa biblioteca con su persona asignada y su clave avalada (D96), que no esté retirado (D71) ni con valla (D84, D87). La misma regla que decide si sus operaciones cuentan.
2. **Autenticación sin secreto compartido:** cada uno firma un reto con su clave de dispositivo (D56) y el otro la compara con la registrada en la biblioteca.
3. **Encontrarse:** anuncio por mDNS con un identificador aleatorio que cambia, nunca el nombre del dispositivo ni el de la biblioteca. Si no llega (redes de invitados), las últimas direcciones conocidas; si tampoco, el QR.
4. **Retirado o con valla:** quien contesta lo comprueba en cada conexión, aunque el otro no se haya enterado.
5. **Inicia cualquiera**; escucha quien tenga la app abierta.
6. **La primera vez:** el QR, o un **número de 6 cifras comparado en las dos pantallas**, como con Bluetooth.

## Modelo de amenazas

Quién puede estar en la red de casa o en una red ajena (un hotel, la wifi de un club):

- **El curioso:** escucha los anuncios y se conecta a lo que encuentra. No debe saber qué bibliotecas hay ni de quién son; como mucho, que hay una app de Kosmotheka.
- **El que conoce ids:** sabe el id de un dispositivo de la familia y el de la biblioteca (los ids viajan en operaciones, en copias de seguridad…). Sin la clave privada de ese dispositivo no entra.
- **El retirado o echado:** tuvo acceso legítimo y aún tiene la clave. Debe quedar fuera aunque su app no se haya enterado.
- **El impostor que escucha:** finge ser un dispositivo de la familia para que otro le **mande** la biblioteca. Por eso la comprobación va en los dos sentidos: nadie envía nada a quien no ha probado ser de la biblioteca.
- **El intermediario en el emparejamiento por código corto:** se pone en medio de los dos para quedarse con el emparejamiento. Lo para la comparación de las 6 cifras con compromiso previo (abajo).
- **El que repite:** graba una conexión buena y la repite. Lo para un reto nuevo en cada conexión, ligado a esa sesión TLS.

Fuera de alcance: alguien con el móvil desbloqueado en la mano (ya lo cubre el bloqueo de la app, D78) y la red de casa comprometida a nivel del router más allá de lo anterior.

## Cómo funciona (propuesta)

### A. Reconocerse por biblioteca

1. **Anuncio (mDNS):** `_kosmotheka._tcp` con un identificador aleatorio que cambia en cada arranque y cada cierto tiempo. Nada más en el anuncio.
2. **Conexión TLS sin fijar** (aún no hay huella que fijar) y **prueba mutua:** cada lado manda un reto aleatorio; cada uno responde firmando con su clave de dispositivo `reto del otro + huella del certificado TLS del servidor + su id`. Ligar la firma al certificado impide que un intermediario la reenvíe por otra conexión.
3. **Solo después de la prueba** se habla de bibliotecas: cada lado calcula, con lo que **él** sabe, las bibliotecas en las que el otro es de confianza (punto 1 de lo decidido) y en las que él también está. Esas, y solo esas, se sincronizan. Antes de la prueba no sale ningún id de biblioteca ni nombre.
4. Lo que hoy es «servidor» y «cliente» se queda: escucha quien tiene la app abierta y se conecta cualquiera de los dos.

### B. El código corto, la primera vez

Para un dispositivo que aún no está en ninguna biblioteca común (el móvil nuevo de casa):

1. En el que ya tiene la biblioteca, «Emparejar un dispositivo» abre la ventana de siempre y anuncia que está abierta.
2. En el nuevo, «Emparejar sin cámara» busca dispositivos con la ventana abierta y enseña sus nombres (solo los de ventana abierta, y solo durante esos 10 minutos).
3. Al elegir uno se hace un **intercambio con compromiso**: el que tiene la ventana manda primero un compromiso (el hash de su número aleatorio y su clave), el nuevo manda su número, y entonces el primero revela el suyo. Con las dos claves y los dos números, los dos calculan el mismo **número de 6 cifras**. Un intermediario tendría que acertar a la primera, con una probabilidad de uno entre un millón, porque se comprometió antes de ver el número del otro.
4. La persona comprueba que el número es el mismo en las dos pantallas y pulsa «Coinciden» en las dos. A partir de ahí sigue como con el QR: de quién es el dispositivo, qué bibliotecas, con qué rol.

## Casos hostiles (a la batería, antes que su arreglo)

Reconocerse por biblioteca:

1. Un dispositivo del registro **sin persona asignada** no recibe nada.
2. Uno con **clave avalada que no es la suya** (D96) no recibe nada.
3. Uno **retirado** (D71), aunque no lo sepa, no recibe nada y lo que manda no entra.
4. Uno con **valla** o de una persona **echada** (D98): lo mismo.
5. Quien **conoce el id** de un dispositivo de la familia pero firma con otra clave: fuera.
6. **Repetir** una respuesta grabada de otra conexión: fuera (el reto es nuevo y va ligado al certificado).
7. **El impostor que escucha:** quien presenta un id de la familia sin su clave no recibe ni un byte de la biblioteca, aunque sea él quien escucha.
8. Solo viajan las bibliotecas en las que **los dos** son de confianza: una que solo tiene uno de ellos, o en la que el otro está retirado, no sale.
9. **El curioso:** el anuncio no lleva nombres ni ids, el identificador cambia, y antes de la prueba la conexión no dice qué bibliotecas hay.

Código corto:

10. Sin pulsar «Coinciden» en los dos, no hay emparejamiento.
11. Un **intermediario** que cambia una clave o un número hace que las 6 cifras no coincidan; y un compromiso que no cuadra con lo revelado corta el emparejamiento.
12. Con la ventana cerrada o caducada no se puede, y cada ventana empareja un solo dispositivo, como hoy.

## Orden de trabajo

| Parte | Qué | Prueba |
|---|---|---|
| 1 · Prueba mutua | El intercambio de retos y firmas sobre TLS, y la regla de confianza por biblioteca, sin descubrimiento todavía (dirección escrita a mano en los tests) | Casos 1 a 8 con TLS de verdad, como los de `lan/client.rs` |
| 2 · Sincronizar por biblioteca | Dos dispositivos de la misma biblioteca, sin haberse emparejado, se sincronizan solo lo común | `assert_converged` y los casos 3, 4 y 8 de punta a punta |
| 3 · Encontrarse | mDNS con identificador que cambia; el permiso de multicast en Android y en la snap; las últimas direcciones como respaldo | Caso 9; prueba en casa con el móvil y el ordenador |
| 4 · Código corto | El intercambio con compromiso, las 6 cifras y las pantallas | Casos 10 a 12 |
| 5 · Cerrar | «Compartir y dispositivos» enseña quién está en casa; guía, privacidad, arquitectura (`sync.md`), D13 actualizada, novedades | Prueba en el móvil |

Calculo varios días. Cada parte va en su commit, en esta rama, y se puede parar entre parte y parte.

## Preguntas para el maintainer

1. **Las casillas de «Compartir y dispositivos».** Con D88, dos dispositivos de una biblioteca se sincronizan solos, así que «marca qué va a cada uno» deja de tener sentido entre ellos. Propuesta: las casillas siguen siendo **la puerta** (dar a un dispositivo una biblioteca que aún no tiene, con el QR o el código), y entre los que ya la tienen la sincronización es automática.
2. **Una dependencia nueva:** `mdns-sd`, en Rust puro, con licencia MIT o Apache-2.0, para anunciarse y encontrarse. Hacerlo a mano sería más código delicado que mantener.
3. **El orden:** propuesta, reconocerse por biblioteca primero (partes 1 a 3), que es lo que quita emparejar varias veces, y el código corto después (parte 4). Si te molesta más teclear el código, se puede invertir.
