# Operaciones

> Cómo se escribe en el catálogo. Decisiones de fondo: D05 (local-first con servidor autoritativo), D13 (sincronización local) y D25 (forma de una operación). La especificación formal, en inglés y con licencia MIT, está en [`spec/`](../../spec/).

## Idea

Nada escribe directamente en las tablas del catálogo. Cada acción del usuario («añadir libro») se convierte en una **operación**: un lote atómico de cambios de entidad con una etiqueta que dice qué quería hacer el usuario. La operación se guarda en un registro (`operations`) y sus cambios se aplican **en la misma transacción**: o se hace todo, o nada.

¿Para qué tanto rodeo, si hoy todo es local?
- **Sincronizar** (Fase 2) es intercambiar operaciones: «dame las que no tengo».
- **El servidor** (Fase 3) recibe operaciones, comprueba los permisos sobre sus cambios y las vuelve a aplicar.
- **El historial** de un libro sale gratis del registro.

## Forma de una operación

```json
{
  "id": "0199a1b2-…",
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
| `hlc` | Marca del reloj lógico híbrido (ver abajo). Incluye el identificador del dispositivo |
| `intent` | Etiqueta de intención: palabras en minúscula separadas por puntos. **Solo informativa**: los permisos y la fusión miran los cambios, nunca la etiqueta |
| `changes` | Cambios: entidad, id, acción (`create`, `update`, `delete`) y campos |

Qué campos acepta cada entidad está en `crates/domain/src/schema.rs` y en `spec/operation.schema.json`, que deben coincidir.

### Intenciones en uso

| Intención | Cuándo |
|---|---|
| `book.add` | Alta de un libro nuevo: obra, autores, edición, detalles y ejemplar |
| `copy.add` | Alta con un ISBN ya catalogado: solo el ejemplar (y el propietario, si es nuevo) |

## Reloj lógico híbrido (HLC)

Cada operación lleva una marca `milisegundos-contador-dispositivo`, por ejemplo `0001727222222222-00000003-<uuid>`.
- **Milisegundos:** la hora del reloj del dispositivo.
- **Contador:** desempata cuando dos operaciones caen en el mismo milisegundo, o cuando el reloj del sistema retrocede (un cambio de hora, un reloj mal puesto). Así las marcas **nunca retroceden** en un dispositivo.
- **Dispositivo:** desempata entre dispositivos distintos.

Al recibir operaciones de otro dispositivo (Fase 2), el reloj salta por delante de la marca recibida (`HlcClock::receive`). Así lo que hagas después queda siempre por detrás de lo que ya has visto, aunque el otro reloj vaya adelantado.

El texto tiene ceros a la izquierda para que ordenar como texto sea igual que ordenar en el tiempo: la base de datos ordena la columna `hlc` directamente.

## Validación, en dos niveles

1. **Estructural** (`Operation::validate`, en el dominio): ids con formato UUID, etiqueta bien formada, campos que la entidad conoce, tipos correctos, obligatorios presentes al crear y nunca nulos. Cualquier implementación la replica, y los **casos de conformidad** (`spec/conformance/`) lo comprueban.
2. **Al aplicar** (en la base de datos): que las filas referenciadas existan (claves foráneas), que el ISBN no esté repetido, y que una modificación o un borrado encuentre su fila.

Si falla cualquiera de los dos, la transacción entera se deshace.

## Pendiente

- **Fusión campo a campo (Fase 2):** para decidir qué valor gana cuando dos dispositivos cambian el mismo campo, hará falta saber qué HLC escribió por última vez cada campo. Se añadirá con la sincronización; el registro de operaciones ya tiene todo lo necesario para reconstruirlo.
- **Permisos (Fase 3):** reglas por entidad y acción («un lector no puede borrar ejemplares»).
- Los libros dados de alta antes del hito 1.2 no tienen operaciones, y por tanto tampoco historial.
