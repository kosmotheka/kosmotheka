# Kosmotheka · marca

Símbolo: lienzo 2B (un lienzo añil con una estrella de cinco puntas arriba a la derecha que se deshace en píxeles, en rejilla de 8 × 8, hacia la esquina de abajo a la izquierda). Generador: `../busqueda/gen-final.js`. Hoja de uso: `../Marca.dc.html`. Histórico de conceptos: `../LogoConcepts.dc.html`. Comparativa tipográfica: `../Typography.dc.html`.

Todas las piezas son SVG con viewBox, de una sola silueta (sin máscaras, degradados ni imágenes) y con el texto en trazos.

## Colores
- Añil `#4b53c2` · `oklch(0.50 0.17 275)`: color del símbolo, acento en modo claro y fondo de og-image, github-social y feature-graphic (6,3:1 sobre blanco; texto blanco encima 6,3:1).
- Añil claro `#8d9df5` · `oklch(0.72 0.13 275)`: acento en modo oscuro (7,8:1 sobre `#0a0a0a`).
- Tinta `#0a0a0a`, papel `#ffffff` (fondo de los iconos de la app: el símbolo va en añil sobre blanco).

## Tipografía
Figtree SemiBold (600), de Erik Kennedy, con licencia SIL OFL 1.1 y un tracking de −1,5 % en el logotipo. En la app, Figtree para toda la interfaz (sustituye a Geist).

## Archivos
- `marca/`: `simbolo-{color,negro,blanco}`, `logotipo-{color,negro,blanco}`, `horizontal-{color,color-oscuro,negro,blanco}` y `apilada-{…}`.
- `icono/`
  - `icon-fullbleed` (1024, Google Play).
  - `adaptive-background`, `adaptive-foreground` y `adaptive-monochrome` (108; el símbolo mide 52 × 52, y con sus esquinas redondeadas cabe en el círculo de 66; fondo blanco).
  - `icon-desktop` (1024, cuadrado redondeado con un 6 % de margen).
  - `splash` y `splash-oscuro` (240; el símbolo cabe en el círculo de 160). Fondo de la pantalla de inicio: `#ffffff` en claro y `#0a0a0a` en oscuro.
- `web/`
  - `favicon` (16 × 16, dibujado sobre la rejilla de píxeles) y `favicon-32` (32 × 32, rejilla de 4 × 4); los dos cambian a añil claro con `prefers-color-scheme: dark`.
  - `icon-maskable` (512, fondo blanco, símbolo de 320 dentro del círculo del 80 %).
  - `og-image` (1200 × 630) y `github-social` (1280 × 640).
- `play/capturas/`: cinco capturas de móvil para Google Play (1080 × 1920, 9:16), hechas con `?mock` en un navegador sin ventana a 360 × 640 con escala 3.
- `play/`: `feature-graphic` (1024 × 500, SVG y PNG) e `icon-512.png` (el icono de la ficha de Google Play, sacado de `icono/icon-fullbleed`). Los PNG se generan con `rsvg-convert` en Docker.

## Comprobaciones
Símbolo 2B buscado en registros de marcas: sin coincidencias. Falta confirmar el nombre en EUIPO, OEPM y en las tiendas de apps, si no se ha hecho ya.
