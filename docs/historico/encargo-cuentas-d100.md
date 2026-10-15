# Encargo · Cuentas tras la auditoría del 15/10/2026 (D100)

> Encargo cerrado para la sesión de hitos, escrito el 15/10/2026 y **sin enviar** hasta que el maintainer confirme el punto 4. Sin commit, sin push, sin cambiar de rama. Toca `server/`, `app/` y `docs/`, y una prueba de `crates/domain` si hace falta. Nada en `spec/`.

## Para qué

La auditoría de cuentas del 15/10/2026 ([informe](../audits/2026-10-15-2.md)) dejó seis cosas que el maintainer decidió en [D100](../DECISIONS.md). Una sesión abierta en un móvil desbloqueado no debe bastar para cambiar cómo se entra en la cuenta. Una cuenta creada con Google debe poder hacer lo mismo que una con contraseña. Y las respuestas no deben dejar adivinar quién tiene cuenta. El secreto del segundo factor ya va cifrado (`a163742`): eso no se toca.

Lee antes D49, D50, D77, D78 y D100, y `docs/architecture/accounts.md`.

## 1 · El segundo factor en las acciones delicadas (SEG-064, SEG-073)

Si la cuenta tiene el segundo factor activado, piden además un código del segundo factor (TOTP o de respaldo, por `TwoFactor::verify`, que ya gasta su límite por cuenta):

- cambiar la contraseña (`POST /v1/account/password`);
- restablecerla con el código del correo (`POST /v1/account/password/reset`);
- borrar la cuenta (`DELETE /v1/account`).

Quitar el segundo factor y regenerar los códigos de respaldo ya lo piden. El campo es `totp_code`, el mismo nombre que en el inicio de sesión. Sin código, la respuesta es **401 `two_factor_required`**, el mismo código que ya da el inicio de sesión. Con un código malo, **400 `invalid_two_factor_code`**. **No** se pide desbloquear el móvil (PIN o biometría): el maintainer lo descartó.

App: los diálogos de cambiar la contraseña, restablecerla y borrar la cuenta muestran el campo del código cuando hace falta. Para saberlo, usan `two_factor_enabled` de `GET /v1/account` o, al restablecer sin sesión, la respuesta `two_factor_required`.

## 2 · Activar el segundo factor pide la contraseña (SEG-064)

`POST /v1/account/two-factor/enable` pide además `password`. Si la cuenta no tiene contraseña, pide en su lugar un código por correo (punto 3). Con la contraseña mala, 401 `invalid_credentials`, y el fallo cuenta contra el tope por cuenta de los inicios de sesión, como ya hacen cambiar la contraseña y borrar la cuenta (`AccountController::checkCurrentPassword`, `b4cbc26`). `setup` puede seguir sin nada: solo prepara un secreto pendiente.

## 3 · Una cuenta sin contraseña (solo Google) (SEG-062)

- **Código por correo para una acción:** nuevo propósito de `EmailCodePurpose` (`confirm_action`, o el nombre que encaje mejor con el glosario). Se pide con sesión, en una ruta propia (`POST /v1/account/code/action`). Gasta el límite por dirección como los demás. Sirve para **borrar la cuenta** y **activar el segundo factor** cuando no hay contraseña. Lleva texto de correo en `en` y `es`.
- **Quitar el segundo factor y regenerar los códigos de respaldo** sin contraseña: basta su código del segundo factor (hoy responden `no_password`).
- `DELETE /v1/account` acepta `password` o `email_code`, según tenga o no contraseña; nunca ninguno de los dos.

App: en esas pantallas, a una cuenta sin contraseña (`has_password: false`) se le ofrece «Enviarme un código» en lugar del campo de contraseña.

## 4 · Registrar otra vez una dirección sin confirmar (SEG-066) — **pendiente del sí del maintainer**

D100 dice: «la contraseña se guarda con el código y queda al confirmar con ese código». Al prepararlo, la sesión coordinadora vio que así no basta. Solo hay un código vivo por cuenta y propósito, así que si la atacante registra otra vez después de Ana, el código de Ana muere, a Ana le llega el de la atacante y, al teclearlo, quedaría la contraseña de la atacante.

**Propuesta, a confirmar:** la contraseña se elige **al confirmar**. `POST /v1/account/confirm` pide `{email, code, password}`, y esa contraseña es la que queda. El registro deja de cambiar la contraseña de una cuenta sin confirmar (solo manda un código nuevo). Así, la contraseña la pone siempre quien controla el correo. La app ya la tiene en memoria entre registrarse y confirmar; si se cerró entre medias, la pide otra vez en la pantalla del código.

**No construyas este punto hasta que la sesión coordinadora te diga que está confirmado.**

## 5 · La firma del dispositivo nombra el servidor y el propósito (SEG-065)

Hoy se firma `kosmotheka-login\n{challenge}\n{device_id}` (`DeviceProof::MESSAGE_FORMAT`; `app/src-tauri/src/account/mod.rs:442` y `account/google.rs:170`). Pasa a ser `kosmotheka-login-v2\n{origen del servidor}\n{propósito}\n{challenge}\n{device_id}`:

- **Propósito:** `login` o `google`.
- **Origen del servidor:** esquema, host y puerto, en minúsculas, sin barra final. La app usa el del servidor que tiene configurado y el servidor el suyo propio, sacado de `DEFAULT_URI`. Comprueba en `docs/development/server.md` y en `docs/publicacion/servidor-produccion.md` que `DEFAULT_URI` es la dirección pública en producción. Si no lo es, **para y dímelo**.
- **Sin compatibilidad con el formato viejo:** aceptarlo dejaría el hueco abierto. Las apps de la prueba interna tendrán que actualizarse; escríbelo en las notas de la versión.

Caso hostil: una firma hecha para otro origen, o para el otro propósito, se rechaza.

## 6 · Las respuestas no delatan nada (SEG-067, SEG-070)

- **`register`, `code` y `password/forgot` responden siempre 202**, también cuando salta el límite de correos de la dirección: en ese caso no se manda nada y no se cambia ningún código (eso ya lo garantiza `0ccdbce`). El límite por IP sigue dando 429, porque ese no habla de la dirección.
- **El correo sale después de responder:** en `kernel.terminate` (con PHP-FPM, la respuesta ya ha salido), no en la petición. Sin dependencias nuevas ni transporte de Messenger. Si un envío falla ahí, se registra en el log sin la dirección.
- **El registro de una cuenta ya confirmada** hashea la contraseña igualmente, para tardar lo mismo.
- **Una dirección fuera de la lista** (`REGISTRATION_ALLOWED_EMAILS`) recibe 202 y no se crea ni se manda nada. Desaparece `registration_closed` del servidor.
- **App:** `/v1/server` ya dice `registration_restricted`. Antes de registrarse en un servidor así, la app avisa: «Este servidor solo admite algunas direcciones. Si la tuya no está, no te llegará ningún código». Quita el caso `registration_closed` de `account-errors.ts` y sus textos, si ya nadie lo usa.

## Casos hostiles (prueba antes de la defensa, vista fallar; enséñame el rojo)

1. Con sesión y segundo factor activado, cambiar la contraseña, restablecerla o borrar la cuenta sin código → `two_factor_required`, y nada cambia.
2. Activar el segundo factor con la sesión sola (sin contraseña) → rechazado, y el secreto pendiente sigue sin activarse.
3. Una cuenta solo de Google borra la cuenta con un código por correo, y quita el segundo factor con su código.
4. Un código por correo para una acción no sirve para restablecer la contraseña ni para confirmar la cuenta, y viceversa.
5. Una firma de inicio de sesión hecha con otro origen, o para `google`, no sirve para `login` en este servidor.
6. Cuarta petición de código a una dirección con cuenta → 202 igual que a una sin cuenta.
7. Registro de una dirección fuera de la lista → 202, sin fila ni correo.
8. (Si se confirma el punto 4.) Registrar otra vez una dirección sin confirmar no cambia su contraseña. La que queda es la que se da al confirmar.

## Reglas

- Doctrine primero; consultas con parámetros. Sin dependencias nuevas.
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.
- Los textos de error nuevos van en `docs/design/errors.md`.

## Al terminar

Servidor: `make check` **después del último cambio**, y `make migration` si hace falta. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:spec`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. La salida real de cada uno.

Documentación: `docs/architecture/accounts.md`, `docs/design/errors.md`, `docs/plans/fase-7.md` y, para las notas de la versión, lo que diga `docs/publicacion/`.

Informe: qué has hecho, qué decidiste tú y por qué, qué pruebas viste fallar antes, qué no has podido comprobar y cualquier cosa rara.
