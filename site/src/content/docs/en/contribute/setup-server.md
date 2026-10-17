---
title: "Setting up the environment: the server"
description: How to start the Symfony server in Docker, its databases, the development mail, the test data and the checks.
sidebar:
  order: 3
---

The server (`server/`) is optional: the app works without it. This page is for those who want to work on it. **You do not need PHP on your machine**: everything runs in Docker and is started with `make` from `server/`. If what you want is to *host* a server, read [Run your server](../../run-your-server/) instead.

## What you need

Docker with Docker Compose, and your user must be able to use it without `sudo`. Nothing else.

The server is a separate Docker Compose project, with its own ports and its own network, so that it does not clash with other projects. It is **for development only**: nginx runs as root and the PHP container gives `sudo` without a password. A real server uses the image in `server/docker/prod/`.

## What it contains

| Container | What it is | Port on your machine |
|---|---|---|
| `db_kosmotheka` | PostgreSQL 18 | 5433 |
| `php_kosmotheka` | PHP 8.5-FPM, Composer and Xdebug | none |
| `nginx_kosmotheka` | nginx, in front of PHP: this is where the API answers | 8095 |
| `mailpit_kosmotheka` | Mailpit: captures all development mail | 8097 (web) |
| `mercure_kosmotheka` | Mercure hub, for instant notices | none (nginx exposes it) |

The code in `server/` is mounted inside the container, and `spec/` is mounted **read-only**: it is the specification of the operations that the app and the server share.

## Starting

The exact steps (which file is copied and which variables are filled in) are in [`docs/development/server.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/server.md). In short:

1. In `server/docker/`, copy `.env.dist` to `.env`. **`.env` is not versioned.**
2. Generate **two different keys** with `openssl rand -hex 32` for the notices hub (`MERCURE_PUBLISHER_JWT_KEY` and `MERCURE_SUBSCRIBER_JWT_KEY`) and put them in `docker/.env`. Docker Compose refuses to start without them.
3. In `server/`, create `.env.local`, which **is not versioned either**. It holds the database connection string (`DATABASE_URL`), an `APP_SECRET` that each installation generates (without it, the server does not issue codes by email) and the same two Mercure keys, plus `MERCURE_URL`.
4. `make build` (only the first time, or if a Dockerfile changes), `make up` and `make install` (Composer, inside the container).
5. `make check` to confirm that everything passes.

The API answers at `http://localhost:8095`.

`make help` lists all the commands. Some for daily use:

| Task | Command (from `server/`) |
|---|---|
| Stop and start | `make down` · `make up` |
| Symfony console | `make console C="debug:router"` |
| Enter the PHP container | `make sh` |
| Migrations | `make migration` (generates one) · `make migrate` (applies) · `make db-test` (on the test database) |
| Tests | `make test` · `make test ARGS="--filter Conformance"` |
| Check everything without changing anything | `make check` |
| Fix style and Rector | `make fix` |
| Scheduled tasks (the purge of unconfirmed accounts) | `make worker` |

**About `.env`.** The `.env` in `server/` goes in the repository and **carries no credentials**, only default values and empty slots. Anything secret goes in `.env.local`. No `APP_SECRET` is written in the repository either, not even as an example.

## The databases

| Environment | Database | Where it is configured |
|---|---|---|
| Development | `kosmotheka` | `server/.env.local` and `docker/.env` |
| Tests | `kosmotheka_test` | `server/.env.test`, which **is** in the repository |

`.env.test` must be versioned: Symfony does not load `.env.local` when running the tests, and without it a fresh clone would not pass them. It carries no secret: each run of the tests makes up its own.

The test database and its user are created by an SQL script **the first time** the PostgreSQL volume starts. If the volume already existed without it, delete it with the containers stopped (`docker volume rm kosmotheka_db_kosmotheka`), or run that SQL by hand.

Every schema change goes through a migration (`make migration`, then `make migrate`), never through `doctrine:schema:update`.

## Test data: the fixtures

```bash
cd server
make fixtures
```

**It empties the development database** and fills it with test scenarios: families with their libraries in the cloud, and also the hostile attempts that the server must reject (someone who knows the ids of another library, for example). Everything goes in through the same services that the API uses, not with `INSERT`, so the database only contains what the rules let in. Afterwards you can open it with any PostgreSQL client (port 5433) and see what the server stores. The same scenarios run in the tests. The detail of each case is in [`docs/development/fixtures.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/fixtures.md).

`make app-fixtures` does the same and also prepares an app data folder for each character, to open them with `KOSMOTHEKA_DATA_DIR` (see [Setting up the environment: the app](../setup-app/)).

## Mail in development

The server sends email (confirmation codes, password recovery, invitations). In development, the transport (`MAILER_DSN`, in `.env.local`) points to Mailpit, which **captures everything and sends nothing outside**. Open `http://localhost:8097` to read them. In the tests the transport is null and the tests read the messages from the Symfony log.

## External services, always optional

Each one is switched on with a variable in `.env.local`. **Empty, the service stays off** and the server keeps working; what depended on it answers with a clear error (`404 …_unavailable`) and `GET /v1/server` announces it. Never put a real value in a versioned file.

| What it switches on | Variable |
|---|---|
| Signing in with Google | `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET` (guide: [`google-oauth.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/google-oauth.md)) |
| Searching books in Google Books through the server | `GOOGLE_BOOKS_API_KEY` |
| Films and series (TMDB) | `TMDB_API_TOKEN` |
| Video games (RAWG) | `RAWG_API_KEY` |
| Comics (Comic Vine) | `COMIC_VINE_API_KEY` |
| Board games and role-playing games (BoardGameGeek) | `BGG_API_TOKEN` (and the daily cap `BGG_DAILY_CALL_CAP`) |
| The identity of each cloud library | `CLOUD_KEY_SECRET` |
| Invitation links | `INVITATION_BASE_URL` |
| Instant notices (Mercure) | `MERCURE_URL` |
| Restricting who can create an account | `REGISTRATION_ALLOWED_EMAILS` |
| Second factor | `TWO_FACTOR_ENABLED` |

Each data provider has its own terms of use (almost all require non-commercial use and an attribution): [`docs/development/servicios-externos.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/servicios-externos.md) explains them (in Spanish). The RAWG and Comic Vine keys only accept going in the address of the call, so the code handles them with care: it uses an HTTP client without a logger and throws the exceptions without the address. Do not log the server's outgoing calls.

The tests **never** use these services: they read recorded responses.

## The checks

`make check` runs four tools that do not overlap, and takes a few minutes:

| Tool | What it covers |
|---|---|
| php-cs-fixer | Style (`@Symfony` and `@Symfony:risky`) |
| Rector | Modernising the syntax and the quality rules of Symfony and Doctrine |
| PHPStan | Correctness, at the highest level (`max`), **with no baseline file**: where a value may be missing, it is narrowed, not silenced |
| PHPUnit | Behaviour. It includes the conformance cases of `spec/` |

`make fix` fixes style and Rector, and then checks. **Do not pipe it into another command** (`make fix | tail`): the exit code you would see would be that of the second command and would hide the failures.

The server's code rules (where each class goes, errors, Symfony conventions) are in [`server/AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/server/AGENTS.md). Next step: [How to contribute](../how-to-contribute/).
