# Encargo: la identidad de la nube (hito 7.5.2a)

> Encargo cerrado para la sesión `library-sonnet`; lo prepara y lo revisa la sesión principal. Al terminar pasa a `docs/historico/`.

## De dónde se parte

**Del árbol de trabajo tal como está**: el commit `198a5fb` más 7.5.1, que está revisado y **sin commit** (el mantenedor decide los commits al volver). No deshagas ni reordenes nada de 7.5.1.

Lee antes [D76](../DECISIONS.md) y [D95](../DECISIONS.md) enteras (incluida la corrección del 10/10/2026), y en [`fase-7.md`](fase-7.md) «7.5.2 se parte en dos».

**Qué se construye.** Cada biblioteca de la nube tiene un dispositivo propio, «la nube», con su id y su par de claves. La app lo nombra al subir. El servidor aprende a firmar operaciones con esa clave, pero **todavía nadie se lo pide**.

**Qué no se construye.** El enlace automático entre cuenta y persona, `membership.member_id`, escribir `account_id`, vincular dispositivos, pantallas. Nada de eso: su diseño está abierto (7.5.2b).

## Reglas del encargo

- **Sin commit.** Ni `git add`, ni `git commit`, ni `git stash`, ni cambio de rama.
- Código, comentarios y nombres en **inglés**; `docs/` en **español**. Glosario de `AGENTS.md`.
- Rust en el contenedor: `distrobox enter library-dev -- bash -ic '<comando>'`. Servidor con `server/Makefile`. Nada fuera de los contenedores.
- Prohibido tocar `docs/DECISIONS.md`, `docs/audits/` y `spec/` (esta parte no cambia el protocolo; si crees que hace falta, para).
- **Ningún secreto de verdad en el repositorio.** Los valores de desarrollo y de test, como se hace hoy con las claves de Mercure.
- **El servidor no se fía de ningún id del cliente (D84).**
- **Tests hostiles primero**, vistos en rojo; el informe copia la línea del fallo.
- **Si el trabajo crece, algo de aquí choca con el código o con `spec/`, o un comando es denegado, se para y se cuenta.**
- Explicaciones de Rust: en el informe, no en comentarios.
- SQL nuevo: solo en los repositorios que ya escriben SQL o en la migración, con parámetros enlazados.

## 1. Servidor: la identidad

- **Secreto del servidor:** una variable de entorno nueva, `CLOUD_KEY_SECRET` (32 bytes en base64). Vacía en `.env` con su comentario, con valor de test en `.env.test`, y documentada en `docs/development/server.md` (cómo generarla, que producción la define, y que **perderla deja a la nube sin poder firmar** en las bibliotecas ya creadas).
- **La nube es opcional también aquí (D18):** con el secreto vacío el servidor funciona como hoy y las bibliotecas no tienen identidad de nube: los campos nuevos quedan nulos y la API los devuelve nulos. Sin excepción ni `500`.
- **`cloud_library`** gana tres columnas anulables: `cloud_device_id` (UUID que genera el servidor), `cloud_public_key` (como las demás claves: 32 bytes, base64url sin relleno) y `cloud_secret_key` (la clave privada **cifrada**). Migración con `make migration`.
- **Cifrado:** una clase propia en `App\Security` (por ejemplo `CloudKeyVault`), con `sodium_crypto_secretbox`, un *nonce* aleatorio por valor guardado junto al texto cifrado. Descifrar con otro secreto falla con una excepción propia, no con datos basura.
- **Cuándo se genera:** al crear la biblioteca de la nube (`CloudLibraryCreator`). Para las que ya existen sin identidad, al listarlas o al enlazarlas, dentro de una transacción con el bloqueo de la biblioteca. Una vez generada no cambia nunca.
- **Su clave queda registrada desde el principio:** al generar la identidad se guarda también la fila `CloudDeviceKey` de ese id con esa clave en esa biblioteca. Así la regla que ya existe («una clave distinta para un dispositivo registrado se rechaza») impide que nadie ocupe el id de la nube o firme en su nombre.
- **API:** `POST /v1/cloud/libraries` y `GET /v1/cloud/libraries` devuelven, por biblioteca, `cloudDeviceId` y `cloudPublicKey` (o nulos). La clave privada no sale nunca, ni cifrada.

## 2. Servidor: firmar

- Una clase (por ejemplo `App\Sync\CloudAuthor`) con un método que, dada una biblioteca de la nube **enlazada**, una intención y una lista de cambios, construye la operación como lo hace la app, la firma con la clave de la nube y la guarda **por el mismo camino que un `push`**: lectura con `OperationReader`, firma comprobada, cadena (`previous` es su operación anterior en esa biblioteca), HLC único, y entregada al juez (`take` y `settle`). Sin atajos: si mañana una regla del `push` cambia, vale también para lo que firma la nube. Reutiliza el código de `CloudPush`; no lo dupliques.
- **HLC del servidor:** la hora del servidor en milisegundos, nunca menor que la de su operación anterior en esa biblioteca (si el reloj retrocede, sube el contador), con el formato que usa la app.
- **Las bytes que se firman** son las de `OperationSignature::canonical`. El código de firmar que hoy vive en `App\DataFixtures\SigningDevice` no puede usarse desde producción: saca lo común a un sitio de producción y que las fixtures lo usen.
- Exige el bloqueo de la biblioteca y una transacción abierta, como el `push`; `pushed_by_*` queda vacío.
- **Nadie la llama todavía** en producción. Dilo en su comentario de clase (qué la usará: 7.5.2b y 7.5.3).
- Sin identidad (secreto vacío o biblioteca sin enlazar): una excepción propia, clara.

## 3. App: nombrar a la nube al subir

- `CloudLibrary` (lo que devuelve la API) gana `cloud_device_id` y `cloud_public_key`, opcionales.
- En `upload`, si la biblioteca de la nube trae identidad, **la operación de enlace** lleva además, en el mismo cambio de la fila `library`, `cloud_device_id`, y un segundo cambio que crea la fila `device` de la nube con su `public_key` y, como nombre, el servidor (el host de la URL). Es la casilla de D76, marcada por defecto; la pantalla para quitarla es de 7.5.5, y la función que la quita ya existe (`people::remove_cloud_device`).
- `people::name_cloud_device` pasa a recibir también la clave y crea la fila del dispositivo si no existe, para que nombrar a la nube deje siempre su clave anclada. Comprueba que `remove_cloud_device` pone la valla también en ese caso.
- Comprueba que nada de lo que reconoce el enlace se rompe por llevar un campo y un cambio más: `links_to`, `adopt_link`, `link_of`, `has_unsigned_history` en Rust; `CloudLink::isLink` y `check` en PHP. Si alguno exige la forma exacta de hoy, **para y cuéntalo**.
- Sin identidad en la respuesta (servidor sin secreto, o anterior a esto), `upload` hace exactamente lo de hoy.
- `FakeCloud` en `cloud/tests.rs` da identidad a sus bibliotecas (con una opción para no darla).

## 4. Tests

**Hostiles, vistos en rojo**, en `CloudSyncControllerTest`:

1. Un miembro con acceso (Jhonny, con `shareWithJhonny`) sube una operación que registra el id de la nube con una clave suya: rechazada; la clave de la nube en esa biblioteca no cambia.
2. El mismo sube una operación con el id de la nube en el HLC, firmada con su clave: rechazada.
3. El que sube nombra a la nube con una clave distinta de la suya de verdad: rechazada (`key_changed`), y la biblioteca no queda enlazada con esa operación.
4. La respuesta de crear y de listar no contiene la clave privada, ni cifrada; y lo guardado en la base no es la clave en claro.
5. Jhonny, sin acceso, no ve el id de la nube de la biblioteca de la familia (el `404` de siempre).

**Los demás:**

- El almacén: cifrar y descifrar da lo mismo; con otro secreto falla; dos cifrados del mismo valor son distintos.
- Con el secreto vacío: crear, listar, enlazar y subir funcionan como hoy y los campos son nulos.
- `CloudAuthor`: con la nube nombrada por una administradora, escribe `account_id` en una persona; la operación queda guardada y aplicada, su firma se verifica con la clave pública de la biblioteca, encadena con la anterior de la nube en la segunda llamada, y `pull` la reparte. Con la nube sin nombrar, la misma operación queda en petición (lo decide el juez, no esta clase). Con el reloj parado, dos operaciones seguidas tienen HLC distintos y en orden.
- Una operación firmada por `CloudAuthor` la acepta la app: un test en Rust que fusione una operación generada por el servidor. Si no hay ya una forma de pasar fixtures del servidor a los tests de Rust, usa `spec/conformance/signatures.json` o dime cómo lo ves; no inventes un canal nuevo sin contarlo.
- Rust (`cloud/tests.rs`): subir nombra a la nube y deja su fila de dispositivo con su clave; un segundo dispositivo que baja la biblioteca tiene lo mismo y converge; sin identidad, subir hace lo de antes; quitar a la nube tras subir pone la valla.
- Fixtures (`ScenarioFixtures`): las bibliotecas subidas nombran a su nube, como hará la app.

## 5. Documentación

- `docs/architecture/cloud.md`: la identidad de la nube (tablas, rutas, la regla de la clave registrada desde el principio) y `CloudAuthor`.
- `docs/development/server.md`: `CLOUD_KEY_SECRET`.
- `docs/architecture/sync.md` o `accounts.md`, donde encaje: qué hace `upload` ahora.
- `docs/plans/fase-7.md`: una línea de 7.5.2a en «Estado», **sin marcar**; la batería de la nube con los tests nuevos.

## Comprobaciones

`cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cd app && npm run build && npm run typecheck && npm run check:spec`, y en `server/` `make check`. Todo en verde.

## Informe

Qué se hizo punto por punto; la línea del fallo de cada test visto en rojo; cómo se comparte el camino del `push` con `CloudAuthor`; salidas de las comprobaciones; todo lo que no coincidía con este encargo y dónde se paró.
