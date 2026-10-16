# Clasificación de `docs/` para abrir el repositorio

> Informe del 16/10/2026. Encargo: [encargo-clasificar-docs.md](encargo-clasificar-docs.md). Paso 1 de [plans/repositorio-publico.md](../plans/repositorio-publico.md), con la enmienda de D90 del 16/10/2026 (diario de desarrollo público). **Solo lectura:** no se ha movido ni cambiado nada. En este informe no hay ningún secreto, IP ni dato personal: solo ficheros, líneas y tipos.

## Cifras

Alcance: los 521 ficheros que git sigue en `docs/` (450), `brand/` (28) y `web/` (43). `ds-bundle/` **no está en git** (está en `.gitignore`, 8,7 MB en disco): no entra en el repositorio ni en el historial.

| Destino | Ficheros | Dónde |
|---|---|---|
| Público | 413 | casi todo `docs/design/`, `historico/`, `brand/`, `web/` |
| Público con cambios | 17 | ver la tabla |
| Privado | 50 | `docs/research/tmview/` (48) y dos guías de `docs/publicacion/` |
| A decidir | 41 | 8 informes de auditoría y `descartes.md`, el motor de Claude Design (28 `.js`), `assetlinks.json` y tres scripts de investigación |

**Peso que sale del historial:** unos **4,8 MB** de los 31,5 MB del paquete (`.git` en disco son 56 MB): 4,71 MB son `docs/research/tmview/` en todas sus versiones. Las dos guías privadas pesan menos de 0,1 MB. Si el maintainer decidiera sacar también `docs/design/screens/` (hoy público), saldrían otros **21,8 MB**; con la enmienda de D90 no hace falta, porque las capturas son diseños propios.

## Hallazgos de seguridad abiertos (categoría 1)

`docs/audits/hallazgos.md` tiene 106 filas: **91 resueltas, 3 descartadas, 2 aceptadas (SEG-023 por D93 y SEG-032 por D98) y 10 abiertas**. Las 10 abiertas, todas de gravedad baja salvo SEG-045 (media):

| Id | Qué es, en una línea | Descrito con detalle en |
|---|---|---|
| CAL-003 | excepciones genéricas de PHP | `audits/2026-10-09.md` |
| CAL-004 | `catch` vacíos y excepciones genéricas | `audits/2026-10-10.md` |
| CON-007 | la fusión de la app y la del servidor difieren en un orden de llegada | solo la fila de `hallazgos.md` (sin informe) |
| SEG-006 | la CSP deja cargar imágenes de cualquier origen https | `audits/2026-10-03.md` (y `amenazas.md`) |
| SEG-008 | dependencias con avisos sin parche, fuera del uso de la app | `audits/2026-10-03.md` |
| SEG-027 | quien tiene acceso a una biblioteca de la nube puede forzar que se rejuzgue entera con una operación antedatada | solo la fila de `hallazgos.md` |
| SEG-037 | el token de avisos vale hasta una hora tras el corte | `audits/2026-10-10.md` |
| SEG-040 | un servidor hostil puede reenviar una retirada antigua callando la concesión posterior | `audits/2026-10-10.md` |
| SEG-045 | el registro de actividad crece sin tope y cuenta campos, no efectos | `audits/2026-10-11.md` y `plans/fase-7.md:199` |
| SEG-074 | las claves selladas de la nube no van ligadas a su biblioteca | `audits/2026-10-15-2.md` |

Dónde se nombran sus ids (no necesariamente con detalle): los 8 informes de `audits/` (casi todos en una frase tipo «siguen abiertos…»), `hallazgos.md`, `amenazas.md` (SEG-006, 2 líneas), `development/revision-servidor.md:80`, `plans/fase-7.md` (CON-007 y SEG-045) e `historico/encargo-arreglos-auditoria-7.6.md:3`. **En el código no hay ningún id de los abiertos**: los 53 ids que nombran 84 ficheros de `app/`, `server/`, `crates/`, `spec/` y `.claude/` están todos en el registro y ninguno es abierto.

**Una debilidad sin id, en un fichero que sería público:** `docs/development/revision-servidor.md:80` dice que las rutas `pull`, `push` y `claim` «no tienen límite de peticiones». No está en el registro como hallazgo. Si es un fallo abierto, o se registra o se quita esa frase antes de publicar. Esa misma línea nombra SEG-046 como abierto y en el registro está **descartado** (texto desactualizado).

## Tabla

| Ruta | Destino | Por qué | Qué hay que cambiar antes |
|---|---|---|---|
| `README.md`, `ROADMAP.md`, `VISION.md` (docs/) | público | Sin datos de servidor ni secretos. `VISION.md` habla del maintainer y de su familia en primera persona y cita su alias | Que el maintainer lo lea y decida si deja la vida personal (punto 6 de abajo) |
| `docs/DECISIONS.md` | público con cambios | Contiene la dirección del buzón de Gmail del proyecto (línea 1289) y datos del VPS: proveedor y su política de puertos (1283), «su VPS no usa Docker» (689). Trae el enlace a TMview de D72 (externo, no se rompe) | Quitar o generalizar el buzón y el proveedor. D90 pasa de `propuesta` a decidida o se deja como está |
| `architecture/` (accounts, data-model, operations, security, sync) | público | Sin datos reales. `accounts.md:313` y otros usan una red de Docker de desarrollo, no una IP real | Nada |
| `architecture/cloud.md` | público con cambios | Enlaza a `servidor-produccion.md` (619) y `firma-android.md` (922) y dice cuánta memoria tiene el VPS (192) | Quitar los dos enlaces y la cifra de memoria |
| `audits/README.md` | público con cambios | Es el método; D90 (09/10) pide quitar los ejemplos con hallazgos reales (1 línea con un id) | Cambiar el ejemplo por uno inventado |
| `audits/amenazas.md` | público con cambios | D90 dice que se publica «después de repasarlo»; nombra SEG-006 (abierto) dos veces (líneas 45 y 82) | Repasar; decidir si SEG-006 se queda o se dice sin id |
| `audits/hallazgos.md` | público con cambios | Es el registro que D90 publica, pero con los cerrados | Quitar las 10 filas abiertas; decidir las 2 aceptadas (no son «abiertas», pero son límites conocidos) |
| `audits/2026-10-03.md`, `2026-10-03-2.md`, `2026-10-03-3.md`, `2026-10-09.md`, `2026-10-10.md`, `2026-10-11.md`, `2026-10-15.md`, `2026-10-15-2.md` | a decidir | Informes de cada pasada. D90 (09/10) decía que **no** se publican; la enmienda del 16/10 solo habla del registro. Describen con detalle 8 de los 10 abiertos (tabla de arriba) | Elegir: publicarlos quitando lo de los abiertos, o publicarlos al cerrarse todos, o dejarlos privados |
| `audits/descartes.md` | a decidir | Es una tabla mecánica de candidatos ya juzgados (fichero, función, código); no hay secretos. D90 (09/10) lo daba por privado. Los scripts de `.claude/skills/audit/bin/packet.py:228` lo leen | Si es público, nada; si es privado, el script debe funcionar sin él (paso 7 del plan) |
| `audits/mutaciones-2026-10-04.md` | público | Resultado de pruebas de mutación; sin detalle de fallos abiertos | Nada |
| `design/` (11 `.md`: brief, encargos, errors; y `current/escritorio-catalogo.png`) | público | Briefs y una captura de la propia app | Nada |
| `design/screens/` (227 ficheros: pantallas `*.dc.html`, css, svg, QR de ejemplo) | público | Diseño propio con datos inventados (nombres como «Hugo» o «Ana», un correo ficticio). Las portadas son marcas de posición, sin imágenes de terceros. Cargan `unpkg.com` | Nada. Si el maintainer quiere, quitar el correo ficticio de `cloud-account/Auth.dc.html` |
| `design/screens/*-overview.png` (14) | público | Capturas de las pantallas propias; pesan 21,8 MB en el historial | Nada |
| `design/screens/README.md` | público con cambios | Enlaza un proyecto privado de Claude Design con su identificador | Quitar el enlace con el id |
| `design/screens/*/support.js` y `ds-base.js` (28) | a decidir | Motor de Claude Design copiado en cada carpeta (`GENERATED… do not edit`). **Sin licencia clara.** El mismo `support.js` está 14 veces | Decidir: publicarlo, o dejar una copia o ninguna |
| `development/` (fixtures, icons, server, setup) | público | Contraseña de pruebas `fixtures-only` (ya está en el código), correos `.test` y redes de Docker de desarrollo; sin secretos reales | Nada |
| `development/correo.md`, `google-oauth.md`, `servicios-externos.md` | público con cambios | Dan la dirección del buzón de Gmail del proyecto (9 apariciones en 4 ficheros contando D80), y explican qué servicios se usan | Quitar o generalizar el buzón |
| `development/revision-servidor.md` | público con cambios | Nombra CAL-004 y SEG-046 y apunta una debilidad sin id (ver arriba) | Quitar lo abierto o registrarlo |
| `historico/` (44) | público | Encargos, planes y debates. Sin secretos, IP ni personas ajenas. Un nombre de fichero lleva una eñe (`encargo-d99-dueño-de-la-nube.md`) | Opcional: renombrarlo sin eñe (git lo cita entre comillas y rompe herramientas poco cuidadosas). `encargo-arreglos-auditoria-7.6.md:3` nombra un id abierto (SEG-045) |
| `plans/documentacion-publica.md` | público con cambios | Enlaza `servidor-produccion.md` (5 líneas) y `firma-android.md` | Cambiar esos enlaces |
| `plans/fase-7.md` | público con cambios | Enlaza `firma-android.md` (168); nombra CON-007 y SEG-045 como abiertos; cuenta una observación del VPS («Google Books dio 503…», 256) y que lo comparte con otro proyecto (300) | Quitar el enlace y generalizar lo observado en el servidor |
| `plans/fase-8.md` | público con cambios | Enlaza `firma-android.md` (9) | Cambiar el enlace |
| `plans/renombrado-kosmotheka.md`, `plans/repositorio-publico.md` | público | Sin datos privados; el segundo está desactualizado frente a la enmienda de D90 (tabla «Qué hay hoy») | Actualizar el plan (no es privacidad) |
| `publicacion/servidor-produccion.md` | privado | D90: notas del servidor de producción. Hoy son pasos con marcadores (`USUARIO@EL_VPS`) y sin datos observados, pero está pensada para rellenarse con lo que se vea en el VPS | Si algo de lo genérico sirve, pasarlo a la guía de autoalojamiento (paso 5 del plan) |
| `publicacion/firma-android.md` | privado | D90: firma de Android. Explica dónde vive `~/.kosmotheka-release/` y el flujo de la clave de subida; **no contiene claves, contraseñas ni huellas** | Mismo criterio: lo genérico, a la guía |
| `publicacion/ficha-google-play.md` | público con cambios | El texto de la ficha ya es público en Play. La sección «Pendiente para publicar» cuenta el estado de la cuenta de desarrollador y de la prueba cerrada y enlaza `firma-android.md` (62) | Quitar esa sección y el enlace |
| `publicacion/kosmotheka.app/` (`invitacion.html`, `README.md`) | público | Se sirven en un dominio público | `README.md:6` enlaza `firma-android.md`: cambiar el enlace |
| `publicacion/kosmotheka.app/.well-known/assetlinks.json` | a decidir | Lleva 2 huellas SHA-256 de certificados: la de la firma de Google Play y la **de la clave de subida**. El archivo es público por naturaleza (lo sirve el dominio y Android lo descarga) y una huella no es un secreto; pero el encargo pone la huella de la clave de subida como dato de cuenta de Play | Decidir si se publica el fichero con las dos huellas, o con la de Google solo |
| `publicacion/` (claim, novedades, politica-privacidad, politica-privacidad-con-servidor, web.md, `provisional-pages/` ×10) | público | Textos que ya están en la web; `web.md` usa marcadores para el acceso al servidor. `provisional-pages/` está superado | Opcional: quitar `provisional-pages/` (histórico) |
| `research/*.md` (foto-estanteria, fuentes-libros-es, koillection, notificaciones, proveedores, referentes, sync-engines) | público | Notas de investigación. En `proveedores.md` hay dos borradores de correo a proveedores firmados con el nombre del maintainer, con direcciones del dominio del proyecto | Nada |
| `research/logo.md` | público con cambios | Informe de búsqueda de marcas: nombra empresas que tienen marcas parecidas y enlaza 8 veces a `tmview/` (privado) | Rehacer los enlaces a las imágenes; decidir si deja los nombres de empresas |
| `research/naming.md` | público con cambios | Tabla de candidatos con los **titulares de marcas, algunos personas físicas** (líneas 65, 66, 76, 77; 9 filas con titular) y un enlace a `tmview/` (45) | Quitar o iniciales los nombres de personas; arreglar el enlace |
| `research/tmview/` (48: capturas de TMview y de la base de la OMPI, JSON de resultados, texto) | privado | Material de terceros **sin licencia clara** (capturas de bases oficiales con marcas ajenas) y, sobre todo, JSON y texto con **nombres de personas y empresas titulares** de marcas. Pesa 4,71 MB en el historial | Salir del historial. Los enlaces de `logo.md` y `naming.md` |
| `research/tmview-check.mjs`, `tmview-image-check.mjs`, `play-check.mjs` | a decidir | Scripts de Playwright que consultan TMview y Google Play. Sin secretos. Pueden chocar con los términos de esas webs, y generan lo que va a `tmview/` | Decidir si se publican como método o se quedan con los resultados |
| `testing/` (fase-2, 4, 5, 6, hito-1, ux-escanear-y-editar) | público | Pruebas a mano; sin datos reales | Nada |
| `testing/fase-7.md` | público con cambios | Líneas 111 y 114 dan el nombre de la base de datos y la ruta del registro de nginx del servidor real | Generalizar |
| `brand/` (28) | público | Logo y marca propios (SVG); la marca no entra en la licencia (`TRADEMARKS.md`) | Nada |
| `web/` (43) | público | Web ya pública. Tipografía Figtree con su `OFL.txt`; capturas propias | Nada |

## Resto de categorías

2. **Datos del servidor de producción:** ninguna IP, usuario ni ruta real. Sí aparecen en varios ficheros públicos: el proveedor del VPS, su memoria y que se comparte con otro proyecto (`DECISIONS.md`, `cloud.md`, `plans/fase-7.md`), una observación de comportamiento («503 desde el VPS») y dos rutas del servidor real en `testing/fase-7.md`. Ninguna huella de claves salvo las de `assetlinks.json` (arriba). No hay datos de la cuenta de Play (ni identificador de desarrollador).
3. **Datos personales:** correos que no son los permitidos: la dirección del buzón de Gmail del proyecto (9 apariciones en `DECISIONS.md`, `correo.md`, `google-oauth.md`, `servicios-externos.md`; es el buzón al que se reenvía `info@`) y correos inventados (`ana.ruiz@correo.es` en el diseño, `*@*.test` en fixtures, `correo@ejemplo.com`). Rutas `/home/…`: ninguna. Nombres de personas ajenas: solo los titulares de marcas de `tmview/` y `naming.md`. El maintainer sale con su nombre en `proveedores.md` (firma de borradores), con su alias en `VISION.md` y `plans/renombrado-kosmotheka.md` y en la URL del repositorio (`github.com/Tersoal/library`).
4. **Secretos:** ninguno en `docs/`, `brand/` ni `web/`. Lo que parece uno son marcadores (`<valor>`, `openssl rand`) y la contraseña de fixtures, que ya está en el código. Un detalle del historial, abajo.
5. **Material de terceros:** `research/tmview/` (sin licencia clara); el motor de Claude Design (`support.js`, `ds-base.js`: sin licencia clara); Figtree (OFL, con su licencia). Los diseños y capturas de `docs/design/` y `web/` son propios; no hay portadas de libros ni imágenes de otras apps.
6. **Para decidir con el maintainer, sin juzgar:** `VISION.md`, `ROADMAP.md`, `DECISIONS.md` y `plans/` hablan de «mi hijo», «mi pareja» y «mi mujer» como casos de uso, y `VISION.md` cuenta su trabajo y sus otros proyectos. No he encontrado opiniones negativas sobre terceros ni comentarios personales fuera de eso; las notas sobre apps de otros (`referentes.md`, `koillection.md`) son técnicas.

## Referencias cruzadas

**De ficheros públicos a privados** (se romperán; los números son líneas):

| Privado | Lo enlazan |
|---|---|
| `publicacion/servidor-produccion.md` | `architecture/cloud.md:619`, `plans/documentacion-publica.md` (49, 51, 56, 58, 61), `historico/encargo-7.8-{comic-vine,google-books,rawg,tmdb}.md`, `historico/encargo-cuentas-d100.md:46` |
| `publicacion/firma-android.md` | `architecture/cloud.md:922`, `plans/fase-7.md:168`, `plans/fase-8.md:9`, `plans/documentacion-publica.md:86`, `publicacion/ficha-google-play.md:62`, `publicacion/kosmotheka.app/README.md:6`, `historico/encargo-7.6.4-enlaces-que-abren-la-app.md` (18, 41), **`app/src-tauri/gen/android/app/build.gradle.kts:17`** (código) y un comentario de `.gitignore:12` |
| `research/tmview/` y sus scripts | `research/logo.md` (8), `research/naming.md` (45, 106, 124, 201), `DECISIONS.md:1081` (enlace externo a TMview, no se rompe) |

**Del código y la configuración a rutas de `docs/`** (si se aplican las decisiones de arriba, no se rompen salvo si alguna de estas carpetas se hiciera privada; las listo porque el plan quiere cambiar las que apuntan a lo interno): `docs/audits` en `.claude/skills/audit/SKILL.md` (10), `bin/facts.py`, `bin/packet.py` (3, lee `descartes.md`), `topics/library-access.md`, `topics/threat-model.md` y `app/src-tauri/src/sync.rs`; `docs/plans` en `AGENTS.md` (2), `server/AGENTS.md`, `.claude/skills/audit/` (3 ficheros), `app/src-tauri/src/data_folder.rs` y dos tests PHP; `docs/historico` en `AGENTS.md`, `server/AGENTS.md` (2), `facts.py` y `prepare.sh`; `docs/design` en 7 ficheros (`use-desktop.ts`, `index.css`, `account-errors.ts`, `SignOutReason.php`, `facts.py`, `prepare.sh`, `topics/ux-wording.md`); `docs/research` en `AGENTS.md`, `facts.py` y `prepare.sh`; `docs/testing` en 5 ficheros de la skill; `docs/publicacion` en `AGENTS.md` y `build.gradle.kts`.

## Historial

- **Ficheros que ya no existen:** 36, ninguno en `docs/`: 11 definiciones de agentes de auditoría antiguas (`.claude/agents/audit-*.md`) y `auditor-rules.md`, seis componentes de la app, tres o cuatro clases del servidor, dos casos de `spec/conformance/draft`, dos iconos de Android y **`server/.env.dev`**.
- **`server/.env.dev`:** estuvo en el historial desde el 25/09/2026 (commit `123ab69`) hasta el commit `77facc2` del 03/10/2026, «Keep APP_SECRET out of the repository». Eso indica que un `APP_SECRET` de desarrollo se subió. No he podido mirar su contenido (me lo han denegado y no lo he rodeado). **Hay que pasarlo por el escaneo del paso 8 (gitleaks y trufflehog) y, si el valor se parece al de algún entorno real, tratarlo como filtrado.** Tampoco hay en el historial `.jks`, `.pem`, bases de datos ni `.env` reales con otro nombre; sí `server/.env`, `server/.env.test` y `server/docker/.env.dist`, que son los de plantilla.
- **Autores y mensajes:** 473 commits de una sola persona con su correo personal (más los de dependabot) y 471 mensajes con la línea `Claude-Session:`. D90 ya prevé cambiarlos.
- Los 36 ficheros borrados no entran en ninguna otra de las categorías por su nombre.

## Los 20 objetos mayores del historial

(Tamaño comprimido en el paquete, y ruta; la misma ruta aparece varias veces porque son versiones distintas.)

| # | Tamaño | Ruta |
|---|---|---|
| 1 | 3,53 MB | `docs/design/screens/desktop-people-overview.png` |
| 2 | 3,53 MB | `docs/design/screens/desktop-people-overview.png` (otra versión) |
| 3 | 2,13 MB | `docs/design/screens/desktop-libraries-overview.png` |
| 4 | 1,63 MB | `docs/design/screens/mobile-people-overview.png` |
| 5 | 1,59 MB | `docs/design/screens/mobile-people-overview.png` (otra versión) |
| 6 | 1,56 MB | `docs/design/screens/mobile-people-overview.png` (otra versión) |
| 7 | 1,28 MB | `docs/design/screens/mobile-libraries-overview.png` |
| 8 | 1,00 MB | `docs/design/screens/desktop-marks-overview.png` |
| 9 | 0,93 MB | `docs/design/screens/desktop-overview.png` |
| 10 | 0,92 MB | `docs/design/screens/desktop-collections-overview.png` |
| 11 | 0,90 MB | `docs/design/screens/desktop-types-detail-overview.png` |
| 12 | 0,77 MB | `docs/design/screens/mobile-overview.png` |
| 13 | 0,66 MB | `docs/research/tmview/logo-2b-europa.png` |
| 14 | 0,60 MB | `docs/design/screens/desktop-types-overview.png` |
| 15 | 0,51 MB | `docs/research/tmview/logo-2b-wipo-concepto.png` |
| 16 | 0,49 MB | `docs/research/tmview/logo-lienzo-3-europa.png` |
| 17 | 0,46 MB | `docs/research/tmview/logo-L-europa.png` |
| 18 | 0,41 MB | `docs/research/tmview/logo-lienzo-4-europa.png` |
| 19 | 0,41 MB | `docs/research/tmview/logo-lienzo-1-europa.png` |
| 20 | 0,33 MB | `docs/design/screens/mobile-marks-overview.png` |

Por carpeta, sumando todas las versiones: `docs/design/screens/` 21,8 MB, `app/` 6,4 MB, `docs/research/` 4,9 MB (4,71 MB de `tmview/`), `docs/` (raíz y subcarpetas pequeñas) unos 5 MB, `server/` 1,9 MB.

## Lo que tiene que decidir el maintainer

1. **Informes de auditoría** (8 ficheros) y `descartes.md`: ¿se publican (quitando los abiertos), se publican al cerrarse, o quedan privados? D90 del 09/10 dice una cosa y la enmienda del 16/10 solo habla del registro.
2. **Los 2 hallazgos «aceptados»** (SEG-023, SEG-032): ¿se publican con el registro?
3. **La debilidad de `pull`, `push` y `claim` sin límite** (en `revision-servidor.md`): ¿se registra como hallazgo o se quita la frase?
4. **El buzón de Gmail del proyecto** y el proveedor y tamaño del VPS en `DECISIONS.md` y otros: ¿se generalizan o se dejan?
5. **`assetlinks.json`:** ¿con las dos huellas o solo la de Google?
6. **El motor de Claude Design (`support.js`, `ds-base.js`)** y los **scripts de TMview y Google Play**: ¿se publican, con qué aviso?
7. **`firma-android.md` y `servidor-produccion.md`:** hoy no tienen nada secreto; ¿van enteros a `library-privado/` o su parte genérica pasa a la guía de autoalojamiento?
8. **Los nombres de personas físicas** que salen en `naming.md` y `tmview/` (titulares de marcas): salen del repositorio, ¿de acuerdo?
9. **`server/.env.dev` en el historial:** pasarlo por el escaneo del paso 8 y decidir si ese `APP_SECRET` vale en algún sitio.
10. **Lo personal de `VISION.md`** (familia, trabajo, alias) y el nombre del fichero con eñe en `historico/`.

## Para el día de abrirlo

Lo que se deja a propósito mientras el repositorio es privado y hay que cambiar al abrirlo (paso 8 del plan). Números de línea del 16/10/2026; se comprueban con `git grep` antes de aplicar. Las decisiones están tomadas (el maintainer aceptó las recomendaciones el 16/10/2026).

**1. Enlaces a lo que sale del repositorio**

| Fichero (línea) | Cambio |
|---|---|
| `architecture/cloud.md` (619) → `servidor-produccion.md` | Quitar el enlace o apuntar a la guía de autoalojamiento |
| `plans/documentacion-publica.md` (49, 51, 56, 58, 61) → `servidor-produccion.md` | Quitar los enlaces o apuntar a la guía |
| `architecture/cloud.md` (922), `plans/fase-7.md` (168), `plans/fase-8.md` (9), `plans/documentacion-publica.md` (86), `publicacion/ficha-google-play.md` (62), `publicacion/kosmotheka.app/README.md` (6) → `firma-android.md` | Quitar el enlace o apuntar a la parte genérica que pase a la guía |
| `historico/encargo-7.6.4-enlaces-que-abren-la-app.md` (18, 41), `encargo-7.8-comic-vine.md`, `encargo-7.8-google-books.md`, `encargo-7.8-rawg.md`, `encargo-7.8-tmdb.md`, `encargo-cuentas-d100.md` → los dos ficheros | El histórico no se edita (`docs/historico/` queda como está): se aceptan esos enlaces rotos, o se deja una nota en `docs/README.md` |
| `app/src-tauri/gen/android/app/build.gradle.kts` (17, comentario) y `.gitignore` (12, comentario) → `firma-android.md` | Cambiar el comentario por la guía pública o quitar la ruta |
| `publicacion/servidor-produccion.md` y `publicacion/firma-android.md` | La parte genérica pasa a la guía de autoalojamiento; el resto, a `library-privado/`; se quitan del repositorio y del historial |

**2. TMview, sus scripts y los nombres de titulares**

| Qué | Cambio |
|---|---|
| `research/tmview/` (48 ficheros) | Quitar del historial (`git filter-repo --path docs/research/tmview --invert-paths`) |
| `research/tmview-check.mjs`, `research/tmview-image-check.mjs`, `research/play-check.mjs` | Quitar del historial. Citan `tmview/` (líneas 3, 15, 16, 24, 29 y 3, 11, 18, 21, 57) |
| `research/logo.md` (7, 13, 15, 41, 56, 60, 63, 64) y `research/naming.md` (45, 124, 201) | Reescribir los enlaces a las imágenes y a los scripts: texto sin imagen, o «resultado de la búsqueda del 26/09/2026» sin enlace |
| `DECISIONS.md` (1081) | Es un enlace externo a TMview y a la OMPI: no se rompe |
| Nombres de titulares que son personas físicas | Ya cambiados en `naming.md` (esta limpieza); los originales siguen solo en `tmview/` hasta que salga |

**3. Claude Design**

`design/screens/*/support.js` y `design/screens/*/ds-base.js` (28 ficheros, 14 carpetas): quitar del historial. Las pantallas `*.dc.html` los cargan (`ds-base.js` carga estilos desde `../..`): sin ellos no se ven en local; `design/screens/README.md` (párrafo «Para verlas en local») hay que reescribirlo o quitarlo.

**4. Hallazgos**

Antes de abrir, las 10 filas abiertas de `audits/hallazgos.md` se cierran o se aceptan: CAL-003 (9), CAL-004 (10), CON-007 (18), SEG-006 (41), SEG-008 (43), SEG-027 (62), SEG-037 (72), SEG-040 (75), SEG-045 (80), SEG-074 (109). Cuando cada una cambie de estado, hay que actualizar a la vez: `audits/amenazas.md` (45, 82, sobre SEG-006), `plans/fase-7.md` (199 y 400, sobre SEG-045 y CON-007), `historico/encargo-arreglos-auditoria-7.6.md` (3, SEG-045; histórico, se queda) y los ocho informes de `audits/`, que dicen «siguen abiertos». Después, `git grep -n -E "abierto" docs/audits/hallazgos.md` no debe dar ninguna fila.

**5. Otros**

- `server/.env.dev`: sale del historial tras el escaneo de secretos (el valor que tuviera `APP_SECRET`, tratarlo como conocido).
- `historico/encargo-d99-dueño-de-la-nube.md`: renombrar sin eñe (`encargo-d99-dueno-de-la-nube.md`) y cambiar el único enlace que lo nombra, `plans/fase-7.md` (190). Se hace con `git filter-repo --path-rename` para que el historial también lo recoja.
- `VISION.md`: aplicar la propuesta que apruebe el maintainer.
- `publicacion/provisional-pages/` (10 ficheros): opcional, quitar (histórico superado por `web/`).
- `plans/repositorio-publico.md`: actualizar la tabla «Qué hay hoy» y la sección de la carpeta ignorada, que ya no valen tras la enmienda de D90.
- Comprobación final del día: `git grep -n -i -E "gmail\.com|digitalocean" -- . ':!app/src/licenses.json'` sin resultados, y el escáner de secretos sobre todo el historial.
