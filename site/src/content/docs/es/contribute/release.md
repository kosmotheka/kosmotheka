---
title: Publicar una versión
description: Qué hay que hacer antes de cada versión de la app de Android de Kosmotheka y cómo se compila y firma el paquete que se sube a Google Play.
sidebar:
  order: 14
---

Esta página es para quien publica una versión de la app: el número, las novedades, la lista de licencias y el paquete firmado de Android. No hace falta para colaborar con código.

## Antes de cada versión

1. **El número de versión, en cuatro sitios que tienen que coincidir:**
   - `version` en `Cargo.toml` (la sección `[workspace.package]`, de la raíz);
   - `version` en `app/package.json`;
   - `version` en `app/src-tauri/tauri.conf.json`;
   - la clave de esa versión en `app/src/whats-new.json` (ver el punto 2).

   Si cambia el primer número distinto de cero (de `0.2.x` a `0.3.0`, por ejemplo), cambia también la versión con la que la app pide el dominio, `kosmotheka-domain = { version = "…" }` en `app/src-tauri/Cargo.toml`: con un `0.` delante, Cargo no da por compatible una `0.3` con una `0.2`, y la compilación falla.

   Los archivos de bloqueo (`Cargo.lock`, `app/package-lock.json`) llevan también la versión: se actualizan al compilar o con `npm install`. Las versiones de `Cargo.toml`, `package.json` y `tauri.conf.json` las compara `npm run check:whats-new`.

   **Cada versión que se sube a Google Play tiene que subir de número:** Google rechaza un `versionCode` repetido. Tauri lo calcula a partir de `version` (`0.1.0` → `1000`).
2. **Las novedades.** Las líneas de lo que cambia para la gente viven en `app/src/whats-new.json`, bajo la clave `next` mientras la versión no tiene número. Al elegirlo: copia esas líneas, tal cual, a la página de la guía «Novedades de cada versión» (`using-the-app/whats-new.md`, en los dos idiomas), que guarda todo el detalle; después **condénsalas** en el fichero (como mucho 5 líneas de unos 250 caracteres: una por cosa grande y lo pequeño junto en una que empieza por «Además:»; las que llevan interruptor conservan lo que cuentan de privacidad) y renombra `next` por el número (`"0.2.1"`, por ejemplo). La app enseña el resumen y enlaza con la guía. Después, `cd app && npm run check:whats-new` tiene que pasar: comprueba que la versión de la app tiene su entrada, no que esté completa; léela.
3. **La lista de licencias, con las versiones exactas.**
   ```bash
   cd app
   npm run licenses              # necesita Docker y red
   npm run check:licenses:exact
   ```
   En la integración continua la comprobación solo mira los nombres de los paquetes, para que las actualizaciones automáticas de dependencias pasen solas. Antes de publicar, la pantalla «Licencias» tiene que llevar las versiones que se publican. Si `app/src/licenses.json` cambia, va en el commit de la versión.
4. **Las comprobaciones de siempre:** `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, y en `app/`: `npm run typecheck`, `npm run lint` y `npm run check:spec` (ver [Preparar el entorno: la app](../setup-app/)).

## Con el workflow

`.github/workflows/release.yml` compila y publica **solo la app** (D74): el servidor, la web y la documentación se despliegan a mano con `deploy/deploy.sh`.

1. Deja la versión lista (el apartado «Antes de cada versión») y súbela a `main`.
2. Crea y sube la etiqueta, que tiene que ser `v` más la versión de `tauri.conf.json` (por ejemplo `v0.3.0`); si no coincide, el workflow falla antes de compilar.
3. El workflow hace, en este orden:
   - **Comprobar la versión:** las versiones de los tres archivos y la entrada de `whats-new.json`.
   - **Linux** (Ubuntu 22.04): `.deb`, `.rpm` y AppImage. **Windows:** el instalador. Los nombres no llevan versión: `Kosmotheka-linux-x86_64.AppImage`, `Kosmotheka-linux-amd64.deb`, `Kosmotheka-linux-x86_64.rpm`, `Kosmotheka-windows-x64-setup.exe`.
   - **El *release* en borrador,** con los cuatro archivos, un `SHA256SUMS` y, de cuerpo, las líneas en inglés de la versión en `whats-new.json`.
   - **Android:** compila el `.aab`, comprueba que va firmado y lo sube a Google Play como versión en **borrador**, a la pista que diga la variable `PLAY_TRACK` del entorno `release` (`internal` si no existe; `alpha` es la prueba cerrada).
4. **Revisa y publica el borrador** en GitHub (*Releases*). Hasta entonces nadie lo ve, y los enlaces `…/releases/latest/download/…` siguen dando la versión anterior.
5. **En Google Play,** abre la pista, revisa la versión en borrador, añade las notas (salen de `whats-new.json`) y envíala a revisión.

Para ensayar sin publicar, lanza el workflow a mano (*Actions › Release › Run workflow*): compila todo y deja los archivos como artefactos del workflow (14 días), pero no crea el *release* ni sube nada a Google Play. Sin los secretos de Android, el `.aab` sale sin firmar y el resumen lo dice.

Los secretos y el entorno se preparan una vez, en el repositorio público, y es cosa de quien lo mantiene. Lo que sigue, hecho a mano, vale si el workflow no está disponible.

## La clave de subida

Google Play usa **Play App Signing**: la clave que firma la app que se instala en los móviles la genera y la guarda Google. Quien publica solo tiene una **clave de subida**, que demuestra a Google que el paquete es suyo. Si se pierde o se filtra, Google permite cambiarla (en Play Console, *Integridad de la app*), pero hay que guardarla bien.

Lo que se sube es un **Android App Bundle** (`.aab`), no un APK; Google genera de él el APK de cada móvil.

**La clave vive fuera del repositorio.** Por defecto, en `~/.kosmotheka-release/`:

```
~/.kosmotheka-release/
  upload.jks               la clave de subida
  keystore.properties      dónde está y sus contraseñas
```

`app/src-tauri/gen/android/app/build.gradle.kts` lee `keystore.properties` de esa carpeta, o de la ruta que diga la variable de entorno `KOSMOTHEKA_KEYSTORE_PROPERTIES`. Si el archivo no existe, la compilación de *release* sale **sin firmar**. El `.gitignore` del repositorio ignora `*.jks`, `*.keystore` y `keystore.properties` en cualquier sitio, por si acaso. **Nunca** los subas.

### Crearla (una sola vez)

En tu propia terminal, y con la contraseña escrita por ti:

```bash
mkdir -p ~/.kosmotheka-release && chmod 700 ~/.kosmotheka-release
keytool -genkeypair -v \
  -keystore ~/.kosmotheka-release/upload.jks \
  -keyalg RSA -keysize 2048 -validity 10000 -alias upload
```

Pide una contraseña (dos veces) y unos datos (nombre, organización, país). Si al final pregunta otra contraseña para la clave, Intro usa la misma. Después crea `~/.kosmotheka-release/keystore.properties`:

```properties
storeFile=/ruta/completa/a/upload.jks
storePassword=<la contraseña>
keyAlias=upload
keyPassword=<la contraseña>
```

y protégelo: `chmod 600 ~/.kosmotheka-release/*`. Guarda la contraseña en un gestor de contraseñas y una copia de `upload.jks` fuera del ordenador.

## Compilar el `.aab`

```bash
cd app
npm run tauri android build -- --aab
```

Compila para las cuatro arquitecturas y tarda bastante. El resultado queda en:

```
app/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

Comprueba que va firmado con la clave de subida (no pide contraseña):

```bash
keytool -printcert -jarfile app/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab
```

Debe mostrar el certificado con los datos que pusiste al crear la clave. Si no hay certificado, el paquete salió sin firmar: falta o está mal `keystore.properties`.

Para compilar con las herramientas de Android, sigue [Preparar el entorno: la app](../setup-app/).

## Subirlo

Lo subes tú, a mano, desde la consola de Google Play: en *Pruebas* eliges la pista, *Crear versión*, subes el `.aab`, pones las notas de la versión y publicas. Las notas de Google Play (500 caracteres por idioma como máximo) salen de las líneas de `app/src/whats-new.json`.

Una cuenta personal nueva de Google Play necesita, antes de producción, una prueba **cerrada** con al menos 12 personas durante 14 días.

## Una cosa que sorprende

La versión de Google Play y un APK de depuración tienen el mismo identificador de aplicación pero distinta firma: Android no deja instalar una encima de la otra. Para pasar del APK de depuración a la de Play hay que desinstalar la primera, y **eso borra sus datos**. Antes, haz una copia de seguridad (o ten las bibliotecas en otro dispositivo emparejado) y restáurala o empareja de nuevo después.
