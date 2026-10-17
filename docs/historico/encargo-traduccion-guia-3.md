# Encargo · La tercera tanda de la guía de usuario, en inglés

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. **Solo crea archivos nuevos en `site/src/content/docs/en/using-the-app/`,** y en `index.md` de esa carpeta quita los seis «(in Spanish for now)». No toques nada de `es/`: otra sesión está escribiendo a la vez en `es/contribute/`.

## Qué

Las seis páginas de la tercera tanda, ya revisadas, en inglés. Cada una con **el mismo nombre de archivo** que en español:

| Español (origen) | Inglés (crear) |
|---|---|
| `es/using-the-app/kinds.md` | `en/using-the-app/kinds.md` |
| `es/using-the-app/marks.md` | `en/using-the-app/marks.md` |
| `es/using-the-app/series.md` | `en/using-the-app/series.md` |
| `es/using-the-app/several-libraries.md` | `en/using-the-app/several-libraries.md` |
| `es/using-the-app/data-sources.md` | `en/using-the-app/data-sources.md` |
| `es/using-the-app/faq.md` | `en/using-the-app/faq.md` |

**`kinds.md` tiene muchos nombres de la interfaz** (campos, formatos y géneros de cada tipo): búscalos todos en `es.json` y copia su valor de `en.json`, como en el paso 4. Las listas de géneros, también.

## Cómo, paso a paso

1. Lee primero, como modelo de tono y de palabras, dos páginas ya traducidas: `en/using-the-app/share-at-home.md` y `en/using-the-app/the-cloud.md`. Entre páginas, los enlaces van con `../página/` (no `./`).
2. **El bloque de cabecera** (`---` … `---`): traduce `title` y `description`. Copia `sidebar.order` sin cambiarlo.
3. **Inglés llano**, para alguien que no programa: frases cortas, de tú («you»), sin jerga. No es palabra por palabra: si en inglés se dice mejor de otra forma, dilo así, pero sin quitar ni añadir nada de lo que cuenta.
4. **Los nombres de botones y pantallas.** En español van entre «comillas angulares». Para cada uno:
   - busca el texto español exacto en `app/src/i18n/locales/es.json`;
   - mira cuál es su clave;
   - copia el valor de esa misma clave en `app/src/i18n/locales/en.json`, entre "comillas dobles";
   - **no lo traduzcas tú.** Si no lo encuentras, déjalo en español y apúntalo en el informe.
5. **Palabras fijas:**

   | Español | Inglés |
   |---|---|
   | biblioteca | library |
   | ejemplar | copy |
   | dispositivo | device |
   | archivo | file |
   | copia de seguridad | backup |
   | servicios de internet | online services |
   | emparejar | pair |
   | obra · ficha | work · details page (mira cómo las llama `en.json` y úsalo) |
   | «Lo deseo» · «Me falta» | "I want it" · el valor de `en.json` |
   | la nube | the cloud |
   | cuenta | account |
   | quien administra / administrador | admin |
   | editor · lector | editor · reader |
   | petición | request |
   | invitación | invitation |

6. **Los enlaces:**
   - entre páginas de la guía (`../first-steps/`, `./the-cloud/`…), se quedan igual;
   - a la web en español (`kosmotheka.com/es/…`), van a la inglesa:

   | Español | Inglés |
   |---|---|
   | `/es/privacidad.html` | `/en/privacy.html` |
   | `/es/borrar-cuenta.html` | `/en/delete-account.html` |
   | `/es/preguntas.html` | `/en/faq.html` |
   | `/es/descargas.html` | `/en/download.html` |

7. **Las imágenes:** apuntan a las de español, como en `en/using-the-app/share-at-home.md`: `![…](../../es/using-the-app/kinds-add.webp)`. El texto alternativo, en inglés.
8. **Los correos** (`privacy@kosmotheka.com`, `info@kosmotheka.com`) se quedan igual.

## Al terminar

1. `distrobox enter library-dev -- bash -ic 'cd site && npm run build && npm run check:links'`. Tiene que acabar en «All internal links resolve.».
2. Comprueba que `ls site/src/content/docs/en/using-the-app/` tiene los seis archivos nuevos.
3. **El informe**, corto:
   - las seis páginas, cada una con su número de palabras;
   - los nombres de la interfaz que no encontraste en `en.json`;
   - cualquier frase del español que te pareciera confusa o que no supieras traducir.
