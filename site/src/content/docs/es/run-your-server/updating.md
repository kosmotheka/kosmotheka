---
title: Actualizar a una versión nueva
description: El orden para actualizar el servidor de Kosmotheka sin perder datos, y cuándo actualizar antes la app.
sidebar:
  order: 5
---

Actualizar es cambiar el código y aplicar las migraciones de la base de datos. El orden importa y **empieza siempre por una copia**: una migración no siempre se puede deshacer.

## Antes: lee las notas de la versión

Las notas de cada versión dicen si hay algo especial. Dos casos que importan:

- **Primero la app nueva en las tiendas, después el servidor**, cuando las notas de la versión lo digan (la 0.2.1 lo pedía). Si el servidor cambia cómo se entra o cómo se firma, las apps viejas dejan de poder conectarse hasta que se actualizan; si primero esperas a que la gente tenga la app nueva, nadie se queda fuera.
- Algunas versiones piden un paso extra tras migrar (por ejemplo, volver a sellar las claves de la nube, o reconstruir el catálogo de las bibliotecas). Lo dicen las notas.

## Los pasos

Con el ejemplo del sitio de la guía, `/srv/kosmotheka`:

```bash
# 1. Copia (la del temporizador, o una a mano ahora)
sudo systemctl start kosmotheka-backup.service

# 2. Código nuevo
cd /srv/kosmotheka
git fetch --tags
git checkout vX.Y.Z

# 3. Dependencias
cd server
php8.5 /usr/bin/composer install --no-dev --optimize-autoloader

# 4. Migraciones
php8.5 bin/console doctrine:migrations:migrate --no-interaction

# 5. Vaciar la caché
php8.5 bin/console cache:clear

# 6. Reiniciar PHP-FPM y el servicio de tareas
sudo systemctl restart php8.5-fpm kosmotheka-worker
```

Después, comprueba que responde: `curl -s https://tu-dominio/v1/server`.

Si hay una variable nueva en las notas, añádela a `.env.local` **antes** del paso 4. `.env.local` no se toca al actualizar el código.

## Si algo sale mal

- **Falla una migración:** no sigas. Léela y busca la causa; una migración se ejecuta dentro de una transacción y, si falla, no deja cambios a medias.
- **El código nuevo no arranca:** vuelve al código anterior (`git checkout` de la etiqueta anterior) si las migraciones no llegaron a aplicarse. Si ya se aplicaron, restaura la copia del paso 1 (ver [restaurar](/es/run-your-server/backups/#restaurar)) y el código anterior juntos.
- **Una migración no se deshace:** alguna lo dice (su `down` lanza un error a propósito). Por eso la copia es el paso 1.
