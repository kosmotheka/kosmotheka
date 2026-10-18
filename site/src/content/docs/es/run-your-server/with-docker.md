---
title: Montarlo con Docker
description: "Construir y arrancar el servidor de Kosmotheka con Docker Compose, con PHP-FPM, nginx, PostgreSQL y el proceso de tareas."
sidebar:
  order: 3
---

Las mismas piezas que [sin Docker](/es/run-your-server/without-docker/), empaquetadas. Los archivos están en `server/docker/prod/` del repositorio. **No se publica ninguna imagen en un registro**: la construyes tú, en tu máquina, a partir del código de la versión que elijas. Los `Dockerfile` de `server/docker/` (sin `prod`) son solo para desarrollo: no los uses en un servidor.

Lo que hace esta configuración:

- La imagen de PHP no lleva depurador ni dependencias de desarrollo, corre como usuario sin privilegios (`www-data`) y lleva el código dentro (no se monta nada).
- Cuatro servicios: `php` (PHP-FPM), `nginx`, `db` (PostgreSQL 18, con su volumen) y `worker` (las tareas programadas, con reinicio automático). Mercure es opcional.
- nginx escucha en **HTTP, en un puerto local** (por defecto `127.0.0.1:8080`). **El HTTPS lo termina un proxy delante**, el que ya tenga la máquina o un nginx con Certbot. La configuración no incluye certificados.
- PostgreSQL no publica ningún puerto.

Necesitas Docker con el plugin Compose (versión 2).

## 1. El código y la configuración

```bash
git clone https://github.com/kosmotheka/kosmotheka.git /srv/kosmotheka
cd /srv/kosmotheka
git checkout vX.Y.Z      # la versión que quieres
cd server/docker/prod
cp env.example .env
chmod 600 .env
```

Rellena `.env` (todos los secretos se generan con `openssl`, como dice el propio archivo):

- `POSTGRES_PASSWORD`: `openssl rand -hex 24`.
- `APP_SECRET`: `openssl rand -hex 32`.
- `CLOUD_KEY_SECRET`: `openssl rand -base64 32` (base64, no hexadecimal).
- `DEFAULT_URI` e `INVITATION_BASE_URL`: `https://tu-dominio`, sin barra final.
- `HTTP_PORT` y `HTTP_BIND`, si el 8080 local está ocupado.

El resto de variables (correo, Google, buscadores…) se añaden al mismo `.env`: llegan tal cual a los contenedores de PHP. Cada una está en [la configuración](/es/run-your-server/configuration/). **Guarda una copia de `.env` fuera de la máquina**: sin `CLOUD_KEY_SECRET` la copia de la base no sirve para firmar.

## 2. Construir y arrancar

```bash
docker compose build
docker compose up -d
docker compose run --rm php bin/console doctrine:migrations:migrate --no-interaction
```

La primera construcción tarda unos minutos. Comprueba que está todo en marcha:

```bash
docker compose ps
curl -s http://127.0.0.1:8080/v1/server
```

Debe responder el JSON del servidor. Para tener varios servidores en la misma máquina (o uno de prueba al lado), cambia `COMPOSE_PROJECT_NAME` y el puerto en `.env`.

## 3. HTTPS: el proxy de delante

Cualquier proxy con TLS sirve; este es un nginx con Certbot en la propia máquina (fuera de Docker):

```nginx
server {
    listen 80;
    server_name tu-dominio;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 300;
        client_max_body_size 16m;
    }

    # Estas dos direcciones llevan un secreto o lo que busca una cuenta:
    # no se escriben en el registro de accesos de este proxy.
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
sudo certbot --nginx -d tu-dominio
curl -s https://tu-dominio/v1/server
```

El nginx del contenedor ya omite esas direcciones de su propio registro. Si usas otro proxy o una CDN, no deben registrarlas tampoco. La variable `TRUSTED_PROXIES` ya viene con `REMOTE_ADDR` para fiarse de lo que mande ese proxy; no abras el puerto del contenedor a internet (por eso escucha solo en `127.0.0.1`).

## 4. Avisos al momento (opcional)

Genera dos claves distintas con `openssl rand -hex 32`, ponlas en `.env` junto a `MERCURE_URL=http://mercure/.well-known/mercure`:

```
MERCURE_URL=http://mercure/.well-known/mercure
MERCURE_PUBLISHER_JWT_KEY=...
MERCURE_SUBSCRIBER_JWT_KEY=...
```

y arranca con el perfil: `docker compose --profile mercure up -d`. nginx ya reenvía `/.well-known/mercure` al hub. Para el proxy de delante, añade la misma `location /.well-known/mercure` que en la [guía sin Docker](/es/run-your-server/without-docker/#10-avisos-al-momento-opcional) apuntando al puerto local, sin búfer y con lectura larga.

## Registros

Todo va a la salida de cada contenedor: `docker compose logs -f php nginx worker`.

## Copias, actualizar y problemas

- **Copia:** el volcado sale del contenedor de la base y se cifra al vuelo, como en [las copias](/es/run-your-server/backups/): `docker compose exec -T db pg_dump -U kosmotheka --format=custom kosmotheka | age -R /etc/kosmotheka/backup-recipient.txt > db.dump.age`, y lo mismo para `.env` en lugar de `.env.local`. Para restaurar sobre una base vacía: `age -d -i clave.txt db.dump.age | docker compose exec -T db pg_restore -U kosmotheka --no-owner --dbname=kosmotheka`. (La parte de Docker de estas dos órdenes no se ha ensayado; el cifrado y `pg_restore` sí, ver las copias.)
- **Actualizar:** copia; `git checkout` de la versión nueva; `docker compose build`; `docker compose run --rm php bin/console doctrine:migrations:migrate --no-interaction`; `docker compose up -d`. La caché ya viene preparada en la imagen. Sigue valiendo lo de [actualizar](/es/run-your-server/updating/): primero la app en las tiendas si las notas lo piden.
- **Un comando de Symfony:** `docker compose run --rm php bin/console …`.
- **Vaciar y empezar de cero:** `docker compose down -v` borra los contenedores **y la base de datos**. Es irreversible.
