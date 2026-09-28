# Plan de la Fase 6 — Catálogo general

> Plan de trabajo mientras dure la fase. Se basa en D60, D61, D62 y D68 ([DECISIONS.md](../DECISIONS.md)), en el [ROADMAP](../ROADMAP.md) y en [research/proveedores.md](../research/proveedores.md). Al cerrar la fase pasa a `historico/`. Escrito el 28/09/2026: lo que no está en esas decisiones es **propuesta**, y hay [preguntas](#pendiente-de-decidir) que conviene hablar antes de construir.

**Hecho cuando:** tengo mis juegos de rol en papel y en PDF, una saga con los tomos que me faltan, y los juegos de mesa y las películas de la familia.

## Lo que ya existe

- **Pantallas de Design de la Fase 3** (`docs/design/screens/{desktop,mobile}/`): catálogo con chips de tipo, alta que empieza por buscar (con «¿Qué es?» y «Añadir a mano»), resultados, alta a mano con «¿Lo tienes? Sí / No, me falta», serie con huecos y «Me falta» con «Solo deseos». Las de la Fase 5.8 ya dibujan el catálogo con todos los tipos. Faltan pantallas para lo propio de cada tipo, las etiquetas y gestionar series.
- **En el código:** todo es libro. `work.kind` solo admite `book`, los formatos son de libro, los contribuidores solo `author`, los datos propios van en `book_details` (ISBN e ids de proveedor) y «Me falta» es la lista de deseos de D36 (entidad `wish`, suelta del catálogo).
- **Marcas por persona** (D68, hito 5.8): hechas; aquí solo cambia el nombre de «en curso» según el tipo.

## Hitos

Cada hito deja la app funcionando, con sus tests, y se prueba antes de seguir. El orden sigue el de D60: libros y «Otro» primero, después juegos de mesa y música (tienen proveedores sin clave que buscan por código de barras), películas y series, videojuegos y cómics.

| Hito | Qué | Dónde |
|---|---|---|
| **6.1 Tipos y formatos (datos)** | La lista cerrada de tipos en `work.kind`, formatos por tipo, papeles de los contribuidores (autor, ilustración, dirección, diseño…) y los datos propios de cada tipo. Migración: todo lo que hay sigue siendo libro | `spec/`, `crates/domain`, migración, `server/` |
| **6.2 Tipos en la interfaz** | Elegir el tipo al añadir y editar, la ficha y el formulario de cada tipo, chips de tipo en el catálogo, portada con la forma de cada tipo, y «Leyendo» → «Jugando» o «Viendo» en las marcas | `app/src` |
| **6.3 El alta empieza por buscar** | Buscar por título en los proveedores del tipo, escanear cualquier código y que la app decida qué es, y «Añadir a mano» siempre a la vista | `metadata/`, `app/src` |
| **6.4 Proveedores sin clave** | Inventaire (libros en español), MusicBrainz y Cover Art Archive (música), GameUPC (juegos de mesa por código de barras) y Wikidata como respaldo de todo. Aviso de lo que solo está con la nube | `metadata/` |
| **6.5 Lo que falta y lo que deseo** | «Me falta» pasa a ser una obra del catálogo sin ejemplares, con su vista y sus huecos en cada serie; «lo deseo», una marca por persona. La lista de deseos de D36 se migra | `spec/`, migración, `app/` |
| **6.6 Series** | Series con orden o sin él, una obra en varias, huecos, y la relación «necesita» (expansiones, suplementos) | `spec/`, `crates/domain`, `app/` |
| **6.7 Géneros, temáticas y etiquetas** | Para clasificar y filtrar | `spec/`, `app/` |

Las pantallas que faltan se piden a Claude Design antes de construir 6.2, 6.5, 6.6 y 6.7.

## Decisiones técnicas propuestas

Todo **propuesta** hasta que el maintainer lo revise.

### 6.1 Tipos y formatos

- **`work.kind`** pasa a admitir `book`, `comic`, `boardgame`, `film`, `videogame`, `music` y `other` (ver pregunta 1 para películas y series).
- **Los formatos, una sola lista para todos los tipos** en `copy.format`: `hardcover`, `paperback`, `ebook`, `pdf`, `audiobook`, `box`, `dvd`, `bluray`, `cartridge`, `disc`, `digital_download`, `vinyl`, `cd`, `cassette`, `other`… La interfaz ofrece solo los del tipo de la obra. Así la validación sigue siendo de una fila sola y no hay que mirar la obra para validar un ejemplar.
- **Lo digital:** los formatos digitales no tienen estantería; su «dónde» es la plataforma («Kindle», «Steam»), como dice D60, en el mismo campo `location`.
- **Los datos propios de cada tipo** en una fila de detalles por tipo, como `book_details` hoy, con pocos campos (pregunta 2). Los identificadores de proveedor, en su fila de detalles.
- **Papeles de los contribuidores:** `work_contributor.role` admite más valores (autoría, ilustración, dirección, diseño, artista, estudio…). La persona sigue siendo `person`.
- **Código de barras genérico** (EAN/UPC) en la edición, aparte del ISBN, para que el escáner reconozca juegos, discos y películas.

### 6.5 Lo que falta y lo que deseo

- **«Me falta»** = una obra del catálogo sin ejemplares (D61). Ya lo es hoy en parte: una obra puede quedarse sin ejemplares.
- **«Lo deseo»** = un campo `wished` en la marca de la persona (D61 y D68: deseado es una marca). Así cada persona desea lo suyo y lo marca sin pedir nada, como las demás marcas.
- **Migración de la lista de deseos (D36):** cada deseo pasa a ser una obra sin ejemplares, con la marca «lo deseo» de… (pregunta 3).

### 6.6 Series

- Entidades nuevas: `series` (nombre, si tiene orden) y `series_entry` (serie, obra, posición). Una obra puede estar en varias series.
- «Necesita»: una relación entre obras (`work_relation`, de una obra a otra, tipo `requires`).
- Los huecos de una serie son sus obras sin ejemplares.

### 6.7 Géneros, temáticas y etiquetas

- Una entidad `tag` con su clase (género, temática o etiqueta libre) y una relación con la obra. Libres, creadas por quien cataloga, con autocompletado (pregunta 5).

## Pendiente de decidir

Para hablarlo antes de construir, no para contestar de golpe.

1. **Películas y series de televisión:** ¿un tipo o dos? D60 dice «películas y series» como uno; una serie de televisión tiene temporadas, que se parecen más a los tomos de una saga.
2. **Qué datos propios tiene cada tipo.** Propuesta de mínimo: juegos de mesa, jugadores (mínimo y máximo), duración y edad; películas, dirección y duración; videojuegos, plataforma; música, artista y sello; cómics, guion, dibujo y número. ¿Algo más, o menos?
3. **La lista de deseos actual y el lector.** Hoy un lector apunta deseos sueltos (lo decidimos al construir 5.7). Si «me falta» pasa a ser una obra del catálogo, un lector que quiera algo que no está tendría que pedirlo (crear la obra es catálogo), aunque luego pueda marcarlo «lo deseo» sin pedir nada. ¿Nos vale, o el lector debe poder crear obras «que faltan» sin pedir permiso? Y al migrar, ¿de quién es la marca «lo deseo» de cada deseo antiguo?
4. **Qué proveedores en esta fase.** Propuesta: solo los sin clave (6.4). Los de clave (BoardGameGeek, TMDB, IGDB, Comic Vine, Discogs) van con la nube (Fase 7), porque sus claves viven en el servidor (D62).
5. **Géneros y temáticas:** ¿lista libre que crece con el uso, o una lista inicial por tipo?
6. **Precio de compra** (ROADMAP): ¿lo metemos como campo opcional del ejemplar, o lo dejamos fuera hasta que alguien lo pida?

## Antes de empezar

- **Auditoría:** se hace antes de construir nada de esta fase.
- **Fase 4:** quedan sus pruebas en pantalla y guardar la copia en Google Drive desde el móvil (ver el ROADMAP).

## Estado

- [x] Plan escrito (28/09/2026).
