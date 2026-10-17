---
title: Problemas comunes
description: Lo que ya ha pasado al montar el servidor de Kosmotheka y cómo se resuelve.
sidebar:
  order: 7
---

## `php` es otra versión

En una máquina con varias versiones de PHP, `php` a secas puede ser la antigua y fallar con errores de sintaxis o de extensiones que no tienen que ver con Kosmotheka. Usa siempre `php8.5` (`php8.5 bin/console …`, `php8.5 /usr/bin/composer …`), también en el servicio de systemd. Comprueba cuál usa nginx: `php8.5-fpm` debe ser el que escucha en el socket del sitio.

## El secreto de la nube, en base64

`CLOUD_KEY_SECRET` tiene que ser **base64** de 32 bytes: `openssl rand -base64 32`. Uno generado en hexadecimal (`openssl rand -hex 32`) no es lo que el servidor espera (pasó en el servidor oficial) y la nube no puede firmar. Si te pasa, cambia el valor **solo si todavía no hay bibliotecas de la nube creadas**: cambiarlo después deja al servidor sin poder firmar en las que ya existen.

## Quiero vaciar la base y empezar de cero

`doctrine:schema:drop --full-database` borra las tablas pero **no la función `catalog_same_library()`**, y las migraciones fallan al encontrarla ya creada. Bórrala a mano antes de migrar:

```bash
sudo -u postgres psql -d kosmotheka -c 'DROP FUNCTION IF EXISTS catalog_same_library() CASCADE;'
php8.5 bin/console doctrine:migrations:migrate --no-interaction
```

Lo más limpio es borrar y volver a crear la base entera (`DROP DATABASE` y `CREATE DATABASE`), que no deja nada atrás. Hazlo solo si no te importa lo que hay dentro: **es irreversible**.

## Nadie puede iniciar sesión (`invalid_device_proof`)

`DEFAULT_URI` no coincide con la dirección que la app tiene guardada para el servidor. Tiene que ser `https://tu-dominio` exactamente igual (minúsculas, sin barra final, sin puerto si es el 443). Corrígela, `cache:clear` y reinicia PHP-FPM.

## No llegan los correos

`MAILER_DSN` por defecto es `null://null`, que no envía nada. Configúralo (ver [el correo](/es/run-your-server/configuration/#el-correo)), vacía la caché y comprueba el registro de errores del servidor.

## Error 502 de nginx

Casi siempre es que PHP-FPM no está corriendo o que el socket de `fastcgi_pass` no coincide con el de su *pool*. Mira `sudo systemctl status php8.5-fpm` y `/var/log/nginx/kosmotheka_error.log`.

## «Permission denied» en `var/`

PHP-FPM no puede escribir en `server/var/`. `sudo chown -R www-data: /srv/kosmotheka/server/var` (o el usuario de tu *pool*). Si ejecutaste `composer` o `bin/console` como administrador, puede haber dejado ficheros de caché que ya no puede tocar PHP-FPM: bórralos con `sudo rm -rf server/var/cache` y vuelve a crear la caché con `cache:clear` como el usuario del *pool*.
