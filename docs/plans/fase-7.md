# Plan de la Fase 7 — La nube

> Plan de trabajo mientras dure la fase. Se basa en D38, D39, D41–D43, D45, D48–D52, D57, D58, D62, D66, D67, D71, D73, D75, D76, D77, D78 y D79 ([DECISIONS.md](../DECISIONS.md)), en el [ROADMAP](../ROADMAP.md) y en lo que sigue valiendo del [plan anterior del servidor](../historico/plan-servidor-2026-09.md). Escrito el 01/10/2026. Al cerrar la fase pasa a `historico/`.

**Hecho cuando:** un amigo de otra ciudad y yo compartimos una biblioteca a través de la nube.

## El principio: la sincronización no cambia

Decidido por el maintainer al pedir el plan (01/10/2026): **la forma de sincronizar se queda como está.** La nube es un dispositivo más (D57), siempre encendido, que habla el mismo protocolo que la red local ([sync.md](../architecture/sync.md)): un vector de versiones por biblioteca, `pull` y `push`, operaciones firmadas por cada dispositivo (D56) y los permisos juzgados por cada uno con las reglas de D73.

Lo que sí es nuevo:
- **el transporte:** HTTPS con un certificado de verdad en lugar del certificado fijado por el QR (D34), y un token de dispositivo que se obtiene con la cuenta en lugar del secreto del emparejamiento;
- **quién es quién:** la cuenta (D57), que se enlaza con una persona de cada biblioteca;
- **lo que solo tiene sentido con un servidor:** registro con email, avisos al momento, invitación por enlace, registro de actividad con hora y dirección de confianza, y proveedores con clave.

## Lo que ya existe

- **Servidor (hito 7.0):** Symfony 8.1, PHP 8.5, PostgreSQL 18 y Docker propio; valida la estructura de las operaciones y sus firmas con los casos de `spec/` (`operation-validation.json`, `signatures.json`). No tiene entidades, ni controladores, ni motor de fusión, ni permisos.
- **App:** el protocolo de sincronización en `sync.rs` y `lan/`, con su servidor y su cliente HTTPS; permisos y peticiones en `permissions.rs` y `requests.rs`; las reglas puras de permisos en `crates/domain/src/permission.rs`.
- **Casos compartidos:** `library-merge.json` y `permissions.json` ya existen y la app los pasa. **Faltan** los de las reglas básicas de fusión (campo a campo, el borrado gana, referencias rotas), que [sync.md](../architecture/sync.md) ya apuntaba como pendientes para el servidor.
- **Pantallas de Design:** las de la Fase 3 no dibujan la cuenta ni la nube; se piden al empezar esta fase (ROADMAP, Fase 3).

## Hitos

Cada hito deja la app y el servidor funcionando, con sus tests, y se prueba antes de seguir.

| Hito | Qué | Dónde |
|---|---|---|
| **7.1 Cuentas** | Registro con email y código (D42, D49) o con Google (D77), segundo factor opcional y desbloqueo con biometría (D78), iniciar sesión y dar de alta el dispositivo con su token, recuperar la contraseña (D50), límites de peticiones, cuentas sin confirmar que caducan, borrar la cuenta y superadministrador por consola (D51). En la app: las pantallas de la cuenta | `server/`, `app/` |
| **7.2 Sincronizar con la nube** | `pull` y `push` en el servidor, con el mismo formato que por la red local; el servidor guarda el registro de operaciones de cada biblioteca, comprueba su estructura y sus firmas, y solo entrega una biblioteca a quien tiene acceso. **Subir una biblioteca** es la primera sincronización: viaja todo el historial. En la app: la nube como un sitio más de cada biblioteca | `server/`, `app/src-tauri` |
| **7.3 Avisos al momento** | Con la app abierta, el servidor avisa en cuanto hay algo nuevo en una biblioteca y la app sincroniza (D75). Sustituye a preguntar cada cierto tiempo. Por la red local, un cambio se envía al momento a los dispositivos a la vista | `server/` (Mercure), `app/` |
| **7.4 El servidor aplica y juzga** | El motor de fusión y los permisos de D73 en PHP, con las tablas del catálogo en PostgreSQL (D41); recalcular una biblioteca cuando llega algo tarde. Antes: los casos de fusión que faltan, en `spec/`, pasando en Rust | `spec/`, `server/` |
| **7.5 La cuenta en la biblioteca** | Enlazar una cuenta con una persona; un dispositivo nuevo de la misma cuenta entra en las bibliotecas de esa persona sin que nadie lo vincule a mano; recuperar el control con el email (D58). Retirar el acceso de una persona, al momento (D71) | `spec/`, `crates/domain`, `server/`, `app/` |
| **7.6 Invitar con un enlace** | Un enlace de un solo uso que caduca en 24 horas (D43), para una persona nueva o para una que ya está en la biblioteca, con el rol que elige quien invita. El QR sigue igual | `server/`, `app/` |
| **7.7 Registro de actividad** | La tabla de solo añadir de D66, con hora del servidor, IP y app; su plazo y la purga | `server/` |
| **7.8 Proveedores con clave** | BoardGameGeek, TMDB, IGDB o RAWG y Comic Vine a través del servidor, que pone las claves (D62); se activan solos al conectarse | `server/`, `app/src-tauri/src/metadata` |
| **7.9 Producción** | Imágenes y `docker-compose` de producción (D48), despliegue en el VPS, copias de PostgreSQL, y la guía para quien monte su propio servidor | `server/docker/`, `docs/` |

Las pantallas de la cuenta, la nube y las invitaciones se piden a Claude Design antes de construir la interfaz de 7.1. Los textos de error nuevos (código caducado, invitación caducada o usada, sesión cerrada por cambio de contraseña) van a [design/errors.md](../design/errors.md), como piden D43 y D50.

**Orden.** Con 7.1 y 7.2 ya se puede compartir a distancia, porque cada dispositivo juzga lo que recibe. 7.4 hace que el servidor entienda los datos (D57) y es el hito más grande: es portar a PHP buena parte de `sync.rs` y `permissions.rs`. Si conviene probar antes con el amigo, 7.9 puede adelantarse detrás de 7.3.

## Propuestas técnicas

Todo es **propuesta** hasta que el maintainer lo revise; lo que toca una decisión aceptada está en las preguntas.

### 7.1 Cuentas

- **Entidades (ORM):** `User` (email, contraseña con argon2id, nombre, confirmado o no, bloqueado), `Device` (cuenta, id de dispositivo, que es **el mismo de su `hlc`**, nombre, hash del token, último acceso) y `EmailCode` (para qué es, hash del código, caducidad, intentos).
- **Token del dispositivo** como en la red local: `X-Device-Id` y `Authorization: Bearer`; el servidor solo guarda su hash. En la app, el token se guarda en `local_state`, como la clave de firma, y **no va en las copias** (D58).
- **Iniciar sesión también prueba la clave del dispositivo:** la app firma un reto del servidor con su clave Ed25519, y el servidor guarda la clave pública junto al token. Así una cuenta no puede reclamar el id de un dispositivo que no es suyo.
- **Continuar con Google** (D77): por el navegador del sistema, nunca dentro de la app. El servidor hace el OAuth (`knpuniversity/oauth2-client-bundle`, con PKCE y un autenticador propio; elegido frente a HWIOAuthBundle el 01/10/2026, porque la app acaba con un token de dispositivo y no con una sesión web) y devuelve a la app un código de un solo uso, que la app cambia por su token. Si el email de Google ya tiene cuenta, se enlaza con ella. El servidor dice si ofrece Google; si no, solo email. Los pasos para configurarlo, en [development/servicios-externos.md](../development/servicios-externos.md).
- **Segundo factor y biometría** (D78): TOTP con `spomky-labs/otphp` y un flujo propio (D78, revisada el 02/10/2026) y códigos de respaldo, opcional para cada usuario y desactivable por quien aloja el servidor; desbloqueo de la app con el plugin `biometric` de Tauri, también sin cuenta.
- **Límites** con `symfony/rate-limiter` por IP en el alta, el inicio de sesión y los códigos, y por dirección en los correos enviados (D49). `trusted_proxies` bien puesto, como en Riddle.
- **Cuentas sin confirmar** que se borran a las 24 horas, con una tarea de Symfony Scheduler.
- **Borrar la cuenta** (lo pedirá Google Play): borra la cuenta y sus dispositivos; las bibliotecas siguen para los demás, y las que solo eran suyas se borran (con aviso y copia previa, como D65).
- **La app y el servidor:** «Iniciar sesión» lleva al servidor oficial, con «Otro servidor» para quien aloja el suyo (D77).

### 7.2 Sincronizar con la nube

- **Mismas rutas que la red local** (`/v1/sync/pull` y `/v1/sync/push`) y mismos cuerpos: versión de esquema, vectores por biblioteca, operaciones y avisos. El código de la app se reutiliza; lo que cambia es el transporte (pregunta técnica para Rust: un *trait* que tienen los dos clientes, el de la red local y el de la nube).
- **Registro de operaciones en PostgreSQL** (`operations`, con el cuerpo en JSONB, la biblioteca y el dispositivo autor), del que salen los vectores, como en la app.
- **El servidor rechaza solo lo inválido**, como cualquier dispositivo: estructura, versión de esquema, firma o acceso retirado. Lo no permitido no se rechaza: llega como petición (D73, que reemplaza a D47). Lo rechazado no se reenvía.
- **La red local sigue a la vez** (D39): un cambio que llega por los dos caminos no se aplica dos veces, por el id de la operación y el vector de versiones.
- **Quién recibe una biblioteca:** un dispositivo de una cuenta enlazada con una persona de esa biblioteca (7.5), que no esté retirado (`revocation`). Hasta 7.5, quien la subió.
- **Subir una biblioteca** la puede hacer quien puede compartirla (administrador o «Puede compartir», D73; sin roles, cualquiera), igual que al emparejar. En la app, «Subir a la nube» añade la nube a `library_peers` y sincroniza: viaja todo el registro, firmas incluidas, y no se pierde nada.
- **Cuándo se sincroniza:** como con la red local (al abrir, al volver a primer plano y a mano), más tras cada cambio local, con un pequeño retraso para juntar varios. Sin preguntar cada cierto tiempo: de eso se encargan los avisos (7.3).

### 7.3 Avisos al momento (D75)

- **Mercure** en el servidor (`symfony/mercure-bundle` y el hub oficial en su propio contenedor), porque PHP-FPM no sirve para mantener conexiones abiertas. Tras aplicar un `push`, el servidor publica «hay algo nuevo» en el tema de esa biblioteca, sin datos.
- **El núcleo de Rust** mantiene una sola conexión con el servidor mientras la app está abierta, suscrita a las bibliotecas de la cuenta (con un token de Mercure que da el servidor solo para esas), reconecta sola y, al recibir un aviso, sincroniza esa biblioteca.
- **La interfaz** escucha un evento de Tauri y refresca lo que se ve; añade un indicador discreto de conectado o sin conexión y el contador de la campanita.
- **Red local:** tras un cambio, se envía al momento a los dispositivos emparejados que respondan.
- **Opcional (D18):** sin Mercure (un servidor propio que no lo monte), la app sincroniza al abrirse, al volver a primer plano y tras cada cambio.
- Las notificaciones con la app cerrada (FCM en Android) son de la Fase 8.

### 7.4 El servidor aplica y juzga

- **Primero los casos de fusión** que faltan, en `spec/conformance/merge.json`: campo a campo por HLC, el borrado gana, una operación repetida no cambia nada, el orden de llegada no importa y la referencia rota se salta. Pasan primero en Rust, contra `sync::merge`, y después en PHP.
- **Tablas del catálogo en PostgreSQL**, espejo de las de SQLite, con clave `(library_id, id)` también en las foráneas (D41), y `field_versions` y `tombstones` por biblioteca.
- **Motor con DBAL**, sin ORM, en `server/src/Sync/`: SQL directo en una transacción, guiado por `CatalogEntity`, como el motor de Rust por `schema.rs`.
- **Permisos:** las mismas reglas que `crates/domain/src/permission.rs`, en PHP, con `permissions.json`; y recalcular la biblioteca desde el registro cuando llega algo tarde.

### 7.5 La cuenta en la biblioteca

Decidido en D76 (01/10/2026):
- **Subir es por biblioteca:** al iniciar sesión, la app pregunta cuáles subir; solo quien puede compartirlas.
- **Enlace automático:** si el dispositivo que inicia sesión ya es de una persona en una biblioteca de la nube, la cuenta queda enlazada con ella. Un dispositivo sin persona no enlaza nada; uno de otra persona hace que la app pregunte.
- **La nube vincula los dispositivos nuevos** de una cuenta enlazada, si quien administra le da ese poder (casilla marcada al subir, que se puede quitar). Así un móvil nuevo recibe sus bibliotecas con su rol, y la administradora única recupera el control con el email (D58).
- Cuando alguien sale o lo echan, la nube deja de entregarle la biblioteca al momento (D71).

Propuestas para construirlo:
- **Enlace cuenta–persona en el servidor** (D76), en una tabla suya (`membership`: cuenta, biblioteca, id de la persona). **El rol no se copia ahí:** se lee siempre de la fila `member`. Con la biblioteca solo viaja `member.account_id`, un id aleatorio de la cuenta que escribe la nube, para saber quién tiene cuenta; el email nunca sale del servidor.
- El poder de la nube, como regla nueva de permisos en `spec/` con sus casos, en `crates/domain` y en PHP a la vez.

### 7.6 Invitar con un enlace

- Quien puede compartir crea la invitación: si la persona es nueva, una operación firmada crea su fila `member` con el rol elegido (un rol nunca se hereda, D73), y el servidor guarda la invitación con el id de esa persona, el hash del token y la caducidad.
- Al abrirla, la app pide crear la cuenta o iniciar sesión; el servidor enlaza la cuenta con esa persona (7.5) y la biblioteca llega entera.
- El enlace da exactamente eso: entrar una vez, como esa persona, con ese rol. Un enlace usado o caducado no da nada y la app lo explica (D43).

### 7.7 Registro de actividad

- Tabla de solo añadir, sin claves foráneas hacia lo que registra, para que sobreviva a lo borrado (D66). Se llena con lo que hace la cuenta (inicio de sesión, cambio de contraseña, borrar la cuenta) y con las operaciones que llegan con ciertos cambios (crear o borrar una biblioteca, roles, «Puede compartir», de quién es un dispositivo, retiradas), con la hora y la IP de cuando llegaron.
- **Se guarda un año** (D66), con purga automática. Se cuenta en la política de privacidad.
- Es también la tabla de eventos para los KPI del panel futuro (D52).

### 7.8 Proveedores con clave

- Rutas del servidor que buscan por la app (`/v1/metadata/…`) con sus claves, con límite por cuenta y caché. En la app, la nube es una fuente más del mismo *trait*; si no responde, siguen los proveedores sin clave (D18).
- Las condiciones de cada proveedor (uso no comercial, atribución) se revisan en [research/proveedores.md](../research/proveedores.md) antes de integrarlo; uno a uno.

### 7.9 Producción

**El VPS** (datos del maintainer, 01/10/2026; se revisan al llegar):
- Pequeño: poca CPU y poca memoria, un Ubuntu reciente (por mirar), disco suficiente para empezar (por mirar cuánto). Si la app pide más, se verá entonces.
- Es un servidor **compartido** con otro proyecto del maintainer.
- **Sin Docker:** se despliega directamente, con PostgreSQL en la propia máquina. Es lo más barato, y el proyecto se hace por amor al arte.
- nginx y Certbot delante.
- **Sin copias todavía:** hay que empezar, también para el otro proyecto del servidor.

**Lo que eso cambia, a revisar en 7.9:**
- **D48 se cumple sin Docker** en el VPS oficial: nginx + PHP-FPM + PostgreSQL instalados en la máquina. Sus consecuencias («imágenes de producción y un `docker-compose` de producción») habrá que revisarlas: el `docker-compose` puede quedarse solo para quien se aloje el suyo, o dejarse para más adelante. Se habla antes de tocar D48.
- **Versiones:** Ubuntu no trae PHP 8.5 ni PostgreSQL 18 en sus repositorios; harán falta el PPA de Ondřej Surý (PHP) y el repositorio oficial de PostgreSQL. Varias versiones de PHP-FPM conviven sin problema, cada una con su *pool*; si el otro proyecto usa otro PostgreSQL, se valora si conviven las dos versiones o se actualiza.
- **Memoria:** con poca memoria para los dos proyectos, PHP-FPM con pocos procesos, PostgreSQL ajustado (memoria compartida pequeña) y un fichero de intercambio por si acaso. Mercure es un único binario ligero, como servicio de `systemd` detrás de nginx.
- **Copias:** volcado diario de PostgreSQL, cifrado, fuera del VPS, en algo gratuito o casi (por ejemplo, la capa gratuita de Backblaze B2), con días, semanas y meses, y una prueba de restauración documentada. Sirve igual para el otro proyecto del servidor.
- **Actualizar:** un script que hace una copia, baja la versión nueva, aplica las migraciones y recarga PHP-FPM, con forma de volver atrás.
- **Guía para quien monte su servidor**, en `docs/`: requisitos, variables, correo, proxy con HTTPS, copias y actualización.

## Preguntas

Lo demás quedó decidido al preparar la fase (01/10/2026); queda una, para una conversación aparte.

1. **Correo** (D49): el buzón con el dominio propio y el servicio que envía los códigos se deciden en una conversación aparte, cuando el maintainer la pida (01/10/2026). Se da por hecho que habrá correo con el dominio. Mientras tanto, en desarrollo los correos los recoge **Mailpit** (un contenedor más en `server/docker/`, con su bandeja en el navegador), así que 7.1 no espera a esa decisión.

## Fuera de esta fase

- El panel de administración (D52), la web (D53) y las organizaciones (D59): Fase 11. Hasta entonces, comandos de consola.
- Copias automáticas en la nube (D58 las deja fuera del roadmap) y Google Drive desde el móvil (aplazado en la Fase 4).
- Otras redes sociales para entrar: solo email y Google (D77); se pueden añadir después.

## Antes de empezar

- **Auditoría:** como en la Fase 6, antes de construir nada.
- **Pruebas en pantalla de las fases 4, 5 y 6:** el maintainer las hace el fin de semana del 03/10/2026, con todo lo construido. Si salen fallos en la sincronización, se arreglan antes de 7.2.

## Estado

- [x] 7.0 Esqueleto del servidor (hito de la revisión anterior; ver el ROADMAP).
- [x] Plan escrito (01/10/2026).
- [x] D39 aceptada (nube y red local a la vez) y D47 reemplazada por D73 (la nube juzga como un dispositivo más) (01/10/2026).
- [x] D76 aceptada (enlace automático y la nube vincula los dispositivos nuevos) y D77 aceptada (entrar con email o con Google) (01/10/2026).
- [x] Datos del VPS apuntados en 7.9 (01/10/2026).
- [x] D79 aceptada: `api.kosmotheka.app`, invitaciones en `kosmotheka.app/i/…` y la web en `kosmotheka.com` (01/10/2026).
- [x] D78 aceptada (biometría y segundo factor, opcionales, en 7.1); «Otro servidor» en D77 y un año para el registro de actividad en D66 (01/10/2026).
- [x] D75 aceptada: avisos al momento con Mercure en esta fase; con la app cerrada, FCM en la Fase 8 (01/10/2026).
- [ ] 7.1 Cuentas. **Servidor, parte básica hecha (02/10/2026):** correo (Mailpit en desarrollo, Brevo en producción), límites de peticiones, registro con código, inicio de sesión del dispositivo con reto firmado y token, cuenta y dispositivos, recuperar y cambiar la contraseña, borrar la cuenta, purga cada hora y comandos de consola; ver [architecture/accounts.md](../architecture/accounts.md). Después (02/10/2026): dispositivo quitado y deshacer (D81) y segundo factor con `otphp` (D78 revisada). Falta: Google y todo lo de la app; encargo de pantallas en [design/encargo-fase-7-cuentas.md](../design/encargo-fase-7-cuentas.md).
