# Encargo 7.6.4 · Que un enlace abra la app

> Encargo cerrado para la sesión de hitos, 11/10/2026. Sin commit, sin push, sin cambiar de rama. Deshace el aplazamiento de esta mañana ([plan](fase-7.md), «Que el enlace abra la app sola»): el maintainer quiere «Continuar con Google» (D77), que necesita el mismo mecanismo para volver a la app. Este encargo monta **el mecanismo y las invitaciones**; Google es el encargo siguiente y se apoya en este.

## Qué hay

Lee en `docs/architecture/cloud.md` «Invitaciones en la app (7.6.2, D43)» (el párrafo «Enlaces profundos: qué hay hoy») y «Las pantallas de las invitaciones (7.6.3)», D79 y D43 en `docs/DECISIONS.md`, y `docs/publicacion/kosmotheka.app/README.md`. Hoy todo enlace entra pegado a mano por `open_invitation(link)`, que ya valida su forma y ya rechaza los de otro servidor. Eso no cambia: **ninguna regla nueva sobre qué enlaces se siguen.**

Consulta la documentación actual de `tauri-plugin-deep-link` y `tauri-plugin-single-instance` (Tauri 2) antes de escribir nada: cómo se declaran los enlaces de Android y los esquemas de escritorio, y qué genera cada uno en `gen/android/` y en los instaladores.

## Qué construir (solo `app/` y `docs/`)

1. **El mecanismo:** `tauri-plugin-deep-link`, y en escritorio `tauri-plugin-single-instance`, para que un enlace abierto con la app ya en marcha llegue a esa ventana y no arranque otra. Versiones fijadas como las demás; `cargo deny check` y `cargo machete` tienen que seguir pasando (dime la salida, o que no los tienes).
2. **Android, enlaces verificados** para `https://kosmotheka.app/i/…` (y solo esa ruta de ese dominio), con `android:autoVerify="true"`. `gen/android/` está en el repositorio: toca lo mínimo y dime exactamente qué líneas cambian y si las genera la herramienta o las escribes a mano.
3. **Escritorio y el esquema propio:** registra el esquema `kosmotheka` (lo usará el encargo de Google para volver a la app). En este encargo **el esquema no abre invitaciones**: `kosmotheka://i/<token>` sigue rechazado, como hoy (un enlace sin dominio no dice de qué servidor es; está por decidir). Un enlace con ese esquema que no sea de nada conocido se ignora sin error visible.
4. **Un solo sitio por el que entra todo enlace**, en Rust: recibe la dirección, decide qué es por su forma y la pasa a quien toca. Hoy solo hay un destino: una invitación, que va por el mismo camino que la pegada a mano (`open_invitation`) y abre el diálogo «Tengo una invitación» ya con la biblioteca a la vista. Déjalo de forma que el encargo de Google añada su destino sin rehacerlo.
5. **La app cerrada, abierta y sin sesión:** el enlace con el que arranca la app, el que llega con la app abierta, y el que llega sin sesión (se queda esperando, como ya hace el pegado). Sin perder el enlace si la interfaz aún no ha cargado.
6. **El fichero para Android**, como plantilla en `docs/publicacion/kosmotheka.app/.well-known/assetlinks.json`: el paquete `app.kosmotheka` y un hueco claro para las huellas SHA-256 (la de la firma de Google Play y la de subida). Explica en `docs/publicacion/firma-android.md` de dónde se saca cada huella y cómo se comprueba en un móvil (`adb shell pm get-app-links app.kosmotheka`). **No inventes huellas.**
7. **La pantalla:** al llegar un enlace se enseña lo mismo que al pegarlo. Si llega uno malo (de otro servidor, mal formado, caducado), el diálogo lo dice con los textos que ya existen.

## Seguridad: cualquiera puede lanzar un enlace a la app

Una página web, otra app o un mensaje pueden abrir `https://kosmotheka.app/i/lo-que-sea` o `kosmotheka://lo-que-sea` sin que la persona lo haya pedido. Tests hostiles antes de la defensa y vistos fallar (en Rust, con `FakeCloud`):

1. **Un enlace nunca hace nada por sí solo:** como mucho abre el diálogo. Entrar en una biblioteca pide siempre que la persona pulse. Ningún enlace acepta, sube, borra, cambia de biblioteca ni inicia o cierra sesión.
2. Un enlace de otro dominio, de otra ruta de `kosmotheka.app`, con usuario, parámetros o fragmento, o de un esquema desconocido: no llama a ningún servidor.
3. El enlace no decide a qué servidor se llama (ya es así: que siga siéndolo por este camino).
4. Cien enlaces seguidos no dejan a la app colgada ni abren cien diálogos: se atiende uno a la vez, y un enlace nuevo no sustituye en silencio a la invitación que la persona estaba mirando.
5. El enlace y su código no se escriben en registros ni en mensajes de error, tampoco los que el sistema pasa al arrancar (argumentos de la orden).
6. Un enlace que llega mientras se enseña otra cosa delicada (el diálogo de dejar el dispositivo, el de borrar) no la cierra ni la tapa.

## Límites

- Solo `app/` y `docs/`. Nada en `server/`, `spec/` ni `crates/`. Nada de Google todavía.
- Ninguna regla nueva de permisos ni de qué enlaces valen. Si algo contradice una decisión aceptada o el encargo crece, **para y dímelo**.
- Modo enseñanza de Rust y Tauri: las explicaciones (plugins, cómo llega un enlace a un proceso ya abierto), en el informe, no en comentarios.
- No deshagas nada con git. Rust y npm en `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`. **Todos después del último cambio**, con su salida real. Si puedes, comprueba que el proyecto de Android sigue generándose (dime qué orden usaste y qué no pudiste probar). Actualiza `cloud.md`, `docs/plans/fase-7.md` (la nota del aplazamiento: qué queda hecho y qué falta), `docs/publicacion/kosmotheka.app/README.md` y `firma-android.md`. Informe a la sesión `library` con SendMessage: qué hiciste por punto, qué cambia en `gen/android/`, tests hostiles y cuáles viste fallar, qué decidiste por tu cuenta, y **una lista paso a paso de qué probar en un móvil y en escritorio**, porque nada de esto se puede comprobar sin un dispositivo.
