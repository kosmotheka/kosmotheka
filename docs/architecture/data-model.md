# Modelo de datos

> Esquema de la base de datos local (SQLite) de la app. Se amplía en cada hito; aquí se documenta el estado actual. Decisiones de fondo: D07, D08 y D24.

## Principios

- **Identificadores UUID v7** generados en la app, guardados como texto. No hay IDs autoincrementales: cuando haya sincronización (D05, D13), dos dispositivos pueden crear registros sin chocar. Los UUID v7 empiezan por la fecha, así que se ordenan por momento de creación.
- **Tablas `STRICT`**: SQLite comprueba los tipos de cada columna (por defecto no lo hace).
- **Fechas** en texto ISO 8601: `2026-09-24` para días y `2026-09-24T21:30:00.000Z` para instantes.
- **ISBN normalizado a ISBN-13** y sin guiones. Un ISBN-10 se convierte al darlo de alta; el ISBN-10 se puede recalcular cuando haga falta.
- **Herencia por tablas** (D08): `editions` guarda lo común a cualquier catálogo y `book_details` lo específico de los libros.

## Esquema (hitos 1.1 y 1.2)

```
works ──< work_contributors >── people
  │
  └──< editions ──1:1── book_details
          │
          └──< copies >── members
```

| Tabla | Qué es | Columnas principales |
|---|---|---|
| `works` | Obra: el libro en abstracto | `id`, `kind` (de momento solo `book`), `title` |
| `people` | Autores y otros participantes | `id`, `name` |
| `work_contributors` | Quién participó en una obra y en qué orden | `id`, `work_id`, `person_id`, `role` (`author`), `position` |
| `editions` | Edición: una publicación concreta | `id`, `work_id`, `title`, `subtitle`, `publisher_id`, `published_year`, `language`, `cover` (fichero, D27) |
| `publishers` | Editoriales (D29) | `id`, `name` (no único desde D33; la app reutiliza el existente) |
| `book_details` | Datos propios de un libro | `edition_id`, `isbn13` (no único desde D33), `google_books_id`, `open_library_id` |
| `members` | Miembros de la casa (propietarios). En modo local son solo nombres, no cuentas | `id`, `name` (no único desde D33; la app reutiliza el existente) |
| `copies` | Ejemplar: lo que hay en casa | `id`, `edition_id`, `owner_id`, `format`, `quantity`, `location`, `acquired_on` |
| `operations` | Registro de todas las escrituras, propias y recibidas ([operations.md](operations.md)) | `id`, `hlc`, `device_id`, `intent`, `changes` (JSON), `schema_version`, `replicable` |
| `field_versions` | Última marca HLC que escribió cada campo de cada fila ([sync.md](sync.md)) | `entity`, `entity_id`, `field`, `hlc` |
| `tombstones` | Filas borradas: el borrado gana ([sync.md](sync.md)) | `entity`, `entity_id`, `hlc` |
| `search_index` | Índice de texto completo FTS5, mantenido por `ops::record` (D29) | `edition_id`, `content` |
| `local_state` | Valores propios de este dispositivo: su identificador, la última marca HLC emitida y los ajustes (clave de Google Books) | `key`, `value` |

**Formatos** (`copies.format`): `hardcover` (tapa dura), `paperback` (tapa blanda), `pocket` (bolsillo), `ebook`, `audiobook`, `other`. El formato va en el ejemplar, como pide VISION.md: lo que interesa saber es en qué formato lo tienes.

## Reglas del alta

- Toda escritura pasa por una operación ([operations.md](operations.md)).
- Si el ISBN **ya existe**, no se duplica la edición: se añade un ejemplar nuevo a la edición existente y la app lo avisa.
- Si no hay ISBN o es nuevo, se crean a la vez la obra, la edición y el ejemplar.
- Autores, editoriales y propietarios se buscan por nombre (sin distinguir mayúsculas) y se crean si no existen.

## Pendiente en hitos posteriores

- Ubicaciones como jerarquía (salón → estantería → balda) en lugar de texto libre (1.5).
- Saga y número, temáticas, etiquetas y campos de rol (sin hito asignado todavía).

## Ficheros fuera de la base de datos

- `covers/`: portadas, con nombre `<sha256>.<ext>` (D27). Una portada que ya no usa ningún libro se queda en disco; limpiarlas es una tarea pendiente.
