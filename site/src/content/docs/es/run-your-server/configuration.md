---
title: La configuración, variable a variable
description: Todas las variables de entorno del servidor de Kosmotheka, para qué sirven, cómo se generan y qué pasa si se dejan vacías.
sidebar:
  order: 4
---

Toda la configuración son variables de entorno. Se ponen en `server/.env.local` (un archivo que no está en git y al que solo debe poder acceder el usuario del servidor), o en el entorno del servicio. **No edites `server/.env`**: es el archivo de valores por defecto y se sobrescribe al actualizar.

Después de cambiar una variable: `php8.5 bin/console cache:clear` y reinicia PHP-FPM (y el servicio de tareas si la usa).

Una convención repetida: **vacía = esa función no se ofrece**, y el servidor sigue funcionando. `GET /v1/server` (que puedes abrir en el navegador) dice qué ofrece tu servidor y es lo primero que mira la app.

## Lo básico

| Variable | Obligatoria | Qué es |
|---|---|---|
| `APP_ENV` | sí | `prod` en un servidor de verdad. Por defecto es `dev`, que es más lento y enseña errores detallados |
| `APP_SECRET` | sí | Secreto de la aplicación. Se genera con `openssl rand -hex 32`. Vacío, el servidor se niega a emitir códigos por correo |
| `DATABASE_URL` | sí | Conexión a PostgreSQL: `postgresql://usuario:contraseña@127.0.0.1:5432/base?serverVersion=18&charset=utf8`. La `serverVersion` debe ser `18` |
| `DEFAULT_URI` | sí | La dirección pública del servidor, `https://tu-dominio`, sin barra final. Tiene que ser exactamente la que ve la gente: de aquí sale la dirección de vuelta de Google y el servidor que firma cada dispositivo al entrar. Si no coincide con la que tiene guardada la app, **nadie puede iniciar sesión** |
| `TRUSTED_PROXIES` | sí, si hay proxy | Direcciones de los proxys de confianza delante del servidor, para leer la IP real de quien llama (los límites de peticiones la usan). Con nginx en la misma máquina, `127.0.0.1`. Si hay una CDN, añade sus rangos |
| `CLOUD_KEY_SECRET` | sí, para la nube | 32 bytes aleatorios **en base64**: `openssl rand -base64 32`. Cifra la clave privada con que firma cada biblioteca. Vacío, las bibliotecas no tienen identidad de nube y la API devuelve nulos. **Perderlo, o cambiarlo, deja al servidor sin poder firmar en las bibliotecas ya creadas.** Guárdalo en tus copias, aparte de la base de datos |
| `INVITATION_BASE_URL` | sí, para invitar | Con lo que empiezan los enlaces de invitación, sin barra final (`https://tu-dominio`; el enlace es `<esto>/i/<código>`). Vacío, el servidor no hace invitaciones |
| `REGISTRATION_ALLOWED_EMAILS` | no | Correos que pueden crear una cuenta, separados por comas. **Vacío, cualquiera puede registrarse.** Para un servidor familiar, ponlo: para el resto el registro parece funcionar pero no se crea nada ni se envía ningún correo, y quien ya tiene cuenta entra como siempre. Entrar con Google también respeta la lista |
| `TWO_FACTOR_ENABLED` | no | `true` por defecto: las cuentas nuevas pueden activar un segundo factor (app de autenticación). A `false`, ya no se ofrece; las cuentas que lo tenían siguen necesitándolo para entrar |
| `SERVER_EXTRA_ORIGINS` | no | **Déjala vacía.** Otras direcciones por las que se llega al mismo servidor; solo sirve en desarrollo. Un dispositivo firma su entrada para la dirección del servidor, y cada dirección que añades aquí es otra en la que esa firma también vale |

Variables que ya traen un valor razonable y no hace falta tocar: `APP_SHARE_DIR`, `LOCK_DSN` (`flock`, bloqueos en el sistema de archivos de una sola máquina) y `MESSENGER_TRANSPORT_DSN` (la cola de tareas va en la propia base de datos).

## El correo

Las cuentas envían correo: el código para confirmar el alta, el de recuperar la contraseña y los avisos de invitación.

| Variable | Obligatoria | Qué es |
|---|---|---|
| `MAILER_DSN` | sí, para registrarse | Cómo se envía. Por defecto es `null://null`, que **no envía nada**: nadie podrá confirmar una cuenta nueva con un código. Con un servidor SMTP, `smtp://usuario:contraseña@servidor:587`. Con Brevo (incluido de serie), `brevo+api://CLAVE@default`. Cualquier otro servicio necesita su paquete de Symfony Mailer |
| `MAILER_FROM` | sí | Dirección desde la que se envía. Por defecto es la del servicio oficial: **pon una tuya**, de un dominio que puedas autenticar (SPF y DKIM) para que no caiga en el correo no deseado |

Con Google configurado, una cuenta se puede crear sin correo del servidor (Google confirma la dirección), pero la recuperación de contraseña, y los avisos de invitación, sí lo necesitan. Así que configura el correo siempre.

## Entrar con Google

Opcional. Hace falta un «cliente OAuth» en la consola de Google Cloud.

| Variable | Qué es |
|---|---|
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` | El identificador y el secreto de ese cliente. **Las dos vacías, y Google no se ofrece.** El secreto nunca va al repositorio |

La dirección de vuelta autorizada que registras en Google es `DEFAULT_URI` más `/v1/auth/google/callback`. Las direcciones `/v1/auth/google/connect` y `/v1/auth/google/callback` llevan un secreto en la dirección y **no deben escribirse en ningún registro de accesos** (nginx, un proxy, una CDN): la configuración de nginx de la guía ya lo evita.

## Buscadores de datos

Con una cuenta, la app puede buscar libros, películas y demás a través del servidor. Cada buscador es opcional y necesita **una clave que pides tú** a su proveedor, aceptando sus condiciones. Sin clave, el servidor no ofrece esa búsqueda (la app avisa y sigue funcionando, y busca por su cuenta lo que puede).

| Variable | Qué busca | Condiciones del proveedor |
|---|---|---|
| `GOOGLE_BOOKS_API_KEY` | Libros por ISBN (Google Books) | Una clave de API de Google Cloud, restringida a la API Books |
| `TMDB_API_TOKEN` | Películas y series (TMDB) | El «API Read Access Token» de tu cuenta de TMDB. Solo uso no comercial, con su atribución |
| `RAWG_API_KEY` | Videojuegos (RAWG) | Una clave de la API de RAWG, aceptando sus condiciones y su atribución |
| `COMIC_VINE_API_KEY` | Cómics (Comic Vine) | Una cuenta de Comic Vine; uso estrictamente no comercial. El servidor se limita a 150 peticiones por hora y recurso (el servicio admite 200) |
| `BGG_API_TOKEN` | Juegos de mesa (BoardGameGeek) y libros de rol (RPGGeek) | El token de una aplicación registrada en BoardGameGeek, no comercial. Hay que mostrar el logo «Powered by BGG», y la app lo hace |
| `BGG_DAILY_CALL_CAP` | — | Tope de llamadas reales al día a BoardGameGeek (1000 por defecto). El servicio no publica cifras: ajústalo mirando el «uso» de tu aplicación registrada |

Cómo se guardan: todas viajan en una cabecera salvo RAWG y Comic Vine, que **solo admiten la clave en la dirección de la llamada**. Por eso con esas dos no debes poner delante del servidor un proxy de salida ni un registro que guarde las direcciones de `api.rawg.io` y `comicvine.gamespot.com`, ni bajar el nivel de registro por debajo de `warning`. Las direcciones `/v1/metadata/` llevan el ISBN que busca una cuenta: tampoco se escriben en el registro de accesos (la guía ya lo hace).

## Avisos al momento (Mercure)

Opcional. Ver [el último paso de montarlo sin Docker](/es/run-your-server/without-docker/#10-avisos-al-momento-opcional).

| Variable | Qué es |
|---|---|
| `MERCURE_URL` | La dirección interna del hub, que también es la audiencia de sus tokens (`http://127.0.0.1:3000/.well-known/mercure`). **Vacía, no hay avisos al momento** y el servidor no publica nada |
| `MERCURE_PUBLISHER_JWT_KEY` | Firma los tokens con que el servidor publica en el hub. `openssl rand -hex 32` |
| `MERCURE_SUBSCRIBER_JWT_KEY` | Firma los tokens de suscripción que se dan a los dispositivos. **Distinta de la anterior**: así un token de suscriptor no puede publicar. `openssl rand -hex 32` |

Las dos claves van con el mismo valor en el servidor y en el hub.
