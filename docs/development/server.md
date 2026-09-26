# Servidor: entorno de desarrollo

> El servidor Symfony (`server/`) corre en Docker, en su propio proyecto de Docker Compose, separado del contenedor Distrobox de la app. No hace falta PHP en el sistema: todo se lanza con `make` desde `server/`.

## Qué hay en `server/`

| Ruta | Qué es |
|---|---|
| `docker/` | Docker Compose del servidor: PHP-FPM, nginx y PostgreSQL |
| `src/` | Código, en carpetas por rol (`Dto/`, `Enum/`, `Validator/`…); ver las convenciones en `server/AGENTS.md` |
| `tests/` | PHPUnit, espejo de `src/`; `tests/Validator/` ejecuta los casos de conformidad de `spec/` |
| `Makefile` | Todos los comandos (`make help`) |
| `AGENTS.md` | Instrucciones para agentes específicas del servidor |

## Arrancar

```bash
cd server/docker
cp .env.dist .env        # solo la primera vez; .env no se versiona
cd ..
cat > .env.local <<'ENV'  # solo la primera vez; tampoco se versiona
DATABASE_URL="postgresql://kosmotheka:kosmotheka@db_kosmotheka:5432/kosmotheka?serverVersion=18&charset=utf8"
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

- Subred `172.194.0.0/24`; el proyecto Compose se llama `kosmotheka`.
- El código de `server/` se monta en `/var/www/kosmotheka.local`; `spec/` se monta **en solo lectura** en `/var/www/spec`.
- PostgreSQL guarda los datos en el volumen `kosmotheka_db_kosmotheka`, no en el árbol del repo.

## Bases de datos

| Entorno | Base | Usuario | Dónde se configura |
|---|---|---|---|
| dev | `kosmotheka` | `kosmotheka` | `server/.env.local` y `docker/.env` (ninguno se versiona) |
| test | `kosmotheka_test` | `test` | `server/.env.test` (Doctrine añade `_test` al nombre) |

`kosmotheka_test` y su usuario los crea `docker/postgres/init/01-test-database.sql` la **primera vez** que arranca el volumen. Si el volumen ya existía sin ella, bórralo (`docker volume rm kosmotheka_db_kosmotheka`, con los contenedores parados) o ejecuta ese SQL a mano.

Como en Riddle, `.env` no lleva credenciales: las de desarrollo van en `.env.local`. `.env.test` sí se versiona, porque Symfony no carga `.env.local` en test y sin él un clon recién hecho no pasaría la suite; su usuario `test` solo existe dentro del contenedor y solo tiene acceso a `kosmotheka_test`. Las de producción irán en `.env.local` en el VPS, nunca en el repo.

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

