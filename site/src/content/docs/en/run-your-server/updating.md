---
title: Updating to a new version
description: The order for updating the Kosmotheka server without losing data, and when to update the app first.
sidebar:
  order: 6
---

Updating means changing the code and applying the database migrations. The order matters, and it **always starts with a copy**: a migration cannot always be undone.

## Before: read the release notes

The release notes of each version say whether there is anything special. Two cases matter:

- **First the new app in the stores, then the server**, when the release notes say so (version 0.2.1 asked for it). If the server changes how sign-in or signing works, the old apps stop being able to connect until they are updated; if you wait for people to have the new app first, nobody is left out.
- Some versions ask for an extra step after migrating (for example, sealing the cloud keys again, or rebuilding the library catalogue). The notes say so.

## The steps

With the example site of the guide, `/srv/kosmotheka`:

```bash
# 1. Copy (the timer's, or one by hand now)
sudo systemctl start kosmotheka-backup.service

# 2. New code
cd /srv/kosmotheka
git fetch --tags
git checkout vX.Y.Z

# 3. Dependencies
cd server
php8.5 /usr/bin/composer install --no-dev --optimize-autoloader

# 4. Migrations
php8.5 bin/console doctrine:migrations:migrate --no-interaction

# 5. Clear the cache
php8.5 bin/console cache:clear

# 6. Restart PHP-FPM and the task service
sudo systemctl restart php8.5-fpm kosmotheka-worker
```

Then check that it answers: `curl -s https://your-domain/v1/server`.

If there is a new variable in the notes, add it to `.env.local` **before** step 4. `.env.local` is not touched when you update the code.

## If something goes wrong

- **A migration fails:** do not go on. Read it and look for the cause; a migration runs inside a transaction and, if it fails, it leaves no half-done changes.
- **The new code does not start:** go back to the previous code (`git checkout` of the previous tag) if the migrations did not get applied. If they were applied, restore the copy from step 1 (see [restore](../backups/#restore)) and the previous code together.
- **A migration cannot be undone:** some say so (their `down` throws an error on purpose). That is why the copy is step 1.
