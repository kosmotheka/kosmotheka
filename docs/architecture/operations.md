# Operaciones

> Cómo se escribe en el catálogo. Decisiones de fondo: D05 (local-first con servidor autoritativo), D13 (sincronización local) y D25 (forma de una operación). La especificación formal, en inglés y con licencia MIT, está en [`spec/`](../../spec/).

## Idea

Nada escribe directamente en las tablas del catálogo. Cada acción del usuario («añadir libro») se convierte en una **operación**: un lote atómico de cambios de entidad con una etiqueta que dice qué quería hacer el usuario. La operación se guarda en un registro (`operations`) y sus cambios se aplican **en la misma transacción**: o se hace todo, o nada.

¿Para qué tanto rodeo, si hoy todo es local?
- **Sincronizar** (Fase 2) es intercambiar operaciones: «dame las que no tengo».
- **El servidor** (Fase 7, la nube) recibe operaciones, comprueba los permisos sobre sus cambios y las vuelve a aplicar.
- **El historial** de un libro sale gratis del registro.

## Forma de una operación

```json
{
  "id": "0199a1b2-…",
  "library": "0199a0aa-…",
  "hlc": "0001727222222222-00000000-0199a0ff-…",
  "intent": "book.add",
  "changes": [
    { "entity": "work",         "id": "0199…11", "action": "create", "fields": { "kind": "book", "title": "El nombre del viento" } },
    { "entity": "edition",      "id": "0199…12", "action": "create", "fields": { "work_id": "0199…11", "title": "El nombre del viento" } },
    { "entity": "book_details", "id": "0199…12", "action": "create", "fields": { "isbn13": "9788401352188" } },
    { "entity": "copy",         "id": "0199…13", "action": "create", "fields": { "edition_id": "0199…12", "format": "hardcover", "quantity": 1 } }
  ]
}
```

| Campo | Qué es |
|---|---|
| `id` | UUID v7 de la operación |
| `library` | La biblioteca a la que pertenecen todos sus cambios (D54). Un cambio no puede elegir otra; solo puede tocar la entidad `library` si es la suya |
| `hlc` | Marca del reloj lógico híbrido (ver abajo). Incluye el identificador del dispositivo |
| `intent` | Etiqueta de intención: palabras en minúscula separadas por puntos. **Solo informativa**: los permisos y la fusión miran los cambios, nunca la etiqueta |
| `changes` | Cambios: entidad, id, acción (`create`, `update`, `delete`) y campos |
| `signature` | Firma Ed25519 del dispositivo autor (D56). Las operaciones anteriores a la Fase 5 no la llevan. Ver «Firmas», más abajo |

Qué campos acepta cada entidad está en `crates/domain/src/schema.rs` y en `spec/operation.schema.json`, que deben coincidir.

### Intenciones en uso

| Intención | Cuándo |
|---|---|
| `book.add` | Alta de un libro nuevo: obra, autores, edición, detalles y ejemplar |
| `copy.add` | Alta con un ISBN ya catalogado: solo el ejemplar (y el propietario, si es nuevo) |
| `edition.edit` | Cambios en la ficha: solo los campos que cambian, más autores añadidos, movidos o quitados |
| `copy.edit` | Cambios en un ejemplar: solo los campos que cambian |
| `copy.remove` | Borrado de un ejemplar |
| `book.remove` | Borrado de un libro: ejemplares, detalles, edición y, si no quedan más ediciones, la obra y sus autorías |
| `backup.restore` | Restauración de una copia de seguridad: crea todas las filas del catálogo en una sola operación (D30) |
| `sync.baseline` | Una vez por dispositivo: registra todo lo que ya había, sin tocar las filas, para que pueda viajar a otros dispositivos (D31) |
| `wish.add` / `wish.remove` | Apuntar o quitar un deseo (D36) |
| `library.create` / `library.rename` / `library.delete` | Crear, renombrar o borrar una biblioteca (D65) |
| `library.revoke` / `library.grant` | Retirar o devolver el acceso de un dispositivo a una biblioteca (D71) |
| `device.register` | La primera escritura de un dispositivo en una biblioteca: su fila `device` con la clave pública (D56) |
| `member.roles` y parecidas | Asignar roles o decir de quién es un dispositivo (D73) |
| `request.accept` / `request.reject` | Resolver una petición: crea su fila `change_request` y, al aceptar, repite los cambios pedidos (D73) |


## Reloj lógico híbrido (HLC)

Cada operación lleva una marca `milisegundos-contador-dispositivo`, por ejemplo `0001727222222222-00000003-<uuid>`.
- **Milisegundos:** la hora del reloj del dispositivo.
- **Contador:** desempata cuando dos operaciones caen en el mismo milisegundo, o cuando el reloj del sistema retrocede (un cambio de hora, un reloj mal puesto). Así las marcas **nunca retroceden** en un dispositivo.
- **Dispositivo:** desempata entre dispositivos distintos.

Al recibir operaciones de otro dispositivo (Fase 2), el reloj salta por delante de la marca recibida (`HlcClock::receive`). Así lo que hagas después queda siempre por detrás de lo que ya has visto, aunque el otro reloj vaya adelantado.

El texto tiene ceros a la izquierda para que ordenar como texto sea igual que ordenar en el tiempo: la base de datos ordena la columna `hlc` directamente.

## Firmas (D56, hito 5.1)

- **Cada dispositivo tiene su clave Ed25519**, creada la primera vez que hace falta y guardada en su base de datos (`local_state`, clave `signing_seed`). No va en las copias de seguridad (D58).
- **Qué se firma:** la operación sin su firma, en JSON canónico (RFC 8785: claves ordenadas, sin espacios). Es lo que Rust (`crates/domain/src/signature.rs`) y PHP (`server/src/Sync/OperationSignature.php`) comprueban con los mismos casos, `spec/conformance/signatures.json`.
- **La clave pública viaja con la biblioteca** en una fila `device` (su id es el del dispositivo). La crea el propio dispositivo en su primera operación de esa biblioteca, firmada con esa misma clave. Así una operación que llega a través de otro dispositivo se puede comprobar igual.
- **Al recibir** (`sync::merge`), se rechaza y se cuenta como rechazada:
  - una operación con firma que no cuadra con la clave de su autor, o de un autor sin clave conocida;
  - una que intenta cambiar la clave de un dispositivo que ya tiene una: la primera es la que vale.
- **Las operaciones sin firma** (anteriores a esta fase) se siguen aceptando mientras no haya roles. Qué hacer con ellas después está pendiente (pregunta 1 de `docs/plans/fase-5.md`).

## Permisos (D73, hitos 5.2 a 5.4)

Las reglas están en `spec/README.md` («Roles and permissions») y en D73. Resumen:
- **Sin roles, como siempre:** hasta que algún miembro tiene rol, todos pueden todo.
- Con roles, cada operación se juzga con el rol de su autor en ese momento: el autor es el dispositivo del `hlc`, su persona es el `member_id` de su fila `device`, y un dispositivo que no es de nadie, o una operación sin firma, cuenta como lector.
- **Todo o nada:** si un cambio no está permitido, la operación entera queda como **petición** (`operations.status = 'request'`): se guarda en el registro y viaja, pero no se aplica.

Dónde está:
- **Las reglas**, como función pura, en `crates/domain/src/permission.rs` (`is_allowed`), para que el servidor pueda usar las mismas.
- **Las preguntas** que necesitan («¿tiene roles la biblioteca?», «¿qué rol tiene este dispositivo?», «¿creó esta fila su misma persona?»), en `app/src-tauri/src/permissions.rs`, leyendo la base de datos.
- **Escrituras propias:** `ops::record` juzga la operación igual que si llegara de fuera. Si no está permitida, devuelve `NotAllowed` y no escribe nada, salvo en **modo petición** (`ops::as_request`): entonces la guarda como petición, sin aplicarla. Es lo que usará el botón «Pedir que se añada».
- **Peticiones** (`app/src-tauri/src/requests.rs`, hito 5.5): `list_requests` las lista con su autor, persona, fecha, cambios y estado (pendiente, aceptada o rechazada). `accept` graba una operación del administrador que crea la fila `change_request` y repite los cambios pedidos; si ya no se pueden aplicar (una fila que editaban se borró), falla sin cambiar nada. `reject` solo crea la fila. Resolver una ya resuelta da `RequestResolved`.
- **El registro del dispositivo** (su fila `device` con la clave) va en una operación propia, `device.register`, antes de la primera escritura en cada biblioteca. Así «todo o nada» nunca se lo lleva por delante.

## Validación, en dos niveles

1. **Estructural** (`Operation::validate`, en el dominio): ids con formato UUID, etiqueta bien formada, campos que la entidad conoce, tipos correctos, obligatorios presentes al crear y nunca nulos. Cualquier implementación la replica, y los **casos de conformidad** (`spec/conformance/`) lo comprueban.
2. **Al aplicar** (en la base de datos): que las filas referenciadas existan (claves foráneas), que el ISBN no esté repetido, y que una modificación o un borrado encuentre su fila.

Si falla cualquiera de los dos, la transacción entera se deshace.

## Pendiente

- **Fusión campo a campo (Fase 2):** para decidir qué valor gana cuando dos dispositivos cambian el mismo campo, hará falta saber qué HLC escribió por última vez cada campo. Se añadirá con la sincronización; el registro de operaciones ya tiene todo lo necesario para reconstruirlo.
- **Permisos (Fase 5):** reglas por entidad y acción («un lector no puede borrar ejemplares»). Las firmas (5.1) ya dicen quién hizo cada operación; los roles (5.3) dirán qué puede hacer.
- Los libros dados de alta antes del hito 1.2 no tienen operaciones, y por tanto tampoco historial.
- ~~Versionar el formato de las operaciones~~ Resuelto en D31: cada operación guarda su versión de esquema, las anteriores a la sincronización quedan como historial no replicable y cada dispositivo registra una operación inicial `sync.baseline`. Ver [sync.md](sync.md).
- **Índice de búsqueda:** `ops::record` también reindexa las ediciones afectadas (D29), así que cualquier operación, venga de donde venga, deja la búsqueda al día.
