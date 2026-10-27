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
  "previous": "0199a1a0-…",
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
| `previous` | Id de la última operación replicable del mismo dispositivo en la misma biblioteca cuando se hizo esta, o `null` si es la primera (D87). Obligatoria: la clave siempre está, aunque sea nula. Va firmada. Ver «Cadena de cada autor», más abajo |
| `changes` | Cambios: entidad, id, acción (`create`, `update`, `delete`) y campos |
| `signature` | Firma Ed25519 del dispositivo autor (D56). El formato no la exige, pero quien recibe una operación sin firma la rechaza siempre. Ver «Firmas», más abajo |

Qué campos acepta cada entidad está en `crates/domain/src/schema.rs` y en `spec/operation.schema.json`, que deben coincidir.

### Intenciones en uso

Sacadas del código de la app (`app/src-tauri/src`) y del servidor. La intención **solo informa** (historial y pantalla); no hay una lista cerrada: el formato exige palabras en minúscula separadas por puntos y el servidor acepta cualquiera bien formada.

**Catálogo**

| Intención | Qué cambia |
|---|---|
| `book.add` | Alta de una obra nueva: obra, personas, editorial, edición, detalles del tipo, géneros, etiquetas y ejemplar |
| `copy.add` | Alta con un ISBN ya catalogado: solo el ejemplar (y el propietario, si es nuevo) |
| `edition.edit` | Cambios en la ficha: solo los campos que cambian, más personas, géneros y etiquetas añadidos o quitados |
| `copy.edit` | Cambios en un ejemplar: solo los campos que cambian |
| `copy.trash` | Mandar un ejemplar a la papelera (D105). `copy.remove`, el borrado directo, es de antes de la papelera |
| `book.trash` | Mandar una edición a la papelera (D105): solo su fila; lo que cuelga de ella se queda y se esconde. `book.remove`, el borrado directo, es de antes de la papelera |
| `book.move_in` · `book.move_out` | Mover una obra a otra biblioteca (D46): la primera crea la obra en el destino, con ids nuevos; la segunda la manda a la papelera del origen (D105), donde se puede deshacer 30 días. Son dos operaciones, cada una en su biblioteca |
| `work.add_missing` | Apuntar una obra sin ejemplar («Me falta», D61) |
| `work.require` · `work.unrequire` | Apuntar o quitar que una obra «necesita» otra (una expansión y su juego base) |
| `mark.set` | Lo que una persona apunta de una obra: estado, valoración, favorito, «lo deseo» (D68). Solo viajan los campos que cambian |
| `tag.rename` · `tag.trash` | Renombrar una etiqueta (o unirla con otra que ya existe, y la vaciada va a la papelera) y mandarla a la papelera (antes, `tag.remove`) |
| `series.add` · `series.update` · `series.trash` | Crear, renombrar y mandar a la papelera una serie (antes, `series.remove`) |
| `series.entry_add` · `series.entry_remove` · `series.reorder` | Meter una obra en una serie, sacarla, y cambiar los números de varias a la vez |
| `backup.restore` | Restaurar una copia de seguridad: crea todas las filas del catálogo en una sola operación (D30), con lo que estaba en la papelera tal cual |
| `trash.restore` | Sacar de la papelera una o varias cosas: `trashed_at` y `trashed_by` a `null` (D105) |
| `trash.purge` | Eliminar definitivamente una o varias cosas de la papelera, con lo que cuelga de cada una, hijos primero: lo que hacía el borrado antes de la papelera |
| `trash.expire` | Lo mismo, hecho solo por un dispositivo a los 30 días de verlo en la papelera, o por la nube a los 60 (D105) |
| `sync.baseline` | Una vez por dispositivo: registra todo lo que ya había, sin tocar las filas, para que pueda viajar a otros dispositivos (D31) |

**Bibliotecas**

| Intención | Qué cambia |
|---|---|
| `library.found` | La fundación (D97): la fila `library` con su `founding_nonce`, su primera persona como `admin` y el `device` del autor asignado a ella |
| `library.rename` · `library.delete` | Renombrar y borrar una biblioteca (D65) |
| `library.duplicate` | Copiar el catálogo de una biblioteca en otra nueva, fundada por quien copia: el último recurso de D58 |
| `library.revoke` · `library.grant` | Retirar o devolver el acceso de un dispositivo a una biblioteca (D71) |
| `library.cloud` | Nombrar al dispositivo de la nube en la biblioteca, o quitarle su poder (`cloud_device_id`, D76, D95), y enlazar la biblioteca con su identidad en un servidor |
| `library.vouch` | Una sola vez, para las asignaciones anteriores a D96: un administrador escribe la `vouched_key` de los dispositivos que ya son de una persona y no la tienen |

**Personas y dispositivos**

| Intención | Qué cambia |
|---|---|
| `device.register` | La primera escritura de un dispositivo en una biblioteca: su fila `device` con la clave pública (D56). También la escribe el servidor para el dispositivo de la nube (D95) |
| `device.rename` | Un dispositivo publica su nombre nuevo en las bibliotecas donde ya está |
| `device.assign` | Decir de quién es un dispositivo, con su clave avalada y, si cambia de persona, su valla (D96, D84) |
| `device.give_access` | Al emparejar: una persona entra en las bibliotecas elegidas, con el rol dado si no tenía ninguno, y el dispositivo nuevo pasa a ser suyo (D73) |
| `device.remove` | Dispositivo perdido: deja de ser de su persona y se le retira el acceso, en cada biblioteca donde escribe (D58) |
| `member.role` | Cambiar el rol o el permiso de compartir de una persona; si baja poder, lleva la valla de sus dispositivos |
| `member.rename` | Cambiar el nombre de una persona |
| `member.invite` | Crear a la persona que va a entrar con una invitación: siempre lector y sin poder compartir (D43) |
| `member.withdraw` · `member.restore` | Echar a una persona de la biblioteca y devolverle el acceso (D98) |

**Peticiones**

| Intención | Qué cambia |
|---|---|
| `request.accept` · `request.reject` | Resolver una petición: crea su fila `change_request` y, al aceptar, repite los cambios pedidos (D73). Al aceptar, la operación lleva la intención `request.accept` aunque el cambio fuera un alta |
| `request.withdraw` | Quien pidió la retira mientras está pendiente |

**Las que escribe el servidor** como dispositivo de la nube, con su propia clave: `device.register` (su identidad en la biblioteca), `member.link_account` y `member.unlink_account` (enlazar y desenlazar la cuenta de una persona) y `device.link_account` (vincular a una persona un dispositivo en el que su cuenta ha entrado).

### Ya no se usan

Quedan en migraciones, en pruebas o en bibliotecas antiguas, pero la app ya no las escribe:

| Intención | La sustituyó |
|---|---|
| `wish.add` · `wish.remove` | La lista de deseos era una entidad propia, `wish` (D36). Se sustituyó por las obras sin ejemplar, `work.add_missing`, y por el campo «lo deseo» de las marcas, `mark.set` (D61, D68). La migración 25 borró la tabla sin convertir lo que había. Una operación así en el registro no impide abrir la base de datos |
| `book.edit` | `edition.edit` |
| `member.add`, `member.roles`, `member.account`, `member.remove`, `person.rename`, `person.claim`, `publisher.add`, `publisher.remove`, `work.remove`, `work.genre_add`, `work.genre_remove`, `edition.add`, `tag.add`, `device.swap` | Solo aparecen en pruebas. Las personas, editoriales, etiquetas y géneros nuevos nacen dentro de la operación que los usa (`book.add`, `edition.edit`, `member.role`…) |

Una operación vieja con una de estas intenciones se sigue leyendo y aplicando: la intención nunca decide nada.

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
- **La clave pública viaja con la biblioteca** en una fila `device` (su id es el del dispositivo). La crea el propio dispositivo en una operación suya (`device.register`) antes de su primera escritura en esa biblioteca, firmada con esa misma clave. Así una operación que llega a través de otro dispositivo se puede comprobar igual.
- **Al recibir** (`sync::merge`), se rechaza y se cuenta como rechazada:
  - una operación con firma que no cuadra con la clave de su autor, o de un autor sin clave conocida;
  - una que intenta cambiar la clave de un dispositivo que ya tiene una: la primera es la que vale.
- **Una operación sin firma se rechaza siempre** (D56, ampliada el 03/10/2026), con roles o sin ellos, en los dispositivos y en la nube. Las de antes de esta fase eran de prueba: ya no viajan.

## Cadena de cada autor (D87)

Cada dispositivo encadena sus operaciones por biblioteca: `previous` apunta a la anterior suya, y la firma cubre el campo. Quien recibe una operación cuya anterior no tiene la deja esperando y no se salta nunca; una que apunta a una operación de otro autor, de otra biblioteca o posterior en el reloj está mal formada. Con la cadena, las vallas de D84 señalan una operación (`trusted_through`) y no una fecha. El detalle, en [sync.md](sync.md). Las operaciones registradas antes de D87 no tienen `previous` y no son válidas: una biblioteca de prueba anterior se recupera con «Duplicar como nueva» (D58).

## Permisos (D73, hitos 5.2 a 5.4)

Las reglas están en `spec/README.md` («Roles and permissions») y en D73. Resumen:
- **Toda biblioteca nace con administrador (D97):** su primera operación es la fundación, y solo vale si su id sale de la clave de quien la firma. Hasta entonces solo se admite que cada dispositivo se registre.
- Cada operación se juzga con el rol de su autor en ese momento: el autor es el dispositivo del `hlc`, su persona es el `member_id` de su fila `device`, y un dispositivo que no es de nadie cuenta como lector (una operación sin firma ni siquiera llega aquí: se rechaza antes).
- **Todo o nada:** si un cambio no está permitido, la operación entera queda como **petición** (`operations.status = 'request'`): se guarda en el registro y viaja, pero no se aplica.

Dónde está:
- **Las reglas**, como función pura, en `crates/domain/src/permission.rs` (`is_allowed`), para que el servidor pueda usar las mismas.
- **Las preguntas** que necesitan («¿está fundada la biblioteca?», «¿qué rol tiene este dispositivo?», «¿creó esta fila su misma persona?»), en `app/src-tauri/src/permissions.rs`, leyendo la base de datos.
- **Escrituras propias:** `ops::record` juzga la operación igual que si llegara de fuera. Si no está permitida, devuelve `NotAllowed` y no escribe nada, salvo en **modo petición** (`ops::as_request`): entonces la guarda como petición, sin aplicarla. Es lo que usará el botón «Pedir que se añada».
- **Peticiones** (`app/src-tauri/src/requests.rs`, hito 5.5): `list_requests` las lista con su autor, persona, fecha, cambios y estado (pendiente, aceptada, rechazada o retirada). `accept` graba una operación del administrador que crea la fila `change_request` y repite los cambios pedidos; si ya no se pueden aplicar (una fila que editaban se borró), falla sin cambiar nada. `reject` solo crea la fila, y `withdraw` es la de quien pidió, mientras sigue pendiente. Resolver una ya resuelta da `RequestResolved`.
- **El registro del dispositivo** (su fila `device` con la clave) va en una operación propia, `device.register`, antes de la primera escritura en cada biblioteca. Así «todo o nada» nunca se lo lleva por delante.

## Validación, en dos niveles

1. **Estructural** (`Operation::validate`, en el dominio): ids con formato UUID, etiqueta bien formada, campos que la entidad conoce, tipos correctos, obligatorios presentes al crear y nunca nulos. Cualquier implementación la replica, y los **casos de conformidad** (`spec/conformance/`) lo comprueban.
2. **Al aplicar** (en la base de datos): que las filas referenciadas existan (claves foráneas), y que una modificación o un borrado encuentre su fila.

Si falla cualquiera de los dos, la transacción entera se deshace.

## Pendiente

- **Fusionar duplicados a mano (D33):** lo que se crea a la vez en dos dispositivos sin conexión (un mismo autor, una misma editorial, un mismo ISBN) se conserva duplicado al sincronizar. La opción de juntarlos, que quedaría registrada como una operación más, no existe todavía.

## Ya resuelto (antes figuraba aquí)

- ~~**Fusión campo a campo:**~~ hecha. Cada dispositivo guarda, por campo, la marca que lo escribió por última vez (`field_versions`). Ver [sync.md](sync.md).
- ~~**Permisos:**~~ hechos. Las firmas dicen quién hizo cada operación y los roles qué puede hacer; lo que no puede queda como petición.
- ~~**Versionar el formato de las operaciones:**~~ resuelto en D31: cada operación guarda su versión de esquema, las anteriores a la sincronización quedan como historial no replicable y cada dispositivo registra una operación inicial `sync.baseline`. Ver [sync.md](sync.md).
- **Índice de búsqueda:** `ops::record` también reindexa las ediciones afectadas (D29), así que cualquier operación, venga de donde venga, deja la búsqueda al día.
- Los libros dados de alta antes de que existieran las operaciones no tienen historial propio: su primer rastro es la operación `sync.baseline`.
