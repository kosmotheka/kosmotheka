# Entorno de desarrollo

> Pasos comprobados el 24/09/2026 en un Slimbook con Ubuntu 20.04 (ver D23). Semilla de la futura guía de contribución.

## Por qué un contenedor

Tauri 2 necesita WebKitGTK 4.1, que no existe en Ubuntu 20.04. En lugar de actualizar el sistema, se desarrolla dentro de un contenedor **Distrobox** con Ubuntu 24.04. Distrobox comparte con el sistema la carpeta personal, la red y la pantalla:
- el código se edita con el editor de siempre;
- las ventanas de Tauri se ven como apps normales;
- Rust, el SDK de Android y Node viven en la carpeta personal, pero **Rust y las variables de Android solo se activan dentro del contenedor**.

En una distribución reciente (Ubuntu 22.04 o posterior, Fedora, Arch…) no hace falta el contenedor: basta con instalar los mismos paquetes directamente.

## Requisitos en el sistema

- Docker utilizable sin sudo (el usuario debe estar en el grupo `docker`).
- Nada más: todo lo demás se instala en la carpeta personal o dentro del contenedor.

## 1. Distrobox y el contenedor

```bash
curl -s https://raw.githubusercontent.com/89luca89/distrobox/main/install | sh -s -- --prefix ~/.local
distrobox create --name library-dev --image ubuntu:24.04
distrobox enter library-dev
```

## 2. Paquetes del sistema (dentro del contenedor)

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev pkg-config \
  openjdk-17-jdk-headless unzip git
```

## 3. Variables de entorno solo para el contenedor

Se añade al final de `~/.bashrc`. Como la carpeta personal es compartida, el bloque comprueba en qué contenedor se está, para no alterar el sistema:

```bash
# library-dev distrobox: toolchains only active inside the container
if [ "$CONTAINER_ID" = "library-dev" ]; then
    [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
    export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
    export ANDROID_HOME="$HOME/Android/Sdk"
    export NDK_HOME="$ANDROID_HOME/ndk/current"
    export PATH="$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$PATH"
fi
```

## 4. Rust (dentro del contenedor)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
. ~/.cargo/env
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

`--no-modify-path` evita que rustup toque los ficheros de la shell: la activación la hace el bloque del paso 3.

### Ventana en blanco o gris (kernel antiguo)

Si la ventana de Tauri sale vacía y el log muestra `amdgpu: DRM version is 3.38.0, but this driver is only compatible with 3.42.0 (kernel 5.15+)`, significa que los drivers gráficos del contenedor son más nuevos de lo que admite el kernel del sistema. Solución: renderizar WebKitGTK por software, añadiendo al bloque del paso 3:

```bash
export LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 WEBKIT_DISABLE_DMABUF_RENDERER=1
```

Solo afecta al desarrollo en esta máquina: la app compilada para otros usuarios no lleva estas variables.

## 5. SDK de Android (dentro del contenedor)

1. Descargar las *command-line tools* (la URL actual está en developer.android.com/studio) y descomprimirlas en `~/Android/Sdk/cmdline-tools/latest`.
2. Instalar los paquetes:

```bash
yes | sdkmanager --licenses
sdkmanager --install "platform-tools" "platforms;android-36" "build-tools;37.0.0" "ndk;30.0.16248370"
ln -sfn "$ANDROID_HOME/ndk/30.0.16248370" "$ANDROID_HOME/ndk/current"
```

El enlace `ndk/current` permite cambiar de versión del NDK sin tocar `NDK_HOME`. Nota: `sdkmanager` avisa de que está obsoleto en favor de la nueva «Android CLI»; de momento sigue funcionando.

## 6. Node

Node 22 viene de `nvm`, instalado en la carpeta personal, así que está disponible también dentro del contenedor.

## 7. Móvil Android

Se conecta por **depuración inalámbrica** (Android 11 o superior), no por USB:
1. En el móvil: Opciones de desarrollador → Depuración inalámbrica → Vincular con código.
2. En el contenedor: `adb pair IP:PUERTO` con el código mostrado y después `adb connect IP:PUERTO`.

3. Instalar y ejecutar: `npm run tauri android dev` (desde `app/`) compila, instala y abre la app en el móvil, con recarga en caliente.

## 8. Compilar un APK

```bash
cd app
npm run tauri android build -- --debug --apk --target aarch64
```

El APK queda en `app/src-tauri/gen/android/app/build/outputs/apk/universal/debug/`. El de depuración ocupa unos 135 MB porque lleva el Rust sin optimizar y con símbolos de depuración; el de *release* es mucho más pequeño, pero tiene que ir firmado (pendiente).

`--target aarch64` compila solo para ARM de 64 bits, que es lo que llevan los móviles actuales. Sin esa opción compila para las cuatro arquitecturas y tarda cuatro veces más.

### Fallo al descargar Gradle (IPv6)

Si la primera compilación falla en `org.gradle.wrapper.Download`, puede que la red resuelva direcciones IPv6 sin tener salida por IPv6 (en esta máquina pasa). Solución: forzar IPv4 una vez, hasta que Gradle quede descargado y en caché:

```bash
JAVA_TOOL_OPTIONS=-Djava.net.preferIPv4Stack=true npm run tauri android build -- --debug --apk --target aarch64
```

## Versiones instaladas (24/09/2026)

| Herramienta | Versión |
|---|---|
| Ubuntu (contenedor) | 24.04.5 LTS |
| WebKitGTK | 2.52.6 (API 4.1) |
| Rust | 1.98.1 |
| Node / npm | 22.23.2 / 10.9.8 |
| JDK | 17.0.20.1 |
| NDK | 30.0.16248370 |
| Android platform / build-tools | 36 / 37.0.0 |

## Deshacer la instalación

- Quitar el bloque `library-dev` de `~/.bashrc`. Hay una copia previa en `~/.bashrc.bak-before-library`.
- Borrar el contenedor: `distrobox rm library-dev`.
- Borrar `~/.cargo`, `~/.rustup` y `~/Android`.
- Borrar Distrobox: `~/.local/bin/distrobox*`.
