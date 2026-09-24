# Biblioteca familiar — visión del proyecto

> Nombre provisional. Documento de traspaso: resume la conversación de ideación del 24 de septiembre de 2026 para seguir en Claude Code con todo el contexto.
> **Documento histórico.** Varias decisiones de la sección 4 se han revisado después: el estado actual está en [DECISIONS.md](DECISIONS.md) y el plan, en [ROADMAP.md](ROADMAP.md).
> **Qué pido a Claude Code:** leer este documento entero, hacerme las preguntas que necesite sobre las decisiones abiertas y proponer un plan de ejecución por fases **antes de escribir código**.

---

## 1. Por qué este proyecto

- Es un proyecto personal, por diversión. Soy desarrollador web: mi stack habitual es **Symfony + Docker**, con PHP y Python.
- **Con este proyecto quiero aprender algo distinto de la web**: **Rust y Tauri**.
- Me encantan los libros, sobre todo las novelas y los libros de rol. En casa somos varios lectores, cada uno con los suyos, y no hay forma sencilla de saber qué tenemos, dónde está y de quién es.
- Uso **Linux** y **Windows**, así que el escritorio tiene que funcionar en los dos.

## 2. Qué es

Un catálogo de la biblioteca de casa, compartido por la familia, para saber qué tenemos, dónde está, quién lo ha leído y a quién se lo hemos prestado. Primero es un MVP para uso familiar. **Si funciona, quiero publicarlo algún día para que cualquiera lo use gratis.**

El caso de uso estrella es el móvil: estoy en una librería o en una tienda de rol, escaneo el código de barras y me dice si ya lo tengo, de quién es y en qué formato.

## 3. Plataformas

- Escritorio: **Linux y Windows**.
- Móvil: **Android**, instalando una **APK** directamente, sin pasar por la tienda.
- Propuesta: **Tauri 2**, que compila escritorio y Android desde el mismo código (y también iOS). La interfaz va en web (HTML, CSS y JS/TS) y la parte nativa en Rust.
- Una versión web no está descartada del todo.

## 4. Arquitectura: decisión tomada y decisiones abiertas

**Tomada:** hace falta **servidor con base de datos persistente**. Los permisos por usuario, que cada miembro tenga su lista de favoritos y sus leídos, y la futura versión pública lo exigen. El servidor es la fuente de la verdad.

Durante la ideación se valoró el enfoque **local-first** (CRDTs, sincronización sin servidor, con librerías como Automerge o Yjs). Se descartó como arquitectura principal porque sin autoridad central es difícil imponer permisos (que los niños de la casa no puedan borrar). Me sigue dando curiosidad; puede servir como caché sin conexión en el cliente.

**Abiertas (quiero que me ayudes a decidirlas):**
1. **Lenguaje del servidor.** ¿Symfony, que domino y cuya infraestructura puedo copiar de otros proyectos míos? ¿O Rust (por ejemplo Axum), para aprender más Rust? Hay tensión entre ir rápido y aprender.
2. **Base de datos.** Probablemente PostgreSQL.
3. **Sin conexión en el móvil.** Para el caso de la librería conviene que «¿lo tengo ya?» funcione sin cobertura: caché local en SQLite y sincronización al volver.
4. **Autenticación**, pensando ya en una futura versión pública multiusuario.
5. **Modelo de catálogos genéricos** (sección 6): ¿tabla común con campos por tipo en JSONB, o tablas por tipo?
6. **Alojamiento**: servidor en casa (homelab, otra idea que tengo) o VPS.

## 5. Modelo de datos inicial (libros)

Separar **obra/edición** (lo que es el libro) de **ejemplar** (la copia física o digital que tenemos). Así se resuelven de forma natural las «unidades», los dueños y los préstamos.

**Edición**
- Título, subtítulo, autores (varios), editorial, año, idioma.
- **ISBN** (10 y 13).
- Saga y número dentro de la saga.
- Temáticas y géneros, etiquetas.
- Portada: subida a mano o importada.
- **Enlaces a las fichas online** de Google Books y Open Library: guardar sus IDs y mostrar enlace directo.
- Campos específicos para libros de rol: sistema, edición del sistema, tipo (básico, suplemento, aventura…) y de qué libro depende.

**Ejemplar**
- Propietario (miembro de la familia).
- **Formato**: tapa dura (cartoné), tapa blanda (rústica), bolsillo, ebook… Un mismo libro puede tenerse en varios formatos.
- **Número de unidades**, si hay varias copias iguales.
- **Fecha de compra**.
- Ubicación física: «salón, estantería 2, balda 3».
- Estado de préstamo.

**Por usuario**
- Estado de lectura: pendiente, leyendo, leído o abandonado, con fechas.
- Valoración y **favoritos**.
- **Lecturas sin ejemplar**: un libro que me han prestado a mí se puede marcar como leído aunque ya lo haya devuelto y no esté en la biblioteca.

**Préstamos**
- Salientes: qué ejemplar, a quién, cuándo y cuándo volvió. Aviso si pasan X semanas. Historial completo.
- Entrantes: quién me lo prestó.

## 6. Más allá de los libros: catálogos por tipo

Motor común con fichas distintas según el tipo de catálogo:
- Libros (novela, rol…), cómics.
- Juegos de mesa (con datos de BoardGameGeek), juegos de ordenador, juegos de consola (con plataforma).

**Ludoteca**: la misma app podría servir a una biblioteca o a una asociación de juegos para prestar su colección a sus socios.

## 7. Permisos

- Una biblioteca compartida con varios miembros y roles, por ejemplo: administrador, editor y lector.
- Un niño de la casa puede ver el catálogo y marcar sus lecturas y favoritos, **pero no puede borrar**.

## 8. Formas de añadir libros

1. Alta manual.
2. **Escáner de código de barras (ISBN)** con la cámara del móvil. Busca en Google Books u Open Library y rellena los datos y la portada. Es lo más fiable. Google Books suele cubrir mejor los libros en español (conviene comprobarlo).
3. **Foto de la estantería**: enviar la imagen a la **API de Claude**, que identifica los lomos y devuelve una lista para confirmar. Ideal para la carga inicial.
4. Importar desde Calibre (su catálogo es SQLite) o desde CSV.

## 9. Ideas para después del MVP

- «¿Qué leo ahora?»: recomendaciones de Claude solo entre lo que hay en casa sin leer.
- Estantería visual con lomos.
- Sagas en orden: qué toca y qué tomo falta.
- Estadísticas de lectura por año y por persona.
- **Comunidad**: contactar con los amigos que te prestan libros, ver qué tienen y pedírselos. Una red de préstamo entre conocidos.
- Enlazar libros de rol con herramientas de rol.

## 10. Mercado (revisado el 24 de septiembre de 2026)

| Producto | Qué hace | Hueco |
|---|---|---|
| **Libib** | Cataloga libros, películas, música, videojuegos y juegos de mesa. Lo usan bibliotecas y colegios. Gratis hasta 5.000 elementos. | El préstamo y los varios gestores son de pago (Pro, 9 $/mes). Sin comunidad. Sin escritorio en Linux. |
| **Plumerie** | Biblioteca familiar compartida, ubicaciones, préstamos, escaneo de estantería. | Gratis solo hasta 100 libros; Plus cuesta 3,99 €/mes. Sin red social: en su propia comparativa dice que ninguna app tiene comunidad de préstamo ni registro de lo que te prestan a ti. |
| **CLZ** (Books, Comics, Games) | Una app de pago por cada tipo de colección. | Catálogos separados y de pago. |
| **Koillection** | Gestor de colecciones autoalojado y open source, hecho en **Symfony**. | Web; conviene mirarlo como referencia. |
| **Goodreads / StoryGraph** | Red social de lectura. | No gestionan préstamos ni la biblioteca física. |
| **ludoteca** (GitHub, Vue + Django) | Préstamo de juegos de mesa en convenciones. | Archivado en noviembre de 2025. |

**Hueco:** gratis u open source, **multicatálogo**, familia con permisos, **comunidad de préstamo entre amigos** y utilizable por **asociaciones y ludotecas**. Hay que validarlo más a fondo.

Fuentes: libib.com/pricing, plumerielibrary.com (incluida su comparativa de apps de préstamo), clz.com, github.com/benjaminjonard/koillection, github.com/fabiogasparferreira/ludoteca.

## 11. Cómo quiero trabajar

- Es un proyecto por diversión, sin prisa: prefiero hacerlo bien antes que correr, y que no se convierta en una obligación.
- Primero, un **plan de ejecución por fases**, cada una útil por sí sola.
- Propuesta de fases para discutir:
  1. Escritorio con Tauri: catálogo, alta manual, búsqueda por ISBN con portada y enlaces a las fichas.
  2. Servidor con base de datos y usuarios de la familia con permisos.
  3. APK Android con escáner de ISBN y «¿lo tengo ya?» sin conexión.
  4. Préstamos, lecturas por usuario y favoritos.
  5. Foto de la estantería con Claude, catálogos de juegos y cómics.
  6. Comunidad y versión pública.
- Explícame Rust y Tauri a medida que avancemos: es parte del objetivo.
