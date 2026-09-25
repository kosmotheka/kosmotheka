# Servidor: entorno de desarrollo

> El servidor Symfony (`server/`) corre en Docker, en su propio proyecto de Docker Compose, separado del contenedor Distrobox de la app. No hace falta PHP en el sistema: todo se lanza con `make` desde `server/`.

## Qué hay en `server/`

| Ruta | Qué es |
|---|---|
| `docker/` | Docker Compose del servidor: PHP-FPM, nginx y PostgreSQL |
| `src/Operation/` | Validación de operaciones en PHP: la misma que `crates/domain`, contra los mismos casos de `spec/` |
| `tests/` | PHPUnit; `tests/Operation/` ejecuta los casos de conformidad |
| `Makefile` | Todos los comandos (`make help`) |
| `AGENTS.md` | Instrucciones para agentes específicas del servidor |

## Arrancar

```bash
cd server/docker
cp .env.dist .env        # solo la primera vez; .env no se versiona
cd ..
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
| `db_library` | PostgreSQL 18 | 5433 | 172.194.0.10 |
| `php_library` | PHP 8.5-FPM, Composer, Xdebug (puerto 9001) | — | 172.194.0.11 |
| `nginx_library` | nginx, delante de PHP | 8095 | 172.194.0.12 |

- Subred `172.194.0.0/24`; el proyecto Compose se llama `library`.
- El código de `server/` se monta en `/var/www/library.local`; `spec/` se monta **en solo lectura** en `/var/www/spec`.
- PostgreSQL guarda los datos en el volumen `library_db_library`, no en el árbol del repo.

## Bases de datos

| Entorno | Base | Usuario | Dónde se configura |
|---|---|---|---|
| dev | `library` | `library` | `server/.env` y `docker/.env` |
| test | `library_test` | `test` | `server/.env.test` (Doctrine añade `_test` al nombre) |

`library_test` y su usuario los crea `docker/postgres/init/01-test-database.sql` la **primera vez** que arranca el volumen. Si el volumen ya existía sin ella, bórralo (`docker volume rm library_db_library`, con los contenedores parados) o ejecuta ese SQL a mano.

Las credenciales de desarrollo y test solo existen dentro del contenedor y no son secretas. Las de producción irán en `.env.local` en el VPS, nunca en el repo.

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
