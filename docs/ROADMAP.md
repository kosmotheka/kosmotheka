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

## Fase 3 — Servidor y familia

- Servidor Symfony + PostgreSQL en una imagen Docker (el modelo de FrankenPHP de Koillection), desplegado en el VPS (D06, D07, D10).
- Autenticación sencilla con token para sincronizar (D09). Roles: administrador, editor y lector.
- Sincronización con el servidor usando el mismo formato de operaciones; el servidor reaplica cada operación con autoridad (D05).
- **Solicitudes**: lo que un rol no puede hacer (tu hijo borrando) se convierte en una petición que aprueba un administrador.
- **Invitar a familiares con un QR** (URL del servidor + token de invitación).
- Pruebas de conformidad compartidas entre Rust y PHP en la CI.
- **Hecho cuando:** la familia comparte biblioteca desde el escritorio y el móvil, cada uno con su rol, y los cambios sin conexión se sincronizan con el servidor.

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
