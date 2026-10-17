---
title: "Setting it up with Docker"
description: "Building and starting the Kosmotheka server with Docker Compose, with PHP-FPM, nginx, PostgreSQL and the task process."
sidebar:
  order: 3
---

The same parts as [without Docker](../without-docker/), packaged. The files are in `server/docker/prod/` of the repository. **No image is published in any registry**: you build it yourself, on your machine, from the code of the version you choose. The `Dockerfile`s in `server/docker/` (without `prod`) are only for development: do not use them on a server.

What this setup does:

- The PHP image has no debugger and no development dependencies, runs as an unprivileged user (`www-data`) and carries the code inside (nothing is mounted).
- Four services: `php` (PHP-FPM), `nginx`, `db` (PostgreSQL 18, with its volume) and `worker` (the scheduled tasks, with automatic restart). Mercure is optional.
- nginx listens on **HTTP, on a local port** (by default `127.0.0.1:8080`). **HTTPS is terminated by a proxy in front**, the one the machine already has or an nginx with Certbot. The configuration does not include certificates.
- PostgreSQL publishes no port.

You need Docker with the Compose plugin (version 2).

## 1. The code and the configuration

```bash
git clone https://github.com/kosmotheka/kosmotheka.git /srv/kosmotheka
cd /srv/kosmotheka
git checkout vX.Y.Z      # the version you want
cd server/docker/prod
cp env.example .env
chmod 600 .env
```

Fill in `.env` (all the secrets are generated with `openssl`, as the file itself says):

- `POSTGRES_PASSWORD`: `openssl rand -hex 24`.
- `APP_SECRET`: `openssl rand -hex 32`.
- `CLOUD_KEY_SECRET`: `openssl rand -base64 32` (base64, not hexadecimal).
- `DEFAULT_URI` and `INVITATION_BASE_URL`: `https://your-domain`, without a trailing slash.
- `HTTP_PORT` and `HTTP_BIND`, if the local 8080 is busy.

The rest of the variables (email, Google, search services…) are added to the same `.env`: they reach the PHP containers as they are. Each one is in [the configuration](../configuration/). **Keep a copy of `.env` outside the machine**: without `CLOUD_KEY_SECRET` the copy of the database is no use for signing.

## 2. Build and start

```bash
docker compose build
docker compose up -d
docker compose run --rm php bin/console doctrine:migrations:migrate --no-interaction
```

The first build takes a few minutes. Check that everything is running:

```bash
docker compose ps
curl -s http://127.0.0.1:8080/v1/server
```

It must answer with the server's JSON. To have several servers on the same machine (or a test one next to it), change `COMPOSE_PROJECT_NAME` and the port in `.env`.

## 3. HTTPS: the proxy in front

Any proxy with TLS works; this is an nginx with Certbot on the machine itself (outside Docker):

```nginx
server {
    listen 80;
    server_name your-domain;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 300;
        client_max_body_size 16m;
    }

    # These two addresses carry a secret or what an account searches for:
    # they are not written to this proxy's access log.
    location ~ ^/v1/(auth/google/(connect|callback)|metadata/) {
        access_log off;
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```bash
sudo nginx -t && sudo systemctl reload nginx
sudo certbot --nginx -d your-domain
curl -s https://your-domain/v1/server
```

The nginx inside the container already leaves those addresses out of its own log. If you use another proxy or a CDN, they must not log them either. The `TRUSTED_PROXIES` variable already comes with `REMOTE_ADDR` to trust what that proxy sends; do not open the container's port to the internet (that is why it listens only on `127.0.0.1`).

## 4. Instant notices (optional)

Generate two different keys with `openssl rand -hex 32`, put them in `.env` next to `MERCURE_URL=http://mercure/.well-known/mercure`:

```
MERCURE_URL=http://mercure/.well-known/mercure
MERCURE_PUBLISHER_JWT_KEY=...
MERCURE_SUBSCRIBER_JWT_KEY=...
```

and start with the profile: `docker compose --profile mercure up -d`. nginx already forwards `/.well-known/mercure` to the hub. For the proxy in front, add the same `location /.well-known/mercure` as in the [guide without Docker](../without-docker/#10-instant-notices-optional), pointing to the local port, without buffering and with a long read timeout.

## Logs

Everything goes to the output of each container: `docker compose logs -f php nginx worker`.

## Copies, updating and problems

- **Copy:** the dump comes out of the database container and is encrypted on the fly, as in [the backups](../backups/): `docker compose exec -T db pg_dump -U kosmotheka --format=custom kosmotheka | age -R /etc/kosmotheka/backup-recipient.txt > db.dump.age`, and the same for `.env` instead of `.env.local`. To restore onto an empty database: `age -d -i clave.txt db.dump.age | docker compose exec -T db pg_restore -U kosmotheka --no-owner --dbname=kosmotheka`. (The Docker part of these two commands has not been tested; the encryption and `pg_restore` have, see the backups.)
- **Updating:** copy; `git checkout` of the new version; `docker compose build`; `docker compose run --rm php bin/console doctrine:migrations:migrate --no-interaction`; `docker compose up -d`. The cache is already prepared in the image. What is said in [updating](../updating/) still applies: first the app in the stores, if the notes ask for it.
- **A Symfony command:** `docker compose run --rm php bin/console …`.
- **Emptying and starting from zero:** `docker compose down -v` deletes the containers **and the database**. It is irreversible.
