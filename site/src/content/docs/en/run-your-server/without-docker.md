---
title: Setting it up without Docker
description: Installing the Kosmotheka server on a Linux machine with PHP, PostgreSQL and nginx, step by step.
sidebar:
  order: 2
---

This is how the official server works. The steps assume a recent Debian or Ubuntu machine with administrator access; on another distribution the package names and paths change, not the ideas. In the examples, `your-domain` is the name of your server and `/srv/kosmotheka` the place where you put the code: change them for yours.

## 1. What to install

| Part | Version | What for |
|---|---|---|
| PHP | 8.5 or later, with PHP-FPM | The server is a Symfony 8 application |
| PostgreSQL | 18 | The database. Doctrine generates SQL for the version it is told (`serverVersion=18`) |
| nginx | any recent one | Serves HTTPS and passes the requests to PHP-FPM |
| Composer | 2 | Installs the PHP dependencies |
| Certbot | any | The HTTPS certificate (Let's Encrypt) |

PHP extensions: `pdo_pgsql`, `sodium` (the signatures and the encryption of the cloud keys), `mbstring`, `intl`, `xml` (includes `simplexml`), `curl`, `ctype`, `iconv` and `opcache`. The PHP and PostgreSQL versions that come with the distributions are usually older: use the official PostgreSQL repository and, for PHP, one that carries 8.5 (on Debian and Ubuntu, the one by Ondřej Surý).

If several PHP versions live on the machine, **call `php8.5` explicitly** in all the commands in this guide (`php8.5 bin/console …`, `php8.5 /usr/bin/composer …`). A bare `php` may be another version and fail in confusing ways.

## 2. The database

Its own user and database, only for Kosmotheka:

```bash
sudo -u postgres psql <<'SQL'
CREATE ROLE kosmotheka LOGIN PASSWORD 'put-a-long-password-here';
CREATE DATABASE kosmotheka OWNER kosmotheka ENCODING 'UTF8';
SQL
```

Generate the password with `openssl rand -hex 24` instead of making one up. PostgreSQL must listen only on `127.0.0.1` (it does so by default): do not open its port to the internet.

## 3. The code

```bash
sudo mkdir -p /srv/kosmotheka && sudo chown "$USER": /srv/kosmotheka
git clone https://github.com/kosmotheka/kosmotheka.git /srv/kosmotheka
cd /srv/kosmotheka
git checkout vX.Y.Z    # the tag of the version you want; see the published versions
```

The server is the `server/` folder. It needs the `spec/` folder next to it (the format of the operations, which it reads on startup), so clone the whole repository and do not move one folder without the other.

## 4. The configuration: `.env.local`

All the configuration goes in `server/.env.local`, a file that is not in git. Create a minimal one (all the variables, one by one, are in [the configuration](../configuration/)):

```bash
cd /srv/kosmotheka/server
umask 077
cat > .env.local <<ENV
APP_ENV=prod
APP_SECRET=$(openssl rand -hex 32)
DATABASE_URL="postgresql://kosmotheka:THE_PASSWORD@127.0.0.1:5432/kosmotheka?serverVersion=18&charset=utf8"
DEFAULT_URI=https://your-domain
INVITATION_BASE_URL=https://your-domain
CLOUD_KEY_SECRET=$(openssl rand -base64 32)
TRUSTED_PROXIES=127.0.0.1
ENV
chmod 640 .env.local
```

- `CLOUD_KEY_SECRET` is in **base64** (`openssl rand -base64 32`), not hexadecimal.
- **Keep a copy of `.env.local` outside the machine.** Without `CLOUD_KEY_SECRET`, a copy of the database does not let the server sign for the libraries already created.
- `DEFAULT_URI` is the public address, without a trailing slash, and it is the same one you will write in the app. Changing it is, for the apps, changing server.
- For email, Google and the search services to work, add their variables afterwards (see [the configuration](../configuration/)).

## 5. Dependencies and migrations

```bash
cd /srv/kosmotheka/server
php8.5 /usr/bin/composer install --no-dev --optimize-autoloader
php8.5 bin/console doctrine:migrations:migrate --no-interaction
php8.5 bin/console cache:clear
```

The migrations create all the tables. If one fails, do not go on: read the error (almost always it is the connection to the database or a missing extension).

PHP-FPM must be able to write to `server/var/` (the cache and the logs). Give that folder to the user that runs the pool (on Debian and Ubuntu, `www-data`):

```bash
sudo chown -R www-data: /srv/kosmotheka/server/var
```

## 6. PHP-FPM

A pool of its own keeps Kosmotheka's memory separate from everything else. In `/etc/php/8.5/fpm/pool.d/kosmotheka.conf`:

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

On a small machine, a low `pm.max_children` stops a spike from using up the memory. Restart: `sudo systemctl restart php8.5-fpm`.

## 7. nginx

Two files. The first goes **outside** the site, in `/etc/nginx/conf.d/kosmotheka-log.conf`, because `map` only works at the `http` level:

```nginx
# Which addresses are never written to the access log:
# Google's return address carries a secret, and /v1/metadata/ carries what an account searches for.
map $request_uri $kosmotheka_loggable {
    ~^/v1/auth/google/(connect|callback)(\?|$) 0;
    ~^/v1/metadata/ 0;
    default 1;
}
```

The second is the site, in `/etc/nginx/sites-available/kosmotheka` (link it in `sites-enabled`):

```nginx
server {
    listen 80;
    server_name your-domain;
    root /srv/kosmotheka/server/public;

    # Instant notices (optional, see below). If you do not install Mercure, remove this block.
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

    # It is an API: everything goes to Symfony.
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

If there is a proxy or a CDN in front of nginx, it must not log those addresses either. Check and reload:

```bash
sudo nginx -t && sudo systemctl reload nginx
```

## 8. HTTPS with Certbot

With the name already pointing to the machine and port 80 open:

```bash
sudo certbot --nginx -d your-domain
```

Certbot adds the certificate, redirects port 80 to 443 and renews by itself (check that its timer exists: `systemctl list-timers | grep certbot`). Open in the firewall only 80 and 443 (and SSH).

Test the server from outside:

```bash
curl -s https://your-domain/v1/server
```

It must answer with a JSON with what the server offers (`invitations`, `events`, `sign_in_with_google`…). If it answers, you can write `https://your-domain` under "Other server" in the app.

## 9. The scheduled-task process

The server has a task every hour (it deletes unconfirmed accounts and expired codes) that **does nothing by itself**: it only runs while something consumes its queue. It is a systemd service, in `/etc/systemd/system/kosmotheka-worker.service`:

```ini
[Unit]
Description=Kosmotheka scheduled tasks
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

The `--time-limit` makes it restart itself every hour, with clean memory; `Restart=always` brings it back up. If you do not want the service, you can run the purge by hand from time to time: `php8.5 bin/console app:accounts:purge`.

## 10. Instant notices (optional)

Mercure is a separate component that lets open apps know that there is something new. Without it everything works the same: the apps update when they are opened. The server says so in `/v1/server` (`events: false`).

If you want it, install the [Mercure hub](https://mercure.rocks/docs/hub/install) (version 1.0) as a systemd service, listening only on `127.0.0.1:3000`, and:

1. Generate **two different keys**: `openssl rand -hex 32` for the publisher (only the server uses it) and another one for the subscriber (it signs the tokens the apps receive). One cannot publish with the other.
2. In the hub: both keys with the HS256 algorithm, `MERCURE_TRUSTED_ISSUERS=urn:kosmotheka:server`, and the directive `resource_identifier http://127.0.0.1:3000/.well-known/mercure` (the audience of the tokens, fixed).
3. In `.env.local`: `MERCURE_URL=http://127.0.0.1:3000/.well-known/mercure` (the same address as `resource_identifier`), `MERCURE_PUBLISHER_JWT_KEY` and `MERCURE_SUBSCRIBER_JWT_KEY`.
4. Remove the comments from the `/.well-known/mercure` block in nginx.
5. `php8.5 bin/console cache:clear` and restart PHP-FPM.

> **To confirm:** this part is taken from how development works (where the hub runs in a container) and has not been tested with the hub installed as a service. If you do not need it, skip it.

## Afterwards

With this the server already works. What is left is [the backups](../backups/), which are not optional, and configuring [the email, Google and the search services](../configuration/) according to what you want to offer.
