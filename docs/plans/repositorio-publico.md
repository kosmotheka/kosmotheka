# Dejar el repositorio listo para abrirlo

> Inventario y orden de trabajo de los hitos 8.4 y 8.5 ([fase-8.md](fase-8.md)), escrito el 16/10/2026. Se basa en D90 (que sustituye a D64), D72 y D02. Se hace en paralelo a la prueba cerrada de Google Play. La reescritura del historial y la subida son del maintainer; todo lo demás, de las sesiones.

## Lo que pide el maintainer

- **Todo en un repositorio**, también la documentación pública. Nada de un repositorio aparte para la documentación.
- **Quien quiera colaborar** tiene que ver claro el procedimiento habitual: *fork*, rama y *pull request*.
- **Quien quiera montar su servidor** tiene que ver claros los dos caminos (sin Docker y con Docker, D48) y las pocas limitaciones:
  - la app oficial es la nuestra;
  - con un servidor propio no hay avisos con la app cerrada (D75);
  - lo que hay que configurar aparte (correo, Google, claves de proveedores).
- **La guía de usuario, en lenguaje que entienda cualquiera**, sin jerga.

## Qué hay hoy

| Dónde | Qué | Tamaño | Qué hacer |
|---|---|---|---|
| `README.md` | En inglés, pero se quedó en las fases 1 y 2 | | Rehacer: qué es, capturas, descargas, enlaces a la documentación, cómo colaborar, licencia y marca |
| `SECURITY.md`, `LICENSE`, `THIRD-PARTY-NOTICES.md` | Ya existen | | Revisar. LICENSE y README, con la exclusión de la marca (D72) |
| `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `TRADEMARKS.md` | No existen | | Escribirlos |
| `AGENTS.md`, `CLAUDE.md`, `server/AGENTS.md`, `.claude/skills/` | Instrucciones para agentes | | Se publican (son útiles a quien colabore con un asistente), sin rutas a lo interno ni ejemplos de hallazgos reales |
| `docs/DECISIONS.md` | 100 decisiones | | Se publica (D90). Traducir |
| `docs/architecture/` | Cómo está construido | 6 ficheros | Se publica; es la base de la parte para colaboradores. Traducir |
| `docs/development/` | Entorno, servidor, correo, Google | 8 | Se publica casi todo; `revision-servidor.md`, a mirar |
| `docs/ROADMAP.md`, `docs/VISION.md` | Plan y visión | | Se publican; el ROADMAP, resumido, sin notas de trabajo |
| `docs/plans/`, `docs/historico/` | Planes y encargos | 4 + 42 | Se publica, como diario de desarrollo (D90, 16/10/2026) |
| `docs/audits/` | Hallazgos de seguridad | 13 | Se publican el método, el modelo de amenazas y los hallazgos cerrados; los abiertos, a `library-privado/` hasta cerrarse |
| `docs/research/` | Investigación | 60 ficheros, 5,8 MB | Interno: hay capturas de terceros. Se puede publicar alguna nota, como la de motores de sincronización |
| `docs/design/` | Encargos y pantallas de Claude Design | 282 ficheros, 19 MB | Interno: imágenes y encargos. `errors.md` (catálogo de errores) se queda a la vista |
| `docs/testing/` | Pruebas a mano por fase | 7 | Se publica; su material alimenta también la guía de usuario |
| `docs/publicacion/` | Ficha, firma, políticas, web | 21 | Mezcla: las políticas ya son públicas en la web; la firma, el servidor de producción y la ficha, internos |
| Código y tests | 75 ficheros nombran hallazgos (`SEG-058`…) y 8 apuntan a `docs/plans`, `docs/audits` o `docs/design` | | Los ids se quedan y el registro de los cerrados se publica. Las rutas a lo interno se cambian por la página pública que lo explique |

El repositorio tiene 470 commits y `.git` ocupa 55 MB. La mayor parte son las imágenes de `docs/design/` y `docs/research/`; al quitarlas del historial bajará mucho.

## La documentación pública

En `docs/` del mismo repositorio, con Starlight (D90), en español e inglés, publicada en docs.kosmotheka.com. Tres partes, cada una con su manera de escribir:

### Quien usa la app

- **Lenguaje de la calle.** Nada de «operación», «sincronizar el vector», «HLC», «valla» ni «sello». Se dice «los cambios llegan a los demás», «el móvil de Hugo», «quitar a una persona».
- Cada página responde a una pregunta de alguien con la app en la mano («¿cómo comparto la biblioteca con mi pareja?»), con capturas.
- El índice ya está en [documentacion-publica.md](documentacion-publica.md), con lo que hay que tener antes de la prueba cerrada («A»).
- Regla: la escribe una sesión y la revisa otra **solo buscando jerga**. Lo que no entendería alguien que no programa, se reescribe.

### Quien monta su servidor

- **Primero, las limitaciones**, en una caja al principio, sin esconderlas:
  - **La app oficial es la nuestra.** Desde ella se puede elegir otro servidor, y la marca no se puede usar en una app o un servicio que se haga pasar por el oficial (D72).
  - **Sin avisos con la app cerrada.** La app se pone al día al abrirla (D75).
  - Los enlaces de invitación de un servidor propio se pegan en la app; no se abren solos con un toque.
  - Entrar con Google, el correo y las claves de los proveedores son opcionales, y cada uno se configura aparte.
  - Quien administra el servidor puede leer las bibliotecas que se suben a él: no hay cifrado de extremo a extremo.
- **Dos caminos con las mismas piezas** (D48): sin Docker (requisitos y pasos, como el servidor oficial) y con Docker (`Dockerfile` y `docker-compose`).
- Después: HTTPS, copias de PostgreSQL, actualizar a una versión nueva y cada variable de entorno.

### Quien quiere colaborar

- **`CONTRIBUTING.md`** con el procedimiento habitual:
  1. abrir antes un *issue* si el cambio es grande;
  2. hacer *fork* y una rama;
  3. hacer los cambios con las comprobaciones de la CI en verde;
  4. abrir un *pull request*;
  5. pasar la revisión.

  Más las reglas que de verdad importan aquí: todo cambio es una operación, el formato vive en `spec/`, los casos hostiles van primero, las novedades y las traducciones van en el mismo cambio.
- **Cómo funciona por dentro:** la arquitectura traducida y ordenada, con la explicación del motor local-first y la comparación con otros enfoques (ROADMAP, Fase 8).
- **Cómo avisar de un fallo de seguridad** (`SECURITY.md`) y el método de auditoría (D90).

## Orden de trabajo

Cada paso es un encargo cerrado para `library-sonnet`, revisado antes del siguiente:

1. **Inventario fino de `docs/`, fichero a fichero:** público, interno o «se reescribe en la documentación pública». Sale una tabla para que la apruebe el maintainer.
2. **Esqueleto de Starlight** en el repositorio (con Node en `library-dev`, sin instalar nada fuera), con los dos idiomas y la estructura de las tres partes. Compila en la CI; aún sin desplegar.
3. **`CONTRIBUTING.md`, `CODE_OF_CONDUCT.md` (Contributor Covenant), `TRADEMARKS.md` y el README nuevo**, en inglés.
4. **Las páginas «A» de la guía de usuario**, en español primero, con la revisión de jerga.
5. **La guía de autoalojamiento**, al cerrar 7.9: los dos caminos y las limitaciones.
6. **La parte de colaboradores:** la arquitectura traducida y ordenada.
7. **Quitar las rutas a lo interno** de `AGENTS.md`, las skills y el código; los scripts de auditoría, que funcionen sin los ficheros privados (D90).
8. **Con el maintainer:** escanear secretos, ensayar la reescritura del historial en una copia, mover lo interno a su carpeta y abrir `kosmotheka/kosmotheka`.

## Decidido (16/10/2026)

- **Lo que sale de la clasificación** ([clasificacion-docs.md](../historico/clasificacion-docs.md)), aceptado entero por el maintainer: los hallazgos abiertos se cierran o se aceptan antes de abrir; SEG-023 y SEG-032 se publican; el buzón de Gmail y los datos del VPS, en genérico; `assetlinks.json` se queda; TMview, los titulares de marcas y el código de Claude Design salen del historial; la firma y las notas de producción pasan, lo genérico, a la guía y el resto a `library-privado/`; `server/.env.dev`, al escaneo de secretos; `VISION.md`, reescrito si lo que queda explica el proyecto o a privado si no. Encargo: [encargo-limpiar-docs.md](../historico/encargo-limpiar-docs.md).

- **Diario de desarrollo público** (D90, enmendada): se publican las decisiones, los planes, el histórico de encargos, la arquitectura y las pruebas. Lo privado de verdad (hallazgos abiertos, firma, notas de producción) va a `library-privado/`. Sin segundo repositorio.
- **Los ids de hallazgos se quedan en el código** y se publica el registro de los cerrados.
- **Esqueleto de docs.kosmotheka.com hecho** (paso 2, 16/10/2026): Starlight en `site/`, `/es/` y `/en/` con el español por defecto (lo no traducido sale en español con aviso), mismas carpetas en inglés en los dos idiomas, sin nada de terceros, compilado en la CI y sin publicar. No se publica antes de abrir el repositorio: enlaza a `github.com/kosmotheka/kosmotheka`.
- **Guía de usuario, primera tanda** (paso 4, 16/10/2026): siete páginas en español de lo que funciona sin cuenta. Falta la revisión de jerga por otra sesión, sobre todo «Compartir en casa» y «Copias de seguridad». Diferencias con la app que salieron al escribirla, por arreglar:
  - **La Ayuda de la app está desfasada:** nombra «Mostrar código» y «Guardar copia», que ya no se llaman así, y no cuenta el «¿De quién es?» con el rol al emparejar (D97).
  - **La landing** dice «Dar acceso a otro dispositivo» para emparejar, los estados de lectura antiguos y promete préstamos, que aún no existen. **La política de privacidad** habla de la nube en futuro y no menciona Google Books a través de la cuenta, TMDB, RAWG ni Comic Vine, ni que cada catálogo ve la dirección de internet y la versión de la app.
  - **«Añadir» no reconoce un ISBN con guiones o de 10 cifras** en el buscador: lo busca como título. La casilla ISBN del formulario sí lo acepta.
  - **El botón «Escanear» del escritorio** lleva a «¿Lo tengo?», no escanea.
  - **El CSV** solo lleva datos de libros, y al importar todo entra como libro.
- **Ficheros de la raíz hechos** (paso 3): README, CONTRIBUTING, código de conducta, TRADEMARKS en borrador (D72) y plantillas de *issue* y PR. La licencia de las contribuciones (la del proyecto, sin CLA), confirmada por el maintainer el 16/10/2026.
