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
| `login` | 10 cada 15 min | IP (hito 7.1, parte 2) |
| `email_code` | 10 cada 15 min | IP |
| `email_per_address` | 3 cada 15 min | dirección de correo normalizada |

`email_per_address` protege a terceros: sin él, registrar repetidamente con el correo de otra persona podría inundarle la bandeja de entrada.

## Correo

`symfony/mailer`, texto plano (sin Twig), asunto y cuerpo desde `translations/emails.{en,es}.yaml` en el idioma de la cuenta. En desarrollo lo captura Mailpit; en producción, Brevo (ver `docs/development/server.md`).

## Pendiente

- Inicio de sesión y tokens de dispositivo (hito 7.1, parte 2).
- Recuperación de contraseña (`reset_password`, parte 3).
- Borrado de cuenta.
- Purga de cuentas sin confirmar a las 24 h.
- Comandos de consola para `ROLE_SUPER_ADMIN` (D51) y bloquear cuentas.
- Cuentas con Google.
- Segundo factor.
