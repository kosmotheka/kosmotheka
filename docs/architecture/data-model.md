# Modelo de datos

> Esquema de la base de datos local (SQLite) de la app. Se amplía en cada hito; aquí se documenta el estado actual. Decisiones de fondo: D07, D08 y D24.

## Principios

- **Identificadores UUID v7** generados en la app, guardados como texto. No hay IDs autoincrementales: cuando haya sincronización (D05, D13), dos dispositivos pueden crear registros sin chocar. Los UUID v7 empiezan por la fecha, así que se ordenan por momento de creación.
- **Tablas `STRICT`**: SQLite comprueba los tipos de cada columna (por defecto no lo hace).
- **Fechas** en texto ISO 8601: `2026-09-24` para días y `2026-09-24T21:30:00.000Z` para instantes.
- **ISBN normalizado a ISBN-13** y sin guiones. Un ISBN-10 se convierte al darlo de alta; el ISBN-10 se puede recalcular cuando haga falta.
- **Herencia por tablas** (D08): `editions` guarda lo común a cualquier catálogo y una tabla `<tipo>_details` lo específico de cada tipo. La música, el videojuego y «Otro» no tienen (la plataforma del videojuego va en el ejemplar): el artista es un contribuidor (`artist`) y el sello, la editorial.
- **Todo pertenece a una biblioteca** (D46, D54, migración 9): cada tabla del catálogo lleva `library_id` y su clave es `(library_id, id)`. El mismo id en dos bibliotecas son dos filas distintas, y las referencias entre tablas incluyen la biblioteca. No hay clave foránea hacia `libraries`: un cambio se aplica aunque la fila de la biblioteca no haya llegado todavía.
- **Al borrar, el borrado gana** (D89, migración 30): una referencia obligatoria entre tablas (`editions.work_id`, `copies.edition_id`, `work_contributors.person_id`…) es `ON DELETE CASCADE`; una opcional (`editions.publisher_id`, `copies.owner_id`, `devices.member_id`) se pone a `NULL` con un disparador (`SET NULL` también borraría `library_id`). Ver [sync.md](sync.md).

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
| `editions` | Edición: una publicación concreta | `id`, `work_id`, `title`, `subtitle`, `publisher_id`, `published_year`, `language`, `cover` (fichero, D27), `barcode` (EAN o UPC de lo que no tiene ISBN), `notes` (texto libre) |
| `publishers` | Editoriales (D29) | `id`, `name` (no único desde D33; la app reutiliza el existente) |
| `tags` | Etiquetas: palabras libres que cada biblioteca crea para sí misma | `id`, `name` |
| `work_tags` | Qué etiquetas tiene cada obra. Sin restricción de unicidad a propósito: dos dispositivos pueden añadir la misma etiqueta sin conexión, y las dos filas deben sobrevivir para que converjan; quien lee, deduplica | `id`, `work_id`, `tag_id` |
| `work_genres` | Qué géneros tiene cada obra, de la lista cerrada por tipo (`crates/domain/src/genre.rs`). Misma regla de no unicidad que `work_tags` | `id`, `work_id`, `genre` |
| `series` | Series (D61): una saga (ordenada) o una línea de producto (no). Una obra puede estar en varias, y una serie puede mezclar autores | `id`, `name`, `ordered` |
| `series_entries` | Qué obras tiene cada serie y en qué posición. Sin restricción de unicidad a propósito (misma regla que `work_tags`) | `id`, `series_id`, `work_id`, `position` (empieza en 1, opcional) |
| `work_relations` | Qué obra necesita a cuál (D61): p. ej. una expansión necesita su juego base. Sin restricción de unicidad a propósito (misma regla que `work_tags`) | `id`, `work_id`, `related_work_id`, `relation` (`requires`) |
| `book_details` | Datos propios de un libro | `edition_id`, `isbn13` (no único desde D33), `google_books_id`, `open_library_id` |
| `comic_details` | Datos propios de un cómic | `edition_id`, `number` |
| `boardgame_details` | Datos propios de un juego de mesa | `edition_id`, `players_min`, `players_max`, `duration_minutes`, `min_age` |
| `film_details` | Datos propios de una película | `edition_id`, `duration_minutes` |
| `tv_series_details` | Datos propios de una serie | `edition_id`, `seasons` |
| `members` | Personas de la biblioteca: dueñas de ejemplares y, con roles, de permisos (D55). En modo local son solo nombres, no cuentas | `id`, `name` (no único desde D33; la app reutiliza el existente), `role` (`admin`, `editor`, `reader` o vacío), `can_share` (D73) |
| `copies` | Ejemplar: lo que hay en casa | `id`, `edition_id`, `owner_id`, `format`, `quantity`, `location` (en lo digital, la plataforma: «Kindle», «Steam»), `acquired_on`, `platform` (videojuegos: «Switch»), `seasons` (series: qué temporadas tiene, texto libre) |
| `marks` | Lo que cada persona apunta de cada obra (D68): estado, valoración, favorito y «lo deseo» (D61) | `id` (derivado de persona y obra), `member_id`, `work_id`, `status`, `status_on`, `rating`, `favorite`, `wished` |
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
| `forgotten_libraries` | Bibliotecas que este dispositivo dejó o de las que lo retiraron, para que la nube no se las devuelva; local, no es una entidad | `library_id` |
| `local_state` | Valores propios de este dispositivo: su identificador, la última marca HLC emitida, la biblioteca activa y los ajustes (clave de Google Books) | `key`, `value` |

**Formatos** (`copies.format`): `hardcover` (tapa dura), `paperback` (tapa blanda), `pocket` (bolsillo), `ebook`, `pdf`, `audiobook`, `single_issue` (grapa), `box` (caja), `dvd`, `bluray`, `uhd` (4K), `physical` (videojuego en cartucho o disco), `digital`, `vinyl`, `cd`, `cassette`, `other`. Es una sola lista para todos los tipos, y la interfaz ofrece solo los de cada uno (así un ejemplar se valida sin mirar su obra). Estas listas no llevan `CHECK` en SQLite: solo las fija la spec. El formato va en el ejemplar, como pide VISION.md: lo que interesa saber es en qué formato lo tienes.

## Reglas del alta

- Toda escritura pasa por una operación ([operations.md](operations.md)).
- Si el ISBN **ya existe**, no se duplica la edición: se añade un ejemplar nuevo a la edición existente y la app lo avisa.
- Si no hay ISBN o es nuevo, se crean a la vez la obra, la edición y el ejemplar.
- Autores, editoriales y propietarios se buscan por nombre (sin distinguir mayúsculas) y se crean si no existen.

## Pendiente en hitos posteriores

- Ubicaciones como jerarquía (salón → estantería → balda) en lugar de texto libre (1.5).
- Campos de rol (sin hito asignado todavía).

## Actualizar la app: copia automática

Antes de aplicar una migración a una base de datos que ya tiene datos, la app deja una copia al lado con la versión de la que viene: por ejemplo `library.sqlite3.v8.bak` junto a `library.sqlite3`, en la carpeta de datos de la app. Si una actualización sale mal, basta con cerrar la app, borrar `library.sqlite3` y renombrar la copia a `library.sqlite3`; la app anterior la abrirá tal cual. La copia se hace una vez por versión y no se borra sola.

**Migración 9 (bibliotecas):** todo lo que había pasa a una biblioteca con un id fijo igual en todos los dispositivos (`00000000-0000-7000-8000-000000000001`), para que dos dispositivos que ya se sincronizaban se fundieran sin duplicados (D46). Las operaciones anteriores quedan como historial y cada dispositivo registra una nueva operación inicial con lo que tiene. La migración es historia y no se toca; el id fijo se retira después (D85).

**Ninguna biblioteca tiene id fijo (D85).** Tras migrar, en cada arranque y de forma idempotente, la app ejecuta `libraries::retire_fixed_library` y `libraries::ensure_first_library`:
- si la biblioteca con el id fijo está **vacía** (sin filas de catálogo, sin operaciones y sin emparejamientos, avisos, retiradas, dueños pendientes ni avisos de petición), se borra, y se activa otra;
- si tiene **datos**, se duplica como biblioteca nueva con id aleatorio (D58: los datos se conservan con operaciones firmadas, el historial no), se quita la vieja de este dispositivo como al salir de ella (contenido, registro y emparejamientos, sin operación) y la copia pasa a ser la activa. Todo en una transacción;
- si después no hay ninguna biblioteca (instalación nueva), se crea la primera con id aleatorio y su operación `library.create` firmada, y es la activa.

La biblioteca activa es la guardada si aún existe y, si no, la primera por nombre e id (`ops::active_library`). Una biblioteca nace en un dispositivo y llega a los demás sincronizando; los dispositivos que ya compartían la antigua por la red local vuelven a compartir la nueva.

**Cambio de identificador (D70):** la carpeta de datos se llama como el identificador de la app, que pasó de `com.tersoal.library` a `app.kosmotheka` (`~/.local/share/app.kosmotheka/` en Linux, `%APPDATA%\app.kosmotheka\` en Windows). Mientras la carpeta nueva no tenga base de datos, la app **copia** la antigua (base de datos con `VACUUM INTO`, portadas e identidad de sincronización, con sus permisos) y la deja intacta: volver a la versión anterior la encuentra como estaba. Los datos de la vista web (cachés, almacenamiento local) no se copian. En Android el identificador nuevo es otra app y no puede leer la antigua: los datos se recuperan sincronizando o con una copia de seguridad.

## Copias de seguridad (D30, D86)

Una copia es un zip con `backup.json` (catálogo de cada biblioteca e historial de operaciones) y `covers/`. **Nunca lleva secretos**: ni la semilla de la clave de firma, ni el token y la sesión de la cuenta, ni los secretos de emparejamiento (`known_servers`, `known_clients`), ni la identidad TLS; esos viven en `local_state`, en esas tablas y en la carpeta de datos, y la exportación no los lee. Lo garantiza el test `no_backup_carries_a_secret`, que exporta una copia sin cifrar y otra cifrada y busca cada secreto en sus bytes y en cada fichero de dentro.

**Restaurar y la identidad de la biblioteca (D97).** El id de una biblioteca sale de la clave de quien la fundó, y la semilla de esa clave no va en la copia. Por eso una biblioteca vuelve con su id (y se une a la original al sincronizar) solo en el dispositivo que la fundó, con su misma clave; en cualquier otro, por ejemplo un móvil nuevo, la restauración funda una biblioteca nueva con el mismo catálogo y **no se une a la original**. La pantalla de restaurar lo dice por cada biblioteca (`sameId` en el resumen). Para tener la original en un móvil nuevo hay que emparejarlo con un dispositivo que la tenga, o bajarla de la nube. Duplicar una biblioteca crea siempre una nueva con su propio id.

Por defecto la copia va **cifrada con contraseña** (D86): un fichero `.kosmotheka` con un sobre propio, sencillo y documentado aquí, cuyo contenido es el zip de siempre. Se construye con `scrypt` y ChaCha20-Poly1305 de RustCrypto (crates `scrypt` y `chacha20poly1305`). Una copia sin cifrar (zip) se sigue restaurando igual.

**El sobre, byte a byte:**

| Bytes | Contenido |
|---|---|
| 18 | Marca: el texto ASCII `kosmotheka-backup` seguido del byte `0x01` (versión 1 del formato) |
| 1 | `log_n` de scrypt |
| 4 | `r` de scrypt, entero sin signo, big-endian |
| 4 | `p` de scrypt, entero sin signo, big-endian |
| 16 | Sal, aleatoria |
| 12 | Nonce, aleatorio |
| el resto | El zip cifrado con ChaCha20-Poly1305 (IETF, RFC 8439), con su etiqueta de 16 bytes al final |

- **Clave:** `scrypt(contraseña en bytes UTF-8, sal, log_n, r, p)` con salida de 32 bytes. Se escribe `log_n = 17, r = 8, p = 1` (la recomendación de OWASP para scrypt).
- **Datos asociados:** toda la cabecera (los primeros 55 bytes, de la marca al nonce). Cambiar cualquier byte de ella hace que el descifrado falle.
- **Nonce aleatorio:** un nonce de 96 bits al azar solo es un riesgo cuando una misma clave cifra muchísimos mensajes. Aquí la clave se usa **una sola vez** (cada copia lleva su propia sal, y por tanto su propia clave), así que no hay colisión que temer.
- **Límites al restaurar:** la app rechaza, antes de ejecutar scrypt, un fichero con parámetros por encima de `log_n 20, r 16, p 4`, o que pida más de 256 MiB de memoria (scrypt usa 128 · r · 2^log_n bytes; las copias de la app piden 128 MiB), para que un fichero manipulado no pueda agotar la memoria de un móvil con el error genérico `unreadable`. Los parámetros más bajos se aceptan.
- **Detección:** si el fichero empieza por la marca, está cifrado y la app pide la contraseña; si empieza por `PK`, es un zip sin cifrar. Una contraseña equivocada o un fichero alterado dan el mismo error, `wrong_passphrase` (no se distinguen), y una contraseña ausente `passphrase_needed`: en ambos casos no se escribe nada. Las copias cifradas con `age` de las versiones de desarrollo del 3 de octubre de 2026 no se pueden abrir y dan `old_encrypted_backup`.

**Abrir una copia sin la app** (Python 3 con `cryptography`; `hashlib.scrypt` viene con Python). Escribe el zip junto al fichero, con la extensión `.zip` añadida:

```python
import hashlib, struct, sys
from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305

path, passphrase = sys.argv[1], sys.argv[2]
data = open(path, "rb").read()

magic = b"kosmotheka-backup\x01"           # 18 bytes: name + format version
assert data.startswith(magic), "not an encrypted Kosmotheka backup"
header = data[:55]                          # magic .. nonce, also the associated data
log_n = data[18]
r, p = struct.unpack(">II", data[19:27])
salt, nonce = data[27:43], data[43:55]

key = hashlib.scrypt(passphrase.encode(), salt=salt, n=2**log_n, r=r, p=p,
                     maxmem=2**31 - 1, dklen=32)
# Raises InvalidTag on a wrong passphrase or a modified file.
zip_bytes = ChaCha20Poly1305(key).decrypt(nonce, data[55:], header)
open(path + ".zip", "wb").write(zip_bytes)
```

```bash
python decrypt.py copia.kosmotheka "la contraseña"   # crea copia.kosmotheka.zip
```

El formato se comprobó así con una copia generada por la app (test `print_encrypted_backup_sample`, a mano) y un script como este en un contenedor de Docker, con versiones fijadas.

**La app no guarda la contraseña en ningún sitio. Si se pierde, la copia no se puede abrir** (ni con la app ni con el script): hay que avisarlo antes de crearla. Exportar a CSV no se cifra.

## Ficheros fuera de la base de datos

- `covers/`: portadas, con nombre `<sha256>.<ext>` (D27). Una portada que ya no usa ningún libro se queda en disco; limpiarlas es una tarea pendiente.
