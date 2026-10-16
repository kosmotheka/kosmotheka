# Encargo · Pantalla «Licencias» en la app

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Toca `app/`, quizá `.github/workflows/ci.yml`, `THIRD-PARTY-NOTICES.md` y `docs/`. **Empieza por la fase 0 y espera la respuesta de la sesión coordinadora antes de escribir código.**

## Para qué

`docs/ROADMAP.md`, Fase 8, «Antes de abrir el repositorio» (auditoría del 03/10/2026): una pantalla «Licencias» en la app con los avisos de terceros, para que viajen también en los instaladores y no solo en el repositorio. Es requisito antes de abrir el código y de publicar fuera de la prueba interna.

## Lo que existe

- `THIRD-PARTY-NOTICES.md` en la raíz: solo Figtree (OFL 1.1, con el texto en `web/assets/fonts/OFL.txt`) y Lucide (ISC). Dice que las demás dependencias «guardan su licencia en su paquete».
- «Acerca de» (`app/src/components/about-dialog.tsx`), que ya tiene el botón «Novedades» desde `d18ba30`.
- «Créditos» de los proveedores de datos (`app/src/components/source-credits.tsx`, desde Proveedores): son las atribuciones que piden los catálogos (TMDB, RAWG…), no licencias de código. No se mezclan, pero «Licencias» puede enlazar con ellas o al revés, si lo propones.

## La duda que hay que resolver primero

Las licencias MIT, Apache-2.0, BSD e ISC piden que su aviso acompañe **también a las copias binarias**. Los instaladores llevan compiladas cientos de crates de Rust y los paquetes de npm que entran en el bundle de la interfaz, y hoy ninguno de esos avisos viaja con la app. Con solo Figtree y Lucide, la pantalla no cumpliría lo que promete.

## Fase 0 · Propuesta (sin código)

Mándame un informe corto con:

1. **Qué debe listar.** Las dependencias que de verdad van dentro de los instaladores (escritorio y Android): las crates del binario, sin las de desarrollo ni las de pruebas, y los paquetes de npm que acaban en `app/dist`. Cuántas son, y qué licencias aparecen (sale de `Cargo.lock` y `package-lock.json`). Si alguna pide algo más que el aviso (MPL-2.0, por ejemplo, o una sin licencia clara), dilo.
2. **Cómo se genera.** Una lista escrita a mano se queda vieja con la primera actualización. Propón cómo se genera desde los lockfiles y qué herramienta usarías (por ejemplo `cargo-about` para Rust y, para npm, leer `license` y el fichero de licencia de cada paquete del bundle). **Regla del maintainer:** las herramientas auxiliares van en Docker, con la versión fijada y nada instalado en su máquina. Si una herramienta no cabe así, dilo y propón otra forma.
3. **Cuándo se genera y dónde vive.** ¿Se genera un fichero que se commitea y la CI comprueba que está al día (como `check:spec`), o se genera al compilar? Ten en cuenta que la CI compila para Linux, Windows y Android, y que el fichero generado puede ser grande: propón formato y tamaño.
4. **La pantalla.** Desde «Acerca de»: qué se ve (por ejemplo, la lista agrupada por licencia, con el texto de cada licencia una vez y los nombres y copyright de cada paquete), cómo se lee en el móvil y si hace falta buscar. Textos con i18n en `en` y `es`; los textos de las licencias van tal cual, en inglés.
5. **`THIRD-PARTY-NOTICES.md`.** Si se queda como resumen a mano (fuentes e iconos, que no salen de los lockfiles de la misma forma) o pasa a generarse.
6. **Tamaño.** Lo que se toca y si cabe en una entrega.

## Reglas

- Ningún texto de la interfaz escrito a mano en el código: claves de i18n en `en` y `es`.
- Nada de red al compilar ni en las pruebas.
- Si algo contradice una decisión aceptada o el encargo crece, **para y dímelo**.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`, `check:codes`, `check:whats-new` y la comprobación nueva si la hay; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. La salida real de cada uno.

Documentación: `docs/ROADMAP.md` (Fase 8), la tabla de comandos de `AGENTS.md` si hay comando nuevo, y una línea en `app/src/whats-new.json` (`next`) solo si una persona lo notaría.
