# Política de privacidad (borrador para la web y Google Play)

> **VIGENTE desde el 17/10/2026: [politica-privacidad-con-servidor.md](politica-privacidad-con-servidor.md)**, ya en la web (`web/es/privacidad.html` y `web/en/privacy.html`). Este fichero es la política anterior, para la app sin servidor, y se conserva como historia: **no se edita ni se publica**. Lo que se sube a la web sale del otro fichero (sin su cabecera de borrador, sin el apartado de Play Console y sin «Lo que queda fuera»); si cambia uno, cambia el otro. La web la sube el maintainer.

> Redactada el 29/09/2026 para la primera publicación en Google Play (prueba cerrada). Describe la app **sin servidor**. Irá en HTML en la web del proyecto, junto a una página provisional de «próximamente».
>
> **Antes de que llegue el inicio de sesión en el servidor (nube opcional)** hay que ampliarla: con cuenta se guardarán en el servidor nombre, apellidos y correo, además del catálogo de las bibliotecas que se suban. Habrá que explicar qué se guarda, dónde, para qué, cuánto tiempo, cómo borrar la cuenta y los derechos del RGPD (acceso, rectificación, supresión, portabilidad), y el responsable del tratamiento. También cambia el formulario de seguridad de los datos de Play Console: pasará a «recoge datos» (nombre, correo, contenido del usuario).

> **Para la ampliación con servidor, sobre el registro de actividad (7.7, D66)**: el servidor guarda, durante **un año** y después lo borra solo, un registro de lo que pasa con una cuenta y sus bibliotecas: crear la cuenta, iniciar o cerrar sesión (y los intentos fallidos, que si no hay cuenta con esa dirección se apuntan sin la dirección), cambiar la contraseña, activar o quitar el segundo factor, borrar la cuenta; y, en las bibliotecas subidas, subirlas, cambiar un rol o permiso, retirar o devolver el acceso a una persona o dispositivo, y crear, anular o aceptar invitaciones. De cada suceso guarda la **fecha y hora del servidor**, la cuenta y el dispositivo (cuando un cambio lo sube un dispositivo distinto del que lo hizo, los dos), la **dirección IP** y el nombre de la app y su versión de quien lo envía, y el nombre de la biblioteca de entonces. **No guarda** contraseñas, códigos, enlaces de invitación ni correos: de una cuenta borrada conserva solo su identificador y una huella irreversible de su correo, que permite comprobar «¿fue esta dirección?» pero no averiguarla. Sirve para responder a una reclamación («¿quién borró esta biblioteca?») y para la seguridad del servicio; solo lo ve quien administra el servidor. Base: interés legítimo en la seguridad del servicio. Se cuenta con el plazo y la IP como dato personal. Esto **no** está en el texto de abajo, que describe la app sin servidor.

> **Para la ampliación con servidor, sobre «Continuar con Google» (7.1, D77)**: quien entre con Google (opcional; también se puede con correo y contraseña) hace el inicio de sesión **en la página de Google, en el navegador**, no dentro de la app; Kosmotheka nunca ve la contraseña de Google. De Google, el servidor recibe **solo tres datos**: el identificador que Google da a esa cuenta, el correo (y si Google lo ha confirmado) y el nombre. Los pide con los permisos básicos (`openid`, `email`, `profile`) y **no accede a nada más** de la cuenta de Google (ni correo, ni contactos, ni Drive). Guarda el identificador, el correo y el nombre como datos de la cuenta; **no guarda ningún token de Google**. Un intento de entrar con Google deja en el servidor, diez minutos como máximo, huellas de secretos propios (no datos de Google) y se borra solo. El registro de actividad anota que se entró o se creó la cuenta con Google, sin ningún dato de Google. Google, por su parte, sabe que esa cuenta entró en Kosmotheka y se rige por su política de privacidad.

---

Política de privacidad de Kosmotheka

Última actualización: 2 de octubre de 2026

Kosmotheka es una aplicación para catalogar lo que tiene una familia o un grupo: libros, cómics, juegos, películas, series, videojuegos, música y más. Está pensada para funcionar sin cuentas y sin servidores: tus datos son tuyos y se quedan en tus dispositivos.

## Qué datos guarda la aplicación

- Tu catálogo: las obras, sus ejemplares, dónde están y de quién son, y lo que cada persona apunta (estado, valoración, favoritos).
- Las personas de cada biblioteca y sus permisos, si los usas.
- Los nombres de los dispositivos con los que compartes una biblioteca.

Todo se guarda solo en el almacenamiento de tu dispositivo. No tenemos servidores que reciban estos datos, no hay cuentas de usuario y no los vemos nunca.

## Compartir entre tus dispositivos

Si emparejas dos dispositivos (con un código QR), se sincronizan directamente entre ellos a través de tu red local, cifrados. Los datos no pasan por ningún servidor nuestro ni de terceros.

## Búsquedas en catálogos externos

Para rellenar los datos de una obra, la aplicación puede consultar catálogos públicos: Biblioteca Nacional de España, Library of Congress (Estados Unidos), Open Library, Inventaire, Wikidata, MusicBrainz y Cover Art Archive, y Google Books o GameUPC si pones tu propia clave en Ajustes. En esas consultas solo se envía lo que buscas (un ISBN, un código de barras o un título), nunca tu catálogo ni datos personales. Cada servicio tiene su propia política de privacidad. Estas búsquedas son opcionales: la aplicación funciona sin ellas.

## Cámara

La cámara se usa solo para leer códigos de barras cuando tú lo pides. Las imágenes se procesan en el dispositivo y no se guardan ni se envían.

## Copias de seguridad y exportaciones

Las copias de seguridad y los ficheros CSV que creas se guardan donde tú elijas. Si los guardas en un servicio en la nube, se aplica la política de ese servicio.

## Lo que no hacemos

No hay publicidad, ni analítica, ni seguimiento, ni venta de datos. La aplicación no recoge datos para nosotros.

## Menores

La aplicación no pide ni recoge datos personales, por lo que puede usarla cualquier miembro de la familia.

## Borrar tus datos

Como todo está en tu dispositivo, borrar una biblioteca o desinstalar la aplicación elimina sus datos de ese dispositivo. Las copias que hayas hecho siguen donde las guardaste.

## Cambios

Si la aplicación cambia lo que hace con los datos (por ejemplo, cuando exista la sincronización opcional con un servidor), actualizaremos esta política antes de que el cambio llegue a la aplicación.

## Contacto

[privacy@kosmotheka.com](mailto:privacy@kosmotheka.com)
