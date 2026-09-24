# Modelo de datos

> Esquema de la base de datos local (SQLite) de la app. Se amplía en cada hito; aquí se documenta el estado actual. Decisiones de fondo: D07, D08 y D24.

## Principios

- **Identificadores UUID v7** generados en la app, guardados como texto. No hay IDs autoincrementales: cuando haya sincronización (D05, D13), dos dispositivos pueden crear registros sin chocar. Los UUID v7 empiezan por la fecha, así que se ordenan por momento de creación.
- **Tablas `STRICT`**: SQLite comprueba los tipos de cada columna (por defecto no lo hace).
- **Fechas** en texto ISO 8601: `2026-09-24` para días y `2026-09-24T21:30:00.000Z` para instantes.
- **ISBN normalizado a ISBN-13** y sin guiones. Un ISBN-10 se convierte al darlo de alta; el ISBN-10 se puede recalcular cuando haga falta.
- **Herencia por tablas** (D08): `editions` guarda lo común a cualquier catálogo y `book_details` lo específico de los libros.

## Esquema (hito 1.1)

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
| `work_contributors` | Quién participó en una obra y en qué orden | `work_id`, `person_id`, `role` (`author`), `position` |
| `editions` | Edición: una publicación concreta | `id`, `work_id`, `title`, `subtitle`, `publisher`, `published_year`, `language` |
| `book_details` | Datos propios de un libro | `edition_id`, `isbn13` (único) |
| `members` | Miembros de la casa (propietarios). En modo local son solo nombres, no cuentas | `id`, `name` (único, sin distinguir mayúsculas) |
| `copies` | Ejemplar: lo que hay en casa | `id`, `edition_id`, `owner_id`, `format`, `quantity`, `location`, `acquired_on` |

**Formatos** (`copies.format`): `hardcover` (tapa dura), `paperback` (tapa blanda), `pocket` (bolsillo), `ebook`, `audiobook`, `other`. El formato va en el ejemplar, como pide VISION.md: lo que interesa saber es en qué formato lo tienes.

## Reglas del alta

- Si el ISBN **ya existe**, no se duplica la edición: se añade un ejemplar nuevo a la edición existente y la app lo avisa.
- Si no hay ISBN o es nuevo, se crean a la vez la obra, la edición y el ejemplar.
- Autores y propietarios se buscan por nombre (sin distinguir mayúsculas) y se crean si no existen.

## Pendiente en hitos posteriores

- Registro de operaciones (1.2).
- Portadas y enlaces a Google Books y Open Library (1.3).
- Ubicaciones como jerarquía (salón → estantería → balda) en lugar de texto libre (1.5).
- Saga y número, temáticas, etiquetas y campos de rol (sin hito asignado todavía).
