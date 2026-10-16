# Encargo · Clasificar `docs/` fichero a fichero para abrir el repositorio

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Es el paso 1 de [plans/repositorio-publico.md](../plans/repositorio-publico.md): léelo antes, y la enmienda de D90 del 16/10/2026 en `docs/DECISIONS.md`. **Solo lee y escribe un informe: no mueve, borra ni cambia ningún fichero.**

## Para qué

El maintainer ha decidido publicar la documentación de trabajo como **diario de desarrollo**. Solo sale del repositorio lo privado de verdad, que irá a `library-privado/`, fuera de él. Antes de reescribir el historial hay que saber qué fichero va a cada sitio, y si hay algo que no puede salir tal cual.

## Qué hay que mirar, en cada fichero de `docs/` (y `brand/`, `web/`, `ds-bundle/` si está en git)

Una fila por fichero o por carpeta homogénea (por ejemplo, las capturas de una pantalla de `docs/design/screens/`). Columnas:

| Ruta | Destino | Por qué | Qué hay que cambiar antes |
|---|---|---|---|

**Destino**, uno de:
- **público:** sale tal cual;
- **público con cambios:** sale, pero antes hay que quitar o cambiar algo (dilo en la última columna);
- **privado:** va a `library-privado/` y se quita del historial;
- **a decidir:** no está claro. Explica la duda en una línea.

**Lo que hace que algo sea privado o necesite cambios.** Búscalo con `grep`; no te fíes de los nombres:
1. **Hallazgos de seguridad abiertos** en `docs/audits/hallazgos.md` y en los informes. Los cerrados se publican (D90). Di cuántos abiertos hay y en qué ficheros se describen con detalle.
2. **Datos del servidor de producción:** IP, nombre del VPS, usuarios, rutas del servidor, versiones instaladas y la configuración de nginx o PHP del servidor real. También datos de la cuenta de Google Play, la clave de subida o su huella.
3. **Datos personales:** correos que no sean `info@`, `privacy@` o `noreply@` de kosmotheka; nombres de personas reales que no sean el maintainer como autor; rutas locales `/home/…`.
4. **Secretos o cosas que lo parezcan:** claves, tokens, contraseñas, aunque sean de prueba. Distingue los de prueba que ya están en el código (fixtures) de los reales.
5. **Material de terceros:** imágenes, capturas de otras apps, textos copiados o fuentes. Di qué licencia tienen, si se sabe, o «sin licencia clara».
6. **Lo que a una persona le daría vergüenza ver publicado:** opiniones sobre terceros o comentarios personales. Señálalo sin juzgar y que decida el maintainer.

## Además

- **Las referencias cruzadas.** Si un fichero privado se enlaza desde uno público, apúntalo: el enlace se romperá. Igual con el código: lista los ficheros de `app/`, `server/`, `crates/`, `spec/`, `.claude/` y `.github/` que apuntan a una ruta que vaya a ser privada.
- **El historial.** Con `git log --all --diff-filter=D --name-only` y parecidos, lista los ficheros **que ya no existen** pero siguen en el historial y caerían en las mismas categorías. Mira solo los nombres y, si hace falta, su último contenido; no hace falta un escaneo de secretos completo (eso va aparte, con gitleaks, en el paso 8).
- **El tamaño.** Lo que más pesa en el historial (`git rev-list --objects --all` con el tamaño de cada objeto): los 20 mayores, con su ruta.

## Reglas

- **Solo lectura.** Ni `git mv`, ni `rm`, ni editar. Nada de `git filter-repo` ni parecidos.
- **No copies en el informe ningún secreto, IP ni dato personal** que encuentres: di el fichero, la línea y de qué tipo es, nunca el valor.
- Los comandos de git, solo de lectura. No deshagas nada.

## Al terminar

Escribe el informe en `docs/historico/clasificacion-docs.md`, en español:
- la tabla;
- un resumen de cifras: cuántos ficheros públicos, con cambios, privados y a decidir, y cuántos MB salen del historial;
- las referencias cruzadas;
- lo encontrado en el historial;
- los 20 objetos mayores;
- una lista corta de lo que tiene que decidir el maintainer.

Mándame un aviso corto cuando esté.
