# Encargo · El esqueleto de docs.kosmotheka.com

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Es el paso 2 de [plans/repositorio-publico.md](../plans/repositorio-publico.md): léelo antes, junto con D90 y [plans/documentacion-publica.md](../plans/documentacion-publica.md). **Empieza por la fase 0 y espera mi respuesta antes de instalar nada.**

## Para qué

La documentación pública vive en el mismo repositorio y se publica en docs.kosmotheka.com con **Starlight** (Astro), en español y en inglés (D90, acordado el 13/10/2026). Este encargo deja montado el sitio vacío y bien hecho. Las páginas de verdad vienen después, en otros encargos.

## Decidido ya

- **Carpeta `site/`** en la raíz del repositorio. No va en `docs/`, que es el diario de desarrollo: así no se mezclan las dos cosas.
- **Dos idiomas**: español (`es`) e inglés (`en`). El inglés, en la raíz del sitio, como el resto del repositorio; o el español primero, si Starlight lo pone más fácil: dímelo en la fase 0.
- **Tres partes**, en el menú lateral: «Usar la app», «Montar tu servidor» y «Colaborar».
- **Búsqueda sin servidor:** la que trae Starlight, Pagefind.
- **Nada de terceros al visitar la web:** sin analítica, sin fuentes de Google, sin CDN. La fuente es Figtree, la misma de la web, servida desde el propio sitio (`web/assets/fonts/`, con su `OFL.txt`).
- **Sin telemetría de Astro** al compilar (`ASTRO_TELEMETRY_DISABLED=1` en los scripts).
- **Versiones exactas** en `package.json` y `package-lock.json` en git, como en `app/`.
- **La CI compila el sitio** en cada push (un job nuevo y corto). **No se despliega**: cómo se publica se decide aparte (D90).

## Fase 0 · Propuesta (sin instalar nada)

Mándame un informe corto con:

1. **Versiones** de `astro` y `@astrojs/starlight` que usarías y cuántos paquetes entran en `node_modules` (`npm view` y la documentación, sin instalar).
2. **Cómo queda la estructura** de `site/`: ficheros de configuración y carpetas de contenido por idioma.
3. **Cómo se parece a la web:**
   - colores y logo de `brand/` y `web/assets/` (claro y oscuro);
   - la cabecera enlaza a kosmotheka.com;
   - el pie lleva la misma licencia y los mismos contactos.

   Sin copiar el CSS de la web entero: solo los colores y la fuente, con las variables de Starlight.
4. **El job de la CI:** qué hace y cuánto tarda, más o menos.
5. **Si `npm install` funciona desde tu sesión.** Antes no tenías red con `curl`. Si no la tienes, dímelo y la instalación la hago yo.

## Después de mi visto bueno

- El sitio, con una portada por idioma y una página de entrada por cada parte, que diga en dos o tres frases qué habrá en ella. **Nada de «en construcción» vacío:** si una parte aún no tiene nada, su portada explica qué va a tener y enlaza a lo que ya existe (por ejemplo, `CONTRIBUTING.md` en la parte de colaborar).
- `npm run build` y `npm run dev` en `site/`. Dos líneas en la tabla de comandos de `AGENTS.md`.
- `site/README.md` corto: cómo se añade una página y en qué idioma se empieza (español primero, como manda el plan).
- Que `node_modules/` y la salida de la compilación estén en `.gitignore`.

## Reglas

- Textos del sitio en el idioma de cada página; el código y la configuración, en inglés.
- Nada fuera de `site/`, salvo `.gitignore`, `.github/workflows/ci.yml` y la tabla de `AGENTS.md`.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

`npm run build` en `site/`, con su salida real, y una captura de la portada en español y en inglés, en claro y en oscuro: con el navegador que tengas a mano o con `npx playwright` dentro de `library-dev`. Si no puedes hacer capturas, dilo.
