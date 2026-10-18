# Encargo · El workflow de publicación (8.2, D74)

> Encargo cerrado para la sesión de hitos, escrito el 18/10/2026. Sin commit, sin push. D74 aceptada ese día con un límite: **el workflow solo compila y publica la app.** El servidor, la web y la documentación no se tocan desde GitHub: se despliegan a mano en el VPS con `deploy/deploy.sh` (D90). Lee D74 entera antes de empezar.

## Qué

Un archivo nuevo, `.github/workflows/release.yml`, que se lanza **al subir una etiqueta `v*`** (y a mano con `workflow_dispatch`, como ensayo que no publica nada).

1. **Comprobar la versión:** la etiqueta (`v0.3.0`) tiene que coincidir con `version` de `app/src-tauri/tauri.conf.json`. Si no coincide, falla antes de compilar. Si existe una comprobación así en `app/scripts/`, úsala.
2. **Escritorio, en dos trabajos:** `ubuntu-22.04` (nunca 24.04: glibc, D74 punto 5) y `windows-latest`.
   - Los paquetes de sistema de Linux, los mismos que el trabajo `build` de `ci.yml`.
   - Producen `.deb`, `.rpm`, AppImage y el instalador de Windows.
   - **Nombres sin versión** (D74 punto 2): `Kosmotheka-linux-x86_64.AppImage`, `Kosmotheka-linux-amd64.deb`, `Kosmotheka-linux-x86_64.rpm`, `Kosmotheka-windows-x64-setup.exe`.
3. **El *release*, en borrador:** con los cuatro archivos y un `SHA256SUMS`. Usa `gh release create --draft` con el `GITHUB_TOKEN` (`permissions: contents: write` solo en ese trabajo) en lugar de una acción de terceros, si alcanza. El cuerpo, las líneas en inglés de esa versión de `app/src/whats-new.json`.
4. **Android:**
   - Java, el NDK y los objetivos de Rust que pida Tauri; mira `docs/development/setup.md` para saber qué versiones usa la máquina de desarrollo.
   - **La clave de subida, desde secretos:** `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD` y `ANDROID_KEY_PASSWORD`; el alias es `upload`. Se escribe en `$RUNNER_TEMP` un `upload.jks` y un `keystore.properties`, y se pasa su ruta con `KOSMOTHEKA_KEYSTORE_PROPERTIES`, que `build.gradle.kts` ya lee (`docs/publicacion/firma-android.md`). Nada se escribe dentro del repositorio ni sale en el registro.
   - `npm run tauri android build -- --aab`; después, que el `.aab` va firmado (`keytool -printcert -jarfile`, sin contraseña).
   - **Subirlo a Google Play,** a la pista que diga una variable: `internal` por defecto, y `alpha` es la prueba cerrada. Con una cuenta de servicio (`PLAY_SERVICE_ACCOUNT_JSON`) y el paquete de la app (míralo en `tauri.conf.json`). Como versión en borrador si la API lo deja (`status: draft`): el paso a la gente lo da el maintainer en la consola. Una acción conocida para esto, fijada por SHA, o la API con `curl`; elige y di por qué.
   - **Sin los secretos, el trabajo de Android compila sin firmar y no sube nada**, con un aviso claro en el resumen. Así el ensayo funciona antes de la sesión de los secretos.
   - El `.aab` también como artefacto del *workflow*; no va en el *release*, que es para el escritorio.
5. **Seguridad:**
   - todas las acciones fijadas por SHA con su versión en comentario, como en `ci.yml`;
   - `permissions:` mínimos por trabajo;
   - los secretos de Android y de Play, en un **entorno de GitHub** (`release`), no en los del repositorio, para que el maintainer pueda pedir su aprobación antes de cada uso;
   - nada se lanza desde *pull requests*.

## La documentación

- `site/src/content/docs/es/contribute/release.md` y `en/`: un apartado «Con el workflow»: subir la etiqueta, qué hace, revisar el borrador y publicarlo; en Play, pasar la versión de borrador a la pista. Lo que se hace a mano queda como alternativa.
- `docs/publicacion/firma-android.md`: un apartado nuevo «Los secretos del workflow». Es **una sesión con el maintainer, en su terminal y no a través de Claude Code**, porque las contraseñas nunca pasan por un agente:
  - el keystore en base64 (`base64 -w0 ~/.kosmotheka-release/upload.jks`), copiado al secreto sin pasar por un archivo del repositorio;
  - la cuenta de servicio de Google Cloud con permiso solo para esta app en Play Console;
  - el entorno `release` con su aprobación.

  Se hace en el **repositorio público**, después de abrirlo, porque es otro repositorio.
- `docs/ROADMAP.md` y `docs/plans/fase-8.md`: 8.2 escrito, a falta de los secretos y de una primera etiqueta.

## Al terminar

- **`actionlint`** sobre los dos workflows, con Docker y la versión fijada (por ejemplo `rhysd/actionlint:1.7.7`; mira la última estable).
- No hay forma de lanzarlo en local: dilo, y di qué partes no se han podido comprobar.
- En `site/`: `npm run build && npm run check:links`.

El informe:
- el workflow, trabajo a trabajo;
- las acciones nuevas, con su SHA;
- los nombres exactos de los secretos y del entorno;
- lo que queda sin comprobar hasta la primera etiqueta.
