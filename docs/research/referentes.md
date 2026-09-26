# Referentes

> Apps parecidas y qué ideas tomar de ellas. Recopilado el 26/09/2026; amplía la sección de mercado de [VISION.md](../VISION.md). Los candidatos de nombre están en [naming.md](naming.md).

## 1. Apps de las que sacar ideas
## 1. Apps de las que sacar ideas

### Catálogo de libros y colecciones

| App | Qué es | Qué tomar |
|---|---|---|
| **Libib** | Web, iOS, Android. Cataloga libros, películas, música, videojuegos y juegos de mesa. La usan bibliotecas y colegios. Gratis hasta 5.000 elementos; los préstamos a usuarios son de pago (9 $/mes). | El modelo: gratis para particulares y de pago para organizaciones. La ludoteca para asociaciones encaja ahí. |
| **Plumerie** | Web. Biblioteca familiar, ubicaciones por habitación, préstamos, escaneo de estantería y exportación «sin ataduras». Gratis hasta 100 libros; Plus, 3,99 €/mes. | Ubicaciones por habitación, escaneo con foto y la promesa de que los datos se pueden exportar siempre. |
| **iCollect Everything** | iOS, Android, Mac, Windows y web. Más de 25 tipos de colección, código de barras, préstamos, sincronización. | Cómo resuelve fichas distintas por tipo, y la carga masiva con el escáner. |
| **Coleka** | iOS y Android, más de 100.000 descargas, 4,5★. La IA reconoce cartas con la cámara. Base de datos colaborativa con más de un millón de referencias. | Una base de datos que crece con la comunidad. |
| **Cubby** | Android e iOS. «Coleccionar con cabeza»: lista de deseos, presupuesto de gasto, reservas, evitar compras duplicadas. | Lista de deseos y aviso de duplicados: el «¿lo tengo ya?» visto desde otro ángulo. |
| **Kolekto** | Web, iOS, Android. 4,50–5,99 £/mes. Atributos personalizados, etiquetas imprimibles, compartir con acceso por elemento. | Etiquetas QR imprimibles para la ludoteca. |
| **CatalogIt** | Freemium, para museos y coleccionistas. Tiene un «HUB» para publicar colecciones. | Un catálogo público para que una asociación muestre qué tiene disponible. |
| **CLZ** (Books, Comics, Games) | Una app de pago por cada tipo de colección. | Qué campos de ficha espera un coleccionista serio. |
| **Book Catalogue** | Android, open source (GPL), más de 500.000 descargas, 3,7★. | La lección de sus quejas: portadas perdidas y datos que dejan de llegar cuando cambia la fuente externa. No depender de un solo proveedor y guardar copia de las portadas. |

### Préstamo y comunidad

| App | Qué es | Qué tomar |
|---|---|---|
| **Bookery** | Web, gratis. Un club de lectura convertido en biblioteca compartida, con recordatorios de devolución. | Unirse a un grupo con código y avisos ligados a las fechas de reunión. |
| **ShareMyBooks** | Bibliotecas agrupadas en comunidades: amigos, oficinas, colegios, cooperativas. Reservas y avisos de retraso. | Un modelo que crece por escalas, con entrada aprobada por un administrador. |
| **Mooch** | iOS y Android, gratis del todo. Préstamo de cualquier objeto entre amigos, con mensajería y grupos de barrio. | El flujo pedir → prestar → recordar y las invitaciones por enlace. |
| **BooksToBorrow** | iOS, gratis. Préstamo de libros entre vecinos. | Descubrir libros por cercanía, para la versión pública. |

**Conclusión:** ninguna pieza suelta es nueva; la comunidad de préstamo ya existe. El hueco está en la **combinación**: catálogo familiar con permisos, varios tipos de catálogo, préstamos hechos y recibidos, comunidad y uso por asociaciones, gratis u open source.


Fuentes: libib.com/pricing, plumerielibrary.com (incluida su comparativa de apps de préstamo), clz.com, github.com/benjaminjonard/koillection, github.com/fabiogasparferreira/ludoteca, bookeryapp.co, sharemybooks.app, moochapp.com, icollecteverything.com y kolekto.io.

## 2. Qué se nos había escapado

Revisión del 26/09/2026: cada idea de las apps anteriores y de la [visión original](../VISION.md) comparada con el [ROADMAP](../ROADMAP.md), [DECISIONS](../DECISIONS.md) y el esquema actual (`crates/domain/src/schema.rs`).

### No está en el roadmap

| Idea | De dónde | Comentario |
|---|---|---|
| **Red de préstamo entre amigos**: ver qué tienen tus amigos y pedirles algo | Visión original (§9), Mooch, Bookery, ShareMyBooks | Es la mitad del «hueco» que justifica el proyecto y se perdió al pasar de la visión al roadmap. La Fase 9 solo cubre préstamos hechos y recibidos. Encaja con compartir una biblioteca en modo lector, pero pedir un préstamo es un flujo nuevo, parecido a las peticiones de la Fase 5. |
| **Estado de lectura completo**: pendiente, leyendo, leído o abandonado, con fechas | Visión original (§5) | El roadmap solo tiene «leído» y «deseado» por persona (Fase 6). Las estadísticas por año de la Fase 9 necesitan las fechas. |
| **Valoración y favoritos** por persona | Visión original (§5) | No aparece en ninguna fase. |
| **Géneros, temáticas y etiquetas** | Visión original (§5) | No aparece. Es distinto de los campos personalizados, que están en «Queda abierto». |
| **Reservar un ejemplar prestado**: «me lo pido cuando vuelva» | ShareMyBooks | Útil en familia y más aún en una ludoteca. Depende de los préstamos (Fase 9). |
| **Etiquetas QR imprimibles** por ejemplar | Kolekto | Escanear la etiqueta abre el ejemplar para prestarlo o devolverlo. Encaja con el modo inventario (Fase 10) y con las ludotecas (Fase 11). |
| **Catálogo público de solo lectura** | CatalogIt, BooksToBorrow | Para que una asociación muestre qué tiene, o para enseñar tu biblioteca a un amigo sin darle cuenta. D22 solo habla de la landing y de la documentación. |
| **Precio de compra y gasto** | Cubby | El ejemplar tiene fecha de compra, pero no precio. Sirve para un presupuesto, para las estadísticas y para el valor a efectos de seguro. |
| **Importar desde Calibre** | Visión original (§8) | El CSV (D30) está hecho; Calibre no aparece. |
| **«¿Qué leo ahora?»**: recomendaciones solo entre lo que hay en casa sin leer | Visión original (§9) | Encaja con la IA opcional (D18, D20). |
| **Estantería visual con lomos** | Visión original (§9) | No aparece. |

### Ya está previsto

| Idea | De dónde | Dónde está |
|---|---|---|
| «¿Lo tengo ya?» y aviso de duplicados | Cubby | «¿Lo tengo?» y D28; en todas las bibliotecas en la Fase 4 |
| Exportar siempre, sin ataduras | Plumerie | D30 (ZIP y CSV); Google Drive en la Fase 4 |
| Ubicaciones por habitación | Plumerie | Campo de ubicación; jerarquía en la Fase 10 |
| Foto de la estantería | Plumerie | D19 y Fase 10 |
| Fichas distintas por tipo de catálogo | iCollect Everything, CLZ | D60 y Fase 6 |
| Carga masiva con el escáner | iCollect Everything | D19 y D35 |
| No depender de un solo proveedor; guardar las portadas | Book Catalogue | D62 y D27 |
| Unirse con un código o un enlace | Bookery, Mooch | QR de emparejamiento (D13) e invitaciones (D43) |
| Recordatorios de devolución | Bookery, ShareMyBooks | Préstamos con avisos, Fase 9 |
| Entrada aprobada por un administrador | ShareMyBooks | Roles y peticiones, Fase 5 |
| Libros de rol: de qué libro depende | Visión original (§5) | Relación «necesita» en las series, Fase 6 |

### Chocan con decisiones tomadas

| Idea | De dónde | Por qué no encaja tal cual |
|---|---|---|
| Compartir un solo elemento | Kolekto | La unidad que se comparte es la biblioteca (D54). El catálogo público o un préstamo con enlace cubrirían la necesidad. |
| Base de datos colaborativa que crece con los usuarios | Coleka | Choca con local-first y con las licencias de los proveedores (D62). Si llega, sería un servicio de la nube. |
| Gratis para particulares, de pago para organizaciones | Libib | D04 dice que todo es open source; un servicio de pago necesitaría licencias de los proveedores (D62). Es una cuestión de modelo de negocio, no de código. |
