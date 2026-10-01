# Cuentas

> La cuenta (`user`) en el servidor: alta, confirmación de correo, errores y límites de peticiones. Decisiones de fondo: D38–D40, D49, D50, D51. Documento vivo: se completa hito a hito (hito 7.1).

## Qué es una cuenta

Una **cuenta** (`App\Entity\User`) es el acceso de una persona al servidor: correo, contraseña (hash `auto`, argon2id si está disponible), nombre, idioma (`en` o `es`) y roles. No es lo mismo que un **miembro** (`member`) de una biblioteca local: una cuenta puede más adelante enlazarse a un miembro de cada biblioteca que comparte (D46), pero esta parte del trabajo todavía no toca esa relación.

Una cuenta se crea **sin confirmar**: hasta que se confirma el correo con un código de 6 cifras (D49), no se puede usar para nada más (iniciar sesión es el hito 7.1, parte 2). Las cuentas sin confirmar se borran a las 24 h (pendiente, ver abajo).

## Códigos por correo (D49, D50)

Dos usos comparten el mismo mecanismo (`App\Entity\EmailCode`, `App\Security\EmailCodeIssuer`): confirmar el correo al registrarse y recuperar la contraseña olvidada (esta segunda, en la parte 3).

- El código son **6 cifras** (`random_int`, con ceros a la izquierda).
- Caduca a los **15 minutos**.
- Permite **5 intentos** fallidos; al quinto, el código se borra y no se puede reintentar (hay que pedir uno nuevo).
- Es de **un solo uso**: acertarlo lo borra.
- Solo existe **un código vivo por cuenta y propósito**: pedir uno nuevo borra el anterior.
- Se guarda el **HMAC-SHA256** del código (con `kernel.secret`), nunca el código en claro. La comparación usa `hash_equals`.

## Formato de error

Los errores de negocio de la API de cuentas responden con un cuerpo JSON `{"error": "<código>"}` y el código HTTP correspondiente. Los códigos son casos de `App\Enum\AccountErrorCode`. `App\Security\AccountException` los lleva; un único listener del kernel (`App\EventListener\AccountExceptionListener`) los convierte en la respuesta JSON — así ningún controlador construye esa respuesta a mano.

Los errores de validación de los DTOs (`#[MapRequestPayload]`) no pasan por aquí: Symfony responde **422** con su formato habitual. Un límite de peticiones alcanzado responde **429** con `{"error":"too_many_requests"}` y una cabecera `Retry-After`.

## Endpoints (parte 1)

Todos bajo `/v1/account`, JSON, `snake_case`.

### `POST /v1/account/register`

Cuerpo: `{email, password, name, locale}` (contraseña 8–4096 caracteres, nombre 1–100 tras recortar espacios, `locale` `en` o `es`). Limitado por `registration` (por IP).

Responde siempre **202** con `{}`, **nunca revela si el correo ya existe** (evita que alguien compruebe qué direcciones están registradas):

- correo nuevo → crea la cuenta sin confirmar y envía un código `confirm_email`;
- correo existente **sin confirmar** → actualiza contraseña, nombre e idioma, y envía un código nuevo (sirve también para reenviar un código perdido, cambiando los datos a la vez);
- correo existente **confirmado** → no cambia nada; envía un correo corto, en su idioma, avisando de que alguien ha intentado crear una cuenta con su dirección y que puede iniciar sesión o recuperar la contraseña si fue quien lo intentó.

### `POST /v1/account/confirm`

Cuerpo: `{email, code}`. Limitado por `email_code` (por IP). Responde **204** si el código es correcto y marca `confirmedAt`. Si la cuenta no existe, ya está confirmada, o el código es incorrecto, caducado o ha agotado los intentos, responde **400** con `invalid_code`, `code_expired` o `too_many_attempts` — confirmar una cuenta ya confirmada da `invalid_code`, no un error distinto, por la misma razón de no revelar estado.

### `POST /v1/account/code`

Cuerpo: `{email, purpose}`. Por ahora solo acepta `purpose: "confirm_email"` (**422** para cualquier otro valor; `reset_password` llega en la parte 3 con su propio endpoint). Limitado por `email_code` (por IP). Responde siempre **202** con `{}`; solo reenvía el código si la cuenta existe y no está confirmada.

## Límites de peticiones

`symfony/rate-limiter`, ventana deslizante, consumidos siempre a través de `App\Security\RequestLimiter` (los controladores nunca llaman a las factories directamente):

| Limitador | Límite | Clave |
|---|---|---|
| `registration` | 5 por hora | IP |
| `login` | 10 cada 15 min | IP |
| `email_code` | 10 cada 15 min | IP |
| `email_per_address` | 3 cada 15 min | dirección de correo normalizada |

`email_per_address` protege a terceros: sin él, registrar repetidamente con el correo de otra persona podría inundarle la bandeja de entrada.

## Correo

`symfony/mailer`, texto plano (sin Twig), asunto y cuerpo desde `translations/emails.{en,es}.yaml` en el idioma de la cuenta. En desarrollo lo captura Mailpit; en producción, Brevo (ver `docs/development/server.md`).

## Dispositivos y su token (parte 2)

Cada instalación de la app es un **dispositivo** (`App\Entity\Device`): su id es el mismo UUID que ya usa en sus HLCs (no lo genera el servidor), y tiene una clave pública Ed25519 (32 bytes, base64url sin relleno — el mismo formato que decodifica `App\Sync\OperationSignature`). El servidor guarda solo el **hash SHA-256** del token de sesión (`tokenHash`); `tokenHash = null` significa que el dispositivo está desconectado. **Un id de dispositivo queda ligado para siempre a la cuenta que lo registró primero**, y solo puede volver a iniciar sesión con esa cuenta y con la misma clave pública: así un id robado o reutilizado no puede «mudarse» de cuenta ni cambiar de clave en silencio.

### El reto de inicio de sesión

`POST /v1/auth/challenge` (sin cuerpo) → **200** `{"challenge": "<32 bytes aleatorios, base64url sin relleno>", "expires_at": "<ISO 8601 UTC>"}`. Válido **5 minutos**, de **un solo uso** (se consume tanto si se usa bien como si no). Se guarda en su propio pool de caché, `cache.auth_challenge` (`config/packages/cache.yaml`), aparte de `cache.rate_limiter`. Limitado por `login` (por IP).

La app demuestra que tiene la clave privada firmando, con Ed25519, los bytes UTF-8 exactos de:

```
kosmotheka-login\n<challenge>\n<device_id>
```

(`App\Security\DeviceProof::MESSAGE_FORMAT`). La firma viaja en base64url sin relleno (64 bytes). `App\Security\DeviceProof::verify()` decodifica reto, clave pública y firma igual que `OperationSignature`, y lanza `invalid_device_proof` si no cuadra.

### `POST /v1/auth/login`

Cuerpo: `{email, password, device_id, device_name, public_key, challenge, signature}`. Limitado por `login` (por IP, consumido el primero). Comprobaciones, en este orden:

1. Reto desconocido, caducado o ya usado → **400** `invalid_challenge` (se consume de todas formas).
2. La firma no verifica con `public_key` → **400** `invalid_device_proof`.
3. Credenciales: correo desconocido, cuenta sin contraseña o contraseña incorrecta → **401** `invalid_credentials`. Para un correo desconocido se ejecuta igualmente el hasher contra una contraseña ficticia, para que el tiempo de respuesta no revele si la cuenta existe. Si el hash necesita renovarse (`needsRehash`), se renueva (`UserRepository` implementa `PasswordUpgraderInterface`).
4. Estado de la cuenta (`App\Security\UserChecker`, compartido con el firewall de abajo): sin confirmar → **403** `account_not_confirmed`; bloqueada → **403** `account_blocked`.
5. El dispositivo: su id ya existe para **otra** cuenta → **409** `device_taken`; existe para esta cuenta con **otra** clave pública → **409** `device_key_mismatch`; si no, se crea (o se actualiza su nombre) y se emite un token **nuevo**, que invalida el anterior. Solo se guarda su hash; el token en claro se devuelve una vez, en la respuesta.

Responde **200** `{"token": "<32 bytes aleatorios, base64url sin relleno>", "user": {id, email, name, locale}}`. Los campos mal formados (correo, `device_id` como UUID, `device_name` de 1 a 100 caracteres tras recortar, `public_key`/`challenge`/`signature` como cadenas no vacías de longitud razonable) dan **422**.

`App\Security\UserChecker` lanza directamente `App\Security\AccountException` (no una excepción de autenticación de Symfony): tanto si lo llama el login a mano como si lo llama el firewall, el error de negocio sale siempre por el mismo camino, `App\EventListener\AccountExceptionListener`.

### Autenticar una llamada a `/v1`

Cada llamada autenticada lleva las cabeceras `X-Device-Id: <id>` y `Authorization: Bearer <token>`. El firewall `api` (`config/packages/security.yaml`, patrón `^/v1`, sin estado) usa `App\Security\DeviceTokenAuthenticator`: busca el dispositivo por `X-Device-Id`, compara el SHA-256 del token con `hash_equals` y autentica a su cuenta. Si falla (dispositivo desconocido, desconectado, o token equivocado) responde **401** `{"error":"invalid_token"}`. Una llamada sin ninguna credencial a una ruta protegida responde **401** `{"error":"authentication_required"}` (el mismo autenticador hace de punto de entrada). `App\Security\CurrentDevice` da a los controladores el dispositivo autenticado (se guarda como atributo de la petición, no en un servicio con estado, que no sería seguro de reutilizar entre peticiones de un mismo proceso PHP).

Rutas públicas bajo `/v1` (lista fácil de ampliar en la parte 3): `/v1/account/(register|confirm|code)` y `/v1/auth/(challenge|login)`. El resto exige `ROLE_USER`.

`lastSeenAt` del dispositivo se actualiza como mucho una vez por hora, para no escribir en cada llamada.

### Endpoints autenticados

Todos bajo `/v1`, requieren las cabeceras de arriba.

| Endpoint | Responde |
|---|---|
| `GET /v1/account` | `{id, email, name, locale, has_password, created_at}` |
| `PATCH /v1/account` `{name?, locale?}` | **200**, mismo cuerpo que el GET; los campos omitidos no cambian |
| `POST /v1/auth/logout` | **204**; pone a `null` el token del dispositivo que llama |
| `GET /v1/account/devices` | `[{id, name, created_at, last_seen_at, signed_in, current}]`, por `created_at` |
| `DELETE /v1/account/devices/{id}` | **204**; desconecta ese dispositivo (token a `null`, la fila se conserva para que el id siga ligado a la cuenta). Un dispositivo de otra cuenta o desconocido → **404** `{"error":"device_not_found"}` |

## Pendiente

- Recuperación de contraseña (`reset_password`, parte 3).
- Borrado de cuenta.
- Purga de cuentas sin confirmar a las 24 h.
- Comandos de consola para `ROLE_SUPER_ADMIN` (D51) y bloquear cuentas.
- Cuentas con Google.
- Segundo factor.
