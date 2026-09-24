# Koillection: análisis como referencia

> Analizado el 24/09/2026 a partir del código fuente (último commit del 03/09/2026, versión 1.8.4). Repositorio: github.com/koillection/koillection.

## Qué es

Un gestor de colecciones **web y autoalojado**, pensado para cualquier cosa: libros, DVD, sellos, juegos… Es **genérico a propósito**: no trae descarga de metadatos, cada usuario define sus campos.

## Stack y proyecto

| Aspecto | Qué usa |
|---|---|
| Backend | PHP 8.5, Symfony 8, Doctrine ORM 3, API Platform 4 (API REST con OpenAPI) |
| Frontend | Plantillas Twig (web tradicional, sin app nativa ni modo sin conexión) |
| Base de datos | PostgreSQL, MySQL o MariaDB |
| Despliegue | **Una sola imagen Docker** basada en FrankenPHP, más la base de datos, en un `docker-compose` de unas 45 líneas |
| Licencia | MIT |
| Comunidad | Un mantenedor principal y algunos colaboradores. Traducciones en Crowdin, documentación en la wiki de GitHub, demo desechable en Gitpod y un CHANGELOG que cita a cada colaborador |

## Modelo de datos

- **Collection**: árbol de colecciones anidadas (una colección puede tener subcolecciones).
- **Item**: nombre, cantidad, imagen, etiquetas, **ítems relacionados** y visibilidad.
- **Datum**: modelo **EAV** (entidad-atributo-valor). Cada campo es una fila con tipo, etiqueta y valor. Hay 17 tipos: texto, fecha, valoración, precio, enlace, lista, lista de opciones, imagen, fichero, **firma**, vídeo, sección…
- **Template + Field**: plantillas de campos que se aplican a los ítems de una colección («todo libro tiene autor, editorial…»).
- **Loan**: ítem, a quién (texto libre), fecha de préstamo y de devolución. Nada más.
- **Visibilidad** en tres niveles (`public`, `internal`, `private`), por colección, por ítem e incluso **por campo**. Cada usuario tiene un perfil público en `/user/{username}`.
- **Usuarios**: roles `user` y `admin`, cuota de disco por usuario. **Cada usuario tiene sus propias colecciones: no hay biblioteca compartida entre varios usuarios.**

## Funcionalidades

- **Inventario**: una foto de la colección convertida en lista de comprobación, para ir marcando lo que se encuentra físicamente.
- **Listas de deseos**, con precio, URL e imagen.
- **Scrapers configurables por el usuario**: extraen datos de una web mediante XPath (HTML) o rutas JSON, con Chrome sin interfaz si hace falta.
- **Firmas**: galería de autógrafos.
- **Álbumes** de fotos.
- **Estadísticas** y gráficas.
- **Historial** de creaciones y borrados.
- **Búsqueda avanzada** con bloques y filtros, y búsquedas guardadas.
- Categorías de etiquetas y listas de opciones reutilizables.
- Configuración de vista por colección: cuadrícula o lista, columnas y orden.
- **Exportación**: CSV, lista imprimible y ZIP con las imágenes.
- Temas con CSS personalizado, miniaturas en AVIF o WebP y un endpoint de métricas.

## Ideas que nos llevamos

| Idea | Cómo encaja en nuestro proyecto |
|---|---|
| **Modo inventario** | Con nuestro escáner es mucho mejor: vas balda a balda escaneando y la app confirma las ubicaciones y detecta lo que falta o está mal colocado. |
| **Lista de deseos** | Encaja con el caso estrella: en la tienda, si no lo tienes, lo añades a deseos. Con la familia se convierte en **ideas de regalo**, y alguien puede «reservar» un regalo sin que lo vea el destinatario. |
| **Firmado o dedicado** | Un atributo del **ejemplar** (no de la edición), con foto de la dedicatoria. |
| **Ítems relacionados** | Generaliza lo que ya teníamos (sagas, dependencias entre libros de rol) en un tipo de relación. |
| **Visibilidad por niveles** | La base para la fase de comunidad: qué ven los amigos y qué es privado. |
| **Exportación** (CSV, imprimible, imágenes) | Clave en local-first: **nada de encierro**. También sirve de copia de seguridad y, en versión imprimible, para el seguro del hogar. |
| **Historial** | Con nuestro registro de operaciones (D05) lo obtenemos gratis. |
| **Scrapers configurables** | Para lo que no tiene ISBN (PDF de rol, ediciones raras). A largo plazo, «fuentes de metadatos» enchufables. |
| **Campos personalizados** | Nuestros tipos son fijos (D08). Más adelante se podrían añadir unos pocos campos libres por catálogo, en tipo EAV, sin tocar los tipos base. |
| **Proyecto open source** | Una sola imagen Docker, traducciones en Crowdin, demo desechable, CHANGELOG que cita a los colaboradores. Es un buen modelo para nosotros. |
| **Traducciones desde el principio** | Koillection nació multidioma. Nosotros también deberíamos (D17). |
| **Aviso de copias de seguridad** | Su README insiste en hacer copia antes de actualizar, porque las migraciones pueden romper datos. En local-first eso nos afecta en cada dispositivo. |

## En qué somos distintos

| | Koillection | Nuestro proyecto |
|---|---|---|
| Enfoque | Genérico y configurable (sirve para sellos) | Especializado por tipo (libros, juegos): menos configuración, más automático |
| Metadatos | Ninguno de serie: los defines tú o los extrae un scraper | ISBN o código de barras → Google Books, Open Library, BoardGameGeek |
| Compartir | Cada usuario tiene lo suyo | **Biblioteca familiar compartida con roles** |
| Modelo | Ítem con cantidad | **Edición separada del ejemplar**: formatos, varios dueños |
| Lectura | No existe | Estado de lectura por usuario, favoritos, lecturas sin ejemplar |
| Préstamos | Texto libre, sin avisos | Préstamos salientes y entrantes, avisos, contactos y, más adelante, red de préstamo |
| Plataformas | Solo web | Escritorio y Android nativos, **sin conexión**, escáner, modo local sin servidor |
| Alta masiva | Manual | Foto de la estantería con Claude |

**Conclusión:** no compite con nuestra idea, la complementa. Koillection gana en flexibilidad; nosotros apostamos por **automatizar, compartir en familia, funcionar sin conexión y prestar**. Su código es una buena referencia de Symfony para el servidor, en particular su API con API Platform y su despliegue con FrankenPHP.
