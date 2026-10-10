# Encargo 7.6.2 · Invitaciones: la app por dentro

> Encargo cerrado para la sesión de hitos, 10/10/2026. Segunda de las tres partes de 7.6 ([plan](fase-7.md), «7.6 Invitar con un enlace»). Sin commit, sin push, sin cambiar de rama. Las pantallas son la tercera parte: aquí, Rust, comandos, tipos y lo mínimo de interfaz para que un enlace abierto llegue a algún sitio.

## Qué hay

El servidor está hecho (7.6.1): lee «Invitaciones (7.6.1, D43)» en `docs/architecture/cloud.md` (rutas, reglas, códigos, límites, el aviso `invitation_accepted`). La especificación es **D43 con su enmienda del 10/10/2026** en `docs/DECISIONS.md`. Lee también `AGENTS.md`, D76, D79, D96, D98, la pieza de invitaciones de `docs/architecture/security.md`, y en `app/src-tauri/src/cloud/` cómo están hechos `CloudApi`, `HttpCloud`, `FakeCloud`, `sync_all`, `set_cloud_power` y los avisos (`events`).

## Qué construir (solo `app/` y `docs/`)

1. **Cliente:** las cinco rutas de invitaciones en el trait `CloudApi`, en `HttpCloud` y en `FakeCloud` (crear, listar, anular, mirar sin sesión, aceptar), con sus tipos. El enlace completo de una invitación `link` solo existe en la respuesta de crearla: **no se guarda** en la base de datos ni en registros.
2. **Invitar** (`cloud::invite` o como encaje), para la biblioteca activa:
   - **comprueba antes, en Rust**, lo mismo que el servidor, para poder explicarlo: este dispositivo administra, la biblioteca está enlazada a la nube de este servidor, tiene roles, y la nube tiene el poder de vincular. El servidor tiene la última palabra; la app no decide el permiso, solo evita ofrecer lo que será rechazado;
   - **persona nueva:** una operación firmada crea la fila `member` con `role: reader` (la app nunca crea ahí otro rol), se sube (`sync_library`), y solo entonces se pide la invitación al servidor con el id de esa persona. Si pedir la invitación falla, la persona se queda creada: dilo en el resultado para que la pantalla lo cuente, no la borres a escondidas;
   - **persona que ya está:** solo `email`; no se toca su fila;
   - `link` solo para una persona nueva. La regla de «nueva» es del servidor; la app solo ofrece `link` al crear a la persona en ese mismo paso;
   - el correo se valida en forma en Rust (lo justo) y viaja tal cual; no se guarda.
3. **Qué se puede hacer con cada persona:** un dato calculado en Rust para la pantalla de Personas (por ejemplo `Person.invitable: "email" | "none"` y si tiene una invitación pendiente, cruzando con la lista del servidor): no se invita a quien administra, tiene el acceso retirado o ya tiene cuenta. Y un estado de la biblioteca, «se puede invitar» o por qué no (sin nube, sin roles, la nube sin poder, sin sesión, el servidor no ofrece invitaciones: `GET /v1/server` dice `invitations`).
4. **Listar y anular** las invitaciones pendientes de la biblioteca activa.
5. **Abrir un enlace:**
   - una función que saca el token de un enlace `<base>/i/<token>` y lo valida en forma; la base es la del servidor de la cuenta o, sin cuenta, la del servidor por defecto. **Un enlace de otro servidor no se sigue a ciegas:** averigua cómo sabe hoy la app a qué servidor hablar sin sesión («Otro servidor», D77) y dime cómo queda; no inventes una regla que haga llamar a un servidor que diga el enlace;
   - **mirar** (sin sesión) y **aceptar** (con sesión). Tras aceptar, `sync_all`: la biblioteca baja y el dispositivo se vincula por el camino que ya existe. El resultado dice en qué biblioteca se ha entrado;
   - si no hay sesión, el token pendiente se recuerda **en memoria o en estado local de este dispositivo** hasta iniciar sesión o crear la cuenta, y caduca con la invitación; nunca en una operación;
   - **cómo llega el enlace a la app:** mira qué hay hoy de enlaces profundos (App Links en Android para `kosmotheka.app`, D79; esquema o argumento en escritorio; el plugin de Tauri). Si ya hay un mecanismo, engánchalo. Si no hay nada, **no lo montes en este encargo**: deja un comando `open_invitation(link)` que acepta el enlace pegado a mano, y dime qué haría falta (plugin, manifiesto, verificación del dominio) para decidirlo aparte.
6. **El aviso a quien administra:** el evento `invitation_accepted` del canal de avisos se convierte en un aviso de la app («Alguien ha aceptado tu invitación a «Casa»»), con la app abierta. Sin datos personales en el evento: la app sincroniza y, si puede, dice quién. Déjalo de forma que las notificaciones con la app cerrada (más adelante en la fase) puedan engancharse sin rehacerlo.
7. **Errores:** los códigos de invitación del servidor, con texto propio en `en` y `es` y en `docs/design/errors.md`. Texto llano: «Este enlace ya se usó», «Ha caducado: pide otro a quien administra», «Esta invitación es para otra cuenta».
8. **Interfaz, lo mínimo:** tipos y funciones en `api.ts`, y lo necesario para que el flujo se pueda ejercitar (por ejemplo, el aviso). Las pantallas de invitar y de abrir un enlace son 7.6.3: no las construyas, pero deja los comandos listos y el mock de desarrollo al día.

## Seguridad

- Casos hostiles con test antes de la defensa y visto fallar, con `FakeCloud`: un enlace de otro servidor; un token mal formado; la app nunca crea a la persona nueva con otro rol que lector, tampoco si el comando recibe otro; un dispositivo que no administra no llega a crear la persona ni a llamar al servidor; el token pendiente no sobrevive a cambiar de cuenta ni a «dejar el dispositivo a otra persona» (D81).
- El token y el enlace no se escriben en registros ni en mensajes de error.
- Nunca el `MutexGuard` de la base de datos a través de un `.await`.

## Límites

- Solo `app/` y `docs/`. Nada en `server/`, `spec/` ni `crates/`, ni migraciones del catálogo. Si hace falta algo del servidor (un dato que no da, un código que falta), **para y dímelo**.
- Ninguna regla nueva de permisos. Si algo contradice una decisión aceptada o el encargo crece, para y explícamelo.
- Modo enseñanza de Rust: las explicaciones, en el informe, no en comentarios.
- No deshagas nada con git. Rust en `distrobox enter library-dev -- bash -ic '…'`.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`. **Pásalos todos al final, después del último cambio**, y dime la salida real de cada uno. Actualiza `docs/architecture/cloud.md` y la línea de 7.6.2 en `docs/plans/fase-7.md` (sin marcarla). Informe corto: qué hiciste por punto, qué encontraste de enlaces profundos, qué decidiste por tu cuenta, tests hostiles y cuáles viste fallar, comprobaciones, y qué no pudiste comprobar.
