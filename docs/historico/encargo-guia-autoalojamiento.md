# Encargo · La guía para montar tu propio servidor

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push, sin cambiar de rama. Es el paso 5 de [plans/repositorio-publico.md](../plans/repositorio-publico.md), con D48 precisada el 16/10/2026: **dos caminos con las mismas piezas**, sin Docker (como el servidor oficial) y con Docker. El maintainer desplegó la 0.2.1 en producción el 17/10/2026 sin Docker, y todo funcionó. **Fase 0 primero para la parte de Docker; la parte sin Docker puedes escribirla ya.**

**Regla del maintainer para hoy:** nada de funcionalidades nuevas hasta que pruebe lo hecho. Esto es documentación y empaquetado: no cambia el comportamiento del servidor ni de la app.

## Dónde y cómo

- En `site/src/content/docs/es/run-your-server/`, en español primero, como la guía de usuario.
- Para quien sabe administrar un servidor Linux, pero no conoce Symfony ni el proyecto. Lenguaje técnico normal, sin la jerga interna del proyecto (nada de valla, sello, D84…). Si hace falta el porqué de algo, se dice en una frase.
- **Nada del VPS del maintainer:** ni IPs, ni nombres, ni proveedor, ni rutas reales. Marcadores (`tu-dominio`, `/srv/kosmotheka`), como hace `docs/publicacion/servidor-produccion.md`.

## Las páginas

1. **Antes de empezar: lo que tienes que saber.** Las limitaciones, al principio y sin esconderlas (están en `plans/repositorio-publico.md`, «Quien monta su servidor»):
   - la app oficial puede conectarse a tu servidor;
   - sin notificaciones con la app cerrada (D75);
   - los enlaces de invitación se pegan en la app;
   - Google, el correo y las claves de los proveedores se configuran aparte y son opcionales;
   - quien administra el servidor puede leer lo que se sube;
   - el nombre y el logo no se pueden usar como si fuera el servicio oficial (`TRADEMARKS.md`).

   Y qué hace falta: un dominio con HTTPS y un servidor Linux con recursos modestos (di cuánto, según `docs/architecture/cloud.md`, «Primera revisión, con las tablas llenas»).
2. **Sin Docker.** Requisitos exactos y pasos:
   - PHP 8.5 y sus extensiones, PostgreSQL 18, nginx y Composer (versiones de `server/composer.json` y `docs/development/server.md`);
   - el usuario y la base;
   - `.env.local`;
   - `composer install --no-dev`;
   - las migraciones;
   - el sitio de nginx con PHP-FPM;
   - HTTPS con Certbot;
   - el *worker* de tareas programadas como servicio de systemd (la purga horaria, D49).

   Si en la máquina conviven varias versiones de PHP, hay que llamar a `php8.5` explícitamente (pasó en el despliegue real).
3. **Con Docker:** solo en la fase 0, ver abajo.
4. **La configuración, variable a variable.** Todas las de `server/.env`: para qué sirve cada una, si es obligatoria, cómo se genera (`openssl rand…`) y qué pasa si se deja vacía. Agrupadas: lo básico, el correo, Google, los proveedores (TMDB, RAWG, Comic Vine, Google Books, BGG, con sus condiciones y su atribución) y los avisos al momento (Mercure, opcional).
5. **Copias de seguridad y restaurar.** Un volcado diario con `pg_dump --format=custom` desde un temporizador de systemd o cron, con rotación (por ejemplo, 7 diarios y 4 semanales); copiar el `.env.local` aparte; **llevar las copias fuera de la máquina**; y cómo restaurar, probado. Escribe el script de copia en la guía, sin dependencias raras.
6. **Actualizar a una versión nueva.** El orden que funcionó:
   1. copia;
   2. código nuevo;
   3. `composer install`;
   4. migraciones;
   5. vaciar la caché;
   6. reiniciar PHP-FPM y el *worker*.

   **Primero la app nueva en las tiendas, después el servidor**, cuando una versión lo diga en sus notas (la 0.2.1 lo pedía).
7. **Problemas comunes.** Lo que ya ha pasado:
   - `php` a secas es otra versión;
   - para vaciar la base, `schema:drop --full-database` no quita la función `catalog_same_library()`, y hay que borrarla a mano antes de migrar;
   - `doctrine:schema:validate` da un `[FAIL]` en `CloudOperation#body` que es un aviso, no un error (se arreglará aparte; dilo así hasta entonces);
   - el secreto de la nube, en base64, no en hexadecimal (pasó en producción el 11/10/2026).

## Fase 0 · El camino con Docker

Hoy los `Dockerfile` de `server/docker/` son **solo de desarrollo** (SEG-009: corren como root y llevan Xdebug). Propón, sin escribirlo todavía:

- una imagen de producción: PHP-FPM sin Xdebug ni dependencias de desarrollo, sin root y con `composer install --no-dev` dentro;
- un `docker-compose` de producción con PHP-FPM, nginx, PostgreSQL y el *worker*, y Mercure opcional;
- dónde viven: `server/docker/prod/`, por ejemplo;
- cómo se prueba aquí: construirla y levantarla en local con otro nombre de proyecto y otros puertos, **sin tocar el entorno de desarrollo** que está corriendo;
- cuánto trabajo es.

**No publicamos ninguna imagen en un registro.** La guía dice cómo construirla.

## Al terminar

`npm run build` en `site/`. El informe: las páginas con su título y lo que has dejado marcado como «por confirmar». Y la propuesta de la fase 0 de Docker.
