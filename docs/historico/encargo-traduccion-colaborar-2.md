# Encargo · La parte «Colaborar», segunda tanda, en inglés

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. **Solo `site/src/content/docs/en/contribute/`.** No toques nada de `es/` ni de `en/using-the-app/` ni de `en/run-your-server/`: otra sesión trabaja a la vez en `es/`.

## Qué

Las cinco páginas nuevas de `es/contribute/` en inglés, con **el mismo nombre de archivo**, y el índice al día:

| Español (origen) | Inglés |
|---|---|
| `index.md` | `index.md`: **ya existe. Sustitúyelo** por la traducción del español entero, que ahora tiene el grupo «Cómo funciona por dentro» |
| `sync-engine.md` | crear |
| `operations-spec.md` | crear |
| `security.md` | crear |
| `audit-method.md` | crear |
| `report-a-vulnerability.md` | crear |
| `how-to-contribute.md` | **ya existe:** en español cambió el paso de la migración (`schemaVersion` en `spec/sync.json`); pasa ese cambio al inglés, sin tocar lo demás |

`sync-engine.md` es la más larga y la más importante: un ejemplo con Ana y Hugo que se retoma en cada parte. Que se siga igual de bien en inglés.

## Cómo, paso a paso

1. **El lector** programa y quiere colaborar, pero no conoce el proyecto. Inglés técnico normal, claro y directo, de tú («you»).
   - **El glosario** (`glossary.md`): la columna del español («obra», «ejemplar»…) se queda en español, porque es la de los documentos de trabajo. La columna del código se queda igual. Se traduce la explicación.
   - Los textos de la interfaz citados (entre «comillas»), igual que en las traducciones anteriores: el valor de la misma clave en `app/src/i18n/locales/en.json`.
2. **El bloque de cabecera** (`---` … `---`): traduce `title` y `description`. Copia `sidebar.order` sin cambiarlo.
3. **Dentro de los bloques de código** (` ``` `) no se traducen las órdenes, las rutas reales, los nombres de variables ni la configuración. **Sí se traducen los ejemplos inventados** (un dominio, una contraseña, un nombre de archivo de ejemplo), que en la guía del servidor se quedaron en español por error.
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
   | operación · cambio | operation · change |
   | reloj lógico híbrido | hybrid logical clock |
   | casos de conformidad | conformance cases |
   | petición | request |
   | lápida | tombstone |
   | valla · sello | fence · seal |
   | fusión | merge |
   | modelo de amenazas | threat model |
   | hallazgo | finding |

6. **Los enlaces:**
   - entre páginas de la guía (`../with-docker/`, `./configuration/`…), se quedan **exactamente igual**;
   - los enlaces a GitHub, igual;
   - a la web en español (`kosmotheka.com/es/…`), a la inglesa: `/es/privacidad.html` → `/en/privacy.html`.
7. No es palabra por palabra, pero **no quites ni añadas nada** de lo que cuenta: ni pasos, ni avisos, ni valores.

## Al terminar

1. `distrobox enter library-dev -- bash -ic 'cd site && npm run build && npm run check:links'`. Tiene que acabar en «All internal links resolve.».
2. **El informe**, corto:
   - las seis páginas, cada una con su número de palabras;
   - cualquier frase del español que te pareciera confusa o que no supieras traducir.
