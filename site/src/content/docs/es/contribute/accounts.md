---
title: Cuentas e inicio de sesión
description: "Cómo funciona la cuenta del servidor: registro con correo y código, entrar con Google, el segundo factor, el dispositivo como sesión, los límites de peticiones y el borrado."
sidebar:
  order: 12
---

La cuenta es **opcional** y solo existe en el servidor: la app funciona entera sin ella. Esta página explica cómo se crea, cómo se entra, qué protege cada paso y qué guarda el servidor. El detalle de cada ruta está en [La nube: la API](../cloud-api/), y la parte de seguridad en [el modelo de amenazas](../security/). El código vive en `server/src/` (`Controller/`, `Security/`).

## Qué es una cuenta, y qué no

Una **cuenta** (`user`) es el acceso de una persona al servidor: correo, contraseña (guardada como hash, argon2id si está disponible), nombre, idioma (`en` o `es`) y roles de servidor. **No es un miembro** (`member`) de una biblioteca: el miembro es una persona dentro del catálogo, y puede estar o no enlazado a una cuenta. El enlace es un id aleatorio, `account_id`, único de cada cuenta, que es lo **único de la cuenta que viaja con una biblioteca**; no es el id interno ni sale del correo, así que quien lee una biblioteca no aprende ni el correo de nadie.

Una cuenta nace **sin confirmar**, y se borra a las 24 horas si nadie confirma el correo.

## Registro con correo y código

1. **`POST /v1/account/register`** `{email, password, name, locale}`. Responde **siempre `202`** con un cuerpo vacío, haga lo que haga por dentro: no dice si el correo ya tiene cuenta, si está en la lista de un servidor cerrado ni si ha gastado sus correos. *Por qué:* para que nadie pueda comprobar qué direcciones están registradas.
   - Correo nuevo: crea la cuenta sin confirmar y manda un código.
   - Correo con cuenta sin confirmar: solo manda un código nuevo. **No cambia nada de la cuenta**, ni la contraseña.
   - Correo con cuenta confirmada: no cambia nada y manda un aviso corto («alguien intentó crear una cuenta con tu dirección»).
2. **`POST /v1/account/confirm`** `{email, code, password}` confirma la cuenta y **fija la contraseña**. La contraseña se elige aquí, con el código del correo, y no al registrarse: así la que queda es siempre la de quien controla el correo, aunque otra persona hubiera registrado la misma dirección antes.

El **código** es de 6 cifras, caduca a los 15 minutos, permite 5 intentos fallidos (al quinto se borra), es de un solo uso y solo hay uno vivo por cuenta y propósito. Se guarda como HMAC del código, nunca en claro. Sirve para tres cosas que no se mezclan: confirmar el correo, recuperar la contraseña y confirmar una acción delicada de una cuenta sin contraseña.

**El correo sale después de responder** (en `kernel.terminate`), para que el tiempo de la respuesta no diga si la dirección tiene cuenta.

**Servidor cerrado.** Con `REGISTRATION_ALLOWED_EMAILS` (una lista de correos), solo esas direcciones pueden crear cuenta; a las demás se les responde el mismo `202` sin crear nada. `GET /v1/server` dice `registration_restricted: true`, nunca la lista.

### Contraseña olvidada y cambio

- **`POST /v1/account/password/forgot`** manda un código de recuperación (siempre `202`, por lo mismo).
- **`POST /v1/account/password/reset`** `{email, code, password, totp_code?}`: con el código (y el segundo factor, si lo tiene) cambia la contraseña y **cierra la sesión de los demás dispositivos**.
- **`POST /v1/account/password`** (con sesión) cambia la contraseña conociendo la actual.

## El dispositivo como sesión

No hay cookies ni sesión de navegador. Cada instalación de la app es un **dispositivo** (`device`) con un id y un par de claves Ed25519 propio, y **entrar es probar que se tiene esa clave privada**:

1. **`POST /v1/auth/challenge`** devuelve un reto: 32 bytes aleatorios, válido 5 minutos y **de un solo uso** (se consume se use bien o mal).
2. **`POST /v1/auth/login`** `{email, password, device_id, device_name, public_key, challenge, signature, totp_code?}`. La app firma con la clave del dispositivo un texto que incluye el reto, el id del dispositivo, el **propósito** (`login` o `google`) y el **origen del servidor**. *Por qué:* una firma hecha para otro servidor, o para entrar con Google, no vale aquí para entrar con contraseña, ni al revés.
3. El servidor comprueba, **en este orden**: el reto, la firma, el límite de fallos de la cuenta, las credenciales, el estado de la cuenta (sin confirmar o bloqueada), el segundo factor, y por último el dispositivo. Responde con un **token** de 32 bytes aleatorios.

El servidor **guarda solo el hash del token**. Cada llamada autenticada lleva `X-Device-Id` y `Authorization: Bearer <token>`.

**Un dispositivo es su id junto con su clave pública**, no el id solo. El id viaja en cada operación y lo ven todos los que comparten una biblioteca; si bastara con él, quien lo supiera podría iniciar sesión el primero con una clave propia y dejar al móvil verdadero fuera. Un dispositivo puede estar **con sesión**, **con la sesión cerrada** (la misma cuenta puede volver a entrar con él) o **quitado**. Un dispositivo quitado no vuelve a iniciar sesión con ninguna cuenta; la app, al saberlo, crea una identidad nueva.

Si el servidor cierra una sesión (por ejemplo, porque la contraseña cambió en otro dispositivo), la siguiente llamada responde `401` con el motivo, para que la app lo explique.

## Entrar con Google

Es opcional (el servidor tiene que tener un cliente de Google configurado) y sigue el camino de «OAuth con PKCE» pensado para que **lo que viaja por el navegador no sirva por sí solo**:

1. La app genera un secreto que **no sale de ella** y manda su huella a `POST /v1/auth/google/start`. El servidor guarda un **intento** (10 minutos) y devuelve la dirección a abrir.
2. El navegador del sistema va a `GET /v1/auth/google/connect`. El servidor genera su propio `state` y una cookie ligada a ese navegador, y redirige a Google.
3. Google vuelve a `GET /v1/auth/google/callback`. El servidor solo sigue si el `state` es suyo, de un intento que espera, **y la cookie es la del mismo navegador**. Cada `state` vale una vez.
4. La app termina con `POST /v1/auth/google/finish`, con **la misma prueba de dispositivo que el inicio de sesión normal** y el secreto del paso 1. Sin ese secreto, el código que volvió por el navegador no da nada.

Además, la app **solo acepta la vuelta de un intento que ella empezó y está esperando**, y solo vale un correo que Google confirma. Se reconoce a la persona por su **identificador de Google**, no por el correo. Si una cuenta registrada con contraseña nunca se confirmó y la verdadera dueña entra con Google, esa cuenta pasa a ser suya y **la contraseña de la otra persona deja de valer**.

**Segundo factor al entrar con Google:** el servidor **lo pide también** si la cuenta lo tiene activado (`GoogleController::finish` comprueba el límite de fallos de la cuenta y luego llama a `requireTwoFactor`). Google no sustituye al segundo factor de la cuenta.

## El segundo factor

Opcional por cuenta, y el servidor puede desactivarlo del todo (`TWO_FACTOR_ENABLED`). Es un código **TOTP** (el de una app autenticadora: 6 cifras, 30 segundos) más **diez códigos de reserva** de un solo uso, para quien pierda el móvil.

- **`POST /v1/account/two-factor/setup`** crea un secreto pendiente y devuelve el secreto y su dirección `otpauth://`.
- **`POST /v1/account/two-factor/enable`** `{code, …}` comprueba un código contra el secreto pendiente, lo activa y devuelve los diez códigos **una sola vez**.
- **`DELETE /v1/account/two-factor`** y **`POST /v1/account/two-factor/backup-codes`** lo quitan y regeneran los códigos.
- El código del segundo factor puede ser el de 6 cifras o uno de reserva (con o sin guion, en mayúsculas o minúsculas).
- **Anti-repetición:** un código de un paso de tiempo igual o anterior al último aceptado se rechaza, aunque sea válido.
- **Los secretos se guardan cifrados** (XChaCha20-Poly1305, con una clave derivada de `APP_SECRET` y el id de la cuenta como dato asociado): una copia de la base de datos sola no sirve para generar códigos. Los códigos de reserva, como hash.

### Las acciones delicadas

Una sesión dura mucho, y un móvil desbloqueado en otras manos la tiene. Lo que cambia **cómo se entra en la cuenta** pide algo más que la sesión:

| Acción | Se pide | Y, con el segundo factor activado |
|---|---|---|
| Cambiar la contraseña | la contraseña actual | un código |
| Restablecerla | el código del correo | un código |
| Borrar la cuenta | la contraseña, o un código del correo si no tiene contraseña | un código |
| Activar el segundo factor | la contraseña, o un código del correo | su propio código |
| Quitarlo o regenerar los códigos | la contraseña si tiene | un código |

Una cuenta sin contraseña (solo Google) usa un **código del correo** (`POST /v1/account/code/action`) en vez de la contraseña, **nunca al revés**: una cuenta que tiene contraseña no la sustituye por un código del correo, que tendría también quien tuviera el correo abierto. El orden es siempre: quién lo pide, luego el segundo factor, y solo al final se gasta el código del correo.

## Los límites de peticiones

`symfony/rate-limiter`, de ventana deslizante. Los que gasta una petición por llegar son atributos `#[RateLimit]` en la acción; los que dependen del flujo los consume `App\Security\RequestLimiter`. Todos acaban en la misma respuesta, `429 {"error":"too_many_requests"}` con `Retry-After`.

| Límite | Cuánto | Clave | Para qué |
|---|---|---|---|
| Registro | 5 por hora | IP | Que nadie cree cuentas en masa |
| Inicio de sesión (el reto y el inicio) | 30 cada 15 minutos | IP | Ancho a propósito: una familia en la misma wifi comparte IP |
| **Fallos de una cuenta** | 5 cada 15 minutos | huella del correo escrito | Solo cuenta los intentos **fallidos** (contraseña o código equivocados). Un correo sin cuenta cuenta igual, para no revelar cuáles existen |
| Códigos por correo | 10 cada 15 minutos | IP | |
| Correos por dirección | 3 cada 15 minutos | huella de la dirección | Que no se pueda inundar a un tercero con correos |
| Código del segundo factor | 5 cada 15 minutos | cuenta | Que desde muchas IP no se prueben los 6 dígitos |

Un cuerpo mal formado (`400` o `422`) **no gasta** el límite; una petición bien formada con la contraseña equivocada, sí. Los límites de la nube, las invitaciones y los metadatos están en [La nube: la API](../cloud-api/).

## Borrar la cuenta

**`DELETE /v1/account`** pide la contraseña (o un código del correo, y el segundo factor si está activo). Una biblioteca **no se pierde con la cuenta de quien la subió**: antes de borrarla, y en la misma transacción, cada biblioteca de la nube de la que es dueña pasa a otra cuenta con un sitio activo en ella (administradora primero), y solo se borra con la cuenta si no queda nadie.

Además, una purga cada hora (`App\Scheduler\PurgeSchedule`, que necesita un *worker* en marcha) borra las cuentas sin confirmar de más de 24 horas y los códigos caducados.

## Qué guarda el servidor

- **De la cuenta:** correo, hash de la contraseña, nombre, idioma, el `account_id`, y si hay segundo factor, su secreto cifrado y los hashes de los códigos de reserva.
- **De cada dispositivo:** su id, el nombre, su clave pública, el **hash** del token de sesión y cuándo se usó por última vez.
- **Los códigos** del correo, como HMAC, y se borran al caducar.
- **De Google:** el identificador de Google, el correo y el nombre. **Ningún token de Google.**
- **El registro de actividad:** una tabla donde solo se añade, con qué pasó, a qué cuenta y dispositivo, cuándo y desde qué IP y versión de la app. Se conserva un año. No guarda contraseñas, códigos ni enlaces; de una cuenta borrada solo queda una huella del correo.

**Qué no guarda:** la contraseña en claro, el token en claro, los códigos en claro, ni lo que una cuenta busca en los servicios de datos.

## Los errores

Los errores de negocio responden con un cuerpo `{"error": "<código>"}` y el estado HTTP que toque. Los códigos son casos de `AccountErrorCode`, y un único listener (`AccountExceptionListener`) los convierte en respuesta, de modo que ningún controlador la construye a mano. Los errores de validación del cuerpo responden `422` con el formato de Symfony.
