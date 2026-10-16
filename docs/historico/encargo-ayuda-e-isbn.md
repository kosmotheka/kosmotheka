# Encargo · La Ayuda al día y el ISBN con guiones en «Añadir»

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Son dos diferencias que encontraste al escribir la guía (ver `plans/repositorio-publico.md`). Toca `app/`. Si algo no encaja, para y dímelo.

## 1. Un ISBN con guiones o de 10 cifras, en el buscador de «Añadir»

**Hoy:** `isCode` (`app/src/components/add-search.tsx`) solo da por código un número de 8, 12 o 13 cifras seguidas. Si alguien escribe `978-84-450-7389-3` o `8445073893`, se busca como título. La casilla ISBN del formulario sí lo acepta (`Isbn::parse` en `crates/domain/src/isbn.rs`).

**Lo que hace falta:**
- Que el buscador reconozca también un **ISBN-10** (con `X` final) y un ISBN con **guiones o espacios**, y lo busque como código, igual que uno de 13 cifras seguidas.
- La lógica de qué es un ISBN válido **vive en Rust** (AGENTS.md: nada de dominio en la interfaz). Mira si ya hay un comando que normalice o valide un ISBN (`lookup_isbn` o similar). Si lo hay, la interfaz quita guiones y espacios y deja que Rust decida. Si no, propónmelo antes de añadir lógica de ISBN en TypeScript. Quitar guiones y espacios en la interfaz sí vale: no es una regla del dominio.
- Un EAN de 8 o 12 cifras (juegos, música) sigue funcionando como hoy.
- Test donde corresponda (Rust, si cambia algo allí).
- Una línea en `app/src/whats-new.json` (`next`), en los dos idiomas: «Al añadir, un ISBN con guiones o de 10 cifras se busca como ISBN».

## 2. La Ayuda de la app, al día

Los textos de `help.topics` en `app/src/i18n/locales/es.json` y `en.json` se han quedado atrás. Ya viste:
- «Mostrar código»: hoy es la pestaña «Mostrar mi código», y ya se abre en ella;
- «Guardar copia»: hoy es «Guardar copia de <biblioteca>»;
- al emparejar no cuenta el «¿De quién es?» con el rol, que pregunta la app si administras (D97).

**Lo que hace falta:** repasar **todos** los temas de la Ayuda contra la app de hoy, igual que hiciste con la guía (código e interfaz con `?mock`), y corregir lo que no coincida, en los dos idiomas. Mismo tono que la guía: llano y sin jerga. **No amplíes la Ayuda**: solo corrige lo que está mal o falta para entender un paso. Si un tema necesita más, apúntalo y lo llevamos a la guía.

En el informe, una línea por cambio: tema, qué decía y qué dice.

## Reglas

- Ningún texto de la interfaz escrito a mano en el código: claves de i18n en `en` y `es`.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:whats-new`; si tocas Rust, `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. La salida real de cada uno.
