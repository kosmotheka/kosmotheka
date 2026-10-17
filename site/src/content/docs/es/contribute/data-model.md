---
title: El modelo de datos
description: Las entidades del catálogo y de compartir, cómo se relacionan, por qué la obra y la edición van separadas y cómo se identifican las filas.
sidebar:
  order: 11
---

Esta página es el mapa de los datos: qué se guarda, cómo se relaciona y por qué. Para el detalle exacto de cada campo, el código manda: [`crates/domain/src/schema.rs`](https://github.com/kosmotheka/kosmotheka/blob/main/crates/domain/src/schema.rs) lista los campos de cada entidad, y [`spec/operation.schema.json`](https://github.com/kosmotheka/kosmotheka/blob/main/spec/operation.schema.json) los mismos para quien implemente el formato. Los nombres están en el [glosario](../glossary/). Cada cambio de datos viaja como una [operación](../operations-spec/).

## Principios

- **Los ids son UUID, generados por la app**, y se guardan como texto. No hay ids autoincrementales: dos dispositivos pueden crear filas sin conexión y sin chocar. (Los UUID v7 empiezan por la fecha y se ordenan por momento de creación.) Una excepción: el id de una marca sale de su persona y su obra.
- **Todo pertenece a una biblioteca.** Cada fila del catálogo es de una biblioteca, y su identidad es la pareja (biblioteca, id): **el mismo id en dos bibliotecas son dos filas distintas**, y las referencias entre filas se resuelven siempre dentro de la misma biblioteca. *Por qué:* que una biblioteca nunca pise ni referencie datos de otra, aunque alguien invente ids a propósito.
- **Una base de datos SQLite en cada dispositivo** (con tablas `STRICT`, que comprueban los tipos) y **PostgreSQL** en el servidor. El mismo modelo funciona en las dos.
- **Fechas** en texto ISO 8601 (`2026-09-24` para días); **ISBN** normalizado a trece cifras sin guiones.
- **Al borrar, el borrado gana.** Una referencia obligatoria entre filas borra en cascada lo que cuelga; una opcional se deja vacía. Está explicado en [El motor de sincronización](../sync-engine/).

## El catálogo

```
works ──< work_contributors >── people
  │  │
  │  ├──< work_tags >── tags
  │  ├──< work_genres            (la lista cerrada de la app)
  │  ├──< series_entries >── series
  │  ├──< work_relations         (una obra necesita a otra)
  │  └──< marks >── members      (lo de cada persona)
  │
  └──< editions ──< copies >── members
          │   └── publishers
          └── 1:1 ── <tipo>_details
```

`──<` quiere decir «uno a muchos». Las piezas principales:

| Entidad | Qué es | Campos principales |
|---|---|---|
| `work` (obra) | La cosa en abstracto: «Dune», sin decir qué edición | `kind` (`book`, `comic`, `boardgame`, `film`, `tv_series`, `videogame`, `music`, `other`), `title` |
| `edition` (edición) | Una publicación concreta de una obra | `work_id`, `title`, `subtitle`, `publisher_id`, `published_year`, `language`, `cover`, `barcode`, `notes` |
| `copy` (ejemplar) | Lo que hay en casa | `edition_id`, `owner_id`, `format`, `quantity`, `location`, `acquired_on`, `platform` (videojuegos), `seasons` (series) |
| `person` (persona) | Quien contribuyó a una obra | `name` |
| `work_contributor` | Quién participó en una obra, en qué papel y en qué orden | `work_id`, `person_id`, `role` (`author`, `writer`, `illustrator`, `designer`, `director`, `artist`, `studio`), `position` |
| `publisher` (editorial) | La editorial o el sello | `name` |
| `tag` y `work_tag` | Etiquetas: palabras libres que cada biblioteca crea para sí, y su vínculo con las obras | `name`; `work_id`, `tag_id` |
| `work_genre` | Un género de una obra, de la lista cerrada de la app por tipo (`genre.rs`) | `work_id`, `genre` |
| `series` y `series_entry` | Una saga (con orden) o una línea de productos (sin él), y las obras que contiene | `name`, `ordered`; `series_id`, `work_id`, `position` |
| `work_relation` | Una obra necesita a otra (una expansión y su juego base) | `work_id`, `related_work_id`, `relation` (`requires`) |
| `mark` (marca) | Lo que una persona apunta de una obra | `member_id`, `work_id`, `status`, `status_on`, `rating`, `favorite`, `wished` |

El **formato** de un ejemplar es una sola lista para todos los tipos (`hardcover`, `paperback`, `pocket`, `ebook`, `pdf`, `audiobook`, `single_issue`, `box`, `dvd`, `bluray`, `uhd`, `physical`, `digital`, `vinyl`, `cd`, `cassette`, `other`). La interfaz ofrece solo los de cada tipo. *Por qué una lista única:* así un ejemplar se valida sin mirar su obra.

### Por qué la obra y la edición van separadas

Un libro puede tener muchas ediciones (tapa dura de 1990, bolsillo de 2010, una traducción), y cada una puede tener varios ejemplares en casa. Separar las tres capas evita repetir datos y permite preguntas que una sola tabla no resuelve:

- **La obra** es lo que te interesa como lector o jugador: lo que **lees, valoras y deseas** (las marcas cuelgan de la obra, no de la edición) y lo que agrupas en series.
- **La edición** es lo que se **identifica** (por ISBN o código de barras), lo que tiene portada, editorial y año.
- **El ejemplar** es lo que **tienes**: formato, dueño, dónde está. El formato va aquí, no en la edición, porque lo que interesa saber es en qué formato lo tienes.

También hace barata una regla del alta: si el ISBN ya está en la biblioteca, no se duplica la edición, solo se añade un ejemplar. Y una obra sin ejemplares (porque aún no la tienes) es la lista «Me falta».

### Detalles por tipo

Lo que es propio de cada tipo vive en una tabla `<tipo>_details` con una fila por edición (la herencia por tablas de D08: lo común en `editions`, lo específico aparte, con columnas normales):

| Tabla | Campos |
|---|---|
| `book_details` | `isbn13`, `google_books_id`, `open_library_id` |
| `comic_details` | `number` |
| `boardgame_details` | `players_min`, `players_max`, `duration_minutes`, `min_age` |
| `film_details` | `duration_minutes` |
| `tv_series_details` | `seasons` |

La música, el videojuego y «Otro» no tienen tabla: el artista es un contribuidor (`artist`), el sello es la editorial, y la plataforma de un videojuego va en el ejemplar. *Por qué columnas y no un campo JSON:* se pueden consultar, validar y ordenar en SQLite y en PostgreSQL sin trucos. Añadir un tipo con datos propios supone una migración, y se acepta porque se hace muy de vez en cuando.

## Compartir y permisos

| Entidad | Qué es | Campos principales |
|---|---|---|
| `library` (biblioteca) | La unidad que se comparte, se sincroniza y tiene permisos. Su id es el de la biblioteca | `name`, `cloud_library_id`, `cloud_server`, `cloud_device_id`, `founding_nonce` |
| `member` (miembro) | Una persona de la biblioteca: dueña de ejemplares y, con roles, de permisos. **No es una cuenta**, aunque puede estar enlazada a una | `name`, `role` (`admin`, `editor`, `reader`), `can_share`, `account_id`, `access_withdrawn` |
| `device` (dispositivo) | Un dispositivo que escribe en la biblioteca, con la clave pública con la que firma | `public_key`, `name`, `member_id` (de quién es), `vouched_key`, `trusted_through` |
| `revocation` (retirada) | Un dispositivo al que se retiró el acceso | `device_id`, `device_name` |
| `change_request` | Cómo se resolvió una petición. Su id es el de la operación pedida | `status` (`accepted`, `rejected`, `withdrawn`), `note` |

- Un **miembro** (`member`) es dueño de ejemplares y puede tener un rol; una **cuenta** (`user`) existe solo en el servidor. Se enlazan con `account_id`, un id aleatorio de la cuenta que solo escribe la nube.
- El **dispositivo** es de un miembro por `member_id`, pero solo si su `vouched_key` coincide con su clave. `trusted_through` es la valla (ver [seguridad](../security/)).
- **Quién puede cambiar qué** no está en las tablas sino en las reglas de [la especificación](../operations-spec/).

## El registro y lo que lo hace converger

Tres tablas, que no son catálogo sino lo que permite sincronizarlo:

| Tabla | Qué guarda |
|---|---|
| `operations` | El registro de todas las operaciones, propias y recibidas: id, biblioteca, marca de reloj, dispositivo, intención, los cambios en JSON, versión del esquema, firma, y su estado (`applied` o `request`) |
| `field_versions` | Por cada campo de cada fila, la marca de la última operación que lo escribió |
| `tombstones` | Las lápidas: las filas borradas, para que un borrado no pueda deshacerse solo |

Además hay `search_index` (el texto de búsqueda, que mantiene `ops::record`) y `row_authors` (qué dispositivo creó cada fila, para que un editor solo borre lo que creó su persona; se recalcula desde el registro).

## Lo que no se sincroniza: el estado local

Cada dispositivo guarda además cosas que **son suyas** y no viajan: los dispositivos emparejados y sus secretos, qué bibliotecas van a cada uno (`library_peers`), los avisos pendientes de salida (`leave_notices`), las bibliotecas que dejó o de las que lo retiraron (`forgotten_libraries`) y sus ajustes y su identidad (`local_state`, que incluye la semilla de la clave de firma).

## En el servidor

El catálogo de cada biblioteca de la nube vive en PostgreSQL, con las mismas entidades. Las diferencias importantes:

- **La clave primaria es del servidor.** Cada fila lleva una `key` (UUID v7 que no sale nunca del servidor) además del `id` de la app, y es única la pareja (biblioteca de la nube, `id`). Las referencias entre filas usan la `key`. *Por qué:* el servidor nunca se fía de un id del cliente para buscar ni decidir nada.
- **La biblioteca de la nube tiene su propio id** (`cloud_library_id`, que crea el servidor), distinto del id local de la biblioteca.
- **Una fila no puede apuntar a otra biblioteca**: unos disparadores lo rechazan en la base de datos.
- **Las versiones por campo van en la propia fila** (una columna JSON), en vez de en una tabla aparte.
- **Los booleanos y los enteros son `BOOLEAN` y `BIGINT`**; no hay `CHECK` de roles ni de estados, porque son reglas de `spec/`.

Cada entidad es una clase de Doctrine en `server/src/Entity/Catalog/`. Cómo añadir un campo, y los seis sitios que hay que tocar a la vez, está en [Cómo contribuir](../how-to-contribute/#añadir-un-campo-o-una-entidad-paso-a-paso).

## Archivos fuera de la base de datos

- **`covers/`**: las portadas, con el nombre `<sha256>.<extensión>`. *Por qué el nombre por huella:* dos dispositivos que bajan la misma portada generan el mismo nombre, y al sincronizar basta pedir los archivos que faltan por nombre; además las operaciones no cargan con imágenes.
- **Las copias de seguridad**: un `.zip` con el catálogo de cada biblioteca, su historial de operaciones y las portadas, que por defecto se guarda cifrado con contraseña (un archivo `.kosmotheka`). Nunca llevan la clave de firma, ni la sesión, ni los secretos de emparejamiento.

## Al actualizar la app

Antes de aplicar una migración a una base que ya tiene datos, la app deja una copia al lado con la versión de la que viene (por ejemplo `library.sqlite3.v8.bak`). Si una actualización sale mal, basta cerrar la app, borrar `library.sqlite3` y renombrar la copia: la versión anterior la abre tal cual.
