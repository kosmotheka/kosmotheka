# Encargo 7.6.3 · Invitaciones: las pantallas

> Encargo cerrado para la sesión de hitos, 11/10/2026. Tercera y última parte de 7.6 ([plan](fase-7.md), «7.6 Invitar con un enlace»). Sin commit, sin push, sin cambiar de rama. **Sin pasar por Design:** se adapta `cloud-account/Invite` a mano.

## Qué hay

El servidor (7.6.1) y la app por dentro (7.6.2) están hechos: lee «Invitaciones (7.6.1, D43)» e «Invitaciones en la app (7.6.2, D43)» en `docs/architecture/cloud.md`. La especificación es **D43 con su enmienda del 10/10/2026**. En `app/src/lib/api.ts` están los tipos y funciones (`inviteStatus`, `inviteToLibrary`, `listInvitations`, `revokeInvitation`, `openInvitation`, `pendingInvitation`, `discardPendingInvitation`, `acceptInvitation`, `invitationReasonKey`, `Person.invitable`), marcados `@public` para knip.

El diseño de partida es `docs/design/screens/cloud-account/Invite.dc.html`, y **lo que cambia respecto a él** está en `docs/design/screens/README.md`, «Diferencias de `cloud-account/Invite` con lo decidido»: síguelo al pie de la letra. Mira cómo están hechos `components/people/access-dialog.tsx`, `person-sheet.tsx`, `components/cloud/autolink-switch.tsx` y `cloud-section.tsx` y construye igual (mismos componentes de shadcn, mismas claves i18n por zona).

## Qué construir

1. **Invitar, desde Personas** (solo lo ve quien administra):
   - un botón «Invitar» que abre un diálogo con **dos formas**, sin selector de rol: **«Por correo»** (nombre de la persona nueva y su correo) y **«Copiar enlace»** (solo el nombre). Texto llano bajo cada una: por correo, «le llegará un enlace; solo podrá usarlo con ese correo»; enlace, «quien lo abra primero entra; pásaselo solo a esa persona». En las dos, «Entra como lector; después puedes cambiar su rol aquí»;
   - en la ficha de una **persona que ya está** y con `invitable: "email"`: «Invitar por correo» (solo el correo; conserva su rol, y se dice). Con `invitable: "none"` no se ofrece nada;
   - **antes de ofrecerlo**, `inviteStatus`: si hay un bloqueo, el diálogo no enseña el formulario sino qué falta y un botón que lleva a arreglarlo (`not_signed_in` → iniciar sesión; `not_in_cloud` → subir la biblioteca; `no_roles` → activar personas y roles; `no_cloud_power` → el interruptor de la nube; `server_without_invitations` y `not_admin` → solo se explica);
   - **resultado:** por correo, «Enviado a …» ; enlace, el enlace con «Copiar» y «Compartir…» (el compartir del sistema si existe; si no, solo copiar), y «Vale una vez, hasta mañana a las 18:20» con la caducidad real. El enlace solo vive en ese diálogo: ni en estado global, ni en `localStorage`, ni en `console`. Al cerrar el diálogo se pierde, y se avisa antes («no podrás volver a verlo»);
   - si la persona se creó pero pedir la invitación falló (`InviteResult` lo dice), se cuenta: «Hemos añadido a Ana, pero no se pudo crear la invitación» y se ofrece reintentar por correo desde su ficha;
   - «Está aquí: usar el QR» lleva al emparejamiento que ya existe.
2. **Invitaciones pendientes, en Personas:** junto a cada persona con una invitación sin usar, «Tiene una invitación sin usar · caduca …» y «Anular» (con confirmación corta). A quien ya tiene una pendiente no se le crea otra sin anular la primera: mira qué responde el servidor y enséñalo. Las aceptadas dicen «Aceptó la invitación» mientras el servidor las liste. La lista se pide al abrir Personas y tras `invitation-accepted` y `cloud-changed`; si falla (sin red), Personas funciona igual y no enseña nada de invitaciones.
3. **Abrir un enlace:** una entrada «Tengo una invitación» (en la pantalla de la cuenta/nube y donde se entra sin bibliotecas) con un campo para **pegar el enlace**. Con `openInvitation`:
   - enseña **solo el nombre de la biblioteca** y cuándo caduca; nada de quién invita ni de quién hay dentro;
   - sin sesión: «Inicia sesión o crea una cuenta para entrar», lleva a ello, y al volver (`pendingInvitation`) retoma donde estaba;
   - con sesión: «Entrar en «Casa»» → `acceptInvitation`; mientras baja, la espera que ya existe (`download-wait`); al terminar, se cambia a esa biblioteca;
   - «No, gracias» → `discardPendingInvitation`;
   - cada error con su texto (`invitationReasonKey`): usada, caducada, anulada, para otra cuenta, de otro servidor, mal pegada.
4. **El aviso a quien administra:** el de 7.6.2 lleva a Personas al pulsarlo.
5. **Quita las marcas `@public`** de `api.ts` de lo que ya se use; lo que siga sin uso, dímelo en vez de dejar la marca.
6. **Mock de desarrollo** al día para poder ver cada estado sin servidor: los seis bloqueos, enlace creado, correo enviado, pendiente, aceptada y cada error al abrir.

## Un cambio pequeño en el servidor (aceptado por el maintainer el 11/10/2026)

`GET /v1/server` publica además la **base de los enlaces de invitación** (`invitation_base`, o como encaje con lo que hay; `null` si no hay invitaciones), y la app la usa en `token_of_link` en vez de la regla provisional «mismo host o el dominio de encima». **Lo que no cambia:** un enlace nunca decide a qué servidor se llama; la app sigue hablando solo con el servidor de la cuenta (o el de por defecto sin cuenta), y esa base solo dice qué enlaces son *de ese servidor*. Sin el dato (servidor antiguo), vale la regla de hoy. Test hostil antes, visto fallar: un servidor que publica una base ajena no consigue que la app llame a esa dirección; y un enlace con la base de otro servidor se sigue rechazando. Test en PHP de la ruta. Si ves que esto pide más que eso, **para y dímelo**.

## Fuera de este encargo

- **Que el enlace abra la app sola** (enlaces profundos): aplazado a antes de publicar, decidido el 11/10/2026. No añadas el plugin ni toques el manifiesto de Android. No rompas lo que lo hará fácil: todo enlace entra por `open_invitation(link)`.
- Elegir rol al invitar, invitaciones que no caducan, QR de la invitación.

## Reglas

- Ninguna cadena a pelo: claves i18n en `en` y `es`. Texto llano, sin palabras internas (token, hash, vincular).
- La interfaz no decide permisos: enseña u oculta según lo que dice Rust (`inviteStatus`, `invitable`), y el servidor tiene la última palabra. Ninguna lógica de dominio en React.
- Accesible con teclado, y bien en móvil estrecho y en escritorio.
- Solo `app/`, `docs/` y, en `server/`, lo del apartado anterior. Nada en `spec/` ni `crates/`. Ninguna regla nueva de permisos; si algo contradice una decisión aceptada o el encargo crece, para y explícamelo.
- Modo enseñanza de Rust: las explicaciones, en el informe, no en comentarios.
- No deshagas nada con git. Rust y npm en `distrobox enter library-dev -- bash -ic '…'`; servidor con `cd server && make …`.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; en `server`: `make check`. **Pásalos todos al final, después del último cambio**, y dime la salida real de cada uno. Actualiza `docs/architecture/cloud.md`, `docs/design/screens/README.md` (qué se construyó distinto del diseño), `docs/design/errors.md` si hay textos nuevos y la línea de 7.6 en `docs/plans/fase-7.md` (sin marcarla). **Manda el informe a la sesión `library` con SendMessage:** qué hiciste por punto, qué decidiste por tu cuenta, tests hostiles y cuáles viste fallar, comprobaciones, y qué no pudiste comprobar (nada se ha visto en pantalla salvo que lo digas). Añade una lista corta de **qué probar en pantalla**, paso a paso, para el maintainer.
