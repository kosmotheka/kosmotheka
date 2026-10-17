# Encargo · Los idiomas, en un solo sitio por parte

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Refactorización: nada cambia para quien usa la app**, ni la API ni el formato de los archivos. Aprobado por el maintainer el mismo día. Sale de lo que viste al escribir «Añadir un idioma» (`site/src/content/docs/es/contribute/add-a-language.md`, la lista de sitios con «español o inglés» escrito a mano). Prepara la traducción colaborativa del roadmap: quien añada un idioma tiene que tocar una lista, no buscar por el código.

## Qué

Cada parte tiene **una sola lista** de idiomas, y lo demás la lee:

1. **La interfaz:** `supportedLanguages` de `app/src/i18n` ya existe. Que la usen:
   - `whats-new-dialog.tsx`;
   - `about-dialog.tsx` (los enlaces a la privacidad por idioma);
   - `add-search.tsx`;
   - lo demás que encuentres con `es`/`en` escrito a mano.

   Donde hace falta algo por idioma (la URL de la política, por ejemplo), un mapa junto a la lista, con el inglés como respaldo para un idioma que no lo tenga.
2. **Rust:**
   - **`whats_new.rs`, `Line { es, en }`:** que lea las líneas como un mapa idioma → texto (el JSON sigue igual: `{"es": …, "en": …, "switch": …}`), con el inglés como respaldo. Los tests de `whats_new` siguen pasando sin cambiar lo que comprueban.
   - **`tmdb_tag`:** una tabla idioma → etiqueta de TMDB (`es` → `es-ES`, `en` → `en-US`), con `en-US` como respaldo.
3. **`app/scripts/check-whats-new.mjs`:** los idiomas que exige, leídos de `src/i18n/locales/` (los nombres de archivo) en lugar de `["es", "en"]`.
4. **Servidor:** los tres `#[Assert\Choice(choices: ['en', 'es'])]` (`RegisterAccountRequest`, `UpdateAccountRequest`, `GoogleStartRequest`) pasan a leer **una sola lista**. Lo propio de Symfony: un enum respaldado `App\Enum\Locale` y `#[Assert\Choice(callback: [Locale::class, 'values'])]` o `Assert\Choice` con el enum, lo que admita Symfony 8.1; míralo en `vendor/`. Los correos (`translations/emails.*.yaml`) se quedan como están: son traducciones, no una lista.
5. **La página «Añadir un idioma»** (`es/` y `en/`): la lista de sitios a mano pasa a decir dónde está la lista de cada parte.

## Cómo

- Los tests que ya existen son la red: ninguno debería cambiar lo que comprueba. Si uno tiene que cambiar, di por qué.
- Sin línea en `whats-new.json`: no hay cambio visible.

## Al terminar

En `app/`:
- `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:whats-new` y `check:i18n`;
- `cargo fmt`, `clippy` y `cargo test`;
- en `server/`, `make fix` y **un solo `make check`**;
- en `site/`, `npm run build && npm run check:links`.

El informe: cada sitio cambiado y dónde está ahora la lista de cada parte.
