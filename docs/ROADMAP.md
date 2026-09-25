# Roadmap

> Plan en discusión. Se basa en [DECISIONS.md](DECISIONS.md). Cada fase es útil por sí sola y entrega **código y documentación** (D12).

## Visión general

| Fase | Nombre | Qué se puede hacer al terminarla |
|---|---|---|
| 0 | Esqueleto | Compilar una app vacía en Linux, Windows y Android, con CI |
| 1 | Catálogo en el escritorio | Catalogar mis libros en el escritorio, sin servidor |
| 2 | Móvil y sincronización local | «¿Lo tengo ya?» en la tienda; carga inicial con el escáner en ráfaga |
| 3 | Servidor y familia | La familia comparte biblioteca, con roles, desde cualquier lugar |
| 4 | Vida de la biblioteca | Lecturas, préstamos, deseos y regalos, sagas, estadísticas |
| 5 | Herramientas de estantería | Inventario y foto de balda |
| 6 | Multicatálogo | Juegos de mesa, cómics y videojuegos |
| 7 | Comunidad y versión pública | Red de préstamo entre amigos, ludotecas, repositorio público |

---

## Fase 0 — Esqueleto

El objetivo es eliminar riesgos técnicos antes de escribir funcionalidad: ver que la cadena de compilación funciona en las **tres** plataformas.

- Monorepo (D16) con licencias (D11), README en inglés y la estructura de `docs/`.
- App Tauri 2 + React + TypeScript + Vite + shadcn/ui (D15), con i18n en inglés y español (D17).
- `rusqlite` con una migración de prueba (D14).
- **Resolver antes el entorno de desarrollo en Linux** (D23) e instalar Rust, Node, Android Studio (SDK, NDK, JDK 17) y `gh`.
- Compilación y arranque en Linux y en Windows, y una **APK de depuración instalada en tu móvil**. Android es la parte más delicada de Tauri: mejor descubrir sus problemas el primer día.
- CI en GitHub Actions: lint, tests y compilación.
- **Rust que aprendes:** Cargo, *crates* y *workspaces*, estructura de un proyecto Tauri, comandos de Tauri (cómo llama la interfaz a Rust).
- **Hecho cuando:** una ventana «Hello, library» que lee un dato de SQLite en las tres plataformas, y la CI en verde.

**Estado (24/09/2026):**
- [x] Entorno en Distrobox (D23), documentado en `development/setup.md`.
- [x] Monorepo con *workspace* de Cargo, licencia AGPL, README y `AGENTS.md`.
- [x] Tauri 2 + React + TypeScript + Vite + Tailwind + shadcn/ui, con i18n en inglés y español y modo oscuro según el sistema.
- [x] `rusqlite` con migraciones y un comando que cuenta los arranques. Tests de Rust, Clippy y ESLint en verde.
- [x] App de escritorio funcionando en Linux.
- [x] APK de depuración compilado.
- [x] APK instalado y funcionando en un móvil real (moto g84 5G, Android 15).
- [x] Repo privado en GitHub y CI en verde: lint, tests y compilación en Linux y Windows.
- [ ] Probar en Windows a mano.

## Fase 1 — Catálogo en el escritorio (modo local)

- *Crate* de dominio (`domain`), separado de Tauri: ediciones, ejemplares, ubicaciones, formatos (D08).
- **Registro de operaciones desde el principio**: toda escritura es una operación con UUID v7, identificador de dispositivo y marca HLC (D05, D13). Todavía no se sincroniza, pero así el formato ya queda probado.
- Primera versión de la especificación de operaciones en `spec/`, con sus casos de conformidad (D06).
- Alta manual; búsqueda por ISBN en Google Books y Open Library detrás de un «proveedor de metadatos» (D18); portadas; enlaces a las fichas.
- Campo de ISBN que acepta «número + Intro» y un modo ráfaga en el escritorio: sirve para teclear rápido y, de paso, **soporta cualquier lector de códigos USB** (D19, vía 2). Se prueba tecleando, sin comprar nada.
- Listado, búsqueda y filtros.
- **Historial** a partir del registro de operaciones.
- **Exportación e importación** en JSON y CSV: copia de seguridad y garantía de que el usuario no queda atado a la app.
- **Rust que aprendes:** *ownership* y *borrowing*, `Result` y el manejo de errores, *traits* (el proveedor de metadatos), *serde*, HTTP con `reqwest`, tests.
- **Hecho cuando:** catalogo mis libros en el escritorio sin servidor. Es la primera versión útil del modo local open source.

**Hitos** (cada uno se puede ver y probar):

| Hito | Qué se construye | Qué se ve al terminar |
|---|---|---|
| 1.1 Dominio y alta manual | *Crate* `domain`; esquema obra/edición/ejemplar (D24); alta manual y listado | Doy de alta un libro a mano y aparece en la lista |
| 1.2 Registro de operaciones | Toda escritura pasa a ser una operación (UUID v7, dispositivo, HLC); especificación en `spec/` y primeros casos de conformidad | Lo mismo, pero cada cambio queda registrado; historial de un libro |
| 1.2b Editar y borrar | Editar la ficha y los autores; añadir, editar y borrar ejemplares; borrar libros | Corrijo lo que tecleé mal y quito lo que ya no está |
| 1.3 Búsqueda por ISBN | Proveedores de Google Books y Open Library detrás de un *trait*; portadas guardadas en local | Tecleo un ISBN y se rellena la ficha con su portada |
| 1.4 Modo ráfaga | «ISBN + Intro» encadenado; sirve para lectores USB | Doy de alta una pila de libros seguidos |
| 1.5 Buscar y filtrar | Texto completo con FTS5; filtros por formato, ubicación y propietario | Encuentro cualquier libro al instante |
| 1.6 Exportar e importar | JSON completo (copia de seguridad) y CSV | Saco y recupero mis datos |

**Estado:**
- [x] 1.1 Dominio y alta manual. *Crate* `domain` (ISBN con dígito de control y paso de ISBN-10 a ISBN-13, validación que devuelve todos los errores a la vez), esquema obra/edición/ejemplar, alta manual con varios autores y listado. Un ISBN repetido añade un ejemplar a la edición existente.
- [x] 1.2 Registro de operaciones. Operaciones como cambios de entidad con etiqueta de intención (D25), reloj HLC, identificador de dispositivo, registro `operations` aplicado en la misma transacción, especificación en `spec/` (JSON Schema y 18 casos de conformidad, licencia MIT) e historial de cada libro. Detalles en [architecture/operations.md](architecture/operations.md).
- [x] 1.2b Editar y borrar (añadido al plan el 25/09/2026). Ficha del libro con pestañas; al editar, cada operación solo lleva los campos que han cambiado, y editar sin cambios no registra nada. Borrar un libro elimina la edición, sus ejemplares y, si era la única edición, la obra; autores y propietarios se conservan.
- [x] 1.3 Búsqueda por ISBN. Open Library y Google Books a la vez (D26), con clave de Google opcional en Ajustes; los fallos de un proveedor son avisos. Portadas descargadas y guardadas por hash (D27) y mostradas en la tabla y la ficha; enlaces a las fichas externas. Pruebas reales contra los servicios con `cargo test -p library-app -- --ignored live_`.
- [x] 1.4 Alta rápida. Cola de ISBN con datos de ejemplar fijados por sesión; guarda lo encontrado al momento, no duplica lo que ya existe y deja pendiente lo no encontrado, con un campo para el título (D28). Funciona con teclado y con lectores de códigos USB.
- [x] 1.5 Buscar y filtrar. Búsqueda de texto completo (FTS5) sin acentos, por prefijos, por editorial y por ISBN en cualquier formato; filtros por formato, propietario y ubicación; editoriales como catálogo (D29); autocompletado de autores, editoriales, propietarios y ubicaciones. Las ubicaciones siguen siendo texto libre: la jerarquía salón → estantería → balda queda para más adelante.
- [x] 1.6 Exportar e importar. Copia de seguridad en ZIP (datos, portadas e historial) que se restaura sobre un catálogo vacío como una sola operación; CSV de una fila por ejemplar, exportable e importable con informe de filas rechazadas (D30).

**Mejoras pedidas tras probar el 1.1** (se harán sin prisa, junto al hito que las toque):
- [x] Autocompletar autores al escribir, a partir de los ya catalogados (1.5).
- [x] Editoriales como catálogo propio, con autocompletado (1.5).
- [x] Rellenar datos y autores consultando catálogos online (Google Books, Open Library) (1.3).

## Fase 2 — Móvil y sincronización local

- APK Android con escáner de ISBN.
- **Escáner en ráfaga** para la carga inicial (D19, vía 1).
- **«¿Lo tengo ya?»**: escanear y ver al momento si lo tenemos, de quién es y en qué formato.
- **Lista de deseos básica**: «no lo tengo → a deseos», desde la misma pantalla.
- Emparejamiento por QR y sincronización por la red local (D13):
  - **v1:** copia completa del escritorio al móvil.
  - **v2:** en los dos sentidos, por operaciones (HLC, el último en escribir gana campo a campo, lápidas), con descubrimiento por mDNS al llegar a casa.
- **Rust que aprendes:** red (TCP y TLS con una clave fijada por el QR), concurrencia, plugins de Tauri en móvil.
- **Hecho cuando:** en una tienda sin cobertura, escaneo un libro y sé si ya lo tengo; al volver a casa, todo se sincroniza.

**Hitos** (diseño en [architecture/sync.md](architecture/sync.md)):

| Hito | Qué se construye | Quién lo prueba |
|---|---|---|
| 2.0 Motor de sincronización | Operaciones replicables y operación inicial (D31); aplicar operaciones recibidas campo a campo, con lápidas (D32) y duplicados (D33); vector de versiones | Tests que simulan dos y tres dispositivos |
| 2.1 Sincronización por la red local | Servidor HTTPS en el escritorio, certificado fijado, emparejamiento con código (D34) | Dos instancias de la app en el mismo equipo |
| 2.2 Interfaz móvil y escáner | Pantallas adaptadas al móvil, escáner de ISBN con la cámara, «¿lo tengo ya?», lista de deseos básica | En el móvil |
| 2.3 Móvil emparejado | Emparejar escaneando el QR del escritorio; reencontrar al otro dispositivo en la wifi de casa (D37) | En el móvil |

**Estado:**
- [x] 2.0 Motor de sincronización. Operaciones replicables con versión de esquema; operación inicial `sync.baseline` por dispositivo; vector de versiones; aplicación de operaciones recibidas campo a campo, con lápidas y puntos de restauración (*savepoints*) para saltar cambios imposibles sin romper la sincronización; duplicados permitidos; filtros y sugerencias por nombre. 12 tests simulan dos y tres dispositivos y comprueban que acaban con el mismo estado.
- [x] 2.1 Sincronización por la red local. Servidor HTTPS (axum, rustls con ring) que escucha solo cuando hace falta; certificado propio fijado por la huella del código; emparejamiento con secreto de un solo uso; sincronización en los dos sentidos; diálogo «Dispositivos» con código en texto y QR; sincronización automática al abrir la app. Los tests emparejan y sincronizan por TLS real.
- [x] 2.2 Interfaz móvil y escáner. «¿Lo tengo ya?» consultando solo la base de datos local (funciona sin cobertura), con escáner de cámara en el móvil (D35) y búsqueda online opcional; lista de deseos sincronizada (D36) con «Comprado: añadir al catálogo»; vista en tarjetas y cabecera compacta en pantallas estrechas. Compila para Android con permiso de cámara; falta probarlo en el móvil.
- [x] 2.3 Móvil emparejado. Emparejar escaneando el QR con la cámara; sincronización al abrir la app y al volver a ella; reencontrar al otro dispositivo en la subred si cambió de dirección (D37, en lugar de mDNS). **Fase 2 completa en código**; pendiente de probar en el móvil (`docs/testing/fase-2.md`).

## Pendiente antes de seguir con el diseño de interfaz

- **Validar el frontend en el dispositivo real, sobre todo en Android** (`docs/testing/hito-1.md` y `docs/testing/fase-2.md`), antes de diseñar más pantallas.
- **Sincronizar con Claude Design** (`/design-sync`): requiere que el usuario autorice antes con `/design-login`. El brief y el catálogo de errores están en `docs/design/`. Importación completa y de alta fidelidad, ya decidida.
- Mejorar los mensajes de error siguiendo `docs/design/errors.md`, cuando llegue el diseño.

## Fase 3 — Servidor y familia

- Servidor Symfony + PostgreSQL en Docker (nginx + PHP-FPM, igual que en desarrollo, D48), desplegado en el VPS (D06, D07, D10).
- Autenticación sencilla con token para sincronizar (D09). Roles: administrador, editor y lector.
- Sincronización con el servidor usando el mismo formato de operaciones; el servidor reaplica cada operación con autoridad (D05).
- **Solicitudes**: lo que un rol no puede hacer (tu hijo borrando) se convierte en una petición que aprueba un administrador.
- **Invitar a familiares con un QR** (URL del servidor + token de invitación).
- Pruebas de conformidad compartidas entre Rust y PHP en la CI.
- **Hecho cuando:** la familia comparte biblioteca desde el escritorio y el móvil, cada uno con su rol, y los cambios sin conexión se sincronizan con el servidor.

**Hitos** (decisiones D38–D40; detalle y preguntas abiertas en [plans/fase-3.md](plans/fase-3.md)):

| Hito | Qué se construye | Cómo se prueba |
|---|---|---|
| 3.0 Esqueleto del servidor ✅ | Symfony + PostgreSQL en Docker en `server/`; los casos de conformidad de `spec/` en PHPUnit y en la CI ([guía](development/server.md)) | Tests, CI |
| 3.1 Aplicar operaciones en PHP | El mismo motor de fusión que en Rust; casos de conformidad **de fusión** compartidos por Rust y PHP | Casos compartidos |
| 3.2 Cuentas, bibliotecas, roles e invitaciones | Usuarios, bibliotecas (D38), membresías con rol, token por dispositivo, invitación por QR | Tests de API |
| 3.3 La app habla con el servidor | «Conectar a un servidor»; sincronización con él como única vía (D39); permisos en la app y en el servidor (D40) | Extremo a extremo con el servidor en Docker |
| 3.4 Solicitudes y despliegue | Solicitudes que aprueba un administrador; imagen Docker para el VPS | En el VPS |

## Fase 4 — Vida de la biblioteca

- Estado de lectura por usuario (pendiente, leyendo, leído, abandonado) con fechas, valoraciones y favoritos.
- **Lecturas sin ejemplar**: libros que te han prestado a ti.
- **Préstamos** salientes y entrantes, con historial y avisos pasadas X semanas.
- **Lista de deseos familiar con reserva de regalos**, oculta para quien recibe el regalo.
- **Ejemplar firmado o dedicado**, con foto.
- **Relaciones entre ítems**: sagas en orden («qué toca y qué tomo falta»), dependencias entre libros de rol.
- **Estadísticas** de lectura por año y por persona.
- Exportación como lista imprimible y ZIP de portadas.

## Fase 5 — Herramientas de estantería

- **Modo inventario**: balda a balda, escaneando o con una foto de la balda y OCR local contra el propio catálogo (D19, vía 3). Detecta lo que falta y lo que está mal colocado.
- **Foto de balda para la carga inicial**: primero la prueba de concepto con OCR local; si no basta, IA en la nube como opción (D19, vía 4).
- *Aunque la fase sea la 5, la prueba de concepto del OCR puede hacerse antes como experimento, cuando apetezca: no bloquea nada.*

## Fase 6 — Multicatálogo

- Juegos de mesa (BoardGameGeek), cómics, videojuegos (con plataforma), **películas y series (TMDB)**: nuevas tablas `*_details` (D08) y nuevos proveedores de metadatos (D18, D21).
- Campos personalizados por catálogo.
- Fuentes de metadatos enchufables o scrapers configurables, para lo que no tiene ISBN.

## Fase 7 — Comunidad y versión pública

- Visibilidad por niveles (privado, familia, amigos, público).
- Red de préstamo entre conocidos: ver qué tienen tus amigos y pedírselo.
- Ludotecas y asociaciones: prestar su colección a sus socios.
- Revisión de la autenticación para el público (D09).
- **Landing y sitio de documentación** (D22), a partir del Markdown acumulado en el repositorio.
- Publicación del repositorio y del servicio alojado. *Abrir el repositorio podría adelantarse, por ejemplo al terminar la Fase 2, cuando el modo local ya sea útil.*

## Más adelante (sin fase)

- **Dispositivo principal en modo local** (D13): árbitro y permisos para familias sin servidor.
- «¿Qué leo ahora?»: recomendaciones entre lo que hay en casa sin leer (opcional, detrás de D18).
- Estantería visual con lomos.
- IA de varios proveedores (Anthropic y compatibles con OpenAI, incluidos modelos locales) para reconocer imágenes (D20).
- Enlace con rpgutilities.
- Versión web e iOS.
