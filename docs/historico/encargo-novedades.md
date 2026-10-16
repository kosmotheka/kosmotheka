# Encargo · Pantalla de «Novedades» tras actualizar

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Toca `app/` (React y Rust), `AGENTS.md`, `.github/workflows/ci.yml` y `docs/`. **Empieza por la fase 0 y espera la respuesta de la sesión coordinadora antes de escribir código.**

## Para qué

Por transparencia: quien actualiza la app ve qué ha cambiado sin tener que buscarlo. Lo acordó el maintainer el 13/10/2026; el detalle está en `docs/ROADMAP.md`, Fase 8, «Pantalla de "Novedades" tras actualizar». Léelo entero: estas son sus reglas, no se rediscuten.

- **Sale una sola vez**, con lo nuevo desde la última versión que esa persona vio; si se saltó varias, las ve juntas. Se puede volver a consultar desde «Acerca de» (`app/src/components/about-dialog.tsx`).
- **De tres a cinco líneas por versión, escritas para personas.** No entran arreglos internos ni refactorizaciones.
- **Lo que cambia qué sale del dispositivo va siempre arriba, con su interruptor** (por ejemplo, una fuente de datos nueva): la pantalla deja activarlo o desactivarlo ahí mismo. Sustituye a repetir la bienvenida entera.
- **Un solo fichero dentro de la app, en español e inglés, con una línea por cambio**, escrita cuando se hace el cambio, no al publicar. De ahí sale también el texto de Google Play (`docs/publicacion/novedades.md`).
- **Para no olvidarse:** una regla en `AGENTS.md` (un cambio que un usuario notaría no está terminado sin su línea de novedades) y una comprobación automática que falla si la versión de la app no tiene su entrada. La comprobación asegura que hay entrada, no que esté completa.
- Descartado: generarlo desde los mensajes de commit.

## Lo que ya existe y sirve de modelo

- La bienvenida: `WELCOME_VERSION` en `app/src/components/welcome-dialog.tsx`, `settings::welcome_seen` / `set_welcome_seen` en Rust y su uso en `app/src/App.tsx` (la puerta de D97: sin nombre, solo la bienvenida). La última versión vista de las novedades se guarda igual.
- La versión instalada: `appVersion()` en `app/src/lib/api.ts`.
- Comprobaciones de la CI escritas como script de npm: `check:spec`, `check:codes` (`app/scripts/`, y su paso en `.github/workflows/ci.yml` con `working-directory: app`).

## Fase 0 · Propuesta (sin código)

Mándame un informe corto con:

1. **El fichero.** Dónde vive, qué formato (que lo lean la app y el script de la CI sin duplicarlo) y cómo se escriben las líneas de un cambio que aún no tiene número de versión: el número lo elige el maintainer al publicar. Propón cómo conviven «lo que va en la próxima» y la comprobación, sin que la CI falle todo el tiempo entre versiones.
2. **Cuándo sale.** Una instalación nueva no debe verla (ya tiene la bienvenida); quien viene de una versión anterior a que existiera la pantalla, sí. Cómo se distinguen, y qué pasa si la bienvenida y las novedades tocan el mismo arranque.
3. **El interruptor.** Cómo marca una línea que cambia qué sale del dispositivo y a qué ajuste está ligada. Hoy no hay ninguna línea así en la próxima versión: basta con que el formato lo permita y que haya una prueba.
4. **El contenido de la próxima versión**, en español e inglés, de tres a cinco líneas, a partir de `docs/publicacion/novedades.md`, «Próxima versión»: escrito para personas, sin términos internos. Lo que haya que hacer a mano (borrar los datos de prueba) cuenta como línea si una persona lo necesita.
5. **Tamaño.** Lo que se toca y si cabe en una entrega.

## Reglas

- Textos de la interfaz con claves de i18n en `en` y `es`. Las líneas de novedades viven en su fichero, no en los JSON de traducción, salvo que propongas otra cosa con motivo.
- La lógica de «qué versiones faltan por ver» va en Rust o en una función pura con su prueba, no repartida por los componentes.
- Mock de la interfaz (`?mock`) con novedades que ver.
- Si algo contradice una decisión aceptada o el encargo crece, **para y dímelo**.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`, `check:codes` y la comprobación nueva; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. La salida real de cada uno.

Documentación: la regla en `AGENTS.md` (y la tabla de comandos), el paso de la CI, `docs/ROADMAP.md` (Fase 8) y una nota en `docs/publicacion/novedades.md` de que el texto de Play sale del fichero nuevo.
