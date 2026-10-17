---
title: "Preparar el entorno: el servidor"
description: Cómo levantar el servidor Symfony en Docker, sus bases de datos, el correo de desarrollo, los datos de prueba y las comprobaciones.
sidebar:
  order: 3
---

El servidor (`server/`) es opcional: la app funciona sin él. Esta página es para quien quiera trabajar en él. **No hace falta PHP en tu máquina**: todo corre en Docker y se lanza con `make` desde `server/`. Si lo que quieres es *alojar* un servidor, lee mejor [Montar tu servidor](../../run-your-server/).

## Qué necesitas

Docker con Docker Compose, y que tu usuario pueda usarlo sin `sudo`. Nada más.

El servidor es un proyecto de Docker Compose aparte, con sus propios puertos y su propia red, para no chocar con otros proyectos. Es **solo para desarrollo**: nginx corre como root y el contenedor de PHP da `sudo` sin contraseña. Un servidor real usa la imagen de `server/docker/prod/`.

## Qué contiene

| Contenedor | Qué es | Puerto en tu máquina |
|---|---|---|
| `db_kosmotheka` | PostgreSQL 18 | 5433 |
| `php_kosmotheka` | PHP 8.5-FPM, Composer y Xdebug | ninguno |
| `nginx_kosmotheka` | nginx, delante de PHP: aquí responde la API | 8095 |
| `mailpit_kosmotheka` | Mailpit: captura todo el correo de desarrollo | 8097 (web) |
| `mercure_kosmotheka` | Hub Mercure, para los avisos al momento | ninguno (nginx lo expone) |

El código de `server/` se monta dentro del contenedor, y `spec/` se monta **solo de lectura**: es la especificación de las operaciones que comparten la app y el servidor.

## Arrancar

Los pasos exactos (qué archivo se copia y qué variables se rellenan) están en [`docs/development/server.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/server.md). En resumen:

1. En `server/docker/`, copia `.env.dist` a `.env`. **`.env` no se versiona.**
2. Genera con `openssl rand -hex 32` **dos claves distintas** para el hub de avisos (`MERCURE_PUBLISHER_JWT_KEY` y `MERCURE_SUBSCRIBER_JWT_KEY`) y ponlas en `docker/.env`. Docker Compose se niega a arrancar sin ellas.
3. En `server/`, crea `.env.local`, que **tampoco se versiona**. Lleva la cadena de conexión a la base (`DATABASE_URL`), un `APP_SECRET` que genera cada instalación (sin él, el servidor no emite códigos por correo) y las mismas dos claves de Mercure, más `MERCURE_URL`.
4. `make build` (solo la primera vez, o si cambia un Dockerfile), `make up` y `make install` (Composer, dentro del contenedor).
5. `make check` para comprobar que todo pasa.

La API responde en `http://localhost:8095`.

`make help` lista todos los comandos. Algunos de uso diario:

| Tarea | Comando (desde `server/`) |
|---|---|
| Parar y arrancar | `make down` · `make up` |
| Consola de Symfony | `make console C="debug:router"` |
| Entrar al contenedor PHP | `make sh` |
| Migraciones | `make migration` (genera una) · `make migrate` (aplica) · `make db-test` (en la base de pruebas) |
| Pruebas | `make test` · `make test ARGS="--filter Conformance"` |
| Comprobar todo sin tocar nada | `make check` |
| Arreglar estilo y Rector | `make fix` |
| Tareas programadas (la purga de cuentas sin confirmar) | `make worker` |

**Sobre `.env`.** El `.env` de `server/` va en el repositorio y **no lleva credenciales**, solo valores por defecto y huecos vacíos. Lo que sea secreto va en `.env.local`. Tampoco se escribe ningún `APP_SECRET` en el repositorio, ni de ejemplo.

## Las bases de datos

| Entorno | Base | Dónde se configura |
|---|---|---|
| Desarrollo | `kosmotheka` | `server/.env.local` y `docker/.env` |
| Pruebas | `kosmotheka_test` | `server/.env.test`, que **sí** va en el repositorio |

`.env.test` tiene que estar versionado: Symfony no carga `.env.local` al ejecutar las pruebas, y sin él un clon recién hecho no las pasaría. No lleva ningún secreto: cada ejecución de las pruebas se inventa uno.

La base de pruebas y su usuario las crea un script SQL **la primera vez** que arranca el volumen de PostgreSQL. Si el volumen ya existía sin ella, bórralo con los contenedores parados (`docker volume rm kosmotheka_db_kosmotheka`), o ejecuta ese SQL a mano.

Todo cambio en el esquema pasa por una migración (`make migration`, luego `make migrate`), nunca por `doctrine:schema:update`.

## Datos de prueba: las fixtures

```bash
cd server
make fixtures
```

**Vacía la base de desarrollo** y la llena con escenarios de prueba: familias con sus bibliotecas en la nube, y también los intentos hostiles que el servidor debe rechazar (alguien que conoce los ids de otra biblioteca, por ejemplo). Todo entra por los mismos servicios que usa la API, no con `INSERT`, así que la base solo contiene lo que las reglas dejan entrar. Después puedes abrirla con cualquier cliente de PostgreSQL (puerto 5433) y mirar qué guarda el servidor. Los mismos escenarios corren en las pruebas. El detalle de cada caso está en [`docs/development/fixtures.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/fixtures.md).

`make app-fixtures` hace lo mismo y además prepara una carpeta de datos de la app para cada personaje, para abrirlos con `KOSMOTHEKA_DATA_DIR` (ver [Preparar el entorno: la app](../setup-app/)).

## El correo en desarrollo

El servidor envía correos (códigos de confirmación, recuperación de contraseña, invitaciones). En desarrollo, el transporte (`MAILER_DSN`, en `.env.local`) apunta a Mailpit, que **captura todo y no envía nada fuera**. Abre `http://localhost:8097` para leerlos. En las pruebas el transporte es nulo y los tests leen los mensajes del registro de Symfony.

## Servicios de fuera, siempre opcionales

Cada uno se activa con una variable de `.env.local`. **Vacía, el servicio queda apagado** y el servidor sigue funcionando; lo que dependía de él responde con un error claro (`404 …_unavailable`) y `GET /v1/server` lo anuncia. Nunca pongas un valor real en un archivo versionado.

| Qué activa | Variable |
|---|---|
| Entrar con Google | `GOOGLE_CLIENT_ID` y `GOOGLE_CLIENT_SECRET` (guía: [`google-oauth.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/google-oauth.md)) |
| Buscar libros en Google Books por el servidor | `GOOGLE_BOOKS_API_KEY` |
| Películas y series (TMDB) | `TMDB_API_TOKEN` |
| Videojuegos (RAWG) | `RAWG_API_KEY` |
| Cómics (Comic Vine) | `COMIC_VINE_API_KEY` |
| Juegos de mesa y de rol (BoardGameGeek) | `BGG_API_TOKEN` (y el tope diario `BGG_DAILY_CALL_CAP`) |
| La identidad de cada biblioteca de la nube | `CLOUD_KEY_SECRET` |
| Los enlaces de invitación | `INVITATION_BASE_URL` |
| Avisos al momento (Mercure) | `MERCURE_URL` |
| Restringir quién puede crear cuenta | `REGISTRATION_ALLOWED_EMAILS` |
| Segundo factor | `TWO_FACTOR_ENABLED` |

Cada proveedor de datos tiene sus propias condiciones de uso (casi todos exigen uso no comercial y una atribución): las explica [`docs/development/servicios-externos.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/servicios-externos.md) (en español). Las claves de RAWG y Comic Vine solo admiten ir en la dirección de la llamada, por lo que el código las trata con cuidado: usa un cliente HTTP sin registrador y lanza las excepciones sin la dirección. No registres las llamadas salientes del servidor.

Las pruebas **nunca** usan estos servicios: leen respuestas grabadas.

## Las comprobaciones

`make check` ejecuta cuatro herramientas que no se solapan, y tarda unos minutos:

| Herramienta | Qué cubre |
|---|---|
| php-cs-fixer | Estilo (`@Symfony` y `@Symfony:risky`) |
| Rector | Modernización de la sintaxis y reglas de calidad de Symfony y Doctrine |
| PHPStan | Corrección, en el nivel más alto (`max`), **sin archivo de base**: donde un valor puede faltar, se acota, no se silencia el aviso |
| PHPUnit | Comportamiento. Incluye los casos de conformidad de `spec/` |

`make fix` corrige el estilo y Rector, y después comprueba. **No lo canalices a otro comando** (`make fix | tail`): el código de salida que verías sería el del segundo comando y ocultaría los fallos.

Las reglas de código del servidor (dónde va cada clase, errores, convenciones de Symfony) están en [`server/AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/server/AGENTS.md). Siguiente paso: [Cómo contribuir](../how-to-contribute/).
