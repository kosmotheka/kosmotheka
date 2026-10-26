# Actualizaciones automáticas en el escritorio

> 27/10/2026. Estudio para el punto 6 de «Tras el lanzamiento» del [ROADMAP](../ROADMAP.md). Sin código: es para decidir con el maintainer. Fuente principal: la documentación del plugin `updater` de Tauri 2 (v2.tauri.app/plugin/updater).

## El problema

En el móvil, Google Play actualiza la app solo. En el ordenador, quien instala Kosmotheka desde la web se queda en esa versión hasta que vuelve a la web, descarga la nueva e instala encima. Nadie le avisa. Con una versión por semana durante la prueba, en pocas semanas habrá ordenadores varias versiones por detrás, y eso pesa más cuando dos dispositivos necesitan la misma versión para algo (como emparejar con el número de la 0.7.0).

## Cómo se instala hoy en el escritorio

| Forma de instalar | Quién la usa | ¿Se actualiza sola hoy? |
|---|---|---|
| Windows (`Kosmotheka-windows-x64-setup.exe`, NSIS) | Casi todo el que use Windows | No |
| Linux AppImage | Quien no quiere instalar nada | No |
| Linux `.deb` / `.rpm` | Ubuntu, Debian, Fedora… | No (el gestor de paquetes no conoce nuestras versiones) |
| Snap Store | Ubuntu, con un clic | **Sí**, la tienda se encarga |
| Android, Google Play | | **Sí** |

## Qué ofrece el plugin `updater` de Tauri

- **Busca** si hay una versión nueva, la **descarga**, la **instala** y la app se **reinicia**. Todo desde la propia app, con su API (`check`, `downloadAndInstall` y `relaunch`, este último del plugin `process`).
- **Solo sabe instalar en Windows (NSIS o MSI) y en la AppImage**, y en macOS. **No actualiza el `.deb` ni el `.rpm`**: esos los instala el sistema y los tiene que actualizar el sistema.
- **Cada actualización va firmada, y no se puede desactivar.** Se genera un par de claves propio (`tauri signer generate`). La clave pública va dentro de la app (`plugins.updater.pubkey`) y la privada firma cada versión al compilar (`TAURI_SIGNING_PRIVATE_KEY`, con su contraseña). La app solo instala lo que esa clave firmó, así que alguien que se colara en la descarga no podría meter otra cosa. **Si se pierde la clave privada, las instalaciones existentes ya no pueden actualizarse**: tendrían que descargar e instalar a mano una vez.
- **Dónde mira:** una dirección fija que devuelve un pequeño `latest.json` (versión, notas y, por cada sistema, el enlace y la firma). Con GitHub Releases vale `https://github.com/kosmotheka/kosmotheka/releases/latest/download/latest.json`. Exige HTTPS.
- **Requisitos:** Rust 1.90 o posterior (tenemos 1.98) y un permiso más en la app (`updater:default`).

## Cómo encajaría con lo que ya tenemos

- **Las versiones salen como borrador** y el maintainer las publica a mano (D74). Eso viene muy bien: `releases/latest` solo apunta a una versión publicada, así que **nadie recibe una actualización hasta que el maintainer pulsa «Publish»**. No habría un botón nuevo que vigilar.
- **Los nombres de los ficheros ya son fijos** (`Kosmotheka-windows-x64-setup.exe`, `Kosmotheka-linux-x86_64.AppImage`), lo que simplifica el `latest.json`.
- **Nuestro workflow no usa la acción oficial de Tauri** (que genera el `latest.json`), sino `npm run tauri build` y `gh release create`. Habría que añadir tres cosas: activar `bundle.createUpdaterArtifacts` para que salgan las firmas (`.sig`), un paso que escriba el `latest.json` con esas firmas, y subir las dos cosas al borrador.
- **La clave privada** iría a los secretos del entorno `release` de GitHub, como la de Android, con copia fuera del repositorio. Es una clave nueva y distinta de la firma de Windows (SignPath, que sigue pendiente). Firmar el instalador de Windows quita el aviso de SmartScreen; esta clave solo dice a la app «esta actualización es de verdad».
- **Para cada forma de instalar, lo que haría la app:**
  - Windows y AppImage: «Hay una versión nueva» → «Actualizar» → se descarga, se instala y se reinicia.
  - `.deb` y `.rpm`: solo el aviso, con un enlace a la página de descargas. Una app no debe tocar lo que instaló el sistema.
  - Snap: nada. La tienda ya la actualiza y la carpeta de la snap no se puede escribir. La app sabe que es una snap por la variable `SNAP`, y que es una AppImage por `APPIMAGE`.

## Lo que sale del dispositivo

Buscar actualizaciones es **una petición a GitHub**: GitHub ve la dirección IP del ordenador y sabe que se ha descargado el `latest.json`, sin nada más (ni cuenta, ni bibliotecas). Es algo nuevo que sale del dispositivo, así que, por la regla de las novedades y por D91:

- va en «Qué sale de tu dispositivo» y en la política de privacidad;
- lleva su interruptor («Buscar actualizaciones», en Ajustes);
- **por decidir:** encendido por defecto (lo normal en las apps de escritorio; se explica en la bienvenida y en las novedades) o apagado hasta que la persona lo active.

## Opciones

| Opción | Qué hace | A favor | En contra |
|---|---|---|---|
| A · Solo avisar | La app mira la última versión publicada (el mismo `latest.json` o la API de GitHub) y dice «Hay una 0.8.0» con un enlace a la página de descargas | Vale para **todas** las formas de instalar. Sin claves nuevas ni dependencias; poco código | La persona descarga e instala a mano |
| B · Avisar e instalar | Lo de A, y en Windows y la AppImage el botón «Actualizar» lo hace todo con el plugin `updater` | Lo más cómodo para la mayoría (Windows) | Una clave más que guardar para siempre, dos dependencias (`updater`, `process`), cambios en el workflow y más que probar en cada sistema |
| C · Nada por ahora | Seguir como hoy: la web y las novedades | Cero trabajo | Ordenadores varias versiones por detrás |

## Propuesta

**B, en dos pasos**, para que el primero ya sirva solo:

1. **Primero A:** el aviso con enlace, para todos, con su interruptor. Es poco trabajo y resuelve lo principal: que la gente sepa que hay versión nueva.
2. **Después, instalar en Windows y la AppImage:** generar la clave (la genera y guarda el maintainer; nunca pasa por el asistente), activar las firmas en el workflow, escribir el `latest.json` y el botón «Actualizar». Antes de publicarlo, una prueba de verdad: instalar la versión N en Windows y en una AppImage, publicar la N+1 y ver que se actualizan y se reinician bien.

**Para decidir con el maintainer:**
1. ¿A, B en dos pasos, o C?
2. ¿El interruptor viene encendido o apagado?
3. ¿Cada cuánto se mira? Propuesta: al abrir la app, como mucho una vez al día, y desde Ajustes › Acerca de, a mano.
4. Si se hace B: dónde guarda la copia de la clave privada, además de los secretos de GitHub.

**Lo que no cambia:** Android sigue con Google Play y la snap con su tienda; los `.deb` y `.rpm` solo reciben el aviso.
