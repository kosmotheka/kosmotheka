# Encargo · Dejar el árbol de trabajo listo para abrir el repositorio

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **No se toca el historial** (eso lo hace el maintainer con `git filter-repo`, en un ensayo que prepara la sesión coordinadora). Aquí solo se cambian ficheros del árbol de trabajo. La lista de lo que hay que cambiar está en [clasificacion-docs.md](clasificacion-docs.md), sección «Para el día de abrirlo»: **léela entera**. Los números de línea son del 16/10/2026: compruébalos con `git grep` antes de cambiar nada.

## Qué hacer

1. **Enlaces a `servidor-produccion.md` y a `firma-android.md`** (punto 1 de la lista). Esos dos ficheros saldrán del repositorio.
   - Cambia los enlaces de los documentos vivos (`architecture/`, `plans/`, `publicacion/`, los comentarios de `build.gradle.kts` y de `.gitignore`) para que apunten a la página pública que diga lo mismo, o quítalos.
   - **`docs/historico/` no se edita.**
   - **No borres esos dos ficheros:** el maintainer los mueve a su carpeta privada el día de abrir.
2. **Una página nueva, `site/src/content/docs/es/contribute/release.md` · «Publicar una versión».**
   - Con la parte genérica de `docs/publicacion/firma-android.md`: la clave de subida fuera del repositorio, la orden de compilar el `.aab` y la versión.
   - Con los pasos de antes de cada versión: el número de versión en los cuatro sitios, `whats-new.json` y `npm run licenses` con `check:licenses:exact`.
   - **Sin rutas de la máquina del maintainer ni nada de su cuenta de Google Play.**

   Es la que enlazan ahora los comentarios del punto 1. Ponla en el `index.md` de «Colaborar».
3. **TMview** (punto 2): reescribe en `research/logo.md` y `research/naming.md` los enlaces a `tmview/` y a los scripts `tmview-check.mjs`, `tmview-image-check.mjs` y `play-check.mjs`. Que digan «resultado de la búsqueda del 26/09/2026», sin enlace. **No borres** `tmview/` ni los scripts: salen con el historial.
4. **Claude Design** (punto 3): reescribe `design/screens/README.md`.
   - Quita el párrafo «Para verlas en local», porque `support.js` y `ds-base.js` saldrán del historial.
   - Quita el enlace con el id del proyecto privado, si sigue ahí.
5. **Otros** (punto 5):
   - pon al día la tabla «Qué hay hoy» de `plans/repositorio-publico.md`, que ya no vale tras la enmienda de D90;
   - quita la sección de la carpeta ignorada.
6. **Comprueba y apunta en el informe:**
   - `git grep -n -E "\| abierto \|" docs/audits/hallazgos.md` tiene que dar 0;
   - los ocho informes de `audits/` que dicen «siguen abiertos» de hallazgos ya cerrados: **no los edites**, son históricos; solo apúntalo;
   - `AGENTS.md`, `CLAUDE.md`, `server/AGENTS.md` y `.claude/skills/`: cualquier ruta que apunte a algo que vaya a salir del repositorio (`library-privado`, `servidor-produccion.md`, `firma-android.md`, `research/tmview`). Cámbiala o dime cuál;
   - los scripts de `.claude/skills/audit/bin/`: ¿funcionan sin `docs/audits/descartes.md` y sin informes anteriores? Mira el código y dilo (D90 lo pide); no lo arregles todavía.

## Al terminar

- `cd site && npm run build && npm run check:links`.
- **El informe:**
  - cada cambio, con su fichero;
  - lo que no encontraste donde decía la lista;
  - lo del punto 6.
