# Análisis de calidad: lo que el framework ya resuelve (17/10/2026)

> Solo lectura y este informe: no he cambiado código. No es una auditoría de seguridad. Todo mecanismo que propongo lo he comprobado en `server/vendor/` (versión instalada, Symfony 8.1) o está dicho como «por mirar». Con `grep` y lectura de lo imprescindible, sin leer el servidor entero: lo que digo de un fichero que no abrí entero lo digo así. `make check` pasa (1.958 tests) con el cambio de `schema:validate` de hoy.

## Resumen

1. **`#[RateLimit]` cubre unas 24 de las 30 llamadas a mano** de los controladores, pero no es un cambio mecánico: el límite se gasta *después* de resolver el cuerpo (un cuerpo inválido deja de gastar) y responde con otro cuerpo de error. Rentable y con riesgo medio; ver la fila 1.
2. **Quitar `currentUser()` (4 copias) y los `#[IsGranted('ROLE_USER')]` redundantes** (`access_control` ya exige `ROLE_USER` en todo `/v1`): es lo más barato y lo más limpio. `#[CurrentUser]` existe y hoy no se usa en ningún sitio.
3. **Los cuatro proveedores de metadatos repiten su esqueleto**: `*Search` casi igual (tras cambiar los nombres, solo difieren ~58 de 200 líneas entre TMDB y RAWG) y `*HttpSource` con su `read()` casi igual. Una base común ahorra unas 600 líneas, pero es la parte más delicada del servidor (claves en la dirección, excepciones sin URL): solo con los tests delante.
4. **El reloj**: 18 ficheros usan `ClockInterface` y ~21 sitios siguen haciendo `new \DateTimeImmutable('now', UTC)`. Unificarlo hace los tests más fáciles (`ClockSensitiveTrait` existe).
5. **En Rust no hay nada urgente.** Errores con `thiserror` en todas partes, `From` donde toca, sin `map_err(|e| e.to_string())`. Lo rentable: tres copias de `now_millis`/`now`, y media docena de lints de `pedantic` que sí aportan.

## Parte 1 · Servidor (Symfony 8.1)

Ordenada de más a menos rentable. Esfuerzo: S pocas líneas, M un rato con tests, L varias sesiones.

| # | Qué | Dónde | Mecanismo (comprobado en `vendor/`) | Ganancia | Riesgo | Esf. |
|---|---|---|---|---|---|---|
| 1 | Límites de peticiones llamados a mano: **30 llamadas en 8 controladores** (Account 8, Metadata 5, Google 4, Cloud 4, Invitation 3, TwoFactor 3, Auth 2, CloudEvents 1). Encajan: `consumeRegistration/Login/Google/EmailCode`, `consumeCloudPush/Pull/Link/Events`, `consumeMetadata`, `consumeInvitationLookup/Accept/Create`, `consumeTwoFactor` (controlador) | `src/Controller/*`; `src/Security/RequestLimiter.php` (335 líneas) | `#[RateLimit(limiter:, key: Closure\|Expression, tokens:)]`, repetible (`http-kernel/Attribute/RateLimit.php` + `RateLimitAttributeListener`) | Cada acción declara su límite; `RequestLimiter` pierde ~15 métodos; se ve a simple vista qué ruta tiene qué límite | **Medio.** (a) El listener corre en `ControllerArguments`, o sea *después* de resolver `#[MapRequestPayload]`: un cuerpo inválido da 400/422 sin gastar el límite, hoy sí lo gasta (importa para SEG-067). (b) Lanza `TooManyRequestsHttpException`, no `AccountException::tooManyRequests`: cambia el cuerpo de error `{"error":…}` que usa la app, salvo que un listener lo traduzca. (c) Sin `key` la clave es `ip~método~ruta`: cinco rutas que hoy comparten el cubo `consumeEmailCode` pasarían a tener uno cada una; hay que dar la clave a mano | M |
| 2 | **No encajan** en `#[RateLimit]` y se quedan a mano: `login_failed_account` (`requireLoginOpen`/`noteLoginFailure`, solo cuenta fallos), `consumeEmailPerAddress`, los tres de correo de invitación (`require…`/`spend…`), y los topes de llamadas reales a proveedores (`consume*CallOf`, gastados en un `Closure` solo cuando la caché falla) | `RequestLimiter`, `ProviderGate`, `*Search` | — | — | — | — |
| 3 | `currentUser()` privado repetido 4 veces (`getUser()` + `assert`) | `TwoFactorController:115`, `InvitationController:103`, `AccountController:358`, `CloudController:137` (42 usos de `getUser()`/`currentUser()` en controladores; `#[CurrentUser]`: 0) | `#[CurrentUser] User $user` como argumento (`security-http/Attribute/CurrentUser.php`) | Menos código; el argumento dice que la ruta exige sesión; elimina los `assert` | Bajo: en rutas públicas que lo usen, el argumento sería `null` | S |
| 4 | `#[IsGranted('ROLE_USER')]` en unas rutas sí y en otras no, aunque `security.yaml` ya dice `^/v1 → ROLE_USER` (salvo las públicas listadas) | `InvitationController:43,61,71,91`, `CloudController:35` (clase), `AccountController:151,200,223` | Quitarlos (la regla está en `access_control`) o, al revés, ponerlos en todos y dejar `access_control` solo para las públicas | Una sola forma de decirlo. Hoy parece que falta protección donde no hay atributo, y no falta | Bajo, pero conviene dejar un test que pruebe «toda ruta de `/v1` sin sesión → 401» | S |
| 5 | Reloj: `new \DateTimeImmutable('now', UTC)` en ~21 sitios (p. ej. `AccountController:126,305`, `DeviceTokenAuthenticator:78,127`, `DeviceSignIn:93`, `EmailCodeIssuer:51,103`) frente a `ClockInterface` en 18 ficheros | `src/Controller`, `src/Security` | `Symfony\Component\Clock\ClockInterface` inyectado; `ClockSensitiveTrait` en tests (`symfony/clock/Test`) | Un solo origen del tiempo; se pueden fijar fechas en los tests en lugar de dormir | Bajo | M |
| 6 | Esqueleto repetido de los cuatro `*Search` (caché por huella, `gate->requireOpen`, `gate->call`, `remember`, detalle por hit) | `TmdbSearch` (200), `RawgSearch` (188), `ComicVineSearch` (201), `BggSearch` (192); `BookLookup` (106) aparte | Clase abstracta o *trait* con el flujo «huella → caché fresca → puerta → llamada → recordar»; cada proveedor da la huella, la fuente y el mapeo | ~400 líneas menos; un fallo se arregla una vez | **Medio-alto:** es donde está la regla «sin guardar quién preguntó ni el texto». Solo con los tests de cada proveedor delante y sin cambiar la huella (`fingerprint` es estática y pública en cada uno: los tests la usan). **Por mirar** si los tres que traen detalles se parecen tanto como TMDB y RAWG | L |
| 7 | `*HttpSource` repiten `begin()`/`start()`/`read()` (estado 401/403/429 → cuota, ≥500 → rota, ≠200 → inservible) y las opciones `timeout`/`max_duration`/`max_redirects` | los cuatro `*HttpSource` (260 a 328 líneas) y `GoogleBooksHttpProvider`; `TIMEOUT_SECONDS` 5,0 / 6,0 repetido en cinco | Un cliente con nombre en `framework.http_client.scoped_clients` que ponga `timeout`, `max_duration`, `max_redirects` y `Accept`; y un método común `read(ResponseInterface)` | Opciones HTTP en un solo sitio (config); menos código por proveedor | Medio: RAWG y Comic Vine llevan la clave en la dirección y usan un cliente sin registrador a propósito (`UnloggedClientFactory`): ese tiene que seguir fuera del cliente con nombre | M |
| 8 | Reintentos: no hay (`RetryableHttpClient` no se usa) | `*HttpSource` | `RetryableHttpClient` | Posible ganancia ante 5xx puntuales | **No lo recomiendo hoy:** cada reintento gasta cuota del proveedor y el límite diario se cuenta antes; está bien que falle y se pause (`ProviderGate`) | — | — |
| 9 | Descodificar `CLOUD_KEY_SECRET` a mano (`CloudKeyVault` lo recibe como cadena y comprueba el base64 por dentro, con su excepción) | `config/services.yaml:61`, `src/Security/CloudKeyVault.php` | `%env(base64:CLOUD_KEY_SECRET)%` (existe: `EnvVarProcessor.php:289`) | Poco: ahorra una línea de descodificación | Medio: perdería el mensaje propio «32 bytes en base64» y distinguir vacío de inválido; **no merece la pena** | — | — |
| 10 | Dos `random_bytes(32)` + `sodium_bin2base64(..., URLSAFE_NO_PAD)` iguales para tokens/retos; `rtrim(strtr(base64_encode()))` en Google; `sodium_bin2base64`/`base642bin` en 13 ficheros | `DeviceSignIn:111`, `LoginChallengeIssuer:36`, `LeagueGoogleSignIn:41,46`; 13 ficheros en total | Una clase pequeña `Base64Url` (no hay componente de Symfony que lo haga) | Un único lugar para el formato (y una sola forma de descodificar con error) | Bajo | S |
| 11 | Respuestas de éxito `new JsonResponse(…)` a mano (29) | `src/Controller/*` | `#[Serialize]` (existe: `http-kernel/Attribute/Serialize.php`) y devolver el DTO | Controladores más cortos | Medio: el formato del cable es `snake_case` con `#[SerializedName]` ya puesto a mano; hay que probar que los cuerpos salen byte a byte iguales. **Por mirar** | M |
| 12 | `Request $request` solo para `getClientIp()` (21 usos) | los mismos controladores de la fila 1 | Desaparece con `#[RateLimit]` (la clave por IP la saca el listener) | Firmas de acción más limpias | — (va con la 1) | — |
| 13 | Caché de metadatos propia en base de datos (`MetadataCacheEntry`, `MetadataCacheRepository`, purga) | `src/Entity/MetadataCacheEntry.php`, `src/Repository/MetadataCacheRepository.php`, `*Search` | `symfony/cache` con *pools* y etiquetas (el pool `cache.metadata` ya existe para las pausas) | Menos código propio | **Es a propósito:** la fila guarda `found`, el momento y el aviso «sin quién preguntó», sobrevive a un despliegue y se consulta por proveedor y huella; la huella de un texto no se puede recuperar. Con `cache.app` en ficheros se perdería la persistencia o habría que poner una base. **No tocar.** | — | — |
| 14 | Dos `Uuid::v7()` en constructores de entidades (4) y `random_bytes` para claves | `Entity/*` | `symfony/uid` — ya se usa | Nada que ganar | — | — | — |

Otros puntos mirados y **sin hallazgo**: validación de entrada (27 `#[MapRequestPayload]`, ningún `json_decode`/`getContent` en controladores; sin `MapQueryString` porque no hay consultas con parámetros); variables de entorno (`#[Autowire(env: …)]` y `bool:` ya se usan en todos los servicios que leen configuración, 0 `getenv`); bloqueos (los de biblioteca son `PESSIMISTIC_WRITE` de Doctrine, lo correcto para filas; el `LockFactory` solo está en `LoginChallengeIssuer`); tareas programadas (un `#[AsSchedule]` único, `PurgeSchedule`, y 6 comandos de consola con `#[AsCommand]`); *voters* (no hay reglas de acceso por objeto: el acceso a la biblioteca va por `CloudLibraryAccess` y el juez, que son del dominio).

Fuera de las 30 filas: nada más. Dos cosas menores sin fila: `AccountController` pasa de 350 líneas con 8 llamadas de límite que se van con la fila 1; y el helper `fingerprint` de cada proveedor tiene el mismo nombre con firmas distintas (por mirar al hacer la 6).

## Lo que está bien hecho a propósito y no conviene tocar

- **`ValidOperation` a mano**, con el orden de reglas y los códigos fijados por `spec/` y comparado con Rust (dicho en `server/AGENTS.md`).
- **El motor de fusión con DBAL** (`Sync/Merge/`), una sola clase con SQL (`CatalogSql`) generada desde los metadatos de Doctrine.
- **`MailOutbox` en `kernel.terminate`** en lugar de Messenger: es para que el tiempo de la respuesta no diga si hay cuenta (D100) y para que quien se aloje su servidor no tenga una cola más que vigilar.
- **`MetadataCacheEntry` en base de datos**, con la huella sin texto (fila 13).
- **`UnloggedClientFactory`** (clientes sin registrador para RAWG y Comic Vine): las claves van en la dirección.
- **El `AccountExceptionListener` + `AccountException`**: no hay errores construidos a mano en los controladores (0 `JsonResponse(['error'`); todos pasan por la excepción y su listener.
- **`$request->getClientIp()` con `TRUSTED_PROXIES`**: la IP real que usan los límites sale del framework, no de cabeceras leídas a mano.
- **`#[Autowire(env: …)]`** en todos los lugares que leen configuración.

## No sobreingeniería (servidor)

Solo lo que de verdad sobra; poco:

- **`RequestLimiter` como fachada de ~25 métodos `consumeX`** que solo llaman a `consume`/`requireOpen` con un cubo distinto. Es la contrapartida de la fila 1, no una pieza mala: una vez migrados los controladores, quedaría lo que no encaja (fila 2).
- **Las interfaces `*Source`** (`TmdbSource`, `RawgSource`, `ComicVineSource`, `BggSource`, `BookProvider`) tienen una implementación real (`*HttpSource`) y otra falsa en tests. **Se justifican por los tests** sin red (regla de `AGENTS.md`), así que no sobran; lo único que sobra es el nombre repetido de métodos casi idénticos (fila 6).
- No encontré capas que solo pasen llamadas ni clases con una sola llamada fuera de lo anterior. **Por mirar:** `Sync/` tiene muchas clases pequeñas (`CloudAuthor`, `CloudLink`, `CloudPush`, `CloudIdentityIssuer`, `AccessCutter`…); no las he leído enteras y no puedo decir si alguna sobra.

## Parte 2 · App (Rust y Tauri)

El maintainer no tiene criterio en Rust, así que cada punto lleva su porqué.

### 2.1 Lo mecánico: `cargo clippy -- -W clippy::pedantic`

`clippy` es el revisor automático de Rust; `pedantic` es el grupo más estricto, que el proyecto no usa entero (solo cinco lints de ese grupo, ver `Cargo.toml`). Salen **533 avisos** (muchos salen dos veces, en el código y en sus tests, así que los distintos son bastantes menos). Agrupados:

| Aviso (nombre de clippy) | Cuántos | ¿Merece la pena? | Por qué |
|---|---|---|---|
| Falta la sección «# Errors» en la documentación (`missing_errors_doc`) | 126 | **Ruido** | Pide describir en cada función qué errores devuelve; con un tipo `Error` común y mensajes claros no añade nada |
| Argumento pasado por valor y no consumido (`needless_pass_by_value`) | 186 | **Revisar los de `String`/`Vec`** | Cuando una función recibe `String` y solo la lee, es más barato y claro recibir `&str`. En los comandos de Tauri el tipo viene impuesto (llega de JSON), así que muchos son inevitables |
| Falta backticks en la documentación (`doc_markdown`) | 49 | Ruido | Estilo de comentarios |
| Falta `#[must_use]` (`must_use_candidate`) | 38 | Ruido | Avisa si se ignora el valor devuelto; solo útil en una biblioteca pública |
| `async` sin `await` (`unused_async`) | 25 | **Sí, mirar** | Una función `async` que no espera nada cuesta algo y confunde; en los *trait* de pruebas (`FakeCloud`) es inevitable |
| Más de 3 booleanos en una estructura (`struct_excessive_bools`) | 8 | Mirar | Varias casillas sí/no juntas suelen ser un estado mal modelado (un `enum` lo expresaría mejor); aquí son campos de la API a la interfaz |
| Funciones de más de 100 líneas (`too_many_lines`) | ~12 | **Sí, mirar** | La mayor, 215 líneas. Las más largas están en `sync.rs` y `catalog.rs`; conviene partirlas al tocarlas, no antes |
| Cierre redundante (`redundant_closure_for_method_calls`) | 8 | Fácil | `|x| x.foo()` se escribe `Foo::foo`; limpieza menor |
| `format!` con `push_str` y cadenas vacías (`format_push_string`, `manual_string_new`, `string_add_assign`) | 17 | Limpieza menor | |
| Destructurar con `match` de un caso (`single_match_else`) | 7 | Estilo | |

**Lo que de verdad aporta:** las `unused_async`, las funciones largas y los `String` por valor sin necesidad. Todo lo demás es ruido para un proyecto de aplicación (la mayoría de `pedantic` está pensado para bibliotecas públicas).

### 2.2 Patrones a mano y duplicación

| # | Qué | Dónde | Mecanismo | Ganancia | Riesgo | Esf. |
|---|---|---|---|---|---|---|
| 1 | `now_millis()` copiada: dos veces igual y otra en segundos | `sync.rs:632`, `ops.rs:641`, `account/google.rs:39` (`now()` en segundos) | Una función en un módulo común (`clock.rs`) | Un solo origen del tiempo; si algún día hay que fijarlo en pruebas, un solo cambio | Nulo | S |
| 2 | Cuatro sitios construyen un `reqwest::Client` con sus tiempos y su agente | `account/client.rs:22,37`, `lan/client.rs:112`, `metadata/mod.rs:699` | Una función `http_client(timeout)` común (en Rust no hay clientes con nombre como en Symfony) | Un lugar para tiempo máximo, agente y *rustls* | Bajo | S |
| 3 | `.lock()` sobre la base de datos en ~274 sitios (137 solo en `cloud/tests.rs`) | todo `src-tauri` | Ya hay `database.lock()?` que devuelve el error de `Error`; en pruebas `.lock().unwrap()` es normal | Nada que ganar | — | — |
| 4 | Siete `impl From<…> for …` y `thiserror` en los seis tipos de error | `error.rs`, `domain/*` | — | **Ya está bien:** conversiones con `#[from]` y sin `map_err(|e| e.to_string())` (0) | — | — |
| 5 | `sync.rs` (4.526 líneas), `catalog.rs` (3.400), `commands.rs` (1.632) | `app/src-tauri/src` | Partir en submódulos por tema | Más fácil de leer y de revisar | Medio (muchos *imports*, y hay tests que lo recorren); no es urgente | L |
| 6 | Proveedores de metadatos: `bgg.rs`, `comic_vine.rs`, `tmdb.rs`, `gameupc.rs` repiten «construir la petición → leer JSON → mapear a `Candidate`»; `via_server.rs` (661 líneas) repite ese esquema por el servidor | `app/src-tauri/src/metadata/` | Una función común de «pedir y leer» por proveedor (por mirar: no los he leído enteros) | Probablemente 200–300 líneas | Medio: las pruebas con respuestas grabadas (`tests/fixtures/`) hacen de red de seguridad | M |
| 7 | Duplicación entre `crates/domain` y `app/src-tauri` | `signature`, `permission`, `operation` viven en `domain` y la app los usa (no hay copia); lo que sí está dos veces es la *especificación* en PHP, a propósito (`spec/`) | — | — | — | — |

Sin más filas: no he encontrado SQL repetido que una función común ahorraría más de lo que cuesta, ni comandos de Tauri con el mismo cuerpo (0 funciones con `State<…>` en `commands.rs`; usan una `Database` propia).

### Lo que está bien hecho a propósito en Rust

- Dominio sin almacenamiento ni interfaz (`crates/domain`), con la misma especificación que PHP.
- Errores con `thiserror` y conversión automática con `#[from]`.
- Los lints «útiles» de `pedantic` ya puestos en `Cargo.toml` (conversiones numéricas, tiempo negativo).
- Pruebas que no tocan la red (respuestas grabadas) y las `live_*` ignoradas.

### No sobreingeniería (app)

- **Nada a eliminar.** Los *trait* para la nube (`CloudApi`, `FakeCloud`) existen para probar sin red.
- **Por mirar:** la partición de `sync.rs` y `catalog.rs` es más un tema de tamaño que de abstracción.

## Recuento de filas

- **Parte 1 (servidor):** 14 filas en la tabla.
- **Parte 2 (app):** 7 filas en la tabla de patrones, más la tabla de clippy (10 grupos).
