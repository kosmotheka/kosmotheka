# Encargo · Arreglos de la auditoría del 11/10/2026 (7.6 y 7.7)

> Encargo cerrado para la sesión de hitos, 11/10/2026. Sin commit, sin push, sin cambiar de rama. Los hallazgos están en [docs/audits/2026-10-11.md](../audits/2026-10-11.md): lee cada uno antes de tocarlo. Solo los de esta lista; los que piden una decisión del maintainer (SEG-041, SEG-042, SEG-044, SEG-045, SEG-046) **no se tocan**.

Para cada uno: el test del caso primero, visto fallar, y después el arreglo. En la app no hay tests de componentes: lo que sea regla va en Rust con su test; lo que sea solo pantalla, dímelo como «sin test».

## Servidor

1. **SEG-051 · límite de inicios de sesión** (decidido por el maintainer el 11/10/2026, opción A): una familia en la misma wifi comparte IP.
   - el límite **por IP** de `login` sube a **30 cada 15 minutos** (lo gastan el reto y el inicio, como hoy);
   - límite nuevo **por cuenta, solo de intentos fallidos**: **5 cada 15 minutos**. Al pasarse, esa cuenta responde `too_many_requests` hasta que pase la ventana, también con la contraseña buena. La clave es la cuenta que el servidor encuentra por el correo, y **tiene que contar igual para un correo sin cuenta** (clave por la huella del correo normalizado, por ejemplo), para que la respuesta y el tiempo no digan qué direcciones tienen cuenta. Un inicio correcto no lo gasta;
   - el segundo factor ya tiene su límite por cuenta: no lo dupliques, pero un código equivocado cuenta como intento fallido;
   - tests hostiles: seis fallos seguidos contra una cuenta desde IPs distintas se cortan; un correo sin cuenta se comporta igual que uno con cuenta; cuatro dispositivos de la misma IP inician sesión seguidos sin corte; el corte de una cuenta no afecta a otra de la misma IP;
   - actualiza las cifras en `docs/architecture/accounts.md` y `security.md` (en llano: qué pasa si alguien falla tu contraseña cinco veces).
2. **SEG-043 · anular compite con aceptar:** `revoke` en una transacción con los mismos bloqueos y en el mismo orden que `InvitationAcceptor` (biblioteca, luego la fila), y la comprobación de «ya aceptada» después de bloquear. Test que lo demuestre (si una carrera real no se puede montar en PHPUnit, un test que acepte entre la lectura y el `flush` con dos EntityManager, o dime cómo lo has probado).
3. **SEG-047:** `kind: link` con `email` en el cuerpo se rechaza con `invalid_request`.

## App (Rust)

4. **COS-002:** antes de pedir una invitación para una persona que ya está, se sincroniza la biblioteca igual que para una nueva; si no se puede, el motivo es el de la sincronización («sin conexión»), no el rechazo del servidor.
5. **SEG-048:** las comparaciones de dominio de `token_of_link` se hacen sobre el host normalizado (sin punto final, sin distinguir mayúsculas), y un servidor propio nunca da por suyo `kosmotheka.app` ni un subdominio suyo. La regla de reserva para servidores antiguos (sin `invitation_base`) se queda en **el mismo host**: se quita «el dominio de encima». Tests de `kosmotheka.app.`, de un subdominio y de `github.io`.
6. **SEG-049:** `revoke_invitation` solo acepta un id con forma de UUID antes de formar la ruta.
7. **SEG-050:** tras aceptar, la app solo cambia de biblioteca si la que bajó es la de la invitación que tenía pendiente (compara lo que guardó al abrir el enlace con lo que el servidor dice al aceptar; si el servidor no da con qué comparar al abrir, dime qué dato falta en vez de inventarlo). Si no coincide: «Ya estás dentro; búscala en tu lista», sin cambiar.
8. **COS-003, lo que es de Rust:** `pending_invitation` distingue «no hay» de «ha caducado mientras esperabas», para que la pantalla lo diga.

## App (pantallas y textos)

9. **CON-011:** vuelve la clave `invitations.notice.accepted` en `en` y `es` (estaba en el commit `eb7a141`). Mira si al construir 7.6.3 se perdió alguna otra clave de `invitations.*` comparando con ese commit, y dímelo.
10. **COS-003:**
    - «Invitar» sin conexión: el motivo real (`invitationErrorKey`), con «Cerrar» y «Reintentar»;
    - invitación caducada al volver de iniciar sesión: se dice, y se ofrece pegar otra;
    - tras aceptar, un fallo al refrescar o al cambiar de biblioteca no se cuenta como fallo de la invitación: «Ya estás dentro; aparecerá en la próxima sincronización». `syncError` se enseña si lo hay.
11. **CAL-005:** el texto de ejemplo del campo de pegar, a las traducciones y sin nombrar el dominio oficial («Pega aquí el enlace que te mandaron»).
12. El selector «Por correo» / «Copiar enlace» se mueve con las flechas, como un grupo de opciones (apuntado en la revisión de 7.6.3).

## Límites

- Nada más que esta lista. Ninguna regla nueva de permisos; si un arreglo pide cambiar el contrato entre app y servidor más allá de lo dicho, o contradice una decisión aceptada, **para y dímelo**.
- Consultas con parámetros; sin dependencias nuevas; sin tocar `spec/` ni `crates/`.
- Modo enseñanza de Rust: las explicaciones, en el informe.
- No deshagas nada con git.

## Al terminar

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; en `app`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; en `server`: `make check`. **Todos después del último cambio**, con su salida real. Actualiza `docs/architecture/cloud.md`, `accounts.md`, `security.md` y `docs/design/errors.md` donde toque. **No toques `docs/audits/`** (lo cierra la auditoría). Informe a la sesión `library` con SendMessage: por hallazgo, qué hiciste, qué test lo cubre y si lo viste fallar; qué decidiste por tu cuenta; qué no pudiste comprobar.
