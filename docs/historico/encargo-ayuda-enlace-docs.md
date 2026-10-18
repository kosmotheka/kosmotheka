# Encargo · La Ayuda de la app enlaza con la documentación

> Encargo cerrado para la sesión de hitos, escrito el 18/10/2026. Sin commit, sin push. Aprobado por el maintainer el mismo día. **Por qué:** el plan de la Fase 8 (8.4) dice que la Ayuda de la app enlaza con docs.kosmotheka.com, y hoy no lo hace. La web se publica el día de abrir el repositorio; este cambio sale en la versión de la prueba cerrada, que es posterior.

## Qué

En `app/src/pages/help-page.tsx`, debajo de la lista de temas y antes del botón «Ver de nuevo la bienvenida» (`help.showWelcome`), una línea corta con un enlace:

- **Texto:** «¿No está aquí? La guía completa está en docs.kosmotheka.com.», con `docs.kosmotheka.com` como enlace. En inglés: «Not here? The full guide is at docs.kosmotheka.com.». Claves nuevas en `help` de `es.json` y `en.json`. Para meter el enlace en la frase, haz lo que ya haga el resto de la app (`<Trans>` o dos claves); míralo antes.
- **El enlace:** con `ExternalLink` (`app/src/components/external-link.tsx`), que abre el navegador del sistema.
- **La dirección por idioma,** como el mapa `PRIVACY` de `about-dialog.tsx`, con el inglés de respaldo:
  - `es`: `https://docs.kosmotheka.com/es/using-the-app/`
  - `en`: `https://docs.kosmotheka.com/en/using-the-app/`

  Usa `languageOf(i18n.language)` de `app/src/i18n`, igual que allí.
- Mira que `opener:default` deja abrir esa dirección (los otros enlaces externos ya funcionan con él). Si no, dilo antes de tocar los permisos.

## Novedades

Una línea en `next` de `app/src/whats-new.json`, en español e inglés, para personas. Por ejemplo: «La Ayuda enlaza con la guía completa, en docs.kosmotheka.com.» / «Help links to the full guide, at docs.kosmotheka.com.». Sin `switch`: abrir un enlace no envía nada por sí solo.

## La guía

En `site/src/content/docs/es/using-the-app/` y `en/`, si alguna página describe la pantalla de Ayuda, añade una línea sobre el enlace. Si ninguna la describe, no toques nada y dilo.

## Al terminar

En `app/`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:i18n` y `check:whats-new`. Si tocas la guía: en `site/`, `npm run build && npm run check:links`.

El informe: los archivos cambiados, las claves nuevas y cómo se ve (una captura con `?mock` si es fácil).
