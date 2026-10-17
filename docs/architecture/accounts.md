# Cuentas

> La cuenta (`user`) en el servidor: alta, confirmación de correo, inicio de sesión con el dispositivo, entrar con Google, segundo factor, errores, límites de peticiones y borrado. Decisiones de fondo: D38–D40, D49, D50, D51, D77, D78 y D100. Puesto al día con el código el 17/10/2026.

## Qué es una cuenta

Una **cuenta** (`App\Entity\User`) es el acceso de una persona al servidor: correo, contraseña (hash `auto`, argon2id si está disponible), nombre, idioma (`en` o `es`) y roles. No es lo mismo que un **miembro** (`member`) de una biblioteca local: una cuenta puede más adelante enlazarse a un miembro de cada biblioteca que comparte (D46), pero esta parte del trabajo todavía no toca esa relación.

Una cuenta se crea **sin confirmar**: hasta que se confirma el correo con un código de 6 cifras (D49), no se puede usar para nada más (iniciar sesión se explica más abajo). Las cuentas sin confirmar se borran a las 24 h (ver «Purga de cuentas e invitaciones caducadas»).

## Códigos por correo (D49, D50)

Tres usos comparten el mismo mecanismo (`App\Entity\EmailCode`, `App\Security\EmailCodeIssuer`): confirmar el correo al registrarse (`confirm_email`), recuperar la contraseña olvidada (`reset_password`) y confirmar una acción delicada de una cuenta sin contraseña (`confirm_action`, D100). Un código de un propósito no sirve para otro: el código vivo se busca por cuenta y propósito.

- El código son **6 cifras** (`random_int`, con ceros a la izquierda).
- Caduca a los **15 minutos**.
- Permite **5 intentos** fallidos; al quinto, el código se borra y no se puede reintentar (hay que pedir uno nuevo).
- Es de **un solo uso**: acertarlo lo borra. Cuando lo que se confirma pide además otra cosa que puede salir mal (el código del segundo factor), el código del correo se **mira sin gastar** (`EmailCodeIssuer::check()`, que cuenta el intento si es erróneo) y se gasta al final (`spend()`): un descuido en lo otro no cuesta otro correo. Gastarlo es un `DELETE` que dice si encontró el código, y los intentos fallidos se suman en la base, no en memoria: de dos peticiones a la vez con el mismo código, solo una lo gasta, y ningún intento se pierde (SEG-072). Lo mismo para los códigos de respaldo y el último paso TOTP usado (un `UPDATE` condicionado), y el reto de inicio de sesión se consume bajo un cerrojo.
- Solo existe **un código vivo por cuenta y propósito**: pedir uno nuevo borra el anterior.
- Se guarda el **HMAC-SHA256** del código (con `kernel.secret`), nunca el código en claro. La comparación usa `hash_equals`.

## Formato de error

Los errores de negocio de la API de cuentas responden con un cuerpo JSON `{"error": "<código>"}` y el código HTTP correspondiente. Los códigos son casos de `App\Enum\AccountErrorCode`. `App\Security\AccountException` los lleva; un único listener del kernel (`App\EventListener\AccountExceptionListener`) los convierte en la respuesta JSON — así ningún controlador construye esa respuesta a mano.

Los errores de validación de los DTOs (`#[MapRequestPayload]`) no pasan por aquí: Symfony responde **422** con su formato habitual. Un límite de peticiones alcanzado responde **429** con `{"error":"too_many_requests"}` y una cabecera `Retry-After`.

## Alta y códigos del correo

Todos bajo `/v1/account`, JSON, `snake_case`.

### `POST /v1/account/register`

Cuerpo: `{email, password, name, locale}` (contraseña 8–4096 caracteres, nombre 1–100 tras recortar espacios, `locale` `en` o `es`). Limitado por `registration` (por IP).

Responde siempre **202** con `{}` (D100): **nunca revela** si el correo ya tiene cuenta, si está en la lista de un servidor cerrado, ni si ha gastado sus correos (evita que alguien compruebe qué direcciones están registradas). Lo único que se sale del 202 es el límite **por IP**, que no habla de la dirección (**429**).

- correo nuevo → crea la cuenta sin confirmar, con la contraseña del cuerpo, y envía un código `confirm_email`. Esa contraseña es provisional: solo hace que iniciar sesión diga «falta confirmar» (`account_not_confirmed`) en lugar de «credenciales incorrectas», y se sustituye por la de `confirm`;
- correo existente **sin confirmar** → solo envía un código nuevo (sirve también para reenviar un código perdido). **No cambia nada de la cuenta: ni la contraseña, ni el nombre, ni el idioma**: solo hay un código vivo por cuenta, así que quien se inscribe el último invalida el código del primero, y si la contraseña se guardara con el código, la que quedaría sería la de quien llegó después. La contraseña se elige **al confirmar**, con el código del correo, y por eso la pone siempre quien controla el correo;
- correo existente **confirmado** → no cambia nada; envía un correo corto, en su idioma, avisando de que alguien ha intentado crear una cuenta con su dirección y que puede iniciar sesión o recuperar la contraseña si fue quien lo intentó.

La contraseña del cuerpo se pasa por el hasher en todos los caminos (también el de una dirección fuera de la lista o con cuenta ya confirmada) para que ninguno responda antes que otro.

Si los correos de la dirección están gastados (`email_per_address`), no se manda nada ni se cambia ningún código (el límite se gasta antes de tocar el código vivo), y la respuesta es la misma 202.

**El correo sale después de responder** (`App\Security\MailOutbox`): durante la petición solo se guarda, y sale en `kernel.terminate`, cuando con PHP-FPM la respuesta ya ha salido; así lo que tarda la respuesta no dice si la dirección tiene cuenta. Sin transporte de Messenger. Si un envío falla ahí, se registra sin la dirección (solo la clase de la excepción) y no cambia nada de lo respondido. Fuera de una petición (consola) sale en el acto. Solo van diferidos los correos cuya respuesta no debe depender de la dirección: el código de confirmación, el de recuperación, el de acción y el aviso de «ya tienes cuenta»; los avisos de algo hecho (contraseña cambiada, segundo factor) y las invitaciones salen en el acto.

**Registro cerrado** (11/10/2026, para el despliegue de prueba): con `REGISTRATION_ALLOWED_EMAILS` (correos separados por comas) solo esas direcciones pueden crear una cuenta; para las demás, `register` responde el mismo **202** y **no crea nada ni manda ningún correo** (D100: ya no hay `403 registration_closed` aquí), y solo gasta el límite de registro por IP. Vacía (el valor por defecto), el registro está abierto. La comparación usa la misma normalización que el registro (sin espacios, en minúsculas) y es exacta: `ada+1@…` no es `ada@…`. Quien ya tiene cuenta entra igual; aceptar una invitación no crea cuentas, así que un invitado nuevo también tiene que estar en la lista. `GET /v1/server` dice `registration_restricted: true` y nunca la lista, y la app avisa antes de registrarse en un servidor así («si tu dirección no está, no te llegará ningún código»). La decide `App\Security\RegistrationGate`. **Excepción:** al entrar con Google (`GoogleAccounts`), una dirección que Google acaba de confirmar y que no está en la lista sí recibe `registration_closed`: quien lo recibe ha probado que la dirección es suya, así que no enseña la lista a nadie más, y no le deja a oscuras tras ir al navegador. Tests: `ClosedRegistrationTest`, `GoogleSignInTest`.

### `POST /v1/account/confirm`

Cuerpo: `{email, code, password}` (contraseña 8–4096 caracteres: **la que queda**, D100). Limitado por `email_code` (por IP). Responde **204** si el código es correcto, guarda la contraseña y marca `confirmedAt`. Si la cuenta no existe, ya está confirmada, o el código es incorrecto, caducado o ha agotado los intentos, responde **400** con `invalid_code`, `code_expired` o `too_many_attempts` — confirmar una cuenta ya confirmada da `invalid_code`, no un error distinto, por la misma razón de no revelar estado.

### `POST /v1/account/code`

Cuerpo: `{email, purpose}`. Solo acepta `purpose: "confirm_email"` (**422** para cualquier otro valor): la recuperación de la contraseña tiene su propio endpoint, `POST /v1/account/password/forgot`, y el código `confirm_action` su propia ruta, `POST /v1/account/code/action`. Limitado por `email_code` (por IP). Responde siempre **202** con `{}`, también cuando la dirección ha gastado sus correos (D100); solo reenvía el código si la cuenta existe, no está confirmada y le quedan correos.

### `POST /v1/account/code/action` (autenticada)

Sin cuerpo. Limitada por `email_code` (por IP) y por `email_per_address`. Manda a la cuenta **su propio** correo un código `confirm_action` y responde **202**. Es lo que da una cuenta sin contraseña (solo Google) en lugar de la contraseña para borrar la cuenta o activar el segundo factor (ver «Acciones delicadas»). Como es la dirección de la propia cuenta, un límite alcanzado responde **429** con normalidad.

## Límites de peticiones

`symfony/rate-limiter`, ventana deslizante. Los límites que gasta una petición por el hecho de llegar son atributos `#[RateLimit]` en la acción del controlador, con su clave en `App\Security\RateLimitKey` (la IP o la cuenta); se gastan **después** de leer el cuerpo, así que una petición con el cuerpo mal formado (`400`/`422`) no los gasta, y una bien formada con la contraseña o el código equivocados sí. Los que dependen del flujo (fallos de una cuenta, correos, segundo factor, llamadas reales a los proveedores) los consume a mano `App\Security\RequestLimiter`. Los dos acaban en el mismo `429` `{"error":"too_many_requests"}` con `Retry-After` (`AccountExceptionListener`). Los controladores nunca llaman a las factories directamente:

| Limitador | Límite | Clave |
|---|---|---|
| `registration` | 5 por hora | IP |
| `login` | 30 cada 15 min | IP: el reto y el inicio de sesión gastan uno cada uno, también los que salen bien. Es ancho a propósito: una familia en la misma wifi comparte IP (SEG-051) |
| `login_failed_account` | 5 cada 15 min | cuenta: **solo los intentos fallidos** (contraseña o código del segundo factor equivocados), también la contraseña actual equivocada al cambiarla o al borrar la cuenta: una sesión en un móvil desbloqueado no sirve para probar contraseñas. La clave es la huella del correo escrito, normalizado, así que un correo sin cuenta cuenta igual que uno con cuenta |
| `email_code` | 10 cada 15 min | IP |
| `email_per_address` | 3 cada 15 min | dirección: la huella del correo normalizado, nunca el correo |
| `invitation_create`, `invitation_email`, `invitation_recipient_library`, `invitation_recipient`, `invitation_lookup`, `invitation_accept` | ver [cloud.md](cloud.md#límites-de-peticiones) | las invitaciones (7.6.1): por cuenta, por huella del correo de destino o por IP, según el caso |
| `two_factor` | 5 cada 15 min | cuenta: cada intento de código del segundo factor, al entrar o al activarlo, desactivarlo o regenerar los de respaldo. El de `login` es por IP, y desde muchas IP se podrían probar los 6 dígitos |

`email_per_address` protege a terceros: sin él, registrar repetidamente con el correo de otra persona podría inundarle la bandeja de entrada. Para los códigos se gasta **antes** de cambiar el código vigente: cualquiera puede pedir un código para una dirección, y pedirlo cuando ya no quedan correos no le quita a su dueño el que ya tiene (auditoría del 15/10/2026). El aviso de contraseña cambiada no lo consume: el cambio ya se ha hecho (un 429 diría que falló), y solo puede provocarlo quien tiene la contraseña o un código.

## Correo

`symfony/mailer`, texto plano (sin Twig), asunto y cuerpo desde `translations/emails.{en,es}.yaml` en el idioma de la cuenta. En desarrollo lo captura Mailpit; en producción, Brevo (ver `docs/development/server.md`).

## Dispositivos y su token

Cada instalación de la app es un **dispositivo** (`App\Entity\Device`): su id (`device_id`) es el mismo UUID que ya usa en sus HLCs (la clave primaria de la fila la genera el servidor), y tiene una clave pública Ed25519 (32 bytes, base64url sin relleno — el mismo formato que decodifica `App\Sync\OperationSignature`). El servidor guarda solo el **hash SHA-256** del token de sesión (`tokenHash`); `tokenHash = null` significa que el dispositivo está desconectado. **Un dispositivo es una instalación de la app, no un móvil**: restaurar el móvil o reinstalar la app crea un dispositivo nuevo. **El servidor identifica un dispositivo por su id y su clave pública juntos**, no por el id solo: el id es público (viaja en cada operación), y quien lo supiera podría quedárselo iniciando sesión el primero con una clave propia (D84, costura 6; SEG-016). Una misma instalación (id y clave) no puede «mudarse» a otra cuenta, y una cuenta no puede cambiar la clave de un id que ya tiene en silencio. Para dar el móvil a otra persona, la app cambia de identidad (D81, abajo).

Un dispositivo tiene tres estados: **con sesión**, **sesión cerrada** (`tokenHash = null`; la misma cuenta puede volver a entrar con él) y **quitado** (`removedAt`; D81). La identidad de un dispositivo quitado no vuelve a iniciar sesión con ninguna cuenta; la app, al saberlo, crea una identidad nueva, y con esa puede entrar cualquier cuenta o usarse sin cuenta.

### El reto de inicio de sesión

`POST /v1/auth/challenge` (sin cuerpo) → **200** `{"challenge": "<32 bytes aleatorios, base64url sin relleno>", "expires_at": "<ISO 8601 UTC>"}`. Válido **5 minutos**, de **un solo uso** (se consume tanto si se usa bien como si no). Se guarda en su propio pool de caché, `cache.auth_challenge` (`config/packages/cache.yaml`), aparte de `cache.rate_limiter`. Limitado por `login` (por IP).

La app demuestra que tiene la clave privada firmando, con Ed25519, los bytes UTF-8 exactos de:

```
kosmotheka-login-v2\n<origen del servidor>\n<propósito>\n<challenge>\n<device_id>
```

(`App\Security\DeviceProof::MESSAGE_FORMAT`, D100). El **propósito** es `login` (contraseña) o `google`; el **origen** es `scheme://host[:port]` en minúsculas, sin barra final y sin el puerto si es el del esquema (`App\Security\ServerOrigin`, que lo saca de `DEFAULT_URI`, la dirección pública del servidor; en desarrollo `SERVER_EXTRA_ORIGINS`, vacía por defecto y siempre vacía en producción, añade otras direcciones por las que se llega al mismo servidor (emulador, red de casa) y la firma vale para cualquiera de ellas y ninguna más; la app escribe lo mismo con `Url::origin()` en `app/src-tauri/src/account/origin.rs`). Así una firma hecha para otro servidor, o para entrar con Google, no vale aquí para entrar con contraseña, ni al revés. **No se acepta el formato viejo** (`kosmotheka-login\n<challenge>\n<device_id>`): aceptarlo dejaría el hueco abierto. Las apps de la prueba interna tienen que actualizarse a la vez que el servidor. La firma viaja en base64url sin relleno (64 bytes). `App\Security\DeviceProof::verify()` decodifica reto, clave pública y firma igual que `OperationSignature`, y lanza `invalid_device_proof` si no cuadra.

### `POST /v1/auth/login`

Cuerpo: `{email, password, device_id, device_name, public_key, challenge, signature}`. Limitado por `login` (por IP, consumido el primero). Comprobaciones, en este orden:

1. Reto desconocido, caducado o ya usado → **400** `invalid_challenge` (se consume de todas formas).
2. La firma no verifica con `public_key` → **400** `invalid_device_proof`.
3. Límite de la cuenta: si ya ha fallado 5 veces en los últimos 15 minutos → **429** `too_many_requests`, **también con la contraseña buena**, hasta que pase la ventana. Se mira sin gastar nada, antes de tocar la contraseña, y para un correo sin cuenta responde igual (misma clave, misma respuesta y mismo tiempo), de modo que no dice qué direcciones tienen cuenta. Un inicio correcto no gasta este límite.
4. Credenciales (cada fallo, también un código del segundo factor equivocado, gasta un intento de la cuenta): correo desconocido, cuenta sin contraseña o contraseña incorrecta → **401** `invalid_credentials`. Para un correo desconocido se ejecuta igualmente el hasher contra una contraseña ficticia, para que el tiempo de respuesta no revele si la cuenta existe. Si el hash necesita renovarse (`needsRehash`), se renueva (`UserRepository` implementa `PasswordUpgraderInterface`).
5. Estado de la cuenta (`App\Security\UserChecker`, compartido con el firewall de abajo): sin confirmar → **403** `account_not_confirmed`; bloqueada → **403** `account_blocked`.
6. El dispositivo: su id está quitado → **403** `device_removed`, con cualquier cuenta; **el servidor identifica un dispositivo por su id y su clave pública juntos**, no solo por el id (D84, costura 6; SEG-016). El id viaja en cada operación y lo ven todos los que comparten una biblioteca, así que quien lo supiera podría iniciar sesión el primero con una clave propia y dejar al teléfono verdadero sin poder entrar. La clave, en cambio, la prueba la firma del reto. Con el par (`device_id`, `public_key`) que se acaba de probar: existe y está quitado → **403** `device_removed`; existe para **otra** cuenta (la misma instalación entrando en una segunda cuenta) → **409** `device_taken`; no existe el par, pero esta cuenta ya tiene ese id con **otra** clave → **409** `device_key_mismatch`; si no, se crea una fila nueva (o se actualiza el nombre de la existente) y se emite un token **nuevo**, que invalida el anterior. La fila de otra cuenta con el mismo id y otra clave no importa: es otra instalación, o la inútil de un intruso. Solo se guarda su hash; el token en claro se devuelve una vez, en la respuesta.

Responde **200** `{"token": "<32 bytes aleatorios, base64url sin relleno>", "user": {id, email, name, locale}}`. Los campos mal formados (correo, `device_id` como UUID, `device_name` de 1 a 100 caracteres tras recortar, `public_key`/`challenge`/`signature` como cadenas no vacías de longitud razonable) dan **422**.

`App\Security\UserChecker` lanza directamente `App\Security\AccountException` (no una excepción de autenticación de Symfony): tanto si lo llama el login a mano como si lo llama el firewall, el error de negocio sale siempre por el mismo camino, `App\EventListener\AccountExceptionListener`.

### Autenticar una llamada a `/v1`

Cada llamada autenticada lleva las cabeceras `X-Device-Id: <id>` y `Authorization: Bearer <token>`. El firewall `api` (`config/packages/security.yaml`, patrón `^/v1`, sin estado) usa `App\Security\DeviceTokenAuthenticator`: busca el dispositivo por el SHA-256 del token (único), comprueba con `hash_equals` que su id coincide con `X-Device-Id` y autentica a su cuenta. Varias filas pueden compartir id, por eso el token localiza la fila y no la cabecera. Si falla (dispositivo desconocido, desconectado, o token equivocado) responde **401** `{"error":"invalid_token"}`; si el dispositivo está quitado, **401** `{"error":"device_removed"}`, y la app empieza de cero (D81); si el servidor cerró su sesión porque la contraseña cambió en otro dispositivo, **401** `{"error":"signed_out_password_changed"}`, para que la app lo explique (D50). En ese caso el servidor guarda el motivo (`signOutReason`) y conserva el hash del token: así solo se lo dice a la instalación de verdad. Al volver a iniciar sesión, el motivo se borra. Una llamada sin ninguna credencial a una ruta protegida responde **401** `{"error":"authentication_required"}` (el mismo autenticador hace de punto de entrada). `App\Security\CurrentDevice` da a los controladores el dispositivo autenticado (se guarda como atributo de la petición, no en un servicio con estado, que no sería seguro de reutilizar entre peticiones de un mismo proceso PHP).

Rutas públicas bajo `/v1`: `/v1/account/(register|confirm|code)`, `/v1/account/password/(forgot|reset)`, `/v1/auth/(challenge|login)`, `/v1/auth/google/(start|connect|callback|finish)` y `/v1/server`. El resto exige `ROLE_USER`.

`lastSeenAt` del dispositivo se actualiza como mucho una vez por hora, para no escribir en cada llamada.

### Endpoints autenticados

Todos bajo `/v1`, requieren las cabeceras de arriba.

| Endpoint | Responde |
|---|---|
| `GET /v1/account` | `{id, email, name, locale, has_password, created_at}` |
| `PATCH /v1/account` `{name?, locale?}` | **200**, mismo cuerpo que el GET; los campos omitidos no cambian. El email no se cambia nunca (D77): para pasar a otro, se invita a ese email a las bibliotecas como administrador y la cuenta nueva quita a la vieja |
| `POST /v1/account/password` `{current_password, new_password}` | **204** (ver «Contraseña olvidada y cambio de contraseña» abajo) |
| `DELETE /v1/account` `{password}` | **204**; borra la cuenta (ver «Borrado de cuenta» abajo) |
| `POST /v1/auth/logout` | **204**; pone a `null` el token del dispositivo que llama |
| `GET /v1/account/devices` | `[{id, name, created_at, last_seen_at, signed_in, removed, can_restore, current}]`, por `created_at` |
| `POST /v1/account/devices/{id}/restore` | **204**; deshace la retirada mientras la instalación no se haya enterado: sigue con su sesión como si nada. Si ya recibió `device_removed` (y por tanto ya borró sus bibliotecas) → **409** `device_already_reset`. Uno no quitado → **204** sin cambios; de otra cuenta o desconocido → **404** `device_not_found` |
| `DELETE /v1/account/devices/{id}` | **204**; **quita** ese dispositivo (D81): marca `removedAt` y conserva la fila, para que esa instalación (id y clave) siga ligada a la cuenta, y el hash del token, para poder deshacerlo. Sirve también para el propio dispositivo: es lo que hace la app en «Dejar este dispositivo a otra persona», y entonces no se puede deshacer. Quitarlo otra vez no cambia nada. Un dispositivo de otra cuenta o desconocido → **404** `{"error":"device_not_found"}` |

## Contraseña olvidada y cambio de contraseña (D50)

### `POST /v1/account/password/forgot`

Cuerpo: `{email}`. Pública. Limitada por `email_code` (por IP). Responde siempre **202** con `{}`, igual que el registro no revela si la cuenta existe: solo envía un código `reset_password` (mismas reglas que el de confirmación: 6 cifras, 15 min, 5 intentos, un solo uso) si la cuenta existe, **está confirmada** y **tiene contraseña**.

### `POST /v1/account/password/reset`

Cuerpo: `{email, code, password, totp_code?}`. Pública. Limitada por `email_code` (por IP). Verifica el código como `confirm` (mismos códigos de error: `invalid_code`, `code_expired`, `too_many_attempts`), guarda la contraseña nueva y responde **204**.

**Con el segundo factor activado** (D100) el correo no basta: sin `totp_code` responde **401** `two_factor_required`, con uno malo **400** `invalid_two_factor_code`, y no cambia nada. El código del correo se mira **sin gastar** hasta que el del segundo factor también es bueno: la app lo pide y repite la misma petición. Solo se llega a ese punto con un código del correo válido, así que la ruta no sirve para saber qué cuentas tienen segundo factor.

Desconecta todos los dispositivos de la cuenta **excepto** el que hace la llamada, si lleva un token de dispositivo válido de esa misma cuenta (las cabeceras son opcionales en esta ruta). Esto es una excepción deliberada al comportamiento normal del firewall `api`: en el resto de rutas protegidas, un token inválido corta la petición con `invalid_token`; aquí no puede, porque la ruta es pública y quien ha olvidado la contraseña puede no llevar ningún dispositivo autenticado, o llevar uno con un token ya caducado. `App\Security\DeviceTokenAuthenticator::onAuthenticationFailure()` devuelve `null` (en vez de la respuesta 401 habitual) para esta ruta concreta, lo que dentro de Symfony significa «sigue sin autenticar», y `PUBLIC_ACCESS` deja pasar la petición. Si el token es válido, `App\Security\CurrentDevice::tryGet()` da ese dispositivo al controlador; si no hay ninguno, o pertenece a otra cuenta, no se excluye ningún dispositivo y se desconectan todos.

Envía un correo corto («se ha cambiado tu contraseña») en el idioma de la cuenta.

### `POST /v1/account/password` (autenticada)

Cuerpo: `{current_password, new_password, totp_code?}`. Limitada por `login` (por IP). Contraseña actual incorrecta → **401** `invalid_credentials`. Con el segundo factor activado, además `totp_code` (D100): sin él **401** `two_factor_required`, malo **400** `invalid_two_factor_code`. Cuenta sin contraseña (solo Google) → **409** `no_password`. Si acierta, guarda la contraseña nueva, desconecta el resto de dispositivos de la cuenta (nunca el que llama) y envía el mismo correo de aviso.

Ambos cambios de contraseña reutilizan `App\Repository\DeviceRepository::signOutAllExcept()`, una actualización masiva (DQL `UPDATE`) en vez de cargar cada `Device` en memoria.

## El id público de la cuenta (7.5.2b-2, D76)

`user.account_id` es un UUID v4 aleatorio, único, que se genera al crear la cuenta y no cambia nunca (la migración `Version20261010082211` da uno distinto a cada cuenta que ya existía). Es lo **único de la cuenta que viaja con una biblioteca**: `member.account_id` lo escribe la nube cuando enlaza a esa persona con la cuenta (`AccountLinker`, `docs/architecture/cloud.md`). No es la clave primaria (`user.id`) ni sale del correo: quien lee la biblioteca no aprende ni el correo ni el id interno de nadie.

## Borrado de cuenta

`DELETE /v1/account` (autenticada). Cuerpo: `{password?, email_code?, totp_code?}` (D100). Limitada por `login` (por IP). Pide **la contraseña** o, a una cuenta que no la tiene, **un código del correo** (`email_code`, de `POST /v1/account/code/action`): nunca ninguno de los dos, y un código del correo no vale en lugar de una contraseña que existe. Contraseña incorrecta o ausente → **401** `invalid_credentials`. Con el segundo factor activado, además `totp_code` (**401** `two_factor_required` / **400** `invalid_two_factor_code`). Si acierta, borra la cuenta (`EntityManager::remove`) y responde **204**. Los dispositivos y códigos de la cuenta se borran en cascada a nivel de base de datos (`ON DELETE CASCADE`).

**Sus bibliotecas de la nube (D99).** Una biblioteca no se pierde con la cuenta de quien la subió: antes de borrarla, y en la misma transacción, cada biblioteca de la que es dueña pasa a otra cuenta con un sitio activo en ella (administradora primero, y entre iguales la membresía más antigua), y solo se borra con la cuenta si no queda nadie. La base ya no lo hace por su cuenta (`owner_id` es `RESTRICT`). Detalle en [cloud.md](cloud.md#quién-es-la-dueña-de-una-biblioteca-de-la-nube-d99).

**Por la ruta y por la consola, el mismo código.** `App\Security\AccountDeletion::delete()` hace las tres cosas en una transacción (traspaso de bibliotecas, línea `account_deleted`, borrado). La usan `DELETE /v1/account` y `app:user:delete <email> [--force]`, que pide confirmación enseñando qué bibliotecas pasan a otra cuenta y cuáles se van (`CloudOwnership::previewFor`); desde la consola no hay dispositivo y el registro anota el canal `console`.

## Purga de cuentas e invitaciones caducadas (D49)

`App\Security\AccountPurger` borra, en una pasada: las cuentas nunca confirmadas con más de 24 h desde su creación, y los códigos de email caducados (de cualquier cuenta: un `reset_password` sin usar también caduca). Cada borrado es un `DELETE` DQL masivo; borrar un usuario arrastra sus dispositivos y códigos por la cascada de la base de datos. Cada paso va en su propia transacción: si uno falla, los demás se hacen igual, y al final la purga avisa con `PurgeFailedException` de cuáles fallaron (CAL-008).

- **A mano**: `app:accounts:purge` (ver `docs/development/server.md`).
- **Cada hora**: `App\Scheduler\PurgeSchedule` (`#[AsSchedule]`) programa un `App\Scheduler\PurgeAccountsMessage`, que `App\Scheduler\PurgeAccountsMessageHandler` (`#[AsMessageHandler]`) recoge y ejecuta. Solo se dispara mientras algo consume el transporte `scheduler_default` que crea automáticamente el atributo `#[AsSchedule]`; sin un worker corriendo, no pasa nada (`docs/development/server.md` explica cómo arrancarlo en desarrollo).

`App\Security\AccountPurger` recibe la hora por `Symfony\Component\Clock\ClockInterface` en vez de llamar a `new \DateTimeImmutable()` directamente, para poder fijarla en los tests con `ClockSensitiveTrait`.

## Segundo factor (D78)

Segundo factor opcional por cuenta: TOTP (RFC 6238, app autenticadora) más diez códigos de reserva, con `spomky-labs/otphp` y un flujo propio — **no** `scheb/2fa-bundle`, que depende de sesión y de un estado de firewall «a medias autenticado», algo que esta API, sin estado, no tiene: el inicio de sesión devuelve siempre un token de dispositivo de una sola vez, nunca una sesión intermedia. El host del servidor puede desactivarlo con una variable de entorno (por defecto, activado); el super-administrador puede retirar el segundo factor de una cuenta desde la consola (para quien pierde el teléfono y los códigos de reserva a la vez). Se pide al iniciar sesión con correo y contraseña **y también al entrar con Google** (7.1, [abajo](#continuar-con-google-71-d77)).

### Datos

`App\Entity\User` gana cuatro columnas: `totpSecret` (base32, solo cuando está activado), `totpPendingSecret` (base32, mientras se confirma el alta), `totpEnabledAt` y `totpLastTimestep` (anti-repetición: un código de un paso de tiempo igual o anterior al último aceptado se rechaza, aunque sea válido). `App\Entity\BackupCode` (tabla `backup_code`) guarda el HMAC-SHA256 (con `kernel.secret`, igual que los códigos de email) de cada uno de los **diez** códigos de reserva, y `usedAt` cuándo se gastó. Son diez caracteres de un alfabeto sin ambigüedad (sin `0`, `O`, `1`, `I`, `L`), que la app muestra como `xxxxx-xxxxx`; al comprobarlos da igual mayúsculas/minúsculas y el guion. Regenerarlos borra los diez anteriores.

**Los dos secretos se guardan cifrados** (SEG-063, 15/10/2026): `TwoFactorSecretBox` los sella con XChaCha20-Poly1305, con una clave sacada de `APP_SECRET` con su propio propósito (como el hash de los códigos de reserva) y el id de la cuenta como dato asociado. Una copia de la base de datos sola no sirve para generar los códigos, y un secreto copiado a otra cuenta no se abre allí. Se usa `APP_SECRET` y no el cofre de las claves de la nube porque ese cofre puede estar apagado en un servidor sin identidad de la nube, y el segundo factor no depende de ella. Cambiar `APP_SECRET` anula los segundos factores, igual que ya anulaba los códigos de reserva. La migración `Version20261015170000` sella los que hubiera en claro; necesita `APP_SECRET` solo si encuentra alguno.

### `App\Security\TwoFactor`

- `startSetup(user)`: genera un secreto nuevo en `totpPendingSecret` y devuelve `{secret, otpauth_uri}` (emisor `Kosmotheka`, etiqueta el correo de la cuenta; 6 cifras, 30 s, SHA1 — lo que todas las apps autenticadoras soportan). Ya activado → **409** `two_factor_already_enabled`; servidor con el segundo factor apagado → **409** `two_factor_unavailable`.
- `enable(user, code)`: comprueba `code` contra el secreto pendiente, lo traslada a `totpSecret`, fija `totpEnabledAt` y emite los diez códigos de reserva, devueltos **en claro una sola vez**. Sin `startSetup` previo → **409** `two_factor_not_started`; código incorrecto → **400** `invalid_two_factor_code`.
- `verify(user, code)`: acepta un código TOTP (ventana ±1 paso, con anti-repetición) o un código de reserva sin usar (lo marca usado). Si no cuadra ninguno de los dos → **400** `invalid_two_factor_code`.
- `disable(user)` y `regenerateBackupCodes(user)`.

La hora la da `Symfony\Component\Clock\ClockInterface`, no `new \DateTimeImmutable()` directamente, para poder fijarla en los tests (igual que `App\Security\AccountPurger`) y poder calcular con `otphp` el código válido en cada momento.

### Endpoints

Autenticados (`ROLE_USER`), bajo `/v1/account/two-factor`:

| Endpoint | Responde |
|---|---|
| `POST /setup` | **200** `{secret, otpauth_uri}` |
| `POST /enable` `{code, password?, email_code?}` | **200** `{backup_codes: [...]}`. Antes de nada pide quién lo pide (D100, ver «Acciones delicadas»): la contraseña o, a una cuenta sin ella, `email_code`; la sesión sola no basta (**401** `invalid_credentials`). Consume `login`. Envía el aviso «segundo factor activado» (sin límite de peticiones, como el de contraseña cambiada) |
| `DELETE` `{code, password?}` | **204**; `code` es un código TOTP o uno de reserva. Pide `password` solo si la cuenta tiene una; si no (Google), basta el código (D100). Contraseña incorrecta o ausente → **401** `invalid_credentials`; sin segundo factor activado → **409** `two_factor_not_enabled`; código incorrecto → **400** `invalid_two_factor_code`. Consume `login`. Envía el aviso «segundo factor retirado» |
| `POST /backup-codes` `{code, password?}` | **200** `{backup_codes: [...]}`; mismas comprobaciones y orden que `DELETE`. Consume `login` |

`GET /v1/account` gana `two_factor_enabled` y `backup_codes_left`.

### Acciones delicadas (D100)

Una sesión dura mucho, y un móvil desbloqueado en otras manos la tiene. Lo que cambia **cómo se entra en la cuenta** pide algo más que la sesión (`App\Security\SensitiveAction`). **No se pide desbloquear el móvil** (PIN o huella): el maintainer lo descartó.

| Acción | Quién lo pide | Segundo factor, si lo tiene activado |
|---|---|---|
| Cambiar la contraseña (`POST /v1/account/password`) | la contraseña actual | `totp_code` |
| Restablecerla con el código del correo (`POST /v1/account/password/reset`) | el código del correo | `totp_code` |
| Borrar la cuenta (`DELETE /v1/account`) | la contraseña, o `email_code` si no tiene | `totp_code` |
| Activar el segundo factor (`POST /two-factor/enable`) | la contraseña, o `email_code` si no tiene | su propio `code` |
| Quitarlo (`DELETE /two-factor`) y regenerar los códigos (`POST /two-factor/backup-codes`) | la contraseña si tiene; si no, basta `code` | `code` (siempre) |

- Sin `totp_code` la respuesta es **401** `two_factor_required`, el mismo código que el inicio de sesión; con uno malo, **400** `invalid_two_factor_code`. `TwoFactor::verify` gasta su límite por cuenta.
- **Una cuenta sin contraseña** (solo Google) pide un código del correo en lugar de la contraseña (`email_code`, propósito `confirm_action`, `POST /v1/account/code/action`). Nunca al revés: una cuenta que tiene contraseña no la sustituye por un código del correo, que alguien con el correo abierto también tendría. Falta de prueba → **401** `invalid_credentials`; código malo → **400** `invalid_code` / `code_expired` / `too_many_attempts`.
- El código del segundo factor puede ser el de 6 cifras de la app **o un código de respaldo**, con o sin guion y en mayúsculas o minúsculas (`TwoFactor` lo normaliza); los campos de la app aceptan las dos formas (`src/lib/second-factor-code.ts`, comprobado con `npm run check:codes`).
- Los fallos de contraseña cuentan contra el tope por cuenta de los inicios de sesión (SEG-051), lo mismo que cambiar la contraseña o borrar la cuenta.
- Orden: primero quién lo pide, luego el segundo factor, y solo al final se gasta el código del correo.
- Tests: `SensitiveAccountActionsTest` (los casos hostiles de D100).

### En el inicio de sesión

`POST /v1/auth/login` admite ahora `totp_code` (opcional). Tras las credenciales y el estado de la cuenta, y **antes** de tocar el dispositivo: si la cuenta tiene el segundo factor activado y falta `totp_code` → **401** `two_factor_required`; si no cuadra → **401** `invalid_two_factor_code`. El reto de inicio de sesión ya se ha consumido en ese punto (es de un solo uso), así que la app tiene que pedir uno **nuevo** antes de reintentar con el código. El segundo factor se sigue exigiendo al iniciar sesión aunque el host apague `TWO_FACTOR_ENABLED` más adelante: la variable solo oculta el alta para cuentas nuevas, nunca lo que ya tiene activado una cuenta existente.

### Variable de entorno y consola

`TWO_FACTOR_ENABLED` (por defecto `true`, en `.env`): si está a `false`, `POST /v1/account/two-factor/setup` responde `two_factor_unavailable` para cuentas nuevas, y `GET /v1/server` devuelve `two_factor: false`. `app:user:two-factor-disable <email>`: retira el secreto y los códigos de reserva de una cuenta (D78, para quien pierde el teléfono y los códigos a la vez); nunca expuesto por la API.

## Super-administrador y bloqueo (D51)

Dos comandos de consola, nunca expuestos por la API:

- `app:user:super-admin <email> [--revoke]`: concede o retira `ROLE_SUPER_ADMIN`.
- `app:user:block <email> [--undo]`: bloquea o desbloquea la cuenta (`blockedAt`). Bloquear también desconecta todos sus dispositivos (`DeviceRepository::signOutAllExcept()` sin excepción).

Correo desconocido en cualquiera de los dos → mensaje de error y salida distinta de cero.

## Qué ofrece el servidor (D77)

`GET /v1/server` (pública): `{"name": "Kosmotheka", "api_version": 1, "registration": true, "registration_restricted": false, "sign_in_with_google": false, "two_factor": true, "events": false, "invitations": false, "invitation_base": null, "metadata": []}`. `registration_restricted` dice que solo algunas direcciones pueden crear cuenta (nunca cuáles); `events`, `invitations` e `invitation_base` cuentan los avisos en directo y los enlaces de invitación ([cloud.md](cloud.md)); `metadata` lista los servicios de datos que el servidor busca para sus cuentas (`google_books`, `tmdb`, `rawg`, `bgg`, `comic_vine`). «El servidor dice lo que ofrece»: la app usa esta respuesta para decidir qué mostrar, en vez de dar por supuesto qué funciones existen. `sign_in_with_google` es `true` si el servidor tiene un cliente de Google configurado (`GoogleSignIn::isConfigured`); `two_factor` lee `TWO_FACTOR_ENABLED` (D78, más abajo).

## Continuar con Google (7.1, D77)

Código: `GoogleController`, `App\Security\Google\{GoogleAttempts, GoogleAccounts, GoogleSignIn, LeagueGoogleSignIn}`, la entidad `GoogleAttempt`. Dependencias: solo las dos de D77, `knpuniversity/oauth2-client-bundle` (la configuración del cliente) y `league/oauth2-google` (hablar con Google); traen `league/oauth2-client` y `guzzlehttp/*` como suyas. Google queda detrás de la interfaz `GoogleSignIn`: los tests usan un doble (`FakeGoogle`) y nunca tocan la red.

### El camino

1. **La app empieza.** Genera un secreto al azar (el «verificador») que **no sale de ella** y llama a `POST /v1/auth/google/start` `{verifier_hash, locale?}` con su SHA-256 (base64url, 43 caracteres). El servidor guarda un **intento** (id al azar, la huella, el idioma, caducidad de **10 minutos**) y responde `{attempt, url, expires_at}`. La app solo abre `url` si es `http(s)` sin credenciales (no un fichero, ni un esquema de otra app, ni el suyo).
2. **El navegador** del sistema va a `GET /v1/auth/google/connect?attempt=…`. El intento tiene que estar recién creado y sin usar (**sirve una vez**). El servidor genera su `state` y un valor para una cookie (`kosmotheka_google`, `HttpOnly`, `SameSite=Lax`, solo `/v1/auth/google`, 10 minutos), guarda las huellas de los dos y el secreto PKCE del trayecto servidor↔Google (no sale del servidor), y redirige a Google.
3. **Google devuelve** a `GET /v1/auth/google/callback?code=…&state=…`. Solo sigue si el `state` es uno que dio este servidor, de un intento que espera a Google, **y la cookie es la del mismo navegador**; un `state` ajeno, repetido o sin la cookie da una página «este enlace ya no vale» y no se pregunta nada a Google. Cada `state` vale **una vez**. Si Google trae `error` (p. ej. el usuario cancela), se vuelve a la app con el motivo. Si no, el servidor cambia el código por quien es la persona (PKCE; el token de Google se usa una vez para preguntar y se tira) y **solo sigue si Google dice `email_verified`**. Decide la cuenta (abajo), crea un **código de un solo uso** (32 bytes, solo se guarda su SHA-256, **2 minutos**) y deja al navegador en una página «Ya puedes volver a Kosmotheka» que lo manda a la app: `kosmotheka://auth/google?attempt=…&code=…` (o `&error=<código>` en vez de `code`). Sin referer, sin caché, `noindex`.
4. **La app termina.** Solo acepta la vuelta si su `attempt` es **el que ella empezó y tiene pendiente** (si no hay ninguno, o es otro, no llama a ningún servidor). Llama a `POST /v1/auth/google/finish` `{attempt, code, verifier, device_id, device_name, public_key, challenge, signature, totp_code?}`: **la misma prueba de dispositivo que `/login`** (reto de un solo uso firmado con la clave del dispositivo). El servidor comprueba, en este orden: reto y firma; intento existente, terminado, sin caducar, con código sin caducar; código **y** verificador (la huella del verificador tiene que ser la del intento; no dice cuál falló, y a los **5 fallos** el intento se borra); cuenta no bloqueada; el límite de fallos de la cuenta; el segundo factor; el dispositivo (`device_removed`, `device_taken`, `device_key_mismatch` como en `/login`). Responde lo mismo que `/login`: `{token, user}`. El código se gasta **al entregar el token**, no antes: pedir el segundo factor no lo quema.

Por qué así: lo que viaja por el navegador y por el enlace (el código) **no sirve sin el verificador**, que nunca salió de la app. Otra app que registre `kosmotheka://` y robe la vuelta no consigue nada. Por eso vale el esquema propio en todas las plataformas (también en servidores propios) sin depender de enlaces verificados.

### Qué cuenta resulta

Tras confirmar Google la dirección (en minúsculas y sin espacios, como siempre: dos que solo difieren en eso son la misma cuenta):

- **La cuenta ya enlazada a ese `sub`** (el identificador que Google da a la persona): se entra en ella, aunque su correo en Google haya cambiado. Se reconoce por el `sub`, no por el correo.
- **El correo ya tiene una cuenta confirmada sin `sub`:** se enlaza y se entra (misma persona, misma cuenta). Su contraseña sigue valiendo.
- **El correo tiene una cuenta ya enlazada a otro `sub`:** no entra nadie (`google_other_account`): otra cuenta de Google con una dirección que fue de la primera no hereda su cuenta.
- **El correo tiene una cuenta SIN confirmar** (alguien la registró con contraseña y no confirmó; puede haberla hecho otra persona para quedarse con una puerta): queda confirmada, **sin contraseña** (la que tenía ya no entra) y sin los códigos pendientes. Quien quiera contraseña la pone después con «olvidé mi contraseña».
- **No hay cuenta:** se crea, confirmada, sin contraseña, con el nombre de Google (o la parte local del correo) y el idioma que mandó la app. **El registro cerrado (`REGISTRATION_ALLOWED_EMAILS`) vale igual**: fuera de la lista no nace ninguna cuenta (`registration_closed`).
- **Cuenta bloqueada:** no entra (`account_blocked`), ni al volver de Google ni en `finish`.
- **Cuenta con segundo factor (D78):** entrar con Google **también lo pide** (`two_factor_required` en `finish`; la app pide el código y repite con un reto nuevo). Ver «La duda de este encargo».

Del Google solo se piden `openid`, `email` y `profile`, y solo se guardan el `sub` (en la cuenta, `user.google_sub`, único), y el correo y el nombre como cualquier cuenta. **Ningún token de Google se guarda.** `no_password` deja de ser solo teórico: una cuenta de Google no tiene contraseña que cambiar; si quiere una, «olvidé mi contraseña».

### Errores

Los de `finish`, con el cuerpo de siempre (`{"error": "<código>"}`): `google_unavailable` (**404**, sin cliente de Google en el servidor), `invalid_google_code` (**400**; desconocido, caducado, usado o equivocado: una sola respuesta), `invalid_challenge`, `invalid_device_proof`, `account_blocked`, `two_factor_required`, `invalid_two_factor_code`, los del dispositivo y `too_many_requests`. Los del camino de vuelta a la app, en `error=` del enlace: `google_cancelled`, `google_failed`, `google_email_not_verified`, `google_other_account`, `registration_closed`, `account_blocked`; la app no enseña ninguno que no conozca.

### Configuración, límites y registros

- `GOOGLE_CLIENT_ID` y `GOOGLE_CLIENT_SECRET` (`.env.local`). **Vacías = sin Google**: las cuatro rutas responden que no está disponible (404) y `GET /v1/server` dice `sign_in_with_google: false`. La dirección de vuelta registrada en Google es **`DEFAULT_URI` + `/v1/auth/google/callback`** (en producción `DEFAULT_URI=https://api.kosmotheka.app`); `DEFAULT_URI` tiene que ser, pues, la dirección pública del servidor.
- Límite `google`: **30 cada 15 minutos por IP** para `start`, `connect`, `callback` y `finish` juntos, más el de fallos por cuenta que ya existía (`finish` lo mira antes del segundo factor).
- **Registro de actividad** (D66): `login_succeeded_with_google`, `account_created_with_google` y `account_linked_with_google`, con su propio tipo. Sin el `sub`, el código, el verificador ni nada de Google en la fila.
- **Purga:** los intentos caducados se borran con la purga horaria (`AccountPurger`, `app:accounts:purge`).
- **Rutas con un secreto en la dirección** (no se pueden evitar del todo): `GET /v1/auth/google/connect?attempt=…` (el id del intento) y `GET /v1/auth/google/callback?code=…&state=…` (el código de Google y el `state`). Hay que **excluirlas del registro de accesos** de cualquier proxy (en desarrollo, el `map $loggable` de `server/docker/nginx/nginx.conf`; ver [development/server.md](../development/server.md)). El código de la app (`kosmotheka://auth/google?attempt=…&code=…`) va en la página que se enseña y en el enlace al sistema, nunca en una petición al servidor.

### El segundo factor al entrar con Google

Con Google **también se pide el segundo factor de la cuenta**, si lo tiene activado: `GoogleController::finish` comprueba el límite de fallos de la cuenta y llama a `requireTwoFactor` después de comprobar el estado de la cuenta y antes de tocar el dispositivo. La razón: quien activó el segundo factor lo hizo para que una sola llave (aquí, entrar en Google) no baste. D78 proponía lo contrario («al entrar con Google lo protege el segundo factor de Google», a revisar al construirlo); lo que hace el código es lo de arriba, y si D78 cambia, es quitar esa llamada.

### En la app

`app/src-tauri/src/account/google.rs`: `start` (verificador al azar, solo su huella sale), `complete` (la vuelta que llega por `links.rs`, `Destination::GoogleReturn`), `finish`, `cancel`. El intento pendiente vive en `local_state` (`google_pending`: id, verificador, servidor, caducidad y, tras la vuelta, el código), se borra al terminar, cancelar, cerrar sesión o fallar, y no se escribe en registros (`Secret`). Un enlace de vuelta solo cuenta si su `attempt` es el pendiente y no ha caducado; si no, se ignora sin preguntar al servidor. El motivo de un `error=` solo se enseña si es uno de los seis conocidos; otro se enseña como `google_failed`. En la pantalla de iniciar sesión (y en la de crear cuenta) el botón «Continuar con Google» solo aparece si `GET /v1/server` dice `sign_in_with_google`; abre el navegador, espera con «Cancelar» y vuelve por el punto de entrada de enlaces de 7.6.4. La vuelta se atiende al momento, aunque haya otro diálogo delante (termina una entrada que la persona empezó), pero un fallo o el segundo factor solo se enseñan en el diálogo de iniciar sesión.

## Dar el dispositivo a otra persona (D81)

La identidad de un dispositivo (su id y su clave) firma sus operaciones, y cada biblioteca sabe de quién es ese dispositivo. Por eso la misma instalación nunca pasa de una cuenta a otra: quien recibe el móvil tiene que empezar con una identidad nueva.

- **«Cerrar sesión»** (`POST /v1/auth/logout`): para la misma persona, que volverá a entrar. No borra nada en la app.
- **«Dejar este dispositivo a otra persona»**: la app llama a `DELETE /v1/account/devices/{su propio id}`, borra todas sus bibliotecas y genera un id y una clave nuevos.
- **Quitado desde otro dispositivo:** la próxima llamada de esa instalación recibe `device_removed`, y la app hace lo mismo que en el punto anterior. El servidor apunta cuándo se lo dijo (`removalNoticedAt`); hasta entonces, la retirada se puede deshacer con `restore`, por si fue un error. Solo cuenta si se lo dice a la instalación de verdad: con su token, o al iniciar sesión con su clave. El id de un dispositivo lo ven todos los que comparten una biblioteca, y alguien que solo conoce el id no puede acabar con la posibilidad de deshacer.
- La app, al quitar un dispositivo, avisa de lo que pasará, y de que se perderán las bibliotecas que ese dispositivo no tenga en la nube.
- Quitar un dispositivo de la cuenta **no le retira el acceso a las bibliotecas compartidas**: eso lo hace quien administra cada una («Retirar el acceso de este dispositivo», intención `device.remove`, D71), y la nube corta el acceso al momento (ver [cloud.md](cloud.md#la-nube-corta-el-acceso-754b-d98-d71)).

## En la app

El núcleo en Rust habla con el servidor (`app/src-tauri/src/account/`); la interfaz solo llama a comandos de Tauri.

- **Qué guarda el dispositivo**, en `local_state` (nunca se sincroniza ni va en las copias): el servidor elegido (si no hay ninguno, el oficial, cuya dirección está fija en el código de la app), el token, una copia del usuario, el último email usado (para rellenar el formulario) y un aviso pendiente (`device_removed` o `signed_out_password_changed`) que la interfaz muestra una vez.
- **Otro servidor:** HTTPS siempre; HTTP solo en `localhost`, una IP privada, `10.0.2.2` (el emulador de Android) o en compilaciones de depuración. Para cambiar de servidor hay que cerrar sesión antes.
- **Iniciar sesión** pide un reto nuevo en cada intento (también al reintentar con el código del segundo factor) y lo firma con la clave del dispositivo, junto con el origen del servidor y el propósito (`login` o `google`; `account/origin.rs`, D100). El origen sale de la dirección del servidor que la app tiene guardada (`Url::origin()`: minúsculas, sin barra, sin el puerto del esquema).
- **Acciones delicadas** (D100): cambiar la contraseña, restablecerla, borrar la cuenta y activar o quitar el segundo factor piden lo que dice «Acciones delicadas» arriba. La app lo sabe por `has_password` y `two_factor_enabled` de `GET /v1/account`, y al restablecer la contraseña (sin sesión) por la respuesta `two_factor_required`, que la lleva a una pantalla para el código de la app y repite la petición con el mismo código del correo. A una cuenta sin contraseña se le ofrece «Enviarme un código» (`request_action_code`) en lugar del campo de contraseña (`proof-fields.tsx`).
- **Crear cuenta:** la contraseña se escribe al registrarse, se queda en memoria y viaja otra vez con el código en `confirm`. Si la persona entra más tarde en una cuenta sin confirmar, la pantalla del código la pide de nuevo. Un servidor con `registration_restricted` avisa antes de registrarse, sin decir quién está en la lista.
- **Errores:** llegan a la interfaz como `{kind: "account", reason, retryAfter}`, con el código del servidor tal cual; la interfaz los traduce con [design/errors.md](../design/errors.md). Sin respuesta, o una respuesta que no se entiende, es `server_unavailable`.
- **Reacción central:** tras cualquier llamada, `device_removed` reinicia el dispositivo; `signed_out_password_changed` e `invalid_token` olvidan la sesión, y la primera deja el aviso.
- **Al abrir la app**, si hay sesión, se pide `GET /v1/account` en segundo plano: así se entera un dispositivo que se quitó mientras estaba cerrado.
- **Reiniciar el dispositivo** (`device_reset.rs`, D81): borra el fichero de la base de datos con sus `-wal`, `-shm` y las copias `.bak` de antes de migrar, y abre uno nuevo (id y clave nuevos, sin bibliotecas); vacía las portadas; para el servidor de la red local y crea un certificado nuevo. Después la app emite el evento `device-reset` y la interfaz se recarga. Se hace sin reiniciar la aplicación, también en Android.
- **Desbloqueo con huella o cara** (D78, `tauri-plugin-biometric`): solo en el móvil, apagado por defecto, también sin cuenta. Se guarda en `local_state` (`app_lock`). Activarlo pide la huella antes. La app se bloquea al abrirse y al volver tras estar fuera el tiempo elegido en Ajustes (`app_lock_after` en `local_state`, en minutos): enseguida, 5 minutos, **1 hora** (por defecto), 12 horas o 1 día. Era un minuto fijo; el maintainer pidió elegirlo el 03/10/2026, porque al catalogar se sale y se vuelve a menudo y no es una app de banco; si la huella falla, el PIN del móvil. Si el móvil ya no tiene huella ni cara configuradas, abre sin bloquear, para no dejar a nadie fuera de sus datos. Es un bloqueo de pantalla, no cifra los datos.
- **Pantallas:** Ajustes (cuenta y «Este dispositivo»), iniciar sesión, crear cuenta, recuperar la contraseña, la cuenta, el segundo factor, mis dispositivos y los avisos, según [design/screens/cloud-account/](../design/screens/cloud-account/). El email se muestra pero no se edita (D77). Los códigos de respaldo solo se pueden copiar; guardarlos en un archivo, más adelante.

## Pendiente

- Guardar los códigos de respaldo en un archivo: hoy la app solo permite copiarlos.
- El correo de una cuenta no se cambia nunca (D77): para pasar a otro hay que invitar a ese correo a las bibliotecas como administrador y que la cuenta nueva quite a la vieja.
