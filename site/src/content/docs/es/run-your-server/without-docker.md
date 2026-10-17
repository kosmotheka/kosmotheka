---
title: Montarlo sin Docker
description: Instalar el servidor de Kosmotheka en una máquina Linux con PHP, PostgreSQL y nginx, paso a paso.
sidebar:
  order: 2
---

Así funciona el servidor oficial. Los pasos suponen una máquina Debian o Ubuntu reciente con acceso de administrador; en otra distribución cambian los nombres de los paquetes y las rutas, no las ideas. En los ejemplos, `tu-dominio` es el nombre de tu servidor y `/srv/kosmotheka` el sitio donde pones el código: cámbialos por los tuyos.

## 1. Lo que hay que instalar

| Pieza | Versión | Para qué |
|---|---|---|
| PHP | 8.5 o superior, con PHP-FPM | El servidor es una aplicación Symfony 8 |
| PostgreSQL | 18 | La base de datos. Doctrine genera SQL para la versión que se le dice (`serverVersion=18`) |
| nginx | cualquiera reciente | Sirve HTTPS y pasa las peticiones a PHP-FPM |
| Composer | 2 | Instala las dependencias de PHP |
| Certbot | cualquiera | El certificado HTTPS (Let's Encrypt) |

Extensiones de PHP: `pdo_pgsql`, `sodium` (las firmas y el cifrado de las claves de la nube), `mbstring`, `intl`, `xml` (incluye `simplexml`), `curl`, `ctype`, `iconv` y `opcache`. Las versiones de PHP y PostgreSQL que traen las distribuciones suelen ser más viejas: usa el repositorio oficial de PostgreSQL y, para PHP, uno que lleve la 8.5 (en Debian y Ubuntu, el de Ondřej Surý).

Si en la máquina conviven varias versiones de PHP, **llama a `php8.5` explícitamente** en todas las órdenes de esta guía (`php8.5 bin/console …`, `php8.5 /usr/bin/composer …`). Un `php` a secas puede ser otra versión y fallar de formas confusas.

## 2. La base de datos

Un usuario y una base propios, solo para Kosmotheka:

```bash
sudo -u postgres psql <<'SQL'
CREATE ROLE kosmotheka LOGIN PASSWORD 'pon-aqui-una-contraseña-larga';
CREATE DATABASE kosmotheka OWNER kosmotheka ENCODING 'UTF8';
SQL
```

Genera la contraseña con `openssl rand -hex 24` en vez de inventarla. PostgreSQL debe escuchar solo en `127.0.0.1` (es lo que hace por defecto): no abras su puerto a internet.

## 3. El código

```bash
sudo mkdir -p /srv/kosmotheka && sudo chown "$USER": /srv/kosmotheka
git clone https://github.com/kosmotheka/kosmotheka.git /srv/kosmotheka
cd /srv/kosmotheka
git checkout vX.Y.Z    # la etiqueta de la versión que quieres; mira las versiones publicadas
```

El servidor es la carpeta `server/`. Necesita tener al lado la carpeta `spec/` (el formato de las operaciones, que lee al arrancar), así que se clona el repositorio entero y no se mueve una carpeta sin la otra.

## 4. La configuración: `.env.local`

Toda la configuración va en `server/.env.local`, un fichero que no está en git. Crea uno mínimo (todas las variables, una por una, están en [la configuración](/es/run-your-server/configuration/)):

```bash
cd /srv/kosmotheka/server
umask 077
cat > .env.local <<ENV
APP_ENV=prod
APP_SECRET=$(openssl rand -hex 32)
DATABASE_URL="postgresql://kosmotheka:LA_CONTRASEÑA@127.0.0.1:5432/kosmotheka?serverVersion=18&charset=utf8"
DEFAULT_URI=https://tu-dominio
INVITATION_BASE_URL=https://tu-dominio
CLOUD_KEY_SECRET=$(openssl rand -base64 32)
TRUSTED_PROXIES=127.0.0.1
ENV
chmod 640 .env.local
```

- `CLOUD_KEY_SECRET` es en **base64** (`openssl rand -base64 32`), no hexadecimal.
- **Guarda una copia de `.env.local` fuera de la máquina.** Sin `CLOUD_KEY_SECRET`, una copia de la base de datos no permite que el servidor firme en las bibliotecas ya creadas.
- `DEFAULT_URI` es la dirección pública, sin barra final, y es la misma que escribirás en la app. Cambiarla es, para las apps, cambiar de servidor.
- Para que el correo, Google y los buscadores funcionen, añade sus variables después (ver [la configuración](/es/run-your-server/configuration/)).

## 5. Dependencias y migraciones

```bash
cd /srv/kosmotheka/server
php8.5 /usr/bin/composer install --no-dev --optimize-autoloader
php8.5 bin/console doctrine:migrations:migrate --no-interaction
php8.5 bin/console cache:clear
```

Las migraciones crean todas las tablas. Si falla alguna, no sigas: lee el error (casi siempre es la conexión a la base o una extensión que falta).

PHP-FPM tiene que poder escribir en `server/var/` (la caché y los registros). Dale esa carpeta al usuario con el que corre el *pool* (en Debian y Ubuntu, `www-data`):

```bash
sudo chown -R www-data: /srv/kosmotheka/server/var
```

## 6. PHP-FPM

Un *pool* propio mantiene la memoria de Kosmotheka separada de lo demás. En `/etc/php/8.5/fpm/pool.d/kosmotheka.conf`:

```ini
[kosmotheka]
user = www-data
group = www-data
listen = /run/php/php8.5-fpm-kosmotheka.sock
listen.owner = www-data
listen.group = www-data
pm = ondemand
pm.max_children = 8
pm.process_idle_timeout = 30s
php_admin_value[memory_limit] = 256M
```

Con una máquina pequeña, `pm.max_children` bajo evita que un pico se coma la memoria. Reinicia: `sudo systemctl restart php8.5-fpm`.

## 7. nginx

Dos ficheros. El primero va **fuera** del sitio, en `/etc/nginx/conf.d/kosmotheka-log.conf`, porque `map` solo vale en el nivel `http`:

```nginx
# Qué direcciones no se escriben nunca en el registro de accesos:
# la vuelta de Google lleva un secreto, y /v1/metadata/ lleva lo que busca una cuenta.
map $request_uri $kosmotheka_loggable {
    ~^/v1/auth/google/(connect|callback)(\?|$) 0;
    ~^/v1/metadata/ 0;
    default 1;
}
```

El segundo es el sitio, en `/etc/nginx/sites-available/kosmotheka` (enlázalo en `sites-enabled`):

```nginx
server {
    listen 80;
    server_name tu-dominio;
    root /srv/kosmotheka/server/public;

    # Avisos al momento (opcional, ver más abajo). Si no instalas Mercure, quita este bloque.
    # location /.well-known/mercure {
    #     proxy_pass http://127.0.0.1:3000;
    #     proxy_http_version 1.1;
    #     proxy_set_header Connection "";
    #     proxy_set_header Host $host;
    #     proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    #     proxy_set_header X-Forwarded-Proto $scheme;
    #     proxy_buffering off;
    #     proxy_cache off;
    #     proxy_read_timeout 24h;
    # }

    # Es una API: todo va a Symfony.
    location / {
        try_files $uri /index.php$is_args$args;
    }

    location ~ ^/index\.php(/|$) {
        fastcgi_pass unix:/run/php/php8.5-fpm-kosmotheka.sock;
        fastcgi_split_path_info ^(.+\.php)(/.*)$;
        include fastcgi_params;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
        fastcgi_param DOCUMENT_ROOT $document_root;
        internal;
    }

    location ~ \.php$ {
        return 404;
    }

    client_max_body_size 16m;
    access_log /var/log/nginx/kosmotheka_access.log combined if=$kosmotheka_loggable;
    error_log /var/log/nginx/kosmotheka_error.log warn;
}
```

Si hay un proxy o una CDN delante de nginx, tampoco puede registrar esas direcciones. Comprueba y recarga:

```bash
sudo nginx -t && sudo systemctl reload nginx
```

## 8. HTTPS con Certbot

Con el nombre ya apuntando a la máquina y el puerto 80 abierto:

```bash
sudo certbot --nginx -d tu-dominio
```

Certbot añade el certificado, redirige el puerto 80 al 443 y renueva solo (comprueba que existe su temporizador: `systemctl list-timers | grep certbot`). Abre en el cortafuegos solo el 80 y el 443 (y el del SSH).

Prueba el servidor desde fuera:

```bash
curl -s https://tu-dominio/v1/server
```

Debe responder un JSON con lo que ofrece el servidor (`invitations`, `events`, `sign_in_with_google`…). Si responde, ya puedes escribir `https://tu-dominio` en «Otro servidor» de la app.

## 9. El proceso de tareas programadas

El servidor tiene una tarea cada hora (borra las cuentas sin confirmar y los códigos caducados) que **no hace nada por sí sola**: solo corre mientras algo consume su cola. Es un servicio de systemd, en `/etc/systemd/system/kosmotheka-worker.service`:

```ini
[Unit]
Description=Tareas programadas de Kosmotheka
After=network.target postgresql.service

[Service]
User=www-data
WorkingDirectory=/srv/kosmotheka/server
ExecStart=/usr/bin/php8.5 bin/console messenger:consume scheduler_default --time-limit=3600 --memory-limit=128M
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now kosmotheka-worker
systemctl status kosmotheka-worker
```

El `--time-limit` hace que se reinicie a sí mismo cada hora, con la memoria limpia; `Restart=always` lo levanta de nuevo. Si no quieres el servicio, puedes lanzar la purga a mano de vez en cuando: `php8.5 bin/console app:accounts:purge`.

## 10. Avisos al momento (opcional)

Mercure es un componente aparte que permite avisar a las apps abiertas de que hay algo nuevo. Sin él todo funciona igual: las apps se ponen al día al abrirlas. El servidor lo dice en `/v1/server` (`events: false`).

Si lo quieres, instala el [hub de Mercure](https://mercure.rocks/docs/hub/install) (versión 1.0) como servicio de systemd, escuchando solo en `127.0.0.1:3000`, y:

1. Genera **dos claves distintas**: `openssl rand -hex 32` para el publicador (solo el servidor la usa) y otra para el suscriptor (con ella se firman los tokens que reciben las apps). Una no puede publicar con la otra.
2. En el hub: ambas claves con algoritmo HS256, `MERCURE_TRUSTED_ISSUERS=urn:kosmotheka:server`, y la directiva `resource_identifier http://127.0.0.1:3000/.well-known/mercure` (la audiencia de los tokens, fija).
3. En `.env.local`: `MERCURE_URL=http://127.0.0.1:3000/.well-known/mercure` (la misma dirección que `resource_identifier`), `MERCURE_PUBLISHER_JWT_KEY` y `MERCURE_SUBSCRIBER_JWT_KEY`.
4. Quita los comentarios del bloque `/.well-known/mercure` de nginx.
5. `php8.5 bin/console cache:clear` y reinicia PHP-FPM.

> **Por confirmar:** esta parte está sacada de cómo funciona el desarrollo (donde el hub corre en un contenedor) y no se ha probado con el hub instalado como servicio. Si no la necesitas, sáltatela.

## Después

Con esto el servidor ya funciona. Faltan [las copias de seguridad](/es/run-your-server/backups/), que no son opcionales, y configurar [el correo, Google y los buscadores](/es/run-your-server/configuration/) según lo que quieras ofrecer.
