# Encargo · Limpiar lo que no puede salir tal cual en `docs/`

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Sigue a tu informe [clasificacion-docs.md](clasificacion-docs.md). El maintainer ha aceptado todas las recomendaciones (abajo). Solo se cambian ficheros de `docs/`. Si algo no encaja, para y dímelo.

## Lo que decidió el maintainer

1. **Hallazgos abiertos:** se cierran o se aceptan antes de abrir el repositorio, y así todos los informes salen enteros. Esto no es de este encargo: es trabajo de desarrollo.
2. **SEG-023 y SEG-032 (aceptados):** se publican.
3. **El buzón de Gmail y los datos del VPS** (proveedor, memoria, que se comparte con otro proyecto, lo observado allí): se escriben en genérico.
4. **`assetlinks.json`:** se queda con las dos huellas, porque ya es público por diseño.
5. **TMview, sus scripts, los nombres de los titulares de marcas y el código de Claude Design:** fuera del repositorio, al reescribir el historial.
6. **`firma-android.md` y `servidor-produccion.md`:** su parte genérica pasa a la guía de autoalojamiento y el resto va a `library-privado/`, al abrir el repositorio.
7. **`server/.env.dev` del historial:** al escaneo de secretos, con el maintainer.
8. **`VISION.md`:** lo personal se reescribe si ayuda a entender lo que se ha hecho; si no, va a privado.

## Qué hacer ahora

Solo lo que no rompe nada mientras el repositorio sigue privado:

1. **El buzón de Gmail** (9 apariciones: `DECISIONS.md`, `development/correo.md`, `google-oauth.md` y `servicios-externos.md`): se cambia la dirección por «el buzón de Gmail del proyecto». **Antes, apunta en `/home/sergio/Proyectos/personales/library-privado/notas-publicacion.md` qué dirección era y en qué ficheros estaba**, para que el maintainer no pierda el dato. Crea el fichero si no existe. Allí no hay git: es una carpeta normal.
2. **El VPS:**
   - el proveedor y su política de puertos (`DECISIONS.md`) pasan a «el proveedor del VPS»;
   - la memoria (`cloud.md`, línea 192 más o menos) pasa a «un VPS pequeño»;
   - «compartido con otro proyecto» (`fase-7.md`) pasa a «un VPS pequeño y compartido»;
   - la observación de «Google Books dio 503 desde el VPS» se queda sin la parte del VPS si se entiende igual.

   Cuida que el porqué de cada decisión se siga entendiendo; si se pierde, dímelo en vez de borrarlo. Apunta lo que quitas en el mismo fichero privado.
3. **`testing/fase-7.md`, líneas 111 y 114:** el nombre de la base de datos y la ruta del registro de nginx del servidor real pasan a marcadores (`<la base de datos>`, `<el registro de nginx>`), como hace `servidor-produccion.md`.
4. **`design/screens/README.md`:** quitar el identificador del proyecto de Claude Design (el enlace).
5. **`research/naming.md`:** en las filas de los titulares que son personas físicas, cambiar el nombre por «una persona física (titular)». Las empresas se quedan.
6. **`audits/README.md`:** cambiar el ejemplo con un id real por uno inventado (`SEG-999`), como pide D90.
7. **`VISION.md`.** Léelo entero y propón en tu informe **una de las dos cosas**:
   - **reescribirlo:** tu texto nuevo, que conserve lo que explica el proyecto (de dónde salió la idea, qué problema resuelve, para quién) y quite lo personal (familia con nombres, trabajo, alias), dicho de forma general («una familia con varios lectores»). No lo apliques: ponlo en el informe para que lo vea el maintainer;
   - **o moverlo a privado**, si lo que queda sin lo personal no aporta.
8. **El fichero de `historico/` con eñe en el nombre:** di cuál es y cuántos enlaces lo nombran. No lo renombres todavía.

## Qué no hacer ahora (va al paso 8, al abrir el repositorio)

No toques los enlaces a `firma-android.md` ni a `servidor-produccion.md`, ni las filas abiertas de `hallazgos.md`, ni `tmview/`. Siguen en uso mientras el repositorio sea privado. **Pero escribe la lista**: cada fichero y línea que habrá que cambiar ese día, con qué cambio. Ponla en una sección «Para el día de abrirlo» al final de tu informe de clasificación (`clasificacion-docs.md`); es lo único que añades a ese fichero.

## Reglas

- Nunca copies en el repositorio la dirección de Gmail ni ningún dato del VPS. Solo van al fichero privado.
- No cambies nada fuera de `docs/`, salvo el fichero privado.
- No deshagas nada con git.

## Al terminar

Un informe corto: qué ficheros has cambiado, la propuesta de `VISION.md`, el nombre con eñe y un `grep` que demuestre que la dirección de Gmail ya no sale en el repositorio (`git grep -i "@gmail" -- docs AGENTS.md`; las de `app/src/licenses.json` son de autores de paquetes y se quedan).
