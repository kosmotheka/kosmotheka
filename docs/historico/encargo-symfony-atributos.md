# Encargo · El servidor con los mecanismos de Symfony: límites, usuario actual, acceso y reloj

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Es una refactorización: ninguna respuesta del servidor cambia** (salvo lo que se dice en 1.4). Solo `server/` y las líneas de `docs/` que lo describan. Sale de las filas 1, 3, 4, 5 y 12 de [`docs/research/analisis-calidad-2026-10.md`](../research/analisis-calidad-2026-10.md): léelo antes (la parte 1). El maintainer quiere que en el servidor se usen los mecanismos del framework (atributos, configuración) antes que código a mano, y lo revisará con ese criterio.

Comprobado ya en `vendor/` (Symfony 8.1, PHP 8.5):
- `Symfony\Component\HttpKernel\Attribute\RateLimit(limiter, key, tokens, methods)`, repetible. Lo aplica `RateLimitAttributeListener` en `kernel.controller_arguments`, **después** de resolver los argumentos.
- `key` puede ser una cadena, una `Expression` o un `Closure`. **`symfony/expression-language` no está instalado: usa `Closure`, no lo instales.** El cierre recibe `(array $args, Request $request, ?object $controller)`, y `$args` son los argumentos ya resueltos de la acción, por nombre.
- Sin `key`, la clave es `ip~método~ruta`: un cubo por ruta. **No lo queremos en ningún caso**: da la clave siempre.
- Si se agota, lanza `TooManyRequestsHttpException` con la cabecera `Retry-After`.

## 1. `#[RateLimit]` en los límites sencillos

### 1.1 Qué se mueve

Las llamadas a `RequestLimiter::consume*` al principio de cada acción pasan a ser atributos `#[RateLimit]` en la acción. Las mismas factorías de `config/packages/rate_limiter.yaml` y **las mismas claves que hoy**, para que cada cubo siga igual, también los compartidos:

| Hoy | Limitador | Clave |
|---|---|---|
| `consumeLogin` (`AuthController`, `TwoFactorController`, `AccountController`: un cubo compartido) | `login` | IP |
| `consumeRegistration` | `registration` | IP |
| `consumeEmailCode` (5 rutas de `AccountController`, un cubo compartido) | `email_code` | IP |
| `consumeGoogle` (4 rutas) | `google` | IP |
| `consumeMetadata` (5 rutas) | `metadata_account` | id de la cuenta |
| `consumeCloudPush` · `consumeCloudPull` | `cloud_sync` | id de la cuenta + `\|push` · `\|pull` |
| `consumeCloudLink` | `cloud_link` | id de la cuenta |
| `consumeCloudEvents` | `cloud_events` | id de la cuenta |
| `consumeInvitationLookup` | `invitation_lookup` | IP |
| `consumeInvitationAccept` | `invitation_accept` | id de la cuenta |

- La IP es `(string) $request->getClientIp()`, como hoy (con `TRUSTED_PROXIES`).
- El id de la cuenta es `->getId()->toRfc4122()` del usuario, sacado de `$args` (el argumento `#[CurrentUser]` de la parte 2).
- **Dónde aceptar una invitación gasta dos** (IP y cuenta): dos atributos, en el mismo orden que hoy.
- **Las claves como funciones con nombre, no cierres repetidos.** PHP 8.5 admite *first-class callables* en atributos: una clase pequeña, por ejemplo `App\Security\RateLimitKey` con métodos estáticos `ip(array $args, Request $request): string`, `account(...)`, `accountPush(...)`, `accountPull(...)`, y `key: RateLimitKey::ip(...)`. Si no compila así, cierres `static fn` y dilo en el informe.

### 1.2 Qué no se toca

Esto se queda a mano en `RequestLimiter`, porque no encaja en el atributo (fila 2 del análisis):
- `login_failed_account`: `requireLoginOpen` y `noteLoginFailure` solo cuentan los fallos;
- `consumeEmailPerAddress`, con la huella de la dirección y gastado dentro del flujo;
- los tres de correo de invitación (`requireInvitationEmailOpen` y `spendInvitationEmail`);
- `consumeInvitationCreate`, si se llama con algo que no está en los argumentos (míralo: si es solo la cuenta y va al principio de la acción, pásalo también);
- `consumeTwoFactor`, si no lo llama un controlador al principio;
- los topes de llamadas reales a los proveedores (`consume*Call`, `consume*CallOf`) y `metadataQuotaExhausted`.

`RequestLimiter` pierde los métodos que ya no llama nadie, y sus factorías del constructor. El comentario de la clase y el de la cabecera de `rate_limiter.yaml` dicen hoy que es «el único que consume»: ponlos al día. Que digan que los límites por petición van en el atributo y los que dependen del flujo, aquí.

### 1.3 La respuesta 429, igual que hoy

La app lee `{"error":"too_many_requests"}`, con `Retry-After`. `AccountExceptionListener` traduce hoy `AccountException`. Haz que traduzca también `TooManyRequestsHttpException` al mismo cuerpo (`AccountErrorCode::TooManyRequests`), con el estado 429 y la cabecera `Retry-After` que trae la excepción, sin un listener nuevo. Los tests de límites que ya existen deben pasar sin tocar sus aserciones sobre el cuerpo:
- `tests/Controller/LoginLimitTest.php`;
- `CloudLimitsTest.php`;
- `AccountControllerTest.php`;
- `InvitationControllerTest.php`;
- `TwoFactorControllerTest.php`;
- y los de metadatos (`*SearchTest.php`, `GoogleBooksLookupTest.php`).

**Ellos son la red.** Si uno falla, el fallo es del cambio, no del test, salvo lo de 1.4.

### 1.4 El único cambio de comportamiento, aceptado

El atributo se aplica después de resolver `#[MapRequestPayload]`. Por eso, una petición con un cuerpo mal formado (400 o 422) **deja de gastar** el límite. Hoy sí lo gasta. Se acepta: una petición así no llega a mirar ninguna cuenta ni la base de datos.
- Si un test comprueba justo que un cuerpo inválido gasta el límite, cámbialo para que diga lo nuevo y apúntalo en el informe.
- Pon al día las frases de `docs/architecture/cloud.md` y `docs/architecture/accounts.md` que digan que el límite «se gasta antes de mirar nada», o que se consume siempre a través de `RequestLimiter`.

Una petición bien formada con la contraseña o el código equivocados **sigue gastando**: el atributo va antes del cuerpo de la acción.

### 1.5 Sin `Request` de sobra

Las acciones que reciben `Request $request` solo para la IP del límite lo pierden (fila 12).

## 2. `#[CurrentUser]`

Los cuatro `currentUser()` privados (`TwoFactorController`, `InvitationController`, `AccountController`, `CloudController`, con `getUser()` y un `assert`) y los `getUser()` sueltos de los controladores pasan a ser un argumento `#[CurrentUser] User $user`. Se borran los métodos privados.
- En las rutas públicas (las `PUBLIC_ACCESS` de `config/packages/security.yaml`) no se pide un usuario.
- Si alguna pública lo lee de forma opcional, `?User $user` y dilo.

## 3. Quitar los `#[IsGranted('ROLE_USER')]` que repiten `access_control`

`security.yaml` ya exige `ROLE_USER` en todo `^/v1`, salvo las rutas públicas listadas antes. Quita los `#[IsGranted('ROLE_USER')]`, tanto los de acción como el de clase de `CloudController`. **Antes**, si no existe ya, añade un test que recorra todas las rutas de `/v1` que no estén en la lista pública y compruebe que sin sesión dan 401. Sácalas del router, no de una lista escrita a mano, para que una ruta nueva entre sola. Si una ruta da otra cosa, para y dilo: no la arregles por tu cuenta.

## 4. El reloj

Donde se haga `new \DateTimeImmutable('now', …)` o `new \DateTimeImmutable()` en servicios y controladores de `src/` (unos 18 sitios), inyecta `Symfony\Component\Clock\ClockInterface` y usa `$this->clock->now()`. 18 ficheros ya lo hacen así: copia su forma.
- **No toques las entidades** (no se les inyecta nada) ni los `new \DateTimeImmutable($cadena)` que leen una fecha.
- Comprueba que la zona sigue siendo UTC donde hoy se pide UTC (mira cómo está configurado el reloj).
- No cambies los tests que ya pasan para que usen `ClockSensitiveTrait`: eso es otro encargo.

## Al terminar

- `make fix`, y **un solo `make check` al final** (unos 10 minutos; nunca dos a la vez). Si falla, arregla y repite solo lo que falló (`make console`, o PHPUnit con `--filter`), y al final un `make check` más.
- **El informe:**
  - la tabla de 1.1 con lo que quedó en atributo;
  - lo que quedó a mano y por qué;
  - si funcionaron los *first-class callables*;
  - el test de 401 (si lo añadiste y qué rutas recorre);
  - los sitios del reloj;
  - cualquier test que hayas cambiado y por qué;
  - las líneas de docs tocadas;
  - el resultado de `make check`.
