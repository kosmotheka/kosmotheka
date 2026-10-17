# Encargo · Lo que salió de la prueba en pantalla de la 0.2.1

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push, sin cambiar de rama. El maintainer probó la app de escritorio con la 0.2.1 y salieron cuatro cosas. Va antes de desplegar: corto y sin ampliar nada. Toca `app/`. El maintainer tiene `tauri dev` abierto: no lo cierres; los cambios de la interfaz le llegan solos.

## 1. «Licencias» necesita un botón para cerrar abajo

Como «Novedades»: un botón «Entendido» al final, para no tener que volver arriba a la X. Usa el mismo componente o patrón que «Novedades» y la misma clave de i18n si ya existe.

## 2. «Fuentes de datos» («Créditos») desde «Acerca de»

- El mismo botón «Entendido» abajo.
- **Al abrirla desde «Acerca de», se cierra «Acerca de»**; con «Licencias» y «Novedades» no pasa. Que se comporte igual que las otras: al cerrarla se vuelve a «Acerca de». Mira cómo lo hacen «Licencias» y «Novedades» (`about-dialog.tsx`) y haz lo mismo.

## 3. En escritorio, la ventana se abre maximizada

Hoy abre a 1024×720 y algún botón del menú se corta al empezar. Que abra maximizada (`"maximized": true` en `tauri.conf.json`, ventana principal). Comprueba que no afecte a Android y que el tamaño mínimo siga igual. Si Tauri recuerda el tamaño entre arranques (plugin `window-state`), dímelo; no lo añadas.

## 4. Un número de código que no se encuentra pregunta «¿Qué es?», aunque el tipo «Libro» ya esté elegido

El maintainer escribió en «Añadir» un ISBN con guiones, con «Libro» elegido. **El número de prueba que le dio la sesión coordinadora tenía mal el dígito de control** (`978-84-450-7389-3`; el bueno acaba en 6). Rust no lo dio por ISBN, lo buscó como código de barras, no lo encontró y la app preguntó qué era. Es confuso por dos lados:

- **Un número con forma de ISBN (978 o 979 y 13 cifras, o 10 con su X) cuyo dígito de control no cuadra:** la app tiene que decirlo, no preguntar qué es. Por ejemplo: «Este número no es un ISBN válido: revisa que esté bien escrito». Que lo decida Rust (`Isbn::parse` ya distingue un error de dígito de control) y la interfaz solo enseñe el mensaje.
- **Un ISBN válido que ningún servicio encuentra:** es un libro seguro. No preguntes el tipo: ofrece «Añadir a mano» como libro, con el ISBN ya puesto.
- **Un código que no es ISBN** (EAN de juego, disco…): se queda como hoy. Pero si el tipo ya está elegido, ¿hace falta preguntarlo otra vez? Mira el flujo y propón lo mínimo en el informe; no lo cambies sin decírmelo.

Tests en Rust de lo que decida Rust. Textos en `en` y `es`.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`; si tocas Rust, `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. Una línea en `whats-new.json`, en la entrada `0.2.1` (ya tiene número), solo si alguno se nota y cabe en cinco líneas. Si no cabe, dímelo.
