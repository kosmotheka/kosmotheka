# Encargo 7.8 (cuarta parte) · Clave propia para TMDB y RAWG, y «Todas» en Proveedores

> Encargo cerrado para la sesión de hitos, 11/10/2026 ([plan](fase-7.md), «7.8 Proveedores con clave»). **Se lanza cuando la tercera parte (RAWG) esté revisada y en su commit.** Sin commit, sin push, sin cambiar de rama. Solo `app/` y `docs/`. Nada en `server/`, `spec/` ni `crates/`.

Acordado con el maintainer el 11/10/2026, punto por punto.

## 1. Clave propia para TMDB y RAWG (D62)

D62: **todo proveedor que pida clave admite la clave propia en Ajustes**, sin nube, como Google Books. Hoy TMDB y RAWG solo funcionan con cuenta.

- En Proveedores, las tarjetas de TMDB y RAWG ganan el campo de clave, con su ayuda «Cómo conseguir una clave» (enlace a la página de cada uno), igual que Google Books. `KEYED_PROVIDERS` en `settings.rs` crece.
- **Orden, el mismo que en Google Books:** clave propia → la app llama **directamente** al proveedor, sin pasar por el servidor; si no hay clave, servidor (sesión, oferta, fuente encendida); si no, no se pregunta (no hay modo anónimo en estos dos).
- TMDB: la clave propia es su «API Read Access Token», por cabecera `Authorization: Bearer`. RAWG: su clave va en la dirección de la llamada (no admite otra cosa): que ningún error ni log de la app la enseñe.
- La llamada directa pasa por **las mismas comprobaciones** que la respuesta del servidor (8 candidatos, textos cortados, portadas solo de su sitio, números posibles): lo que venga de TMDB o RAWG es tan poco de fiar como lo que venga de un servidor. Reutiliza los validadores; si hacen falta dos lectores (el del formato del servidor y el del formato del proveedor), que compartan los recortes.
- Una clave rechazada (`401`/`403`) da el aviso de clave incorrecta que ya existe (`WarningKind::BadKey`); cuota, el de cuota.
- Con clave propia, la fuente cuenta como disponible aunque no haya sesión, y deja de decir «llega con la cuenta».
- Tests sin red, con respuestas grabadas en `tests/fixtures/` (escribe a mano unas pequeñas con la forma documentada de cada API y dilo en el informe: no hay clave real para grabarlas).

## 2. «Todas», la primera y preseleccionada

Hoy, con «Libros» seleccionado, ver abajo el crédito de TMDB parece un proveedor descolocado, y no hay un sitio donde ver todas las fuentes.

- La lista de tipos empieza por **«Todas»**, seleccionada al abrir. A la derecha, **todas las fuentes, cada una una sola vez**.
- En «Todas», cada tarjeta dice **qué tipos cubre** («Libros · Películas · Series…»), con los nombres de tipo que ya usa la app.
- Al pulsar un tipo se filtra, como ahora.
- **Un interruptor por fuente**, que es lo que ya hay por dentro (`disabled_sources` no distingue tipos). No cambies el modelo.
- En el móvil, donde se ve primero la lista de tipos y luego se entra, «Todas» es la primera de esa lista.
- Los contadores por tipo («5 de 5») siguen; decide qué enseña «Todas» y dímelo.

## 3. «Créditos»

La sección de abajo deja de llamarse «Fuentes de datos» y pasa a **«Créditos»**, más discreta (tamaño y color de pie de página), con lo que cada proveedor exige: el logo y el aviso de TMDB, y RAWG con su enlace. Sigue viéndose con cualquier tipo seleccionado.

## Cuidado con la pantalla

El maintainer vio que la sección de abajo **se pintaba encima de la lista** (un contenedor con `min-h-0` que encogía). Ya está arreglado: el diálogo se desplaza entero y nada dentro debe poder encogerse. No lo reintroduzcas, y comprueba a mano el ancho de escritorio y el de móvil si puedes abrir la app; si no puedes, dilo.

## Casos hostiles (test antes de la defensa, visto fallar)

1. Con clave propia, **ninguna petición sale hacia el servidor de la cuenta** para esa fuente, haya o no sesión.
2. Sin clave y sin sesión, no sale ninguna petición hacia nadie para TMDB ni RAWG.
3. La respuesta directa del proveedor con basura (cien resultados, textos enormes, portadas de otro sitio, ids imposibles): mismos recortes que por el servidor.
4. La clave propia no aparece en ningún mensaje de error que llegue a la pantalla ni en un log de la app.
5. Una clave en blanco o solo con espacios no es una clave.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. La salida real de cada uno.

Documentación: `docs/architecture/cloud.md` (el orden clave propia → servidor), `docs/research/proveedores.md`, la ayuda de la app si menciona los proveedores, `docs/plans/fase-7.md` y `docs/ROADMAP.md`.

Informe: qué has hecho, qué decidiste tú y por qué, qué tests viste fallar antes, qué no has podido comprobar (en pantalla, sobre todo) y cualquier cosa rara.
