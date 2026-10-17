---
title: Common problems
description: What has already happened when setting up the Kosmotheka server, and how it is solved.
sidebar:
  order: 10
---

## `php` is another version

On a machine with several PHP versions, a bare `php` may be the old one and fail with syntax errors or extension errors that have nothing to do with Kosmotheka. Always use `php8.5` (`php8.5 bin/console …`, `php8.5 /usr/bin/composer …`), also in the systemd service. Check which one nginx uses: `php8.5-fpm` must be the one listening on the site's socket.

## The cloud secret, in base64

`CLOUD_KEY_SECRET` must be **base64** of 32 bytes: `openssl rand -base64 32`. One generated in hexadecimal (`openssl rand -hex 32`) is not what the server expects (it happened on the official server) and the cloud cannot sign. If it happens to you, change the value **only if there are no cloud libraries created yet**: changing it afterwards leaves the server unable to sign in the ones that already exist.

## I want to empty the database and start from zero

`doctrine:schema:drop --full-database` deletes the tables but **not the function `catalog_same_library()`**, and the migrations fail when they find it already created. Delete it by hand before migrating:

```bash
sudo -u postgres psql -d kosmotheka -c 'DROP FUNCTION IF EXISTS catalog_same_library() CASCADE;'
php8.5 bin/console doctrine:migrations:migrate --no-interaction
```

The cleanest is to delete and create the whole database again (`DROP DATABASE` and `CREATE DATABASE`), which leaves nothing behind. Do it only if you do not mind what is inside: **it is irreversible**.

## Nobody can sign in (`invalid_device_proof`)

`DEFAULT_URI` does not match the address the app has saved for the server. It must be exactly `https://your-domain` (lowercase, no trailing slash, no port if it is 443). Correct it, `cache:clear` and restart PHP-FPM.

## Emails do not arrive

`MAILER_DSN` by default is `null://null`, which sends nothing. Configure it (see [the email](../configuration/#the-email)), clear the cache and check the server's error log.

## nginx error 502

Almost always PHP-FPM is not running or the `fastcgi_pass` socket does not match its pool's. Look at `sudo systemctl status php8.5-fpm` and `/var/log/nginx/kosmotheka_error.log`.

## "Permission denied" in `var/`

PHP-FPM cannot write to `server/var/`. `sudo chown -R www-data: /srv/kosmotheka/server/var` (or your pool's user). If you ran `composer` or `bin/console` as the administrator, it may have left cache files that PHP-FPM can no longer touch: delete them with `sudo rm -rf server/var/cache` and rebuild the cache with `cache:clear` as the pool's user.
