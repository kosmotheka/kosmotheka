# Encargo · Las últimas páginas de la documentación

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Mismas reglas que todas las anteriores: todo comprobado en el código de hoy, sin secretos, nada del servidor oficial, sin hitos ni fechas de trabajo. Solo en `site/src/content/docs/es/` (`library-haiku` traduce a la vez en `en/contribute/`: no lo toques).

## Para quien colabora (`contribute/`)

1. **`add-a-data-source.md` · Añadir una fuente de datos.**
   - El patrón de `app/src-tauri/src/metadata/`: el trait, un proveedor existente como modelo, el mapeo a `Candidate`, las respuestas grabadas en `tests/fixtures/` y las pruebas `live_*` ignoradas.
   - Dónde se registra la fuente y cómo sale en Ajustes › Proveedores (clave propia, interruptor).
   - Si la fuente pasa por el servidor, el patrón de `server/src/Metadata/`: `*Source`, `*HttpSource`, `*Search`, `ProviderGate`, la caché sin quién preguntó y los límites.
   - Lo que hay que tocar fuera del código: la línea de `whats-new.json` con su `switch`, la política de privacidad y la página «De dónde salen los datos».
   - Las condiciones del proveedor: leerlas antes, porque algunos no dejan guardar ni modificar sus datos.
2. **`add-a-language.md` · Añadir un idioma.**
   - Los archivos de `app/src/i18n/locales/` y cómo se registran.
   - Las listas cerradas (géneros, estados por tipo), que también tienen sus textos.
   - La comprobación que avisa de las claves que faltan (mira qué scripts o pruebas lo hacen).
   - `whats-new.json`.
   - La documentación y la web, que son aparte.

   Si añadir un idioma toca algo más (el servidor y sus correos, `locale` en la cuenta), dilo.

Añádelas al `index.md` de «Colaborar».

## Para quien usa la app (`using-the-app/`)

3. **`error-messages.md` · Mensajes de error y qué hacer.**
   - Los mensajes que una persona puede ver de verdad, agrupados: sin conexión, el servidor no responde, la cuenta (código caducado, demasiados intentos, segundo factor), compartir y emparejar, y copias de seguridad.
   - Para cada uno: el texto tal como sale (`es.json`), qué significa en llano y qué hacer.
   - Material: `docs/design/errors.md`, si sigue al día, y las claves de error de `es.json`.
   - No hace falta que estén todos: los que alguien se encontrará.
   - Revisión de jerga como en la guía: frases cortas, sin palabras internas.
4. **`whats-new.md` · Novedades de cada versión.**
   - Las versiones de `app/src/whats-new.json` (sin `next`), de la más nueva a la más antigua, con sus líneas en español.
   - Escrita a mano a partir del archivo.
   - Una línea arriba: la app las enseña al actualizarse, y se vuelven a ver en «Acerca de».

Añádelas al `index.md` de «Usar la app», en el grupo que les toque.

## Al terminar

- `cd site && npm run build && npm run check:links`.
- **El informe:**
  - cada página con sus palabras;
  - lo que no coincidía entre los documentos y el código;
  - si `docs/design/errors.md` está al día.
