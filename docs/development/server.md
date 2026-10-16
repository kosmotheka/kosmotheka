# Servidor: entorno de desarrollo

> El servidor Symfony (`server/`) corre en Docker, en su propio proyecto de Docker Compose, separado del contenedor Distrobox de la app. No hace falta PHP en el sistema: todo se lanza con `make` desde `server/`.

## Qué hay en `server/`

| Ruta | Qué es |
|---|---|
| `docker/` | Docker Compose del servidor: PHP-FPM, nginx y PostgreSQL. **Solo para desarrollo:** nginx corre como root y el contenedor de PHP da `sudo` sin contraseña; un servidor real necesita sus propias imágenes (pendiente de la guía de instalación) |
| `src/` | Código, en carpetas por rol (`Dto/`, `Enum/`, `Validator/`…); ver las convenciones en `server/AGENTS.md` |
| `tests/` | PHPUnit, espejo de `src/`; `tests/Validator/` ejecuta los casos de conformidad de `spec/` |
| `Makefile` | Todos los comandos (`make help`) |
| `AGENTS.md` | Instrucciones para agentes específicas del servidor |

## Arrancar

```bash
# Dos claves distintas para el hub de avisos (hito 7.3); van en docker/.env y en .env.local
PUBLISHER_KEY=$(openssl rand -hex 32); SUBSCRIBER_KEY=$(openssl rand -hex 32)
cd server/docker
cp .env.dist .env        # solo la primera vez; .env no se versiona
printf 'MERCURE_PUBLISHER_JWT_KEY=%s\nMERCURE_SUBSCRIBER_JWT_KEY=%s\n' "$PUBLISHER_KEY" "$SUBSCRIBER_KEY" >> .env
# (.env.dist trae esas dos claves vacías: si ya las copiaste, edita las líneas en vez de añadir otras)
cd ..
cat > .env.local <<ENV   # solo la primera vez; tampoco se versiona
DATABASE_URL="postgresql://kosmotheka:kosmotheka@db_kosmotheka:5432/kosmotheka?serverVersion=18&charset=utf8"
APP_SECRET=$(openssl rand -hex 32)
MERCURE_URL=http://mercure_kosmotheka/.well-known/mercure
MERCURE_PUBLISHER_JWT_KEY=$PUBLISHER_KEY
MERCURE_SUBSCRIBER_JWT_KEY=$SUBSCRIBER_KEY
ENV
make build               # solo la primera vez o si cambia un Dockerfile
make up
make install             # composer install dentro del contenedor
make check               # estilo, Rector, PHPStan y tests
```

La API responde en <http://localhost:8095>.

## Contenedores, puertos y red

Elegidos para no chocar con otros proyectos Docker de la máquina (Riddle, rpgutilities…).

| Contenedor | Qué es | Puerto en el equipo | IP interna |
|---|---|---|---|
| `db_kosmotheka` | PostgreSQL 18 | 5433 | 172.194.0.10 |
| `php_kosmotheka` | PHP 8.5-FPM, Composer, Xdebug (puerto 9001) | — | 172.194.0.11 |
| `nginx_kosmotheka` | nginx, delante de PHP | 8095 | 172.194.0.12 |
| `mailpit_kosmotheka` | Mailpit: captura todo el correo en desarrollo | 8097 (web) | 172.194.0.13 |
| `mercure_kosmotheka` | Hub Mercure 1.0 (avisos al momento, D75) | — **ninguno publicado** | 172.194.0.14 |

- Subred `172.194.0.0/24`; el proyecto Compose se llama `kosmotheka`.
- El código de `server/` se monta en `/var/www/kosmotheka.local`; `spec/` se monta **en solo lectura** en `/var/www/spec`.
- PostgreSQL guarda los datos en el volumen `kosmotheka_db_kosmotheka`, no en el árbol del repo.

## Bases de datos

| Entorno | Base | Usuario | Dónde se configura |
|---|---|---|---|
| dev | `kosmotheka` | `kosmotheka` | `server/.env.local` y `docker/.env` (ninguno se versiona) |
| test | `kosmotheka_test` | `test` | `server/.env.test` (Doctrine añade `_test` al nombre) |

`kosmotheka_test` y su usuario los crea `docker/postgres/init/01-test-database.sql` la **primera vez** que arranca el volumen. Si el volumen ya existía sin ella, bórralo (`docker volume rm kosmotheka_db_kosmotheka`, con los contenedores parados) o ejecuta ese SQL a mano.

Como en Riddle, `.env` no lleva credenciales: las de desarrollo van en `.env.local`, también `APP_SECRET`, que cada instalación genera (sin él, el servidor se niega a emitir códigos por email). `.env.test` sí se versiona, porque Symfony no carga `.env.local` en test y sin él un clon recién hecho no pasaría la suite. No lleva secreto: cada ejecución de los tests se inventa uno (`tests/bootstrap.php`), salvo que `.env.test.local` o el entorno fijen otro; su usuario `test` solo existe dentro del contenedor y solo tiene acceso a `kosmotheka_test`. Las de producción irán en `.env.local` en el VPS, nunca en el repo.

## Correo (hito 7.1)

Las cuentas envían correo (código de confirmación, recuperación de contraseña; ver `docs/architecture/accounts.md`). El transporte lo fija `MAILER_DSN`:

| Entorno | `MAILER_DSN` | Dónde se fija |
|---|---|---|
| dev | `smtp://mailpit_kosmotheka:1025` | `server/.env.local` (no versionado) |
| test | `null://null` (no se envía nada; los tests leen `mailer.message_logger_listener`) | `server/.env.test` |
| prod | `brevo+api://CLAVE@default` (`symfony/brevo-mailer`) | `.env.local` en el VPS |

**Mailpit** (`mailpit_kosmotheka`, imagen `axllent/mailpit`, fijada a una versión concreta) captura todo el correo de desarrollo: su interfaz web está en <http://localhost:8097>. El puerto SMTP (1025) solo es accesible dentro de la red de Docker, no desde el equipo.

`TWO_FACTOR_ENABLED` (por defecto `true` en `.env`) activa el alta de segundo factor (D78, `docs/architecture/accounts.md`) para cuentas nuevas; a `false` en un `.env.local`, `POST /v1/account/two-factor/setup` responde `two_factor_unavailable` y `GET /v1/server` anuncia `two_factor: false`. No afecta a una cuenta que ya lo tenga activado: se le sigue pidiendo al iniciar sesión.

`TRUSTED_PROXIES` (por defecto `127.0.0.1` en desarrollo) le dice a Symfony qué proxys delante del servidor son de confianza para leer `X-Forwarded-For` y `X-Forwarded-Proto`: en producción, nginx y quizá un CDN. Se usa para que los limitadores de peticiones por IP (`config/packages/rate_limiter.yaml`) vean la IP real del cliente.

## Avisos al momento: el hub de Mercure (hito 7.3)

`mercure_kosmotheka` (`dunglas/mercure:v1.0.3`, fijada) manda a la app abierta un aviso «hay algo nuevo» (D75; el diseño, en `docs/architecture/cloud.md`, «Avisos al momento»). **No publica ningún puerto en el equipo**: Symfony le habla por la red de Docker y la app llega a través de nginx (`/.well-known/mercure`, por el 8095 en desarrollo).

| Clave | Dónde | Para qué |
|---|---|---|
| `MERCURE_PUBLISHER_JWT_KEY` | `docker/.env` (el hub) y `.env.local` (Symfony) | Firma los tokens con que Symfony publica. Solo la usa el servidor |
| `MERCURE_SUBSCRIBER_JWT_KEY` | los mismos dos | Firma los tokens de suscripción que se dan a los dispositivos. **Distinta de la anterior**: un token de suscriptor no puede publicar |
| `MERCURE_URL` | `.env.local` (`.env` la trae vacía) | `http://mercure_kosmotheka/.well-known/mercure`. **Vacía = sin Mercure** (D18): no se publica nada y `GET /v1/server` dice `events: false` |

- Cada clave se genera con `openssl rand -hex 32` (32 bytes o más) y **el mismo valor va en `docker/.env` y en `.env.local`**. El `docker-compose` se niega a arrancar sin ellas. Quien ya tenía el entorno montado: añade las dos claves y `MERCURE_URL`, y `make up`.
- `.env.test` lleva claves y una URL falsas, solo para los tests (que no usan el hub real: `tests/Support/RecordingHub.php` apunta lo que se publicaría). Ningún valor real va al repositorio.
- El emisor (`urn:kosmotheka:server`) y la audiencia (la misma `MERCURE_URL`) están fijos en `docker/docker-compose.yaml`; si cambias `MERCURE_URL`, cambia también `resource_identifier` allí.
- Los suscriptores anónimos, la API de suscripciones y CORS vienen apagados por defecto en el hub y no se tocan. Latido: 40 s.

## La clave de la nube (hito 7.5.2a)

Cada biblioteca de la nube tiene un dispositivo propio con su clave (D95). La clave privada se guarda en la base de datos **cifrada** con un secreto del servidor que no está en la base:

| Variable | Dónde | Qué es |
|---|---|---|
| `CLOUD_KEY_SECRET` | `.env.local` (`.env` la trae vacía; `.env.test` trae un valor de prueba que no sirve fuera de los tests) | 32 bytes aleatorios en base64. **Vacía = sin identidad de nube** (D18): el servidor funciona como antes, las bibliotecas no tienen dispositivo de nube y la API devuelve nulos |
| `INVITATION_BASE_URL` | `.env.local` (`.env` la trae vacía; `.env.test` trae una de prueba) | Con lo que empiezan los enlaces de invitación, sin barra al final: `<esta>/i/<token>` (D43, D79). En desarrollo, p. ej. `http://localhost:8095`; en el servidor oficial, `https://kosmotheka.app`. **Vacía = sin invitaciones** (D18): `409 invitations_unavailable` y `GET /v1/server` dice `invitations: false`. Ver [cloud.md](../architecture/cloud.md#invitaciones-761-d43) |
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` | `.env.local` (`.env` las trae vacías) | El cliente OAuth de «Continuar con Google» (D77, [google-oauth.md](google-oauth.md)). **Vacías = sin Google**: sus rutas responden `404 google_unavailable` y `GET /v1/server` dice `sign_in_with_google: false`. **Nunca van al repositorio.** La dirección de vuelta que se registra en Google es `DEFAULT_URI` + `/v1/auth/google/callback` |
| `DEFAULT_URI` | `.env.local` (`.env` trae `http://localhost`) | La dirección pública del servidor (en producción, `https://api.kosmotheka.app`): de aquí sale la dirección de vuelta de Google y la del navegador de un intento. Tiene que ser la que ve la gente, no la interna. **Es también parte de lo que firma un dispositivo para entrar** (D100, `App\Security\ServerOrigin`): `esquema://host[:puerto]` en minúsculas, sin barra ni el puerto del esquema, y debe coincidir con la dirección que la app tiene guardada; si no, nadie puede iniciar sesión (`invalid_device_proof`). Cambiarla es cambiar de servidor para las apps. **En desarrollo** el valor de `.env` (`http://localhost`, sin puerto) no es la dirección por la que se llega al servidor: ver `SERVER_EXTRA_ORIGINS` abajo |
| `SERVER_EXTRA_ORIGINS` | `.env.local` (`.env` la trae **vacía**) | Otras direcciones por las que se llega a **este mismo** servidor, separadas por comas y escritas igual que `DEFAULT_URI` (se normalizan igual). Solo para desarrollo: la firma con la que un dispositivo entra (D100) vale si su dirección es `DEFAULT_URI` o una de estas, y ninguna otra. En el escritorio la app llega por `http://localhost:8095` y desde el emulador de Android por `http://10.0.2.2:8095`; para cada una, la dirección que la app tiene en «Otro servidor». Ejemplo para escritorio y emulador a la vez, en `.env.local`: `DEFAULT_URI=http://localhost:8095` y `SERVER_EXTRA_ORIGINS=http://10.0.2.2:8095` (un móvil por la red de casa añade `http://192.168.x.y:8095`). **En producción se deja vacía** |
| `REGISTRATION_ALLOWED_EMAILS` | `.env.local` (`.env` la trae vacía) | Correos que pueden crear una cuenta, separados por comas. **Vacía = registro abierto.** Con contenido, los demás reciben `403 registration_closed` y `GET /v1/server` dice `registration_restricted: true`. Ver [accounts.md](../architecture/accounts.md#post-v1accountregister) |

- Se genera con `openssl rand -base64 32`. En producción la define el entorno (7.9) y entra en las copias de seguridad **aparte de la base de datos**: una copia de la base sin el secreto no permite firmar.
- **Perderla deja a la nube sin poder firmar** en las bibliotecas ya creadas (la clave cifrada no se puede abrir; el servidor lo dice con una excepción, no con datos basura). Cambiarla tiene el mismo efecto. Las bibliotecas creadas con el secreto vacío reciben su identidad la primera vez que se listan o se enlazan, si el secreto ya está definido.

### Sellar de nuevo las claves de la nube al desplegar (SEG-074, 16/10/2026)

Desde esta versión la clave privada de cada biblioteca de la nube se guarda como `v1.` más XChaCha20-Poly1305 con **el id de su biblioteca** como dato asociado: copiar el valor de una biblioteca a la fila de otra ya no lo abre. Las claves guardadas antes (`secretbox`, sin versión) se siguen leyendo, y la migración `Version20261016120000` las sella otra vez, una por una, con el mismo `CLOUD_KEY_SECRET`. **Hay que desplegar código y migración a la vez.** Pasos, en el servidor (los hace el maintainer; ningún agente entra en él):

1. **Copia antes**, fuera del VPS: un volcado de la base (`pg_dump`) y una copia del `.env.local`. Sin el secreto la copia de la base no sirve para firmar, y sin la copia no hay vuelta atrás: la migración **no se deshace** (`down` lanza una excepción).
2. Comprobar que `CLOUD_KEY_SECRET` está en `.env.local` y es el mismo de siempre (la migración lo lee del entorno solo si hay claves que sellar).
3. Desplegar el código nuevo y `php bin/console doctrine:migrations:migrate`. La migración es una sola transacción: abre y sella todas las claves antes de escribir ninguna, y **se aborta sin cambiar nada** si falta el secreto o si alguna clave no abre con él. Si no hay nada que sellar no pide el secreto.
4. Comprobarlo: `SELECT count(*) FROM cloud_library WHERE cloud_secret_key IS NOT NULL AND cloud_secret_key NOT LIKE 'v1.%'` debe dar 0, y un `push` a una biblioteca con la nube nombrada tiene que seguir firmando.
5. **Si el despliegue se queda a medias:** código nuevo con claves viejas funciona (lee las dos); la migración que falla no cambia nada. Código **viejo** con claves ya selladas de nuevo no las abre: la nube dice «no puede firmar aquí» (`noIdentity`) y no hay datos en riesgo; se arregla poniendo el código nuevo, o, si hay que volver al viejo, restaurando la copia del paso 1.

### Qué no debe llegar al registro de accesos (7.1, D77)

Dos rutas llevan un secreto en la dirección, porque es la vuelta del navegador por Google: `GET /v1/auth/google/connect?attempt=…` y `GET /v1/auth/google/callback?code=…&state=…`. **No pueden escribirse en el registro de accesos** de nginx ni de ningún proxy o CDN delante. En desarrollo lo hace `map $request_uri $loggable` en `docker/nginx/nginx.conf` con `access_log … if=$loggable` en el sitio; en producción, lo mismo (y sin guardar la cabecera `Referer` de esas rutas; el servidor ya responde `Referrer-Policy: no-referrer` y `Cache-Control: no-store`).

### Google Books a través del servidor (7.8, D83)

`GOOGLE_BOOKS_API_KEY` (en `.env.local`; `.env` la trae vacía): la clave de la API de Google Books. Con ella, las cuentas pueden buscar un ISBN por el servidor y `GET /v1/server` dice `metadata: ["google_books"]`; vacía, la ruta responde `404 metadata_unavailable`. Se crea en la consola de Google Cloud (API «Books API», clave de API restringida a esa API) y viaja siempre en la cabecera `X-Goog-Api-Key`, nunca en una dirección. Los límites y la caché, en [cloud.md](../architecture/cloud.md#buscar-libros-a-través-del-servidor-78-d83).

**Tampoco se escriben en el registro de accesos** las direcciones `/v1/metadata/…`, porque llevan el ISBN que busca una cuenta. En desarrollo ya está en `docker/nginx/nginx.conf` (`map $request_uri $loggable`, junto a las de Google); en producción hay que repetirlo.

### TMDB: películas y series por el servidor (7.8, D62)

`TMDB_API_TOKEN` (en `.env.local`; `.env` lo trae vacío): el «API Read Access Token» de la cuenta de TMDB (Ajustes › API en themoviedb.org; hay que aceptar su uso no comercial y la atribución). Con él, las cuentas pueden buscar películas y series por título y `GET /v1/server` lista `tmdb` en `metadata`; vacío, la ruta responde `404 metadata_unavailable`. Viaja siempre en la cabecera `Authorization: Bearer`. Los límites, la caché y el freno ante TMDB están en [cloud.md](../architecture/cloud.md#buscar-películas-y-series-por-título-en-tmdb-78-segunda-parte-d62).

### RAWG: videojuegos por el servidor (7.8, D62)

`RAWG_API_KEY` (en `.env.local`; `.env` la trae vacía): la clave de la API de RAWG (rawg.io/apidocs, hay que aceptar sus condiciones y la atribución). Con ella, las cuentas pueden buscar videojuegos por título y `GET /v1/server` lista `rawg` en `metadata`; vacía, la ruta responde `404 metadata_unavailable`. **RAWG solo la admite en la dirección de la llamada saliente**, no en una cabecera: por eso esa llamada se hace con un cliente HTTP sin registrador y todas sus excepciones se relanzan sin la dirección (ver [cloud.md](../architecture/cloud.md#buscar-videojuegos-por-título-en-rawg-78-tercera-parte-d62)). Si se añade un proxy o un registro de llamadas salientes, no debe guardar las direcciones de `api.rawg.io`.

### Comic Vine: cómics por el servidor (7.8, D62)

`COMIC_VINE_API_KEY` (en `.env.local`; `.env` la trae vacía): la clave de la API de Comic Vine (`comicvine.gamespot.com/api/`, hace falta una cuenta de Comic Vine; uso estrictamente no comercial). Con ella, las cuentas pueden buscar cómics por título y `GET /v1/server` lista `comic_vine` en `metadata`; vacía, la ruta responde `404 metadata_unavailable`. **Solo la admite en la dirección de la llamada saliente**, como RAWG: se pregunta con un cliente HTTP sin registrador y con las excepciones sin la dirección. Comic Vine cuenta 200 peticiones por recurso y hora; el servidor se limita a 150 por recurso (`comic_vine_search`, `comic_vine_volume`, `comic_vine_issue`). Ver [cloud.md](../architecture/cloud.md#buscar-cómics-por-título-en-comic-vine-78-quinta-parte-d62).

## Comandos habituales

| Tarea | Comando (desde `server/`) |
|---|---|
| Parar / arrancar | `make down` · `make up` |
| Consola de Symfony | `make console C="debug:router"` |
| Shell en el contenedor PHP | `make sh` |
| Migraciones | `make migration` (generar) · `make migrate` (aplicar) · `make db-test` (en test) |
| Tests | `make test` · `make test ARGS="--filter Conformance"` |
| Comprobar todo sin tocar nada | `make check` |
| Arreglar estilo y Rector | `make fix` |
| Purgar cuentas sin confirmar y códigos caducados (D49) | `make console C="app:accounts:purge"` |
| Trabajador que ejecuta la purga cada hora | `make worker` (o `make console C="messenger:consume scheduler_default -vv"`) |
| Conceder/retirar `ROLE_SUPER_ADMIN` (D51) | `make console C="app:user:super-admin correo@ejemplo.com"` (`--revoke` para retirarlo) |
| Ver el registro de actividad (D66; enseña el autor y quien subió, y `--account` busca por cualquiera): `--account`, `--library`, `--kind`, `--since`, `--until`, `--limit` | `make console C="app:activity:list --account=<id>"` |
| Bloquear/desbloquear una cuenta (D51) | `make console C="app:user:block correo@ejemplo.com"` (`--undo` para desbloquear) |
| Reconstruir el catálogo de una biblioteca de la nube desde su registro (7.4.4) | `make console C="app:catalog:rebuild <id de la biblioteca de la nube>"` |
| Reconstruir el catálogo de **todas** las bibliotecas de la nube, de una en una (7.4.6). **Se ejecuta una vez al desplegar 7.4.6**: las operaciones guardadas antes no están juzgadas y no hay catálogo | `make console C="app:catalog:rebuild --all"` |
| Prueba de carga del motor de fusión (no entra en `make check`) | `make stress BOOKS=5000 LIMIT=600`; se repite dos veces al año, ver «Revisión periódica de la carga» en [cloud.md](../architecture/cloud.md) |
| Prueba de carga con las tablas llenas (un millón de filas, su propia base de datos) | `make stress-full BOOKS=2000 LIMIT=10800`; la primera vez llena durante unas dos horas |
| Retirar el segundo factor de una cuenta (D78) | `make console C="app:user:two-factor-disable correo@ejemplo.com"` |

### El worker de tareas programadas (hito 7.1, parte 3)

`App\Scheduler\PurgeSchedule` (`symfony/scheduler`) programa la purga de cuentas cada hora, pero **no hace nada por sí solo**: solo se ejecuta mientras algo consume el transporte `scheduler_default`, que Symfony crea automáticamente a partir del atributo `#[AsSchedule]`. En desarrollo, eso significa tener `make worker` corriendo en una terminal aparte mientras se prueba la app; si no está corriendo, la purga simplemente no ocurre (y `app:accounts:purge` sigue estando ahí para lanzarla a mano). En producción (hito 7.9) este worker tendrá que ser un servicio del sistema (p. ej. systemd) que lo mantenga vivo y lo reinicie si cae.

La CI ejecuta lo mismo que `make check`, sin Docker (trabajo *Server (PHP)* en `.github/workflows/ci.yml`).

## Versiones (25/09/2026)

| Herramienta | Versión |
|---|---|
| PHP | 8.5.11 |
| Symfony | 8.1.7 |
| Doctrine ORM / DBAL bundle | 3.7 / 3.3 |
| PostgreSQL | 18.6 |
| PHPUnit / PHPStan / Rector / PHP CS Fixer | 13.3 / 2.2 / 2.6 / 3.95 |

**Renombrado a Kosmotheka (26/09/2026, D70).** El proyecto Compose, los contenedores, el volumen y las bases pasaron de `library` a `kosmotheka`. Quien tuviera el entorno montado de antes tiene que actualizar a mano sus ficheros no versionados: `docker/.env` (copiándolo de `docker/.env.dist`) y `DATABASE_URL` en `.env.local`. Después, `make down` con el `docker/.env` antiguo (o `docker rm -f db_library php_library nginx_library`), y `make build` y `make up` con el nuevo. El volumen antiguo, `library_db_library`, no se borra solo: `docker volume rm library_db_library` cuando ya no haga falta.

