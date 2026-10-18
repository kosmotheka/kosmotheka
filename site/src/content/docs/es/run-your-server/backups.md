---
title: Copias de seguridad y restaurar
description: Un script de copia diaria de la base de datos de Kosmotheka y de su configuración, cómo llevarla fuera de la máquina y cómo restaurarla.
sidebar:
  order: 8
---

Las bibliotecas de la nube viven en la base de datos del servidor. Si la máquina se pierde y no tienes copia, se pierde lo que solo estaba allí (las apps conservan su propia copia local, pero reconstruir desde ellas es mucho trabajo). Una copia útil cumple cuatro cosas: **está fuera de la máquina**, **está cifrada**, **contiene todo lo que no se puede regenerar** y **se ha restaurado una vez de verdad**.

## Qué copiar

1. **La base de datos**, con `pg_dump --format=custom` (comprimida y se restaura con `pg_restore`).
2. **`server/.env.local`**, que lleva `CLOUD_KEY_SECRET`, `APP_SECRET` y las claves de los servicios. **Sin `CLOUD_KEY_SECRET`, la copia de la base no permite firmar** en las bibliotecas ya creadas. El script lo cifra igual que la base.
3. Tu configuración de nginx y de systemd, si no la tienes escrita en otro sitio. El código no hace falta: está en su repositorio.

## Cifrar las copias: quien entre en la máquina no las lee

Las copias llevan los datos de todas las bibliotecas y los secretos del servidor. Si alguien entra en la máquina y las copias están en claro, se lleva también las antiguas. Por eso se cifran con [age](https://github.com/FiloSottile/age) con una clave **pública** que vive en el servidor y **solo sirve para cifrar**; la clave **privada** no está nunca en el servidor, sino en tu ordenador (y en otro sitio seguro). Así ni la copia de ayer se puede abrir desde la máquina.

`age` está en los repositorios de Debian 12 o posterior y Ubuntu 22.04 o posterior: `sudo apt install age`. Crea el par de claves **en tu ordenador**, no en el servidor:

```bash
age-keygen -o clave-copias-kosmotheka.txt
# Public key: age1...
```

- Guarda `clave-copias-kosmotheka.txt` (la privada) fuera de la máquina del servidor, con copia en otro lugar. **Sin ella, las copias no se pueden abrir**: es una copia más que proteger, y perderla es perder las copias.
- En el servidor pon solo la línea `age1...` (la pública):

```bash
sudo mkdir -p /etc/kosmotheka
echo 'age1...tu-clave-publica...' | sudo tee /etc/kosmotheka/backup-recipient.txt
```

### Con gpg en lugar de age

Si prefieres no instalar nada, `gpg` hace lo mismo y ya viene en casi cualquier Linux, en el servidor y en tu ordenador. Crea la pareja de claves **en tu ordenador**; el correo es solo una etiqueta, no hace falta que exista:

```bash
gpg --quick-generate-key "Copias de Kosmotheka <copias@example.org>" future-default default never
gpg --armor --export copias@example.org > copias-publica.asc
gpg --armor --export-secret-keys copias@example.org > copias-privada.asc   # guárdala fuera y bórrala de aquí
```

En el servidor, importa solo la pública (`sudo gpg --import copias-publica.asc`) y apunta su huella, la línea de 40 caracteres de `sudo gpg --list-keys`. En el script, cambia cada `age -R "$RECIPIENT"` por:

```bash
gpg --homedir /root/.gnupg --batch --yes --trust-model always --no-auto-key-locate --encrypt --recipient LA_HUELLA
```

- Usa la huella y no el correo: con un correo que no está en el llavero, gpg intenta buscar la clave por internet y la copia falla.
- `--homedir` fija el llavero de `root` aunque systemd lance el servicio sin `HOME`.
- Cambia también la extensión `.age` de los nombres por `.gpg`. La comprobación del final del script cambia: un archivo de gpg no empieza por `age-encryption.org/v1`. Quítala, o compara con `gpg --list-packets` desde tu ordenador.
- Para abrir una copia: `gpg -d db-AAAA-MM-DD.dump.gpg`, en lugar de `age -d -i …`.

## El script

Guárdalo como `/usr/local/sbin/kosmotheka-backup`, con `chmod 750`. Se ejecuta como administrador (`root`), pide el volcado a `postgres`, lo cifra por tubería (el volcado en claro nunca toca el disco) y guarda siete copias diarias y cuatro semanales.

```bash
#!/usr/bin/env bash
set -euo pipefail
umask 077

DB=kosmotheka
ENV_FILE=/srv/kosmotheka/server/.env.local
RECIPIENT=/etc/kosmotheka/backup-recipient.txt
DEST=/var/backups/kosmotheka
STAMP=$(date +%F)

mkdir -p "$DEST/daily" "$DEST/weekly"

# El volcado se cifra al vuelo y se escribe con otro nombre, que se cambia al terminar:
# una copia a medias nunca parece completa. Con pipefail, si pg_dump falla, falla todo.
runuser -u postgres -- pg_dump --format=custom "$DB" \
  | age -R "$RECIPIENT" > "$DEST/daily/db-$STAMP.dump.age.partial"
mv "$DEST/daily/db-$STAMP.dump.age.partial" "$DEST/daily/db-$STAMP.dump.age"

age -R "$RECIPIENT" < "$ENV_FILE" > "$DEST/daily/env-$STAMP.local.age"

# El servidor no puede descifrar, así que solo comprueba que lo escrito es un archivo de age.
for f in "$DEST/daily/db-$STAMP.dump.age" "$DEST/daily/env-$STAMP.local.age"; do
  [ "$(head -c 21 "$f")" = "age-encryption.org/v1" ] || { echo "copia dañada: $f" >&2; exit 1; }
done

# Los domingos, además, una copia semanal.
if [ "$(date +%u)" = 7 ]; then
  cp "$DEST/daily/db-$STAMP.dump.age" "$DEST/daily/env-$STAMP.local.age" "$DEST/weekly/"
fi

# Rotación: 7 días y 4 semanas.
find "$DEST/daily" -type f -mtime +7 -delete
find "$DEST/weekly" -type f -mtime +28 -delete

echo "kosmotheka-backup: copia $STAMP hecha"
```

Si usas otra base o ruta, cambia las primeras líneas. **Comprueba que sabes abrir una copia** (ver «Restaurar») antes de fiarte: es lo único que el servidor no puede hacer por ti.

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

Lánzalo desde un temporizador de ese otro ordenador. Como ya van cifradas con age, pueden viajar y guardarse en cualquier sitio sin que quien las tenga las lea.

## Restaurar

Las copias se abren con la clave **privada**, que tienes en tu ordenador. Descifra y restaura por tubería, así el volcado en claro tampoco toca el disco. **Practica la restauración antes de necesitarla**, en una base de pruebas, no en la de verdad. Con la copia `db-AAAA-MM-DD.dump.age` y tu `clave-copias-kosmotheka.txt`:

```bash
sudo -u postgres createdb -O kosmotheka kosmotheka_prueba
age -d -i clave-copias-kosmotheka.txt db-AAAA-MM-DD.dump.age \
  | sudo -u postgres pg_restore --no-owner --role=kosmotheka --dbname=kosmotheka_prueba
sudo -u postgres psql -d kosmotheka_prueba -c '\dt'
sudo -u postgres psql -d kosmotheka_prueba -c 'SELECT count(*) FROM cloud_library;'
sudo -u postgres dropdb kosmotheka_prueba
```

Para ver tu `.env.local` de la copia: `age -d -i clave-copias-kosmotheka.txt env-AAAA-MM-DD.local.age`. Si restauras en otro ordenador que no es el servidor, solo cambia `sudo -u postgres` por lo que corresponda allí. Vale si `pg_restore` no da errores, aparecen las tablas y el recuento tiene sentido.

Para recuperar el servidor en una máquina nueva:

1. Instala PHP, PostgreSQL, nginx y el código como en [Montarlo sin Docker](/es/run-your-server/without-docker/), **hasta el paso de la base de datos incluido y sin ejecutar las migraciones**.
2. Restaura la copia en la base vacía: `age -d -i clave-copias-kosmotheka.txt db-AAAA-MM-DD.dump.age | sudo -u postgres pg_restore --no-owner --role=kosmotheka --dbname=kosmotheka`.
3. Descifra tu `.env.local` de la copia y ponlo en `server/` (con el mismo `CLOUD_KEY_SECRET`).
4. Continúa desde nginx y HTTPS. Si el nombre del servidor cambia, **las apps ya no lo reconocerán**: `DEFAULT_URI` es parte de lo que firma cada dispositivo para entrar. Mantén el mismo nombre.
5. Ejecuta `php8.5 bin/console doctrine:migrations:migrate --no-interaction`: solo aplicará las que falten si la copia era de una versión anterior del código.

> **Por confirmar:** el guion de restauración sobre un servidor completo (paso a paso en una máquina nueva) está escrito pero no se ha ensayado de principio a fin; el script de copia cifrada, su descifrado y el `pg_restore` de la base se han probado con PostgreSQL 18 y age 1.2.1 en un contenedor de usar y tirar.
