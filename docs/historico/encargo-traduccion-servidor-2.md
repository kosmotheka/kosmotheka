# Encargo · Montar tu servidor, segunda tanda, en inglés

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. **Solo `site/src/content/docs/en/run-your-server/`.** No toques nada de `es/` ni de `en/using-the-app/`: otra sesión trabaja a la vez en `es/`.

## Qué

Las tres páginas nuevas de `es/run-your-server/` en inglés, con **el mismo nombre de archivo**, y lo que cambió en las que ya estaban:

| Español (origen) | Inglés |
|---|---|
| `connect-the-app.md` | crear |
| `administration.md` | crear |
| `what-the-server-keeps.md` | crear |
| `index.md` | **ya existe:** añade la lista «Después de montarlo» con las tres páginas, como en español |
| `backups.md`, `updating.md`, `troubleshooting.md` | **ya existen:** solo cambia `sidebar.order` para que sea el mismo que en español (8, 9 y 10) |

Los nombres de la interfaz (entre «comillas») se sacan de `app/src/i18n/locales/en.json` por su clave, como en las guías de usuario.

## Cómo, paso a paso

1. **El lector** sabe administrar un servidor Linux, pero no conoce el proyecto. Inglés técnico normal, claro y directo, de tú («you»).
2. **El bloque de cabecera** (`---` … `---`): traduce `title` y `description`. Copia `sidebar.order` sin cambiarlo.
3. **Dentro de los bloques de código** no se traducen las órdenes, las rutas reales, los nombres de variables ni la configuración. **Sí se traducen los ejemplos inventados** (un dominio, un correo o un nombre de ejemplo).
   - Los **comentarios** dentro de un bloque (líneas con `#`) sí se traducen.
   - Si un comentario lleva un valor de ejemplo, el valor se queda igual.
4. **Los nombres de variables** (`DATABASE_URL`, `REGISTRATION_ALLOWED_EMAILS`…), de archivos y de órdenes se quedan tal cual, también en el texto.
5. **Palabras fijas:**

   | Español | Inglés |
   |---|---|
   | servidor | server |
   | copia de seguridad | backup |
   | restaurar | restore |
   | la nube | the cloud |
   | cuenta | account |
   | biblioteca | library |
   | dispositivo | device |
   | archivo | file |
   | variable de entorno | environment variable |
   | proxy inverso | reverse proxy |

6. **Los enlaces:**
   - entre páginas de la guía (`../with-docker/`, `./configuration/`…), se quedan **exactamente igual**;
   - los enlaces a GitHub, igual;
   - a la web en español (`kosmotheka.com/es/…`), a la inglesa: `/es/privacidad.html` → `/en/privacy.html`.
7. No es palabra por palabra, pero **no quites ni añadas nada** de lo que cuenta: ni pasos, ni avisos, ni valores.

## Al terminar

1. `distrobox enter library-dev -- bash -ic 'cd site && npm run build && npm run check:links'`. Tiene que acabar en «All internal links resolve.».
2. **El informe**, corto:
   - las tres páginas, cada una con su número de palabras;
   - cualquier frase del español que te pareciera confusa o que no supieras traducir.
