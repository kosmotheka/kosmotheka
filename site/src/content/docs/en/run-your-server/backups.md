---
title: Backups and restore
description: A script that makes a daily copy of the Kosmotheka database and its configuration, how to take it off the machine and how to restore it.
sidebar:
  order: 8
---

The cloud libraries live in the server's database. If the machine is lost and you have no copy, what was only there is lost (the apps keep their own local copy, but rebuilding from them is a lot of work). A useful copy does four things: it is **outside the machine**, it is **encrypted**, it contains **everything that cannot be regenerated** and it **has been restored once for real**.

## What to copy

1. **The database**, with `pg_dump --format=custom` (compressed, and restored with `pg_restore`).
2. **`server/.env.local`**, which carries `CLOUD_KEY_SECRET`, `APP_SECRET` and the keys of the services. **Without `CLOUD_KEY_SECRET`, the copy of the database does not allow signing** for the libraries already created. The script encrypts it the same way as the database.
3. Your nginx and systemd configuration, if you do not have it written down somewhere else. The code is not needed: it is in its repository.

## Encrypt the copies: whoever gets into the machine cannot read them

The copies carry the data of all the libraries and the server's secrets. If someone gets into the machine and the copies are in the clear, they also take the old ones. That is why they are encrypted with [age](https://github.com/FiloSottile/age), using a **public** key that lives on the server and **only serves to encrypt**; the **private** key is never on the server, but on your computer (and in another safe place). That way not even yesterday's copy can be opened from the machine.

`age` is in the repositories of Debian 12 or later and Ubuntu 22.04 or later: `sudo apt install age`. Create the key pair **on your computer**, not on the server:

```bash
age-keygen -o kosmotheka-backup-key.txt
# Public key: age1...
```

- Keep `kosmotheka-backup-key.txt` (the private one) outside the server's machine, with a copy somewhere else. **Without it, the copies cannot be opened**: it is one more copy to protect, and losing it means losing the copies.
- On the server put only the `age1...` line (the public one):

```bash
sudo mkdir -p /etc/kosmotheka
echo 'age1...your-public-key...' | sudo tee /etc/kosmotheka/backup-recipient.txt
```

## The script

Save it as `/usr/local/sbin/kosmotheka-backup`, with `chmod 750`. It runs as the administrator (`root`), asks `postgres` for the dump, encrypts it through a pipe (the plain dump never touches the disk) and keeps seven daily copies and four weekly ones.

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

# The dump is encrypted on the fly and written under another name, which is changed when it finishes:
# an incomplete copy never looks complete. With pipefail, if pg_dump fails, everything fails.
runuser -u postgres -- pg_dump --format=custom "$DB" \
  | age -R "$RECIPIENT" > "$DEST/daily/db-$STAMP.dump.age.partial"
mv "$DEST/daily/db-$STAMP.dump.age.partial" "$DEST/daily/db-$STAMP.dump.age"

age -R "$RECIPIENT" < "$ENV_FILE" > "$DEST/daily/env-$STAMP.local.age"

# The server cannot decrypt, so it only checks that what was written is an age file.
for f in "$DEST/daily/db-$STAMP.dump.age" "$DEST/daily/env-$STAMP.local.age"; do
  [ "$(head -c 21 "$f")" = "age-encryption.org/v1" ] || { echo "damaged copy: $f" >&2; exit 1; }
done

# On Sundays, also a weekly copy.
if [ "$(date +%u)" = 7 ]; then
  cp "$DEST/daily/db-$STAMP.dump.age" "$DEST/daily/env-$STAMP.local.age" "$DEST/weekly/"
fi

# Rotation: 7 days and 4 weeks.
find "$DEST/daily" -type f -mtime +7 -delete
find "$DEST/weekly" -type f -mtime +28 -delete

echo "kosmotheka-backup: copy $STAMP made"
```

If you use another database or path, change the first lines. **Check that you can open a copy** (see "Restore") before you trust it: it is the only thing the server cannot do for you.

### Make it automatic: a systemd timer

`/etc/systemd/system/kosmotheka-backup.service`:

```ini
[Unit]
Description=Kosmotheka backup

[Service]
Type=oneshot
ExecStart=/usr/local/sbin/kosmotheka-backup
```

`/etc/systemd/system/kosmotheka-backup.timer`:

```ini
[Unit]
Description=Daily Kosmotheka backup

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
sudo systemctl start kosmotheka-backup.service    # a test now
ls -l /var/backups/kosmotheka/daily
```

(With cron it works the same: one line in `/etc/cron.d` that runs the script every day.)

## Take them off the machine

A copy on the same disk does not protect you from losing the disk. Choose a place that is not that machine: another computer of yours, an external disk, another provider's storage. An example with `rsync` from another computer, which **pulls** the copies (so the server's machine keeps no credential to access your copy):

```bash
rsync -a --delete user@your-domain:/var/backups/kosmotheka/ ~/copies/kosmotheka/
```

Run it from a timer on that other computer. Since the copies are already encrypted with age, they can travel and be kept anywhere without whoever holds them being able to read them.

## Restore

The copies are opened with the **private** key, which is on your computer. Decrypt and restore through a pipe, so the plain dump does not touch the disk either. **Practise the restore before you need it**, on a test database, not on the real one. With the copy `db-YYYY-MM-DD.dump.age` and your `kosmotheka-backup-key.txt`:

```bash
sudo -u postgres createdb -O kosmotheka kosmotheka_test
age -d -i kosmotheka-backup-key.txt db-YYYY-MM-DD.dump.age \
  | sudo -u postgres pg_restore --no-owner --role=kosmotheka --dbname=kosmotheka_test
sudo -u postgres psql -d kosmotheka_test -c '\dt'
sudo -u postgres psql -d kosmotheka_test -c 'SELECT count(*) FROM cloud_library;'
sudo -u postgres dropdb kosmotheka_test
```

To see your `.env.local` from the copy: `age -d -i kosmotheka-backup-key.txt env-YYYY-MM-DD.local.age`. If you restore on another computer that is not the server, just change `sudo -u postgres` for whatever applies there. It is good if `pg_restore` gives no errors, the tables appear and the count makes sense.

To recover the server on a new machine:

1. Install PHP, PostgreSQL, nginx and the code as in [Setting it up without Docker](../without-docker/), **up to and including the database step, and without running the migrations**.
2. Restore the copy into the empty database: `age -d -i kosmotheka-backup-key.txt db-YYYY-MM-DD.dump.age | sudo -u postgres pg_restore --no-owner --role=kosmotheka --dbname=kosmotheka`.
3. Decrypt your `.env.local` from the copy and put it in `server/` (with the same `CLOUD_KEY_SECRET`).
4. Continue from nginx and HTTPS. If the server's name changes, **the apps will no longer recognise it**: `DEFAULT_URI` is part of what each device signs to sign in. Keep the same name.
5. Run `php8.5 bin/console doctrine:migrations:migrate --no-interaction`: it will only apply the ones that are missing, if the copy came from an earlier version of the code.

> **To confirm:** the restore script for a whole server (step by step on a new machine) is written but has not been tested from start to finish; the encrypted backup script, its decryption and the `pg_restore` of the database have been tested with PostgreSQL 18 and age 1.2.1 in a throwaway container.
