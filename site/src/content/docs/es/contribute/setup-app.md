---
title: "Preparar el entorno: la app"
description: Qué instalar para compilar y ejecutar la app de escritorio y la de Android, y cómo comprobar que todo pasa.
sidebar:
  order: 2
---

Esta página sirve para compilar la app (`app/` y `crates/`) y probarla. Para el servidor, mira [Preparar el entorno: el servidor](../setup-server/).

## Qué hace falta

- **Rust** estable. El proyecto usa la edición 2024 de Rust, así que necesitas un compilador reciente.
- **Node 22** y npm.
- **En Linux, las bibliotecas de sistema que necesita Tauri.** En Debian y Ubuntu: `libwebkit2gtk-4.1-dev`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev` y `librsvg2-dev`, además de `build-essential`, `curl`, `wget`, `file` y `pkg-config`.
- **Para Android:** JDK 17, el SDK de Android (herramientas de línea de comandos, `platform-tools`, plataforma 36, `build-tools` 37) y el NDK, más los destinos de Rust para Android (`rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`).
- **Docker**, solo si quieres regenerar la lista de licencias o trabajar en el servidor.

En Windows y en macOS las dependencias de sistema las trae Tauri de otra manera (WebView2 en Windows); la guía de Tauri las explica. El proyecto se desarrolla en Linux, y es ahí donde está probado todo lo que sigue.

## Con o sin contenedor

El maintainer trabaja dentro de un contenedor **Distrobox** llamado `library-dev`, con Ubuntu 24.04. *Por qué:* Tauri 2 necesita WebKitGTK 4.1, que los sistemas más antiguos no tienen (Ubuntu 20.04, por ejemplo); un contenedor evita actualizar el sistema entero. Distrobox comparte la carpeta personal, la red y la pantalla, así que el código se edita con el editor de siempre y las ventanas de Tauri salen como apps normales.

**No es la única forma, ni hace falta.** En una distribución reciente (Ubuntu 22.04 o posterior, Fedora, Arch…) instala los mismos paquetes directamente y salta el contenedor. Si quieres reproducir el entorno del maintainer, [`docs/development/setup.md`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/development/setup.md) tiene cada paso (en español): crear el contenedor, instalar paquetes, activar Rust y Android **solo dentro del contenedor** y conectar un móvil por depuración inalámbrica.

En la documentación del proyecto, un comando de la app puede aparecer así:

```bash
distrobox enter library-dev -- bash -ic '<comando>'
```

La opción `-i` es necesaria porque carga el entorno de `~/.bashrc`. Sin contenedor, ejecuta el comando tal cual.

## Arrancar la app en el escritorio

```bash
cd app
npm install
npm run tauri dev
```

Compila el núcleo en Rust, abre la ventana y recarga la interfaz al guardar un archivo.

Para ver solo la interfaz en el navegador, con datos inventados y sin Rust, usa `npm run dev` y abre la dirección que imprime con `?mock` al final (`/?mock`). Es el modo de prueba que usan las capturas de la guía de usuario.

Si necesitas **una segunda instancia con sus propios datos** (por ejemplo, para probar la sincronización por la red de casa con dos «dispositivos» en la misma máquina), en las compilaciones de depuración hay una variable que cambia la carpeta de datos:

```bash
KOSMOTHEKA_DATA_DIR=/tmp/kosmotheka-b target/debug/kosmotheka
```

(con `npm run tauri dev` funcionando antes, para que exista el binario).

## Arrancar en Android

Se prueba en un móvil real, conectado por depuración inalámbrica (Android 11 o superior) en vez de USB:

1. En el móvil: Opciones de desarrollador, «Depuración inalámbrica», «Vincular con código».
2. En tu máquina: `adb pair IP:PUERTO CÓDIGO`. Deja el cuadro del móvil abierto hasta que termine, o el código caduca.
3. `adb devices` debe listar el móvil. Si no, `adb connect IP:PUERTO` con el puerto de la pantalla principal de «Depuración inalámbrica» (no es el de vincular).
4. Desde `app/`: `npm run tauri android dev`. Compila, instala y abre la app, con recarga.

Para generar un APK de depuración:

```bash
cd app
npm run tauri android build -- --debug --apk --target aarch64
```

`--target aarch64` compila solo para los móviles actuales de 64 bits; sin él compila cuatro arquitecturas y tarda cuatro veces más. El proyecto de Android generado por Tauri (`app/src-tauri/gen/android/`) está en el repositorio: edítalo con cuidado.

## Las comprobaciones

Antes de enviar un cambio, ejecuta las que correspondan a lo que tocaste. La integración continua (GitHub Actions) ejecuta todas.

**Rust** (desde la raíz del repositorio). `app/dist` debe existir antes, porque Tauri incrusta la interfaz ya compilada: ejecuta antes `npm run build` en `app/`.

| Para qué | Comando |
|---|---|
| Formato | `cargo fmt --all --check` (para corregir, `cargo fmt --all`) |
| Avisos del compilador | `cargo clippy --all-targets -- -D warnings` |
| Pruebas | `cargo test` |
| Licencias y avisos de las dependencias | `cargo deny check` |
| Dependencias declaradas y sin usar | `cargo machete` |

`cargo deny` y `cargo machete` se instalan con `cargo install`.

**La interfaz** (desde `app/`):

| Para qué | Comando |
|---|---|
| Compilar | `npm run build` |
| Tipos | `npm run typecheck` |
| Estilo | `npm run lint` y `npm run format:check` (`npm run format` corrige) |
| Código sin usar | `npm run knip` |
| Que haya línea de novedades de la versión | `npm run check:whats-new` |
| Que la lista de licencias cubra lo que se entrega | `npm run check:licenses` |

**La especificación** (desde `app/`): `npm run check:spec` comprueba el esquema contra los casos de conformidad, y `npm run check:codes` la regla del código del segundo factor.

**Pruebas con servicios reales.** Las pruebas normales **nunca usan la red**: leen respuestas grabadas en `app/src-tauri/tests/fixtures/`. Las que sí la necesitan están marcadas como ignoradas y su nombre empieza por `live_`; se lanzan a mano con `cargo test -p kosmotheka -- --ignored live_`. Las ignoradas con nombre `print_*` generan esas respuestas grabadas, y se ejecutan solo cuando cambia un formato.

**Si cambias una dependencia que va dentro de la app**, la lista de licencias se regenera con `npm run licenses`. Necesita Docker y red.

## Si algo falla

- **Ventana en blanco o gris en Linux con un kernel antiguo.** Si el registro dice que el controlador gráfico del contenedor es más nuevo de lo que admite el kernel (`amdgpu: DRM version…`), renderiza WebKitGTK por software con `LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1`. Solo es para desarrollo.
- **La primera compilación de Android falla en `org.gradle.wrapper.Download`.** Puede que tu red resuelva IPv6 sin tener salida. Fuerza IPv4 una vez, hasta que Gradle quede en caché: `JAVA_TOOL_OPTIONS=-Djava.net.preferIPv4Stack=true npm run tauri android build -- --debug --apk --target aarch64`.
- **Los instaladores de escritorio** (`npm run tauri build`) solo se instalan en sistemas como el del contenedor o más nuevos. El de Windows lo genera la integración continua.

Siguiente paso: [Cómo contribuir](../how-to-contribute/).
