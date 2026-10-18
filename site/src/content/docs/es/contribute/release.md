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

   Los archivos de bloqueo (`Cargo.lock`, `app/package-lock.json`) llevan también la versión: se actualizan al compilar o con `npm install`. Las versiones de `Cargo.toml`, `package.json` y `tauri.conf.json` las compara `npm run check:whats-new`.

   **Cada versión que se sube a Google Play tiene que subir de número:** Google rechaza un `versionCode` repetido. Tauri lo calcula a partir de `version` (`0.1.0` → `1000`).
2. **Las novedades.** Las líneas de lo que cambia para la gente viven en `app/src/whats-new.json`, bajo la clave `next` mientras la versión no tiene número. Al elegirlo, renombra `next` por el número (`"0.2.1"`, por ejemplo). Después, `cd app && npm run check:whats-new` tiene que pasar: comprueba que la versión de la app tiene su entrada, no que esté completa; léela.
3. **La lista de licencias, con las versiones exactas.**
   ```bash
   cd app
   npm run licenses              # necesita Docker y red
   npm run check:licenses:exact
   ```
   En la integración continua la comprobación solo mira los nombres de los paquetes, para que las actualizaciones automáticas de dependencias pasen solas. Antes de publicar, la pantalla «Licencias» tiene que llevar las versiones que se publican. Si `app/src/licenses.json` cambia, va en el commit de la versión.
4. **Las comprobaciones de siempre:** `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, y en `app/`: `npm run typecheck`, `npm run lint` y `npm run check:spec` (ver [Preparar el entorno: la app](../setup-app/)).

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
