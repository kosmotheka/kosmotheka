---
title: Copias de seguridad y restaurar
description: Un script de copia diaria de la base de datos de Kosmotheka y de su configuración, cómo llevarla fuera de la máquina y cómo restaurarla.
sidebar:
  order: 4
---

Las bibliotecas de la nube viven en la base de datos del servidor. Si la máquina se pierde y no tienes copia, se pierde lo que solo estaba allí (las apps conservan su propia copia local, pero reconstruir desde ellas es mucho trabajo). Una copia útil cumple tres cosas: **está fuera de la máquina**, **contiene todo lo que no se puede regenerar** y **se ha restaurado una vez de verdad**.

## Qué copiar

1. **La base de datos**, con `pg_dump --format=custom` (comprimida y se restaura con `pg_restore`).
2. **`server/.env.local`**, que lleva `CLOUD_KEY_SECRET`, `APP_SECRET` y las claves de los servicios. **Sin `CLOUD_KEY_SECRET`, la copia de la base no permite firmar** en las bibliotecas ya creadas. Guárdalo aparte de la base, en un sitio igual de seguro (el `.env.local` lleva secretos).
3. Tu configuración de nginx y de systemd, si no la tienes escrita en otro sitio. El código no hace falta: está en su repositorio.

## El script

Guárdalo como `/usr/local/sbin/kosmotheka-backup`, con `chmod 750`. Se ejecuta como administrador (`root`), pide el volcado a `postgres` y guarda siete copias diarias y cuatro semanales.

```bash
#!/usr/bin/env bash
set -euo pipefail
umask 077

DB=kosmotheka
ENV_FILE=/srv/kosmotheka/server/.env.local
DEST=/var/backups/kosmotheka
STAMP=$(date +%F)

mkdir -p "$DEST/daily" "$DEST/weekly"

# El volcado se escribe con otro nombre y se renombra al terminar:
# una copia a medias nunca parece completa.
runuser -u postgres -- pg_dump --format=custom "$DB" > "$DEST/daily/db-$STAMP.dump.partial"
pg_restore --list "$DEST/daily/db-$STAMP.dump.partial" > /dev/null   # se puede leer
mv "$DEST/daily/db-$STAMP.dump.partial" "$DEST/daily/db-$STAMP.dump"

cp "$ENV_FILE" "$DEST/daily/env-$STAMP.local"

# Los domingos, además, una copia semanal.
if [ "$(date +%u)" = 7 ]; then
  cp "$DEST/daily/db-$STAMP.dump" "$DEST/daily/env-$STAMP.local" "$DEST/weekly/"
fi

# Rotación: 7 días y 4 semanas.
find "$DEST/daily" -type f -mtime +7 -delete
find "$DEST/weekly" -type f -mtime +28 -delete

echo "kosmotheka-backup: copia $STAMP hecha"
```

Si usas otra base o ruta, cambia las tres primeras líneas.

### Que se haga solo: un temporizador de systemd

`/etc/systemd/system/kosmotheka-backup.service`:

```ini
[Unit]
Description=Copia de seguridad de Kosmotheka

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/kosmotheka-backup
```

`/etc/systemd/system/kosmotheka-backup.timer`:

```ini
[Unit]
Description=Copia diaria de Kosmotheka

[Timer]
OnCalendar=*-*-* 03:30:00
RandomizedDelaySec=15m
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now kosmotheka-backup.timer
sudo systemctl start kosmotheka-backup.service    # una prueba ahora
ls -l /var/backups/kosmotheka/daily
```

(Con cron sirve igual: una línea en `/etc/cron.d` que ejecute el script cada día.)

## Llevarlas fuera de la máquina

Una copia en el mismo disco no protege de perder el disco. Elige un sitio que no sea esa máquina: otro ordenador tuyo, un disco externo, el almacenamiento de otro proveedor. Un ejemplo con `rsync` desde otro ordenador, que **tira** de las copias (así la máquina del servidor no guarda ninguna credencial de acceso a tu copia):

```bash
rsync -a --delete usuario@tu-dominio:/var/backups/kosmotheka/ ~/copias/kosmotheka/
```

Lánzalo desde un temporizador de ese otro ordenador. Las copias llevan secretos y datos de las bibliotecas: guárdalas en un disco cifrado y, si usas un servicio de nube, cífralas antes de subirlas.

## Restaurar

**Practica la restauración antes de necesitarla**, en una base de pruebas, no en la de verdad. Con una copia `db-AAAA-MM-DD.dump`:

```bash
sudo -u postgres createdb -O kosmotheka kosmotheka_prueba
sudo -u postgres pg_restore --no-owner --role=kosmotheka --dbname=kosmotheka_prueba < db-AAAA-MM-DD.dump
sudo -u postgres psql -d kosmotheka_prueba -c '\dt'
sudo -u postgres psql -d kosmotheka_prueba -c 'SELECT count(*) FROM cloud_library;'
sudo -u postgres dropdb kosmotheka_prueba
```

El fichero se pasa con `<` para que lo lea tu usuario y no `postgres` (las copias son privadas). Vale si `pg_restore` no da errores, aparecen las tablas y el recuento tiene sentido.

Para recuperar el servidor en una máquina nueva:

1. Instala PHP, PostgreSQL, nginx y el código como en [Montarlo sin Docker](/es/run-your-server/without-docker/), **hasta el paso de la base de datos incluido y sin ejecutar las migraciones**.
2. Restaura la copia en la base vacía: `sudo -u postgres pg_restore --no-owner --role=kosmotheka --dbname=kosmotheka < db-AAAA-MM-DD.dump`.
3. Pon tu `.env.local` de la copia en `server/` (con el mismo `CLOUD_KEY_SECRET`).
4. Continúa desde nginx y HTTPS. Si el nombre del servidor cambia, **las apps ya no lo reconocerán**: `DEFAULT_URI` es parte de lo que firma cada dispositivo para entrar. Mantén el mismo nombre.
5. Ejecuta `php8.5 bin/console doctrine:migrations:migrate --no-interaction`: solo aplicará las que falten si la copia era de una versión anterior del código.

> **Por confirmar:** el guion de restauración sobre un servidor completo (paso a paso en una máquina nueva) está escrito pero no se ha ensayado de principio a fin; el script de copia y el `pg_restore` de la base se han probado con PostgreSQL 18 en un contenedor de usar y tirar.
