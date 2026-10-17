# Política de privacidad, ampliada para la cuenta y la nube (BORRADOR)

> Borrador del 11/10/2026, puesto al día el 17/10/2026 (entrar con Google, ML Kit, lo que ve cada servicio, borrar la cuenta según D99, claves propias de TMDB y RAWG, BoardGameGeek), escrito por la sesión coordinadora a partir de lo que el servidor guarda **hoy** (hitos 7.1 a 7.7). **No está publicado ni revisado.** Sustituirá a [politica-privacidad.md](politica-privacidad.md) y a las dos páginas de la web cuando el maintainer lo dé por bueno, **antes de publicar una versión de la app que enseñe la cuenta**. Lo marcado **[POR CONFIRMAR]** son datos que solo sabe el maintainer. No es asesoramiento legal.

---

Política de privacidad de Kosmotheka

Última actualización: [fecha de publicación]

Kosmotheka es una aplicación para catalogar lo que tiene una familia o un grupo: libros, cómics, juegos, películas, series, videojuegos, música y más. **Funciona entera sin cuenta y sin servidor**: en ese caso tus datos se quedan en tus dispositivos y no los vemos nunca. La cuenta y la nube son opcionales, y esta política explica qué cambia si las usas.

## Quién es el responsable

Responsable: Sergio Rebollo, España, que mantiene Kosmotheka como proyecto personal. Contacto: privacy@kosmotheka.com

## Si usas la app sin cuenta

- Tu catálogo, las personas de cada biblioteca y los nombres de tus dispositivos se guardan **solo en tu dispositivo**.
- Si emparejas dos dispositivos con un código QR, se sincronizan directamente entre ellos por tu red local, cifrados. No pasa nada por un servidor nuestro.
- Para rellenar los datos de una obra, la app puede consultar catálogos públicos (Biblioteca Nacional de España, Library of Congress (Estados Unidos), Open Library, Inventaire, Wikidata, MusicBrainz y Cover Art Archive, y Google Books o GameUPC si pones tu propia clave). Solo se envía lo que buscas: un ISBN, un código de barras o un título. Como en cualquier conexión a internet, cada servicio ve además desde qué dirección IP le preguntas y el nombre y la versión de la app; lo mismo al descargar las portadas, que vienen de esos servicios.
- El escáner de códigos de barras usa **ML Kit**, de Google, a través de los servicios de Google Play del móvil. Se rige por sus términos ([ML Kit](https://developers.google.com/ml-kit/terms)). Para medir y mejorar su funcionamiento, ML Kit envía a Google datos técnicos de uso (el modelo del dispositivo, la versión del sistema y de la app, los tiempos y los errores del escáner), sin las imágenes. Nosotros no recibimos nada de esto.
- La cámara se usa solo para leer códigos de barras, cuando tú lo pides. Las imágenes no se guardan ni se envían.

## Si creas una cuenta

La cuenta sirve para tener una biblioteca en varios dispositivos sin que estén en la misma red, y para compartirla con otras personas. Al crearla, nuestro servidor guarda:

- **Tu correo y tu nombre**, y tu contraseña de forma que no se puede leer (solo una huella calculada con un algoritmo pensado para contraseñas).
- **Tus dispositivos:** el nombre que le hayas puesto a cada uno, un identificador y su clave pública, y cuándo se usó por última vez.
- **Si entras con Google:** Google nos dice tu correo, si está verificado, tu nombre y un identificador tuyo. Del identificador nos quedamos para reconocerte la próxima vez; nada más de Google. Google sabe que usas Kosmotheka, como con cualquier inicio de sesión con Google.
- **Si activas la verificación en dos pasos:** el secreto que comparte tu app autenticadora, y tus códigos de reserva de forma que no se pueden leer.

## Si subes una biblioteca a la nube

Subir una biblioteca es una decisión aparte de crear la cuenta, y la toma quien administra esa biblioteca. Al hacerlo, el servidor guarda **el contenido de esa biblioteca**: las obras, los ejemplares, dónde están y de quién son, las personas de la biblioteca con su nombre y su papel, lo que cada una apunta (estado, valoración, favoritos, deseos), y el historial de cambios con el dispositivo que hizo cada uno.

**Lo que conviene saber:** estos datos viajan cifrados hasta el servidor, pero **en el servidor no están cifrados de extremo a extremo**. Quien administra el servidor podría técnicamente leerlos. No lo hacemos, salvo que sea imprescindible para resolver una incidencia que tú nos pidas resolver. Si prefieres que nadie más pueda leerlos, usa la app sin nube: la sincronización entre tus dispositivos por la red local no pasa por ningún servidor.

Las bibliotecas que no subes no salen de tus dispositivos, tengas cuenta o no.

## Si buscas un libro con la cuenta

Si no has puesto una clave tuya de Google Books, y tu servidor la ofrece, la app puede preguntar al servidor por un ISBN en lugar de preguntar directamente a Google Books (que falla a menudo para quien no tiene clave). **Se envía el ISBN y nada más**, junto con tu sesión, que el servidor necesita para comprobar que tienes cuenta. El servidor lo busca en Google Books con su propia clave, así que **Google ve las búsquedas como del servidor, no tuyas**. El servidor guarda la respuesta unos días (30 si encontró el libro, 1 si no) para no repetir la pregunta, **sin guardar quién la hizo** ni apuntarlo en ningún registro; solo cuenta cuántas búsquedas haces para ponerte un límite (600 por hora) y no lo guarda más allá de esa hora. Puedes apagarlo en Ajustes › Proveedores de datos (viene encendido) o poner tu propia clave de Google Books, que tiene preferencia: entonces el servidor ni se entera. *Pendiente para el texto final:* el maintainer comprueba en producción que el registro de accesos de nginx no apunta las rutas `/v1/metadata/` (la regla se puso el 11/10/2026; cree que sí funciona, pero hay que verlo).

## Si buscas una película o una serie con la cuenta

Si tu servidor lo ofrece y no lo has apagado en Ajustes › Proveedores de datos, al buscar una película o una serie por su título la app **envía a tu servidor lo que escribes, si es película o serie y el idioma de la app**, junto con tu sesión. El servidor lo busca en TMDB con su propia clave, así que **TMDB ve la búsqueda como del servidor, no tuya**. Lo que el servidor guarda es la respuesta, unos días (como mucho 30; la búsqueda, un día), **sin guardar quién la pidió ni el texto que escribiste**: la búsqueda se guarda bajo una huella que no permite recuperar el texto. Ni el texto ni su autor se apuntan en ningún registro. Solo se cuenta cuántas búsquedas haces para ponerte un límite (600 por hora, junto con las de libros). Si pones tu propia clave de TMDB en Ajustes, la app le pregunta directamente y el servidor ni se entera. Esta aplicación usa TMDB y las API de TMDB, pero TMDB no la respalda, certifica ni aprueba de ningún otro modo.

## Si buscas un videojuego con la cuenta

Si tu servidor lo ofrece y no lo has apagado en Ajustes › Proveedores de datos, al buscar un videojuego por su título la app **envía a tu servidor lo que escribes**, junto con tu sesión. El servidor lo busca en RAWG con su propia clave, así que **RAWG ve la búsqueda como del servidor, no tuya**. El servidor guarda la respuesta unos días (hasta 30; la búsqueda, 7), **sin guardar quién la pidió ni el texto que escribiste**: la búsqueda se guarda bajo una huella que no permite recuperar el texto. Ni el texto ni su autor se apuntan en ningún registro. Solo se cuenta cuántas búsquedas haces para ponerte un límite (600 por hora, junto con las de libros y películas). Si pones tu propia clave de RAWG, la app le pregunta directamente. Los datos de videojuegos vienen de [RAWG](https://rawg.io).

## Si buscas un cómic con la cuenta

Si tu servidor lo ofrece y no has puesto una clave tuya de Comic Vine ni lo has apagado en Ajustes › Proveedores de datos, al buscar un cómic por su título la app **envía a tu servidor lo que escribes**, junto con tu sesión. El servidor lo busca en Comic Vine con su propia clave, así que **Comic Vine ve la búsqueda como del servidor, no tuya**. El servidor guarda la respuesta unos días (hasta 30; la búsqueda, 7), **sin guardar quién la pidió ni el texto que escribiste**: la búsqueda se guarda bajo una huella que no permite recuperar el texto. Ni el texto ni su autor se apuntan en ningún registro. Solo se cuenta cuántas búsquedas haces para ponerte un límite (600 por hora, junto con las de libros, películas y videojuegos). Si pones tu propia clave, la app pregunta directamente a Comic Vine y el servidor ni se entera. Los datos de cómics vienen de [Comic Vine](https://comicvine.gamespot.com).

## Si buscas un juego de mesa o un libro de rol con la cuenta

Si tu servidor lo ofrece y no lo has apagado en Ajustes › Proveedores de datos, al buscar un juego de mesa por su título (y, al buscar un libro por su título, los libros de rol) la app **envía a tu servidor lo que escribes**, junto con tu sesión y si es un juego o un libro de rol. El servidor lo busca en BoardGameGeek con su propio token, así que **BoardGameGeek ve la búsqueda como del servidor, no tuya**. El servidor guarda la respuesta unos días (hasta 30; la búsqueda, 7), **sin guardar quién la pidió ni el texto que escribiste**: la búsqueda se guarda bajo una huella que no permite recuperar el texto. No se envía nada más. BoardGameGeek no tiene relación con esta aplicación.

## Si invitas a alguien

Si invitas por correo, el servidor envía un mensaje a esa dirección. **No guardamos la dirección**: solo una huella que permite comprobar que quien acepta es quien invitaste. El mensaje no incluye tu nombre ni el de la biblioteca.

## El registro de actividad

Para poder responder de lo que pasa con una cuenta o una biblioteca (por ejemplo, quién quitó a quién, o desde dónde se inició una sesión), el servidor apunta: crear la cuenta, iniciar y cerrar sesión, los intentos fallidos, cambiar la contraseña, la verificación en dos pasos, borrar la cuenta; y, en las bibliotecas de la nube, subirlas, cambiar permisos, quitar o devolver el acceso a una persona o a un dispositivo, y las invitaciones.

De cada cosa se guarda **qué fue, cuándo, la cuenta y el dispositivo, la dirección IP y la versión de la app**. No se guardan contraseñas, códigos ni el contenido del catálogo. **Se conserva un año** y después se borra solo. La base legal es nuestro interés legítimo en mantener el servicio seguro.

## Quién más trata tus datos

- **Alojamiento:** el servidor está en DigitalOcean, en su centro de datos de Ámsterdam (Países Bajos, Unión Europea) [A COMPROBAR por el maintainer: Ámsterdam o Fráncfort].
- **Correo:** los mensajes (códigos de confirmación, recuperación de contraseña, invitaciones) se envían a través de Brevo (Sendinblue SAS, Francia), que recibe la dirección de destino y el contenido del mensaje.
- No hay nadie más: ni publicidad, ni analítica nuestra, ni seguimiento, ni venta de datos. Lo único que va a Google son los datos técnicos de ML Kit al escanear (arriba).

## Cuánto tiempo se guardan

- **Una cuenta que no confirma su correo** se borra a las 24 horas.
- **Los códigos** que enviamos por correo caducan en minutos y se borran.
- **Las invitaciones** valen 24 horas y se borran una semana después de caducar.
- **El registro de actividad**, un año.
- **Tu cuenta y tus bibliotecas en la nube**, hasta que las borres.

## Borrar tus datos

- **Sin cuenta:** borrar una biblioteca o desinstalar la app elimina sus datos de ese dispositivo.
- **Borrar la cuenta:** desde la app, en la pantalla de tu cuenta. Se borran la cuenta, sus dispositivos y **las bibliotecas que subiste tú a la nube** que solo usabas tú. **Las que compartes con otras personas siguen en la nube para ellas**: pasan a otra cuenta que tiene acceso, y lo que apuntaste en ellas sigue ahí, porque es parte de esa biblioteca. Lo que tengas en tus dispositivos no se toca. Las entradas del registro de actividad se conservan hasta cumplir su año, sin tu correo.
- **Si no puedes entrar en la app,** escríbenos a privacy@kosmotheka.com desde el correo de la cuenta; la borramos en 30 días como mucho. Los pasos, en [kosmotheka.com/es/borrar-cuenta.html](https://kosmotheka.com/es/borrar-cuenta.html).

## Tus derechos

Puedes pedirnos acceder a tus datos, corregirlos, borrarlos, limitar su uso u oponerte a él, y recibirlos en un formato que puedas llevarte (la app ya exporta tu catálogo a CSV y hace copias de seguridad completas). Escríbenos a privacy@kosmotheka.com. Si crees que no hemos atendido bien tu petición, puedes reclamar ante la autoridad de protección de datos de tu país (en España, la AEPD).

## Menores

La app puede usarla cualquier miembro de la familia sin cuenta. **Crear una cuenta** está pensado para personas adultas, a partir de **14 años** (la edad a la que en España se puede dar el consentimiento propio). Un menor puede estar en una biblioteca familiar sin tener cuenta: lo suyo llega a la nube a través de quien administra la biblioteca.

## Cambios

Si cambia lo que hacemos con los datos, actualizaremos esta política antes de que el cambio llegue a la app, y cambiaremos la fecha de arriba.

## Contacto

privacy@kosmotheka.com

---

## Para el formulario «Seguridad de los datos» de Play Console

Con esta versión la app **recoge datos** (solo si se crea una cuenta) y **no los comparte** con terceros para sus propios fines (Brevo y el alojamiento son encargados del tratamiento). A declarar: correo electrónico y nombre (gestión de la cuenta), y contenido generado por el usuario (el catálogo de las bibliotecas subidas; funcionalidad de la app). Cifrado en tránsito: sí. Se puede pedir el borrado: sí, desde la app y por correo. La recogida es opcional.

## Lo que queda fuera de este borrador

- Las notificaciones con la app cerrada (Firebase): cuando existan.
