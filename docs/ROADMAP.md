# Roadmap

> Plan rehecho el 25/09/2026 tras la [revisión del alcance](historico/revision-alcance.md) (decisiones D54–D63). Se basa en [DECISIONS.md](DECISIONS.md). Cada fase es útil por sí sola y entrega **código y documentación** (D12).

## Qué producto es, en una frase

**Kosmotheka** (D70): una app libre para catalogar lo que tienes (libros, cómics, juegos de mesa, películas, series, videojuegos, música y cualquier otra cosa) en **bibliotecas** que compartes con quien quieres, con permisos, **sin cuenta** por la red local o **con cuenta** a través de la nube.

## Visión general

| Fase | Nombre | Qué se puede hacer al terminarla | Estado |
|---|---|---|---|
| 0 | Esqueleto | Compilar en Linux, Windows y Android, con CI | ✅ |
| 1 | Catálogo en el escritorio | Catalogar libros en el escritorio | ✅ |
| 2 | Móvil y sincronización local | «¿Lo tengo ya?» en la tienda; sincronizar por wifi | ✅ en código, falta probar |
| 3 | Diseño del nuevo modelo | Saber cómo se ve y se usa todo lo decidido, antes de construirlo | ✅ |
| 4 | Bibliotecas | Varias bibliotecas por dispositivo, compartidas una a una | ✅ |
| 5 | Personas, roles y peticiones | Mi hijo ve la familiar, marca lo leído y pide; yo apruebo | ✅ |
| 6 | Catálogo general | Cualquier tipo de cosa, series, lo que falta y lo que deseo | ✅ |
| 7 | La nube | Registro con email, sincronizar a distancia, todos los proveedores | En curso |
| 8 | Publicación | La app en la tienda de Android y una landing | |
| 9 | Vida de la biblioteca | Préstamos, regalos, estadísticas | |
| 10 | Herramientas de estantería | Inventario, foto de balda, IA | |
| 11 | Servicio | Organizaciones, panel, web para el personal | |

**Transversal:** el diseño de la interfaz. La Fase 3 fija la base, y después se trabaja con Claude Design fase a fase. La app tiene que seguir los patrones de Android (navegación inferior, menú lateral…), con una sección de ayuda ([brief](design/brief.md)).

---

## Fase 3 — Diseño del nuevo modelo ✅

Cerrada el 26/09/2026.


Las pruebas a mano de las fases 1 y 2 se aplazan (decisión del 25/09/2026): la app va a cambiar mucho en las fases 4 a 6 y probar ahora sería trabajo perdido. Se probará la app rehecha, en Android y en Windows (`testing/`).

- **Rehacer el brief de diseño** con el nuevo modelo: bibliotecas, personas y roles, peticiones, tipos de catálogo, series, «lo tengo / me falta / lo deseo», cuenta y nube, ayuda, y los patrones de Android.
- Trabajarlo con Claude Design: pantallas del móvil y del escritorio que guíen las fases 4 a 7.
- **Hecho cuando:** hay un diseño aprobado de la navegación y de las pantallas principales del nuevo modelo.

**Estado:**
- [x] Brief nuevo y pantallas de Claude Design descargadas en `design/screens/` (25/09/2026).
- [x] La app sigue el diseño con los datos de hoy (26/09/2026): tokens, navegación (barra inferior, menú lateral, selector de biblioteca, barra lateral y superior en escritorio, botón atrás de Android), catálogo, «¿Lo tengo?» como sección, «Me falta», ficha a pantalla completa en el móvil y en panel en escritorio, y alta a pantalla completa en el móvil. Se puede ver en el navegador con datos de ejemplo: `npm run dev` y abrir `http://localhost:1420/?mock` (añade `&mobile` para simular el móvil).
- [x] Ajustes, Compartir, Alta rápida y Copias de seguridad con el mismo patrón; Copias con su propia entrada en el menú; sección de Ayuda con sus primeros temas.
- Lo que depende del modelo nuevo se hace en su fase, y sus pantallas se piden a Claude Design al empezarla: varias bibliotecas (fase 4); compartir con persona y rol, personas y permisos, peticiones y la campanita (fase 5); tipos, series, «lo deseo» y el alta con búsqueda por título (fase 6); cuenta y nube (fase 7).

## Fase 4 — Bibliotecas ✅

La pieza que se nos escapó: la biblioteca es lo que se comparte, se sincroniza y tiene permisos.

- Entidad **biblioteca** en la misma base de datos, con `library_id` en todas las tablas y clave `(library_id, id)`, como en el servidor.
- Cada biblioteca tiene lo suyo (obras, ediciones, ejemplares, miembros), aunque el mismo libro esté en varias.
- **Las operaciones pertenecen a una biblioteca**: cambia el formato en `spec/` y en `crates/domain`, con casos de conformidad nuevos.
- **Vector de versiones por biblioteca.**
- **Compartir biblioteca a biblioteca:** al emparejar, se elige qué bibliotecas van a cada dispositivo; nada sale sin decirlo.
- **Interfaz:** una biblioteca a la vez con selector; el buscador y «¿Lo tengo?» miran en todas.
- **Mover un libro** a otra biblioteca (ids nuevos, la app recuerda de dónde vino).
- **Copias:** completas y por biblioteca; exportar a Google Drive muy fácil y a la vista.
- **Migración con continuidad** (D46): los datos pasan a una biblioteca con el mismo id en todos los dispositivos, y los ya emparejados siguen sincronizando.
- **Rust que aprendes:** migraciones de datos, rediseño de un protocolo con tests que simulan dispositivos.
- **Hecho cuando:** tengo en el portátil mi biblioteca de rol y la familiar, y el móvil de mi pareja recibe solo la familiar.

**Estado:**
- [x] Borrador de los casos de conformidad de las operaciones con biblioteca: 13 casos en `spec/conformance/draft/library-scope.json` y las reglas en `spec/README.md` (sobre `library`, entidad `library`, códigos `invalid_library` y `other_library`). Revisado el 26/09/2026: el campo `library` y la biblioteca con solo nombre, aceptados; borrar y abandonar en [D65](DECISIONS.md). Queda por valorar mostrar quién creó cada biblioteca.
- [x] Casos de fusión por biblioteca: 8 casos en `spec/conformance/library-merge.json` (misma id en dos bibliotecas, lápidas y versiones que no cruzan, borrar la biblioteca). Plan de la fase en [historico/plan-fase-4.md](historico/plan-fase-4.md).
- [x] Esquema JSON, Rust (`crates/domain`) y PHP con el nuevo formato; los casos de validación pasan de `draft/` a `conformance/` (26/09/2026). Los de fusión siguen en `draft/` hasta el hito 4.3.
- [x] Tablas con `library_id`, migración y vector de versiones por biblioteca (26/09/2026). La migración deja antes una copia de la base de datos.
- [x] Compartir por biblioteca al emparejar, cambiarlo después y retirar el acceso (D71).
- [x] Interfaz: selector, nueva biblioteca, ajustes, borrar con copia, salir, «en todas», mover un libro.
- [x] Copias completas y por biblioteca (formato versión 3, D30).
- [x] Fase cerrada por el maintainer el 01/10/2026. Las pruebas en pantalla de [testing/fase-4.md](testing/fase-4.md) se hacen junto con las de las fases 5 y 6.
- Guardar la copia en Google Drive desde el móvil (en Android el diálogo de ficheros devuelve rutas `content://`, ver D30): aplazado el 28/09/2026, puede llevar trabajo.

## Fase 5 — Personas, roles y peticiones

Los mismos roles con cuenta y sin ella; sin servidor, los garantizan los propios dispositivos.

- **El miembro es la persona:** los dispositivos se asocian a un miembro al compartir («el móvil de Hugo» es de Hugo).
- **Roles:** administrador, editor y lector. Nadie da más de lo que tiene; **compartir es un permiso aparte**; puede haber varios administradores.
- **Peticiones:** el lector pide añadir o borrar; los administradores aceptan o rechazan.
- **Marcas por persona** (D68, adelantadas de la Fase 6): estado (pendiente, en curso, terminado o abandonado, con fecha y sin progreso), valoración y favorito. Cada persona marca lo suyo, también quien solo lee.
- **Firmas:** cada dispositivo firma sus operaciones (Ed25519) y los demás las comprueban.
- **Cada dispositivo comprueba lo que recibe:** lo que no está permitido pasa a petición, y todos llegan al mismo resultado aunque les lleguen las cosas en distinto orden. Casos de conformidad compartidos en `spec/`.
- **Recuperación:** revocar un dispositivo perdido desde otro administrador; para el administrador único, una salida por decidir (código de recuperación u otra).
- **Rust que aprendes:** criptografía con claves públicas, diseño de un motor que converge.
- **Hecho cuando:** mi hijo ve la biblioteca familiar y pide añadir un libro; yo lo apruebo desde el portátil; mi pareja edita sin pedir nada.

**Estado:**
- [x] Plan de la fase con hitos, propuestas técnicas y preguntas: [historico/plan-fase-5.md](historico/plan-fase-5.md) (27/09/2026).
- [x] 5.1 Firmas: cada dispositivo firma sus operaciones y los demás las comprueban, también a través de otro dispositivo (27/09/2026).
- [x] 5.2 a 5.4, sin interfaz: roles y de quién es cada dispositivo; cada dispositivo comprueba lo que recibe y lo que escribe, y lo no permitido queda como petición (D73, 27/09/2026).
- [x] 5.5 Peticiones, sin interfaz: pedir, aceptar y rechazar, también desde otro dispositivo (27/09/2026).
- [x] 5.6 Dispositivo perdido, sin interfaz: quitarlo de todas las bibliotecas, aviso de administrador único y duplicar como nueva (D58, 27/09/2026).
- [x] 5.7 Interfaz: personas y permisos, peticiones con campanita, pedir en lugar de hacer, de quién es cada dispositivo y emparejar con persona y rol (27/09/2026). Lista de pruebas en pantalla: [testing/fase-5.md](testing/fase-5.md).
- [x] 5.8 Marcas por persona (D68, adelantadas de la Fase 6): estado con fecha, valoración y favorito; cada persona marca lo suyo y ve lo de los demás (28/09/2026).
- [x] Fase cerrada por el maintainer el 28/09/2026. El plan pasa a [historico/plan-fase-5.md](historico/plan-fase-5.md).

## Fase 6 — Catálogo general ✅

Deja de estar centrado en libros.

- **Tipos:** libros (incluidos rol, PDF y ebooks), cómics, juegos de mesa, películas y series, videojuegos, música (vinilo, CD, casete y digital), y **«Otro»** para todo lo demás (sellos, monedas, Playmobil) con los campos comunes y sin proveedores. Orden propuesto: libros y «Otro» primero, después juegos de mesa y música (tienen proveedores sin clave que buscan por código de barras), películas y series, videojuegos y cómics.
- **Formatos por tipo**, físicos y digitales (DVD, Blu-ray, cartucho, Steam, Epic, Kindle…); lo digital no tiene estantería.
- **El alta empieza por buscar o escanear, no por el ISBN.** Búsqueda por título en los proveedores; alta a mano siempre posible, sin buscar nada.
- **Proveedores sin clave** para la app sin nube: Open Library, Inventaire, Wikidata, GameUPC; Google Books con clave propia opcional. Aviso y ayuda cuando un proveedor necesita la nube ([proveedores](research/proveedores.md)).
- **Series:** agrupan obras con orden o sin él; una obra puede estar en varias; relación «necesita» (expansiones, suplementos).
- **Tres estados:** lo tengo (por defecto), me falta (su propia vista, y los huecos en cada serie) y lo deseo (marca para regalos). Sustituye a la lista de deseos actual.
- **Marcas por persona** (D68): llegan en la Fase 5 (hito 5.8); aquí, «en curso» pasa a llamarse leyendo, jugando o viendo según el tipo, y los filtros por tipo.
- **Géneros, temáticas y etiquetas.**
- **Hecho cuando:** tengo mis juegos de rol en papel y en PDF, una saga con los tomos que me faltan, y los juegos de mesa y las películas de la familia.

**Estado:** hecha el 01/10/2026. Plan, decisiones y detalle de cada hito en [historico/plan-fase-6.md](historico/plan-fase-6.md); pruebas en pantalla en [testing/fase-6.md](testing/fase-6.md).
- [x] 6.1 Tipos y formatos: ocho tipos, formatos de cada uno, papeles de las personas, datos propios por tipo, código de barras y notas.
- [x] 6.2 Tipos en la interfaz: «¿Qué es?», un formulario por tipo, ficha de cada tipo, portadas con su forma, estados según el tipo y cambio de tipo con aviso. La plataforma del videojuego y las temporadas de una serie van en el ejemplar (D60, cambio del 29/09).
- [x] 6.3 El alta empieza por buscar: título o código escaneado, resultados con su fuente, «¿Qué es?» para un código sin tipo y «Añadir a mano» siempre a la vista.
- [x] 6.4 Proveedores sin clave (Open Library, Inventaire, Wikidata, MusicBrainz y Cover Art Archive), Ajustes › Proveedores de datos con fuentes que se apagan y clave propia para cualquiera que la pida (Google Books, GameUPC).
- [x] 6.5 Me falta (una obra sin ejemplares) y «Lo deseo» (una marca de cada persona); fuera la lista de deseos de D36.
- [x] 6.6 Series con orden o sin él, huecos, y la relación «necesita».
- [x] 6.7 Géneros (la lista de Design, cerrada por tipo) y etiquetas de cada biblioteca, con filtros y gestión.

**Pendiente de pulir** (no bloquea; se decide tras probar con la familia):
- Series en escritorio: la página de una serie se abre como diálogo y no como página; reordenar es solo con números (sin arrastrar); las filas no muestran año, formato ni el aviso «necesita».
- La ficha dice «Nº 2» de una serie en lugar de «2 de 6».
- Filtros: el género y la etiqueta activos se ven dos veces en la barra.
- Móvil: elegir géneros y etiquetas usa menús, no las hojas que dibujó Design.
- Proveedores en escritorio: es un diálogo, no la página de Ajustes que dibujó Design; «Cómo buscar mejor» no tiene página de ayuda aún.
- El formulario: el código de barras no tiene botón de escanear (solo el ISBN), y la portada propia (foto con la cámara o imagen del dispositivo) está desactivada porque el motor no la guarda ni la transmite todavía: ver D82 (abierta), propuesta para un hito propio después de la 7.2.
- Revisar en la app real los desplegables que se abrían fuera de la pantalla (se arreglaron dos).
- Proveedores (D83): el aviso de un proveedor que falla se va antes de leerlo; debe quedarse junto al resultado, con enlace a Proveedores. Botón de ayuda en cada clave, con enlace a la página donde se crea. Investigar la Biblioteca Nacional de España como fuente.
- Emparejar con QR: un enlace a Kosmotheka en Google Play junto al código, para quien aún no tiene la app instalada.
- Exportar e importar CSV: siguen siendo solo de libros.
- Wikidata: muchos cómics no están clasificados como cómic allí, y los juegos de mesa casi nunca tienen autor; GameUPC no da jugadores ni duración.

## Fase 7 — La nube

La nube es un dispositivo más, siempre encendido, con los mismos roles.

- **Plan de la fase:** [plans/fase-7.md](plans/fase-7.md), con hitos, propuestas y preguntas (01/10/2026).
- **Hecho: 7.0 Esqueleto del servidor** (Symfony 8.1, PHP 8.5, PostgreSQL 18, Docker propio, calidad, validación de operaciones en PHP con los casos de `spec/`, CI). Guía: [development/server.md](development/server.md).
- **Hecho: 7.2 Sincronizar con la nube** (04/10/2026): bibliotecas de la nube con id propio y tabla de acceso (D84), `pull` y `push`, subir una biblioteca con su historial, cadena de operaciones (D87) y copias cifradas sin `age` (D86).
- **Hecho: 7.4 El servidor aplica y juzga** (10/10/2026): catálogo de cada biblioteca en PostgreSQL (D92), fusión y permisos en PHP con los mismos casos de `spec/` que la app, `push` que juzga lo que guarda, enlazar como paso propio (D93) y el sello, que impide antedatar operaciones a antes de la activación de los roles (D94).
- **Registro con email** y código en la app, o con Google (D77); recuperación con código. Segundo factor opcional y desbloqueo con biometría (D78). La app entra en el servidor oficial o en otro.
- **La cuenta en la biblioteca** (D76): se sube biblioteca a biblioteca; la cuenta se enlaza sola con su persona, y un móvil nuevo recibe sus bibliotecas con su rol.
- Subir una biblioteca a la nube sin perder nada, historial incluido.
- **Enlace para invitar a una biblioteca**, además del QR (D43, D67).
- **Acceso seguro antes de sincronizar** (hito 7.A, auditoría del 03/10/2026): arreglar dos fallos de la red local (SEG-010, SEG-011) y una batería de tests de casos hostiles, que es la primera línea de defensa.
- **Sincronizar a distancia** con los mismos roles y peticiones; la nube juzga como un dispositivo más (D73) y la red local sigue funcionando a la vez (D39). La nube decide quién recibe qué con su propio id de biblioteca y una tabla de acceso, nunca con un id que llegue del cliente (D84).
- **Avisos al momento** con la app abierta (Mercure, D75), en lugar de preguntar cada cierto tiempo.
- **Proveedores con clave** (BoardGameGeek, TMDB, IGDB o RAWG, Comic Vine) a través del servidor, que pone las claves; se activan solos al conectarse.
- Límites de peticiones y cuentas sin confirmar que caducan.
- **Dar un dispositivo a otra persona** (D81): «Dejar este dispositivo a otra persona» borra sus bibliotecas y crea una identidad nueva, también sin cuenta; un dispositivo quitado desde otro hace lo mismo y pierde el acceso a las bibliotecas.
- **Registro de actividad** (D66): quién borró o creó una biblioteca, echó a alguien o cambió un rol, cuándo (hora del servidor) y desde dónde (IP y app), que sobrevive a lo borrado; con su plazo de conservación en la política de privacidad. *Servidor hecho en 7.7 (11/10/2026, pendiente de revisión): tabla `activity_log`, purga al año, consulta por consola; ver [cloud.md](architecture/cloud.md#el-registro-de-actividad-77-d66).*
- **Seguridad del servidor antes de producción:** una sesión con el maintainer para revisarla, proporcionada a un VPS pequeño y compartido; ahí se decide cómo hacer las copias de PostgreSQL sin gastar o gastando lo mínimo.
- **Producción:** nginx + PHP-FPM + PostgreSQL en el VPS; guía para quien monte el suyo.
- Se reutiliza del [plan anterior del servidor](historico/plan-servidor-2026-09.md) lo que sigue valiendo: D41, D42, D43, D48, D49 y D50 (D47 quedó reemplazada por D73).
- **Hecho cuando:** un amigo de otra ciudad y yo compartimos una biblioteca a través de la nube.

## Fase 8 — Publicación

- **Google Play:** cuenta de desarrollador (25 $, una vez), prueba cerrada con al menos 12 personas durante 14 días, y verificación de identidad.
- Mientras tanto, distribución a familia y amigos con la cuenta de público reducido (hasta 20 dispositivos, gratis).
- **Notificaciones push en Android** con la app cerrada (D75): FCM, y UnifiedPush sin Google. Para empezar: petición nueva, petición aceptada o rechazada, invitación aceptada. Con el permiso de notificaciones y Ajustes de avisos.
- **F-Droid:** estudiar cómo queda tras la verificación de desarrolladores de Google.
- ✅ **Landing pública** (02/10/2026): web estática de Claude Design en `web/`, publicada en kosmotheka.com con inicio, cómo funciona, descargas, el proyecto y privacidad, en español e inglés ([web.md](publicacion/web.md)). Falta el enlace al código cuando el repositorio sea público.
- **Experiencia de uso del alta y la ficha** (prueba del 05/10/2026): hecho lo urgente (escáner, BNE, editar en su propia pantalla, idioma, portadas); lo que falta, en [ux-escanear-y-editar.md](testing/ux-escanear-y-editar.md#pendiente).
- ✅ **Bienvenida al primer arranque (D91)** (05/10/2026): pocas pantallas que se pueden saltar: los datos se quedan en el dispositivo, escanear (solo en el móvil), los catálogos públicos con su interruptor (activados por defecto) y qué se envía, y compartir. Se vuelve a ver desde Ayuda. `WELCOME_VERSION` sube cuando se añade un servicio, y entonces sale otra vez a todos. Pendiente: la pantalla de los servicios que irán a través del servidor.
- Sección de ayuda completa en la app; documentación para quien quiera montar su servidor.
- **Documentación pública:** antes de escribirla, revisar la de proyectos open source famosos y bien estructurados para inspirarse. Dos públicos: quien usa la app y quien quiere colaborar. Entre otras cosas, explicar bien el motor de sincronización local-first (operaciones, reloj, fusión campo a campo, firmas y permisos) y compararlo con otros enfoques (WebRTC con relé, sincronizar ficheros, servidor central).
- **Abrir el repositorio con su historial (D90, propuesta):** escaneo de secretos de todo el historial, `git filter-repo` para quitar lo interno, autor `noreply` y sin líneas `Claude-Session:`; ensayo en una copia, `git bundle` de respaldo y subida a un repositorio nuevo, `kosmotheka/kosmotheka`; el privado se borra después. La documentación de trabajo pasa a una carpeta ignorada dentro de `docs/`, que es su propio repositorio privado.
- **Documentación en docs.kosmotheka.com (D90, propuesta):** Markdown en el repo, convertido con un generador estático (Starlight propuesto) y desplegado solo al cambiar `main`. La landing también deja rsync y pasa a desplegarse sola.
- **El método de auditoría, público y repetible** (09/10/2026, D90): se publica qué se audita y cómo (temas, preguntas, reglas, scripts, gravedad, lo que aún no cubre), no los informes ni los hallazgos de nuestro proceso. Una página en la documentación pública separa la parte determinista (cualquiera la ejecuta con Docker; cada salida explicada y cómo interpretarla) de la de juicio (un procedimiento que sigue una persona o cualquier asistente, no solo Claude), y explica cómo avisar de un fallo. El método se revisa cada cierto tiempo con lo que llegue de fuera.
- **Antes de abrir el repositorio** (auditoría 03/10/2026): activar en GitHub el aviso privado de vulnerabilidades (*Settings › Code security › Private vulnerability reporting*), al que remite `SECURITY.md`. Y una pantalla «Licencias» en la app con `THIRD-PARTY-NOTICES.md`, para que los avisos viajen también en los instaladores.
- **Marca (D72, propuesta):** buscar que el logo esté libre (TMview, WIPO Global Brand Database, búsqueda inversa; clases de Niza 9 y 42) y registrarlo como marca figurativa, junto al nombre si se registra. Antes de abrir el repositorio: exclusión de la marca en LICENSE y README, y un TRADEMARKS.md.
- **Publicar versiones (D74, propuesta):** un workflow que, al subir una etiqueta, crea un *release* en borrador en GitHub con los instaladores de Linux y Windows (nombres sin versión, para enlazar siempre a la última) y sube el `.aab` firmado a la pista de prueba de Google Play. La primera subida a Play es a mano; las claves, en los secretos de GitHub y con copia fuera del repositorio.
- **Linux para todos (D74):** el `.deb`, `.rpm` y AppImage, compilados en Ubuntu 22.04 en el CI (valen de la 22.04 en adelante); la **Snap Store** para Ubuntu 20.04 y cualquier Ubuntu, con un clic desde el centro de software; después Flathub.
- **Página de descargas** en la landing: ya está, con todo en «Próximamente» y qué sistemas admite cada fichero. Falta enlazar los ficheros de verdad y las versiones anteriores (en estática, enlaces a la última versión de GitHub; si pasa a Symfony, rutas `/download/{plataforma}`).
- Por estudiar: actualizaciones automáticas de escritorio (plugin `updater` de Tauri) y firma de código en Windows.
- **Hecho cuando:** cualquiera puede instalarla desde la tienda.

## Fase 9 — Vida de la biblioteca

- **Préstamos** salientes y entrantes, con historial y avisos. Basta con que lo apunte quien presta: el otro no necesita la app.
- **Red de préstamo entre amigos:** los dos ven el mismo préstamo y el amigo puede pedir algo prestado. Sin nube, solo dentro del círculo de confianza; con amigos a distancia, a través de la nube ([referentes §3.1](research/referentes.md)).
- **Lecturas sin ejemplar:** libros que te han prestado.
- **Reserva de regalos**, oculta para quien los recibe. **Oculta solo en pantalla** (D69): el deseo lo ve toda la familia; la reserva («lo compro yo»), todos menos quien recibe el regalo. Igual con nube y sin ella.
- **Enlaces para compartir un ejemplar u ofrecerlo en préstamo** a quien no es miembro (D67): un permiso de la biblioteca con token aleatorio, revocable y con caducidad, que muere si el ejemplar se mueve o se borra. Necesita la nube (Fase 7), porque quien lo abre no tiene la app.
- Ejemplar firmado o dedicado, con foto.
- **Estadísticas** de lectura por año y por persona.
- **Copia automática periódica** (D58, cambiado el 03/10/2026): la app guarda su propia copia sin claves cada cierto tiempo y, al reinstalar, ofrece restaurarla. Un ajuste para activarla o desactivarla. Por decidir en detalle: dónde se guarda (solo en el dispositivo, Google Drive u otro servicio), cada cuánto y qué hacer con las portadas si no caben.
- ¿**Papelera** para deshacer borrados? (abierto; podría adelantarse a la Fase 5).

## Fase 10 — Herramientas de estantería

- **Modo inventario** balda a balda, y ubicaciones con jerarquía (salón → estantería → balda).
- **Etiquetas QR imprimibles** para pegar en los ejemplares (D67), útiles en el modo inventario y en ludotecas: código corto aleatorio de la biblioteca, no el UUID; al mover el ejemplar se imprime otra.
- **Foto de balda** para la carga inicial: OCR local y, si no basta, IA (D19, D20).
- **Leer el ISBN impreso con la cámara** (05/10/2026), para cuando el código de barras es una etiqueta de tienda: reconocimiento de texto en el propio móvil (ML Kit, sin red), buscar un número que empiece por 978 o 979 y comprobar su dígito de control. No hay plugin de Tauri para esto: se escribe uno y se publica como **plugin libre de Tauri en su propio repositorio**, como las passkeys. Mientras tanto, el escáner salta los códigos de tienda y deja escribir el número.
- **IA como complemento de cualquier catálogo:** reconocer una portada o una caja y buscarla. Referencias: Google Lens, bookidentifier.com; probar antes el OCR local con el título y la autoría de la portada ([notas](testing/ux-escanear-y-editar.md)).
- **Importar desde plataformas** digitales (Steam).

## Fase 11 — Servicio

Solo si llega el escenario de ludotecas, clubes o colegios.

- **Organizaciones** por encima de las bibliotecas (la biblioteca ya tiene un propietario que lo permite).
- **Panel del superadministrador** con EasyAdmin y KPI (D51, D52).
- Inicio de sesión corporativo (Google, Microsoft).
- **Web para el personal** (D53).
- Licencias comerciales de los proveedores que las exigen; protección de datos con menores.

## Queda abierto

- Papelera para deshacer borrados.
- Salida para el administrador único que pierde su único dispositivo.
- Campos personalizados por el usuario (solo si alguien los pide).

## Ideas para el futuro

No entran en ninguna fase; se guardan para no perderlas de vista ([referentes §3](research/referentes.md)).

- **La dimensión social:** comentarios, préstamo entre desconocidos, compartir en redes (§3.3).
- **Bibliotecas públicas** que se descubren buscando y se navegan sin invitación.
- **Reservar un ejemplar prestado.**
- **Importar desde Calibre** (muy opcional).
- **«¿Qué leo ahora?».**
- **Estantería visual** con lomos.
- **Base de datos colaborativa** (futuro lejano).
- **Reconocer mejor los dispositivos «sin persona»** (27/09/2026): con muchos dispositivos y sin servidor pueden juntarse varios sin persona difíciles de distinguir. Ideas: en Android, usar como nombre por defecto el de los ajustes del móvil (el mismo que ve el Bluetooth del coche) o el modelo, en vez de «Kosmotheka»; enseñar el nombre al emparejar («Te verán como…», con «Cambiar»); y mostrar en la fila cuándo apareció y cuándo se sincronizó por última vez. Se dejó para más adelante para no añadir código nativo de Android por un detalle.
- **Precio de compra** de cada ejemplar: fuera de la Fase 6 (28/09/2026); obligaría a elegir divisa, entre otras cosas.
- **Llaves de acceso (passkeys)** para entrar sin contraseña (01/10/2026): Tauri no tiene plugin oficial; si se construye la integración, publicarla como plugin libre de Tauri. La biometría y el segundo factor entran ya en la Fase 7 (D78).
- **Modelo de pago:** no se espera; la app es gratuita. Si llegara, sería un módulo aparte, fuera del repositorio público.

---

## Anexo: fases completadas

### Fase 0 — Esqueleto

El objetivo es eliminar riesgos técnicos antes de escribir funcionalidad: ver que la cadena de compilación funciona en las **tres** plataformas.

- Monorepo (D16) con licencias (D11), README en inglés y la estructura de `docs/`.
- App Tauri 2 + React + TypeScript + Vite + shadcn/ui (D15), con i18n en inglés y español (D17).
- `rusqlite` con una migración de prueba (D14).
- **Resolver antes el entorno de desarrollo en Linux** (D23) e instalar Rust, Node, Android Studio (SDK, NDK, JDK 17) y `gh`.
- Compilación y arranque en Linux y en Windows, y una **APK de depuración instalada en tu móvil**. Android es la parte más delicada de Tauri: mejor descubrir sus problemas el primer día.
- CI en GitHub Actions: lint, tests y compilación.
- **Rust que aprendes:** Cargo, *crates* y *workspaces*, estructura de un proyecto Tauri, comandos de Tauri (cómo llama la interfaz a Rust).
- **Hecho cuando:** una ventana «Hello, library» que lee un dato de SQLite en las tres plataformas, y la CI en verde.

**Estado (24/09/2026):**
- [x] Entorno en Distrobox (D23), documentado en `development/setup.md`.
- [x] Monorepo con *workspace* de Cargo, licencia AGPL, README y `AGENTS.md`.
- [x] Tauri 2 + React + TypeScript + Vite + Tailwind + shadcn/ui, con i18n en inglés y español y modo oscuro según el sistema.
- [x] `rusqlite` con migraciones y un comando que cuenta los arranques. Tests de Rust, Clippy y ESLint en verde.
- [x] App de escritorio funcionando en Linux.
- [x] APK de depuración compilado.
- [x] APK instalado y funcionando en un móvil real (moto g84 5G, Android 15).
- [x] Repo privado en GitHub y CI en verde: lint, tests y compilación en Linux y Windows.
- [ ] Probar en Windows a mano.

### Fase 1 — Catálogo en el escritorio (modo local)

- *Crate* de dominio (`domain`), separado de Tauri: ediciones, ejemplares, ubicaciones, formatos (D08).
- **Registro de operaciones desde el principio**: toda escritura es una operación con UUID v7, identificador de dispositivo y marca HLC (D05, D13). Todavía no se sincroniza, pero así el formato ya queda probado.
- Primera versión de la especificación de operaciones en `spec/`, con sus casos de conformidad (D06).
- Alta manual; búsqueda por ISBN en Google Books y Open Library detrás de un «proveedor de metadatos» (D18); portadas; enlaces a las fichas.
- Campo de ISBN que acepta «número + Intro» y un modo ráfaga en el escritorio: sirve para teclear rápido y, de paso, **soporta cualquier lector de códigos USB** (D19, vía 2). Se prueba tecleando, sin comprar nada.
- Listado, búsqueda y filtros.
- **Historial** a partir del registro de operaciones.
- **Exportación e importación** en JSON y CSV: copia de seguridad y garantía de que el usuario no queda atado a la app.
- **Rust que aprendes:** *ownership* y *borrowing*, `Result` y el manejo de errores, *traits* (el proveedor de metadatos), *serde*, HTTP con `reqwest`, tests.
- **Hecho cuando:** catalogo mis libros en el escritorio sin servidor. Es la primera versión útil del modo local open source.

**Hitos** (cada uno se puede ver y probar):

| Hito | Qué se construye | Qué se ve al terminar |
|---|---|---|
| 1.1 Dominio y alta manual | *Crate* `domain`; esquema obra/edición/ejemplar (D24); alta manual y listado | Doy de alta un libro a mano y aparece en la lista |
| 1.2 Registro de operaciones | Toda escritura pasa a ser una operación (UUID v7, dispositivo, HLC); especificación en `spec/` y primeros casos de conformidad | Lo mismo, pero cada cambio queda registrado; historial de un libro |
| 1.2b Editar y borrar | Editar la ficha y los autores; añadir, editar y borrar ejemplares; borrar libros | Corrijo lo que tecleé mal y quito lo que ya no está |
| 1.3 Búsqueda por ISBN | Proveedores de Google Books y Open Library detrás de un *trait*; portadas guardadas en local | Tecleo un ISBN y se rellena la ficha con su portada |
| 1.4 Modo ráfaga | «ISBN + Intro» encadenado; sirve para lectores USB | Doy de alta una pila de libros seguidos |
| 1.5 Buscar y filtrar | Texto completo con FTS5; filtros por formato, ubicación y propietario | Encuentro cualquier libro al instante |
| 1.6 Exportar e importar | JSON completo (copia de seguridad) y CSV | Saco y recupero mis datos |

**Estado:**
- [x] 1.1 Dominio y alta manual. *Crate* `domain` (ISBN con dígito de control y paso de ISBN-10 a ISBN-13, validación que devuelve todos los errores a la vez), esquema obra/edición/ejemplar, alta manual con varios autores y listado. Un ISBN repetido añade un ejemplar a la edición existente.
- [x] 1.2 Registro de operaciones. Operaciones como cambios de entidad con etiqueta de intención (D25), reloj HLC, identificador de dispositivo, registro `operations` aplicado en la misma transacción, especificación en `spec/` (JSON Schema y 18 casos de conformidad, licencia MIT) e historial de cada libro. Detalles en [architecture/operations.md](architecture/operations.md).
- [x] 1.2b Editar y borrar (añadido al plan el 25/09/2026). Ficha del libro con pestañas; al editar, cada operación solo lleva los campos que han cambiado, y editar sin cambios no registra nada. Borrar un libro elimina la edición, sus ejemplares y, si era la única edición, la obra; autores y propietarios se conservan.
- [x] 1.3 Búsqueda por ISBN. Open Library y Google Books a la vez (D26), con clave de Google opcional en Ajustes; los fallos de un proveedor son avisos. Portadas descargadas y guardadas por hash (D27) y mostradas en la tabla y la ficha; enlaces a las fichas externas. Pruebas reales contra los servicios con `cargo test -p kosmotheka -- --ignored live_`.
- [x] 1.4 Alta rápida. Cola de ISBN con datos de ejemplar fijados por sesión; guarda lo encontrado al momento, no duplica lo que ya existe y deja pendiente lo no encontrado, con un campo para el título (D28). Funciona con teclado y con lectores de códigos USB.
- [x] 1.5 Buscar y filtrar. Búsqueda de texto completo (FTS5) sin acentos, por prefijos, por editorial y por ISBN en cualquier formato; filtros por formato, propietario y ubicación; editoriales como catálogo (D29); autocompletado de autores, editoriales, propietarios y ubicaciones. Las ubicaciones siguen siendo texto libre: la jerarquía salón → estantería → balda queda para más adelante.
- [x] 1.6 Exportar e importar. Copia de seguridad en ZIP (datos, portadas e historial) que se restaura sobre un catálogo vacío como una sola operación; CSV de una fila por ejemplar, exportable e importable con informe de filas rechazadas (D30).

**Mejoras pedidas tras probar el 1.1** (se harán sin prisa, junto al hito que las toque):
- [x] Autocompletar autores al escribir, a partir de los ya catalogados (1.5).
- [x] Editoriales como catálogo propio, con autocompletado (1.5).
- [x] Rellenar datos y autores consultando catálogos online (Google Books, Open Library) (1.3).

### Fase 2 — Móvil y sincronización local

- APK Android con escáner de ISBN.
- **Escáner en ráfaga** para la carga inicial (D19, vía 1).
- **«¿Lo tengo ya?»**: escanear y ver al momento si lo tenemos, de quién es y en qué formato.
- **Lista de deseos básica**: «no lo tengo → a deseos», desde la misma pantalla.
- Emparejamiento por QR y sincronización por la red local (D13):
  - **v1:** copia completa del escritorio al móvil.
  - **v2:** en los dos sentidos, por operaciones (HLC, el último en escribir gana campo a campo, lápidas), con descubrimiento por mDNS al llegar a casa.
- **Rust que aprendes:** red (TCP y TLS con una clave fijada por el QR), concurrencia, plugins de Tauri en móvil.
- **Hecho cuando:** en una tienda sin cobertura, escaneo un libro y sé si ya lo tengo; al volver a casa, todo se sincroniza.

**Hitos** (diseño en [architecture/sync.md](architecture/sync.md)):

| Hito | Qué se construye | Quién lo prueba |
|---|---|---|
| 2.0 Motor de sincronización | Operaciones replicables y operación inicial (D31); aplicar operaciones recibidas campo a campo, con lápidas (D32) y duplicados (D33); vector de versiones | Tests que simulan dos y tres dispositivos |
| 2.1 Sincronización por la red local | Servidor HTTPS en el escritorio, certificado fijado, emparejamiento con código (D34) | Dos instancias de la app en el mismo equipo |
| 2.2 Interfaz móvil y escáner | Pantallas adaptadas al móvil, escáner de ISBN con la cámara, «¿lo tengo ya?», lista de deseos básica | En el móvil |
| 2.3 Móvil emparejado | Emparejar escaneando el QR del escritorio; reencontrar al otro dispositivo en la wifi de casa (D37) | En el móvil |

**Estado:**
- [x] 2.0 Motor de sincronización. Operaciones replicables con versión de esquema; operación inicial `sync.baseline` por dispositivo; vector de versiones; aplicación de operaciones recibidas campo a campo, con lápidas y puntos de restauración (*savepoints*) para saltar cambios imposibles sin romper la sincronización; duplicados permitidos; filtros y sugerencias por nombre. 12 tests simulan dos y tres dispositivos y comprueban que acaban con el mismo estado.
- [x] 2.1 Sincronización por la red local. Servidor HTTPS (axum, rustls con ring) que escucha solo cuando hace falta; certificado propio fijado por la huella del código; emparejamiento con secreto de un solo uso; sincronización en los dos sentidos; diálogo «Dispositivos» con código en texto y QR; sincronización automática al abrir la app. Los tests emparejan y sincronizan por TLS real.
- [x] 2.2 Interfaz móvil y escáner. «¿Lo tengo ya?» consultando solo la base de datos local (funciona sin cobertura), con escáner de cámara en el móvil (D35) y búsqueda online opcional; lista de deseos sincronizada (D36) con «Comprado: añadir al catálogo»; vista en tarjetas y cabecera compacta en pantallas estrechas. Compila para Android con permiso de cámara; falta probarlo en el móvil.
- [x] 2.3 Móvil emparejado. Emparejar escaneando el QR con la cámara; sincronización al abrir la app y al volver a ella; reencontrar al otro dispositivo en la subred si cambió de dirección (D37, en lugar de mDNS). **Fase 2 completa en código**; pendiente de probar en el móvil (`docs/testing/fase-2.md`).
