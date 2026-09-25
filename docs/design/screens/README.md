# Pantallas de Claude Design

Descargadas el 25/09/2026 del proyecto Library — sistema de diseño (carpeta `templates/`). Son la referencia visual de la Fase 3 del [ROADMAP](../../ROADMAP.md) para construir la interfaz.

| Carpeta | Qué hay |
|---|---|
| `mobile/` | 10 pantallas de móvil (390 × 844), en claro y oscuro: catálogo, selector de biblioteca, menú lateral, añadir (buscar, resultados, a mano), ficha, ¿Lo tengo? (lo tienes / no lo tienes sin red), Me falta |
| `desktop/` | 6 pantallas de escritorio (1280 × 800), en claro y oscuro: catálogo, ficha en panel, añadir en diálogo, serie con huecos, sin resultados, biblioteca vacía |
| `*-overview.png` | Captura de todas las pantallas de cada carpeta |

- Cada `*.dc.html` es una pieza (pantalla o componente como `AppBar`, `BottomNav`, `Sidebar`, `Cover`). `MobileScreens.dc.html` y `DesktopScreens.dc.html` las reúnen todas.
- `app-tokens.css` propone **tokens nuevos**: `--success`, `--warning`, `--wish` (con sus variantes `-soft`) y `--scrim`, en claro y oscuro.
- `support.js` y `ds-base.js` son el motor de Claude Design para mostrarlas; no se editan.

**Para verlas en local:** `ds-base.js` carga los estilos y componentes desde `../..`. Copia las carpetas a `ds-bundle/templates/mobile-screens` y `ds-bundle/templates/desktop-screens` (tras una sincronización con `/design-sync`) y sirve `ds-bundle/` con cualquier servidor estático.
