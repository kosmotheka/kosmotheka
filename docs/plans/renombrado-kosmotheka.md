# Plan de renombrado: de «library» a Kosmotheka

> Plan de D70. Todavía **no se ha ejecutado nada**. Cada etapa se hace cuando el maintainer la apruebe, con copia de seguridad antes y forma de deshacerla.

## Qué se renombra y qué no

- **Se renombra el producto:** identificador de la app, nombre visible, *crates*, variables de entorno, servidor, repositorio y documentación.
- **No se renombra la entidad `library`** del código (la biblioteca: D38, D46, D54): tablas, `library_id`, `Entity::Library`, las operaciones `library.*` y el glosario de `AGENTS.md`. Buscar y reemplazar «library» a ciegas rompería el dominio. Cada cambio se hace a mano, sitio por sitio.

## Inventario (26/09/2026)

| Dónde | Hoy | Propuesta |
|---|---|---|
| Identificador Tauri (`tauri.conf.json`) | `com.tersoal.library` | `app.kosmotheka` (decidido; ver «Identificador») |
| Android: `applicationId` y `namespace` (`gen/android/app/build.gradle.kts`) | `com.tersoal.library` | el mismo que Tauri |
| Android: paquete de `MainActivity.kt` | `com/tersoal/library/` | lo regenera `tauri android init`, o se mueve a mano |
| Nombre visible (`productName`, título de la ventana) | `Library` | `Kosmotheka` |
| Título en la interfaz (`app.title` en i18n) | `Biblioteca` / `Library` | `Kosmotheka` en los dos idiomas |
| *Crates* | `library-app` (lib `library_app_lib`), `library-domain` | `kosmotheka` (lib `kosmotheka_lib`), `kosmotheka-domain` |
| Paquete npm (`app/package.json`) | `library-app` | `kosmotheka` |
| Variable de desarrollo | `LIBRARY_DATA_DIR` | `KOSMOTHEKA_DATA_DIR` |
| Protocolo LAN: prefijo del código de emparejamiento | `LIBRARY1:` | `KOSMOTHEKA1:` |
| Protocolo LAN: `APP_ID` del saludo | `library` | `kosmotheka` |
| Formato de las copias (`backup.rs`) | `library-backup` | `kosmotheka-backup`, **aceptando también el antiguo** al restaurar |
| Nombres de fichero de copias y CSV | `library-AAAA-MM-DD.zip` | `kosmotheka-…` |
| User-Agent de los proveedores (`metadata/mod.rs`) | `…github.com/Tersoal/library` | la URL pública futura: `github.com/kosmotheka/kosmotheka` o `kosmotheka.app`, nunca el repo privado |
| `spec/operation.schema.json` (`$id`) | `github.com/Tersoal/library/…` | la URL pública futura (como el User-Agent) |
| Servidor: Docker Compose | proyecto `library`; contenedores `db_library`, `php_library`, `nginx_library`; volumen `db_library`; base `library`; `library.local` | `kosmotheka`, `db_kosmotheka`…, base `kosmotheka`, `kosmotheka.local` |
| Servidor: Composer | `tersoal/library-server` | `tersoal/kosmotheka-server` |
| Servidor: correos (Fase 7) | — | remitente en `kosmotheka.app` |
| Repositorio GitHub | `Tersoal/library` (privado) | **no se mueve por ahora**: el renombrado se hace dentro de él. `kosmotheka/kosmotheka` queda reservado para el futuro repositorio público (ver etapa 5) |
| Carpeta local del repositorio | `~/Proyectos/personales/library` | opcional; si cambia, hay que actualizar Distrobox, rutas en la documentación y la memoria de las sesiones |
| README, `AGENTS.md` («Working name: library»), `docs/` | «library», «biblioteca familiar» | Kosmotheka; `docs/historico/` no se toca |

## Identificador: lo primero y lo único irreversible

Google Play identifica una app por su `applicationId` para siempre: no se puede cambiar una vez publicada. Para reservarlo hay que subir una primera versión (AAB firmado) a una prueba interna de Play Console.

**Decidido (26/09/2026): `app.kosmotheka`**, el dominio kosmotheka.app al revés, que es la convención. Alternativa: `com.kosmotheka.app`. Tauri desaconseja los identificadores acabados en `.app`, porque en macOS chocan con la extensión de las aplicaciones. No publicamos para macOS, pero así se evita el aviso.

## Datos: que nada parezca desaparecer

**Escritorio (Linux y Windows).** La carpeta de datos depende del identificador: hoy es `~/.local/share/com.tersoal.library/` (en Windows, `%APPDATA%\com.tersoal.library\`). Con el identificador nuevo, la app buscaría `…/app.kosmotheka/`, la encontraría vacía y parecería que se ha perdido todo.

La solución: al arrancar, si la carpeta nueva no existe y la antigua sí, la app **copia** (no mueve) la antigua a la nueva.
- La antigua se queda intacta: es la forma de deshacer. Basta con volver a la versión anterior de la app.
- La copia usa `VACUUM INTO` para la base de datos, como la copia previa a las migraciones, y copia las portadas y el certificado de sincronización (`sync-cert.der`, `sync-key.der`), para que el dispositivo conserve su identidad y sus emparejamientos.
- Se prueba antes con un test que simula las dos carpetas y, a mano, con una copia de la carpeta real.
- Pasado un tiempo, la ayuda explica cómo borrar la carpeta antigua. La app no la borra sola.

**Android.** Un `applicationId` nuevo es otra app: Android no deja que lea los datos de la antigua. Hay dos formas de recuperar lo que tuviera la app de prueba:
1. **Sincronizar** con el escritorio: se empareja la app nueva y recibe las bibliotecas. Es lo más sencillo.
2. **Copia de seguridad**: exportar en la antigua e importar en la nueva.

Las dos apps pueden convivir en el móvil hasta comprobar que todo ha llegado; después se desinstala la antigua. Hoy el maintainer aún no ha instalado ni emparejado en el móvil, así que no hay nada que perder.

**Servidor.** Cambiar el volumen y el nombre de la base empieza con una base vacía. Hoy solo hay datos de desarrollo. Aun así se hace con `pg_dump` antes y `pg_restore` después, y el volumen antiguo no se borra hasta comprobarlo.

## Etapas

Cada etapa deja todo funcionando y es un commit aparte (o varios).

1. **Identificador y nombre visible** (urgente, para reservar en Play):
   - Confirmar el identificador.
   - Copia de seguridad de `~/.local/share/com.tersoal.library/` fuera de ella.
   - `identifier`, `productName` y título en `tauri.conf.json`; `applicationId`, `namespace` y paquete de `MainActivity` en Android. Regenerar lo que genera Tauri, revisando el diff de `gen/android`.
   - Copia automática de la carpeta de datos antigua a la nueva, con su test.
   - Probar el escritorio con la copia y la APK en el móvil.
2. **AAB firmado para Play Console** (con el maintainer):
   - Crear la clave de subida (*upload key*) con `keytool`. **Nunca se sube al repositorio**. Se guarda con copia en un sitio seguro, y la contraseña en el gestor de contraseñas.
   - Activar la firma de apps de Google Play: Google guarda la clave de firma definitiva, así que perder la de subida se puede arreglar.
   - `npm run tauri android build -- --aab`, subir a una prueba interna y rellenar lo mínimo que pida Play Console. Esto último lo hace el maintainer: son acciones en su cuenta.
   - Documentarlo en `docs/development/` para la Fase 8.
3. **Código:** *crates*, paquete npm, variable de entorno, i18n, User-Agent y los identificadores del protocolo LAN (`LIBRARY1:`, `APP_ID`) y de las copias (aceptando el formato antiguo al restaurar).
   - Cambiar el protocolo exige subir la versión del esquema, para que dos versiones distintas no intenten hablar entre sí.
   - Pasan `cargo test`, clippy, typecheck, lint y `check:spec`.
4. **Servidor:** Docker Compose, contenedores, volumen, base, Composer y nginx, con `pg_dump` antes. `make check` en verde.
5. **Repositorio** (ajustado el 26/09/2026, aprobado por el maintainer):
   - El repositorio de trabajo **sigue privado y no se mueve**: continúa en `Tersoal/library`, y el renombrado se hace dentro de él.
   - La organización `kosmotheka` ya existe. **`kosmotheka/kosmotheka` queda reservado para el futuro repositorio público.** No se crea ni se transfiere nada ahí por ahora: un repo vacío con ese nombre, además, bloquearía una transferencia futura.
   - Dónde acaba viviendo el privado (en la cuenta del maintainer, o en la organización con otro nombre, por ejemplo `kosmotheka/workshop`) depende de cómo se cierre D64.
   - Las URL de `spec/` y del User-Agent apuntan a la dirección pública futura, no al repo privado.
   - Cualquier acción en GitHub la hace el maintainer.
6. **Documentación:** README, `AGENTS.md`, `CLAUDE.md` y `docs/` vigentes. El histórico no se toca.

## Pendiente de decidir

- Si se renombra también la carpeta local del repositorio.

## Estado

- [x] **Etapa 1** (26/09/2026): identificador `app.kosmotheka` en Tauri y Android, nombre visible «Kosmotheka» (ventana, Android, título de la interfaz) y copia automática de la carpeta de datos antigua (`data_folder.rs`, con tests).
  - Copia de seguridad previa en `~/copias-kosmotheka/antes-del-renombrado-2026-09-26/`.
  - Probado sobre una copia de la carpeta real: mismos libros, operaciones y dispositivo; certificado de sincronización idéntico y la clave con permisos 600; carpeta antigua sin tocar.
  - Lección: la vista web crea su carpeta de datos antes de que arranque la app, así que la copia se decide por «¿hay base de datos?», no por «¿existe la carpeta?».
  - El fichero de la base de datos sigue llamándose `library.sqlite3` (interno, no se ve); el tema de Android `Theme.library_app` sale del nombre del *crate* y cambia en la etapa 3.
- [ ] Etapa 2: AAB firmado para Play Console.
- [ ] Etapa 3: código.
- [ ] Etapa 4: servidor.
- [ ] Etapa 5: repositorio.
- [ ] Etapa 6: documentación.

