# Iconos y pantalla de inicio

Las piezas de la marca están en `brand/` (ver `brand/README.md`). La app usa copias de ellas, sin los metadatos de procedencia (C2PA) que traen los SVG, que no hacen falta dentro de la app.

## Qué va dónde

| Pieza | De | En la app |
|---|---|---|
| Iconos de escritorio (Linux, Windows, macOS) y PNG de Android antiguos | `brand/icono/icon-desktop.svg`, pasado a PNG de 1024 | `app/src-tauri/icons/` y `res/mipmap-*/`, generados con `tauri icon` |
| Icono adaptativo de Android 8+ | `adaptive-foreground.svg`, fondo blanco | `res/drawable/ic_launcher_foreground.xml`, `res/values/ic_launcher_background.xml` y `res/mipmap-anydpi-v26/` |
| Icono temático de Android 13+ (monocromo) | `adaptive-monochrome.svg` | `res/drawable/ic_launcher_monochrome.xml` |
| Pantalla de inicio de Android 12+ | `splash.svg` y `splash-oscuro.svg`; fondo `#ffffff` y `#0a0a0a` | `res/drawable/splash_icon.xml`, `res/drawable-night/splash_icon.xml` y `res/values-v31/`, `res/values-night-v31/themes.xml` |
| Favicon (solo en el navegador, en desarrollo) | `brand/web/favicon.svg`, con el cambio a añil claro en modo oscuro | `app/public/favicon.svg` |

`res/` es `app/src-tauri/gen/android/app/src/main/res/`. Los de Android son vectores (`<vector>`): el `d` de cada trazado del SVG va tal cual a `android:pathData`, con `android:fillType="evenOdd"`.

## Regenerarlos si cambia la marca

1. Quitar el bloque `<metadata>…</metadata>` de los SVG que se vayan a usar.
2. Pasar `icon-desktop.svg` a un PNG de 1024 × 1024 con fondo transparente (por ejemplo, abriéndolo en Chromium y haciendo una captura sin fondo).
3. En el contenedor: `cd app && npm run tauri icon <ruta del PNG>`. Borra después `app/src-tauri/icons/ios/`: no hay app de iOS (D63).
4. **`tauri icon` sobrescribe el icono adaptativo** con uno propio hecho de PNG, sin monocromo. Hay que devolverle el nuestro: `git checkout -- app/src-tauri/gen/android/app/src/main/res/mipmap-anydpi-v26 app/src-tauri/gen/android/app/src/main/res/values/ic_launcher_background.xml`, o rehacerlo si cambia el diseño.
5. Si cambian los vectores, copiar el nuevo `d` en los `.xml` de `res/drawable*/`.
6. Si al compilar el APK sale «resource drawable/ic_launcher_foreground not found», es la caché de Gradle: borra `app/src-tauri/gen/android/app/build` y vuelve a compilar.

## Colores

El añil de la marca es el acento de la interfaz (`--primary`, `--ring` y `--sidebar-primary` en `app/src/index.css`): `oklch(0.50 0.17 275)` en claro, con texto blanco, y `oklch(0.72 0.13 275)` en oscuro, con texto casi negro. Solo para el botón principal, los enlaces, el foco y la selección; nunca como fondo de una pantalla entera.
