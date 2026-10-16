# Encargo · «Licencias» resumida, con la lista completa en un `.txt`

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Sigue a `encargo-licencias.md` (commit `6e3e383`). Toca `app/` y `docs/`. No hace falta fase 0: si algo de aquí no encaja con el código, para y dímelo antes de seguir.

## Para qué

El maintainer no quiere 541 entradas en pantalla: nadie las lee. Lo que las licencias piden es que el aviso **viaje dentro de la app**, no que se lea en pantalla. Por eso la pantalla pasa a ser un resumen, y la lista completa se guarda como fichero de texto, generado con lo que la app ya lleva dentro. **Nunca se descarga de la red**: dejaría de ir «incluida en la copia».

La generación (`npm run licenses`), `app/src/licenses.json`, `check:licenses` y su paso en la CI **no cambian**.

## La pantalla (desde «Acerca de», como ahora)

1. **Una frase arriba:** «Kosmotheka se construye con N componentes de código abierto», con N contado del fichero (crates más paquetes de npm; los de `terms` aparte).
2. **Las licencias, resumidas:** una fila por tipo de licencia, con su número de paquetes y una línea en lenguaje llano de lo que pide (por ejemplo, MIT: «permite usarlo, copiarlo y cambiarlo si se mantiene el aviso»). Las expresiones compuestas (`MIT OR Apache-2.0`) se agrupan como hoy; para la línea explicativa basta un texto por cada licencia que aparezca, y uno genérico para las que no tengan el suyo. Los textos, con i18n en `en` y `es`.
3. **Los nombres destacados:** Tauri, React, SQLite (dominio público) y como mucho dos o tres más que tú veas importantes, cada uno con su licencia. Lista fija en el código, pero que la comprobación falle (o al menos la pantalla no los muestre) si uno deja de estar en el fichero.
4. **ML Kit y Play Services**, como hoy: nombre, para qué sirven y enlace a sus términos. Es lo único que no es libre.
5. **Botón «Guardar lista completa»:** abre el diálogo de guardar (`@tauri-apps/plugin-dialog`, como la exportación a CSV en `remove-library-dialogs.tsx`) con el nombre propuesto `kosmotheka-licencias.txt` (o `kosmotheka-licenses.txt` en inglés). Mensaje de éxito o de error, como en las demás exportaciones.

Fuera: el buscador y la lista desplegable por licencia.

## El fichero `.txt`

- **Lo escribe Rust**, en un comando propio (`export_licenses(path)`), con `open_picked(&app, &path, true)` como `export_csv`, para que funcione con las rutas `content://` de Android. **Ningún comando que escriba un texto que llegue de la interfaz**: el contenido sale de `include_str!` de `app/src/licenses.json` (o de `include_bytes!`, como veas) y lo arma Rust. Explícame lo nuevo de Rust en tu informe (modo enseñanza), no en comentarios.
- **Formato:** texto plano en inglés, legible en cualquier editor:
  - cabecera con el nombre de la app, la versión y una línea que explique qué es el fichero;
  - primero los términos de ML Kit y Play Services, con sus enlaces;
  - después, por cada paquete: nombre, versión, licencia y repositorio;
  - al final, el texto completo de cada licencia una sola vez, con la lista de paquetes que la usan.
- Prueba en Rust que el fichero sale con todos los paquetes de `licenses.json` y cada texto una vez (sin red ni ficheros fuera de un directorio temporal).

## Reglas

- Ningún texto de la interfaz escrito a mano en el código: claves de i18n en `en` y `es`. Quita las claves que dejen de usarse.
- El JSON se sigue cargando solo al abrir la pantalla (import dinámico); si el resumen ya no necesita los textos, mejor aún, pero no hace falta partir el fichero.
- `mock-ipc.ts`: el comando nuevo tiene que responder en el modo navegador.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

`npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`, `check:codes`, `check:whats-new`, `check:licenses`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`. La salida real de cada uno.

Documentación: lo que diga la pantalla en `THIRD-PARTY-NOTICES.md` y en `docs/publicacion/ficha-google-play.md`, si lo mencionan. En `whats-new.json`, si la línea de «Licencias» de `next` describe la lista, ajústala al resumen y el fichero.
