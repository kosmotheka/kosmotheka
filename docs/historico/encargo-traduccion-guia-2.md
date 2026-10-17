# Encargo · La segunda tanda de la guía de usuario, en inglés

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. **Solo crea archivos nuevos en `site/src/content/docs/en/using-the-app/`.** No toques nada de `es/` ni el `index.md` de `en/`: otra sesión está escribiendo a la vez en `es/` y el índice va después.

## Qué

Las seis páginas de la segunda tanda, ya revisadas, en inglés. Cada una con **el mismo nombre de archivo** que en español, para que el selector de idioma enlace la misma página:

| Español (origen) | Inglés (crear) |
|---|---|
| `es/using-the-app/people-and-permissions.md` | `en/using-the-app/people-and-permissions.md` |
| `es/using-the-app/remove-someone.md` | `en/using-the-app/remove-someone.md` |
| `es/using-the-app/the-cloud.md` | `en/using-the-app/the-cloud.md` |
| `es/using-the-app/away-from-home.md` | `en/using-the-app/away-from-home.md` |
| `es/using-the-app/lost-phone.md` | `en/using-the-app/lost-phone.md` |
| `es/using-the-app/delete-account.md` | `en/using-the-app/delete-account.md` |

## Cómo, paso a paso

1. Lee primero, como modelo de tono y de palabras, dos páginas ya traducidas: `en/using-the-app/share-at-home.md` y `en/using-the-app/backups.md`.
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

7. **Las imágenes:** deja las mismas rutas que en español.
8. **Los correos** (`privacy@kosmotheka.com`, `info@kosmotheka.com`) se quedan igual.

## Al terminar

1. `cd site && npm run build` dentro de library-dev: `distrobox enter library-dev -- bash -ic 'cd site && npm run build'`. Tiene que acabar en «Complete!».
2. Comprueba que `ls site/src/content/docs/en/using-the-app/` tiene los seis archivos nuevos.
3. **El informe**, corto:
   - las seis páginas, cada una con su número de palabras;
   - los nombres de la interfaz que no encontraste en `en.json`;
   - cualquier frase del español que te pareciera confusa o que no supieras traducir.
