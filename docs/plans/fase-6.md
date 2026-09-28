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

- **`work.kind`** pasa a admitir `book`, `comic`, `boardgame`, `film`, `tv_series`, `videogame`, `music` y `other`.
- **Los formatos, una sola lista para todos los tipos** en `copy.format`: `hardcover`, `paperback`, `ebook`, `pdf`, `audiobook`, `box`, `dvd`, `bluray`, `cartridge`, `disc`, `digital_download`, `vinyl`, `cd`, `cassette`, `other`… La interfaz ofrece solo los del tipo de la obra. Así la validación sigue siendo de una fila sola y no hay que mirar la obra para validar un ejemplar.
- **Lo digital:** los formatos digitales no tienen estantería; su «dónde» es la plataforma («Kindle», «Steam»), como dice D60, en el mismo campo `location`.
- **Los datos propios de cada tipo** en una tabla 1:1 por tipo (D08), como `book_details` hoy, con pocos campos (pregunta 2). Los identificadores de proveedor, en su fila de detalles.
- **Papeles de los contribuidores:** `work_contributor.role` admite más valores (autoría, ilustración, dirección, diseño, artista, estudio…). La persona sigue siendo `person`.
- **Código de barras genérico** (EAN/UPC) en la edición, aparte del ISBN, para que el escáner reconozca juegos, discos y películas.

### 6.5 Lo que falta y lo que deseo

- **«Me falta»** = una obra del catálogo sin ejemplares (D61). Crearla es catálogo: un lector la pide.
- **«Lo deseo»** = un campo `wished` en la marca de la persona (D61 y D68). Cada persona desea lo suyo sin pedir nada.
- **La entidad `wish` desaparece,** con su regla de permisos de 5.7.
- **Sin migración de la lista de deseos (D36):** está vacía; se sustituye sin más.

### 6.6 Series

- Entidades nuevas: `series` (nombre, si tiene orden) y `series_entry` (serie, obra, posición). Una obra puede estar en varias series.
- «Necesita»: una relación entre obras (`work_relation`, de una obra a otra, tipo `requires`).
- Los huecos de una serie son sus obras sin ejemplares.

### 6.7 Géneros, temáticas y etiquetas

- Los géneros, una lista cerrada por tipo en la app (no son filas de la biblioteca); las etiquetas, una entidad `tag` de cada biblioteca y una relación con la obra, con autocompletado.

## Decidido al preparar la fase (28/09/2026)

1. **Películas y series de televisión son dos tipos** (D60): sus formatos son distintos; la película relacionada con una serie no se modela.
2. **Datos propios de cada tipo, lo básico** (aceptado tal cual). Además de lo común (título, año, portada, notas, ejemplares, dónde y de quién):

   | Tipo | Datos propios | Formatos |
   |---|---|---|
   | Libro | autoría, editorial, ISBN, idioma | tapa dura, blanda, bolsillo, ebook, PDF, audiolibro |
   | Cómic | guion, dibujo, editorial, número | tapa dura, blanda, grapa, digital |
   | Juego de mesa | diseño, editorial, jugadores (mín.–máx.), duración, edad mínima | caja, digital |
   | Película | dirección, duración | DVD, Blu-ray, 4K, digital |
   | Serie | temporadas | DVD, Blu-ray, digital |
   | Videojuego | plataforma, estudio | físico (cartucho o disco), digital |
   | Música | artista, sello | vinilo, CD, casete, digital |
   | Otro | nada propio | otro |

   Nada de sinopsis, reparto, páginas ni pistas; se añade si alguien lo pide.
3. **Lo que falta y lo que deseo** (D61): la lista de deseos actual está vacía y **no se migra**. «Lo deseo» es una marca por persona, sin pedir nada. **Un lector no crea obras**, tampoco las que faltan: lo pide, como todo lo demás (D73).
4. **Proveedores:** solo los sin clave en esta fase; cualquier proveedor que pida clave admitirá la clave propia en Ajustes (D62).
5. **Géneros y etiquetas, de cada biblioteca:** una lista cerrada de géneros por tipo que trae la app, traducida y revisada, y etiquetas libres que crea cada biblioteca solo para sí. Nada es global, así que nadie ve lo que crea un desconocido.
6. **Precio de compra: fuera** de la fase (obligaría a elegir divisa); queda en ideas para el futuro.

## Antes de empezar

- **Auditoría:** se hace antes de construir nada de esta fase.
- **Fase 4:** quedan sus pruebas en pantalla (ver el ROADMAP). Google Drive desde el móvil, aplazado.

## Estado

- [x] Plan escrito (28/09/2026).
- [x] 6.1 (a) Listas de tipos, papeles y formatos: spec, dominio, PHP y migración 0017 (las tablas se reconstruyen sin `CHECK`).
- [x] 6.1 (b) Código de barras en la edición.
- [x] 6.1 (c) Una tabla de detalles por tipo, como manda D08 (cómic, juego de mesa, película, serie, videojuego); música y «Otro» no tienen. Test de convergencia.
- [x] 6.5 (a) «Lo deseo» como campo `wished` de la marca, con filtros «lo que deseo» y «lo que desean otros» en el motor.
- [x] 6.2 (motor) El catálogo crea, edita, lee, mueve y filtra obras de cualquier tipo: contribuidores por papel, código de barras (también para no duplicar ediciones) y datos propios de cada tipo. `authors` convive con `contributors` hasta que la interfaz pase a usar solo `contributors`.
- [x] 6.7 (motor) Géneros (lista cerrada por tipo, en `genre.rs`; **pendiente de que el maintainer revise la lista**) y etiquetas de cada biblioteca, con filtros, búsqueda por etiqueta y gestión de etiquetas. Las filas de enlace no son únicas a propósito, para converger.
- [x] 6.6 (motor) Series con orden o sin él, una obra en varias, reordenar, «cuántas faltan», y la relación «necesita» en las dos direcciones. Convergencia probada (misma obra añadida sin conexión; serie borrada mientras otro añade).
- [x] 6.5 (b, motor) «Me falta»: crear una obra sin ejemplares (o pedirla, si se es lector) y filtrar por «lo tengo» o «me falta». La entidad `wish` sigue hasta que la interfaz se pase a las marcas.
