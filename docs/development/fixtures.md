# Fixtures del servidor: cada caso normal y cada costura, en la base de datos

> Los escenarios del hito 7.2 (la nube, D84) cargados en la base de **desarrollo** de PostgreSQL para abrirla en un cliente (DBeaver, DataGrip, `psql`…) y ver con los ojos qué guarda el servidor y qué no. Los mismos escenarios corren en los tests (`server/tests/DataFixtures/ScenarioFixturesTest.php`), así que lo que lees aquí está comprobado. Contexto: [la nube](../architecture/cloud.md), [las cuentas](../architecture/accounts.md), D84 en [DECISIONS.md](../DECISIONS.md) y la «Batería de acceso» de [fase-7.md](../plans/fase-7.md).

## Cargar y conectar

```bash
cd server
make up          # si los contenedores no están arriba
make fixtures    # migra, VACÍA la base de desarrollo y la llena
```

`make fixtures` **borra todo lo que hubiera** en la base de desarrollo (`kosmotheka`): cuentas, bibliotecas, operaciones. Es una base de pruebas; la de producción no se toca nunca con esto. Al terminar imprime un resumen por escenario: qué se cargó y qué se rechazó.

Para conectar desde el equipo (los datos de `docs/development/server.md` y `server/docker/.env.dist`):

| Dato | Valor |
|---|---|
| Servidor | `localhost` |
| Puerto | `5433` |
| Base | `kosmotheka` |
| Usuario | `kosmotheka` |
| Contraseña | la de `server/docker/.env` (`POSTGRES_PASSWORD`); no se versiona |

La carga es **determinista** en todo lo que fija el cliente: ids de dispositivo, de miembro, de obra, de operación, claves de firma (cada dispositivo sale de una semilla fija de 32 bytes) y HLC. No lo es en lo que pone el servidor: el id de cada `cloud_library` (`C`, lo elige el servidor a propósito, D84) y las fechas (`created_at`, `received_at`). Por eso cada carga cambia los `C` pero no cambia ninguna operación (salvo el cuerpo de las operaciones de enlace, que nombran su `C`); el test `testLoadingAgainGivesTheSameOperations` lo comprueba.

Las cuentas tienen la contraseña `fixtures-only` (solo existe en esta base de pruebas) y cada dispositivo un token fijo (`App\DataFixtures\Cast::token()`), por si quieres llamar a la API como ellos.

## El reparto

| Quién | Qué tiene |
|---|---|
| **Ana** (`ana@garcia.test`) | Administradora de la familia García. Dos dispositivos: portátil y móvil |
| **Hugo** (`hugo@garcia.test`) | Lector. Cuenta y móvil, pero **sin membresía** en la nube: hasta 7.5 la única fila de `membership` es la de la dueña |
| **Lucía** | Hija de los García. Es un `member` de la biblioteca **sin cuenta y sin dispositivo**: solo existe como fila dentro de las operaciones |
| **Kenji** (`kenji@tanaka.test`) | Administrador de la familia Tanaka, con una biblioteca de id local **fijo por defecto** (`00000000-0000-7000-8000-000000000001`) |
| **Jhonny** (`jhonny@example.test`) | Hostil. Conoce los ids de los García (biblioteca, operaciones, dispositivos). Tiene su móvil y tres bibliotecas en la nube |

«Biblioteca García» tiene el id local `0199a3b2-7c4e-7d10-9a52-6f3e81c0b4d7` (parece aleatorio, pero es fijo). El código del reparto está en `server/src/DataFixtures/Cast.php`; los escenarios, en `ScenarioFixtures.php`, un método por escenario.

Todo entra por los servicios reales (`CloudLibraryCreator`, `CloudPush`, `CloudLibraryAccess`), nunca con `INSERT`: la base solo tiene lo que las reglas dejan entrar. Únicas excepciones: las cuentas y sus dispositivos (iniciar sesión es un controlador, no un servicio) se guardan directamente.

## Casos normales

| Escenario | Dónde mirar | Qué se ve y qué demuestra |
|---|---|---|
| **Ana sube «Biblioteca García»** | `cloud_library` (la fila de Ana: `local_library_id` = el id García) y `membership` | La biblioteca ya está enlazada y Ana es su única membresía |
| | `cloud_operation` de esa biblioteca: 13 filas (15 con las dos de abajo) | La historia entera: registro de dispositivos (cada uno el suyo) y la asignación de los móviles a Ana y a Hugo por el portátil de Ana, que administra; creación, tres miembros con sus roles, dos libros (obra, autor, editorial, edición, ejemplar) y el enlace. `body` es la operación tal como llegó, firma incluida |
| | `cloud_operation` con `body @> '{"changes":[{"entity":"mark"}]}'` y `…"change_request"…` | Una marca de Hugo y una solicitud suya: las firmó el móvil de Hugo y llegaron a la nube por Ana |
| | `cloud_device_key` de esa biblioteca: 3 filas | Las claves de portátil de Ana, móvil de Ana y móvil de Hugo, registradas al verse su primera operación |
| **Lucía no tiene cuenta** | `user`: nadie se llama Lucía; `device`: ningún dispositivo suyo | Un hijo no aparece en la tabla de acceso |
| | `cloud_operation` con `body @> '{"changes":[{"entity":"member","fields":{"name":"Lucía"}}]}'` y los `copy` con su `owner_id` | Sus ejemplares y su fila de miembro están en las operaciones, nada más |
| **El móvil de Ana añade un libro más tarde** | `cloud_operation`, autor = id del móvil de Ana | Una operación firmada con la clave que el móvil registró en la subida |
| **Kenji sube su biblioteca con el id por defecto** | `cloud_library`: la de Kenji con `local_library_id = 00000000-0000-7000-8000-000000000001`; 5 operaciones | Un id local fijo es un id local más |
| **Hugo no tiene acceso a la nube** | `membership` (5 filas, ninguna de Hugo) | Hasta 7.5 un lector no recibe la biblioteca por la nube; sigue por la red local |

## Costuras de D84 (cada una deja filas visibles)

| Costura | Dónde mirar | Qué demuestra |
|---|---|---|
| **2 · Mismo id local, dos bibliotecas** | `cloud_library` con `local_library_id = '00000000-0000-7000-8000-000000000001'`: **dos filas**, dueños Kenji y Jhonny. Y con el id García: otras dos (Ana y Jhonny) | Dos desconocidos con el id fijo no se funden: cada uno tiene su `C` y su historia (5 operaciones frente a 2) |
| **1 · Ids de operación robados** | `SELECT operation_id FROM cloud_operation GROUP BY operation_id HAVING COUNT(DISTINCT cloud_library_id) > 1`: **13 filas** | El mismo `operation_id` está en la biblioteca de Ana y en la de Jhonny (que las subió con los ids de ella): la clave es `(cloud_library_id, operation_id)`, así que Jhonny no pudo «ocupar» los ids de la familia ni estorbarla |
| **6 · Un dispositivo es su id junto con su clave** (SEG-016) | `device WHERE device_id = '<id del móvil de Ana>'`: **dos filas**, una de Ana y otra de Jhonny, con `public_key` distinta | Quien inicia sesión con el id público de otro dispositivo, pero con su propia clave, no se lo quita: tiene una fila aparte |
| **4 · Claves de firma dentro de cada `C`** | `cloud_device_key WHERE device_id = '<id del portátil de Ana>'`: **tres filas** (biblioteca de Ana, copia de Jhonny de la García, biblioteca por defecto de Jhonny) | En la de Ana la clave es la de Ana; en la biblioteca por defecto de Jhonny, el mismo id lleva la clave de Jhonny. Una clave registrada en su `C` no vale en otra |
| **7 · No rechazar por el orden de llegada** (DAT-001) | `cloud_operation` de Ana: la del portátil a `…+2900 min` y la del móvil a `…+3000 min`, la antigua con `received_at` posterior | Una operación más antigua que otra de **otro** autor, llegada después, se guarda. Con la cadena de D87 no puede llegar tarde una anterior del mismo autor: esperaría |

## Intentos que no dejan filas

Cada uno se ejecuta en la carga **y la carga se detiene si alguno entra**: que `make fixtures` termine sin error es la prueba de que las reglas aguantaron. No hay filas que mirar; lo que se comprueba es que **no hay** una operación más ni una clave más.

| Intento | Por qué no entra |
|---|---|
| Jhonny pide `pull` o `push` de la `C` de los García | No tiene membresía en ella: `404 not_found`, igual que si no existiera. El acceso sale de la tabla `membership`, nunca de un id que dice el cliente |
| Un `push` a una biblioteca sin enlace | Mientras `local_library_id` es `NULL` solo entra la operación de enlace firmada por un dispositivo de la cuenta dueña. **Queda la biblioteca** (`cloud_library` con `local_library_id NULL`, la tercera de Jhonny): crearla no es un `push`; pero sin ninguna operación |
| Una operación de otro id local | Firmada con la clave buena del portátil de Ana, pero `library` no es el `local_library_id` de esa `C` |
| Una operación sin firma | En la nube no hay operaciones anteriores a las firmas (D56) |
| Una firma falsa: cuerpo cambiado después de firmar | La firma no coincide con el cuerpo |
| Una firma falsa: la clave de Jhonny bajo el id del portátil de Ana | La `C` de los García tiene registrada la clave de Ana para ese id; otra clave se rechaza aunque la firma valga para sí misma |

## Conteo de filas tras cargar

| Tabla | Filas |
|---|---|
| `user` | 4 |
| `device` | 6 |
| `cloud_library` | 5 (Ana, Kenji, tres de Jhonny) |
| `membership` | 5 (una por biblioteca) |
| `cloud_operation` | 36 (García 15, Tanaka 5, Jhonny 14 + 2) |
| `cloud_device_key` | 8 |

## Abrir los dispositivos en la app

Las fixtures del servidor también sirven para ver **el otro lado de la historia**: abrir en la app de escritorio el dispositivo de cada personaje, ya con la sesión iniciada en el servidor de desarrollo.

```bash
cd server
make up              # si los contenedores no están arriba
make app-fixtures    # = make fixtures + el ejemplo de Rust, dentro de library-dev
```

El servidor es la única fuente del reparto: al final de `make fixtures` escribe `server/var/fixtures/cast.json` (ignorado por git; lleva el token y la semilla de firma de cada dispositivo, que son de cuentas de prueba) y lo anuncia en el resumen («Cast for the app»). `make app-fixtures` pasa ese fichero a `app/src-tauri/examples/fixture_devices.rs`, que para cada dispositivo:

1. crea una carpeta de datos que la app abre tal cual (la misma base `library.sqlite3` y las mismas migraciones);
2. abre la base con `Database::open_as`, que fija el id y la semilla de firma **antes de que se cree la primera biblioteca**, así todas sus operaciones, la creación de esa biblioteca incluida, llevan la misma identidad que en el servidor;
3. deja la sesión iniciada (servidor `http://localhost:8095`, token, usuario, correo);
4. ejecuta el `cloud::sync_all` de verdad y escribe lo que informó: la biblioteca que se descargó o el motivo por el que no.

Sin `make`, son los mismos dos pasos: `make fixtures` y, desde la raíz del repo, `distrobox enter library-dev -- bash -ic 'cargo run -p kosmotheka --example fixture_devices -- server/var/fixtures/cast.json /tmp/kosmotheka-fixtures'`. Las carpetas se rehacen desde cero cada vez (`make app-fixtures` borra `/tmp/kosmotheka-fixtures/` antes), y como cada carga cambia los id de nube, hay que repetir los dos pasos juntos.

| Carpeta (`/tmp/kosmotheka-fixtures/…`) | Dispositivo |
|---|---|
| `ana-laptop`, `ana-phone` | Ana |
| `hugo-phone` | Hugo |
| `kenji-laptop` | Kenji |
| `jhonny-phone` | Jhonny |
| `jhonny-as-ana-phone` | Jhonny con el id del móvil de Ana y su propia clave (SEG-016) |

Para abrir uno (con la app compilada en debug, por ejemplo tras `cd app && npm run tauri dev`; ver [fase-2.md](../testing/fase-2.md) para la carpeta de datos propia):

```bash
KOSMOTHEKA_DATA_DIR=/tmp/kosmotheka-fixtures/ana-laptop target/debug/kosmotheka
```

### Qué ves en cada uno y por qué

Esto es lo que informó el ejemplo en la última ejecución; las fixtures son deterministas salvo los id de nube.

| Dispositivo | `sync_all` | Qué ves |
|---|---|---|
| `ana-laptop`, `ana-phone` | `Downloaded`: recibe 15 operaciones y no rechaza ninguna | «Biblioteca García», enlazada a su biblioteca de nube (`0199a3b2-…`), además de la primera biblioteca vacía con la que nace cada dispositivo (id aleatorio, D85) |
| `hugo-phone` | sin bibliotecas en la nube | Sesión iniciada y solo su primera biblioteca vacía («Mi biblioteca», id aleatorio). Hasta el hito 7.5 solo la cuenta dueña recibe la biblioteca por la nube (no tiene membresía) |
| `kenji-laptop` | `Downloaded`: recibe 5 operaciones | Su biblioteca de nube **baja**, con el id local que le dio la fixture. Antes de D85 no bajaba: la app traía siempre «Mi biblioteca» con el id fijo, que era justo el de la de Kenji, y D84, con razón, no las mezclaba. Ahora su dispositivo nace con una biblioteca de id aleatorio y no choca con nada |
| `jhonny-phone`, `jhonny-as-ana-phone` | `NotLinked` para las dos bibliotecas de nube que ve: la copia que hizo de la García y la del id por defecto | Se baja, se comprueba que su registro no la enlaza a esa nube y se retira (`rejected 1` en cada una): no se queda nada de ellas |

**Las fixtures del servidor conservan a propósito el id fijo antiguo** (`00000000-0000-7000-8000-000000000001`): ya no es el id de «Mi biblioteca» de nadie (D85), pero sirve de «id que dos desconocidos comparten» para probar la puerta de la nube y las costuras 2 y 4 (D84). La app ya no puede producirlo; por eso `server/` no cambia con D85.

El enlace de las fixtures nombra el servidor `http://localhost:8095` (`SigningDevice::SERVER`): la app solo confía en un enlace que nombra el servidor con el que habla.

### Lo que la app rechaza (hallazgo, sin resolver)

En los dos dispositivos de Ana llegan las 15 operaciones y no se rechaza ninguna. La marca de Hugo se aplica (cada persona lleva las suyas, D68) y su petición queda como petición, porque Hugo lee. Una primera versión de las fixtures hacía que cada móvil dijera en su propio registro de quién era; eso es cosa de quien administra (D73), así que los dispositivos lo dejaban en petición y luego rechazaban lo que firmaban esos móviles. Se corrigió el 03/10/2026: el servidor lo guardaba porque hasta 7.4 no juzga permisos, y fue la app, al abrir los dispositivos, la que lo descubrió.

## Cuando cambian las reglas

Un caso nuevo de acceso va a `ScenarioFixtures` (un método, una línea de resumen) y a una fila de las tablas de arriba, además de su test en `CloudSyncControllerTest`. Si una regla cambia y un intento deja de rechazarse, `make fixtures` falla con `RuleBrokenException` diciendo cuál.
