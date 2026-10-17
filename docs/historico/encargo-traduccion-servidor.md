# Encargo · La guía para montar tu servidor, en inglés

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. **Solo `site/src/content/docs/en/run-your-server/`.** No toques nada de `es/` ni de `en/using-the-app/`: otra sesión trabaja a la vez en `es/`.

## Qué

Las siete páginas de `es/run-your-server/` en inglés, con **el mismo nombre de archivo**:

| Español (origen) | Inglés |
|---|---|
| `index.md` | `index.md`: **ya existe y es más corto. Sustitúyelo** por la traducción del español entero |
| `without-docker.md` | crear |
| `with-docker.md` | crear |
| `configuration.md` | crear |
| `backups.md` | crear |
| `updating.md` | crear |
| `troubleshooting.md` | crear |

## Cómo, paso a paso

1. **El lector** sabe administrar un servidor Linux, pero no conoce el proyecto. Inglés técnico normal, claro y directo, de tú («you»).
2. **El bloque de cabecera** (`---` … `---`): traduce `title` y `description`. Copia `sidebar.order` sin cambiarlo.
3. **No se traduce nada de dentro de los bloques de código** (` ``` `): ni órdenes, ni rutas, ni nombres de variables, ni el contenido de los archivos de configuración.
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
   - las siete páginas, cada una con su número de palabras;
   - cualquier frase del español que te pareciera confusa o que no supieras traducir.
