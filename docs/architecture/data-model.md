# Modelo de datos

> Esquema de la base de datos local (SQLite) de la app. Se amplía en cada hito; aquí se documenta el estado actual. Decisiones de fondo: D07, D08 y D24.

## Principios

- **Identificadores UUID v7** generados en la app, guardados como texto. No hay IDs autoincrementales: cuando haya sincronización (D05, D13), dos dispositivos pueden crear registros sin chocar. Los UUID v7 empiezan por la fecha, así que se ordenan por momento de creación.
- **Tablas `STRICT`**: SQLite comprueba los tipos de cada columna (por defecto no lo hace).
- **Fechas** en texto ISO 8601: `2026-09-24` para días y `2026-09-24T21:30:00.000Z` para instantes.
- **ISBN normalizado a ISBN-13** y sin guiones. Un ISBN-10 se convierte al darlo de alta; el ISBN-10 se puede recalcular cuando haga falta.
- **Herencia por tablas** (D08): `editions` guarda lo común a cualquier catálogo y una tabla `<tipo>_details` lo específico de cada tipo. La música y «Otro» no tienen: el artista es un contribuidor (`artist`) y el sello, la editorial.
- **Todo pertenece a una biblioteca** (D46, D54, migración 9): cada tabla del catálogo lleva `library_id` y su clave es `(library_id, id)`. El mismo id en dos bibliotecas son dos filas distintas, y las referencias entre tablas incluyen la biblioteca. No hay clave foránea hacia `libraries`: un cambio se aplica aunque la fila de la biblioteca no haya llegado todavía.

## Esquema

```
works ──< work_contributors >── people
  │
  └──< editions ──1:1── book_details
          │
          └──< copies >── members
```

| Tabla | Qué es | Columnas principales |
|---|---|---|
| `libraries` | Bibliotecas: lo que se comparte, se sincroniza y tiene permisos. Un nombre vacío es «sin nombre» y la app muestra el suyo, en el idioma del usuario | `id`, `name` |
| `works` | Obra: el libro en abstracto | `id`, `kind` (`book`, `comic`, `boardgame`, `film`, `tv_series`, `videogame`, `music`, `other`; D60), `title` |
| `people` | Autores y otros participantes | `id`, `name` |
| `work_contributors` | Quién participó en una obra y en qué orden | `id`, `work_id`, `person_id`, `role` (`author`, `writer`, `illustrator`, `designer`, `director`, `artist`, `studio`), `position` |
| `editions` | Edición: una publicación concreta | `id`, `work_id`, `title`, `subtitle`, `publisher_id`, `published_year`, `language`, `cover` (fichero, D27), `barcode` (EAN o UPC de lo que no tiene ISBN) |
| `publishers` | Editoriales (D29) | `id`, `name` (no único desde D33; la app reutiliza el existente) |
| `book_details` | Datos propios de un libro | `edition_id`, `isbn13` (no único desde D33), `google_books_id`, `open_library_id` |
| `comic_details` | Datos propios de un cómic | `edition_id`, `number` |
| `boardgame_details` | Datos propios de un juego de mesa | `edition_id`, `players_min`, `players_max`, `duration_minutes`, `min_age` |
| `film_details` | Datos propios de una película | `edition_id`, `duration_minutes` |
| `tv_series_details` | Datos propios de una serie | `edition_id`, `seasons` |
| `videogame_details` | Datos propios de un videojuego | `edition_id`, `platform` |
| `members` | Personas de la biblioteca: dueñas de ejemplares y, con roles, de permisos (D55). En modo local son solo nombres, no cuentas | `id`, `name` (no único desde D33; la app reutiliza el existente), `role` (`admin`, `editor`, `reader` o vacío), `can_share` (D73) |
| `copies` | Ejemplar: lo que hay en casa | `id`, `edition_id`, `owner_id`, `format`, `quantity`, `location`, `acquired_on` |
| `wishes` | Lista de deseos, sincronizada (D36) | `id`, `title`, `author`, `isbn13`, `note`, `cover_url` |
| `revocations` | Dispositivos a los que se retiró el acceso a la biblioteca, sincronizada (D71) | `device_id`, `device_name` |
| `devices` | Dispositivos que escriben en la biblioteca, con la clave pública con la que firman, sincronizada (D56) | `id` (el del dispositivo), `public_key`, `name`, `member_id` (de quién es, D73) |
| `row_authors` | Qué dispositivo creó cada fila, para que un editor borre lo que creó su persona (D45); local, se recalcula desde el registro | `library_id`, `entity`, `entity_id`, `device_id` |
| `change_requests` | Cómo se resolvió cada petición, sincronizada (D73); su id es el de la operación pedida | `status` (`accepted` o `rejected`) |
| `operations` | Registro de todas las escrituras, propias y recibidas ([operations.md](operations.md)) | `id`, `library_id`, `hlc`, `device_id`, `intent`, `changes` (JSON), `schema_version`, `replicable`, `signature`, `status` (`applied` o `request`, D73) |
| `field_versions` | Última marca HLC que escribió cada campo de cada fila ([sync.md](sync.md)) | `library_id`, `entity`, `entity_id`, `field`, `hlc` |
| `tombstones` | Filas borradas: el borrado gana ([sync.md](sync.md)). Borrar una biblioteca deja una lápida de la biblioteca entera | `library_id`, `entity`, `entity_id`, `hlc` |
| `search_index` | Índice de texto completo FTS5, mantenido por `ops::record` (D29) | `library_id`, `edition_id`, `content` |
| `known_servers`, `known_clients` | Dispositivos emparejados para sincronizar por la red local; locales, no se sincronizan (D34) | ver [sync.md](sync.md) |
| `library_peers` | Qué bibliotecas van a cada dispositivo emparejado; local (D46) | `library_id`, `device_id` |
| `leave_notices` | Bibliotecas de las que este dispositivo salió, pendientes de decírselo a cada dispositivo que se las ofrecía; local (D71) | `library_id`, `device_id` |
| `local_state` | Valores propios de este dispositivo: su identificador, la última marca HLC emitida, la biblioteca activa y los ajustes (clave de Google Books) | `key`, `value` |

**Formatos** (`copies.format`): `hardcover` (tapa dura), `paperback` (tapa blanda), `pocket` (bolsillo), `ebook`, `pdf`, `audiobook`, `single_issue` (grapa), `box` (caja), `dvd`, `bluray`, `uhd` (4K), `physical` (videojuego en cartucho o disco), `digital`, `vinyl`, `cd`, `cassette`, `other`. Es una sola lista para todos los tipos, y la interfaz ofrece solo los de cada uno (así un ejemplar se valida sin mirar su obra). Estas listas no llevan `CHECK` en SQLite: solo las fija la spec. El formato va en el ejemplar, como pide VISION.md: lo que interesa saber es en qué formato lo tienes.

## Reglas del alta

- Toda escritura pasa por una operación ([operations.md](operations.md)).
- Si el ISBN **ya existe**, no se duplica la edición: se añade un ejemplar nuevo a la edición existente y la app lo avisa.
- Si no hay ISBN o es nuevo, se crean a la vez la obra, la edición y el ejemplar.
- Autores, editoriales y propietarios se buscan por nombre (sin distinguir mayúsculas) y se crean si no existen.

## Pendiente en hitos posteriores

- Ubicaciones como jerarquía (salón → estantería → balda) en lugar de texto libre (1.5).
- Saga y número, temáticas, etiquetas y campos de rol (sin hito asignado todavía).

## Actualizar la app: copia automática

Antes de aplicar una migración a una base de datos que ya tiene datos, la app deja una copia al lado con la versión de la que viene: por ejemplo `library.sqlite3.v8.bak` junto a `library.sqlite3`, en la carpeta de datos de la app. Si una actualización sale mal, basta con cerrar la app, borrar `library.sqlite3` y renombrar la copia a `library.sqlite3`; la app anterior la abrirá tal cual. La copia se hace una vez por versión y no se borra sola.

**Migración 9 (bibliotecas):** todo lo que había pasa a una biblioteca con el mismo id en todos los dispositivos (`00000000-0000-7000-8000-000000000001`), así que dos dispositivos que ya se sincronizaban se funden sin duplicados (D46). Los emparejamientos siguen y comparten esa biblioteca. Las operaciones anteriores quedan como historial y cada dispositivo registra una nueva operación inicial con lo que tiene.

**Cambio de identificador (D70):** la carpeta de datos se llama como el identificador de la app, que pasó de `com.tersoal.library` a `app.kosmotheka` (`~/.local/share/app.kosmotheka/` en Linux, `%APPDATA%\app.kosmotheka\` en Windows). Mientras la carpeta nueva no tenga base de datos, la app **copia** la antigua (base de datos con `VACUUM INTO`, portadas e identidad de sincronización, con sus permisos) y la deja intacta: volver a la versión anterior la encuentra como estaba. Los datos de la vista web (cachés, almacenamiento local) no se copian. En Android el identificador nuevo es otra app y no puede leer la antigua: los datos se recuperan sincronizando o con una copia de seguridad.

## Ficheros fuera de la base de datos

- `covers/`: portadas, con nombre `<sha256>.<ext>` (D27). Una portada que ya no usa ningún libro se queda en disco; limpiarlas es una tarea pendiente.
