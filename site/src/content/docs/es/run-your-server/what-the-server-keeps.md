---
title: Qué guarda tu servidor y qué ves tú
description: Los datos que guarda un servidor de Kosmotheka, qué puede ver quien lo administra y qué obligaciones tienes si alojas a otras personas.
sidebar:
  order: 7
---

Esta página es para decidir con los datos delante, y para saber qué le debes a quien aloje allí sus bibliotecas.

## Qué guarda

### Las cuentas

- **Correo y nombre**, y el idioma de la cuenta.
- **La contraseña**, solo como una huella calculada con un algoritmo pensado para contraseñas. Una cuenta que solo entra con Google no tiene contraseña.
- **Con Google**: el identificador que Google da a la persona (para reconocerla la próxima vez). Nada más de Google.
- **Segundo factor**, si lo activa: el secreto de su app de autenticación, **cifrado** con una clave sacada de `APP_SECRET`, y sus diez códigos de reserva, como huellas con clave. Cambiar `APP_SECRET` los invalida.
- Los **códigos de correo** (confirmar, recuperar contraseña) hasta que caducan: minutos.

### Los dispositivos

De cada dispositivo con sesión: el **nombre** que le puso su dueña, un identificador, su **clave pública**, un token (solo su huella, no el token) y cuándo se usó por última vez. Quitar un dispositivo de la cuenta conserva su fila, para que esa instalación siga ligada a la cuenta.

### Las bibliotecas subidas

Subir una biblioteca a la nube es una decisión aparte de tener cuenta. Cuando se hace, el servidor guarda **todo su contenido**: obras, ejemplares, dónde están y de quién son, las personas de la biblioteca con su nombre y su rol, lo que cada una apunta (estado, valoración, favoritos, deseos) y **el historial de cambios** con el dispositivo que hizo cada uno. El historial se guarda tal como llegó, firmado.

**No hay cifrado de extremo a extremo**: viaja cifrado hasta el servidor (HTTPS), pero en la base de datos está legible. Las bibliotecas que no se suben no salen de los dispositivos.

También guarda, por cada biblioteca, una clave propia con la que el servidor firma como «la nube», cifrada con `CLOUD_KEY_SECRET`.

### Las invitaciones

Quién invita, a qué persona y cuándo caduca (24 horas). El código se guarda solo como huella; en las invitaciones por correo **no se guarda la dirección**, sino una huella con clave que solo permite comprobar que quien acepta es quien se invitó. Se borran una semana después de caducar.

### El registro de actividad

Una tabla de solo añadir, para poder responder de lo que pasa con una cuenta o una biblioteca. Apunta:

- de las cuentas: crear la cuenta, iniciar y cerrar sesión, los intentos fallidos, cambiar o restablecer la contraseña, activar o quitar el segundo factor, bloquear, desbloquear y borrar la cuenta;
- de las bibliotecas de la nube: subirlas, cambiar la dueña, activar o apagar el poder de la nube, cambios de rol o del permiso de compartir, retirar o devolver el acceso a una persona o a un dispositivo, asignar un dispositivo y las invitaciones (creada, anulada, aceptada).

De cada línea guarda qué fue, cuándo (la hora del servidor), la **cuenta** y el **dispositivo**, la biblioteca (su id y su nombre de entonces), a quién afectó, un detalle corto (un rol, el tipo de invitación…), la **dirección IP** y el agente de usuario (la versión de la app). **No guarda contraseñas, códigos, tokens, correos ni el contenido del catálogo.** Las cuentas se identifican por su id y una huella del correo.

Se conserva **un año** y después la limpieza periódica lo borra. Sobrevive a la cuenta borrada. La IP es la que ve nginx; para que sea la real y no la del proxy, `TRUSTED_PROXIES` tiene que estar bien puesto ([la configuración](/es/run-your-server/configuration/)).

### La caché de búsquedas

Si activas buscadores ([la configuración](/es/run-your-server/configuration/#buscadores-de-datos)), el servidor guarda la **respuesta** del proveedor para no repetir la pregunta: de un libro, de 30 días si se encontró y 1 si no; las búsquedas por título (películas, videojuegos, cómics, juegos de mesa), 1 día en TMDB y 7 en los demás. **No guarda quién preguntó ni el texto**: la búsqueda se guarda bajo una huella que no permite recuperarlo. Solo cuenta cuántas búsquedas hace cada cuenta en una hora, para el límite (600), y no lo guarda más allá.

## Qué ves tú como administrador

Dicho claro: **quien tiene acceso a la base de datos o a la consola del servidor puede leer todo lo anterior**, incluido el contenido de las bibliotecas subidas y los nombres de las personas, y puede ver quién ha iniciado sesión y desde qué IP. No puede leer las contraseñas ni, sin `APP_SECRET`, los segundos factores.

Lo que **no** hay es una herramienta que lo enseñe: no hay panel, ni ruta HTTP que devuelva el registro. Lo que tienes es:

- la base de datos (`psql`);
- las [órdenes de consola](/es/run-your-server/administration/), en especial `app:activity:list`;
- los registros del sistema y de nginx.

Y puedes **actuar** sobre las cuentas: bloquear, quitar un segundo factor, y a nivel de base de datos casi cualquier cosa. Ten presente que todo eso es tu responsabilidad.

Díselo a quienes alojes: quien administra puede, técnicamente, leer sus bibliotecas. Que no lo haga es una promesa tuya, no una garantía de la técnica.

## Tus obligaciones si alojas a otras personas

Si el servidor es solo para ti, esto no te afecta. Si en él hay datos de otras personas (tu familia incluida, para este fin), **eres tú quien responde de ellos**:

- **Necesitas tu propia política de privacidad.** La del proyecto ([`web/es/privacidad.html`](https://github.com/kosmotheka/kosmotheka/blob/main/web/es/privacidad.html)) describe el servidor oficial y puede servirte de modelo: cambia el responsable, el contacto, el alojamiento y el proveedor del correo por los tuyos, y quita lo que tu servidor no hace (por ejemplo, un buscador que no has activado o Google si no lo configuraste).
- **Atiende sus peticiones de acceso y de borrado.** Para el borrado: la persona puede hacerlo en la app (ver [borrar una cuenta](/es/run-your-server/administration/#borrar-la-cuenta-de-alguien-que-lo-pide-por-correo)). Para el acceso, lo que hay son las consultas del registro de actividad y lo que tú saques de la base de datos.
- **Avisa del nivel de acceso**: que no hay cifrado de extremo a extremo y que tú puedes leerlo.
- **Cuida las copias de seguridad**: contienen todo lo anterior. Cífralas, como en [la guía de copias](/es/run-your-server/backups/).

Esto no es asesoramiento legal. Lo que te exija la ley depende de dónde vivas y de a quién alojes: si tienes dudas, consúltalo con quien sepa.

## Los registros de nginx

El registro de accesos de nginx apunta cada dirección que se pide. Dos familias de direcciones llevan información que no debe quedar escrita:

- **`/v1/metadata/…`**: lleva lo que busca una cuenta (el ISBN, por ejemplo). Guardarlo en un registro rompería la promesa de arriba de no apuntar quién busca qué.
- **`/v1/auth/google/connect` y `/callback`**: llevan un secreto en la dirección.

Las guías de instalación ya los excluyen: un `map` en `/etc/nginx/conf.d/kosmotheka-log.conf` y `access_log off` en la `location` ([sin Docker, paso 7](/es/run-your-server/without-docker/#7-nginx); [con Docker](/es/run-your-server/with-docker/)). Si montaste el proxy a tu manera, o tienes una CDN u otro proxy delante, **añade lo mismo ahí**: cualquier sistema que registre direcciones tiene que saltárselas.
