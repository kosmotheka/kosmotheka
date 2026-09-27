# Pantallas de Claude Design

Descargadas el 25/09/2026 (y las de bibliotecas el 26/09/2026) del proyecto Library — sistema de diseño (carpeta `templates/`). Son la referencia visual de la Fase 3 del [ROADMAP](../../ROADMAP.md) para construir la interfaz.

| Carpeta | Qué hay |
|---|---|
| `mobile/` | 10 pantallas de móvil (390 × 844), en claro y oscuro: catálogo, selector de biblioteca, menú lateral, añadir (buscar, resultados, a mano), ficha, ¿Lo tengo? (lo tienes / no lo tienes sin red), Me falta |
| `desktop/` | 6 pantallas de escritorio (1280 × 800), en claro y oscuro: catálogo, ficha en panel, añadir en diálogo, serie con huecos, sin resultados, biblioteca vacía |
| `mobile-libraries/` | Fase 4, móvil, claro y oscuro (descargadas el 26/09/2026, [encargo](../encargo-fase-4.md)): selector con varias bibliotecas, nueva (y nombre repetido), ajustes, borrar (con copia), salir (y el último administrador, para la Fase 5), emparejar eligiendo bibliotecas, dispositivos y qué va a cada uno, mover un libro, buscar en todas y «¿Lo tengo?» en varias |
| `desktop-libraries/` | Lo mismo en escritorio. Cambia la barra lateral: el selector es un menú desplegable, «Ajustes de la biblioteca» entra en su sección y «Compartir y dispositivos» baja al pie, porque vale para todas; la tabla de dispositivos tiene una columna por biblioteca |
| `mobile-people/` | Fase 5, móvil, claro y oscuro (descargadas el 27/09/2026, [encargo](../encargo-fase-5.md)): personas y permisos (quien administra, quien no, biblioteca sin permisos y paso de activar con «¿de quién es cada dispositivo?»), ficha de la persona (y la última administradora), emparejar eligiendo persona o creando una nueva, dispositivos con su persona, peticiones (pendientes, ya resueltas por otra administradora, ya no aplicables), mis peticiones, pedir en lugar de hacer, cambio que pasó a petición y dispositivo perdido |
| `desktop-people/` | Lo mismo en escritorio. Falta la vista de conjunto `DesktopPeople.dc.html`: en Design llega cortada (sin el final ni el script), así que no se guardó |
| `*-overview.png` | Captura de todas las pantallas de cada carpeta (las de bibliotecas, a mitad de tamaño) |

- Cada `*.dc.html` es una pieza (pantalla o componente como `AppBar`, `BottomNav`, `Sidebar`, `Cover`). `MobileScreens.dc.html` y `DesktopScreens.dc.html` las reúnen todas.
- `app-tokens.css` propone **tokens nuevos**: `--success`, `--warning`, `--wish` (con sus variantes `-soft`) y `--scrim`, en claro y oscuro.
- `support.js` y `ds-base.js` son el motor de Claude Design para mostrarlas; no se editan.

**Para verlas en local:** `ds-base.js` carga los estilos y componentes desde `../..`. Copia las carpetas a `ds-bundle/templates/mobile-screens` y `ds-bundle/templates/desktop-screens` (tras una sincronización con `/design-sync`) y sirve `ds-bundle/` con cualquier servidor estático.

En las carpetas de bibliotecas, `ds-base.js` y `support.js` son copias de las de la primera descarga (el motor de Design no cambia) y `pair-qr.svg` es un QR de ejemplo sin los metadatos de procedencia que añade Design.
